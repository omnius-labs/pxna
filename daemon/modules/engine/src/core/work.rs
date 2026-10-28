//! 一覧と背景処理の枠。HTTP の受付数や他の storage 操作は制限しない。
//!
//! 枠の規則は [`super::schedule`] が持つ。ここは一覧の io と stream の 2 つの資源と、処理の種類ごとの回数を持つ。
use crate::prelude::*;
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::Stream;
use tokio::task::JoinHandle;
use tracing::Instrument as _;

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
    /// 上の段から順に並べたすべての段。
    pub const ALL: [Self; 3] = [Self::Interactive, Self::Foreground, Self::Background];

    pub fn current() -> Self {
        CLASS.try_with(|class| *class).unwrap_or(Self::Foreground)
    }
    pub async fn scope<F: Future>(self, future: F) -> F::Output {
        CLASS.scope(self, future).await
    }

    /// 今の段と Span を引き継いで task を起こす。`tokio::spawn` は task_local を引き継がないので、段を落とさないためにこれを使う。
    pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        Self::current().spawn_as(future)
    }

    /// 作った時点の段を、`stream` の poll ごとに掛けて動かす。作った場所の外で poll される stream（一覧など）が、段を落とさないために使う。
    pub fn bind_stream<S: Stream>(self, stream: S) -> BoundStream<S> {
        BoundStream {
            class: self,
            inner: Box::pin(stream),
        }
    }

    /// [`Self::spawn`] を、段を指定して行う。
    pub fn spawn_as<F>(self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        tokio::spawn(CLASS.scope(self, future).in_current_span())
    }
}

/// 段を決めて動かす stream（[`WorkClass::bind_stream`]）。
pub struct BoundStream<S> {
    class: WorkClass,
    inner: Pin<Box<S>>,
}

impl<S: Stream> Stream for BoundStream<S> {
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let class = self.class;
        CLASS.sync_scope(class, || self.inner.as_mut().poll_next(cx))
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
    io: PriorityPool,
    streams: PriorityPool,
}

/// 種類つきで取った枠。持っている間は動いているものとして数え、drop すると終了を数える。
pub type WorkPermit = SchedulePermit;

impl Default for WorkBudget {
    fn default() -> Self {
        Self::new(&Arc::new(Scheduler::default()))
    }
}
impl WorkBudget {
    /// ファイルを読む処理。Interactive と Foreground で 3、Background に 1。
    const IO: PoolLimits = PoolLimits {
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

    /// `scheduler` の資源として、一覧の io と stream の枠を足す。
    pub fn new(scheduler: &Arc<Scheduler>) -> Self {
        Self {
            io: scheduler.pool(Self::IO),
            streams: scheduler.pool(Self::STREAMS),
        }
    }
    pub fn io(&self) -> &PriorityPool {
        &self.io
    }
    pub async fn enter(&self, class: WorkClass) -> Result<SchedulePermit> {
        Ok(self.streams.acquire(class).await)
    }
    pub async fn acquire(self: &Arc<Self>, class: WorkClass, kind: WorkKind) -> Result<WorkPermit> {
        Ok(self.io.acquire_counted(class, kind).await)
    }
    /// 待たずに始められるときだけ、子の有無の探査の枠を取る。Background の io か stream に待ちがあれば取らない。
    pub fn try_probe(self: &Arc<Self>) -> Option<(WorkPermit, SchedulePermit)> {
        let stream = self.streams.try_acquire(WorkClass::Background)?;
        let worker = self.io.try_acquire_counted(WorkClass::Background, WorkKind::ChildProbe)?;
        Some((worker, stream))
    }
    pub fn counts(&self, kind: WorkKind) -> (usize, usize) {
        self.io.counts(kind)
    }
    pub async fn background_if_needed(self: &Arc<Self>, kind: WorkKind) -> Result<Option<WorkPermit>> {
        if WorkClass::current() == WorkClass::Background {
            self.acquire(WorkClass::Background, kind).await.map(Some)
        } else {
            Ok(None)
        }
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
