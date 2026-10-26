use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use crate::core::{
    schedule::SchedulePermit,
    work::{WorkBudget, WorkClass, WorkKind},
};
use tokio_util::sync::CancellationToken;

use super::{ArchiveFormat, FileStorageLocal};
use crate::prelude::*;

pub(super) type LocalDirectory = Arc<Mutex<std::fs::ReadDir>>;

pub(super) struct LocalListingRuntime {
    pub(super) budget: Arc<WorkBudget>,
    reader: Arc<dyn LocalEntryReader>,
}

pub(super) struct LocalListingBatch {
    pub entries: Vec<FsEntry>,
    pub metadata_sec: f64,
}

struct LocalListedEntry {
    entry: FsEntry,
    metadata_sec: f64,
}

/// OS 境界。テストは属性取得を差し替え、待ち時間に依存せず上限と取消を検証する。
trait LocalEntryReader: Send + Sync {
    fn read(&self, path: &Path) -> Result<LocalListedEntry>;
}

struct MetadataReader;

impl LocalEntryReader for MetadataReader {
    fn read(&self, path: &Path) -> Result<LocalListedEntry> {
        let text = FileStorageLocal::to_utf8(path)?;
        let started = Instant::now();
        let meta = std::fs::symlink_metadata(path).map_err(|e| FileStorageLocal::io_error(e, &text))?;
        let metadata_sec = started.elapsed().as_secs_f64();
        let detail = if meta.is_symlink() {
            let target = std::fs::read_link(path).map_err(|e| FileStorageLocal::io_error(e, &text))?;
            let target = FileStorageLocal::to_utf8(&target)?;
            let target_kind = match std::fs::metadata(path) {
                Ok(meta) if meta.is_dir() => Some(FsEntryKind::Dir),
                Ok(meta) if meta.is_file() => Some(FsEntryKind::File),
                _ => None,
            };
            FsEntryDetail::Link {
                target: Some(target),
                target_kind,
            }
        } else if meta.is_dir() {
            FsEntryDetail::Dir { has_children: None }
        } else if meta.is_file() {
            FsEntryDetail::File {
                archive: ArchiveFormat::supports_path(&text),
            }
        } else {
            return Err(Error::new(ErrorKind::UnsupportedType).with_message(format!("unsupported entry: {text}")));
        };
        Ok(LocalListedEntry {
            entry: FsEntry {
                path: text,
                meta: FileStorageLocal::to_fs_metadata(&meta),
                detail,
            },
            metadata_sec,
        })
    }
}

impl LocalListingRuntime {
    pub const BATCH_SIZE: usize = 64;
    pub const PARALLEL_BATCHES: usize = 2;
    #[cfg(test)]
    const MAX_SCANS: usize = 12;
    #[cfg(test)]
    const MAX_WORKERS: usize = 3;

    #[cfg(test)]
    pub fn new() -> Self {
        Self::with_budget(Arc::new(WorkBudget::default()))
    }

    pub fn with_budget(budget: Arc<WorkBudget>) -> Self {
        Self {
            budget,
            reader: Arc::new(MetadataReader),
        }
    }

    pub async fn enter(&self) -> Result<SchedulePermit> {
        self.budget.enter(WorkClass::current()).await
    }

    pub async fn read_entry(&self, path: &str) -> Result<FsEntry> {
        let cancellation = CancellationToken::new();
        let _cancel_on_drop = cancellation.clone().drop_guard();
        let batch = self.read_batch(vec![PathBuf::from(path)], cancellation).await?;
        batch
            .entries
            .into_iter()
            .next()
            .ok_or_else(|| Error::new(ErrorKind::UnexpectedError).with_message("empty metadata batch"))
    }

    pub async fn open_directory(&self, path: PathBuf, cancellation: CancellationToken) -> Result<LocalDirectory> {
        let permit = self.budget.acquire(WorkClass::current(), WorkKind::Listing).await?;
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            if cancellation.is_cancelled() {
                return Err(Error::new(ErrorKind::Cancelled));
            }
            let reader = std::fs::read_dir(&path).map_err(|error| FileStorageLocal::io_error(error, &path.to_string_lossy()))?;
            Ok(Arc::new(Mutex::new(reader)))
        })
        .await?
    }

    pub async fn read_paths(&self, directory: LocalDirectory, cancellation: CancellationToken) -> Result<Vec<PathBuf>> {
        let permit = self.budget.acquire(WorkClass::current(), WorkKind::Listing).await?;
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let mut reader = directory.lock();
            let mut paths = Vec::with_capacity(Self::BATCH_SIZE);
            for _ in 0..Self::BATCH_SIZE {
                if cancellation.is_cancelled() {
                    return Err(Error::new(ErrorKind::Cancelled));
                }
                let Some(entry) = reader.next() else {
                    break;
                };
                paths.push(entry?.path());
            }
            Ok(paths)
        })
        .await?
    }

    pub async fn read_summaries(&self, directory: LocalDirectory, cancellation: CancellationToken) -> Result<Vec<FsEntrySummary>> {
        let permit = self.budget.acquire(WorkClass::current(), WorkKind::Listing).await?;
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let mut reader = directory.lock();
            let mut entries = Vec::with_capacity(Self::BATCH_SIZE);
            for _ in 0..Self::BATCH_SIZE {
                if cancellation.is_cancelled() {
                    return Err(Error::new(ErrorKind::Cancelled));
                }
                let Some(item) = reader.next() else {
                    break;
                };
                let item = item?;
                let path = FileStorageLocal::to_utf8(&item.path())?;
                // std は列挙時の種別を使い、必要な環境では属性照会へフォールバックする。
                // 参照先の metadata は読まない。種別取得のための追加 syscall 数は OS に依存する。
                let kind = item.file_type().map_err(|error| FileStorageLocal::io_error(error, &path))?;
                let kind = if kind.is_dir() {
                    FsEntryKind::Dir
                } else if kind.is_file() {
                    FsEntryKind::File
                } else if kind.is_symlink() {
                    FsEntryKind::Link
                } else {
                    return Err(Error::new(ErrorKind::UnsupportedType).with_message(format!("unsupported entry: {path}")));
                };
                entries.push(FsEntrySummary {
                    archive: kind == FsEntryKind::File && ArchiveFormat::supports_path(&path),
                    path,
                    kind,
                });
            }
            Ok(entries)
        })
        .await?
    }

    pub async fn read_batch(&self, paths: Vec<PathBuf>, cancellation: CancellationToken) -> Result<LocalListingBatch> {
        let permit = self.budget.acquire(WorkClass::current(), WorkKind::Listing).await?;
        let reader = self.reader.clone();
        tokio::task::spawn_blocking(move || {
            // JoinHandle が捨てられても実際の worker が終わるまで枠を返さない。
            let _permit = permit;
            let mut entries = Vec::with_capacity(paths.len());
            let mut metadata_sec = 0.0;
            for path in paths {
                if cancellation.is_cancelled() {
                    return Err(Error::new(ErrorKind::Cancelled));
                }
                let listed = reader.read(&path)?;
                entries.push(listed.entry);
                metadata_sec += listed.metadata_sec;
            }
            Ok(LocalListingBatch { entries, metadata_sec })
        })
        .await?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::nfs::FileStorageDriver;
    use futures::StreamExt as _;
    use omnius_core_base::{clock::ClockUtc, tsid::TsidProviderImpl};
    use rand::{SeedableRng as _, rngs::ChaCha20Rng};
    use std::sync::{
        Condvar, Mutex as StdMutex,
        atomic::{AtomicUsize, Ordering},
    };
    use testresult::TestResult;

    struct Probe {
        entered: tokio::sync::mpsc::UnboundedSender<()>,
        released: StdMutex<bool>,
        release_signal: Condvar,
        calls: AtomicUsize,
        active: AtomicUsize,
        peak: AtomicUsize,
    }

    impl Probe {
        fn new(released: bool) -> (Arc<Self>, tokio::sync::mpsc::UnboundedReceiver<()>) {
            let (entered, receiver) = tokio::sync::mpsc::unbounded_channel();
            (
                Arc::new(Self {
                    entered,
                    released: StdMutex::new(released),
                    release_signal: Condvar::new(),
                    calls: AtomicUsize::new(0),
                    active: AtomicUsize::new(0),
                    peak: AtomicUsize::new(0),
                }),
                receiver,
            )
        }
        fn release(&self) {
            *self.released.lock().unwrap() = true;
            self.release_signal.notify_all();
        }
    }

    impl LocalEntryReader for Probe {
        fn read(&self, path: &Path) -> Result<LocalListedEntry> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
            self.peak.fetch_max(active, Ordering::SeqCst);
            let _ = self.entered.send(());
            let _released = self.release_signal.wait_while(self.released.lock().unwrap(), |released| !*released).unwrap();
            self.active.fetch_sub(1, Ordering::SeqCst);
            Ok(LocalListedEntry {
                entry: FsEntry {
                    path: path.to_str().unwrap().to_string(),
                    meta: FsMetadata {
                        len: 7,
                        readonly: false,
                        created: None,
                        modified: None,
                        accessed: None,
                    },
                    detail: FsEntryDetail::File { archive: false },
                },
                metadata_sec: 0.0,
            })
        }
    }

    struct ReleaseOnDrop(Arc<Probe>);
    impl Drop for ReleaseOnDrop {
        fn drop(&mut self) {
            self.0.release();
        }
    }

    fn runtime(probe: Arc<Probe>) -> Arc<LocalListingRuntime> {
        let mut runtime = LocalListingRuntime::new();
        runtime.reader = probe;
        Arc::new(runtime)
    }

    #[tokio::test]
    async fn background_listing_waits_until_foreground_details_finish_test() -> TestResult {
        let (probe, mut entered) = Probe::new(false);
        let _release = ReleaseOnDrop(probe.clone());
        let runtime = runtime(probe.clone());
        let mut jobs = Vec::new();
        for class in [WorkClass::Foreground, WorkClass::Foreground, WorkClass::Foreground, WorkClass::Background] {
            let runtime = runtime.clone();
            jobs.push(tokio::spawn(class.scope(async move { runtime.read_entry("/one").await })));
        }
        for _ in 0..3 {
            entered.recv().await.unwrap();
        }
        let waiting = runtime.read_entry("/next");
        tokio::pin!(waiting);
        assert!(futures::poll!(&mut waiting).is_pending());
        assert_eq!(runtime.budget.workers().running(WorkClass::Background), 0, "Foreground の間は Background を始めない");
        assert!(runtime.budget.try_probe().is_none());
        probe.release();
        waiting.await?;
        for job in jobs {
            job.await??;
        }
        assert_eq!(probe.calls.load(Ordering::SeqCst), 5);
        assert_eq!(probe.peak.load(Ordering::SeqCst), 3);
        Ok(())
    }

    #[tokio::test]
    async fn listing_stream_preserves_background_class_after_creation_scope_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let tsid = Arc::new(Mutex::new(TsidProviderImpl::new(ClockUtc, ChaCha20Rng::seed_from_u64(0), 16)));
        let budget = Arc::new(WorkBudget::default());
        let driver = FileStorageLocal::with_budget(tsid, budget.clone());
        let held = budget.acquire(WorkClass::Background, WorkKind::Exif).await?;
        let mut stream = WorkClass::Background.scope(driver.find_entries(dir.path().to_str().unwrap(), false)).await?;
        let item = stream.next();
        tokio::pin!(item);
        assert!(futures::poll!(&mut item).is_pending());
        assert_eq!(budget.counts(WorkKind::Listing), (0, 0));
        drop(held);
        assert!(item.await.is_none());
        assert!(budget.counts(WorkKind::Listing).0 > 0);
        Ok(())
    }

    #[tokio::test]
    async fn batch_reads_each_path_once_test() -> TestResult {
        let (probe, _) = Probe::new(true);
        let runtime = runtime(probe.clone());
        let paths: Vec<_> = (0..LocalListingRuntime::BATCH_SIZE).map(|i| PathBuf::from(format!("/entry-{i}"))).collect();
        let batch = runtime.read_batch(paths.clone(), CancellationToken::new()).await?;
        assert_eq!(probe.calls.load(Ordering::SeqCst), paths.len());
        assert_eq!(batch.entries.iter().map(|entry| PathBuf::from(&entry.path)).collect::<Vec<_>>(), paths);
        assert!(batch.entries.iter().all(|entry| entry.meta.len == 7));
        Ok(())
    }

    #[tokio::test]
    async fn worker_permits_are_shared_across_requests_test() -> TestResult {
        let (probe, mut entered) = Probe::new(false);
        let _release = ReleaseOnDrop(probe.clone());
        let runtime = runtime(probe.clone());
        let mut jobs = Vec::new();
        for i in 0..LocalListingRuntime::MAX_WORKERS + 1 {
            let runtime = runtime.clone();
            jobs.push(tokio::spawn(async move {
                runtime.read_batch(vec![PathBuf::from(format!("/{i}"))], CancellationToken::new()).await
            }));
        }
        for _ in 0..LocalListingRuntime::MAX_WORKERS {
            entered.recv().await.unwrap();
        }
        assert_eq!(probe.calls.load(Ordering::SeqCst), LocalListingRuntime::MAX_WORKERS);
        assert_eq!(runtime.budget.workers().running(WorkClass::Foreground), LocalListingRuntime::MAX_WORKERS);
        probe.release();
        for job in jobs {
            job.await??;
        }
        assert_eq!(probe.peak.load(Ordering::SeqCst), LocalListingRuntime::MAX_WORKERS);
        assert_eq!(probe.calls.load(Ordering::SeqCst), LocalListingRuntime::MAX_WORKERS + 1);
        Ok(())
    }

    #[tokio::test]
    async fn summary_and_leaf_details_share_existing_workers_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("file.zip"), b"")?;
        let (probe, mut entered) = Probe::new(false);
        let _release = ReleaseOnDrop(probe.clone());
        let runtime = runtime(probe.clone());
        let directory = runtime.open_directory(dir.path().to_path_buf(), CancellationToken::new()).await?;
        let mut jobs = Vec::new();
        for i in 0..LocalListingRuntime::MAX_WORKERS {
            let runtime = runtime.clone();
            jobs.push(tokio::spawn(async move { runtime.read_entry(&format!("/{i}")).await }));
        }
        for _ in 0..LocalListingRuntime::MAX_WORKERS {
            entered.recv().await.unwrap();
        }
        let summaries = runtime.read_summaries(directory, CancellationToken::new());
        tokio::pin!(summaries);
        assert!(futures::poll!(&mut summaries).is_pending());
        assert_eq!(runtime.budget.workers().running(WorkClass::Foreground), LocalListingRuntime::MAX_WORKERS);
        probe.release();
        let summaries = summaries.await?;
        for job in jobs {
            job.await??;
        }
        assert_eq!(summaries.len(), 1);
        assert!(summaries[0].archive);
        // 名前・種別の列挙は、詳細の MetadataReader を呼ばない。
        assert_eq!(probe.calls.load(Ordering::SeqCst), LocalListingRuntime::MAX_WORKERS);
        assert_eq!(probe.peak.load(Ordering::SeqCst), LocalListingRuntime::MAX_WORKERS);
        Ok(())
    }

    #[tokio::test]
    async fn active_scans_wait_before_preparing_more_work_test() -> TestResult {
        let runtime = LocalListingRuntime::new();
        let mut permits = Vec::new();
        for _ in 0..LocalListingRuntime::MAX_SCANS {
            permits.push(runtime.enter().await?);
        }
        let next = runtime.enter();
        tokio::pin!(next);
        assert!(futures::poll!(&mut next).is_pending());
        permits.pop();
        let _permit = next.await?;
        Ok(())
    }

    #[tokio::test]
    async fn cancellation_stops_the_rest_of_a_batch_test() -> TestResult {
        let (probe, mut entered) = Probe::new(false);
        let _release = ReleaseOnDrop(probe.clone());
        let runtime = runtime(probe.clone());
        let cancellation = CancellationToken::new();
        let task_cancellation = cancellation.clone();
        let job = tokio::spawn(async move { runtime.read_batch((0..64).map(|i| PathBuf::from(format!("/{i}"))).collect(), task_cancellation).await });
        entered.recv().await.unwrap();
        cancellation.cancel();
        probe.release();
        let error = job.await?.err().unwrap();
        assert_eq!(error.kind(), &ErrorKind::Cancelled);
        assert_eq!(probe.calls.load(Ordering::SeqCst), 1);
        Ok(())
    }

    #[tokio::test]
    async fn dropping_a_stream_cancels_prefetched_metadata_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        for i in 0..128 {
            std::fs::write(dir.path().join(i.to_string()), b"")?;
        }
        let (probe, mut entered) = Probe::new(false);
        let _release = ReleaseOnDrop(probe.clone());
        let runtime = runtime(probe.clone());
        let tsid = Arc::new(Mutex::new(TsidProviderImpl::new(ClockUtc, ChaCha20Rng::seed_from_u64(0), 16)));
        let mut driver = FileStorageLocal::new(tsid);
        driver.listing = runtime.clone();
        let mut stream = driver.find_entries(dir.path().to_str().unwrap(), false).await?;
        tokio::select! {
            result = stream.next() => panic!("metadata returned before release: {}", result.is_some()),
            _ = entered.recv() => {}
        }
        drop(stream);
        probe.release();
        let mut all_permits = Vec::new();
        for _ in 0..LocalListingRuntime::MAX_WORKERS {
            all_permits.push(runtime.budget.workers().acquire(WorkClass::Foreground).await);
        }
        assert!(probe.calls.load(Ordering::SeqCst) <= LocalListingRuntime::PARALLEL_BATCHES);
        assert_eq!(probe.active.load(Ordering::SeqCst), 0);
        Ok(())
    }
}
