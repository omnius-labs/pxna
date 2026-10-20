use std::{cmp::Ordering, path::Path, time::Instant};

use futures::StreamExt as _;
use unicode_normalization::UnicodeNormalization as _;

use std::collections::HashSet;

use omnius_pxna_engine::{
    model::{FsEntryDetail, NaturalOrder, Rating},
    service::{MediaKind, RatingTagService, ResolvedStoragePath},
};
use omnius_pxna_interface::{models, types::Nullable};

use crate::{prelude::*, service::library::EntryConverter};

mod snapshot;
mod summary;

pub use snapshot::*;
pub use summary::*;

const DEFAULT_LIST_LIMIT: u32 = 200;
const MAX_LIST_LIMIT: u32 = 1000;

/// 一覧の走査、絞り込み、並べ替え、ページの指定。
#[derive(Debug, Clone)]
pub struct ListOptions {
    recursive: bool,
    include_hidden: bool,
    folders_only: bool,
    sort: ListSort,
    filter: NameFilter,
    marks: MarkFilter,
    selection: MarkSelection,
    offset: u64,
    limit: usize,
}

impl ListOptions {
    pub fn from_query(query: &models::ListFolderEntriesQueryParams) -> Result<Self> {
        Ok(Self {
            recursive: query.recursive.unwrap_or(false),
            include_hidden: query.include_hidden.unwrap_or(false),
            folders_only: query.folders_only.unwrap_or(false),
            sort: ListSort::parse(query.sort.as_deref(), query.order.as_deref(), query.folders_first.unwrap_or(false))?,
            filter: NameFilter::new(query.query.as_deref().unwrap_or_default()),
            marks: MarkFilter::parse(query.rating.as_deref(), query.tag.as_deref())?,
            selection: MarkSelection::default(),
            offset: query.offset.unwrap_or(0),
            limit: Self::parse_limit(query.limit)?,
        })
    }

    pub fn recursive(&self) -> bool {
        self.recursive
    }

    /// 評価とタグでの絞り込みの指定。
    pub fn marks(&self) -> &MarkFilter {
        &self.marks
    }

    /// 絞り込みの指定をユーザの評価とタグで引いた結果を持たせる。指定がなければ何も変わらない。
    pub fn with_selection(mut self, selection: MarkSelection) -> Self {
        self.selection = selection;
        self
    }
    pub fn include_hidden(&self) -> bool {
        self.include_hidden
    }

    pub fn sort(&self) -> ListSort {
        self.sort
    }

    fn parse_limit(limit: Option<u32>) -> Result<usize> {
        let limit = limit.unwrap_or(DEFAULT_LIST_LIMIT);
        if limit == 0 || limit > MAX_LIST_LIMIT {
            return Err(Error::new(ErrorKind::InvalidInput).with_message(format!("limit must be 1-{MAX_LIST_LIMIT}: {limit}")));
        }
        Ok(limit as usize)
    }

    #[cfg(test)]
    pub fn paginate(&self, entries: &ListingEntries, snapshot: String) -> models::FolderListing {
        self.page(entries, &self.sort.indices(entries), snapshot)
    }

    pub fn page(&self, entries: &ListingEntries, indices: &[usize], snapshot: String) -> models::FolderListing {
        let mut total = 0_u64;
        let mut page = Vec::new();
        for &index in indices {
            let listed = &entries.entries[index];
            if (self.folders_only && !EntryPosition::is_folder_like(&listed.entry)) || !self.filter.matches(&listed.name) || !self.selection.matches(&listed.entry) {
                continue;
            }
            if total >= self.offset && page.len() < self.limit {
                page.push(listed.entry.clone());
            }
            total += 1;
        }
        models::FolderListing::new(page, total, self.selection.snapshot_id(snapshot), entries.truncated)
    }
}

/// 評価とタグでの絞り込みの指定。指定がなければ何も絞らない。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MarkFilter {
    rating: Option<RatingFilter>,
    tag: Option<String>,
}

/// 評価での絞り込み。未評価は、評価していない画像と動画を選ぶ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RatingFilter {
    Rated(Rating),
    Unrated,
}

impl MarkFilter {
    pub fn parse(rating: Option<&str>, tag: Option<&str>) -> Result<Self> {
        let rating = match rating {
            None => None,
            Some("good") => Some(RatingFilter::Rated(Rating::Good)),
            Some("bad") => Some(RatingFilter::Rated(Rating::Bad)),
            Some("none") => Some(RatingFilter::Unrated),
            Some(other) => return Err(Error::new(ErrorKind::InvalidInput).with_message(format!("rating must be good, bad or none: {other}"))),
        };
        let tag = tag
            .map(|tag| omnius_pxna_engine::model::TagName::parse(tag).map(|tag| tag.as_str().to_string()))
            .transpose()?;
        Ok(Self { rating, tag })
    }

    pub fn is_active(&self) -> bool {
        self.rating.is_some() || self.tag.is_some()
    }

    /// ユーザの評価とタグで引いて、一覧の Entry の照合に使う集合を作る。
    pub async fn select(&self, service: &RatingTagService, user_id: &str, library_id: &str) -> Result<MarkSelection> {
        if !self.is_active() {
            return Ok(MarkSelection::default());
        }
        // 版と集合は、書き込みにはさまれずに同じ状態から読む。読んでいる間、評価とタグの書き込みは待たされる。
        let marks = service.read().await;
        let version = marks.version();
        let rating = match self.rating {
            None => None,
            Some(RatingFilter::Rated(rating)) => Some(RatingMatch::Among(marks.rated_paths(user_id, library_id, rating).await?)),
            Some(RatingFilter::Unrated) => Some(RatingMatch::Outside(marks.all_rated_paths(user_id, library_id).await?)),
        };
        let tagged = match &self.tag {
            None => None,
            Some(tag) => Some(marks.tagged_paths(user_id, library_id, tag).await?),
        };
        Ok(MarkSelection {
            active: Some(version),
            rating,
            tagged,
        })
    }
}

#[derive(Debug, Clone)]
enum RatingMatch {
    /// この集合に含まれる Entry を選ぶ。
    Among(HashSet<String>),
    /// この集合に含まれない Entry を選ぶ。
    Outside(HashSet<String>),
}

/// 評価とタグでの絞り込みを、ユーザの評価とタグで引いた結果。
///
/// 絞り込みは画像と動画だけを選ぶ。フォルダと、画像でも動画でもないファイルは、評価もタグも付けられないので、含めない。
#[derive(Debug, Clone, Default)]
pub struct MarkSelection {
    /// 絞り込みが有効なときの、評価とタグの版。
    active: Option<u64>,
    rating: Option<RatingMatch>,
    tagged: Option<HashSet<String>>,
}

impl MarkSelection {
    fn matches(&self, entry: &models::FileSystemEntry) -> bool {
        if self.active.is_none() {
            return true;
        }
        if MediaKind::of_name(&entry.name).is_none() || entry.kind != models::FileSystemEntryKind::File {
            return false;
        }
        let rating = match &self.rating {
            None => true,
            Some(RatingMatch::Among(paths)) => paths.contains(&entry.path),
            Some(RatingMatch::Outside(paths)) => !paths.contains(&entry.path),
        };
        rating && self.tagged.as_ref().is_none_or(|paths| paths.contains(&entry.path))
    }

    /// 絞り込み中は、評価とタグの版を一覧の識別子に足す。書き込みをまたいで、ページが食い違わないようにする。
    fn snapshot_id(&self, snapshot: String) -> String {
        match self.active {
            Some(version) => format!("{snapshot}.m{version}"),
            None => snapshot,
        }
    }
}

/// 正規化済みの名前と、空白で区切ったすべての語を照合する。
#[derive(Debug, Clone)]
struct NameFilter {
    terms: Vec<String>,
}

impl NameFilter {
    fn new(query: &str) -> Self {
        Self {
            terms: Self::normalize(query).split_whitespace().map(str::to_string).collect(),
        }
    }
    fn normalize(name: &str) -> String {
        name.nfkc().collect::<String>().to_lowercase()
    }
    fn matches(&self, name: &str) -> bool {
        self.terms.iter().all(|term| name.contains(term))
    }
}

struct ListedEntry {
    entry: models::FileSystemEntry,
    name: String,
}

impl ListedEntry {
    fn new(entry: models::FileSystemEntry) -> Self {
        Self {
            name: NameFilter::normalize(&entry.name),
            entry,
        }
    }
}

/// 1 回の走査の結果。照合用の名前は Entry ごとに 1 度だけ正規化する。
pub struct ListingEntries {
    entries: Vec<ListedEntry>,
    truncated: bool,
}

impl ListingEntries {
    #[tracing::instrument(level = "debug", skip_all, fields(recursive, include_hidden, max_entries))]
    pub async fn scan(folder: &ResolvedStoragePath, library_root: &Path, recursive: bool, include_hidden: bool, max_entries: usize) -> Result<Self> {
        let started = Instant::now();
        let mut conversion_sec = 0.0;
        let converter = EntryConverter::new(library_root);
        let mut entries = Vec::new();
        let mut folders = vec![folder.clone()];
        while let Some(folder) = folders.pop() {
            let mut stream = folder.find_entries(false).await?;
            while let Some(nested) = stream.next().await {
                let nested = nested?;
                let conversion_started = Instant::now();
                let entry = converter.convert(&nested)?;
                if !include_hidden && entry.name.starts_with('.') {
                    conversion_sec += conversion_started.elapsed().as_secs_f64();
                    continue;
                }
                if recursive && matches!(nested.detail, FsEntryDetail::Dir { .. }) {
                    folders.push(folder.at(nested.path.iter().last().unwrap_or_default())?);
                }
                entries.push(ListedEntry::new(entry));
                conversion_sec += conversion_started.elapsed().as_secs_f64();
                if recursive && entries.len() >= max_entries {
                    tracing::debug!(target: "pxna::performance", operation = "listing_scan", entry_count = entries.len(),
                        conversion_sec, elapsed_sec = started.elapsed().as_secs_f64(), truncated = true, "listing scan completed");
                    return Ok(Self { entries, truncated: true });
                }
            }
        }
        tracing::debug!(target: "pxna::performance", operation = "listing_scan", entry_count = entries.len(),
            conversion_sec, elapsed_sec = started.elapsed().as_secs_f64(), truncated = false, "listing scan completed");
        Ok(Self { entries, truncated: false })
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[cfg(test)]
    pub fn from_entries(entries: Vec<models::FileSystemEntry>) -> Self {
        Self {
            entries: entries.into_iter().map(ListedEntry::new).collect(),
            truncated: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EntryPosition {
    folder_like: bool,
    key: SortValue,
    path: String,
}

impl EntryPosition {
    fn is_folder_like(entry: &models::FileSystemEntry) -> bool {
        entry.kind == models::FileSystemEntryKind::Folder || matches!(&entry.file, Some(Nullable::Present(file)) if file.archive)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SortValue {
    Name(String),
    Kind(u8),
    Size(u64),
    ModifiedAt(Option<i64>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ListSortKey {
    Name,
    Kind,
    Size,
    ModifiedAt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ListSort {
    key: ListSortKey,
    descending: bool,
    folders_first: bool,
}

impl ListSort {
    fn parse(sort: Option<&str>, order: Option<&str>, folders_first: bool) -> Result<Self> {
        let key = match sort.unwrap_or("name") {
            "name" => ListSortKey::Name,
            "kind" => ListSortKey::Kind,
            "size" => ListSortKey::Size,
            "modified_at" => ListSortKey::ModifiedAt,
            other => return Err(Error::new(ErrorKind::InvalidInput).with_message(format!("invalid sort: {other}"))),
        };
        let descending = match order.unwrap_or("asc") {
            "asc" => false,
            "desc" => true,
            other => return Err(Error::new(ErrorKind::InvalidInput).with_message(format!("invalid order: {other}"))),
        };
        Ok(Self { key, descending, folders_first })
    }

    fn position(&self, entry: &models::FileSystemEntry) -> EntryPosition {
        let key = match self.key {
            ListSortKey::Name => SortValue::Name(entry.name.clone()),
            ListSortKey::Kind => SortValue::Kind(Self::kind_rank(&entry.kind)),
            ListSortKey::Size => SortValue::Size(entry.metadata.size),
            ListSortKey::ModifiedAt => SortValue::ModifiedAt(Self::modified(entry).and_then(|modified| modified.timestamp_nanos_opt())),
        };
        EntryPosition {
            folder_like: EntryPosition::is_folder_like(entry),
            key,
            path: entry.path.clone(),
        }
    }

    /// フォルダ扱いの Entry を先に置くときは、`order` にかかわらずその組を先にする。
    /// 同じ値の Entry は path で並べ、並びを全順序にする。
    fn compare(&self, a: &EntryPosition, b: &EntryPosition) -> Ordering {
        if self.folders_first {
            let group = b.folder_like.cmp(&a.folder_like);
            if group != Ordering::Equal {
                return group;
            }
        }
        let primary = match (&a.key, &b.key) {
            (SortValue::Name(a), SortValue::Name(b)) => NaturalOrder::compare(a, b),
            (SortValue::Kind(a), SortValue::Kind(b)) => a.cmp(b),
            (SortValue::Size(a), SortValue::Size(b)) => a.cmp(b),
            (SortValue::ModifiedAt(a), SortValue::ModifiedAt(b)) => a.cmp(b),
            // 同じ並べ方のキーだけを比べる。
            _ => Ordering::Equal,
        };
        let ordering = primary.then_with(|| a.path.cmp(&b.path));
        if self.descending { ordering.reverse() } else { ordering }
    }

    fn kind_rank(kind: &models::FileSystemEntryKind) -> u8 {
        match kind {
            models::FileSystemEntryKind::Folder => 0,
            models::FileSystemEntryKind::File => 1,
            models::FileSystemEntryKind::Link => 2,
        }
    }

    fn modified(entry: &models::FileSystemEntry) -> Option<chrono::DateTime<chrono::Utc>> {
        match &entry.metadata.modified_at {
            Some(Nullable::Present(value)) => Some(*value),
            _ => None,
        }
    }
}

impl ListSort {
    pub fn indices(&self, entries: &ListingEntries) -> Vec<usize> {
        let started = Instant::now();
        let positions: Vec<_> = entries.entries.iter().map(|entry| self.position(&entry.entry)).collect();
        let mut indices: Vec<_> = (0..positions.len()).collect();
        indices.sort_by(|&a, &b| self.compare(&positions[a], &positions[b]));
        tracing::debug!(target: "pxna::performance", operation = "listing_sort", entry_count = entries.len(),
            elapsed_sec = started.elapsed().as_secs_f64(), "listing sort completed");
        indices
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use omnius_core_base::{clock::ClockUtc, tsid::TsidProviderImpl};
    use omnius_pxna_engine::service::{ArchiveOptions, ArchiveRuntime, LocalStorageDriver, StorageService};
    use rand::{SeedableRng as _, rngs::ChaCha20Rng};
    use std::sync::Arc;

    pub fn entry(name: &str) -> models::FileSystemEntry {
        models::FileSystemEntry::new(
            format!("/{name}"),
            name.to_string(),
            models::FileSystemMetadata::new(0, false),
            models::FileSystemEntryKind::File,
        )
    }
    fn folder(name: &str) -> models::FileSystemEntry {
        let mut e = entry(name);
        e.kind = models::FileSystemEntryKind::Folder;
        e
    }
    fn file(name: &str, archive: bool) -> models::FileSystemEntry {
        let mut e = entry(name);
        e.file = Some(Nullable::Present(models::FileSystemEntryFileDetail::new(archive)));
        e
    }
    pub fn query(offset: Option<u64>, limit: Option<u32>) -> models::ListFolderEntriesQueryParams {
        models::ListFolderEntriesQueryParams {
            path: "/".to_string(),
            recursive: None,
            offset,
            query: None,
            limit,
            sort: None,
            order: None,
            include_hidden: None,
            folders_first: None,
            folders_only: None,
            rating: None,
            tag: None,
        }
    }
    fn list(query: &models::ListFolderEntriesQueryParams, entries: Vec<models::FileSystemEntry>) -> Result<models::FolderListing> {
        Ok(ListOptions::from_query(query)?.paginate(&ListingEntries::from_entries(entries), "test".to_string()))
    }
    fn names(listing: &models::FolderListing) -> Vec<&str> {
        listing.entries.iter().map(|e| e.name.as_str()).collect()
    }

    #[test]
    fn offset_pages_and_out_of_range_test() -> testresult::TestResult {
        let entries = || vec![entry("c"), entry("a"), entry("b")];
        let first = list(&query(None, Some(2)), entries())?;
        assert_eq!(names(&first), ["a", "b"]);
        assert_eq!(first.total_count, 3);
        assert_eq!(names(&list(&query(Some(2), Some(2)), entries())?), ["c"]);
        assert!(list(&query(Some(u64::MAX), None), entries())?.entries.is_empty());
        Ok(())
    }
    #[test]
    fn sorts_names_in_natural_order_test() -> testresult::TestResult {
        let entries = || vec![entry("10.jpg"), entry("2.jpg"), entry("1.jpg"), entry("B.jpg"), entry("a.jpg")];
        assert_eq!(names(&list(&query(None, None), entries())?), ["1.jpg", "2.jpg", "10.jpg", "a.jpg", "B.jpg"]);
        let mut q = query(None, None);
        q.order = Some("desc".to_string());
        assert_eq!(names(&list(&q, entries())?), ["B.jpg", "a.jpg", "10.jpg", "2.jpg", "1.jpg"]);
        Ok(())
    }
    #[test]
    fn folders_first_and_only_test() -> testresult::TestResult {
        let entries = || vec![file("b.txt", false), folder("z"), file("a.zip", true), file("a.txt", false), folder("c")];
        let mut q = query(None, None);
        q.folders_first = Some(true);
        assert_eq!(names(&list(&q, entries())?), ["a.zip", "c", "z", "a.txt", "b.txt"]);
        q.order = Some("desc".to_string());
        assert_eq!(names(&list(&q, entries())?), ["z", "c", "a.zip", "b.txt", "a.txt"]);
        q.folders_only = Some(true);
        assert_eq!(list(&q, entries())?.total_count, 3);
        Ok(())
    }
    #[test]
    fn query_matches_normalized_names_test() -> testresult::TestResult {
        let entries = || vec![entry("か\u{3099}abc.JPG"), entry("other.jpg"), entry("がABC.txt")];
        let mut q = query(None, Some(1));
        q.query = Some(" がＡＢＣ  jpg ".to_string());
        let matched = list(&q, entries())?;
        assert_eq!(names(&matched), ["か\u{3099}abc.JPG"]);
        assert_eq!(matched.total_count, 1);
        q.query = Some("　 ".to_string());
        assert_eq!(list(&q, entries())?.total_count, 3);
        Ok(())
    }
    #[test]
    fn rejects_out_of_range_limit_test() {
        for limit in [0, MAX_LIST_LIMIT + 1] {
            assert_eq!(*ListOptions::from_query(&query(None, Some(limit))).unwrap_err().kind(), ErrorKind::InvalidInput);
        }
    }
    async fn scan_test(root: &Path, include_hidden: bool, limit: usize) -> Result<models::FolderListing> {
        let rng = ChaCha20Rng::seed_from_u64(1);
        let provider = Arc::new(Mutex::new(TsidProviderImpl::new(ClockUtc, rng, 16)));
        let runtime = Arc::new(ArchiveRuntime::open(ArchiveOptions::default(), &root.join("cache")).await?);
        let storage = Arc::new(StorageService::new(Arc::new(LocalStorageDriver::new(provider)), runtime));
        let library_root = root.join("library");
        let nested = omnius_pxna_engine::model::NestedPathBuf::try_from(library_root.as_path()).map_err(|error| Error::from_error(error, ErrorKind::InvalidInput))?;
        let folder = storage.resolve_entry(&nested).await?;
        let scanned = ListingEntries::scan(&folder, &library_root, true, include_hidden, limit).await?;
        let mut q = query(None, None);
        q.recursive = Some(true);
        q.include_hidden = Some(include_hidden);
        Ok(ListOptions::from_query(&q)?.paginate(&scanned, "scan".to_string()))
    }
    #[tokio::test]
    async fn recursive_listing_is_truncated_test() -> testresult::TestResult {
        let dir = tempfile::tempdir()?;
        std::fs::create_dir(dir.path().join("library"))?;
        for n in 0..5 {
            std::fs::write(dir.path().join(format!("library/{n}.txt")), b"x")?;
        }
        let listing = scan_test(dir.path(), false, 3).await?;
        assert_eq!(listing.total_count, 3);
        assert!(listing.truncated);
        Ok(())
    }
    #[tokio::test]
    async fn recursive_listing_skips_hidden_folders_test() -> testresult::TestResult {
        let dir = tempfile::tempdir()?;
        std::fs::create_dir_all(dir.path().join("library/.hidden/deep"))?;
        std::fs::create_dir_all(dir.path().join("library/visible"))?;
        std::fs::write(dir.path().join("library/.hidden/deep/secret.txt"), b"secret")?;
        std::fs::write(dir.path().join("library/visible/show.txt"), b"show")?;
        let visible = scan_test(dir.path(), false, 100).await?;
        assert_eq!(visible.entries.iter().map(|e| e.path.as_str()).collect::<Vec<_>>(), ["/visible/show.txt", "/visible"]);
        let all = scan_test(dir.path(), true, 100).await?;
        assert_eq!(all.total_count, 5);
        Ok(())
    }
}
