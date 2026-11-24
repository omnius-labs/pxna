use omnius_pxna_engine::service::WorkClass;
use std::{cmp::Ordering, path::Path, time::Instant};

use futures::StreamExt as _;
use tracing::{Instrument as _, Span, field::Empty};
use unicode_normalization::UnicodeNormalization as _;

use std::{collections::HashSet, sync::Arc};

use omnius_pxna_engine::{
    model::{FsEntryDetail, NaturalOrder, Rating},
    perf_span,
    service::{MediaKind, RatingTagService, ResolvedStoragePath, StorageService},
};
use omnius_pxna_interface::{models, types::Nullable};

use crate::{prelude::*, service::library::EntryConverter};

mod kind;
mod merge;
mod name_index;
mod scope;
mod scope_results;
mod search;
mod snapshot;
mod summary;
mod timeline_order;

pub use kind::*;
pub use merge::*;
pub use name_index::*;
pub use scope::*;
pub use scope_results::*;
pub use search::*;
pub use snapshot::*;
pub use summary::*;
pub use timeline_order::*;

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
    /// The kinds the Filter names. Empty for the folder listing and the search API, and when a `listEntries` Filter names none.
    kinds: EntryKinds,
    /// Whether the Scope reads the insides of archives, which a listing of direct children does not use.
    inside_archives: bool,
    /// Whether this is a `listEntries` listing, which applies the kinds of a Filter. The folder listing and the search API return every kind.
    scoped: bool,
    /// Whether the listing is read in timeline-date order, which `sort` does not name: it keeps the name order the snapshots of a start with a condition are built and matched with.
    timeline: bool,
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
            marks: MarkFilter::default(),
            selection: MarkSelection::default(),
            kinds: EntryKinds::default(),
            inside_archives: false,
            scoped: false,
            timeline: false,
            offset: query.offset.unwrap_or(0),
            limit: Self::parse_limit(query.limit)?,
        })
    }

    /// 検索の条件。隠し Entry は含めず、フォルダ扱いの Entry だけに絞る指定は受け付けない。
    pub fn from_search(query: &models::ListSearchEntriesQueryParams) -> Result<Self> {
        let tags = query.tag.iter().map(|tag| tag.0.as_str()).collect::<Vec<_>>();
        Ok(Self {
            recursive: query.recursive.unwrap_or(false),
            include_hidden: false,
            folders_only: false,
            sort: ListSort::parse(query.sort.as_deref(), query.order.as_deref(), query.folders_first.unwrap_or(false))?,
            filter: NameFilter::new(query.query.as_deref().unwrap_or_default()),
            marks: MarkFilter::parse(query.rating.as_deref(), &tags)?,
            selection: MarkSelection::default(),
            kinds: EntryKinds::default(),
            inside_archives: false,
            scoped: false,
            timeline: false,
            offset: query.offset.unwrap_or(0),
            limit: Self::parse_limit(query.limit)?,
        })
    }

    /// The conditions of `listEntries`. Hidden Entries are never included, as in the search API, and the Filter's kinds and the Scope's reading of archives apply.
    ///
    /// `sort` of `timeline_date` reads the timeline in descending order only, so an ascending `order`, `foldersFirst`, and `offset` with it are `InvalidInput`.
    pub fn from_entries(request: &models::EntryListRequest) -> Result<Self> {
        let timeline = request.sort.as_deref() == Some("timeline_date");
        let sort = if timeline {
            if request.order.as_deref().is_some_and(|order| order != "desc") {
                return Err(Error::new(ErrorKind::InvalidInput).with_message("timeline_date is read in descending order only"));
            }
            if request.folders_first.is_some() {
                return Err(Error::new(ErrorKind::InvalidInput).with_message("foldersFirst cannot be given with sort=timeline_date"));
            }
            if request.offset.is_some() {
                return Err(Error::new(ErrorKind::InvalidInput).with_message("offset cannot be given with sort=timeline_date"));
            }
            ListSort::parse(None, None, false)?
        } else {
            ListSort::parse(request.sort.as_deref(), request.order.as_deref(), request.folders_first.unwrap_or(false))?
        };
        let mut options = Self::scoped_request(&request.scope, request.filter.as_ref(), sort)?;
        options.offset = request.offset.unwrap_or(0);
        options.limit = Self::parse_limit(request.limit)?;
        options.timeline = timeline;
        Ok(options)
    }

    /// The conditions of the per-month counts of `listEntries`: the Scope's choices and the Filter, with no sort or page.
    pub fn from_entry_months(request: &models::EntryMonthsRequest) -> Result<Self> {
        let mut options = Self::scoped_request(&request.scope, request.filter.as_ref(), ListSort::parse(None, None, false)?)?;
        options.timeline = true;
        Ok(options)
    }

    fn scoped_request(scope: &models::EntryScope, filter: Option<&models::EntryFilter>, sort: ListSort) -> Result<Self> {
        let (query, rating, tags, kinds) = match filter {
            Some(filter) => (
                filter.query.as_deref(),
                filter.rating.as_deref(),
                filter.tags.as_deref().unwrap_or_default(),
                filter.kinds.as_deref().unwrap_or_default(),
            ),
            None => (None, None, Default::default(), Default::default()),
        };
        let tags = tags.iter().map(|tag| tag.0.as_str()).collect::<Vec<_>>();
        let kinds = kinds.iter().map(|kind| EntryKind::parse(kind)).collect::<Result<Vec<_>>>()?;
        Ok(Self {
            recursive: scope.recursive.unwrap_or(false),
            include_hidden: false,
            folders_only: false,
            sort,
            filter: NameFilter::new(query.unwrap_or_default()),
            marks: MarkFilter::parse(rating, &tags)?,
            selection: MarkSelection::default(),
            kinds: EntryKinds::default(),
            inside_archives: false,
            scoped: false,
            timeline: false,
            offset: 0,
            limit: DEFAULT_LIST_LIMIT as usize,
        }
        .with_scope(EntryKinds::of(kinds), scope.inside_archives.unwrap_or(false)))
    }

    /// Whether the listing is read in timeline-date order.
    pub fn is_timeline(&self) -> bool {
        self.timeline
    }

    /// The most Entries of a page.
    pub fn limit(&self) -> usize {
        self.limit
    }

    pub fn recursive(&self) -> bool {
        self.recursive
    }

    /// Turns the listing into a `listEntries` one: the Filter's kinds and whether the Scope reads the insides of archives apply from here on.
    pub fn with_scope(mut self, kinds: EntryKinds, inside_archives: bool) -> Self {
        self.kinds = kinds;
        self.inside_archives = inside_archives;
        self.scoped = true;
        self
    }

    /// Whether the Scope reads the insides of archives. Without `recursive` it changes nothing, so it is false then.
    pub fn inside_archives(&self) -> bool {
        self.inside_archives && self.recursive
    }

    /// The kinds this listing keeps, when its Entries `read_archives` like folders or not.
    ///
    /// A Filter that names kinds keeps exactly those. Without any, a recursive `listEntries` listing keeps every kind except folders, and also except archives that it read like
    /// folders, and a listing of direct children keeps every kind. The folder listing and the search API keep every kind.
    pub fn kept_kinds(&self, read_archives: bool) -> EntryKinds {
        if !self.scoped {
            return EntryKinds::all();
        }
        if !self.kinds.is_empty() {
            return self.kinds;
        }
        if !self.recursive {
            return EntryKinds::all();
        }
        let kept = EntryKinds::all().without(EntryKind::Folder);
        if read_archives { kept.without(EntryKind::Archive) } else { kept }
    }

    /// The conditions the name index answers this listing from, if it is a listing the index can answer: a recursive one without hidden Entries,
    /// with a name, rating, or tag condition, or whose Scope reads the insides of archives. Whether the start is local and the index is usable is for the caller to check.
    pub fn index_conditions(&self) -> Option<IndexConditions> {
        if !self.recursive || self.include_hidden || (self.filter.terms.is_empty() && !self.marks.is_active() && !self.inside_archives()) {
            return None;
        }
        Some(IndexConditions {
            terms: self.filter.terms.clone(),
            marks: format!("{:?}", self.marks),
            selection: self.selection.clone(),
            // An answer from the index with the insides reads archives like folders, and one without does not.
            kinds: self.kept_kinds(self.inside_archives()),
            inside_archives: self.inside_archives(),
        })
    }

    /// What decides which Entries of a snapshot match this listing, for the order in timeline-date order that the snapshot keeps: the name terms, the rating and tag
    /// condition with the version of ratings and tags its selection was read at, the kinds, and the insides choice, as the key of an answer from the name index holds them.
    /// `read_archives` is whether the snapshot's Entries include the insides of archives.
    pub fn timeline_key(&self, read_archives: bool) -> IndexKey {
        IndexKey {
            terms: self.filter.terms.clone(),
            marks: format!("{:?}", self.marks),
            version: self.selection.active,
            kinds: self.kept_kinds(read_archives),
            inside_archives: self.inside_archives(),
        }
    }

    /// The kinds the Filter names, which are none when it names none.
    pub fn kinds(&self) -> EntryKinds {
        self.kinds
    }

    /// Whether the Filter has a name, rating, or tag condition.
    pub fn has_conditions(&self) -> bool {
        !self.filter.terms.is_empty() || self.marks.is_active()
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
        let kept = self.kept_kinds(entries.read_archives);
        let mut total = 0_u64;
        let mut page = Vec::new();
        for &index in indices {
            let listed = &entries.entries[index];
            if !self.matches(listed, kept) {
                continue;
            }
            if total >= self.offset && page.len() < self.limit {
                page.push(listed.entry.clone());
            }
            total += 1;
        }
        models::FolderListing::new(page, total, self.offset, self.selection.snapshot_id(snapshot), entries.truncated)
    }

    /// Finds the position of the Entry at `path` and cuts the page that holds it at a multiple of `limit`.
    /// The position counts after the conditions are applied, and an Entry outside them gives `None`.
    pub fn locate(&self, entries: &ListingEntries, indices: &[usize], path: &str, snapshot: String) -> Option<models::FolderListing> {
        let matching = self.matching(entries, indices);
        let position = matching.iter().position(|&index| entries.entries[index].entry.path == path)? as u64;
        let limit = self.limit as u64;
        let offset = position / limit * limit;
        let page = matching
            .iter()
            .skip(offset as usize)
            .take(self.limit)
            .map(|&index| entries.entries[index].entry.clone())
            .collect();
        Some(models::FolderListing::new(
            page,
            matching.len() as u64,
            offset,
            self.selection.snapshot_id(snapshot),
            entries.truncated,
        ))
    }

    /// 並べ方の順の位置のうち、絞り込みの条件に合うものだけを、順を保って返す。
    pub fn matching(&self, entries: &ListingEntries, indices: &[usize]) -> Vec<usize> {
        let kept = self.kept_kinds(entries.read_archives);
        indices.iter().copied().filter(|&index| self.matches(&entries.entries[index], kept)).collect()
    }

    fn matches(&self, listed: &ListedEntry, kept: EntryKinds) -> bool {
        (!self.folders_only || EntryPosition::is_folder_like(&listed.entry))
            && kept.contains(listed.kind)
            && self.filter.matches(&listed.name)
            && self.selection.matches(&listed.entry)
    }
}

/// Where the name index starts looking for the Entries that match a listing's conditions.
pub enum IndexCandidates<'a> {
    /// There is no rating or tag condition: every Entry under the start whose name matches.
    Names,
    /// These paths only, which are the good, bad, or tagged ones. They may be anywhere in the Library.
    Paths(Vec<String>),
    /// The images and videos under the start that are not in `rated` and, if given, are in `tagged`.
    Unrated { rated: &'a HashSet<String>, tagged: Option<&'a HashSet<String>> },
}

/// What identifies the Entries the name index finds for a listing: the name terms and the rating and tag condition with the version of ratings and tags it was read at.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IndexKey {
    terms: Vec<String>,
    marks: String,
    version: Option<u64>,
    kinds: EntryKinds,
    inside_archives: bool,
}

/// The conditions of a listing that the name index answers.
#[derive(Debug, Clone)]
pub struct IndexConditions {
    terms: Vec<String>,
    marks: String,
    selection: MarkSelection,
    kinds: EntryKinds,
    inside_archives: bool,
}

impl IndexConditions {
    pub fn key(&self) -> IndexKey {
        IndexKey {
            terms: self.terms.clone(),
            marks: self.marks.clone(),
            version: self.selection.active,
            kinds: self.kinds,
            inside_archives: self.inside_archives,
        }
    }

    /// The kinds of Entries to return. A rating or tag condition keeps images and videos among them, which the caller narrows them to.
    pub fn kinds(&self) -> EntryKinds {
        self.kinds
    }

    /// Whether the Entries inside archives are returned, nested archives included.
    pub fn inside_archives(&self) -> bool {
        self.inside_archives
    }

    /// The normalized name terms.
    pub fn terms(&self) -> &[String] {
        &self.terms
    }

    pub fn candidates(&self) -> IndexCandidates<'_> {
        let selection = &self.selection;
        if selection.active.is_none() {
            return IndexCandidates::Names;
        }
        match (&selection.rating, &selection.tagged) {
            (Some(RatingMatch::Outside(rated)), tagged) => IndexCandidates::Unrated { rated, tagged: tagged.as_ref() },
            (Some(RatingMatch::Among(rated)), Some(tagged)) => IndexCandidates::Paths(rated.intersection(tagged).cloned().collect()),
            (Some(RatingMatch::Among(rated)), None) => IndexCandidates::Paths(rated.iter().cloned().collect()),
            (None, Some(tagged)) => IndexCandidates::Paths(tagged.iter().cloned().collect()),
            (None, None) => IndexCandidates::Names,
        }
    }
}

/// 評価とタグでの絞り込みの指定。指定がなければ何も絞らない。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MarkFilter {
    rating: Option<RatingFilter>,
    /// すべてを持つ Entry を選ぶタグ。重複はない。
    tags: Vec<String>,
}

/// 評価での絞り込み。未評価は、評価していない画像と動画を選ぶ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RatingFilter {
    Rated(Rating),
    Unrated,
}

impl MarkFilter {
    pub fn parse(rating: Option<&str>, tags: &[&str]) -> Result<Self> {
        let rating = match rating {
            None => None,
            Some("good") => Some(RatingFilter::Rated(Rating::Good)),
            Some("bad") => Some(RatingFilter::Rated(Rating::Bad)),
            Some("none") => Some(RatingFilter::Unrated),
            Some(other) => return Err(Error::new(ErrorKind::InvalidInput).with_message(format!("rating must be good, bad or none: {other}"))),
        };
        let mut parsed = Vec::<String>::new();
        for tag in tags {
            let tag = omnius_pxna_engine::model::TagName::parse(tag)?.as_str().to_string();
            if !parsed.contains(&tag) {
                parsed.push(tag);
            }
        }
        Ok(Self { rating, tags: parsed })
    }

    pub fn is_active(&self) -> bool {
        self.rating.is_some() || !self.tags.is_empty()
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
        let mut tagged: Option<HashSet<String>> = None;
        for tag in &self.tags {
            let paths = marks.tagged_paths(user_id, library_id, tag).await?;
            tagged = Some(match tagged {
                None => paths,
                Some(previous) => previous.intersection(&paths).cloned().collect(),
            });
        }
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
    kind: EntryKind,
}

impl ListedEntry {
    fn new(entry: models::FileSystemEntry) -> Self {
        Self {
            name: NameFilter::normalize(&entry.name),
            kind: EntryKind::of(&entry),
            entry,
        }
    }
}

/// 1 回の走査の結果。照合用の名前は Entry ごとに 1 度だけ正規化する。
pub struct ListingEntries {
    entries: Vec<ListedEntry>,
    truncated: bool,
    /// The name index answered it, so `entries` holds only the matches.
    indexed: bool,
    /// The insides of archives are among `entries`, because the index answered with them or the walk descended into archives, so an archive is not listed as one Entry of its own.
    read_archives: bool,
}

/// Whether a recursive walk reads the archives it meets like folders, which only the walk of an archive start does, for a Scope that reads their insides.
pub enum ArchiveWalk {
    /// An archive is one Entry and nothing inside it is listed.
    Skip,
    /// An archive is walked like a folder through its root, while the number of archive files on the path of its Entries is under `max_nested_depth`.
    Descend { storage: Arc<StorageService>, max_nested_depth: usize },
}

impl ArchiveWalk {
    /// The root of the archive `entry`, if the walk reads it. An archive that cannot be opened has no root here, and the walk goes on without it.
    async fn root_of(&self, entry: &omnius_pxna_engine::service::NestedFsEntry) -> Option<ResolvedStoragePath> {
        let Self::Descend { storage, max_nested_depth } = self else {
            return None;
        };
        if !matches!(entry.detail, FsEntryDetail::File { archive: true }) || entry.path.rank() >= *max_nested_depth {
            return None;
        }
        let mut path = entry.path.to_nested_path_buf();
        path.push_archive_root().ok()?;
        match storage.resolve_entry(&path, WorkClass::Foreground).await {
            Ok(root) => Some(root),
            Err(error) => {
                debug!(path = %entry.path, ?error, "listing walk: skipped an archive that cannot be opened");
                None
            }
        }
    }

    fn reads_archives(&self) -> bool {
        matches!(self, Self::Descend { .. })
    }
}

/// A folder the walk still has to read, and whether it is the root of an archive that the walk found, so that one that cannot be read is skipped and not an error.
struct WalkFolder {
    folder: ResolvedStoragePath,
    found_archive: bool,
}

impl ListingEntries {
    pub async fn scan(folder: &ResolvedStoragePath, library_root: &Path, recursive: bool, include_hidden: bool, max_entries: usize, archives: &ArchiveWalk) -> Result<Self> {
        let span = perf_span!(
            "listing_scan",
            recursive,
            include_hidden,
            max_entries,
            entry_count = Empty,
            conversion_sec = Empty,
            truncated = Empty
        );
        Self::scan_traced(folder, library_root, recursive, include_hidden, max_entries, archives)
            .instrument(span)
            .await
    }

    async fn scan_traced(folder: &ResolvedStoragePath, library_root: &Path, recursive: bool, include_hidden: bool, max_entries: usize, archives: &ArchiveWalk) -> Result<Self> {
        let mut conversion_sec = 0.0;
        let converter = EntryConverter::new(library_root);
        let mut entries = Vec::new();
        let mut folders = vec![WalkFolder {
            folder: folder.clone(),
            found_archive: false,
        }];
        let read_archives = recursive && archives.reads_archives();
        while let Some(WalkFolder { folder, found_archive }) = folders.pop() {
            let mut stream = match folder.find_entries(false, WorkClass::Foreground).await {
                Ok(stream) => stream,
                Err(error) if found_archive => {
                    debug!(path = %folder.path(), ?error, "listing walk: skipped an archive that cannot be read");
                    continue;
                }
                Err(error) => return Err(error.into()),
            };
            while let Some(nested) = stream.next().await {
                let nested = match nested {
                    Ok(nested) => nested,
                    Err(error) if found_archive => {
                        debug!(path = %folder.path(), ?error, "listing walk: stopped reading an archive that failed");
                        break;
                    }
                    Err(error) => return Err(error.into()),
                };
                let conversion_started = Instant::now();
                let entry = converter.convert(&nested)?;
                if !include_hidden && entry.name.starts_with('.') {
                    conversion_sec += conversion_started.elapsed().as_secs_f64();
                    continue;
                }
                if recursive && matches!(nested.detail, FsEntryDetail::Dir { .. }) {
                    folders.push(WalkFolder {
                        folder: folder.at(nested.path.iter().last().unwrap_or_default())?,
                        found_archive,
                    });
                } else if read_archives && let Some(root) = archives.root_of(&nested).await {
                    folders.push(WalkFolder {
                        folder: root,
                        found_archive: true,
                    });
                }
                entries.push(ListedEntry::new(entry));
                conversion_sec += conversion_started.elapsed().as_secs_f64();
                if recursive && entries.len() >= max_entries {
                    Span::current()
                        .record("entry_count", entries.len())
                        .record("conversion_sec", conversion_sec)
                        .record("truncated", true);
                    return Ok(Self {
                        entries,
                        truncated: true,
                        indexed: false,
                        read_archives,
                    });
                }
            }
        }
        Span::current()
            .record("entry_count", entries.len())
            .record("conversion_sec", conversion_sec)
            .record("truncated", false);
        Ok(Self {
            entries,
            truncated: false,
            indexed: false,
            read_archives,
        })
    }

    /// The Entries that the name index found, with their attributes read from the file system. `read_archives` is whether they include the insides of archives.
    pub fn from_index(entries: Vec<models::FileSystemEntry>, truncated: bool, read_archives: bool) -> Self {
        Self {
            entries: entries.into_iter().map(ListedEntry::new).collect(),
            truncated,
            indexed: true,
            read_archives,
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn indexed(&self) -> bool {
        self.indexed
    }

    #[cfg(test)]
    pub fn from_entries(entries: Vec<models::FileSystemEntry>) -> Self {
        Self {
            entries: entries.into_iter().map(ListedEntry::new).collect(),
            truncated: false,
            indexed: false,
            read_archives: false,
        }
    }

    #[cfg(test)]
    pub fn reading_archives(self) -> Self {
        Self { read_archives: true, ..self }
    }

    #[cfg(test)]
    pub fn truncated_from_entries(entries: Vec<models::FileSystemEntry>) -> Self {
        Self {
            truncated: true,
            ..Self::from_entries(entries)
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
        let _span = perf_span!("listing_sort", entry_count = entries.len()).entered();
        let positions: Vec<_> = entries.entries.iter().map(|entry| self.position(&entry.entry)).collect();
        let mut indices: Vec<_> = (0..positions.len()).collect();
        indices.sort_by(|&a, &b| self.compare(&positions[a], &positions[b]));
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
    pub fn folder(name: &str) -> models::FileSystemEntry {
        let mut e = entry(name);
        e.kind = models::FileSystemEntryKind::Folder;
        e
    }
    pub fn file(name: &str, archive: bool) -> models::FileSystemEntry {
        let mut e = entry(name);
        e.file = Some(Nullable::Present(models::FileSystemEntryFileDetail::new(archive)));
        e
    }
    pub fn query(offset: Option<u64>, limit: Option<u32>) -> models::ListFolderEntriesQueryParams {
        models::ListFolderEntriesQueryParams {
            path: "/".to_string(),
            recursive: None,
            offset,
            locate: None,
            query: None,
            limit,
            sort: None,
            order: None,
            include_hidden: None,
            folders_first: None,
            folders_only: None,
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
    fn offset_in_response_folder_listing_test() -> testresult::TestResult {
        let entries = || vec![entry("c"), entry("a"), entry("b")];
        assert_eq!(list(&query(None, Some(2)), entries())?.offset, 0);
        assert_eq!(list(&query(Some(2), Some(2)), entries())?.offset, 2);
        assert_eq!(list(&query(Some(50), Some(2)), entries())?.offset, 50);
        Ok(())
    }
    fn sized(name: &str, size: u64, day: i64) -> models::FileSystemEntry {
        let mut e = entry(name);
        e.metadata.size = size;
        e.metadata.modified_at = Some(Nullable::Present(chrono::DateTime::from_timestamp(1_800_000_000 + day * 86_400, 0).unwrap()));
        e
    }
    #[test]
    fn locate_folder_matches_the_page_of_the_same_offset_test() -> testresult::TestResult {
        let entries = || vec![folder("b"), sized("c.txt", 300, 3), sized("a.txt", 100, 1), file("d.zip", true), sized("e.txt", 200, 2)];
        for sort in ["name", "kind", "size", "modified_at"] {
            for order in ["asc", "desc"] {
                for folders_first in [false, true] {
                    for folders_only in [false, true] {
                        for recursive in [false, true] {
                            let mut q = query(Some(0), Some(2));
                            q.sort = Some(sort.to_string());
                            q.order = Some(order.to_string());
                            q.folders_first = Some(folders_first);
                            q.folders_only = Some(folders_only);
                            q.recursive = Some(recursive);
                            let options = ListOptions::from_query(&q)?;
                            let entries = ListingEntries::from_entries(entries());
                            let indices = options.sort.indices(&entries);
                            for listed in entries.entries.iter().filter(|listed| options.matches(listed, options.kept_kinds(false))) {
                                let found = options
                                    .locate(&entries, &indices, &listed.entry.path, "test".to_string())
                                    .expect("a matching Entry has a position");
                                assert_eq!(found.offset % 2, 0);
                                let expected = ListOptions {
                                    offset: found.offset,
                                    ..options.clone()
                                }
                                .page(&entries, &indices, "test".to_string());
                                assert_eq!(found, expected, "{sort} {order} first={folders_first} only={folders_only} recursive={recursive}");
                                assert!(found.entries.iter().any(|entry| entry.path == listed.entry.path));
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
    #[test]
    fn locate_folder_skips_entries_outside_the_sequence_test() -> testresult::TestResult {
        let entries = ListingEntries::from_entries(vec![entry("a.txt"), entry(".hidden.txt"), folder("b"), file("c.zip", true)]);
        let mut q = query(Some(0), Some(2));
        let options = ListOptions::from_query(&q)?;
        let indices = options.sort.indices(&entries);
        assert!(options.locate(&entries, &indices, "/c.zip", "test".to_string()).is_some());
        assert!(options.locate(&entries, &indices, "/missing.txt", "test".to_string()).is_none());

        // A hidden Entry is not in the sequence a scan with includeHidden false builds, so it has no position.
        let visible = ListingEntries::from_entries(vec![entry("a.txt"), folder("b"), file("c.zip", true)]);
        let visible_indices = options.sort.indices(&visible);
        assert!(options.locate(&visible, &visible_indices, "/.hidden.txt", "test".to_string()).is_none());

        q.folders_only = Some(true);
        let options = ListOptions::from_query(&q)?;
        assert!(options.locate(&entries, &indices, "/a.txt", "test".to_string()).is_none());
        assert!(options.locate(&entries, &indices, "/b", "test".to_string()).is_some());

        q.folders_only = None;
        q.query = Some("a".to_string());
        let options = ListOptions::from_query(&q)?;
        assert!(options.locate(&entries, &indices, "/c.zip", "test".to_string()).is_none());
        assert!(options.locate(&entries, &indices, "/a.txt", "test".to_string()).is_some());
        Ok(())
    }
    /// The options of a `listEntries` listing over `query`, with the Filter's kinds and the Scope's choice of reading the insides of archives.
    pub fn scoped(query: &models::ListFolderEntriesQueryParams, kinds: &[EntryKind], inside_archives: bool) -> ListOptions {
        ListOptions::from_query(query).unwrap().with_scope(EntryKinds::of(kinds.iter().copied()), inside_archives)
    }
    fn mixed() -> Vec<models::FileSystemEntry> {
        vec![folder("dir"), file("pack.zip", true), entry("notes.txt"), entry("a.png"), entry("clip.mp4")]
    }
    fn recursive_query() -> models::ListFolderEntriesQueryParams {
        let mut q = query(None, Some(100));
        q.recursive = Some(true);
        q
    }
    fn kept(options: &ListOptions, entries: ListingEntries) -> Vec<String> {
        let mut names = names(&options.paginate(&entries, "test".to_string())).into_iter().map(str::to_string).collect::<Vec<_>>();
        names.sort();
        names
    }

    #[test]
    fn a_recursive_listing_without_kinds_returns_no_folder_test() {
        let options = scoped(&recursive_query(), &[], false);
        assert_eq!(
            kept(&options, ListingEntries::from_entries(mixed())),
            ["a.png", "clip.mp4", "notes.txt", "pack.zip"],
            "an archive that was not read is a file"
        );
    }
    #[test]
    fn a_recursive_listing_that_read_archives_returns_no_archive_test() {
        for inside_archives in [false, true] {
            let options = scoped(&recursive_query(), &[], inside_archives);
            assert_eq!(kept(&options, ListingEntries::from_entries(mixed()).reading_archives()), ["a.png", "clip.mp4", "notes.txt"]);
        }
    }
    #[test]
    fn named_kinds_return_exactly_those_kinds_test() {
        for read_archives in [false, true] {
            let entries = || {
                let entries = ListingEntries::from_entries(mixed());
                if read_archives { entries.reading_archives() } else { entries }
            };
            assert_eq!(kept(&scoped(&recursive_query(), &[EntryKind::Folder], true), entries()), ["dir"]);
            assert_eq!(kept(&scoped(&recursive_query(), &[EntryKind::Archive], true), entries()), ["pack.zip"]);
            assert_eq!(
                kept(&scoped(&recursive_query(), &[EntryKind::Image, EntryKind::Video], false), entries()),
                ["a.png", "clip.mp4"]
            );
            assert_eq!(kept(&scoped(&recursive_query(), &[EntryKind::Other], false), entries()), ["notes.txt"]);
            assert_eq!(
                kept(&scoped(&recursive_query(), &[EntryKind::Other, EntryKind::Folder], false), entries()),
                ["dir", "notes.txt"]
            );
        }
    }
    #[test]
    fn a_listing_of_direct_children_returns_every_kind_test() {
        for inside_archives in [false, true] {
            for read_archives in [false, true] {
                let entries = ListingEntries::from_entries(mixed());
                let entries = if read_archives { entries.reading_archives() } else { entries };
                let options = scoped(&query(None, Some(100)), &[], inside_archives);
                assert_eq!(kept(&options, entries), ["a.png", "clip.mp4", "dir", "notes.txt", "pack.zip"]);
            }
        }
        let options = scoped(&query(None, Some(100)), &[EntryKind::Video], true);
        assert_eq!(
            kept(&options, ListingEntries::from_entries(mixed())),
            ["clip.mp4"],
            "named kinds apply to direct children too"
        );
    }
    #[test]
    fn the_folder_listing_and_the_search_api_keep_every_kind_test() -> testresult::TestResult {
        let all = ["a.png", "clip.mp4", "dir", "notes.txt", "pack.zip"];
        assert_eq!(kept(&ListOptions::from_query(&recursive_query())?, ListingEntries::from_entries(mixed())), all);
        assert_eq!(
            kept(&ListOptions::from_query(&recursive_query())?, ListingEntries::from_entries(mixed()).reading_archives()),
            all
        );
        Ok(())
    }
    #[test]
    fn only_a_scope_reading_the_insides_gets_index_conditions_without_a_condition_test() -> testresult::TestResult {
        assert!(scoped(&recursive_query(), &[], false).index_conditions().is_none());
        let inside = scoped(&recursive_query(), &[], true)
            .index_conditions()
            .expect("the insides alone are answered from the index");
        assert!(inside.inside_archives());
        assert!(inside.terms().is_empty());
        assert!(
            scoped(&query(None, None), &[], true).index_conditions().is_none(),
            "direct children are never answered from the index"
        );
        let mut hidden = recursive_query();
        hidden.include_hidden = Some(true);
        assert!(scoped(&hidden, &[], true).index_conditions().is_none());
        // The search API and the folder listing do not read the insides.
        assert!(ListOptions::from_query(&recursive_query())?.index_conditions().is_none());
        // With a name condition, the insides are the only difference between the two answers.
        let mut named = recursive_query();
        named.query = Some("pic".to_string());
        assert!(
            !scoped(&named, &[], false)
                .index_conditions()
                .expect("a name condition is answered from the index")
                .inside_archives()
        );
        Ok(())
    }
    #[test]
    fn index_conditions_differ_by_the_kinds_and_the_insides_test() {
        let key = |kinds: &[EntryKind], inside_archives: bool| scoped(&recursive_query(), kinds, inside_archives).index_conditions().unwrap().key();
        assert_eq!(key(&[], true), key(&[], true));
        assert_ne!(key(&[], true), key(&[EntryKind::Image], true));
        assert_ne!(key(&[EntryKind::Image], true), key(&[EntryKind::Image, EntryKind::Video], true));
        assert_eq!(key(&[EntryKind::Image], true), key(&[EntryKind::Image], true));
        let mut named = recursive_query();
        named.query = Some("pic".to_string());
        let named_key = |inside_archives: bool| scoped(&named, &[], inside_archives).index_conditions().unwrap().key();
        assert_ne!(named_key(false), named_key(true));
        // Without the insides the default kinds keep archives, and with them they do not, so the two answers differ in their kinds too.
        let default_kinds = |inside_archives: bool| scoped(&named, &[], inside_archives).index_conditions().unwrap().kinds;
        assert!(default_kinds(false).contains(EntryKind::Archive));
        assert!(!default_kinds(true).contains(EntryKind::Archive));
        assert!(!default_kinds(false).contains(EntryKind::Folder));
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
    pub fn write_7z(path: &Path, entries: &[(&str, &[u8])]) -> testresult::TestResult {
        let mut writer = sevenz_rust2::ArchiveWriter::create(path)?;
        writer.push_archive_entries(
            entries.iter().map(|(name, _)| sevenz_rust2::ArchiveEntry::new_file(name)).collect(),
            entries
                .iter()
                .map(|(_, bytes)| sevenz_rust2::SourceReader::new(std::io::Cursor::new(bytes.to_vec())))
                .collect(),
        )?;
        writer.finish()?;
        Ok(())
    }
    /// `pack.7z` under `library`, with a nested archive that holds another: three archive files deep.
    pub fn write_nested_archives(library: &Path) -> testresult::TestResult {
        let scratch = library.parent().expect("the library has a parent").join("scratch");
        std::fs::create_dir_all(&scratch)?;
        write_7z(&scratch.join("deeper.7z"), &[("d.txt", b"d")])?;
        write_7z(&scratch.join("nested.7z"), &[("c.png", b"c"), ("deeper.7z", &std::fs::read(scratch.join("deeper.7z"))?)])?;
        write_7z(
            &library.join("pack.7z"),
            &[("a.txt", b"a"), ("dir/b.png", b"b"), ("nested.7z", &std::fs::read(scratch.join("nested.7z"))?)],
        )?;
        Ok(())
    }
    /// Walks the archive `pack.7z` of `root/library` as an archive start is walked, and returns every path it lists and whether it read archives like folders.
    async fn walk_archive_start(root: &Path, max_nested_depth: usize, descend: bool) -> Result<(Vec<String>, bool)> {
        let rng = ChaCha20Rng::seed_from_u64(1);
        let provider = Arc::new(Mutex::new(TsidProviderImpl::new(ClockUtc, rng, 16)));
        let options = ArchiveOptions {
            max_nested_depth,
            ..ArchiveOptions::default()
        };
        let runtime = Arc::new(ArchiveRuntime::open(options, &root.join("cache")).await?);
        let storage = Arc::new(StorageService::new(Arc::new(LocalStorageDriver::new(provider)), runtime));
        let library_root = root.join("library");
        let mut nested =
            omnius_pxna_engine::model::NestedPathBuf::try_from(library_root.join("pack.7z").as_path()).map_err(|error| Error::from_error(error, ErrorKind::InvalidInput))?;
        nested.push_archive_root().map_err(|error| Error::from_error(error, ErrorKind::InvalidInput))?;
        let folder = storage.resolve_entry(&nested, WorkClass::Foreground).await?;
        let walk = if descend {
            ArchiveWalk::Descend { storage, max_nested_depth }
        } else {
            ArchiveWalk::Skip
        };
        let scanned = ListingEntries::scan(&folder, &library_root, true, false, 1000, &walk).await?;
        let mut paths = scanned.entries.iter().map(|listed| listed.entry.path.clone()).collect::<Vec<_>>();
        paths.sort();
        Ok((paths, scanned.read_archives))
    }
    #[tokio::test]
    async fn a_walk_of_an_archive_start_lists_a_nested_archive_as_one_entry_without_the_insides_test() -> testresult::TestResult {
        let dir = tempfile::tempdir()?;
        std::fs::create_dir(dir.path().join("library"))?;
        write_nested_archives(&dir.path().join("library"))?;
        let (paths, read_archives) = walk_archive_start(dir.path(), 4, false).await?;
        assert_eq!(paths, ["/pack.7z/a.txt", "/pack.7z/dir", "/pack.7z/dir/b.png", "/pack.7z/nested.7z"]);
        assert!(!read_archives);
        Ok(())
    }
    #[tokio::test]
    async fn a_walk_of_an_archive_start_reads_nested_archives_with_the_insides_test() -> testresult::TestResult {
        let dir = tempfile::tempdir()?;
        std::fs::create_dir(dir.path().join("library"))?;
        write_nested_archives(&dir.path().join("library"))?;
        let (paths, read_archives) = walk_archive_start(dir.path(), 4, true).await?;
        assert_eq!(
            paths,
            [
                "/pack.7z/a.txt",
                "/pack.7z/dir",
                "/pack.7z/dir/b.png",
                "/pack.7z/nested.7z",
                "/pack.7z/nested.7z/c.png",
                "/pack.7z/nested.7z/deeper.7z",
                "/pack.7z/nested.7z/deeper.7z/d.txt"
            ]
        );
        assert!(read_archives, "archives are read like folders, so they are not returned as Entries of their own");
        Ok(())
    }
    #[tokio::test]
    async fn a_walk_that_reads_nested_archives_stops_at_the_depth_limit_test() -> testresult::TestResult {
        let dir = tempfile::tempdir()?;
        std::fs::create_dir(dir.path().join("library"))?;
        write_nested_archives(&dir.path().join("library"))?;
        // `pack.7z` is the first archive file and `nested.7z` the second, so `deeper.7z` would be the third.
        let (paths, _) = walk_archive_start(dir.path(), 2, true).await?;
        assert!(paths.contains(&"/pack.7z/nested.7z/c.png".to_string()));
        assert!(paths.contains(&"/pack.7z/nested.7z/deeper.7z".to_string()), "the archive at the limit is an Entry");
        assert!(!paths.contains(&"/pack.7z/nested.7z/deeper.7z/d.txt".to_string()), "and is not read: {paths:?}");
        let (paths, _) = walk_archive_start(dir.path(), 1, true).await?;
        assert!(paths.contains(&"/pack.7z/nested.7z".to_string()));
        assert!(!paths.contains(&"/pack.7z/nested.7z/c.png".to_string()), "{paths:?}");
        Ok(())
    }
    async fn listed_paths(state: &crate::state::DaemonState, path: &str, inside_archives: bool) -> Result<Vec<String>> {
        let mut q = recursive_query();
        q.path = path.to_string();
        let options = scoped(&q, &[], inside_archives);
        let mut paths = state
            .library
            .list_folder_entries("test", path, &options)
            .await?
            .entries
            .into_iter()
            .map(|entry| entry.path)
            .collect::<Vec<_>>();
        paths.sort();
        Ok(paths)
    }
    #[tokio::test]
    async fn a_walk_under_a_local_folder_reads_no_archive_and_an_archive_start_reads_its_insides_test() -> testresult::TestResult {
        let (_dir, state) = crate::service::mutation::tests::state().await?;
        let library = &state.conf.libraries[0].path;
        write_nested_archives(library)?;
        std::fs::write(library.join("top.txt"), b"t")?;

        // The index is not usable, so a walk answers, and it reads no archive whatever the Scope reads. An archive is a file to it.
        for inside_archives in [false, true] {
            assert_eq!(listed_paths(&state, "/", inside_archives).await?, ["/pack.7z", "/top.txt"], "insides {inside_archives}");
        }
        // An archive start is always walked inside, and reads the archives nested in it only with the insides.
        assert_eq!(
            listed_paths(&state, "/pack.7z", false).await?,
            ["/pack.7z/a.txt", "/pack.7z/dir/b.png", "/pack.7z/nested.7z"]
        );
        assert_eq!(
            listed_paths(&state, "/pack.7z", true).await?,
            ["/pack.7z/a.txt", "/pack.7z/dir/b.png", "/pack.7z/nested.7z/c.png", "/pack.7z/nested.7z/deeper.7z/d.txt"],
            "an archive read like a folder is not an Entry of its own"
        );
        Ok(())
    }
    async fn scan_test(root: &Path, include_hidden: bool, limit: usize) -> Result<models::FolderListing> {
        let rng = ChaCha20Rng::seed_from_u64(1);
        let provider = Arc::new(Mutex::new(TsidProviderImpl::new(ClockUtc, rng, 16)));
        let runtime = Arc::new(ArchiveRuntime::open(ArchiveOptions::default(), &root.join("cache")).await?);
        let storage = Arc::new(StorageService::new(Arc::new(LocalStorageDriver::new(provider)), runtime));
        let library_root = root.join("library");
        let nested = omnius_pxna_engine::model::NestedPathBuf::try_from(library_root.as_path()).map_err(|error| Error::from_error(error, ErrorKind::InvalidInput))?;
        let folder = storage.resolve_entry(&nested, WorkClass::Foreground).await?;
        let scanned = ListingEntries::scan(&folder, &library_root, true, include_hidden, limit, &ArchiveWalk::Skip).await?;
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
