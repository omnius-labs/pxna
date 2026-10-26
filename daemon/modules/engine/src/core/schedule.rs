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
use std::{
    collections::VecDeque,
    pin::pin,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use tokio::sync::Notify;

use crate::prelude::*;

use super::work::WorkClass;

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
}

impl Default for Scheduler {
    fn default() -> Self {
        Self {
            state: Mutex::new(State { pools: Vec::new() }),
            changed: Notify::new(),
            next_waiter: AtomicU64::new(0),
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
}

/// 1 つの資源の枠。複製しても同じ資源を指す。
#[derive(Clone)]
pub struct PriorityPool {
    scheduler: Arc<Scheduler>,
    index: usize,
}

/// 枠を使っている間持つ。drop すると枠を返す。
pub struct SchedulePermit {
    pool: PriorityPool,
    class: WorkClass,
}

impl Drop for SchedulePermit {
    fn drop(&mut self) {
        self.pool.scheduler.state.lock().pools[self.pool.index].running[self.class.index()] -= 1;
        self.pool.scheduler.changed.notify_waiters();
    }
}

impl SchedulePermit {
    pub fn class(&self) -> WorkClass {
        self.class
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

/// 待っている間に drop されたら、待ち行列から外す。
struct Waiting<'a> {
    pool: &'a PriorityPool,
    class: WorkClass,
    id: u64,
    queued: bool,
}

impl Drop for Waiting<'_> {
    fn drop(&mut self) {
        if self.queued {
            self.pool.scheduler.state.lock().pools[self.pool.index].waiting[self.class.index()].retain(|id| *id != self.id);
            self.pool.scheduler.changed.notify_waiters();
        }
    }
}

impl PriorityPool {
    /// 枠が空き、規則で始めてよくなるまで待つ。待つ間に drop されたら、何も始まらない。
    pub async fn acquire(&self, class: WorkClass) -> SchedulePermit {
        let scheduler = &self.scheduler;
        let id = scheduler.next_waiter.fetch_add(1, Ordering::Relaxed);
        scheduler.state.lock().pools[self.index].waiting[class.index()].push_back(id);
        let mut waiting = Waiting {
            pool: self,
            class,
            id,
            queued: true,
        };
        loop {
            let mut changed = pin!(scheduler.changed.notified());
            changed.as_mut().enable();
            {
                let mut state = scheduler.state.lock();
                if state.can_start(self.index, class, Some(id)) {
                    let pool = &mut state.pools[self.index];
                    pool.waiting[class.index()].pop_front();
                    pool.running[class.index()] += 1;
                    waiting.queued = false;
                    drop(state);
                    // 先頭が抜けたので、同じ段の次の待ちが始められるかもしれない。
                    scheduler.changed.notify_waiters();
                    return SchedulePermit { pool: self.clone(), class };
                }
            }
            changed.await;
        }
    }

    /// 待たずに始められるときだけ枠を取る。同じ段に待ちがあれば取らない。
    pub fn try_acquire(&self, class: WorkClass) -> Option<SchedulePermit> {
        let mut state = self.scheduler.state.lock();
        if !state.can_start(self.index, class, None) {
            return None;
        }
        state.pools[self.index].running[class.index()] += 1;
        Some(SchedulePermit { pool: self.clone(), class })
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
}

/// `[scheduler]` の設定。画像の変換と外部プロセス（ffmpeg、heif-dec）の同時実行の枠の数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduleOptions {
    /// 画像の変換の共有の枠。
    pub image_slots: usize,
    /// 画像の変換の、Interactive だけが使う枠。
    pub image_interactive_slots: usize,
    /// 外部プロセスの共有の枠。
    pub process_slots: usize,
    /// 外部プロセスの、Interactive だけが使う枠。
    pub process_interactive_slots: usize,
}

impl Default for ScheduleOptions {
    fn default() -> Self {
        Self {
            image_slots: 4,
            image_interactive_slots: 1,
            process_slots: 2,
            process_interactive_slots: 1,
        }
    }
}

impl ScheduleOptions {
    pub fn validate(&self) -> Result<()> {
        if self.image_slots == 0 || self.process_slots == 0 {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("scheduler image_slots and process_slots must be greater than zero"));
        }
        Ok(())
    }
}

/// サムネイルと表示用画像の生成が使う資源。
#[derive(Clone)]
pub struct MediaPools {
    /// 画像の変換（decode、縮小、encode）。
    pub images: PriorityPool,
    /// 外部プロセス（ffmpeg、heif-dec）。動画の変換をユーザーが待っている間も、ここで数える。
    pub processes: PriorityPool,
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
}
