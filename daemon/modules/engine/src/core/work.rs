//! 一覧と背景処理の枠。HTTP の受付数や他の storage 操作は制限しない。
//!
//! 枠の規則は [`super::schedule`] が持つ。ここは一覧の worker と stream の 2 つの資源と、処理の種類ごとの回数を持つ。
use crate::prelude::*;
use std::{
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use super::schedule::{PoolLimits, PriorityPool, SchedulePermit, Scheduler};

tokio::task_local! { static CLASS: WorkClass; }

/// 処理の段。上の段ほど先に始まる（[`super::schedule`]）。
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
    pub fn current() -> Self {
        CLASS.try_with(|class| *class).unwrap_or(Self::Foreground)
    }
    pub async fn scope<F: Future>(self, future: F) -> F::Output {
        CLASS.scope(self, future).await
    }
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
}

pub struct WorkBudget {
    workers: PriorityPool,
    streams: PriorityPool,
    started: [AtomicUsize; 6],
    finished: [AtomicUsize; 6],
}

pub struct WorkPermit {
    _permit: SchedulePermit,
    budget: Arc<WorkBudget>,
    kind: WorkKind,
}
impl Drop for WorkPermit {
    fn drop(&mut self) {
        self.budget.finished[self.kind as usize].fetch_add(1, Ordering::Relaxed);
    }
}

impl Default for WorkBudget {
    fn default() -> Self {
        Self::new(&Arc::new(Scheduler::default()))
    }
}
impl WorkBudget {
    /// 一覧の worker。Interactive と Foreground で 3、Background に 1。
    const WORKERS: PoolLimits = PoolLimits {
        shared: 3,
        interactive: 0,
        background: 1,
    };
    /// 一覧の stream の受付。Interactive と Foreground で 12、Background に 4。
    const STREAMS: PoolLimits = PoolLimits {
        shared: 12,
        interactive: 0,
        background: 4,
    };

    /// `scheduler` の資源として、一覧の worker と stream の枠を足す。
    pub fn new(scheduler: &Arc<Scheduler>) -> Self {
        Self {
            workers: scheduler.pool(Self::WORKERS),
            streams: scheduler.pool(Self::STREAMS),
            started: std::array::from_fn(|_| AtomicUsize::new(0)),
            finished: std::array::from_fn(|_| AtomicUsize::new(0)),
        }
    }
    pub async fn enter(&self, class: WorkClass) -> Result<SchedulePermit> {
        Ok(self.streams.acquire(class).await)
    }
    pub async fn acquire(self: &Arc<Self>, class: WorkClass, kind: WorkKind) -> Result<WorkPermit> {
        let permit = self.workers.acquire(class).await;
        Ok(self.record(permit, kind))
    }
    fn record(self: &Arc<Self>, permit: SchedulePermit, kind: WorkKind) -> WorkPermit {
        self.started[kind as usize].fetch_add(1, Ordering::Relaxed);
        WorkPermit {
            _permit: permit,
            budget: self.clone(),
            kind,
        }
    }
    /// 待たずに始められるときだけ、子の有無の探査の枠を取る。Background の worker か stream に待ちがあれば取らない。
    pub fn try_probe(self: &Arc<Self>) -> Option<(WorkPermit, SchedulePermit)> {
        let stream = self.streams.try_acquire(WorkClass::Background)?;
        let worker = self.workers.try_acquire(WorkClass::Background)?;
        Some((self.record(worker, WorkKind::ChildProbe), stream))
    }
    pub fn counts(&self, kind: WorkKind) -> (usize, usize) {
        (self.started[kind as usize].load(Ordering::Relaxed), self.finished[kind as usize].load(Ordering::Relaxed))
    }
    pub async fn background_if_needed(self: &Arc<Self>, kind: WorkKind) -> Result<Option<WorkPermit>> {
        if WorkClass::current() == WorkClass::Background {
            self.acquire(WorkClass::Background, kind).await.map(Some)
        } else {
            Ok(None)
        }
    }
    #[cfg(test)]
    pub(crate) fn workers(&self) -> &PriorityPool {
        &self.workers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testresult::TestResult;

    #[tokio::test]
    async fn background_runs_beside_foreground_only_in_its_own_slot_test() -> TestResult {
        let budget = Arc::new(WorkBudget::default());
        let background = budget.acquire(WorkClass::Background, WorkKind::Exif).await?;
        let mut foreground = Vec::new();
        for _ in 0..3 {
            foreground.push(budget.acquire(WorkClass::Foreground, WorkKind::Listing).await?);
        }
        let next_fg = budget.acquire(WorkClass::Foreground, WorkKind::Listing);
        let next_bg = budget.acquire(WorkClass::Background, WorkKind::Probe);
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
            streams.push(budget.enter(WorkClass::Background).await?);
        }
        let foreground_stream = budget.enter(WorkClass::Foreground).await?;
        let extra = budget.enter(WorkClass::Background);
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
            let permit = worker_budget.acquire(WorkClass::Background, WorkKind::Archive).await.unwrap();
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
            let waiter = budget.acquire(WorkClass::Background, WorkKind::Probe);
            tokio::pin!(waiter);
            assert!(futures::poll!(&mut waiter).is_pending());
        }
        release.send(())?;
        let _finished = budget.acquire(WorkClass::Background, WorkKind::Summary).await?;
        assert_eq!(budget.counts(WorkKind::Archive), (1, 1));
        Ok(())
    }

    #[tokio::test]
    async fn class_is_explicitly_preserved_across_spawn_test() -> TestResult {
        let future = WorkClass::Background.scope(async { WorkClass::current() });
        assert_eq!(tokio::spawn(future).await?, WorkClass::Background);
        assert_eq!(WorkClass::current(), WorkClass::Foreground);
        Ok(())
    }
}
