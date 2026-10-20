//! 一覧と背景処理の予約枠。HTTP の受付数や他の storage 操作は制限しない。
use crate::prelude::*;
use std::{
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

tokio::task_local! { static CLASS: WorkClass; }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkClass {
    Foreground,
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
    foreground: Arc<Semaphore>,
    background: Arc<Semaphore>,
    foreground_streams: Arc<Semaphore>,
    background_streams: Arc<Semaphore>,
    background_waiters: Arc<AtomicUsize>,
    started: [AtomicUsize; 6],
    finished: [AtomicUsize; 6],
}

pub struct WorkPermit {
    _permit: OwnedSemaphorePermit,
    budget: Arc<WorkBudget>,
    kind: WorkKind,
}
impl Drop for WorkPermit {
    fn drop(&mut self) {
        self.budget.finished[self.kind as usize].fetch_add(1, Ordering::Relaxed);
    }
}
struct Waiting(Arc<AtomicUsize>);
impl Drop for Waiting {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

impl Default for WorkBudget {
    fn default() -> Self {
        Self {
            foreground: Arc::new(Semaphore::new(3)),
            background: Arc::new(Semaphore::new(1)),
            foreground_streams: Arc::new(Semaphore::new(12)),
            background_streams: Arc::new(Semaphore::new(4)),
            background_waiters: Arc::new(AtomicUsize::new(0)),
            started: std::array::from_fn(|_| AtomicUsize::new(0)),
            finished: std::array::from_fn(|_| AtomicUsize::new(0)),
        }
    }
}
impl WorkBudget {
    async fn reserve(&self, class: WorkClass, stream: bool) -> Result<OwnedSemaphorePermit> {
        let semaphore = match (class, stream) {
            (WorkClass::Foreground, false) => &self.foreground,
            (WorkClass::Background, false) => &self.background,
            (WorkClass::Foreground, true) => &self.foreground_streams,
            (WorkClass::Background, true) => &self.background_streams,
        };
        let _waiting = if class == WorkClass::Background {
            self.background_waiters.fetch_add(1, Ordering::SeqCst);
            Some(Waiting(self.background_waiters.clone()))
        } else {
            None
        };
        semaphore.clone().acquire_owned().await.map_err(|_| Error::new(ErrorKind::Cancelled))
    }
    pub async fn enter(&self, class: WorkClass) -> Result<OwnedSemaphorePermit> {
        self.reserve(class, true).await
    }
    pub async fn acquire(self: &Arc<Self>, class: WorkClass, kind: WorkKind) -> Result<WorkPermit> {
        let permit = self.reserve(class, false).await?;
        Ok(self.record(permit, kind))
    }
    fn record(self: &Arc<Self>, permit: OwnedSemaphorePermit, kind: WorkKind) -> WorkPermit {
        self.started[kind as usize].fetch_add(1, Ordering::Relaxed);
        WorkPermit {
            _permit: permit,
            budget: self.clone(),
            kind,
        }
    }
    /// 通常の背景 worker または stream の待ち手がいれば探査を開始しない。
    pub fn try_probe(self: &Arc<Self>) -> Option<(WorkPermit, OwnedSemaphorePermit)> {
        if self.background_waiters.load(Ordering::SeqCst) != 0 {
            return None;
        }
        let stream = self.background_streams.clone().try_acquire_owned().ok()?;
        let worker = self.background.clone().try_acquire_owned().ok()?;
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
    pub(crate) fn foreground_workers(&self) -> Arc<Semaphore> {
        self.foreground.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testresult::TestResult;

    #[tokio::test]
    async fn reservations_progress_and_streams_are_independent_test() -> TestResult {
        let budget = Arc::new(WorkBudget::default());
        let mut foreground = Vec::new();
        for _ in 0..3 {
            foreground.push(budget.acquire(WorkClass::Foreground, WorkKind::Listing).await?);
        }
        let background = budget.acquire(WorkClass::Background, WorkKind::Exif).await?;
        let next_fg = budget.acquire(WorkClass::Foreground, WorkKind::Listing);
        let next_bg = budget.acquire(WorkClass::Background, WorkKind::Probe);
        tokio::pin!(next_fg, next_bg);
        assert!(futures::poll!(&mut next_fg).is_pending());
        assert!(futures::poll!(&mut next_bg).is_pending());
        drop(background);
        let next_bg = next_bg.await?;
        assert!(futures::poll!(&mut next_fg).is_pending());
        foreground.pop();
        let _next_fg = next_fg.await?;
        let mut streams = Vec::new();
        for _ in 0..4 {
            streams.push(budget.enter(WorkClass::Background).await?);
        }
        let _foreground_stream = budget.enter(WorkClass::Foreground).await?;
        let extra = budget.enter(WorkClass::Background);
        tokio::pin!(extra);
        assert!(futures::poll!(&mut extra).is_pending());
        drop(next_bg);
        // Ordinary background stream waiters also exclude opportunistic probes.
        assert!(budget.try_probe().is_none());
        streams.pop();
        let _extra = extra.await?;
        drop(streams);
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
        assert_eq!(budget.background_waiters.load(Ordering::SeqCst), 0);
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
