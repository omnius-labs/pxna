use std::{io, path::Path, sync::Arc};

use omnius_pxna_engine::{
    model::{FsEntryDetail, NestedPathBuf},
    service::{ConflictPolicy, Publish, RatingTagService, ResolvedStoragePath, StagingFile, StorageService},
};
use omnius_pxna_interface::models;

use crate::{
    config::LibraryConfig,
    prelude::*,
    service::{
        library::{EntryConverter, LibraryPathResolver, LibraryRelativePath},
        listing::{ListingInvalidation, ListingSnapshots},
        timeline::TimelineService,
    },
};

mod operation;
mod upload;

pub use operation::*;
pub use upload::*;

/// 変更系 API が Library の境界を保って storage の単件操作を呼ぶ入口。
pub struct LibraryMutation {
    libraries: Vec<LibraryConfig>,
    pub storage: Arc<StorageService>,
    resolver: LibraryPathResolver,
    snapshots: Arc<ListingSnapshots>,
    timeline: Arc<TimelineService>,
    rating_tags: Arc<RatingTagService>,
}

impl LibraryMutation {
    pub fn new(
        libraries: &[LibraryConfig],
        storage: Arc<StorageService>,
        max_depth: usize,
        snapshots: Arc<ListingSnapshots>,
        timeline: Arc<TimelineService>,
        rating_tags: Arc<RatingTagService>,
    ) -> Self {
        Self {
            libraries: libraries.to_vec(),
            resolver: LibraryPathResolver::new(storage.clone(), max_depth),
            storage,
            snapshots,
            timeline,
            rating_tags,
        }
    }

    /// 変更の終わりに、一覧のスナップショットを捨て、タイムラインへ変更した Entry を知らせ、なくなった Entry の評価とタグを消す。
    ///
    /// タイムラインは、もう存在しない Entry の行を消し終えてから戻る。
    pub async fn apply_changes(&self, invalidation: &ListingInvalidation) {
        self.storage.invalidate_browse_caches();
        self.snapshots.invalidate(invalidation).await;
        let entries = invalidation.entries();
        self.timeline.changed(&entries).await;
        self.forget_missing(&entries).await;
    }

    /// 変更した Entry のうち、ファイルシステムからなくなったものの評価とタグを消す。フォルダと archive のときは、配下の行も消える。
    ///
    /// 移動は先に `renamed` と `moved_across` が行を付け替えているので、移動元の path に消すものは残っていない。
    /// ファイルの操作は済んでいるので、消すのに失敗しても操作は失敗にしない。
    async fn forget_missing(&self, entries: &std::collections::HashMap<String, Vec<String>>) {
        for (id, paths) in entries {
            let Ok(library) = self.library(id, false) else {
                continue;
            };
            for path in paths {
                let local = library.path.join(path.trim_start_matches('/'));
                let gone = matches!(tokio::fs::symlink_metadata(&local).await, Err(error) if error.kind() == io::ErrorKind::NotFound);
                if gone && let Err(error) = self.rating_tags.forget(id, path).await {
                    warn!(library = id, path, ?error, "rating tags: cannot forget a removed entry");
                }
            }
        }
    }

    /// 同じ Library の中の名前の変更と移動を、タイムラインの行の付け替えで反映する。
    ///
    /// ファイルの操作は済んでいるので、反映に失敗しても操作は失敗にせず、次の集計に任せる。
    pub async fn renamed(&self, id: &str, src: &str, dst: &str) {
        if let Err(error) = self.timeline.renamed(id, src, dst).await {
            warn!(library = id, src, dst, ?error, "timeline: cannot apply rename");
        }
        self.moved_across(id, src, id, dst).await;
    }

    /// 移動した Entry の評価とタグを、移動先の path へ付け替える。Library をまたぐ移動にも使う。
    ///
    /// ファイルの操作は済んでいるので、付け替えに失敗しても操作は失敗にしない。
    pub async fn moved_across(&self, src_library: &str, src: &str, dst_library: &str, dst: &str) {
        if let Err(error) = self.rating_tags.moved(src_library, src, dst_library, dst).await {
            warn!(src_library, src, dst_library, dst, ?error, "rating tags: cannot carry marks to the destination");
        }
    }

    pub fn library(&self, id: &str, writable: bool) -> Result<&LibraryConfig> {
        let library = self
            .libraries
            .iter()
            .find(|library| library.id == id)
            .ok_or_else(|| Error::new(ErrorKind::NotFound).with_message("library not found"))?;
        if writable && library.readonly {
            return Err(Error::new(ErrorKind::ReadOnly).with_message("library is readonly"));
        }
        Ok(library)
    }

    /// 入力の相対 path と親を検査し、archive 内の変更を拒否する。
    pub async fn destination(&self, id: &str, relative: &str, create_parents: bool) -> Result<NestedPathBuf> {
        let library = self.library(id, true)?;
        let relative = LibraryRelativePath::parse(relative)?;
        if relative.segments().is_empty() {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("cannot modify the library root"));
        }
        let mut path = library.path.clone();
        let last = relative.segments().len() - 1;
        for (index, segment) in relative.segments().iter().enumerate() {
            path.push(segment);
            if index == last {
                break;
            }
            match tokio::fs::metadata(&path).await {
                Ok(meta) if meta.is_dir() => {}
                Ok(_) => return Err(Error::new(ErrorKind::ReadOnly).with_message("cannot modify inside a file or archive")),
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    if !create_parents {
                        return Err(Error::new(ErrorKind::NotFound).with_message("parent folder not found"));
                    }
                    tokio::fs::create_dir(&path).await.map_err(Self::io_error)?;
                }
                Err(error) => return Err(Self::io_error(error)),
            }
        }
        Self::nested(&path)
    }

    pub async fn source(&self, id: &str, path: &str, folder: bool, writable: bool) -> Result<ResolvedStoragePath> {
        let library = self.library(id, writable)?;
        let relative = LibraryRelativePath::parse(path)?;
        if writable && relative.segments().is_empty() {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("cannot modify the library root"));
        }
        let mut resolved = self.resolver.resolve_entry(&library.path, &relative).await?;
        if writable {
            if resolved.path().rank() != 0 {
                return Err(Error::new(ErrorKind::ReadOnly).with_message("archive is readonly"));
            }
            self.destination(id, path, false).await?;
        }
        let entry = resolved.get_entry().await?;
        if folder && matches!(entry.detail, FsEntryDetail::File { archive: true }) {
            if writable {
                return Err(Error::new(ErrorKind::ReadOnly).with_message("archive is readonly"));
            }
            let mut nested = resolved.path().to_nested_path_buf();
            nested.push_archive_root().map_err(|error| Error::from_error(error, ErrorKind::InvalidInput))?;
            resolved = self.storage.resolve_entry(&nested).await?;
        } else if folder != matches!(entry.detail, FsEntryDetail::Dir { .. }) {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("unexpected entry type"));
        }
        Ok(resolved)
    }

    pub async fn create_folder(&self, id: &str, body: &models::CreateFolderRequest) -> Result<models::FileSystemEntry> {
        let invalidation = ListingInvalidation::for_entry(id, &body.path)?;
        let path = self.destination(id, &body.path, body.create_parents.unwrap_or(false)).await?;
        self.storage.make_dir(&path).await?;
        self.apply_changes(&invalidation).await;
        self.entry(id, &path).await
    }

    pub async fn staging(&self, id: &str, path: &str, conflict: ConflictPolicy) -> Result<(NestedPathBuf, StagingFile)> {
        let dst = self.destination(id, path, false).await?;
        if conflict == ConflictPolicy::Fail && tokio::fs::symlink_metadata(Self::local(&dst)?).await.is_ok() {
            return Err(Error::new(ErrorKind::AlreadyExists).with_message("destination already exists"));
        }
        let parent = Self::local(&dst)?.parent().ok_or_else(|| Error::new(ErrorKind::InvalidInput))?;
        let parent = Self::nested(parent)?;
        self.storage.sweep_staging(&parent).await?;
        Ok((dst, self.storage.create_staging(&parent).await?))
    }

    pub async fn publish(&self, id: &str, relative: &str, staging: StagingFile, conflict: ConflictPolicy) -> Result<models::FileSystemEntry> {
        let mut invalidation = ListingInvalidation::for_entry(id, relative)?;
        let dst = self.destination(id, relative, false).await?;
        let published = staging
            .publish(Self::local(&dst)?.to_str().ok_or_else(|| Error::new(ErrorKind::InvalidInput))?, conflict)
            .await?;
        match published {
            Publish::Published { dst } => {
                // 衝突で別名になったときは、実際に公開した path も変更として知らせる。
                invalidation.include_entry(id, &self.relative(id, &Self::nested(Path::new(&dst))?)?)?;
                self.apply_changes(&invalidation).await;
                self.entry(id, &Self::nested(Path::new(&dst))?).await
            }
            Publish::Skipped => self.entry(id, &dst).await,
        }
    }

    pub async fn write(&self, id: &str, path: &str, bytes: &[u8], conflict: ConflictPolicy) -> Result<models::FileSystemEntry> {
        let (_, mut staging) = self.staging(id, path, conflict).await?;
        staging.write_all(bytes).await?;
        self.publish(id, path, staging, conflict).await
    }

    pub async fn delete(&self, id: &str, path: &str, folder: bool) -> Result<()> {
        let invalidation = ListingInvalidation::for_entry(id, path)?;
        let source = self.source(id, path, folder, true).await?;
        if folder {
            let mut entries = source.find_entries(false).await?;
            use futures::StreamExt as _;
            if entries.next().await.is_some() {
                return Err(Error::new(ErrorKind::InvalidInput).with_message("folder is not empty; recursive=true is required"));
            }
            self.storage.remove_dir(source.path()).await?;
        } else {
            self.storage.remove_file(source.path()).await?;
        }
        self.apply_changes(&invalidation).await;
        Ok(())
    }

    pub async fn entry(&self, id: &str, path: &NestedPathBuf) -> Result<models::FileSystemEntry> {
        let library = self.library(id, false)?;
        EntryConverter::new(&library.path).convert(&self.storage.get_entry(path).await?)
    }

    pub fn relative(&self, id: &str, path: &NestedPathBuf) -> Result<String> {
        let library = self.library(id, false)?;
        let local = Self::local(path)?.strip_prefix(&library.path).map_err(|_| Error::new(ErrorKind::InvalidInput))?;
        Ok(format!("/{}", local.to_string_lossy().replace('\\', "/")))
    }

    pub fn conflict(policy: Option<models::ConflictPolicy>) -> ConflictPolicy {
        match policy.unwrap_or(models::ConflictPolicy::Fail) {
            models::ConflictPolicy::Fail => ConflictPolicy::Fail,
            models::ConflictPolicy::Skip => ConflictPolicy::Skip,
            models::ConflictPolicy::Overwrite => ConflictPolicy::Overwrite,
            models::ConflictPolicy::Rename => ConflictPolicy::Rename,
        }
    }

    pub fn nested(path: &Path) -> Result<NestedPathBuf> {
        let mut nested = NestedPathBuf::new();
        nested
            .push_str(path.to_str().ok_or_else(|| Error::new(ErrorKind::InvalidInput).with_message("path is not UTF-8"))?)
            .map_err(|error| Error::from_error(error, ErrorKind::InvalidInput))?;
        Ok(nested)
    }

    pub fn local(path: &NestedPathBuf) -> Result<&Path> {
        if path.rank() != 0 {
            return Err(Error::new(ErrorKind::ReadOnly).with_message("archive is readonly"));
        }
        Ok(Path::new(path.iter().next().unwrap_or_default()))
    }

    pub fn io_error(error: io::Error) -> Error {
        let kind = match error.kind() {
            io::ErrorKind::NotFound => ErrorKind::NotFound,
            io::ErrorKind::AlreadyExists => ErrorKind::AlreadyExists,
            _ => ErrorKind::IoError,
        };
        Error::from_error(error, kind).with_message("filesystem operation failed")
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{
        config::{CoreConfig, DaemonConfig, LoggingConfig, StorageConfig},
        state::DaemonState,
    };
    use testresult::TestResult;

    pub async fn state() -> TestResult<(tempfile::TempDir, Arc<DaemonState>)> {
        state_with_clock(Default::default(), Arc::new(omnius_core_base::clock::ClockUtc)).await
    }

    pub async fn state_with_clock(
        listing: crate::config::ListingConfig,
        clock: Arc<dyn omnius_core_base::clock::Clock<chrono::Utc> + Send + Sync>,
    ) -> TestResult<(tempfile::TempDir, Arc<DaemonState>)> {
        let dir = tempfile::tempdir()?;
        let root = dir.path().join("library");
        std::fs::create_dir(&root)?;
        let conf = DaemonConfig {
            core: CoreConfig {
                state_dir: dir.path().join("state"),
                listen_addr: "127.0.0.1:0".to_string(),
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                json: false,
            },
            libraries: vec![LibraryConfig {
                id: "test".to_string(),
                name: "Test".to_string(),
                path: root,
                readonly: false,
            }],
            storage: StorageConfig { archive: Default::default() },
            listing,
            timeline: Default::default(),
            thumbnail: Default::default(),
            media: Default::default(),
            video: Default::default(),
            web: Default::default(),
        };
        Ok((dir, Arc::new(DaemonState::new_with_clock(conf, clock).await?)))
    }
}
