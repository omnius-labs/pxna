use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use tokio::{sync::Semaphore, time::Instant};

use crate::{
    base::util::SingleFlight,
    core::work::{WorkBudget, WorkClass, WorkKind, WorkScheduler},
    prelude::*,
};

use super::{ArchiveError, InspectedArchive, cache::ExtractionCache, extractor::ExtractedBlock, identity::ArchiveIdentity, index::ArchiveBlockId, index_cache::IndexCache};

const MIB: u64 = 1024 * 1024;
const GIB: u64 = 1024 * MIB;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZipLegacyEncoding {
    Cp932,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveOptions {
    pub cache_capacity_bytes: u64,
    pub index_cache_capacity_bytes: u64,
    pub max_nested_depth: usize,
    pub max_entry_count: usize,
    pub max_materialized_file_size: u64,
    pub max_block_extract_bytes: u64,
    pub extract_timeout: Duration,
    pub max_concurrent_extractions: usize,
    pub zip_legacy_encoding: ZipLegacyEncoding,
}

impl Default for ArchiveOptions {
    fn default() -> Self {
        Self {
            cache_capacity_bytes: 10 * GIB,
            index_cache_capacity_bytes: 256 * MIB,
            max_nested_depth: 4,
            max_entry_count: 100_000,
            max_materialized_file_size: 8 * GIB,
            max_block_extract_bytes: 4 * GIB,
            extract_timeout: Duration::from_secs(300),
            max_concurrent_extractions: 2,
            zip_legacy_encoding: ZipLegacyEncoding::Cp932,
        }
    }
}

impl ArchiveOptions {
    /// byte size、件数、時間、並列数は正の値だけを許可する。`0` を unlimited の意味には使わない。
    pub fn validate(&self) -> Result<()> {
        if self.cache_capacity_bytes == 0 {
            return Err(ArchiveError::limit_exceeded("cache_capacity_bytes must be greater than zero"));
        }
        if self.index_cache_capacity_bytes == 0 {
            return Err(ArchiveError::limit_exceeded("index_cache_capacity_bytes must be greater than zero"));
        }
        if self.max_nested_depth == 0 {
            return Err(ArchiveError::limit_exceeded("max_nested_depth must be greater than zero"));
        }
        if self.max_entry_count == 0 {
            return Err(ArchiveError::limit_exceeded("max_entry_count must be greater than zero"));
        }
        if self.max_materialized_file_size == 0 {
            return Err(ArchiveError::limit_exceeded("max_materialized_file_size must be greater than zero"));
        }
        if self.max_block_extract_bytes == 0 {
            return Err(ArchiveError::limit_exceeded("max_block_extract_bytes must be greater than zero"));
        }
        if self.extract_timeout.is_zero() {
            return Err(ArchiveError::limit_exceeded("extract_timeout must be greater than zero"));
        }
        if self.max_concurrent_extractions == 0 {
            return Err(ArchiveError::limit_exceeded("max_concurrent_extractions must be greater than zero"));
        }
        Ok(())
    }
}

/// archive reader が process 全体で共有する制限値、同時展開数の semaphore、cache。
pub struct ArchiveRuntime {
    budget: Arc<WorkBudget>,
    options: ArchiveOptions,
    semaphore: Arc<Semaphore>,
    index_cache: IndexCache<InspectedArchive>,
    extraction_cache: Option<ExtractionCache>,
    block_flights: SingleFlight<(ArchiveIdentity, ArchiveBlockId), ExtractedBlock>,
    #[cfg(test)]
    decoded_blocks: std::sync::atomic::AtomicUsize,
}

impl ArchiveRuntime {
    /// 永続 extraction cache を持たず、取り出した file を lease 専用 temp として返す。
    pub fn new(options: ArchiveOptions) -> Result<Self> {
        options.validate()?;
        Ok(Self::build(options, None))
    }

    /// `cache_dir` に永続 extraction cache を開く。起動時の reconciliation もここで行う。
    pub async fn open(options: ArchiveOptions, cache_dir: &Path) -> Result<Self> {
        options.validate()?;
        let cache = ExtractionCache::open(cache_dir, options.cache_capacity_bytes).await?;
        Ok(Self::build(options, Some(cache)))
    }

    fn build(options: ArchiveOptions, extraction_cache: Option<ExtractionCache>) -> Self {
        let semaphore = Arc::new(Semaphore::new(options.max_concurrent_extractions));
        let index_cache = IndexCache::new(options.index_cache_capacity_bytes);
        Self {
            budget: Arc::new(WorkBudget::default()),
            options,
            semaphore,
            index_cache,
            extraction_cache,
            block_flights: SingleFlight::new(),
            #[cfg(test)]
            decoded_blocks: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    pub fn with_budget(mut self, budget: Arc<WorkBudget>) -> Self {
        self.budget = budget;
        self
    }

    pub fn work_budget(&self) -> &Arc<WorkBudget> {
        &self.budget
    }

    pub fn options(&self) -> &ArchiveOptions {
        &self.options
    }

    pub(super) fn index_cache(&self) -> &IndexCache<InspectedArchive> {
        &self.index_cache
    }

    pub(super) fn extraction_cache(&self) -> Option<&ExtractionCache> {
        self.extraction_cache.as_ref()
    }

    pub(super) fn block_flights(&self) -> &SingleFlight<(ArchiveIdentity, ArchiveBlockId), ExtractedBlock> {
        &self.block_flights
    }

    /// decode の出力先。cache と同じ filesystem に置き、publish を rename で済ませる。
    pub(super) fn staging_dir(&self) -> PathBuf {
        match &self.extraction_cache {
            Some(cache) => cache.staging_dir(),
            None => std::env::temp_dir(),
        }
    }

    #[cfg(test)]
    pub(super) fn record_decoded_block(&self) {
        self.decoded_blocks.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    #[cfg(test)]
    pub(crate) fn decoded_blocks(&self) -> usize {
        self.decoded_blocks.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// 閲覧の展開の permit をすべて取り、閲覧の枠が空いていない状態を作る。
    #[cfg(test)]
    pub(super) async fn hold_all_permits(&self) -> tokio::sync::OwnedSemaphorePermit {
        let count = u32::try_from(self.options.max_concurrent_extractions).expect("permit count fits in u32");
        self.semaphore.clone().acquire_many_owned(count).await.expect("semaphore is open")
    }

    pub(super) async fn run_blocking<T, F>(&self, class: WorkClass, task: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce() -> Result<T> + Send + 'static,
    {
        self.run_blocking_with(class, &self.semaphore, task).await
    }

    /// `semaphore` の permit を持って decoder を動かす。期限は閲覧と同じ `extract_timeout` である。
    pub(super) async fn run_blocking_with<T, F>(&self, class: WorkClass, semaphore: &Arc<Semaphore>, task: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce() -> Result<T> + Send + 'static,
    {
        let deadline = Instant::now() + self.options.extract_timeout;
        let background = tokio::time::timeout_at(deadline, self.budget.background(class, WorkKind::Archive))
            .await
            .map_err(|_| Error::new(ErrorKind::Timeout))??;
        let permit = tokio::time::timeout_at(deadline, semaphore.clone().acquire_owned())
            .await
            .map_err(|_| Error::new(ErrorKind::Timeout).with_message("archive extraction timed out while waiting for a worker"))?
            .map_err(|_| Error::new(ErrorKind::Cancelled).with_message("archive extraction runtime is shutting down"))?;

        let worker = WorkScheduler::run_blocking_with((background, permit), task);

        tokio::time::timeout_at(deadline, worker)
            .await
            .map_err(|_| Error::new(ErrorKind::Timeout).with_message("archive extraction timed out"))?
            .map_err(|error| error.with_message("join failed"))?
    }
}

impl std::fmt::Debug for ArchiveRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArchiveRuntime").field("options", &self.options).finish_non_exhaustive()
    }
}

impl Default for ArchiveRuntime {
    fn default() -> Self {
        Self::new(ArchiveOptions::default()).expect("default archive options are valid")
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    use super::*;

    #[test]
    fn archive_runtime_defaults_test() {
        let runtime = ArchiveRuntime::default();
        assert_eq!(runtime.options.cache_capacity_bytes, 10 * GIB);
        assert_eq!(runtime.options.index_cache_capacity_bytes, 256 * MIB);
        assert_eq!(runtime.options.max_nested_depth, 4);
        assert_eq!(runtime.options.max_entry_count, 100_000);
        assert_eq!(runtime.options.max_materialized_file_size, 8 * GIB);
        assert_eq!(runtime.options.max_block_extract_bytes, 4 * GIB);
        assert_eq!(runtime.options.extract_timeout, Duration::from_secs(300));
        assert_eq!(runtime.options.max_concurrent_extractions, 2);
        assert_eq!(runtime.options.zip_legacy_encoding, ZipLegacyEncoding::Cp932);
    }

    #[test]
    fn archive_runtime_rejects_zero_limits_test() {
        let mut options = ArchiveOptions::default();
        options.cache_capacity_bytes = 0;
        assert_eq!(ArchiveRuntime::new(options).unwrap_err().kind(), &ErrorKind::LimitExceeded);

        let mut options = ArchiveOptions::default();
        options.index_cache_capacity_bytes = 0;
        assert_eq!(ArchiveRuntime::new(options).unwrap_err().kind(), &ErrorKind::LimitExceeded);

        let mut options = ArchiveOptions::default();
        options.max_nested_depth = 0;
        assert_eq!(ArchiveRuntime::new(options).unwrap_err().kind(), &ErrorKind::LimitExceeded);

        let mut options = ArchiveOptions::default();
        options.max_entry_count = 0;
        assert_eq!(ArchiveRuntime::new(options).unwrap_err().kind(), &ErrorKind::LimitExceeded);

        let mut options = ArchiveOptions::default();
        options.max_materialized_file_size = 0;
        assert_eq!(ArchiveRuntime::new(options).unwrap_err().kind(), &ErrorKind::LimitExceeded);

        let mut options = ArchiveOptions::default();
        options.max_block_extract_bytes = 0;
        assert_eq!(ArchiveRuntime::new(options).unwrap_err().kind(), &ErrorKind::LimitExceeded);

        let mut options = ArchiveOptions::default();
        options.extract_timeout = Duration::ZERO;
        assert_eq!(ArchiveRuntime::new(options).unwrap_err().kind(), &ErrorKind::LimitExceeded);

        let mut options = ArchiveOptions::default();
        options.max_concurrent_extractions = 0;
        assert_eq!(ArchiveRuntime::new(options).unwrap_err().kind(), &ErrorKind::LimitExceeded);
    }

    #[tokio::test]
    async fn background_extraction_reserves_before_resource_lock_and_keeps_real_worker_test() {
        let budget = Arc::new(WorkBudget::default());
        let runtime = Arc::new(ArchiveRuntime::default().with_budget(budget.clone()));
        let held = runtime.hold_all_permits().await;
        let (started, ready) = tokio::sync::oneshot::channel();
        let (release, wait) = std::sync::mpsc::channel();
        let work_runtime = runtime.clone();
        let work = tokio::spawn(async move {
            work_runtime
                .run_blocking(WorkClass::Background, move || {
                    started.send(()).unwrap();
                    wait.recv().unwrap();
                    Ok(())
                })
                .await
        });
        while budget.counts(WorkKind::Archive).0 == 0 {
            tokio::task::yield_now().await;
        }
        // A foreground slot remains usable while archive waits for its resource slot.
        let foreground = budget.io().ticket(WorkClass::Foreground).counted(WorkKind::Listing).acquire().await.unwrap();
        assert!(budget.try_probe().is_none());
        drop(foreground);
        drop(held);
        ready.await.unwrap();
        work.abort();
        let _ = work.await;
        assert_eq!(budget.counts(WorkKind::Archive), (1, 0));
        release.send(()).unwrap();
        let _next = budget.io().ticket(WorkClass::Background).counted(WorkKind::Exif).acquire().await.unwrap();
        assert_eq!(budget.counts(WorkKind::Archive), (1, 1));
    }

    #[tokio::test]
    async fn timeout_keeps_process_wide_permit_until_worker_finishes_test() {
        let runtime = Arc::new(
            ArchiveRuntime::new(ArchiveOptions {
                extract_timeout: Duration::from_millis(20),
                max_concurrent_extractions: 1,
                ..ArchiveOptions::default()
            })
            .unwrap(),
        );
        let (started, observed_start) = tokio::sync::oneshot::channel();
        let (release, held) = std::sync::mpsc::channel();
        let first_runtime = runtime.clone();
        let first = tokio::spawn(async move {
            first_runtime
                .run_blocking(WorkClass::Background, move || {
                    let _ = started.send(());
                    held.recv().map_err(|error| Error::from_error(error, ErrorKind::TaskError))?;
                    Ok(())
                })
                .await
        });
        tokio::time::timeout(Duration::from_secs(5), observed_start).await.unwrap().unwrap();
        let error = first.await.unwrap().unwrap_err();
        assert_eq!(error.kind(), &ErrorKind::Timeout);

        let second_started = Arc::new(AtomicBool::new(false));
        let second_started_in_worker = second_started.clone();
        let error = runtime
            .run_blocking(WorkClass::Background, move || {
                second_started_in_worker.store(true, Ordering::SeqCst);
                Ok(())
            })
            .await
            .unwrap_err();
        assert_eq!(error.kind(), &ErrorKind::Timeout);
        assert!(!second_started.load(Ordering::SeqCst));

        release.send(()).unwrap();
        let _permit = tokio::time::timeout(Duration::from_secs(5), runtime.semaphore.clone().acquire_owned())
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn timeout_worker_result_is_cleaned_after_it_finishes_test() {
        let runtime = ArchiveRuntime::new(ArchiveOptions {
            extract_timeout: Duration::from_millis(20),
            ..ArchiveOptions::default()
        })
        .unwrap();
        let temp = tempfile::NamedTempFile::new().unwrap();
        let path = temp.into_temp_path().keep().unwrap();
        let worker_path = path.clone();

        let error = runtime
            .run_blocking(WorkClass::Background, move || {
                std::thread::sleep(Duration::from_millis(60));
                Ok(crate::core::nfs::MaterializedFile::owned_temp(worker_path))
            })
            .await
            .unwrap_err();
        assert_eq!(error.kind(), &ErrorKind::Timeout);
        assert!(path.exists());

        for _ in 0..50 {
            if !path.exists() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn process_wide_concurrency_limit_test() {
        let runtime = Arc::new(
            ArchiveRuntime::new(ArchiveOptions {
                extract_timeout: Duration::from_secs(2),
                max_concurrent_extractions: 2,
                ..ArchiveOptions::default()
            })
            .unwrap(),
        );
        let active = Arc::new(AtomicUsize::new(0));
        let maximum = Arc::new(AtomicUsize::new(0));
        let mut tasks = Vec::new();

        for _ in 0..4 {
            let runtime = runtime.clone();
            let active = active.clone();
            let maximum = maximum.clone();
            tasks.push(tokio::spawn(async move {
                runtime
                    .run_blocking(WorkClass::Foreground, move || {
                        let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                        maximum.fetch_max(now, Ordering::SeqCst);
                        std::thread::sleep(Duration::from_millis(40));
                        active.fetch_sub(1, Ordering::SeqCst);
                        Ok(())
                    })
                    .await
            }));
        }
        for task in tasks {
            task.await.unwrap().unwrap();
        }
        assert_eq!(maximum.load(Ordering::SeqCst), 2);
    }
}
