use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use chrono::{DateTime, Utc};
use futures::StreamExt as _;

use crate::{
    base::util::SingleFlight,
    core::{media::MediaKind, work::WorkClass},
    prelude::*,
    service::{ArchiveIdentity, ResolvedStoragePath, StorageService},
};

const CAPACITY: usize = 4096;
const TTL: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Version {
    Local(DateTime<Utc>),
    Archive(ArchiveIdentity),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct SelectionKey {
    path: NestedPathBuf,
    version: Option<Version>,
    generation: u64,
    revision: u64,
}

struct CachedSelection {
    key: SelectionKey,
    path: Option<String>,
    inserted: Instant,
    accessed: u64,
}

#[derive(Default)]
struct SelectionState {
    entries: HashMap<NestedPathBuf, CachedSelection>,
    revision: u64,
    access: u64,
}

/// 代表の位置だけを短く保持する。画像と候補の属性は保持しない。
pub(super) struct RepresentativeSelection {
    fs: Arc<StorageService>,
    state: Arc<Mutex<SelectionState>>,
    flights: SingleFlight<SelectionKey, Option<String>>,
    #[cfg(test)]
    scans: Arc<std::sync::atomic::AtomicUsize>,
}

impl RepresentativeSelection {
    #[cfg(test)]
    pub(super) fn scan_count(&self) -> usize {
        self.scans.load(std::sync::atomic::Ordering::SeqCst)
    }
    pub fn new(fs: Arc<StorageService>) -> Self {
        Self {
            fs,
            state: Arc::new(Mutex::new(SelectionState::default())),
            flights: SingleFlight::new(),
            #[cfg(test)]
            scans: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    pub fn invalidate(&self) {
        let mut state = self.state.lock();
        state.revision += 1;
        state.entries.clear();
    }

    async fn version(folder: &ResolvedStoragePath) -> Result<Option<Version>> {
        if let Some(identity) = folder.archive_identity() {
            return Ok(Some(Version::Archive(identity)));
        }
        let entry = folder.get_listing_entry().await?;
        Ok(entry
            .meta
            .modified
            .filter(|modified| Utc::now().signed_duration_since(*modified) >= chrono::Duration::seconds(2))
            .map(Version::Local))
    }

    pub async fn select(&self, folder: &ResolvedStoragePath) -> Result<ResolvedStoragePath> {
        let started = Instant::now();
        let key = SelectionKey {
            path: folder.path().to_nested_path_buf(),
            version: Self::version(folder).await?,
            generation: self.fs.browse_generation(),
            revision: self.state.lock().revision,
        };
        if let Some(path) = self.state.lock().get(&key, Instant::now()) {
            tracing::debug!(target: "pxna::performance", operation = "representative_selection", cache_hit = true, has_candidate = path.is_some(), entry_count = 0_u64, elapsed_sec = started.elapsed().as_secs_f64(), "representative selection completed");
            return Self::target(folder, path);
        }

        let (state, fs, owned, flight_key) = (self.state.clone(), self.fs.clone(), folder.clone(), key.clone());
        let class = WorkClass::current();
        #[cfg(test)]
        let scans = self.scans.clone();
        let result = self.flights.run(key, class.scope(async move {
            // 登録直前に別の flight が公開した場合も、再走査しない。
            if let Some(path) = state.lock().get(&flight_key, Instant::now()) { return Ok(path); }
            #[cfg(test)]
            scans.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let scan_started = Instant::now();
            let mut entries = owned.find_entry_summaries().await?;
            let mut best: Option<String> = None;
            let mut count = 0_u64;
            while let Some(entry) = entries.next().await {
                let entry = entry?;
                count += 1;
                let inner = entry.path.iter().last().unwrap_or_default();
                let name = inner.rsplit(['/', '\\']).next().unwrap_or_default();
                if entry.kind != FsEntryKind::File || name.starts_with('.') || MediaKind::of_name(name).is_none() { continue; }
                if best.as_ref().is_none_or(|current| NaturalOrder::compare(name, current.rsplit(['/', '\\']).next().unwrap_or_default()).is_lt()) {
                    best = Some(inner.to_string());
                }
            }
            let current_version = Self::version(&owned).await?;
            if flight_key.version.is_some() && current_version == flight_key.version && fs.browse_generation() == flight_key.generation {
                let mut state = state.lock();
                if state.revision == flight_key.revision { state.put(flight_key, best.clone(), Instant::now()); }
            }
            tracing::debug!(target: "pxna::performance", operation = "representative_selection", cache_hit = false, has_candidate = best.is_some(), entry_count = count, elapsed_sec = scan_started.elapsed().as_secs_f64(), "representative selection completed");
            Ok(best)
        })).await?;
        Self::target(folder, (*result).clone())
    }

    fn target(folder: &ResolvedStoragePath, path: Option<String>) -> Result<ResolvedStoragePath> {
        match path {
            Some(path) => folder.at(&path),
            None => Err(Error::new(ErrorKind::NotFound).with_message(format!("no image or video in: {}", folder.path()))),
        }
    }
}

impl SelectionState {
    fn get(&mut self, key: &SelectionKey, now: Instant) -> Option<Option<String>> {
        key.version.as_ref()?;
        self.access += 1;
        let entry = self.entries.get_mut(&key.path)?;
        if entry.key != *key || now.duration_since(entry.inserted) >= TTL {
            return None;
        }
        entry.accessed = self.access;
        Some(entry.path.clone())
    }

    fn put(&mut self, key: SelectionKey, path: Option<String>, now: Instant) {
        self.access += 1;
        self.entries.insert(
            key.path.clone(),
            CachedSelection {
                key,
                path,
                inserted: now,
                accessed: self.access,
            },
        );
        while self.entries.len() > CAPACITY {
            let oldest = self.entries.iter().min_by_key(|(_, value)| value.accessed).map(|(key, _)| key.clone()).unwrap();
            self.entries.remove(&oldest);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::nfs::FileStorageLocal, service::ArchiveRuntime};
    use omnius_core_base::{
        clock::ClockUtc,
        tsid::{TsidProvider, TsidProviderImpl},
    };
    use rand::{SeedableRng as _, rngs::ChaCha20Rng};
    use std::{
        path::Path,
        str::FromStr,
        sync::atomic::Ordering,
        time::{SystemTime, UNIX_EPOCH},
    };
    use testresult::TestResult;

    fn service() -> Arc<StorageService> {
        let ids: Arc<Mutex<dyn TsidProvider + Send + Sync>> = Arc::new(Mutex::new(TsidProviderImpl::new(ClockUtc, ChaCha20Rng::seed_from_u64(0), 16)));
        Arc::new(StorageService::new(Arc::new(FileStorageLocal::new(ids)), Arc::new(ArchiveRuntime::default())))
    }
    fn old(path: &Path, seconds: u64) -> TestResult {
        std::fs::File::open(path)?.set_times(std::fs::FileTimes::new().set_modified(UNIX_EPOCH + Duration::from_secs(seconds)))?;
        Ok(())
    }
    async fn folder(fs: &StorageService, path: &Path) -> TestResult<ResolvedStoragePath> {
        Ok(fs.resolve_entry(&NestedPathBuf::try_from(path)?).await?)
    }
    fn key(name: &str) -> SelectionKey {
        SelectionKey {
            path: NestedPathBuf::from_str(name).unwrap(),
            version: Some(Version::Local(DateTime::from_timestamp(100, 0).unwrap())),
            generation: 0,
            revision: 0,
        }
    }

    #[test]
    fn bounded_positive_negative_lru_ttl_and_version_test() {
        let mut state = SelectionState::default();
        let now = Instant::now();
        state.put(key("/negative"), None, now);
        assert_eq!(state.get(&key("/negative"), now), Some(None));
        for i in 0..CAPACITY - 1 {
            state.put(key(&format!("/{i}")), Some("image.png".into()), now);
        }
        assert!(state.get(&key("/negative"), now + Duration::from_secs(29)).is_some());
        state.put(key("/last"), None, now);
        assert_eq!(state.entries.len(), CAPACITY);
        assert!(state.get(&key("/0"), now).is_none());
        assert_eq!(state.get(&key("/negative"), now + TTL), None, "hit で TTL は延長しない");
        let mut changed = key("/last");
        changed.generation = 1;
        assert_eq!(state.get(&changed, now), None);
        changed = key("/last");
        changed.version = None;
        assert_eq!(state.get(&changed, now), None);
        changed.version = Some(Version::Archive(ArchiveIdentity::from_local(
            "/a",
            &crate::service::SourceFingerprint { size: 1, mtime_s: 1, mtime_ns: 0 },
        )));
        assert_eq!(state.get(&changed, now), None);
    }

    #[tokio::test]
    async fn positive_negative_natural_hidden_link_mtime_and_own_change_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let fs = service();
        let cache = RepresentativeSelection::new(fs.clone());
        std::fs::create_dir(dir.path().join("sub"))?;
        std::fs::write(dir.path().join("sub/0.png"), b"x")?;
        std::fs::write(dir.path().join(".0.png"), b"x")?;
        #[cfg(unix)]
        std::os::unix::fs::symlink(".0.png", dir.path().join("0.png"))?;
        old(dir.path(), 100)?;
        let folder = folder(&fs, dir.path()).await?;
        for _ in 0..2 {
            assert_eq!(cache.select(&folder).await.err().unwrap().kind(), &ErrorKind::NotFound);
        }
        assert_eq!(cache.scans.load(Ordering::SeqCst), 1);
        std::fs::write(dir.path().join("10.png"), b"x")?;
        std::fs::write(dir.path().join("2.png"), b"x")?;
        old(dir.path(), 101)?;
        for _ in 0..2 {
            assert!(cache.select(&folder).await?.path().to_string().ends_with("/2.png"));
        }
        assert_eq!(cache.scans.load(Ordering::SeqCst), 2);
        fs.invalidate_browse_caches();
        cache.select(&folder).await?;
        assert_eq!(cache.scans.load(Ordering::SeqCst), 3);
        std::fs::File::open(dir.path())?.set_times(std::fs::FileTimes::new().set_modified(SystemTime::now() + Duration::from_secs(60)))?;
        cache.select(&folder).await?;
        cache.select(&folder).await?;
        assert_eq!(cache.scans.load(Ordering::SeqCst), 5, "未来/直近の版は保持しない");
        Ok(())
    }

    #[tokio::test]
    async fn concurrent_selection_cancelled_waiter_and_old_flight_publication_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("a.png"), b"x")?;
        old(dir.path(), 100)?;
        let fs = service();
        let cache = Arc::new(RepresentativeSelection::new(fs.clone()));
        let folder = folder(&fs, dir.path()).await?;
        let mut permits = Vec::new();
        for _ in 0..12 {
            permits.push(fs.work_budget().enter(WorkClass::Foreground).await?);
        }
        let first = {
            let (cache, folder) = (cache.clone(), folder.clone());
            tokio::spawn(async move { cache.select(&folder).await })
        };
        while cache.scans.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
        let mut waiters = Vec::new();
        for _ in 0..4 {
            let (cache, folder) = (cache.clone(), folder.clone());
            waiters.push(tokio::spawn(async move { cache.select(&folder).await }));
        }
        first.abort();
        let _ = first.await;
        fs.invalidate_browse_caches();
        // 新世代の要求もあれば別 flight を開始できる。古いものの公開は許さない。
        for waiter in waiters {
            waiter.abort();
            let _ = waiter.await;
        }
        permits.clear();
        while cache.flights.in_flight() != 0 {
            tokio::task::yield_now().await;
        }
        assert!(cache.state.lock().entries.is_empty());
        cache.select(&folder).await?;
        cache.select(&folder).await?;
        assert!(cache.scans.load(Ordering::SeqCst) >= 2);
        let after = cache.scans.load(Ordering::SeqCst);
        let tasks = (0..8).map(|_| {
            let (cache, folder) = (cache.clone(), folder.clone());
            tokio::spawn(async move { cache.select(&folder).await })
        });
        for result in futures::future::join_all(tasks).await {
            result??;
        }
        assert_eq!(cache.scans.load(Ordering::SeqCst), after);
        Ok(())
    }

    #[tokio::test]
    async fn uncached_concurrent_waiters_share_selection_after_first_is_cancelled_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("a.png"), b"x")?;
        old(dir.path(), 100)?;
        let fs = service();
        let cache = RepresentativeSelection::new(fs.clone());
        let folder = folder(&fs, dir.path()).await?;
        let mut permits = Vec::new();
        for _ in 0..12 {
            permits.push(fs.work_budget().enter(WorkClass::Foreground).await?);
        }
        let mut first = Box::pin(cache.select(&folder));
        assert!(futures::poll!(&mut first).is_pending());
        while cache.scan_count() == 0 {
            assert!(futures::poll!(&mut first).is_pending());
            tokio::task::yield_now().await;
        }
        let mut rest = (0..4).map(|_| Box::pin(cache.select(&folder))).collect::<Vec<_>>();
        for waiter in &mut rest {
            assert!(futures::poll!(waiter).is_pending());
        }
        drop(first);
        permits.clear();
        for value in futures::future::join_all(rest).await {
            value?;
        }
        assert_eq!(cache.scan_count(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn failed_scan_is_not_cached_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let fs = service();
        let cache = RepresentativeSelection::new(fs.clone());
        #[cfg(unix)]
        let socket = std::os::unix::net::UnixListener::bind(dir.path().join("socket"))?;
        old(dir.path(), 100)?;
        let folder = folder(&fs, dir.path()).await?;
        #[cfg(unix)]
        {
            assert_eq!(cache.select(&folder).await.err().unwrap().kind(), &ErrorKind::UnsupportedType);
            assert!(cache.state.lock().entries.is_empty());
            drop(socket);
            std::fs::remove_file(dir.path().join("socket"))?;
        }
        std::fs::write(dir.path().join("a.png"), b"x")?;
        old(dir.path(), 100)?;
        cache.select(&folder).await?;
        Ok(())
    }
}
