//! 優先度つきの同時実行の枠と、その上に載る一覧・背景処理の資源。
//!
//! 処理は 3 つの段（[`WorkClass`]）のどれかに属する。資源ごとに [`WorkPriorityPool`] を 1 つ持ち、次の規則で開始を決める。
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
//! Background を保留するかどうかは、枠を使っている処理と待ちに加えて、[`WorkPriorityPool::occupy`] で数えているものも見る。
//!
//! 待ちの札（[`WorkTicket`]）の状態は、待ち行列に入っているかどうかだけで決まる。次の 2 つを守る。
//!
//! - 起こし損ねない。状態が変わって、待っている側が始められるようになり得るときは、必ず通知する。
//!   待つ側は、通知を受ける用意（`enable`）をしてから状態を確かめる。
//! - 誰も待っていない札が行列に残らない。行列から外すのは、枠を得た札、drop された札、[`WorkRaiseHandle::discard`] された札だけで、
//!   すべて 1 つの lock の中で行う。残った札は、その段の保留を止め続けてしまう。
//!
//! 一覧の io と stream の 2 つの資源と、処理の種類ごとの回数もここに持つ。HTTP の受付数や他の storage 操作は制限しない。
use std::{
    collections::VecDeque,
    future::Future,
    pin::pin,
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
};

use tokio::sync::Notify;
use tracing::{Instrument as _, Span};

use crate::prelude::*;

/// 処理の段。上の段ほど先に始まる（[`WorkScheduler`]）。
///
/// 段は処理を始める時点で、呼び出した側が明示的に渡す。暗黙に引き継ぐ値は持たないので、
/// storage の読み取りや外部プロセスの起動はすべて `class` を引数に取る。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WorkClass {
    /// ユーザーが今まさに待っている 1 件。
    Interactive,
    /// 一覧、検索、グリッドのサムネイルなど、画面に出す処理。
    Foreground,
    /// timeline の取り込みなど、待たれていない処理。
    Background,
}
impl WorkClass {
    /// 上の段から順に並べたすべての段。
    pub const ALL: [Self; 3] = [Self::Interactive, Self::Foreground, Self::Background];
}

#[derive(Clone, Copy, Debug)]
#[repr(usize)]
pub enum WorkKind {
    Listing,
    Exif,
    Probe,
    Archive,
    Summary,
    ChildProbe,
    /// 画像の変換（decode、縮小、encode）。
    Convert,
    /// 画像の寸法や EXIF の読み取り。
    Inspect,
}
impl WorkKind {
    pub const COUNT: usize = 8;
}

pub struct WorkBudget {
    /// ファイルを読む処理（一覧の走査、EXIF の読み取り、archive の探査など）の資源。
    io: WorkPriorityPool,
    streams: WorkPriorityPool,
}

/// 子の有無の探査が取る 2 つの枠。`worker` は属性の取得、`stream` は stream の受付である。
pub struct WorkProbe {
    pub worker: WorkPermit,
    pub stream: WorkPermit,
}

impl Default for WorkBudget {
    fn default() -> Self {
        Self::new(&Arc::new(WorkScheduler::default()))
    }
}
impl WorkBudget {
    /// ファイルを読む処理。Interactive と Foreground で 3、Background に 1。
    const IO_LIMITS: WorkPoolLimits = WorkPoolLimits {
        shared: 3,
        interactive: 0,
        background: 1,
    };
    /// 一覧の stream の受付。Interactive と Foreground で 12、Background に 4。
    const STREAM_LIMITS: WorkPoolLimits = WorkPoolLimits {
        shared: 12,
        interactive: 0,
        background: 4,
    };

    /// `scheduler` の資源として、一覧の io と stream の枠を足す。
    pub fn new(scheduler: &Arc<WorkScheduler>) -> Self {
        Self {
            io: scheduler.pool(Self::IO_LIMITS),
            streams: scheduler.pool(Self::STREAM_LIMITS),
        }
    }
    pub fn io(&self) -> &WorkPriorityPool {
        &self.io
    }
    /// stream の受付の資源。
    pub fn streams(&self) -> &WorkPriorityPool {
        &self.streams
    }
    /// 待たずに始められるときだけ、子の有無の探査の枠を取る。Background の io か stream に待ちがあれば取らない。
    pub fn try_probe(&self) -> Option<WorkProbe> {
        let stream = self.streams.try_acquire(WorkClass::Background)?;
        let worker = self.io.try_acquire_counted(WorkClass::Background, WorkKind::ChildProbe)?;
        Some(WorkProbe { worker, stream })
    }
    pub fn counts(&self, kind: WorkKind) -> (usize, usize) {
        self.io.counts(kind)
    }
    /// Background の処理として動くときだけ、その処理の枠を取る。
    pub async fn background(&self, class: WorkClass, kind: WorkKind) -> Result<Option<WorkPermit>> {
        if class == WorkClass::Background {
            Ok(Some(self.io.ticket(class).counted(kind).acquire().await?))
        } else {
            Ok(None)
        }
    }
}

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
pub struct WorkPoolLimits {
    /// Interactive と Foreground が使う枠。
    pub shared: usize,
    /// 共有の枠とは別に、Interactive だけが使う枠。
    pub interactive: usize,
    /// Background だけが使う枠。
    pub background: usize,
}

#[derive(Default)]
struct WorkPoolState {
    limits: Option<WorkPoolLimits>,
    /// 枠を使って動いている数。
    running: [usize; CLASSES],
    /// 枠を使わずに、動いているものとして数える数（[`WorkPriorityPool::occupy`]）。
    occupied: [usize; CLASSES],
    waiting: [VecDeque<u64>; CLASSES],
}

impl WorkPoolState {
    fn limits(&self) -> WorkPoolLimits {
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

    fn busy(&self, class: WorkClass) -> bool {
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

struct WorkSchedulerState {
    pools: Vec<WorkPoolState>,
}

impl WorkSchedulerState {
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
            WorkClass::Foreground => state.busy(WorkClass::Interactive),
            WorkClass::Background => self.pools.iter().any(|pool| pool.busy(WorkClass::Interactive) || pool.busy(WorkClass::Foreground)),
        };
        first && !held && state.has_room(class)
    }
}

/// すべての資源の枠と、その待ちを持つ。Background の保留はすべての資源を見るので、1 つの lock で守る。
pub struct WorkScheduler {
    state: Mutex<WorkSchedulerState>,
    wake: Notify,
    next_ticket: AtomicU64,
    /// 処理の種類ごとの、枠を得た数と返した数。すべての資源を合わせて数える。
    started: [AtomicUsize; WorkKind::COUNT],
    finished: [AtomicUsize; WorkKind::COUNT],
}

impl Default for WorkScheduler {
    fn default() -> Self {
        Self {
            state: Mutex::new(WorkSchedulerState { pools: Vec::new() }),
            wake: Notify::new(),
            next_ticket: AtomicU64::new(0),
            started: std::array::from_fn(|_| AtomicUsize::new(0)),
            finished: std::array::from_fn(|_| AtomicUsize::new(0)),
        }
    }
}

impl WorkScheduler {
    /// 資源を 1 つ足す。
    pub fn pool(self: &Arc<Self>, limits: WorkPoolLimits) -> WorkPriorityPool {
        let mut state = self.state.lock();
        state.pools.push(WorkPoolState {
            limits: Some(limits),
            ..Default::default()
        });
        WorkPriorityPool {
            scheduler: self.clone(),
            index: state.pools.len() - 1,
        }
    }

    /// `kind` の処理の（枠を得た数, 枠を返した数）。枠を返すのは、処理が本当に終わったときである。
    pub fn counts(&self, kind: WorkKind) -> (usize, usize) {
        (self.started[kind as usize].load(Ordering::Relaxed), self.finished[kind as usize].load(Ordering::Relaxed))
    }

    /// Runs `f` on a blocking thread while holding `guards` until it really ends.
    /// Dropping the returned future does not stop `f` or release the guards. The span is carried over.
    pub async fn run_blocking_with<G, T, F>(guards: G, f: F) -> Result<T>
    where
        G: Send + 'static,
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        let span = Span::current();
        tokio::task::spawn_blocking(move || {
            let _guards = guards;
            span.in_scope(f)
        })
        .await
        .map_err(|error| Error::from_error(error, ErrorKind::TaskError))
    }
}

/// 1 つの資源の枠。複製しても同じ資源を指す。
#[derive(Clone)]
pub struct WorkPriorityPool {
    scheduler: Arc<WorkScheduler>,
    index: usize,
}

/// 枠を使っている間持つ。drop すると枠を返す。種類つきで取ったときは、返すときに終了を数える。
pub struct WorkPermit {
    pool: WorkPriorityPool,
    class: WorkClass,
    kind: Option<WorkKind>,
}

impl Drop for WorkPermit {
    fn drop(&mut self) {
        self.pool.scheduler.state.lock().pools[self.pool.index].running[self.class.index()] -= 1;
        // 枠を返してから数えるので、終了を見た側は、枠がもう空いていると見てよい。
        if let Some(kind) = self.kind {
            self.pool.scheduler.finished[kind as usize].fetch_add(1, Ordering::Relaxed);
        }
        self.pool.scheduler.wake.notify_waiters();
    }
}

impl WorkPermit {
    fn new(pool: WorkPriorityPool, class: WorkClass, kind: Option<WorkKind>) -> Self {
        if let Some(kind) = kind {
            pool.scheduler.started[kind as usize].fetch_add(1, Ordering::Relaxed);
        }
        Self { pool, class, kind }
    }

    pub fn class(&self) -> WorkClass {
        self.class
    }

    /// 持っている枠のまま `future` を task として動かす。枠は `future` が本当に終わるまで返さない。
    /// 返ってきた future を drop しても、動き出した `future` は止まらず、枠もそのままである。段と Span は引き継ぐ。
    pub async fn run<T, F>(self, future: F) -> Result<T>
    where
        T: Send + 'static,
        F: Future<Output = T> + Send + 'static,
    {
        self.run_with((), future).await
    }

    /// [`Self::run`] に加えて、`guards`（ほかの枠や許可）も `future` が終わるまで持つ。
    pub async fn run_with<G, T, F>(self, guards: G, future: F) -> Result<T>
    where
        G: Send + 'static,
        T: Send + 'static,
        F: Future<Output = T> + Send + 'static,
    {
        tokio::spawn(
            async move {
                let _permit = self;
                let _guards = guards;
                future.await
            }
            .in_current_span(),
        )
        .await
        .map_err(|error| Error::from_error(error, ErrorKind::TaskError))
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
        WorkScheduler::run_blocking_with((self, guards), f).await
    }
}

/// 枠を使わずに、動いているものとして数えている間持つ。
pub struct WorkOccupancy {
    pool: WorkPriorityPool,
    class: WorkClass,
}

impl Drop for WorkOccupancy {
    fn drop(&mut self) {
        self.pool.scheduler.state.lock().pools[self.pool.index].occupied[self.class.index()] -= 1;
        self.pool.scheduler.wake.notify_waiters();
    }
}

/// 枠を待っている札。drop すると、待ち行列から外れる。
pub struct WorkTicket {
    pool: WorkPriorityPool,
    id: u64,
    kind: Option<WorkKind>,
    queued: bool,
}

/// 待っている札の段を上げる、または札を捨てるための持ち手。複製できる。
/// 札はもう枠を得た（または外された）ときは、何もしない。
#[derive(Clone)]
pub struct WorkRaiseHandle {
    pool: WorkPriorityPool,
    id: u64,
}

impl WorkRaiseHandle {
    /// 札を `target` の段へ上げる。待ち行列の中の順序（到着順）は保つ。すでに同じ段か上の段にいる、または待っていないときは何もしない。
    pub fn raise(&self, target: WorkClass) {
        let scheduler = &self.pool.scheduler;
        if scheduler.state.lock().pools[self.pool.index].raise(self.id, target) {
            scheduler.wake.notify_waiters();
        }
    }

    /// 札を待ち行列から外し、待っている [`WorkTicket::acquire`] を `Cancelled` で終わらせる。すでに枠を得ていたら何もしない。
    pub fn discard(&self) {
        let scheduler = &self.pool.scheduler;
        if scheduler.state.lock().pools[self.pool.index].remove(self.id) {
            scheduler.wake.notify_waiters();
        }
    }
}

impl Drop for WorkTicket {
    fn drop(&mut self) {
        if self.queued {
            self.pool.scheduler.state.lock().pools[self.pool.index].remove(self.id);
            self.pool.scheduler.wake.notify_waiters();
        }
    }
}

impl WorkTicket {
    /// `kind` の処理として数える。`acquire` の前に付ける。
    pub fn counted(mut self, kind: WorkKind) -> Self {
        self.kind = Some(kind);
        self
    }

    pub fn handle(&self) -> WorkRaiseHandle {
        WorkRaiseHandle {
            pool: self.pool.clone(),
            id: self.id,
        }
    }

    /// 枠が空き、規則で始めてよくなるまで待つ。段は待っている間に上がることがある。
    /// 待つ間に drop されたら何も始まらない。[`WorkRaiseHandle::discard`] で外されたら `Cancelled` を返す。
    pub async fn acquire(mut self) -> Result<WorkPermit> {
        let scheduler = self.pool.scheduler.clone();
        loop {
            let mut wake = pin!(scheduler.wake.notified());
            wake.as_mut().enable();
            {
                let mut state = scheduler.state.lock();
                let Some(class) = state.pools[self.pool.index].class_of(self.id) else {
                    self.queued = false;
                    return Err(Error::new(ErrorKind::Cancelled).with_message("the ticket was discarded"));
                };
                if state.can_start(self.pool.index, class, Some(self.id)) {
                    let pool = &mut state.pools[self.pool.index];
                    pool.waiting[class.index()].pop_front();
                    pool.running[class.index()] += 1;
                    self.queued = false;
                    drop(state);
                    // 先頭が抜けたので、同じ段の次の待ちが始められるかもしれない。
                    scheduler.wake.notify_waiters();
                    return Ok(WorkPermit::new(self.pool.clone(), class, self.kind));
                }
            }
            wake.await;
        }
    }
}

impl WorkPriorityPool {
    /// 待ちの札を発行する。種類を数えるときは [`WorkTicket::counted`]、待つときは [`WorkTicket::acquire`] を使う。
    pub fn ticket(&self, class: WorkClass) -> WorkTicket {
        // 番号は lock の中で振る。lock の外で振ると、番号と行列の並びが食い違い、`raise` の到着順の挿入が崩れる。
        let mut state = self.scheduler.state.lock();
        let id = self.scheduler.next_ticket.fetch_add(1, Ordering::Relaxed);
        state.pools[self.index].waiting[class.index()].push_back(id);
        drop(state);
        WorkTicket {
            pool: self.clone(),
            id,
            kind: None,
            queued: true,
        }
    }

    /// 待たずに始められるときだけ枠を取る。同じ段に待ちがあれば取らない。
    pub fn try_acquire(&self, class: WorkClass) -> Option<WorkPermit> {
        self.try_acquire_as(class, None)
    }

    /// [`Self::try_acquire`] と同じで、`kind` の処理として数える。子の有無の探査だけが使う。
    fn try_acquire_counted(&self, class: WorkClass, kind: WorkKind) -> Option<WorkPermit> {
        self.try_acquire_as(class, Some(kind))
    }

    fn try_acquire_as(&self, class: WorkClass, kind: Option<WorkKind>) -> Option<WorkPermit> {
        let mut state = self.scheduler.state.lock();
        if !state.can_start(self.index, class, None) {
            return None;
        }
        state.pools[self.index].running[class.index()] += 1;
        drop(state);
        Some(WorkPermit::new(self.clone(), class, kind))
    }

    /// 枠を使わずに、`class` の処理がこの資源で動いているものとして数える。
    /// 枠の外で動く処理（動画の変換など）をユーザーが待っている間、下位の段を保留するために使う。
    pub fn occupy(&self, class: WorkClass) -> WorkOccupancy {
        self.scheduler.state.lock().pools[self.index].occupied[class.index()] += 1;
        WorkOccupancy { pool: self.clone(), class }
    }

    /// 枠を使って動いている数。テストと計測に使う。
    pub fn running(&self, class: WorkClass) -> usize {
        self.scheduler.state.lock().pools[self.index].running[class.index()]
    }

    pub fn limits(&self) -> WorkPoolLimits {
        self.scheduler.state.lock().pools[self.index].limits()
    }

    /// 枠を待っている数。テストと計測に使う。
    pub fn waiting(&self, class: WorkClass) -> usize {
        self.scheduler.state.lock().pools[self.index].waiting[class.index()].len()
    }

    /// `kind` の処理の（枠を得た数, 枠を返した数）。この資源を持つ [`WorkScheduler`] の全資源の合計である。
    pub fn counts(&self, kind: WorkKind) -> (usize, usize) {
        self.scheduler.counts(kind)
    }
}

/// `[scheduler]` の設定。画像の変換と外部プロセス（ffmpeg、heif-dec）の同時実行の枠の数。
/// `image_*` は `cpu` の枠、`process_*` は `process_image` の枠、`video_*` は `process_video` の枠である。
/// External processes have two pools: `process_*` for the images that ffmpeg or heif-dec decode, and `video_*` for videos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkOptions {
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

impl Default for WorkOptions {
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

impl WorkOptions {
    pub fn validate(&self) -> Result<()> {
        if self.image_slots == 0 || self.process_slots == 0 || self.video_slots == 0 {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("scheduler image_slots, process_slots, and video_slots must be greater than zero"));
        }
        Ok(())
    }
}

/// 処理の中身（資源の種類）ごとに分けた、サムネイルと表示用画像の生成が使う資源。
/// 一覧の走査や EXIF の読み取りなど、ファイルを読む処理の資源（`io`）は [`WorkBudget`] が持つ。
#[derive(Clone)]
pub struct WorkPools {
    /// CPU を使う処理: 画像の変換（decode、縮小、encode）。
    pub cpu: WorkPriorityPool,
    /// External processes for images: ffmpeg and heif-dec for HEIF, AVIF, JXL, and JP2.
    pub process_image: WorkPriorityPool,
    /// ffmpeg runs for videos: frame extraction, and what the video service runs. A slow video does not take a slot of `process_image`.
    pub process_video: WorkPriorityPool,
}

impl WorkPools {
    pub fn new(scheduler: &Arc<WorkScheduler>, options: &WorkOptions) -> Self {
        // Background は今は使わないが、Background の処理から呼ばれても止まらないよう 1 つ持つ。
        Self {
            cpu: scheduler.pool(WorkPoolLimits {
                shared: options.image_slots,
                interactive: options.image_interactive_slots,
                background: 1,
            }),
            process_image: scheduler.pool(WorkPoolLimits {
                shared: options.process_slots,
                interactive: options.process_interactive_slots,
                background: 1,
            }),
            process_video: scheduler.pool(WorkPoolLimits {
                shared: options.video_slots,
                interactive: options.video_interactive_slots,
                background: 1,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testresult::TestResult;

    const LIMITS: WorkPoolLimits = WorkPoolLimits {
        shared: 2,
        interactive: 1,
        background: 1,
    };

    fn pools() -> (WorkPriorityPool, WorkPriorityPool) {
        let scheduler = Arc::new(WorkScheduler::default());
        (scheduler.pool(LIMITS), scheduler.pool(LIMITS))
    }

    #[tokio::test]
    async fn a_video_permit_does_not_block_the_image_pools_test() -> TestResult {
        let pools = WorkPools::new(&Arc::new(WorkScheduler::default()), &WorkOptions::default());
        let _video = pools.process_video.ticket(WorkClass::Foreground).acquire().await?;
        // The video pool has one shared slot, and it is taken.
        assert!(pools.process_video.try_acquire(WorkClass::Foreground).is_none());
        let first = pools.process_image.try_acquire(WorkClass::Foreground);
        let second = pools.process_image.try_acquire(WorkClass::Foreground);
        assert!(first.is_some() && second.is_some(), "both shared slots of the image process pool are free");
        assert!(pools.cpu.try_acquire(WorkClass::Foreground).is_some());
        Ok(())
    }

    #[test]
    fn defaults_and_validation_cover_the_video_pool_test() {
        let defaults = WorkOptions::default();
        assert_eq!((defaults.video_slots, defaults.video_interactive_slots), (1, 1));
        assert_eq!((defaults.process_slots, defaults.process_interactive_slots), (2, 1));
        assert!(defaults.validate().is_ok());
        assert!(WorkOptions { video_slots: 0, ..defaults }.validate().is_err());
    }

    async fn is_pending<F: Future>(future: std::pin::Pin<&mut F>) -> bool {
        futures::poll!(future).is_pending()
    }

    #[tokio::test]
    async fn interactive_uses_its_reserved_slot_when_the_shared_slots_are_full_test() -> TestResult {
        let (pool, _) = pools();
        let _a = pool.ticket(WorkClass::Foreground).acquire().await?;
        let _b = pool.ticket(WorkClass::Foreground).acquire().await?;
        let interactive = pool.ticket(WorkClass::Interactive).acquire().await?;
        let second = pool.ticket(WorkClass::Interactive).acquire();
        let mut second = pin!(second);
        assert!(is_pending(second.as_mut()).await, "予約の枠は 1 つ");
        drop(interactive);
        let _second = second.await;
        Ok(())
    }

    #[tokio::test]
    async fn foreground_waits_while_interactive_runs_or_waits_in_the_same_pool_test() -> TestResult {
        let (pool, other) = pools();
        let interactive = pool.ticket(WorkClass::Interactive).acquire().await?;
        let foreground = pool.ticket(WorkClass::Foreground).acquire();
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
        let foreground = other.ticket(WorkClass::Foreground).acquire().await?;
        assert!(pool.try_acquire(WorkClass::Background).is_none());
        let background = pool.ticket(WorkClass::Background).acquire();
        let mut background = pin!(background);
        assert!(is_pending(background.as_mut()).await, "別の資源の Foreground でも Background を保留する");
        drop(foreground);
        let _background = background.await;
        Ok(())
    }

    #[tokio::test]
    async fn higher_classes_never_wait_for_background_slots_test() -> TestResult {
        let (pool, _) = pools();
        let _background = pool.ticket(WorkClass::Background).acquire().await?;
        let _a = pool.try_acquire(WorkClass::Foreground).ok_or("Background は共有の枠を使わない")?;
        let _b = pool.try_acquire(WorkClass::Foreground).ok_or("Background は共有の枠を使わない")?;
        Ok(())
    }

    #[tokio::test]
    async fn waiters_of_one_class_start_in_order_and_a_dropped_waiter_leaves_the_queue_test() -> TestResult {
        let (pool, _) = pools();
        let _a = pool.ticket(WorkClass::Foreground).acquire().await?;
        let b = pool.ticket(WorkClass::Foreground).acquire().await?;
        let first = pool.ticket(WorkClass::Foreground).acquire();
        let second = pool.ticket(WorkClass::Foreground).acquire();
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
        let _a = pool.ticket(WorkClass::Foreground).acquire().await?;
        let _b = pool.ticket(WorkClass::Foreground).acquire().await?;
        let _c = pool.ticket(WorkClass::Interactive).acquire().await?;
        let first = pool.ticket(WorkClass::Foreground);
        let second = pool.ticket(WorkClass::Foreground);
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
        let ticket = pool.ticket(WorkClass::Foreground);
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

        let ticket = pool.ticket(WorkClass::Foreground);
        let handle = ticket.handle();
        drop(ticket);
        handle.raise(WorkClass::Interactive);
        assert_eq!(pool.waiting(WorkClass::Interactive), 0);
        Ok(())
    }

    #[tokio::test]
    async fn discarding_a_waiter_ends_its_acquire_and_leaves_the_queue_test() -> TestResult {
        let (pool, _) = pools();
        let _a = pool.ticket(WorkClass::Foreground).acquire().await?;
        let _b = pool.ticket(WorkClass::Foreground).acquire().await?;
        let ticket = pool.ticket(WorkClass::Foreground);
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
        let scheduler = Arc::new(WorkScheduler::default());
        let pool = scheduler.pool(LIMITS);
        let (started, ready) = tokio::sync::oneshot::channel();
        let (release, wait) = std::sync::mpsc::channel::<()>();
        let job = {
            let pool = pool.clone();
            tokio::spawn(async move {
                pool.ticket(WorkClass::Foreground)
                    .counted(WorkKind::Exif)
                    .acquire()
                    .await
                    .expect("the pool has a slot")
                    .run_blocking(move || {
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
    async fn a_ticket_keeps_the_class_it_was_issued_with_test() -> TestResult {
        let (pool, _) = pools();
        let permit = pool.ticket(WorkClass::Background).counted(WorkKind::Summary).acquire().await?;
        assert_eq!(permit.class(), WorkClass::Background);
        let value = permit.run_blocking(|| 7).await?;
        assert_eq!(value, 7);
        Ok(())
    }

    #[tokio::test]
    async fn raised_waiters_start_in_arrival_order_in_the_target_class_test() -> TestResult {
        let (pool, _) = pools();
        let _a = pool.ticket(WorkClass::Foreground).acquire().await?;
        let _b = pool.ticket(WorkClass::Foreground).acquire().await?;
        let reserved = pool.ticket(WorkClass::Interactive).acquire().await?;
        let first = pool.ticket(WorkClass::Foreground);
        let second = pool.ticket(WorkClass::Foreground);
        let third = pool.ticket(WorkClass::Foreground);
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
        let _a = pool.ticket(WorkClass::Foreground).acquire().await?;
        let hold = pool.ticket(WorkClass::Foreground).acquire().await?;
        let front = pool.ticket(WorkClass::Foreground);
        let next = pool.ticket(WorkClass::Foreground);
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
    async fn waiter_numbers_follow_the_queue_order_when_threads_issue_tickets_together_test() -> TestResult {
        let scheduler = Arc::new(WorkScheduler::default());
        let pool = scheduler.pool(LIMITS);
        let mut tasks = Vec::new();
        for _ in 0..8 {
            let pool = pool.clone();
            tasks.push(tokio::spawn(async move { (0..200).map(|_| pool.ticket(WorkClass::Foreground)).collect::<Vec<_>>() }));
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
        let permit = pool.ticket(WorkClass::Foreground).acquire().await?;
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

    #[tokio::test]
    async fn run_keeps_the_permit_and_the_guards_until_the_future_ends_test() -> TestResult {
        let scheduler = Arc::new(WorkScheduler::default());
        let pool = scheduler.pool(LIMITS);
        let guard = Arc::new(());
        let (started, ready) = tokio::sync::oneshot::channel();
        let (release, wait) = tokio::sync::oneshot::channel::<()>();
        let permit = pool.ticket(WorkClass::Background).counted(WorkKind::Probe).acquire().await?;
        let class = permit.class();
        let job = tokio::spawn(permit.run_with(guard.clone(), async move {
            started.send(()).unwrap();
            wait.await.unwrap();
        }));
        ready.await?;
        assert_eq!(class, WorkClass::Background);
        job.abort();
        let _ = job.await;
        assert_eq!(scheduler.counts(WorkKind::Probe), (1, 0), "呼んだ側を捨てても task は動いている");
        assert_eq!(Arc::strong_count(&guard), 2);
        release.send(()).map_err(|()| "the job was dropped")?;
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while scheduler.counts(WorkKind::Probe).1 == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await?;
        assert_eq!(pool.running(WorkClass::Background), 0);
        assert_eq!(Arc::strong_count(&guard), 1);
        Ok(())
    }

    #[tokio::test]
    async fn background_runs_beside_foreground_only_in_its_own_slot_test() -> TestResult {
        let budget = Arc::new(WorkBudget::default());
        let background = budget.io().ticket(WorkClass::Background).counted(WorkKind::Exif).acquire().await?;
        let mut foreground = Vec::new();
        for _ in 0..3 {
            foreground.push(budget.io().ticket(WorkClass::Foreground).counted(WorkKind::Listing).acquire().await?);
        }
        let next_fg = budget.io().ticket(WorkClass::Foreground).counted(WorkKind::Listing).acquire();
        let next_bg = budget.io().ticket(WorkClass::Background).counted(WorkKind::Probe).acquire();
        tokio::pin!(next_fg, next_bg);
        assert!(futures::poll!(&mut next_fg).is_pending());
        assert!(futures::poll!(&mut next_bg).is_pending());
        drop(background);
        assert!(futures::poll!(&mut next_bg).is_pending(), "Foreground が動いている間は、空いても始めない");
        foreground.pop();
        let next_fg = next_fg.await?;
        assert!(futures::poll!(&mut next_bg).is_pending());
        drop((foreground, next_fg));
        let next_bg = next_bg.await?;

        let mut streams = Vec::new();
        drop(next_bg);
        for _ in 0..4 {
            streams.push(budget.streams().ticket(WorkClass::Background).acquire().await?);
        }
        let foreground_stream = budget.streams().ticket(WorkClass::Foreground).acquire().await?;
        let extra = budget.streams().ticket(WorkClass::Background).acquire();
        tokio::pin!(extra);
        assert!(futures::poll!(&mut extra).is_pending());
        // Ordinary background stream waiters also exclude opportunistic probes.
        assert!(budget.try_probe().is_none());
        drop(foreground_stream);
        streams.pop();
        let _extra = extra.await?;
        drop(streams);
        drop(_extra);
        assert!(budget.try_probe().is_some());
        Ok(())
    }

    #[tokio::test]
    async fn cancelling_waiter_and_blocking_caller_retains_real_worker_test() -> TestResult {
        let budget = Arc::new(WorkBudget::default());
        let (started, ready) = tokio::sync::oneshot::channel();
        let (release, wait) = std::sync::mpsc::channel();
        let worker_budget = budget.clone();
        let job = tokio::spawn(async move {
            let permit = worker_budget.io().ticket(WorkClass::Background).counted(WorkKind::Archive).acquire().await.unwrap();
            tokio::task::spawn_blocking(move || {
                let _permit = permit;
                started.send(()).unwrap();
                wait.recv().unwrap();
            })
            .await
        });
        ready.await?;
        job.abort();
        let _ = job.await;
        assert_eq!(budget.counts(WorkKind::Archive), (1, 0));
        assert!(budget.try_probe().is_none());
        {
            let waiter = budget.io().ticket(WorkClass::Background).counted(WorkKind::Probe).acquire();
            tokio::pin!(waiter);
            assert!(futures::poll!(&mut waiter).is_pending());
        }
        release.send(())?;
        let _finished = budget.io().ticket(WorkClass::Background).counted(WorkKind::Summary).acquire().await?;
        assert_eq!(budget.counts(WorkKind::Archive), (1, 1));
        Ok(())
    }
}
