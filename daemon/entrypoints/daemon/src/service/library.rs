use omnius_pxna_engine::service::WorkClass;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use futures::StreamExt as _;
use tokio::sync::Semaphore;

use tokio::io::{AsyncReadExt as _, AsyncSeekExt as _};

use omnius_pxna_engine::{
    model::FsEntryDetail,
    service::{NestedFsEntry, ResolvedStoragePath, StorageService},
};
use omnius_pxna_interface::models;

use crate::{
    config::LibraryConfig,
    prelude::*,
    service::listing::{
        ArchiveWalk, ListOptions, ListingEntries, ListingSnapshot, ListingSnapshots, ListingSource, MarkSelection, NameIndex, ResolvedScope, ResolvedStart, Scope, ScopeListing,
        ScopeResults, ScopeStart, SearchResults, SearchSource, SnapshotKey, SnapshotVersion, StartResolution, SummaryData, SummaryKey, SummaryOptions, SummarySnapshot,
    },
};

mod children;
pub use children::ChildProbeCache;
mod content;
mod entry;
mod indexed;
mod path;

pub use content::*;
pub use entry::*;
pub use indexed::*;
pub use path::*;

/// Library 相対 path で表された読み取り要求を、`StorageService` の操作へ落とす。
pub struct LibraryBrowser {
    /// 一覧で返す順序を保つため、設定に書いた順のまま持つ。
    libraries: Vec<LibraryConfig>,
    storage: Arc<StorageService>,
    resolver: Arc<LibraryPathResolver>,
    snapshots: Arc<ListingSnapshots>,
    details: Arc<Semaphore>,
    indexed: Arc<IndexedListing>,
}

impl LibraryBrowser {
    pub fn new(libraries: &[LibraryConfig], storage: Arc<StorageService>, max_nested_depth: usize, snapshots: Arc<ListingSnapshots>, names: Arc<NameIndex>) -> Self {
        let resolver = Arc::new(LibraryPathResolver::new(storage.clone(), max_nested_depth));
        let details = Arc::new(Semaphore::new(4));
        Self {
            libraries: libraries.to_vec(),
            indexed: Arc::new(IndexedListing::new(names, resolver.clone(), details.clone())),
            resolver,
            storage,
            snapshots,
            details,
        }
    }

    pub fn list_libraries(&self) -> Vec<models::Library> {
        self.libraries.iter().map(models::Library::from).collect()
    }

    pub fn get_library(&self, library_id: &str) -> Result<models::Library> {
        self.library(library_id).map(models::Library::from)
    }

    /// archive file を指す path は、folder として archive の root を返す。
    pub async fn get_folder(&self, library_id: &str, path: &str) -> Result<models::FileSystemEntry> {
        let library = self.library(library_id)?;
        let (_, entry) = self.resolve_folder(library, path).await?;
        EntryConverter::new(&library.path).convert(&entry)
    }

    pub async fn list_folder_entries(&self, library_id: &str, path: &str, options: &ListOptions) -> Result<models::FolderListing> {
        Ok(self.snapshot(library_id, path, options).await?.page(options))
    }

    /// `entry` is relative to the start of the listing; returns the page that holds it.
    pub async fn list_folder_entries_located(&self, library_id: &str, path: &str, options: &ListOptions, entry: &str) -> Result<models::FolderListing> {
        self.snapshot(library_id, path, options)
            .await?
            .locate(options, entry)
            .ok_or_else(|| Error::new(ErrorKind::NotFound).with_message(format!("entry not found in the listing: {entry}")))
    }

    /// Library ごとの一覧を並べ方の順にまとめる。`libraries` は要求の順の Library の id と、その Library の評価とタグで引いた条件の組である。
    /// 起点の `path` は Library が 1 つのときだけ指定でき、複数のときは各 Library のルートから探す。
    pub async fn search_entries(&self, libraries: Vec<(String, ListOptions)>, path: &str, options: &ListOptions) -> Result<models::SearchListing> {
        Ok(self.search_results(libraries, path).await?.page(options))
    }

    /// `entry` is relative to the start of the listing; returns the page that holds the Entry of `library_id`.
    pub async fn search_entries_located(
        &self,
        libraries: Vec<(String, ListOptions)>,
        path: &str,
        options: &ListOptions,
        library_id: &str,
        entry: &str,
    ) -> Result<models::SearchListing> {
        self.search_results(libraries, path)
            .await?
            .locate(options, library_id, entry)
            .ok_or_else(|| Error::new(ErrorKind::NotFound).with_message(format!("entry not found in the listing: {entry}")))
    }

    /// The page of `options` of the Entries of the starts of `scope`, merged in the order of the sort. `selections` holds, by Library id, the rating and tag selection
    /// that `options`' rating and tag conditions came to for the requesting user in that Library.
    pub async fn list_entries(&self, scope: &Scope, selections: &HashMap<String, MarkSelection>, options: &ListOptions) -> Result<ScopeListing> {
        Ok(self.scope_results(scope, selections, options).await?.page(options))
    }

    /// The page that holds the Entry at the Library-relative `path` of `library_id` in the sequence of `list_entries`, or `NotFound` when it is not in it.
    pub async fn list_entries_located(
        &self,
        scope: &Scope,
        selections: &HashMap<String, MarkSelection>,
        options: &ListOptions,
        library_id: &str,
        path: &str,
    ) -> Result<ScopeListing> {
        self.scope_results(scope, selections, options)
            .await?
            .locate(options, library_id, path)
            .ok_or_else(|| Error::new(ErrorKind::NotFound).with_message(format!("entry not found in the listing: {library_id}{path}")))
    }

    /// Builds the listing of each start that adds Entries, from the snapshot of that start with the rating and tag selection of its Library.
    async fn scope_results(&self, scope: &Scope, selections: &HashMap<String, MarkSelection>, options: &ListOptions) -> Result<ScopeResults> {
        let resolved = self.resolve_scope(scope).await?;
        let sources = self.scope_sources(&resolved, selections, options).await?;
        Ok(ScopeResults::new(sources, resolved.missing().to_vec()))
    }

    /// The listing of each start of `resolved`, from the snapshot of that start with the rating and tag selection of its Library.
    pub async fn scope_sources(&self, resolved: &ResolvedScope, selections: &HashMap<String, MarkSelection>, options: &ListOptions) -> Result<Vec<ListingSource>> {
        futures::future::try_join_all(resolved.starts().iter().map(|start| async move {
            let library_id = start.start().library_id();
            let library = self.library(library_id)?;
            let options = options.clone().with_selection(selections.get(library_id).cloned().unwrap_or_default());
            let path = start.start().path().absolute();
            let snapshot = self.snapshot_of(library, &path, start.folder().clone(), start.entry(), &options).await?;
            Ok::<_, Error>(ListingSource::for_start(
                start.start().clone(),
                snapshot,
                options,
                start.folder().archive_identity().is_none(),
            ))
        }))
        .await
    }

    async fn search_results(&self, libraries: Vec<(String, ListOptions)>, path: &str) -> Result<SearchResults> {
        let mut seen = HashSet::new();
        for (library_id, _) in &libraries {
            self.library(library_id)?;
            if !seen.insert(library_id.as_str()) {
                return Err(Error::new(ErrorKind::InvalidInput).with_message(format!("duplicate library id: {library_id}")));
            }
        }
        if libraries.len() > 1 && !LibraryRelativePath::parse(path)?.segments().is_empty() {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("path can be given only with one library"));
        }
        let snapshots = futures::future::try_join_all(libraries.iter().map(|(library_id, options)| self.snapshot(library_id, path, options))).await?;
        let sources = libraries
            .into_iter()
            .zip(snapshots)
            .map(|((library_id, options), snapshot)| SearchSource::new(&library_id, snapshot, options))
            .collect();
        Ok(SearchResults::new(sources))
    }

    pub async fn list_folder_summaries(&self, library_id: &str, path: &str, options: &SummaryOptions) -> Result<models::FolderSummaryListing> {
        Ok(self.summary_snapshot(library_id, path, options).await?.page(options))
    }

    /// `entry` is relative to the start of the listing; returns the page that holds it.
    pub async fn list_folder_summaries_located(&self, library_id: &str, path: &str, options: &SummaryOptions, entry: &str) -> Result<models::FolderSummaryListing> {
        self.summary_snapshot(library_id, path, options)
            .await?
            .locate(options, entry)
            .ok_or_else(|| Error::new(ErrorKind::NotFound).with_message(format!("entry not found in the listing: {entry}")))
    }

    async fn summary_snapshot(&self, library_id: &str, path: &str, options: &SummaryOptions) -> Result<Arc<SummarySnapshot>> {
        let library = self.library(library_id)?;
        let (folder, entry) = self.resolve_folder(library, path).await?;
        let key = SummaryKey::new(library_id, &library.path, path, options)?;
        let version = SnapshotVersion::for_folder(&folder, &entry, false);
        let root = library.path.clone();
        let include_hidden = options.include_hidden();
        let budget = self.storage.work_budget().clone();
        self.snapshots
            .summaries()
            .get_or_scan(key, version, move || async move { SummaryData::scan(&folder, &root, include_hidden, &budget).await })
            .await
    }

    pub async fn entry_details(&self, library_id: &str, paths: &[String]) -> Result<models::EntryDetailsResponse> {
        let library = self.library(library_id)?;
        if paths.is_empty() || paths.len() > 200 {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("paths must contain 1-200 entries"));
        }
        let mut seen = HashSet::new();
        let paths = paths
            .iter()
            .map(|path| {
                let relative = LibraryRelativePath::parse(path)?;
                let path = format!("/{}", relative.segments().join("/"));
                if !seen.insert(path.clone()) {
                    return Err(Error::new(ErrorKind::InvalidInput).with_message("duplicate normalized paths"));
                }
                Ok((path, relative))
            })
            .collect::<Result<Vec<_>>>()?;
        let results = futures::stream::iter(paths)
            .map(|(path, relative)| async move {
                let result = async {
                    let _permit = self.details.acquire().await.map_err(|_| Error::new(ErrorKind::Cancelled))?;
                    let resolved = self.resolver.resolve_listing_entry(&library.path, &relative).await?;
                    let entry = resolved.get_listing_entry(WorkClass::Foreground).await?;
                    EntryConverter::new(&library.path).convert(&entry)
                }
                .await;
                let mut detail = models::EntryDetailResult::new(path);
                match result {
                    Ok(entry) => detail.detail = Some(entry),
                    Err(error) => detail.error = Some(crate::error::ApiError::from_error(&error).body.code),
                }
                detail
            })
            .buffered(4)
            .collect::<Vec<_>>()
            .await;
        Ok(models::EntryDetailsResponse::new(results))
    }

    /// thumbnail の対象になる path を解決する。file、フォルダ、archive のどれでもよく、種別ごとの扱いは engine が決める。
    pub async fn thumbnail_source(&self, library_id: &str, path: &str) -> Result<ResolvedStoragePath> {
        let library = self.library(library_id)?;
        self.resolve(library, path).await
    }

    /// archive file を指す path は、root へ入らず archive file 自体の entry を返す。
    pub async fn get_file(&self, library_id: &str, path: &str) -> Result<models::FileSystemEntry> {
        let library = self.library(library_id)?;
        let (_, entry) = self.resolve_file(library, path).await?;
        EntryConverter::new(&library.path).convert(&entry)
    }

    /// engine からは `open_file` の stream として読み、file 全体を一度に読み込まない。
    pub async fn read_file_content(&self, library_id: &str, path: &str, range: &ReadRange) -> Result<FileContent> {
        let library = self.library(library_id)?;
        let (nested, _) = self.resolve_file(library, path).await?;

        let mut opened = nested.open_file(WorkClass::Foreground).await?;
        let total_len = opened.len().await?;
        let (start, end_exclusive) = range.resolve(total_len)?;

        let mut bytes = vec![0_u8; (end_exclusive - start) as usize];
        opened.seek(std::io::SeekFrom::Start(start)).await?;
        opened.read_exact(&mut bytes).await?;

        Ok(FileContent {
            bytes,
            total_len,
            range: range.is_partial().then(|| (start, end_exclusive - 1)),
        })
    }
}

impl LibraryBrowser {
    async fn snapshot(&self, library_id: &str, path: &str, options: &ListOptions) -> Result<Arc<ListingSnapshot>> {
        let library = self.library(library_id)?;
        let (folder, entry) = self.resolve_folder(library, path).await?;
        self.snapshot_of(library, path, folder, &entry, options).await
    }

    /// The snapshot of the listing of the folder `folder`, which is what `path` of `library` resolved to.
    async fn snapshot_of(&self, library: &LibraryConfig, path: &str, folder: ResolvedStoragePath, entry: &NestedFsEntry, options: &ListOptions) -> Result<Arc<ListingSnapshot>> {
        let library_id = library.id.as_str();
        let mut key = SnapshotKey::new(library_id, path, options)?;
        let version = SnapshotVersion::for_folder(&folder, entry, options.recursive());
        let root = library.path.clone();
        let recursive = options.recursive();
        let include_hidden = options.include_hidden();
        let max_entries = self.snapshots.recursive_max_entries();
        // The walk of an archive start reads the archives nested in it like folders when the Scope reads their insides. A walk under a local folder reads none,
        // since the insides of the archives there come from the name index.
        let archives = if options.recursive() && options.inside_archives() && folder.archive_identity().is_some() {
            ArchiveWalk::Descend {
                storage: self.storage.clone(),
                max_nested_depth: self.storage.archive_options().max_nested_depth,
            }
        } else {
            ArchiveWalk::Skip
        };
        // A recursive listing with conditions from a local folder is answered from the name index while it is usable, and walks otherwise.
        let mut from_index = None;
        if folder.archive_identity().is_none()
            && let Some(conditions) = options.index_conditions()
            && self.indexed.is_usable(library_id).await
        {
            key = key.answered_from(&conditions);
            from_index = Some((
                self.indexed.clone(),
                library.clone(),
                format!("/{}", LibraryRelativePath::parse(path)?.segments().join("/")),
                conditions,
            ));
        }
        self.snapshots
            .get_or_scan(key, version, move || async move {
                if let Some((indexed, library, start, conditions)) = from_index
                    && let Some(entries) = indexed.entries(&library, &start, &conditions, max_entries).await?
                {
                    return Ok(entries);
                }
                ListingEntries::scan(&folder, &root, recursive, include_hidden, max_entries, &archives).await
            })
            .await
    }

    fn library(&self, library_id: &str) -> Result<&LibraryConfig> {
        self.libraries
            .iter()
            .find(|library| library.id == library_id)
            .ok_or_else(|| Error::new(ErrorKind::NotFound).with_message(format!("library not found: {library_id}")))
    }

    async fn resolve(&self, library: &LibraryConfig, path: &str) -> Result<ResolvedStoragePath> {
        let relative = LibraryRelativePath::parse(path)?;
        self.resolver.resolve_entry(&library.path, &relative).await
    }

    /// file 系の endpoint では、archive file を指す path を archive file 自体として解釈する。
    async fn resolve_file(&self, library: &LibraryConfig, path: &str) -> Result<(ResolvedStoragePath, NestedFsEntry)> {
        let nested = self.resolve(library, path).await?;
        let entry = nested.get_entry(WorkClass::Foreground).await?;
        if !matches!(entry.detail, FsEntryDetail::File { .. }) {
            return Err(Error::new(ErrorKind::InvalidInput).with_message(format!("not a file: {path}")));
        }
        Ok((nested, entry))
    }

    /// folder 系の endpoint では、archive file を指す path を archive の root として解釈する。
    async fn resolve_folder(&self, library: &LibraryConfig, path: &str) -> Result<(ResolvedStoragePath, NestedFsEntry)> {
        self.locate_folder(library, path)
            .await?
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput).with_message(format!("not a folder: {path}")))
    }

    /// Resolves `path` as `resolve_folder` does, and tells a file that is neither a folder nor an archive, which is `None`, from an error.
    async fn locate_folder(&self, library: &LibraryConfig, path: &str) -> Result<Option<(ResolvedStoragePath, NestedFsEntry)>> {
        let nested = self.resolve(library, path).await?;
        let entry = nested.get_entry(WorkClass::Foreground).await?;
        match entry.detail {
            FsEntryDetail::Dir { .. } => Ok(Some((nested, entry))),
            FsEntryDetail::File { archive: true } => {
                let mut path = nested.path().to_nested_path_buf();
                path.push_archive_root().map_err(|error| Error::from_error(error, ErrorKind::InvalidInput))?;
                let nested = self.storage.resolve_entry(&path, WorkClass::Foreground).await?;
                let mut root = nested.get_entry(WorkClass::Foreground).await?;
                // archive の root は file 自体の path と名前で見せ、metadata だけを root のものにする。
                root.path = entry.path;
                Ok(Some((nested, root)))
            }
            _ => Ok(None),
        }
    }
}

impl LibraryBrowser {
    /// Whether an ancestor of the start's path is a file that is not an archive, judged as listings judge it, by the name.
    async fn passes_through_plain_file(&self, library: &LibraryConfig, start: &ScopeStart) -> bool {
        let segments = start.path().segments();
        for end in 1..segments.len() {
            let ancestor = format!("/{}", segments[..end].join("/"));
            let Ok(nested) = self.resolve(library, &ancestor).await else {
                return false;
            };
            match nested.get_entry(WorkClass::Foreground).await.map(|entry| entry.detail) {
                Ok(FsEntryDetail::Dir { .. }) => {}
                Ok(FsEntryDetail::File { archive: false }) => return true,
                _ => return false,
            }
        }
        false
    }

    /// Resolves every start of the Scope as `resolve_folder` does, in parallel, then drops the starts that other starts cover.
    ///
    /// An unknown Library, a path that does not exist, and a path that names a file that is neither a folder nor an archive are missing starts, and
    /// any other error fails the request.
    pub async fn resolve_scope(&self, scope: &Scope) -> Result<ResolvedScope> {
        let resolutions = futures::future::try_join_all(scope.starts().iter().map(|start| self.resolve_start(start))).await?;
        Ok(ResolvedScope::new(resolutions, scope))
    }

    async fn resolve_start(&self, start: &ScopeStart) -> Result<StartResolution> {
        let missing = || StartResolution::Missing(start.clone());
        let Ok(library) = self.library(start.library_id()) else {
            return Ok(missing());
        };
        match self.locate_folder(library, &start.path().absolute()).await {
            Ok(Some((folder, entry))) => Ok(StartResolution::Found(Box::new(ResolvedStart::new(start.clone(), folder, entry, &library.path)?))),
            Ok(None) => Ok(missing()),
            Err(error) if error.kind() == &ErrorKind::NotFound => Ok(missing()),
            // A path through a plain file cannot be read as an archive, which is `UnsupportedType`, as an archive start that cannot be opened is too,
            // so only a plain file among the ancestors makes the start missing.
            Err(error) if error.kind() == &ErrorKind::UnsupportedType && self.passes_through_plain_file(library, start).await => Ok(missing()),
            Err(error) => Err(error),
        }
    }
}

impl From<&LibraryConfig> for models::Library {
    /// host の path は API に出さず、id、名前、readonly だけを写す。
    fn from(library: &LibraryConfig) -> Self {
        models::Library::new(library.id.clone(), library.name.clone(), library.readonly)
    }
}

#[cfg(test)]
mod tests {
    use testresult::TestResult;

    use super::*;
    use crate::{
        service::{listing::tests::write_nested_archives, mutation::tests::state},
        state::DaemonState,
    };

    /// A Library with the folders `a`, `a/b`, and `c`, a plain file, and `pack.7z`, which holds `dir` and the archive `nested.7z`.
    async fn library() -> TestResult<(tempfile::TempDir, Arc<DaemonState>)> {
        let (dir, state) = state().await?;
        let root = &state.conf.libraries[0].path;
        std::fs::create_dir_all(root.join("a/b"))?;
        std::fs::create_dir(root.join("c"))?;
        std::fs::write(root.join("file.txt"), b"plain")?;
        write_nested_archives(root)?;
        Ok((dir, state))
    }

    /// The starts that remain and the starts that are missing, by start value, for a Scope of `values`.
    async fn resolved(state: &DaemonState, values: &[&str], recursive: bool, inside_archives: bool) -> TestResult<(Vec<String>, Vec<String>)> {
        let values = values.iter().map(|value| value.to_string()).collect::<Vec<_>>();
        let resolved = state.library.resolve_scope(&Scope::parse(&values, recursive, inside_archives)?).await?;
        Ok((
            resolved.starts().iter().map(|start| start.start().value()).collect(),
            resolved.missing().iter().map(ScopeStart::value).collect(),
        ))
    }

    /// The starts as the text they were parsed from.
    fn values(starts: &[ScopeStart]) -> Vec<String> {
        starts.iter().map(ScopeStart::value).collect()
    }

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[tokio::test]
    async fn the_three_kinds_of_missing_start_are_named_and_the_others_resolve_test() -> TestResult {
        let (_dir, state) = library().await?;
        let (starts, missing) = resolved(&state, &["unknown/a", "test/a", "test/gone", "test/file.txt", "test/pack.7z/gone", "test"], false, false).await?;
        assert_eq!(starts, ["test/a", "test"]);
        assert_eq!(missing, ["unknown/a", "test/gone", "test/file.txt", "test/pack.7z/gone"]);
        Ok(())
    }

    #[tokio::test]
    async fn a_start_whose_path_passes_through_a_plain_file_is_missing_test() -> TestResult {
        let (_dir, state) = library().await?;
        // The resolution fails with `UnsupportedType` there, and the folder listing APIs keep failing with it.
        let options = crate::service::listing::ListOptions::from_query(&crate::service::listing::tests::query(None, None))?;
        let error = state.library.list_folder_entries("test", "/file.txt/x", &options).await.unwrap_err();
        assert_eq!(*error.kind(), ErrorKind::UnsupportedType);
        let (starts, missing) = resolved(&state, &["test/a", "test/file.txt/x", "test/file.txt/x/y", "test/c"], true, false).await?;
        assert_eq!(starts, ["test/a", "test/c"]);
        assert_eq!(missing, ["test/file.txt/x", "test/file.txt/x/y"]);
        // An archive that cannot be opened is not a plain file, so its start, and a start inside it, still fail the request.
        std::fs::write(state.conf.libraries[0].path.join("broken.7z"), b"not an archive")?;
        for value in ["test/broken.7z", "test/broken.7z/x"] {
            let scope = Scope::parse(&strings(&[value]), true, false)?;
            assert!(state.library.resolve_scope(&scope).await.is_err(), "{value}");
        }
        Ok(())
    }

    #[tokio::test]
    async fn a_scope_with_every_start_missing_has_no_start_and_names_them_all_test() -> TestResult {
        let (_dir, state) = library().await?;
        let (starts, missing) = resolved(&state, &["test/gone", "unknown", "test/file.txt"], true, true).await?;
        assert!(starts.is_empty());
        assert_eq!(missing, ["test/gone", "unknown", "test/file.txt"]);
        Ok(())
    }

    #[tokio::test]
    async fn a_missing_start_covers_nothing_and_is_named_though_a_start_covers_its_path_test() -> TestResult {
        let (_dir, state) = library().await?;
        let (starts, missing) = resolved(&state, &["test/a/b", "test/a/gone/deeper", "test/a"], true, false).await?;
        assert_eq!(starts, ["test/a"]);
        assert_eq!(missing, ["test/a/gone/deeper"], "a start under a covering start is still named when it is missing");
        // The same missing start twice is named once.
        let (_, missing) = resolved(&state, &["test/gone", "test/gone"], true, false).await?;
        assert_eq!(missing, ["test/gone"]);
        Ok(())
    }

    #[tokio::test]
    async fn starts_that_another_start_covers_are_dropped_test() -> TestResult {
        let (_dir, state) = library().await?;
        // The covering start keeps its own position, and an identical start is kept at its first place.
        let (starts, _) = resolved(&state, &["test/a/b", "test/c", "test/a", "test/a/b/", "test/c"], true, false).await?;
        assert_eq!(starts, ["test/c", "test/a"]);
        let (starts, _) = resolved(&state, &["test/c", "test/a", "test/c", "test/a"], true, false).await?;
        assert_eq!(starts, ["test/c", "test/a"]);
        // The root covers everything in the Library that is not behind an archive.
        let (starts, _) = resolved(&state, &["test/a", "test", "test/c"], true, false).await?;
        assert_eq!(starts, ["test"]);
        // A start whose path is only written differently is the same start.
        let (starts, _) = resolved(&state, &["test/a/b", "test/a/../a/./b"], true, false).await?;
        assert_eq!(starts, ["test/a/b"]);
        Ok(())
    }

    #[tokio::test]
    async fn without_recursion_only_the_same_start_is_dropped_test() -> TestResult {
        let (_dir, state) = library().await?;
        let (starts, _) = resolved(&state, &["test", "test/a", "test", "test/a/b", "test/a"], false, false).await?;
        assert_eq!(starts, ["test", "test/a", "test/a/b"]);
        let (starts, _) = resolved(&state, &["test", "test/a", "test"], false, true).await?;
        assert_eq!(starts, ["test", "test/a"], "the insides change nothing without recursion");
        Ok(())
    }

    #[tokio::test]
    async fn an_archive_start_and_a_start_inside_one_are_covered_only_with_the_insides_test() -> TestResult {
        let (_dir, state) = library().await?;
        let values = ["test", "test/pack.7z", "test/pack.7z/dir", "test/pack.7z/nested.7z", "test/a"];
        // Without the insides the root lists the archive as one Entry, so the archive start adds what is inside it, and a start inside the archive is covered
        // by the archive start unless a nested archive lies between them.
        let (starts, missing) = resolved(&state, &values, true, false).await?;
        assert_eq!(starts, ["test", "test/pack.7z", "test/pack.7z/nested.7z"]);
        assert!(missing.is_empty());
        // With them the root covers all of it.
        let (starts, _) = resolved(&state, &values, true, true).await?;
        assert_eq!(starts, ["test"]);
        // Without the root, the archive start covers what is inside it, and the nested archive too only with the insides.
        let (starts, _) = resolved(&state, &["test/pack.7z/nested.7z", "test/pack.7z/dir", "test/pack.7z"], true, true).await?;
        assert_eq!(starts, ["test/pack.7z"]);
        let (starts, _) = resolved(&state, &["test/pack.7z/nested.7z", "test/pack.7z/dir", "test/pack.7z"], true, false).await?;
        assert_eq!(starts, ["test/pack.7z/nested.7z", "test/pack.7z"]);
        // The same archive twice is one start, whatever the Scope reads.
        for inside_archives in [false, true] {
            let (starts, _) = resolved(&state, &["test/pack.7z", "test/pack.7z"], true, inside_archives).await?;
            assert_eq!(starts, ["test/pack.7z"]);
        }
        Ok(())
    }

    #[tokio::test]
    async fn an_error_that_is_not_a_missing_start_fails_the_request_test() -> TestResult {
        let (_dir, state) = library().await?;
        let values = strings(&["test", "test/a"]);
        let scope = Scope::parse(&values, true, false)?;
        assert!(state.library.resolve_scope(&scope).await.is_ok());
        // A start that cannot be represented never reaches the resolution.
        assert_eq!(*Scope::parse(&strings(&["test/../.."]), true, false).unwrap_err().kind(), ErrorKind::InvalidInput);
        Ok(())
    }

    /// A listing of the Entries of `starts` as `listEntries` builds it: a recursive one that reads the insides of archives when `inside_archives`.
    async fn list(state: &DaemonState, starts: &[&str], name: Option<&str>, inside_archives: bool, limit: u32, offset: u64) -> TestResult<ScopeListing> {
        let values = strings(starts);
        let scope = Scope::parse(&values, true, inside_archives)?;
        let mut q = crate::service::listing::tests::query(Some(offset), Some(limit));
        q.recursive = Some(true);
        q.query = name.map(str::to_string);
        let options = crate::service::listing::tests::scoped(&q, &[], inside_archives);
        Ok(state.library.list_entries(&scope, &HashMap::new(), &options).await?)
    }

    fn paths(listing: &ScopeListing) -> Vec<String> {
        listing.entries.iter().map(|scoped| format!("{}:{}", scoped.library_id, scoped.entry.path)).collect()
    }

    /// Runs a full aggregation of the test Library and waits for it, which makes its name index usable.
    async fn aggregate(state: &DaemonState) -> TestResult {
        state.names.verified().await;
        state.timeline.refresh("test").await?;
        for _ in 0..400 {
            let status = state.timeline.status("test").await?;
            if !status.running && status.completed_at.is_some() {
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        Err("the aggregation did not finish".into())
    }

    #[tokio::test]
    async fn the_starts_of_a_scope_are_listed_as_one_sequence_with_the_missing_ones_named_test() -> TestResult {
        let (_dir, state) = library().await?;
        let root = &state.conf.libraries[0].path;
        std::fs::write(root.join("a/one.txt"), b"1")?;
        std::fs::write(root.join("a/b/three.txt"), b"3")?;
        std::fs::write(root.join("c/two.txt"), b"2")?;

        let listing = list(&state, &["test/c", "test/gone", "test/a", "test/a/b", "unknown/x"], None, false, 100, 0).await?;
        assert_eq!(
            paths(&listing),
            ["test:/a/one.txt", "test:/a/b/three.txt", "test:/c/two.txt"],
            "sorted by name in natural order, the covered start adding nothing"
        );
        assert_eq!(listing.total_count, 3);
        assert_eq!(values(&listing.missing_starts), ["test/gone", "unknown/x"]);
        assert!(listing.cut_off_starts.is_empty() && listing.walked_starts.is_empty());

        let all_missing = list(&state, &["test/gone", "unknown"], None, false, 100, 0).await?;
        assert!(all_missing.entries.is_empty() && all_missing.total_count == 0);
        assert_eq!(values(&all_missing.missing_starts), ["test/gone", "unknown"]);

        // Pages of one limit joined in order neither repeat nor skip an Entry, and `locate` returns the page of the offset it gives.
        let mut joined = Vec::new();
        for offset in 0..3 {
            joined.extend(paths(&list(&state, &["test/c", "test/a"], None, false, 1, offset).await?));
        }
        assert_eq!(joined, ["test:/a/one.txt", "test:/a/b/three.txt", "test:/c/two.txt"]);
        let values = strings(&["test/c", "test/a"]);
        let scope = Scope::parse(&values, true, false)?;
        let mut q = crate::service::listing::tests::query(Some(0), Some(2));
        q.recursive = Some(true);
        let options = crate::service::listing::tests::scoped(&q, &[], false);
        let located = state.library.list_entries_located(&scope, &HashMap::new(), &options, "test", "/c/two.txt").await?;
        assert_eq!(located.offset, 2);
        assert_eq!(paths(&located), ["test:/c/two.txt"]);
        let missing = state.library.list_entries_located(&scope, &HashMap::new(), &options, "test", "/c/none.txt").await;
        assert_eq!(*missing.unwrap_err().kind(), ErrorKind::NotFound);
        Ok(())
    }

    #[tokio::test]
    async fn a_start_walked_because_the_index_is_not_usable_is_named_test() -> TestResult {
        let (_dir, state) = library().await?;
        let root = &state.conf.libraries[0].path;
        std::fs::write(root.join("a/one.txt"), b"1")?;
        std::fs::write(root.join("c/one.txt"), b"1")?;

        // The index is not usable yet, so a recursive listing with a name condition is walked, from a local folder and not from an archive.
        let before = list(&state, &["test/a", "test/pack.7z"], Some("one"), false, 100, 0).await?;
        assert_eq!(values(&before.walked_starts), ["test/a"]);
        assert_eq!(paths(&before), ["test:/a/one.txt"]);
        // Without a condition a walk is the intended answer and is not named.
        assert!(list(&state, &["test/a"], None, false, 100, 0).await?.walked_starts.is_empty());

        aggregate(&state).await?;
        let after = list(&state, &["test/a", "test/c"], Some("one"), false, 100, 0).await?;
        assert!(after.walked_starts.is_empty());
        assert_eq!(paths(&after), ["test:/a/one.txt", "test:/c/one.txt"]);
        Ok(())
    }

    #[tokio::test]
    async fn a_start_whose_listing_reaches_the_limit_is_named_cut_off_test() -> TestResult {
        let listing = crate::config::ListingConfig {
            recursive_max_entries: 3,
            ..Default::default()
        };
        let (_dir, state) = crate::service::mutation::tests::state_with_clock(listing, Arc::new(omnius_core_base::clock::ClockUtc)).await?;
        let root = &state.conf.libraries[0].path;
        std::fs::create_dir_all(root.join("big"))?;
        std::fs::create_dir_all(root.join("small"))?;
        for number in 0..6 {
            std::fs::write(root.join(format!("big/{number}.txt")), b"x")?;
        }
        std::fs::write(root.join("small/only.txt"), b"x")?;
        let listing = list(&state, &["test/big", "test/small"], None, false, 100, 0).await?;
        assert_eq!(values(&listing.cut_off_starts), ["test/big"]);
        assert_eq!(listing.total_count, 4, "three of the big folder, which stops at the limit, and the small one");
        Ok(())
    }
}
