use std::{
    collections::HashMap,
    future::Future,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use chrono::{DateTime, Utc};
use futures::{
    FutureExt as _, StreamExt as _,
    future::{BoxFuture, WeakShared},
};
use omnius_core_base::clock::Clock;
use omnius_pxna_engine::{
    perf_span,
    service::{ResolvedStoragePath, WorkBudget, WorkKind},
};
use omnius_pxna_interface::models;
use serde::{Deserialize, Serialize};
use tracing::{Instrument as _, Span, field::Empty};

use super::{EntryPosition, ListOptions, ListSort, ListSortKey, ListingInvalidation, MarkFilter, MarkSelection, NameFilter, SnapshotVersion, SortValue};
use crate::{
    config::ListingConfig,
    prelude::*,
    service::library::{EntryConverter, LibraryRelativePath},
};

mod persistence;
use persistence::SummaryPersistence;

type SummaryFuture = BoxFuture<'static, std::result::Result<Arc<SummarySnapshot>, Arc<Error>>>;

/// 軽量一覧の条件。metadata が必要な並びを受け入れない。
pub struct SummaryOptions(ListOptions);

impl SummaryOptions {
    pub fn from_query(query: &models::ListFolderSummariesQueryParams) -> Result<Self> {
        let sort = ListSort::parse(query.sort.as_deref(), query.order.as_deref(), query.folders_first.unwrap_or(false))?;
        if !matches!(sort.key, ListSortKey::Name | ListSortKey::Kind) {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("summary sort must be name or kind"));
        }
        Ok(Self(ListOptions {
            recursive: false,
            include_hidden: query.include_hidden.unwrap_or(false),
            folders_only: query.folders_only.unwrap_or(false),
            sort,
            filter: NameFilter::new(query.query.as_deref().unwrap_or_default()),
            marks: MarkFilter::default(),
            selection: MarkSelection::default(),
            offset: query.offset.unwrap_or(0),
            limit: ListOptions::parse_limit(query.limit)?,
        }))
    }

    pub fn include_hidden(&self) -> bool {
        self.0.include_hidden
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SummaryKey {
    library: String,
    root: PathBuf,
    path: String,
    include_hidden: bool,
}

impl SummaryKey {
    pub fn library(&self) -> &str {
        &self.library
    }

    pub fn new(library: &str, root: &Path, path: &str, options: &SummaryOptions) -> Result<Self> {
        Self::for_probe(library, root, path, options.include_hidden())
    }
    pub fn for_probe(library: &str, root: &Path, path: &str, include_hidden: bool) -> Result<Self> {
        let relative = LibraryRelativePath::parse(path)?;
        Ok(Self {
            library: library.to_string(),
            root: root.to_path_buf(),
            path: format!("/{}", relative.segments().join("/")),
            include_hidden,
        })
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct SummaryEntry {
    entry: models::FileSystemEntrySummary,
    normalized: String,
}

/// 一度だけ整列した軽量な列。詳細の到着では位置も id も変えない。
#[derive(Clone, Serialize, Deserialize)]
pub struct SummaryData {
    entries: Vec<SummaryEntry>,
    name_order: Vec<usize>,
    kind_order: Vec<usize>,
    folder_count: u64,
}

impl SummaryData {
    pub async fn scan(folder: &ResolvedStoragePath, root: &Path, include_hidden: bool, budget: &WorkBudget) -> Result<Self> {
        let span = perf_span!("summary_scan", entry_count = Empty);
        Self::scan_traced(folder, root, include_hidden, budget).instrument(span).await
    }

    async fn scan_traced(folder: &ResolvedStoragePath, root: &Path, include_hidden: bool, budget: &WorkBudget) -> Result<Self> {
        let mut stream = folder.find_entry_summaries().await?;
        let converter = EntryConverter::new(root);
        let mut entries = Vec::new();
        while let Some(entry) = stream.next().await {
            let entry = converter.convert_summary(&entry?)?;
            if !include_hidden && entry.name.starts_with('.') {
                continue;
            }
            entries.push(SummaryEntry {
                normalized: NameFilter::normalize(&entry.name),
                entry,
            });
        }
        let data = budget
            .io()
            .run_blocking(WorkKind::Summary, move || Self::new(entries))
            .await
            .map_err(|error| error.with_message("cannot sort summary"))?;
        Span::current().record("entry_count", data.entries.len());
        Ok(data)
    }

    fn new(entries: Vec<SummaryEntry>) -> Self {
        let order = |sort_key| {
            let sort = ListSort {
                key: sort_key,
                descending: false,
                folders_first: false,
            };
            let positions: Vec<_> = entries.iter().map(|listed| Self::position(&listed.entry, sort_key)).collect();
            let mut indices: Vec<_> = (0..entries.len()).collect();
            indices.sort_by(|&a, &b| sort.compare(&positions[a], &positions[b]));
            indices
        };
        let name_order = order(ListSortKey::Name);
        let kind_order = order(ListSortKey::Kind);
        let folder_count = entries.iter().filter(|entry| Self::folder_like(&entry.entry)).count() as u64;
        Self {
            entries,
            name_order,
            kind_order,
            folder_count,
        }
    }

    fn valid_for(&self, key: &SummaryKey) -> bool {
        let mut paths = std::collections::HashSet::new();
        for listed in &self.entries {
            let entry = &listed.entry;
            let Ok(path) = LibraryRelativePath::parse(&entry.path) else {
                return false;
            };
            let segments = path.segments();
            if segments.is_empty()
                || format!("/{}", segments.join("/")) != entry.path
                || format!("/{}", segments[..segments.len() - 1].join("/")) != key.path
                || segments.last() != Some(&entry.name)
                || !paths.insert(&entry.path)
                || (!key.include_hidden && entry.name.starts_with('.'))
                || (entry.archive && entry.kind != models::FileSystemEntryKind::File)
                || listed.normalized != NameFilter::normalize(&entry.name)
            {
                return false;
            }
        }
        if self.folder_count != self.entries.iter().filter(|entry| Self::folder_like(&entry.entry)).count() as u64 {
            return false;
        }
        for (order, key) in [(&self.name_order, ListSortKey::Name), (&self.kind_order, ListSortKey::Kind)] {
            if order.len() != self.entries.len() {
                return false;
            }
            let mut seen = vec![false; self.entries.len()];
            for &index in order {
                if index >= seen.len() || std::mem::replace(&mut seen[index], true) {
                    return false;
                }
            }
            let sort = ListSort {
                key,
                descending: false,
                folders_first: false,
            };
            if order.windows(2).any(|pair| {
                sort.compare(&Self::position(&self.entries[pair[0]].entry, key), &Self::position(&self.entries[pair[1]].entry, key))
                    .is_gt()
            }) {
                return false;
            }
        }
        true
    }

    fn folder_like(entry: &models::FileSystemEntrySummary) -> bool {
        entry.kind == models::FileSystemEntryKind::Folder || entry.archive
    }

    fn position(entry: &models::FileSystemEntrySummary, key: ListSortKey) -> EntryPosition {
        EntryPosition {
            folder_like: Self::folder_like(entry),
            path: entry.path.clone(),
            key: if key == ListSortKey::Kind {
                SortValue::Kind(ListSort::kind_rank(&entry.kind))
            } else {
                SortValue::Name(entry.name.clone())
            },
        }
    }

    fn page(&self, options: &SummaryOptions, id: &str) -> models::FolderSummaryListing {
        let options = &options.0;
        let base = if options.sort.key == ListSortKey::Name { &self.name_order } else { &self.kind_order };
        let groups = if options.sort.folders_first { vec![Some(true), Some(false)] } else { vec![None] };
        let mut total = 0_u64;
        let mut page = Vec::new();
        for group in groups {
            let indices: Box<dyn Iterator<Item = &usize>> = if options.sort.descending { Box::new(base.iter().rev()) } else { Box::new(base.iter()) };
            for &index in indices {
                let listed = &self.entries[index];
                let folder = Self::folder_like(&listed.entry);
                if group.is_some_and(|expected| folder != expected) || (options.folders_only && !folder) || !options.filter.matches(&listed.normalized) {
                    continue;
                }
                if total >= options.offset && page.len() < options.limit {
                    page.push(listed.entry.clone());
                }
                total += 1;
            }
        }
        models::FolderSummaryListing::new(page, total, id.to_string(), false)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SummarySnapshot {
    schema: u32,
    id: String,
    key: SummaryKey,
    version: Option<String>,
    created_at: DateTime<Utc>,
    data: SummaryData,
}

impl SummarySnapshot {
    pub fn page(&self, options: &SummaryOptions) -> models::FolderSummaryListing {
        self.data.page(options, &self.id)
    }

    fn fresh(&self, version: &Option<String>, now: DateTime<Utc>, config: &ListingConfig) -> bool {
        version.is_some()
            && &self.version == version
            && now >= self.created_at
            && now.signed_duration_since(self.created_at).to_std().unwrap_or_default() < Duration::from_secs(config.snapshot_max_age_secs)
    }
}

struct CachedSummary {
    snapshot: Arc<SummarySnapshot>,
    used: DateTime<Utc>,
}
struct SummaryFlight {
    id: String,
    version: Option<String>,
    future: WeakShared<SummaryFuture>,
}
#[derive(Default)]
struct SummaryStore {
    cached: HashMap<SummaryKey, CachedSummary>,
    flights: HashMap<SummaryKey, SummaryFlight>,
}

impl SummaryStore {
    fn prune(&mut self, now: DateTime<Utc>, config: &ListingConfig) {
        self.cached.retain(|_, value| {
            value.snapshot.fresh(&value.snapshot.version, now, config)
                && now.signed_duration_since(value.used).to_std().unwrap_or_default() < Duration::from_secs(config.snapshot_idle_secs)
        });
        self.flights.retain(|_, value| value.future.upgrade().is_some());
    }

    fn insert(&mut self, snapshot: Arc<SummarySnapshot>, now: DateTime<Utc>, capacity: usize) {
        let len = snapshot.data.entries.len().max(1);
        if len > capacity || snapshot.version.is_none() {
            return;
        }
        let mut total: usize = self.cached.values().map(|cached| cached.snapshot.data.entries.len().max(1)).sum();
        while total > capacity - len {
            let Some(key) = self.cached.iter().min_by_key(|(_, value)| value.used).map(|(key, _)| key.clone()) else {
                break;
            };
            total -= self.cached.remove(&key).unwrap().snapshot.data.entries.len().max(1);
        }
        self.cached.insert(snapshot.key.clone(), CachedSummary { snapshot, used: now });
    }
}

/// Library 単位の世代で、失効前に開始した保存の復活を防ぐ。
#[derive(Default)]
struct SummaryEpochs(Mutex<HashMap<String, u64>>);
impl SummaryEpochs {
    fn current(&self, library: &str) -> u64 {
        *self.0.lock().get(library).unwrap_or(&0)
    }
    fn advance(&self, invalidation: &ListingInvalidation) {
        let mut epochs = self.0.lock();
        for library in invalidation.entries().keys() {
            let value = epochs.entry(library.clone()).or_default();
            *value = value.wrapping_add(1);
        }
    }
}

/// 同一の軽量一覧を要求間で共有する。詳細一覧の metadata を仮値へ置き換えない。
pub struct SummaryIndex {
    config: ListingConfig,
    clock: Arc<dyn Clock<Utc> + Send + Sync>,
    store: Mutex<SummaryStore>,
    persistence: Option<SummaryPersistence>,
    epochs: Arc<SummaryEpochs>,
}

impl SummaryIndex {
    #[cfg(test)]
    pub fn new(config: ListingConfig, clock: Arc<dyn Clock<Utc> + Send + Sync>) -> Arc<Self> {
        Self::from_parts(config, clock, None, Arc::new(SummaryEpochs::default()))
    }

    #[cfg(test)]
    pub async fn open(config: ListingConfig, clock: Arc<dyn Clock<Utc> + Send + Sync>, dir: &Path) -> Arc<Self> {
        Self::open_with_budget(config, clock, dir, Arc::new(omnius_pxna_engine::service::WorkBudget::default())).await
    }
    pub async fn open_with_budget(config: ListingConfig, clock: Arc<dyn Clock<Utc> + Send + Sync>, dir: &Path, budget: Arc<omnius_pxna_engine::service::WorkBudget>) -> Arc<Self> {
        let epochs = Arc::new(SummaryEpochs::default());
        let persistence = SummaryPersistence::open_with_budget(dir, config.clone(), clock.clone(), epochs.clone(), budget).await;
        Self::from_parts(config, clock, persistence, epochs)
    }

    fn from_parts(config: ListingConfig, clock: Arc<dyn Clock<Utc> + Send + Sync>, persistence: Option<SummaryPersistence>, epochs: Arc<SummaryEpochs>) -> Arc<Self> {
        let interval_secs = config.snapshot_idle_secs.min(config.snapshot_max_age_secs).clamp(1, 60);
        let index = Arc::new(Self {
            config,
            clock,
            store: Mutex::new(SummaryStore::default()),
            persistence,
            epochs,
        });
        let weak = Arc::downgrade(&index);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
            loop {
                interval.tick().await;
                let Some(index) = weak.upgrade() else {
                    break;
                };
                index.store.lock().prune(index.clock.now(), &index.config);
            }
        });
        index
    }

    pub async fn invalidate(&self, invalidation: &ListingInvalidation) {
        self.epochs.advance(invalidation);
        {
            let mut store = self.store.lock();
            store.cached.retain(|key, _| !invalidation.affects(&key.library, &key.path));
            store.flights.retain(|key, _| !invalidation.affects(&key.library, &key.path));
        }
        if let Some(persistence) = &self.persistence {
            persistence.invalidate(invalidation).await;
        }
    }

    fn version(version: &Option<SnapshotVersion>, now: DateTime<Utc>) -> Option<String> {
        match version {
            Some(SnapshotVersion::Local(Some(modified))) if now.signed_duration_since(*modified) >= chrono::Duration::seconds(2) => {
                Some(format!("local:{}:{}", modified.timestamp(), modified.timestamp_subsec_nanos()))
            }
            Some(SnapshotVersion::Archive(identity)) => Some(format!("archive:{}", identity.to_hex())),
            _ => None,
        }
    }

    pub fn cached_folder_children(&self, key: &SummaryKey, version: Option<SnapshotVersion>) -> Option<bool> {
        let now = self.clock.now();
        let version = Self::version(&version, now);
        let store = self.store.lock();
        let cached = store.cached.get(key)?;
        cached.snapshot.fresh(&version, now, &self.config).then_some(cached.snapshot.data.folder_count > 0)
    }

    pub async fn get_or_scan<F, Fut>(self: &Arc<Self>, key: SummaryKey, version: Option<SnapshotVersion>, scan: F) -> Result<Arc<SummarySnapshot>>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<SummaryData>> + Send + 'static,
    {
        let future = {
            let now = self.clock.now();
            let version = Self::version(&version, now);
            let epoch = self.epochs.current(&key.library);
            let mut store = self.store.lock();
            store.prune(now, &self.config);
            if let Some(cached) = store.cached.get_mut(&key)
                && cached.snapshot.fresh(&version, now, &self.config)
            {
                cached.used = now;
                if let Some(persistence) = &self.persistence {
                    persistence.touch(key.clone(), epoch);
                }
                return Ok(cached.snapshot.clone());
            }
            store.cached.remove(&key);
            if let Some(future) = store.flights.get(&key).filter(|value| value.version == version).and_then(|value| value.future.upgrade()) {
                future
            } else {
                let owner = self.clone();
                let id = uuid::Uuid::new_v4().to_string();
                let flight_id = id.clone();
                let scan_key = key.clone();
                let scan_version = version.clone();
                let publish_id = flight_id.clone();
                let future: SummaryFuture = async move {
                    let loaded = if let Some(persistence) = &owner.persistence {
                        persistence.load(&scan_key, &scan_version, owner.clock.now()).await
                    } else {
                        None
                    };
                    let was_loaded = loaded.is_some();
                    let snapshot = if let Some(snapshot) = loaded {
                        Arc::new(snapshot)
                    } else {
                        let data = scan().await.map_err(Arc::new)?;
                        Arc::new(SummarySnapshot {
                            schema: 1,
                            id,
                            key: scan_key.clone(),
                            version: scan_version,
                            created_at: owner.clock.now(),
                            data,
                        })
                    };
                    let now = owner.clock.now();
                    let mut store = owner.store.lock();
                    if owner.epochs.current(&scan_key.library) == epoch && store.flights.get(&scan_key).is_some_and(|flight| flight.id == publish_id) {
                        store.flights.remove(&scan_key);
                        store.prune(now, &owner.config);
                        store.insert(snapshot.clone(), now, owner.config.snapshot_capacity_entries);
                        if let Some(persistence) = &owner.persistence {
                            if was_loaded {
                                persistence.touch(scan_key, epoch);
                            } else {
                                persistence.save(snapshot.clone(), epoch);
                            }
                        }
                    }
                    Ok(snapshot)
                }
                .boxed();
                let future = future.shared();
                store.flights.insert(
                    key,
                    SummaryFlight {
                        id: flight_id,
                        version,
                        future: future.downgrade().unwrap(),
                    },
                );
                future
            }
        };
        future.await.map_err(|error| Error::from_error(error.clone(), error.kind().clone()))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    use futures::poll;
    use omnius_core_base::clock::FakeClockUtc;
    use testresult::TestResult;

    use super::*;

    fn options(value: serde_json::Value) -> SummaryOptions {
        SummaryOptions::from_query(&serde_json::from_value(value).unwrap()).unwrap()
    }

    fn data() -> SummaryData {
        SummaryData::new(vec![SummaryEntry {
            entry: models::FileSystemEntrySummary::new("/a".into(), "a".into(), models::FileSystemEntryKind::File, false),
            normalized: "a".into(),
        }])
    }

    #[tokio::test]
    async fn summaries_match_detailed_order_filters_and_pages_test() -> TestResult {
        let clock = Arc::new(FakeClockUtc::new(Utc::now() + chrono::Duration::seconds(10)));
        let (_dir, state) = crate::service::mutation::tests::state_with_clock(Default::default(), clock).await?;
        let root = &state.conf.libraries[0].path;
        for name in ["folder2", "folder10", ".hidden"] {
            std::fs::create_dir(root.join(name))?;
        }
        for name in ["a2.txt", "a10.txt", "Ａ.zip", "が.txt", "か\u{3099}.txt"] {
            std::fs::write(root.join(name), b"x")?;
        }
        for sort in ["name", "kind"] {
            for order in ["asc", "desc"] {
                for first in [false, true] {
                    for only in [false, true] {
                        for hidden in [false, true] {
                            for filter in ["", "a", "が"] {
                                for offset in [0, 2, 50] {
                                    let value = serde_json::json!({"path":"/", "sort":sort, "order":order, "foldersFirst":first,
                                        "foldersOnly":only, "includeHidden":hidden, "query":filter, "offset":offset, "limit":2});
                                    let full = ListOptions::from_query(&serde_json::from_value(value.clone())?)?;
                                    let expected = state.library.list_folder_entries("test", "/", &full).await?;
                                    let actual = state.library.list_folder_summaries("test", "/", &options(value)).await?;
                                    assert_eq!(actual.total_count, expected.total_count);
                                    assert_eq!(
                                        actual.entries.iter().map(|entry| (&entry.path, &entry.kind)).collect::<Vec<_>>(),
                                        expected.entries.iter().map(|entry| (&entry.path, &entry.kind)).collect::<Vec<_>>()
                                    );
                                    assert!(!serde_json::to_string(&actual)?.contains("metadata"));
                                }
                            }
                        }
                    }
                }
            }
        }
        assert!(SummaryOptions::from_query(&serde_json::from_value(serde_json::json!({"path":"/", "sort":"size"}))?).is_err());
        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn detail_batch_keeps_broken_links_and_reports_partial_failure_test() -> TestResult {
        let (_dir, state) = crate::service::mutation::tests::state().await?;
        let root = &state.conf.libraries[0].path;
        std::fs::write(root.join("known.txt"), b"known")?;
        std::os::unix::fs::symlink(root.join("absent"), root.join("broken"))?;
        let details = state.library.entry_details("test", &["/known.txt".into(), "/broken".into(), "/absent".into()]).await?;
        assert_eq!(details.results.len(), 3);
        assert_eq!(details.results[0].detail.as_ref().unwrap().metadata.size, 5);
        assert_eq!(details.results[1].detail.as_ref().unwrap().kind, models::FileSystemEntryKind::Link);
        assert_eq!(details.results[2].error, Some(models::ErrorKind::NotFound));
        for result in details.results {
            assert_ne!(result.detail.is_some(), result.error.is_some());
        }
        assert!(state.library.entry_details("test", &[]).await.is_err());
        assert!(state.library.entry_details("test", &["/x/../known.txt".into(), "/known.txt".into()]).await.is_err());
        assert!(state.library.entry_details("test", &["/../escape".into()]).await.is_err());
        assert!(state.library.entry_details("test", &vec!["/known.txt".to_string(); 201]).await.is_err());
        assert!(state.library.entry_details("missing-library", &["/known.txt".into()]).await.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn archive_summary_and_detail_preserve_namespace_and_readonly_test() -> TestResult {
        let (_dir, state) = crate::service::mutation::tests::state().await?;
        let archive = state.conf.libraries[0].path.join("test.7z");
        let mut writer = sevenz_rust2::ArchiveWriter::create(&archive)?;
        for (name, bytes) in [("dir/a.txt", b"abc".as_slice()), ("hint.zip", b"not opened".as_slice())] {
            writer.push_archive_entry(sevenz_rust2::ArchiveEntry::new_file(name), Some(std::io::Cursor::new(bytes.to_vec())))?;
        }
        writer.finish()?;
        let value = serde_json::json!({"path":"/test.7z", "sort":"name", "foldersFirst":true});
        let full = state
            .library
            .list_folder_entries("test", "/test.7z", &ListOptions::from_query(&serde_json::from_value(value.clone())?)?)
            .await?;
        let summary = state.library.list_folder_summaries("test", "/test.7z", &options(value)).await?;
        assert_eq!(summary.total_count, full.total_count);
        assert_eq!(
            summary.entries.iter().map(|entry| &entry.path).collect::<Vec<_>>(),
            full.entries.iter().map(|entry| &entry.path).collect::<Vec<_>>()
        );
        assert!(summary.entries.iter().any(|entry| entry.path == "/test.7z/hint.zip" && entry.archive));
        let result = state.library.entry_details("test", &["/test.7z/dir/a.txt".into()]).await?;
        let detail = result.results[0].detail.as_ref().unwrap();
        assert_eq!(detail.path, "/test.7z/dir/a.txt");
        assert_eq!(detail.metadata.size, 3);
        assert!(detail.metadata.readonly);
        Ok(())
    }

    #[tokio::test]
    async fn invalidated_summary_flight_cannot_restore_old_cache_test() -> TestResult {
        let clock = Arc::new(FakeClockUtc::new(Utc::now()));
        let index = SummaryIndex::new(Default::default(), clock.clone());
        let key = SummaryKey::new("test", Path::new("/library"), "/", &options(serde_json::json!({"path":"/"})))?;
        let version = Some(SnapshotVersion::Local(Some(clock.now() - chrono::Duration::seconds(10))));
        let (send, receive) = futures::channel::oneshot::channel();
        let mut old = index
            .get_or_scan(key.clone(), version.clone(), move || async move {
                receive.await.unwrap();
                Ok(data())
            })
            .boxed();
        assert!(poll!(old.as_mut()).is_pending());
        index.invalidate(&ListingInvalidation::for_entry("test", "/changed")?).await;
        let current = index.get_or_scan(key.clone(), version.clone(), || async { Ok(data()) }).await?;
        send.send(()).unwrap();
        assert_ne!(old.await?.id, current.id);
        let cached = index.get_or_scan(key, version, || async { panic!("fresh summary must be reused") }).await?;
        assert_eq!(cached.id, current.id);
        Ok(())
    }

    struct DropProbe(Arc<AtomicBool>);
    impl Drop for DropProbe {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn cancelling_all_summary_waiters_drops_the_scan_test() {
        let index = SummaryIndex::new(Default::default(), Arc::new(FakeClockUtc::new(Utc::now())));
        let key = SummaryKey::new("test", Path::new("/library"), "/", &options(serde_json::json!({"path":"/"}))).unwrap();
        let dropped = Arc::new(AtomicBool::new(false));
        let flag = dropped.clone();
        let calls = Arc::new(AtomicUsize::new(0));
        let count = calls.clone();
        let mut first = index
            .get_or_scan(key.clone(), None, move || async move {
                count.fetch_add(1, Ordering::SeqCst);
                let _guard = DropProbe(flag);
                futures::future::pending::<()>().await;
                Ok(data())
            })
            .boxed();
        assert!(poll!(first.as_mut()).is_pending());
        let mut second = index.get_or_scan(key, None, || async { panic!("must share existing flight") }).boxed();
        assert!(poll!(second.as_mut()).is_pending());
        drop(first);
        assert!(!dropped.load(Ordering::SeqCst));
        drop(second);
        assert!(dropped.load(Ordering::SeqCst));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
