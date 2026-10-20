use std::{collections::HashSet, sync::Arc};

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
    service::listing::{ListOptions, ListingEntries, ListingSnapshots, SnapshotKey, SnapshotVersion, SummaryData, SummaryKey, SummaryOptions},
};

mod children;
pub use children::ChildProbeCache;
mod content;
mod entry;
mod path;

pub use content::*;
pub use entry::*;
pub use path::*;

/// Library 相対 path で表された読み取り要求を、`StorageService` の操作へ落とす。
pub struct LibraryBrowser {
    /// 一覧で返す順序を保つため、設定に書いた順のまま持つ。
    libraries: Vec<LibraryConfig>,
    storage: Arc<StorageService>,
    resolver: LibraryPathResolver,
    snapshots: Arc<ListingSnapshots>,
    details: Arc<Semaphore>,
}

impl LibraryBrowser {
    pub fn new(libraries: &[LibraryConfig], storage: Arc<StorageService>, max_nested_depth: usize, snapshots: Arc<ListingSnapshots>) -> Self {
        Self {
            libraries: libraries.to_vec(),
            resolver: LibraryPathResolver::new(storage.clone(), max_nested_depth),
            storage,
            snapshots,
            details: Arc::new(Semaphore::new(4)),
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
        let library = self.library(library_id)?;
        let (folder, entry) = self.resolve_folder(library, path).await?;
        let key = SnapshotKey::new(library_id, path, options)?;
        let version = SnapshotVersion::for_folder(&folder, &entry, options.recursive());
        let root = library.path.clone();
        let recursive = options.recursive();
        let include_hidden = options.include_hidden();
        let max_entries = self.snapshots.recursive_max_entries();
        let snapshot = self
            .snapshots
            .get_or_scan(key, version, move || async move {
                ListingEntries::scan(&folder, &root, recursive, include_hidden, max_entries).await
            })
            .await?;
        Ok(snapshot.page(options))
    }

    pub async fn list_folder_summaries(&self, library_id: &str, path: &str, options: &SummaryOptions) -> Result<models::FolderSummaryListing> {
        let library = self.library(library_id)?;
        let (folder, entry) = self.resolve_folder(library, path).await?;
        let key = SummaryKey::new(library_id, &library.path, path, options)?;
        let version = SnapshotVersion::for_folder(&folder, &entry, false);
        let root = library.path.clone();
        let include_hidden = options.include_hidden();
        let snapshot = self
            .snapshots
            .summaries()
            .get_or_scan(key, version, move || async move { SummaryData::scan(&folder, &root, include_hidden).await })
            .await?;
        Ok(snapshot.page(options))
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
                    let entry = resolved.get_listing_entry().await?;
                    EntryConverter::new(&library.path).convert(&entry)
                }
                .await;
                let mut detail = models::EntryDetailResult::new(path);
                match result {
                    Ok(entry) => detail.detail = Some(entry),
                    Err(error) => detail.error = Some(crate::error::ApiError::from_error(&error).body.kind),
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

        let mut opened = nested.open_file().await?;
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
        let entry = nested.get_entry().await?;
        if !matches!(entry.detail, FsEntryDetail::File { .. }) {
            return Err(Error::new(ErrorKind::InvalidInput).with_message(format!("not a file: {path}")));
        }
        Ok((nested, entry))
    }

    /// folder 系の endpoint では、archive file を指す path を archive の root として解釈する。
    async fn resolve_folder(&self, library: &LibraryConfig, path: &str) -> Result<(ResolvedStoragePath, NestedFsEntry)> {
        let nested = self.resolve(library, path).await?;
        let entry = nested.get_entry().await?;
        match entry.detail {
            FsEntryDetail::Dir { .. } => Ok((nested, entry)),
            FsEntryDetail::File { archive: true } => {
                let mut path = nested.path().to_nested_path_buf();
                path.push_archive_root().map_err(|error| Error::from_error(error, ErrorKind::InvalidInput))?;
                let nested = self.storage.resolve_entry(&path).await?;
                let mut root = nested.get_entry().await?;
                // archive の root は file 自体の path と名前で見せ、metadata だけを root のものにする。
                root.path = entry.path;
                Ok((nested, root))
            }
            _ => Err(Error::new(ErrorKind::InvalidInput).with_message(format!("not a folder: {path}"))),
        }
    }
}

impl From<&LibraryConfig> for models::Library {
    /// host の path は API に出さず、id、名前、readonly だけを写す。
    fn from(library: &LibraryConfig) -> Self {
        models::Library::new(library.id.clone(), library.name.clone(), library.readonly)
    }
}
