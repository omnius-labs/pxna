//! 優先度つきの同時実行の枠。
//!
//! 処理は 3 つの段（[`WorkClass`]）のどれかに属する。資源ごとに [`PriorityPool`] を 1 つ持ち、次の規則で開始を決める。
//!
//! - Interactive は、ユーザーが今まさに待っている 1 件（表示中の画像、再生中の動画のセグメントなど）である。
//!   共有の枠に加えて、Interactive だけが使える予約の枠を持つ。
//! - Foreground は、同じ資源で Interactive が動いているか待っている間は始めない。
//! - Background は、どの資源でも Interactive か Foreground が動いているか待っている間は始めない。
//!   Background は共有の枠を使わず、専用の枠だけを使う。上位の段が Background の終わりを待つことはないので、
//!   Background が 2 つ目の資源を待って止まっても、上位の段を巻き込んで止まらない。
//!
//! 始まった処理は止めない。同じ段の中では、待ち始めた順に始める。上位の段が続く間、下位の段は始まらない。
//!
//! Background を保留するかどうかは、枠を使っている処理と待ちに加えて、[`PriorityPool::occupy`] で数えているものも見る。
//!
//! 待ちの札（[`Ticket`]）の状態は、待ち行列に入っているかどうかだけで決まる。次の 2 つを守る。
//!
//! - 起こし損ねない。状態が変わって、待っている側が始められるようになり得るときは、必ず通知する。
//!   待つ側は、通知を受ける用意（`enable`）をしてから状態を確かめる。
//! - 誰も待っていない札が行列に残らない。行列から外すのは、枠を得た札、drop された札、[`RaiseHandle::discard`] された札だけで、
//!   すべて 1 つの lock の中で行う。残った札は、その段の保留を止め続けてしまう。
use std::{
    collections::VecDeque,
    pin::pin,
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
};

use tokio::sync::Notify;
use tracing::Span;

use crate::prelude::*;

use super::work::{WorkClass, WorkKind};

const CLASSES: usize = 3;

impl WorkClass {
    fn index(self) -> usize {
        match self {
            Self::Interactive => 0,
            Self::Foreground => 1,
            Self::Background => 2,
        }
    }
}

/// 1 つの資源の枠の数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolLimits {
    /// Interactive と Foreground が使う枠。
    pub shared: usize,
    /// 共有の枠とは別に、Interactive だけが使う枠。
    pub interactive: usize,
    /// Background だけが使う枠。
    pub background: usize,
}

#[derive(Default)]
struct PoolState {
    limits: Option<PoolLimits>,
    /// 枠を使って動いている数。
    running: [usize; CLASSES],
    /// 枠を使わずに、動いているものとして数える数（[`PriorityPool::occupy`]）。
    occupied: [usize; CLASSES],
    waiting: [VecDeque<u64>; CLASSES],
}

impl PoolState {
    fn limits(&self) -> PoolLimits {
        self.limits.expect("a registered pool has limits")
    }

    /// 待ち行列にいる `id` の、今の段。行列にいなければ（枠を得たか、外されたか）`None`。
    fn class_of(&self, id: u64) -> Option<WorkClass> {
        WorkClass::ALL.into_iter().find(|class| self.waiting[class.index()].contains(&id))
    }

    /// 待ち行列から `id` を外す。外したときだけ `true`。
    fn remove(&mut self, id: u64) -> bool {
        let Some(class) = self.class_of(id) else {
            return false;
        };
        self.waiting[class.index()].retain(|waiter| *waiter != id);
        true
    }

    /// `id` を、より上の段の行列へ移す。`id` は到着順（番号順）の位置に入るので、同じ段の中の順序は変わらない。
    fn raise(&mut self, id: u64, target: WorkClass) -> bool {
        let Some(current) = self.class_of(id) else {
            return false;
        };
        if target.index() >= current.index() {
            return false;
        }
        self.waiting[current.index()].retain(|waiter| *waiter != id);
        let queue = &mut self.waiting[target.index()];
        let position = queue.iter().position(|waiter| *waiter > id).unwrap_or(queue.len());
        queue.insert(position, id);
        true
    }

    fn active(&self, class: WorkClass) -> bool {
        let index = class.index();
        self.running[index] + self.occupied[index] + self.waiting[index].len() > 0
    }

    /// 枠に空きがあるか。Interactive は予約の枠から先に使う。
    fn has_room(&self, class: WorkClass) -> bool {
        let limits = self.limits();
        let interactive = self.running[WorkClass::Interactive.index()];
        let in_shared = interactive.saturating_sub(limits.interactive) + self.running[WorkClass::Foreground.index()];
        match class {
            WorkClass::Interactive => interactive < limits.interactive || in_shared < limits.shared,
            WorkClass::Foreground => in_shared < limits.shared,
            WorkClass::Background => self.running[WorkClass::Background.index()] < limits.background,
        }
    }
}

struct State {
    pools: Vec<PoolState>,
}

impl State {
    /// `waiter` が待ち行列の先頭にいる（`None` なら同じ段に待ちがない）とき、`class` の処理を `pool` で始めてよいか。
    fn can_start(&self, pool: usize, class: WorkClass, waiter: Option<u64>) -> bool {
        let state = &self.pools[pool];
        let queue = &state.waiting[class.index()];
        let first = match waiter {
            Some(waiter) => queue.front() == Some(&waiter),
            None => queue.is_empty(),
        };
        let held = match class {
            WorkClass::Interactive => false,
            WorkClass::Foreground => state.active(WorkClass::Interactive),
            WorkClass::Background => self.pools.iter().any(|pool| pool.active(WorkClass::Interactive) || pool.active(WorkClass::Foreground)),
        };
        first && !held && state.has_room(class)
    }
}

/// すべての資源の枠と、その待ちを持つ。Background の保留はすべての資源を見るので、1 つの lock で守る。
pub struct Scheduler {
    state: Mutex<State>,
    changed: Notify,
    next_waiter: AtomicU64,
    /// 処理の種類ごとの、枠を得た数と返した数。すべての資源を合わせて数える。
    started: [AtomicUsize; WorkKind::COUNT],
    finished: [AtomicUsize; WorkKind::COUNT],
}

impl Default for Scheduler {
    fn default() -> Self {
        Self {
            state: Mutex::new(State { pools: Vec::new() }),
            changed: Notify::new(),
            next_waiter: AtomicU64::new(0),
            started: std::array::from_fn(|_| AtomicUsize::new(0)),
            finished: std::array::from_fn(|_| AtomicUsize::new(0)),
        }
    }
}

impl Scheduler {
    /// 資源を 1 つ足す。
    pub fn pool(self: &Arc<Self>, limits: PoolLimits) -> PriorityPool {
        let mut state = self.state.lock();
        state.pools.push(PoolState {
            limits: Some(limits),
            ..Default::default()
        });
        PriorityPool {
            scheduler: self.clone(),
            index: state.pools.len() - 1,
        }
    }

    /// `kind` の処理の（枠を得た数, 枠を返した数）。枠を返すのは、処理が本当に終わったときである。
    pub fn counts(&self, kind: WorkKind) -> (usize, usize) {
        (self.started[kind as usize].load(Ordering::Relaxed), self.finished[kind as usize].load(Ordering::Relaxed))
    }
}

/// 1 つの資源の枠。複製しても同じ資源を指す。
#[derive(Clone)]
pub struct PriorityPool {
    scheduler: Arc<Scheduler>,
    index: usize,
}

/// 枠を使っている間持つ。drop すると枠を返す。種類つきで取ったときは、返すときに終了を数える。
pub struct SchedulePermit {
    pool: PriorityPool,
    class: WorkClass,
    kind: Option<WorkKind>,
}

impl Drop for SchedulePermit {
    fn drop(&mut self) {
        self.pool.scheduler.state.lock().pools[self.pool.index].running[self.class.index()] -= 1;
        // 枠を返してから数えるので、終了を見た側は、枠がもう空いていると見てよい。
        if let Some(kind) = self.kind {
            self.pool.scheduler.finished[kind as usize].fetch_add(1, Ordering::Relaxed);
        }
        self.pool.scheduler.changed.notify_waiters();
    }
}

impl SchedulePermit {
    fn new(pool: PriorityPool, class: WorkClass, kind: Option<WorkKind>) -> Self {
        if let Some(kind) = kind {
            pool.scheduler.started[kind as usize].fetch_add(1, Ordering::Relaxed);
        }
        Self { pool, class, kind }
    }

    pub fn class(&self) -> WorkClass {
        self.class
    }

    /// 持っている枠のまま `f` を blocking thread で動かす。枠は `f` が本当に終わるまで返さない。
    /// 返ってきた future を drop しても、動き出した `f` は止まらず、枠もそのままである。
    pub async fn run_blocking<T, F>(self, f: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        self.run_blocking_with((), f).await
    }

    /// [`Self::run_blocking`] に加えて、`guards`（ほかの枠や許可）も `f` が終わるまで持つ。
    pub async fn run_blocking_with<G, T, F>(self, guards: G, f: F) -> Result<T>
    where
        G: Send + 'static,
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        let span = Span::current();
        tokio::task::spawn_blocking(move || {
            let _permit = self;
            let _guards = guards;
            span.in_scope(f)
        })
        .await
        .map_err(|error| Error::from_error(error, ErrorKind::TaskError))
    }
}

/// 枠を使わずに、動いているものとして数えている間持つ。
pub struct Occupancy {
    pool: PriorityPool,
    class: WorkClass,
}

impl Drop for Occupancy {
    fn drop(&mut self) {
        self.pool.scheduler.state.lock().pools[self.pool.index].occupied[self.class.index()] -= 1;
        self.pool.scheduler.changed.notify_waiters();
    }
}

/// 枠を待っている札。drop すると、待ち行列から外れる。
pub struct Ticket {
    pool: PriorityPool,
    id: u64,
    kind: Option<WorkKind>,
    queued: bool,
}

/// 待っている札の段を上げる、または札を捨てるための持ち手。複製できる。
/// 札はもう枠を得た（または外された）ときは、何もしない。
#[derive(Clone)]
pub struct RaiseHandle {
    pool: PriorityPool,
    id: u64,
}

impl RaiseHandle {
    /// 札を `target` の段へ上げる。待ち行列の中の順序（到着順）は保つ。すでに同じ段か上の段にいる、または待っていないときは何もしない。
    pub fn raise(&self, target: WorkClass) {
        let scheduler = &self.pool.scheduler;
        if scheduler.state.lock().pools[self.pool.index].raise(self.id, target) {
            scheduler.changed.notify_waiters();
        }
    }

    /// 札を待ち行列から外し、待っている [`Ticket::acquire`] を `Cancelled` で終わらせる。すでに枠を得ていたら何もしない。
    pub fn discard(&self) {
        let scheduler = &self.pool.scheduler;
        if scheduler.state.lock().pools[self.pool.index].remove(self.id) {
            scheduler.changed.notify_waiters();
        }
    }
}

impl Drop for Ticket {
    fn drop(&mut self) {
        if self.queued {
            self.pool.scheduler.state.lock().pools[self.pool.index].remove(self.id);
            self.pool.scheduler.changed.notify_waiters();
        }
    }
}

impl Ticket {
    pub fn handle(&self) -> RaiseHandle {
        RaiseHandle {
            pool: self.pool.clone(),
            id: self.id,
        }
    }

    /// 枠が空き、規則で始めてよくなるまで待つ。段は待っている間に上がることがある。
    /// 待つ間に drop されたら何も始まらない。[`RaiseHandle::discard`] で外されたら `Cancelled` を返す。
    pub async fn acquire(mut self) -> Result<SchedulePermit> {
        let scheduler = self.pool.scheduler.clone();
        loop {
            let mut changed = pin!(scheduler.changed.notified());
            changed.as_mut().enable();
            {
                let mut state = scheduler.state.lock();
                let Some(class) = state.pools[self.pool.index].class_of(self.id) else {
                    self.queued = false;
                    return Err(Error::new(ErrorKind::Cancelled).with_message("the waiter was discarded"));
                };
                if state.can_start(self.pool.index, class, Some(self.id)) {
                    let pool = &mut state.pools[self.pool.index];
                    pool.waiting[class.index()].pop_front();
                    pool.running[class.index()] += 1;
                    self.queued = false;
                    drop(state);
                    // 先頭が抜けたので、同じ段の次の待ちが始められるかもしれない。
                    scheduler.changed.notify_waiters();
                    return Ok(SchedulePermit::new(self.pool.clone(), class, self.kind));
                }
            }
            changed.await;
        }
    }
}

impl PriorityPool {
    /// 待ち行列に並ぶ。札を [`Ticket::acquire`] で待つ。
    pub fn enqueue(&self, class: WorkClass) -> Ticket {
        self.enqueue_as(class, None)
    }

    fn enqueue_as(&self, class: WorkClass, kind: Option<WorkKind>) -> Ticket {
        // 番号は lock の中で振る。lock の外で振ると、番号と行列の並びが食い違い、`raise` の到着順の挿入が崩れる。
        let mut state = self.scheduler.state.lock();
        let id = self.scheduler.next_waiter.fetch_add(1, Ordering::Relaxed);
        state.pools[self.index].waiting[class.index()].push_back(id);
        drop(state);
        Ticket {
            pool: self.clone(),
            id,
            kind,
            queued: true,
        }
    }

    /// 枠が空き、規則で始めてよくなるまで待つ。待つ間に drop されたら、何も始まらない。
    pub async fn acquire(&self, class: WorkClass) -> SchedulePermit {
        self.enqueue(class).acquire().await.expect("a waiter without a RaiseHandle is never discarded")
    }

    /// [`Self::acquire`] と同じで、`kind` の処理として数える。
    pub async fn acquire_counted(&self, class: WorkClass, kind: WorkKind) -> SchedulePermit {
        self.enqueue_as(class, Some(kind))
            .acquire()
            .await
            .expect("a waiter without a RaiseHandle is never discarded")
    }

    /// 待たずに始められるときだけ枠を取る。同じ段に待ちがあれば取らない。
    pub fn try_acquire(&self, class: WorkClass) -> Option<SchedulePermit> {
        self.try_acquire_as(class, None)
    }

    /// [`Self::try_acquire`] と同じで、`kind` の処理として数える。
    pub fn try_acquire_counted(&self, class: WorkClass, kind: WorkKind) -> Option<SchedulePermit> {
        self.try_acquire_as(class, Some(kind))
    }

    fn try_acquire_as(&self, class: WorkClass, kind: Option<WorkKind>) -> Option<SchedulePermit> {
        let mut state = self.scheduler.state.lock();
        if !state.can_start(self.index, class, None) {
            return None;
        }
        state.pools[self.index].running[class.index()] += 1;
        drop(state);
        Some(SchedulePermit::new(self.clone(), class, kind))
    }

    /// 現在の段で枠を取り、`f` を blocking thread で動かす。枠は `f` が本当に終わるまで返さない。
    pub async fn run_blocking<T, F>(&self, kind: WorkKind, f: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        self.run_blocking_as(WorkClass::current(), kind, f).await
    }

    /// [`Self::run_blocking`] を、段を指定して行う。
    pub async fn run_blocking_as<T, F>(&self, class: WorkClass, kind: WorkKind, f: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        self.acquire_counted(class, kind).await.run_blocking(f).await
    }

    /// 枠を使わずに、`class` の処理がこの資源で動いているものとして数える。
    /// 枠の外で動く処理（動画の変換など）をユーザーが待っている間、下位の段を保留するために使う。
    pub fn occupy(&self, class: WorkClass) -> Occupancy {
        self.scheduler.state.lock().pools[self.index].occupied[class.index()] += 1;
        Occupancy { pool: self.clone(), class }
    }

    /// 枠を使って動いている数。テストと計測に使う。
    pub fn running(&self, class: WorkClass) -> usize {
        self.scheduler.state.lock().pools[self.index].running[class.index()]
    }

    pub fn limits(&self) -> PoolLimits {
        self.scheduler.state.lock().pools[self.index].limits()
    }

    /// 枠を待っている数。テストと計測に使う。
    pub fn waiting(&self, class: WorkClass) -> usize {
        self.scheduler.state.lock().pools[self.index].waiting[class.index()].len()
    }

    /// `kind` の処理の（枠を得た数, 枠を返した数）。この資源を持つ [`Scheduler`] の全資源の合計である。
    pub fn counts(&self, kind: WorkKind) -> (usize, usize) {
        self.scheduler.counts(kind)
    }
}

/// `[scheduler]` の設定。画像の変換と外部プロセス（ffmpeg、heif-dec）の同時実行の枠の数。
/// External processes have two pools: `process_*` for the images that ffmpeg or heif-dec decode, and `video_*` for videos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduleOptions {
    /// 画像の変換の共有の枠。
    pub image_slots: usize,
    /// 画像の変換の、Interactive だけが使う枠。
    pub image_interactive_slots: usize,
    /// 外部プロセスの共有の枠。Images only (HEIF, AVIF, JXL, JP2); videos use `video_slots`.
    pub process_slots: usize,
    /// 外部プロセスの、Interactive だけが使う枠。
    pub process_interactive_slots: usize,
    /// The shared slots of ffmpeg runs for videos. They are apart from `process_slots`, so a slow video does not block image decodes.
    pub video_slots: usize,
    /// The slots only Interactive uses in the video pool.
    pub video_interactive_slots: usize,
}

impl Default for ScheduleOptions {
    fn default() -> Self {
        Self {
            image_slots: 4,
            image_interactive_slots: 1,
            process_slots: 2,
            process_interactive_slots: 1,
            video_slots: 1,
            video_interactive_slots: 1,
        }
    }
}

impl ScheduleOptions {
    pub fn validate(&self) -> Result<()> {
        if self.image_slots == 0 || self.process_slots == 0 || self.video_slots == 0 {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("scheduler image_slots, process_slots, and video_slots must be greater than zero"));
        }
        Ok(())
    }
}

/// サムネイルと表示用画像の生成が使う資源。
#[derive(Clone)]
pub struct MediaPools {
    /// 画像の変換（decode、縮小、encode）。
    pub images: PriorityPool,
    /// External processes for images: ffmpeg and heif-dec for HEIF, AVIF, JXL, and JP2.
    pub processes: PriorityPool,
    /// ffmpeg runs for videos: frame extraction, and what the video service runs. A slow video does not take a slot of `processes`.
    pub videos: PriorityPool,
}

impl MediaPools {
    pub fn new(scheduler: &Arc<Scheduler>, options: &ScheduleOptions) -> Self {
        // Background は今は使わないが、Background の処理から呼ばれても止まらないよう 1 つ持つ。
        Self {
            images: scheduler.pool(PoolLimits {
                shared: options.image_slots,
                interactive: options.image_interactive_slots,
                background: 1,
            }),
            processes: scheduler.pool(PoolLimits {
                shared: options.process_slots,
                interactive: options.process_interactive_slots,
                background: 1,
            }),
            videos: scheduler.pool(PoolLimits {
                shared: options.video_slots,
                interactive: options.video_interactive_slots,
                background: 1,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use testresult::TestResult;

    use super::*;

    const LIMITS: PoolLimits = PoolLimits {
        shared: 2,
        interactive: 1,
        background: 1,
    };

    fn pools() -> (PriorityPool, PriorityPool) {
        let scheduler = Arc::new(Scheduler::default());
        (scheduler.pool(LIMITS), scheduler.pool(LIMITS))
    }

    #[tokio::test]
    async fn a_video_permit_does_not_block_the_image_pools_test() -> TestResult {
        let pools = MediaPools::new(&Arc::new(Scheduler::default()), &ScheduleOptions::default());
        let _video = pools.videos.acquire(WorkClass::Foreground).await;
        // The video pool has one shared slot, and it is taken.
        assert!(pools.videos.try_acquire(WorkClass::Foreground).is_none());
        let first = pools.processes.try_acquire(WorkClass::Foreground);
        let second = pools.processes.try_acquire(WorkClass::Foreground);
        assert!(first.is_some() && second.is_some(), "both shared slots of the image process pool are free");
        assert!(pools.images.try_acquire(WorkClass::Foreground).is_some());
        Ok(())
    }

    #[test]
    fn defaults_and_validation_cover_the_video_pool_test() {
        let defaults = ScheduleOptions::default();
        assert_eq!((defaults.video_slots, defaults.video_interactive_slots), (1, 1));
        assert_eq!((defaults.process_slots, defaults.process_interactive_slots), (2, 1));
        assert!(defaults.validate().is_ok());
        assert!(ScheduleOptions { video_slots: 0, ..defaults }.validate().is_err());
    }

    async fn is_pending<F: Future>(future: std::pin::Pin<&mut F>) -> bool {
        futures::poll!(future).is_pending()
    }

    #[tokio::test]
    async fn interactive_uses_its_reserved_slot_when_the_shared_slots_are_full_test() -> TestResult {
        let (pool, _) = pools();
        let _a = pool.acquire(WorkClass::Foreground).await;
        let _b = pool.acquire(WorkClass::Foreground).await;
        let interactive = pool.acquire(WorkClass::Interactive).await;
        let second = pool.acquire(WorkClass::Interactive);
        let mut second = pin!(second);
        assert!(is_pending(second.as_mut()).await, "予約の枠は 1 つ");
        drop(interactive);
        let _second = second.await;
        Ok(())
    }

    #[tokio::test]
    async fn foreground_waits_while_interactive_runs_or_waits_in_the_same_pool_test() -> TestResult {
        let (pool, other) = pools();
        let interactive = pool.acquire(WorkClass::Interactive).await;
        let foreground = pool.acquire(WorkClass::Foreground);
        let mut foreground = pin!(foreground);
        assert!(is_pending(foreground.as_mut()).await, "空きがあっても Interactive が動いている間は始めない");
        let _elsewhere = other.try_acquire(WorkClass::Foreground).ok_or("別の資源の Foreground は止めない")?;
        drop(interactive);
        let _foreground = foreground.await;
        Ok(())
    }

    #[tokio::test]
    async fn background_waits_while_any_pool_has_higher_work_test() -> TestResult {
        let (pool, other) = pools();
        let foreground = other.acquire(WorkClass::Foreground).await;
        assert!(pool.try_acquire(WorkClass::Background).is_none());
        let background = pool.acquire(WorkClass::Background);
        let mut background = pin!(background);
        assert!(is_pending(background.as_mut()).await, "別の資源の Foreground でも Background を保留する");
        drop(foreground);
        let _background = background.await;
        Ok(())
    }

    #[tokio::test]
    async fn higher_classes_never_wait_for_background_slots_test() -> TestResult {
        let (pool, _) = pools();
        let _background = pool.acquire(WorkClass::Background).await;
        let _a = pool.try_acquire(WorkClass::Foreground).ok_or("Background は共有の枠を使わない")?;
        let _b = pool.try_acquire(WorkClass::Foreground).ok_or("Background は共有の枠を使わない")?;
        Ok(())
    }

    #[tokio::test]
    async fn waiters_of_one_class_start_in_order_and_a_dropped_waiter_leaves_the_queue_test() -> TestResult {
        let (pool, _) = pools();
        let _a = pool.acquire(WorkClass::Foreground).await;
        let b = pool.acquire(WorkClass::Foreground).await;
        let first = pool.acquire(WorkClass::Foreground);
        let second = pool.acquire(WorkClass::Foreground);
        let (mut first, mut second) = (Box::pin(first), pin!(second));
        assert!(is_pending(first.as_mut()).await);
        assert!(is_pending(second.as_mut()).await);
        assert!(pool.try_acquire(WorkClass::Foreground).is_none(), "待ちがあれば割り込まない");
        drop(first);
        drop(b);
        let _second = second.await;
        Ok(())
    }

    #[tokio::test]
    async fn occupancy_holds_lower_classes_without_using_a_slot_test() -> TestResult {
        let (pool, _) = pools();
        let occupancy = pool.occupy(WorkClass::Interactive);
        assert!(pool.try_acquire(WorkClass::Foreground).is_none());
        let _interactive = pool.try_acquire(WorkClass::Interactive).ok_or("枠は使っていない")?;
        drop(occupancy);
        drop(_interactive);
        let _foreground = pool.try_acquire(WorkClass::Foreground).ok_or("数えるのをやめたら始められる")?;
        Ok(())
    }

    #[tokio::test]
    async fn raising_a_waiter_moves_it_ahead_of_foreground_and_keeps_arrival_order_test() -> TestResult {
        let (pool, _) = pools();
        let _a = pool.acquire(WorkClass::Foreground).await;
        let _b = pool.acquire(WorkClass::Foreground).await;
        let _c = pool.acquire(WorkClass::Interactive).await;
        let first = pool.enqueue(WorkClass::Foreground);
        let second = pool.enqueue(WorkClass::Foreground);
        let handle = second.handle();
        let (mut first, mut second) = (Box::pin(first.acquire()), Box::pin(second.acquire()));
        assert!(is_pending(first.as_mut()).await);
        assert!(is_pending(second.as_mut()).await);
        handle.raise(WorkClass::Interactive);
        assert_eq!((pool.waiting(WorkClass::Interactive), pool.waiting(WorkClass::Foreground)), (1, 1));
        drop(_c);
        let second = second.await?;
        assert_eq!(second.class(), WorkClass::Interactive, "上げた札は Interactive の枠で始まる");
        assert!(is_pending(first.as_mut()).await, "Interactive が動いている間、Foreground は始まらない");
        Ok(())
    }

    #[tokio::test]
    async fn raising_a_granted_or_dropped_ticket_does_nothing_test() -> TestResult {
        let (pool, _) = pools();
        let ticket = pool.enqueue(WorkClass::Foreground);
        let handle = ticket.handle();
        let permit = ticket.acquire().await?;
        handle.raise(WorkClass::Interactive);
        handle.discard();
        assert_eq!(
            (pool.waiting(WorkClass::Interactive), pool.waiting(WorkClass::Foreground)),
            (0, 0),
            "枠を得た札を行列に戻さない"
        );
        assert_eq!(pool.running(WorkClass::Foreground), 1);
        drop(permit);
        assert!(pool.try_acquire(WorkClass::Background).is_some(), "取り残された待ちが Background を止めない");

        let ticket = pool.enqueue(WorkClass::Foreground);
        let handle = ticket.handle();
        drop(ticket);
        handle.raise(WorkClass::Interactive);
        assert_eq!(pool.waiting(WorkClass::Interactive), 0);
        Ok(())
    }

    #[tokio::test]
    async fn discarding_a_waiter_ends_its_acquire_and_leaves_the_queue_test() -> TestResult {
        let (pool, _) = pools();
        let _a = pool.acquire(WorkClass::Foreground).await;
        let _b = pool.acquire(WorkClass::Foreground).await;
        let ticket = pool.enqueue(WorkClass::Foreground);
        let handle = ticket.handle();
        let mut waiting = Box::pin(ticket.acquire());
        assert!(is_pending(waiting.as_mut()).await);
        handle.discard();
        let error = tokio::time::timeout(std::time::Duration::from_millis(100), waiting).await?.err().ok_or("discarded")?;
        assert_eq!(error.kind(), &ErrorKind::Cancelled);
        assert_eq!(pool.waiting(WorkClass::Foreground), 0);
        Ok(())
    }

    #[tokio::test]
    async fn run_blocking_keeps_the_permit_and_the_count_until_the_job_ends_test() -> TestResult {
        let scheduler = Arc::new(Scheduler::default());
        let pool = scheduler.pool(LIMITS);
        let (started, ready) = tokio::sync::oneshot::channel();
        let (release, wait) = std::sync::mpsc::channel::<()>();
        let job = {
            let pool = pool.clone();
            tokio::spawn(async move {
                pool.run_blocking_as(WorkClass::Foreground, WorkKind::Exif, move || {
                    started.send(()).unwrap();
                    wait.recv().unwrap();
                })
                .await
            })
        };
        ready.await?;
        job.abort();
        let _ = job.await;
        assert_eq!(scheduler.counts(WorkKind::Exif), (1, 0), "future を捨てても worker は動いている");
        assert_eq!(pool.running(WorkClass::Foreground), 1);
        release.send(())?;
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while scheduler.counts(WorkKind::Exif).1 == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await?;
        assert_eq!(scheduler.counts(WorkKind::Exif), (1, 1));
        assert_eq!(pool.running(WorkClass::Foreground), 0);
        Ok(())
    }

    #[tokio::test]
    async fn run_blocking_uses_the_current_class_and_passes_the_value_test() -> TestResult {
        let (pool, _) = pools();
        let value = WorkClass::Background.scope(pool.run_blocking(WorkKind::Summary, WorkClass::current)).await?;
        assert_eq!(value, WorkClass::Foreground, "blocking thread には段を引き継がない");
        let (running, ()) = WorkClass::Background
            .scope(async {
                let permit = pool.acquire_counted(WorkClass::current(), WorkKind::Summary).await;
                (permit.class(), ())
            })
            .await;
        assert_eq!(running, WorkClass::Background);
        Ok(())
    }

    #[tokio::test]
    async fn spawn_carries_the_class_test() -> TestResult {
        let class = WorkClass::Background.scope(async { WorkClass::spawn(async { WorkClass::current() }).await }).await?;
        assert_eq!(class, WorkClass::Background);
        assert_eq!(WorkClass::Interactive.spawn_as(async { WorkClass::current() }).await?, WorkClass::Interactive);
        Ok(())
    }

    #[tokio::test]
    async fn raised_waiters_start_in_arrival_order_in_the_target_class_test() -> TestResult {
        let (pool, _) = pools();
        let _a = pool.acquire(WorkClass::Foreground).await;
        let _b = pool.acquire(WorkClass::Foreground).await;
        let reserved = pool.acquire(WorkClass::Interactive).await;
        let first = pool.enqueue(WorkClass::Foreground);
        let second = pool.enqueue(WorkClass::Foreground);
        let third = pool.enqueue(WorkClass::Foreground);
        // 後から並んだ札を先に上げても、行列の中では到着順に並ぶ。
        third.handle().raise(WorkClass::Interactive);
        second.handle().raise(WorkClass::Interactive);
        let (mut first, mut second, mut third) = (Box::pin(first.acquire()), Box::pin(second.acquire()), Box::pin(third.acquire()));
        assert!(is_pending(first.as_mut()).await);
        assert!(is_pending(second.as_mut()).await);
        assert!(is_pending(third.as_mut()).await);
        drop(reserved);
        let second = second.await?;
        assert!(is_pending(third.as_mut()).await, "Interactive の先頭は先に並んだ札");
        assert!(is_pending(first.as_mut()).await, "Interactive が待つ間、Foreground は始まらない");
        drop(second);
        let _third = third.await?;
        Ok(())
    }

    #[tokio::test]
    async fn discarding_or_raising_the_front_waiter_lets_the_next_one_start_test() -> TestResult {
        let (pool, _) = pools();
        let _a = pool.acquire(WorkClass::Foreground).await;
        let hold = pool.acquire(WorkClass::Foreground).await;
        let front = pool.enqueue(WorkClass::Foreground);
        let next = pool.enqueue(WorkClass::Foreground);
        let handle = front.handle();
        let (mut front, mut next) = (Box::pin(front.acquire()), Box::pin(next.acquire()));
        assert!(is_pending(front.as_mut()).await);
        assert!(is_pending(next.as_mut()).await);
        drop(hold);
        // 枠が空いたので先頭が始まる。先頭を外せば、次が始まる。
        handle.discard();
        assert!(front.await.is_err());
        let _next = tokio::time::timeout(std::time::Duration::from_millis(500), next).await??;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn waiter_numbers_follow_the_queue_order_when_threads_enqueue_together_test() -> TestResult {
        let scheduler = Arc::new(Scheduler::default());
        let pool = scheduler.pool(LIMITS);
        let mut tasks = Vec::new();
        for _ in 0..8 {
            let pool = pool.clone();
            tasks.push(tokio::spawn(async move { (0..200).map(|_| pool.enqueue(WorkClass::Foreground)).collect::<Vec<_>>() }));
        }
        let mut tickets = Vec::new();
        for task in tasks {
            tickets.extend(task.await?);
        }
        let state = scheduler.state.lock();
        let queue = &state.pools[pool.index].waiting[WorkClass::Foreground.index()];
        assert_eq!(queue.len(), 1600);
        assert!(queue.iter().is_sorted(), "行列は番号の昇順である");
        drop(state);
        drop(tickets);
        Ok(())
    }

    #[tokio::test]
    async fn run_blocking_with_keeps_the_guards_until_the_job_ends_test() -> TestResult {
        let (pool, _) = pools();
        let guard = Arc::new(());
        let (started, ready) = tokio::sync::oneshot::channel();
        let (release, wait) = std::sync::mpsc::channel::<()>();
        let permit = pool.acquire(WorkClass::Foreground).await;
        let job = tokio::spawn(permit.run_blocking_with(guard.clone(), move || {
            started.send(()).unwrap();
            wait.recv().unwrap();
        }));
        ready.await?;
        job.abort();
        let _ = job.await;
        assert_eq!(Arc::strong_count(&guard), 2, "future を捨てても guard は job が持つ");
        release.send(())?;
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while Arc::strong_count(&guard) > 1 {
                tokio::task::yield_now().await;
            }
        })
        .await?;
        Ok(())
    }
}
