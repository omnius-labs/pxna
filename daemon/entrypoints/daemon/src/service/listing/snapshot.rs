use std::{
    collections::{HashMap, HashSet},
    future::Future,
    sync::Arc,
    time::Duration,
};

use chrono::{DateTime, Utc};
use futures::{
    FutureExt as _,
    future::{BoxFuture, WeakShared},
};
use omnius_core_base::clock::Clock;
use omnius_pxna_engine::service::{ArchiveIdentity, NestedFsEntry, ResolvedStoragePath};
use omnius_pxna_interface::models;

use crate::{
    config::ListingConfig,
    prelude::*,
    service::{
        library::LibraryRelativePath,
        listing::{ListOptions, ListSort, ListingEntries, SummaryIndex},
    },
};

type ScanFuture = BoxFuture<'static, std::result::Result<Arc<ListingSnapshot>, Arc<Error>>>;

/// 変更した Entry と、その祖先を Library ごとに重複なく持つ。
#[derive(Default, Clone)]
pub struct ListingInvalidation {
    ancestors: HashMap<String, HashSet<String>>,
    entries: HashMap<String, HashSet<String>>,
}

impl ListingInvalidation {
    pub fn for_entry(library: &str, path: &str) -> Result<Self> {
        let mut invalidation = Self::default();
        invalidation.include_entry(library, path)?;
        Ok(invalidation)
    }

    pub fn include_entry(&mut self, library: &str, path: &str) -> Result<()> {
        let relative = LibraryRelativePath::parse(path)?;
        let ancestors = self.ancestors.entry(library.to_string()).or_default();
        for end in 0..relative.segments().len() {
            ancestors.insert(format!("/{}", relative.segments()[..end].join("/")));
        }
        self.entries.entry(library.to_string()).or_default().insert(format!("/{}", relative.segments().join("/")));
        Ok(())
    }

    /// 変更した Entry の Library 相対パスを Library ごとに返す。
    pub fn entries(&self) -> HashMap<String, Vec<String>> {
        self.entries.iter().map(|(library, paths)| (library.clone(), paths.iter().cloned().collect())).collect()
    }

    pub fn affects(&self, library: &str, path: &str) -> bool {
        self.ancestors.get(library).is_some_and(|paths| paths.contains(path))
    }

    fn matches(&self, key: &SnapshotKey) -> bool {
        self.affects(&key.library, &key.path)
    }
}

/// 対象と、走査範囲を決める指定。並べ方と名前の条件は含めない。
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SnapshotKey {
    library: String,
    path: String,
    recursive: bool,
    include_hidden: bool,
}

impl SnapshotKey {
    pub fn new(library: &str, path: &str, options: &ListOptions) -> Result<Self> {
        let relative = LibraryRelativePath::parse(path)?;
        Ok(Self {
            library: library.to_string(),
            path: format!("/{}", relative.segments().join("/")),
            recursive: options.recursive(),
            include_hidden: options.include_hidden(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SnapshotVersion {
    Local(Option<DateTime<Utc>>),
    Archive(ArchiveIdentity),
}

impl SnapshotVersion {
    pub fn for_folder(folder: &ResolvedStoragePath, entry: &NestedFsEntry, recursive: bool) -> Option<Self> {
        if recursive {
            return None;
        }
        Some(match folder.archive_identity() {
            Some(identity) => Self::Archive(identity),
            None => Self::Local(entry.meta.modified),
        })
    }

    fn needs_next_refresh(&self, started_at: DateTime<Utc>) -> bool {
        matches!(self, Self::Local(Some(modified)) if started_at.signed_duration_since(*modified) < chrono::Duration::seconds(2))
    }
}

/// 1 回の走査の Entry と、要求された並べ方ごとの索引。
pub struct ListingSnapshot {
    id: String,
    entries: ListingEntries,
    version: Option<SnapshotVersion>,
    created_at: DateTime<Utc>,
    refresh_next: bool,
    orders: Mutex<HashMap<ListSort, Arc<Vec<usize>>>>,
}

impl ListingSnapshot {
    pub fn page(&self, options: &ListOptions) -> models::FolderListing {
        options.page(&self.entries, &self.order(options.sort()), self.id.clone())
    }

    /// 条件に合う Entry の位置を、並べ方の順で返す。
    pub fn matching(&self, options: &ListOptions) -> Vec<usize> {
        options.matching(&self.entries, &self.order(options.sort()))
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn entries(&self) -> &ListingEntries {
        &self.entries
    }

    fn order(&self, sort: ListSort) -> Arc<Vec<usize>> {
        self.orders.lock().entry(sort).or_insert_with(|| Arc::new(sort.indices(&self.entries))).clone()
    }

    fn fresh(&self, version: &Option<SnapshotVersion>, now: DateTime<Utc>, max_age: Duration) -> bool {
        !self.refresh_next && self.version == *version && now.signed_duration_since(self.created_at).to_std().unwrap_or_default() < max_age
    }
}

struct CachedSnapshot {
    snapshot: Arc<ListingSnapshot>,
    last_used: DateTime<Utc>,
}

struct ScanFlight {
    id: String,
    version: Option<SnapshotVersion>,
    future: WeakShared<ScanFuture>,
}

#[derive(Default)]
struct SnapshotStore {
    cached: HashMap<SnapshotKey, CachedSnapshot>,
    flights: HashMap<SnapshotKey, ScanFlight>,
}

impl SnapshotStore {
    fn prune(&mut self, now: DateTime<Utc>, config: &ListingConfig) {
        self.cached.retain(|_, cached| {
            now.signed_duration_since(cached.last_used).to_std().unwrap_or_default() < Duration::from_secs(config.snapshot_idle_secs)
                && now.signed_duration_since(cached.snapshot.created_at).to_std().unwrap_or_default() < Duration::from_secs(config.snapshot_max_age_secs)
        });
        self.flights.retain(|_, flight| flight.future.upgrade().is_some());
    }

    fn insert(&mut self, key: SnapshotKey, snapshot: Arc<ListingSnapshot>, now: DateTime<Utc>, capacity: usize) {
        if snapshot.entries.len() > capacity {
            return;
        }
        let mut count: usize = self.cached.values().map(|cached| cached.snapshot.entries.len()).sum();
        while count > capacity - snapshot.entries.len() {
            let Some(oldest) = self.cached.iter().min_by_key(|(_, cached)| cached.last_used).map(|(key, _)| key.clone()) else {
                break;
            };
            count -= self.cached.remove(&oldest).unwrap().snapshot.entries.len();
        }
        self.cached.insert(key, CachedSnapshot { snapshot, last_used: now });
    }
}

/// 完成済みの一覧と、要求が共有する走査を所有する。
pub struct ListingSnapshots {
    child_probes: Arc<crate::service::library::ChildProbeCache>,
    config: ListingConfig,
    clock: Arc<dyn Clock<Utc> + Send + Sync>,
    store: Mutex<SnapshotStore>,
    summaries: Arc<SummaryIndex>,
}

impl ListingSnapshots {
    pub fn child_probes(&self) -> &Arc<crate::service::library::ChildProbeCache> {
        &self.child_probes
    }

    pub async fn invalidate(&self, invalidation: &ListingInvalidation) {
        {
            let mut store = self.store.lock();
            store.cached.retain(|key, _| !invalidation.matches(key));
            store.flights.retain(|key, _| !invalidation.matches(key));
        }
        self.child_probes.invalidate(invalidation);
        self.summaries.invalidate(invalidation).await;
    }

    #[cfg(test)]
    pub fn new(config: ListingConfig, clock: Arc<dyn Clock<Utc> + Send + Sync>) -> Arc<Self> {
        let summaries = SummaryIndex::new(config.clone(), clock.clone());
        Self::from_parts(config, clock, summaries)
    }

    pub async fn open(config: ListingConfig, clock: Arc<dyn Clock<Utc> + Send + Sync>, dir: &std::path::Path, budget: Arc<omnius_pxna_engine::service::WorkBudget>) -> Arc<Self> {
        let summaries = SummaryIndex::open_with_budget(config.clone(), clock.clone(), dir, budget).await;
        Self::from_parts(config, clock, summaries)
    }

    fn from_parts(config: ListingConfig, clock: Arc<dyn Clock<Utc> + Send + Sync>, summaries: Arc<SummaryIndex>) -> Arc<Self> {
        let interval_secs = config.snapshot_idle_secs.min(config.snapshot_max_age_secs).clamp(1, 60);
        let snapshots = Arc::new(Self {
            child_probes: Arc::new(Default::default()),
            summaries,
            config,
            clock,
            store: Mutex::new(SnapshotStore::default()),
        });
        let weak = Arc::downgrade(&snapshots);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
            loop {
                interval.tick().await;
                let Some(snapshots) = weak.upgrade() else {
                    break;
                };
                snapshots.store.lock().prune(snapshots.clock.now(), &snapshots.config);
            }
        });
        snapshots
    }

    pub fn summaries(&self) -> Arc<SummaryIndex> {
        self.summaries.clone()
    }

    pub fn recursive_max_entries(&self) -> usize {
        self.config.recursive_max_entries
    }

    pub async fn get_or_scan<F, Fut>(self: &Arc<Self>, key: SnapshotKey, version: Option<SnapshotVersion>, scan: F) -> Result<Arc<ListingSnapshot>>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<ListingEntries>> + Send + 'static,
    {
        let future = {
            let now = self.clock.now();
            let mut store = self.store.lock();
            store.prune(now, &self.config);
            if let Some(cached) = store.cached.get_mut(&key)
                && cached.snapshot.fresh(&version, now, Duration::from_secs(self.config.snapshot_max_age_secs))
            {
                cached.last_used = now;
                return Ok(cached.snapshot.clone());
            }
            store.cached.remove(&key);
            let existing = store
                .flights
                .get(&key)
                .filter(|flight| flight.version == version)
                .and_then(|flight| flight.future.upgrade());
            match existing {
                Some(future) => future,
                None => {
                    let owner = self.clone();
                    let flight_id = uuid::Uuid::new_v4().to_string();
                    let id = flight_id.clone();
                    let scan_key = key.clone();
                    let scan_version = version.clone();
                    let future: ScanFuture = async move {
                        let entries = scan().await.map_err(Arc::new)?;
                        let created_at = owner.clock.now();
                        let snapshot = Arc::new(ListingSnapshot {
                            id,
                            entries,
                            refresh_next: scan_version.as_ref().is_some_and(|version| version.needs_next_refresh(now)),
                            version: scan_version,
                            created_at,
                            orders: Mutex::new(HashMap::new()),
                        });
                        let mut store = owner.store.lock();
                        // 破棄や版の変更で別の走査が始まった場合、古い走査の結果は保持しない。
                        if store.flights.get(&scan_key).is_some_and(|flight| flight.id == snapshot.id) {
                            store.flights.remove(&scan_key);
                            store.prune(created_at, &owner.config);
                            store.insert(scan_key, snapshot.clone(), created_at, owner.config.snapshot_capacity_entries);
                        }
                        Ok(snapshot)
                    }
                    .boxed();
                    let future = future.shared();
                    // 集合は弱参照だけを持つ。待つ要求がすべて消えれば走査も Drop される。
                    store.flights.insert(
                        key,
                        ScanFlight {
                            id: flight_id,
                            version,
                            future: future.downgrade().unwrap(),
                        },
                    );
                    future
                }
            }
        };
        future.await.map_err(|error| Error::from_error(error.clone(), error.kind().clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::listing::tests::{entry, query};
    use futures::poll;
    use omnius_core_base::clock::FakeClockUtc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    fn clock() -> Arc<FakeClockUtc> {
        Arc::new(FakeClockUtc::new(DateTime::from_timestamp(1_800_000_000, 0).unwrap()))
    }

    fn options(offset: u64) -> ListOptions {
        ListOptions::from_query(&query(Some(offset), Some(1))).unwrap()
    }
    fn key(path: &str) -> SnapshotKey {
        SnapshotKey::new("test", path, &options(0)).unwrap()
    }
    fn entries(count: usize) -> ListingEntries {
        ListingEntries::from_entries((0..count).map(|index| entry(&format!("{index}.txt"))).collect())
    }
    async fn get(snapshots: &Arc<ListingSnapshots>, path: &str, count: usize) -> Arc<ListingSnapshot> {
        snapshots.get_or_scan(key(path), None, move || async move { Ok(entries(count)) }).await.unwrap()
    }

    #[tokio::test]
    async fn second_page_does_not_rescan_test() -> testresult::TestResult {
        let snapshots = ListingSnapshots::new(Default::default(), clock());
        let calls = Arc::new(AtomicUsize::new(0));
        let mut pages = Vec::new();
        for offset in 0..2 {
            let calls = calls.clone();
            let snapshot = snapshots
                .get_or_scan(key("/"), None, move || async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok(entries(3))
                })
                .await?;
            pages.push(snapshot.page(&options(offset)));
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(pages[0].snapshot, pages[1].snapshot);
        assert_eq!(pages[1].entries[0].name, "1.txt");
        Ok(())
    }

    #[tokio::test]
    async fn external_change_rebuilds_snapshot_test() -> testresult::TestResult {
        let clock = Arc::new(FakeClockUtc::new(Utc::now() + chrono::Duration::seconds(10)));
        let (_dir, state) = crate::service::mutation::tests::state_with_clock(Default::default(), clock).await?;
        let root = &state.conf.libraries[0].path;
        let options = options(0);
        let first = state.library.list_folder_entries("test", "/", &options).await?;
        let modified = std::fs::metadata(root)?.modified()?;
        std::fs::write(root.join("new.txt"), b"new")?;
        // 更新時刻の精度に依存せず、外部で版が変わった状態を作る。
        filetime::set_file_mtime(root, filetime::FileTime::from_system_time(modified + Duration::from_secs(3)))?;
        let second = state.library.list_folder_entries("test", "/", &options).await?;
        assert_ne!(first.snapshot, second.snapshot);
        assert_eq!(second.entries[0].name, "new.txt");
        let third = state.library.list_folder_entries("test", "/", &options).await?;
        assert_eq!(second.snapshot, third.snapshot);
        Ok(())
    }

    #[tokio::test]
    async fn capacity_evicts_least_recently_used_test() {
        let clock = clock();
        let config = ListingConfig {
            snapshot_capacity_entries: 5,
            recursive_max_entries: 2,
            ..Default::default()
        };
        let snapshots = ListingSnapshots::new(config, clock.clone());
        let a = get(&snapshots, "/a", 2).await;
        clock.advance(Duration::from_secs(1));
        let b = get(&snapshots, "/b", 2).await;
        clock.advance(Duration::from_secs(1));
        assert_eq!(a.id, get(&snapshots, "/a", 2).await.id);
        clock.advance(Duration::from_secs(1));
        get(&snapshots, "/c", 2).await;
        assert!(snapshots.store.lock().cached.values().map(|cached| cached.snapshot.entries.len()).sum::<usize>() <= 5);
        assert_eq!(a.id, get(&snapshots, "/a", 2).await.id);
        assert_ne!(b.id, get(&snapshots, "/b", 2).await.id);
        let oversized = get(&snapshots, "/huge", 6).await;
        assert_eq!(oversized.entries.len(), 6);
        assert!(!snapshots.store.lock().cached.contains_key(&key("/huge")));
        assert_ne!(oversized.id, get(&snapshots, "/huge", 6).await.id);
    }

    #[tokio::test]
    async fn max_age_and_idle_expire_test() {
        let clock = clock();
        let snapshots = ListingSnapshots::new(
            ListingConfig {
                snapshot_max_age_secs: 5,
                snapshot_idle_secs: 3,
                ..Default::default()
            },
            clock.clone(),
        );
        let first = get(&snapshots, "/", 1).await;
        clock.advance(Duration::from_secs(2));
        assert_eq!(first.id, get(&snapshots, "/", 1).await.id);
        clock.advance(Duration::from_secs(2));
        assert_eq!(first.id, get(&snapshots, "/", 1).await.id);
        clock.advance(Duration::from_secs(1));
        let aged = get(&snapshots, "/", 1).await;
        assert_ne!(first.id, aged.id);
        clock.advance(Duration::from_secs(3));
        assert_ne!(aged.id, get(&snapshots, "/", 1).await.id);
    }

    #[tokio::test]
    async fn recent_local_mtime_refreshes_next_request_test() -> testresult::TestResult {
        let clock = clock();
        let snapshots = ListingSnapshots::new(Default::default(), clock.clone());
        let version = Some(SnapshotVersion::Local(Some(clock.now() - chrono::Duration::seconds(1))));
        let first = snapshots.get_or_scan(key("/"), version.clone(), || async { Ok(entries(1)) }).await?;
        let second = snapshots.get_or_scan(key("/"), version.clone(), || async { Ok(entries(1)) }).await?;
        assert_ne!(first.id, second.id);
        clock.advance(Duration::from_secs(1));
        let third = snapshots.get_or_scan(key("/"), version.clone(), || async { Ok(entries(1)) }).await?;
        let fourth = snapshots.get_or_scan(key("/"), version, || async { Ok(entries(1)) }).await?;
        assert_eq!(third.id, fourth.id);
        Ok(())
    }

    #[tokio::test]
    async fn sort_and_query_share_snapshot_test() {
        let snapshots = ListingSnapshots::new(Default::default(), clock());
        let snapshot = get(&snapshots, "/", 3).await;
        snapshot.page(&options(0));
        snapshot.page(&options(1));
        assert_eq!(snapshot.orders.lock().len(), 1);
        let mut q = query(None, Some(3));
        q.query = Some("1".to_string());
        let filtered = snapshot.page(&ListOptions::from_query(&q).unwrap());
        assert_eq!(filtered.total_count, 1);
        assert_eq!(filtered.snapshot, snapshot.id);
        assert_eq!(snapshot.orders.lock().len(), 1);
        q.order = Some("desc".to_string());
        snapshot.page(&ListOptions::from_query(&q).unwrap());
        assert_eq!(snapshot.orders.lock().len(), 2);
    }

    struct DropProbe(Arc<AtomicBool>);
    impl Drop for DropProbe {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn walk_stops_when_all_waiters_leave_test() {
        let snapshots = ListingSnapshots::new(Default::default(), clock());
        let dropped = Arc::new(AtomicBool::new(false));
        let scanned = Arc::new(AtomicUsize::new(0));
        let stopped = dropped.clone();
        let calls = scanned.clone();
        let mut first = snapshots
            .get_or_scan(key("/"), None, move || async move {
                let _probe = DropProbe(stopped);
                calls.fetch_add(1, Ordering::SeqCst);
                futures::future::pending::<()>().await;
                Ok(entries(1))
            })
            .boxed();
        assert!(poll!(first.as_mut()).is_pending());
        let calls = scanned.clone();
        let mut second = snapshots
            .get_or_scan(key("/"), None, move || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(entries(1))
            })
            .boxed();
        assert!(poll!(second.as_mut()).is_pending());
        assert_eq!(scanned.load(Ordering::SeqCst), 1);
        drop(first);
        assert!(!dropped.load(Ordering::SeqCst));
        drop(second);
        assert!(dropped.load(Ordering::SeqCst));
        let rebuilt = get(&snapshots, "/", 1).await;
        assert_eq!(rebuilt.entries.len(), 1);
    }

    #[tokio::test]
    async fn simultaneous_requests_share_one_walk_test() -> testresult::TestResult {
        let snapshots = ListingSnapshots::new(Default::default(), clock());
        let calls = Arc::new(AtomicUsize::new(0));
        let count = calls.clone();
        let first = snapshots.get_or_scan(key("/"), None, move || async move {
            count.fetch_add(1, Ordering::SeqCst);
            tokio::task::yield_now().await;
            Ok(entries(2))
        });
        let count = calls.clone();
        let second = snapshots.get_or_scan(key("/"), None, move || async move {
            count.fetch_add(1, Ordering::SeqCst);
            Ok(entries(2))
        });
        let (first, second) = tokio::join!(first, second);
        assert_eq!(first?.id, second?.id);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        Ok(())
    }

    #[tokio::test]
    async fn invalidation_removes_all_ancestor_variants_test() -> testresult::TestResult {
        let snapshots = ListingSnapshots::new(Default::default(), clock());
        let mut invalidated = Vec::new();
        for path in ["/", "/a", "/a/b"] {
            for recursive in [false, true] {
                for hidden in [false, true] {
                    let mut q = query(None, None);
                    q.recursive = Some(recursive);
                    q.include_hidden = Some(hidden);
                    let key = SnapshotKey::new("test", path, &ListOptions::from_query(&q)?)?;
                    snapshots.get_or_scan(key.clone(), None, || async { Ok(entries(1)) }).await?;
                    invalidated.push(key);
                }
            }
        }
        let sibling = get(&snapshots, "/a-other", 1).await;
        let other_key = SnapshotKey::new("other", "/", &options(0))?;
        let other = snapshots.get_or_scan(other_key.clone(), None, || async { Ok(entries(1)) }).await?;
        let changes = ListingInvalidation::for_entry("test", "/a/x/../b/changed.txt")?;
        snapshots.invalidate(&changes).await;
        let store = snapshots.store.lock();
        assert!(invalidated.iter().all(|key| !store.cached.contains_key(key)));
        assert_eq!(store.cached[&key("/a-other")].snapshot.id, sibling.id);
        assert_eq!(store.cached[&other_key].snapshot.id, other.id);
        Ok(())
    }

    #[tokio::test]
    async fn invalidated_walk_cannot_restore_old_snapshot_test() -> testresult::TestResult {
        let snapshots = ListingSnapshots::new(Default::default(), clock());
        let (send, receive) = futures::channel::oneshot::channel();
        let mut old = snapshots
            .get_or_scan(key("/"), None, move || async move {
                receive.await.unwrap();
                Ok(entries(1))
            })
            .boxed();
        assert!(poll!(old.as_mut()).is_pending());
        snapshots.invalidate(&ListingInvalidation::for_entry("test", "/changed.txt")?).await;
        let new = get(&snapshots, "/", 2).await;
        send.send(()).unwrap();
        assert_ne!(old.await?.id, new.id);
        assert_eq!(get(&snapshots, "/", 2).await.id, new.id);
        Ok(())
    }
}
