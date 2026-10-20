use std::{
    collections::{HashMap, HashSet},
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
};

use chrono::{Duration, Utc};
use futures::StreamExt as _;
use omnius_core_base::tsid::TsidProvider;
use omnius_pxna_engine::{
    model::FsEntryDetail,
    service::{ConflictPolicy, Publish, ResolvedStoragePath},
};
use omnius_pxna_interface::{models, types::Nullable};
use tokio_util::sync::CancellationToken;

use crate::{
    error::ApiError,
    prelude::*,
    service::{library::LibraryRelativePath, listing::ListingInvalidation, mutation::LibraryMutation},
};

struct OperationRecord {
    model: models::Operation,
    cancel: CancellationToken,
    changes: Arc<Mutex<ListingInvalidation>>,
}

struct Transfer {
    src_library: String,
    dst_library: String,
    pairs: Vec<(String, String)>,
    folder: bool,
    moving: bool,
    conflict: ConflictPolicy,
    swept: Mutex<HashSet<PathBuf>>,
    changes: Arc<Mutex<ListingInvalidation>>,
}

pub enum MoveResult {
    Completed(Vec<models::FileSystemEntry>),
    Started(models::Operation),
}

pub enum DeleteResult {
    Completed,
    Started(models::Operation),
}

/// ツリーを単件操作へ分解し、部分結果と協調キャンセルを所有する。
pub struct OperationService {
    mutation: Arc<LibraryMutation>,
    ids: Arc<Mutex<dyn TsidProvider + Send + Sync>>,
    operations: Mutex<HashMap<String, Arc<Mutex<OperationRecord>>>>,
    permits: Arc<tokio::sync::Semaphore>,
}

impl OperationService {
    pub fn new(mutation: Arc<LibraryMutation>, ids: Arc<Mutex<dyn TsidProvider + Send + Sync>>) -> Arc<Self> {
        let service = Arc::new(Self {
            mutation,
            ids,
            operations: Mutex::new(HashMap::new()),
            permits: Arc::new(tokio::sync::Semaphore::new(4)),
        });
        let weak = Arc::downgrade(&service);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                let Some(service) = weak.upgrade() else {
                    break;
                };
                service.reap();
            }
        });
        service
    }

    pub fn get(&self, id: &str) -> Result<models::Operation> {
        self.reap();
        Ok(self.record(id)?.lock().model.clone())
    }

    pub fn list(&self) -> Vec<models::Operation> {
        self.reap();
        let mut operations = self.operations.lock().values().map(|record| record.lock().model.clone()).collect::<Vec<_>>();
        operations.sort_by(|a, b| b.created_at.cmp(&a.created_at).then_with(|| a.id.cmp(&b.id)));
        operations
    }

    pub fn cancel(&self, id: &str) -> Result<models::Operation> {
        let record = self.record(id)?;
        let mut record = record.lock();
        if matches!(record.model.status, models::OperationStatus::Pending | models::OperationStatus::Running) {
            record.cancel.cancel();
            if record.model.status == models::OperationStatus::Pending {
                record.model.status = models::OperationStatus::Cancelled;
                record.model.finished_at = Some(Nullable::Present(Utc::now()));
            }
        }
        Ok(record.model.clone())
    }

    pub async fn copy(self: &Arc<Self>, library: &str, request: &models::EntryTransferRequest, folder: bool) -> Result<models::Operation> {
        let transfer = self.prepare(library, request, folder, false).await?;
        Ok(self.start_transfer(transfer))
    }

    pub async fn move_entries(self: &Arc<Self>, library: &str, request: &models::EntryTransferRequest, folder: bool) -> Result<MoveResult> {
        let transfer = self.prepare(library, request, folder, true).await?;
        if transfer.pairs.len() != 1 || transfer.src_library != transfer.dst_library {
            return Ok(MoveResult::Started(self.start_transfer(transfer)));
        }
        let (src, dst) = &transfer.pairs[0];
        let source = self.mutation.source(library, src, folder, true).await?;
        let destination = self.mutation.destination(library, dst, false).await?;
        let merge =
            folder && transfer.conflict == ConflictPolicy::Overwrite && tokio::fs::symlink_metadata(LibraryMutation::local(&destination)?).await.is_ok_and(|meta| meta.is_dir());
        if !merge {
            let published = self.mutation.storage.rename(source.path(), &destination, transfer.conflict).await?;
            if let Publish::Published { dst: published_dst } = &published {
                let published_dst = self.mutation.relative(library, &LibraryMutation::nested(Path::new(published_dst))?)?;
                self.mutation.renamed(library, src, &published_dst).await;
                // 衝突で別名になったときは、実際の宛先も変更として知らせ、判定し直させる。
                transfer.changes.lock().include_entry(library, &published_dst)?;
                let changes = transfer.changes.lock().clone();
                self.mutation.apply_changes(&changes).await;
            }
            return Ok(MoveResult::Completed(match published {
                Publish::Published { dst } => vec![self.mutation.entry(library, &LibraryMutation::nested(Path::new(&dst))?).await?],
                Publish::Skipped => Vec::new(),
            }));
        }
        // 同名フォルダへの overwrite は各子を rename してマージする。バイト列のコピーは伴わない。
        let result = self
            .transfer_entry(&transfer, source, src.clone(), LibraryMutation::local(&destination)?.to_path_buf(), None)
            .await;
        let changes = transfer.changes.lock().clone();
        self.mutation.apply_changes(&changes).await;
        let mut result = result?;
        result.truncate(1);
        Ok(MoveResult::Completed(result))
    }

    pub async fn delete_entries(self: &Arc<Self>, library: &str, paths: &[models::RelativePath], folder: bool, recursive: bool) -> Result<DeleteResult> {
        if paths.is_empty() {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("srcPaths must not be empty"));
        }
        self.mutation.library(library, true)?;
        for path in paths {
            self.mutation.source(library, &path.0, folder, true).await?;
        }
        if (folder && !recursive) || (!folder && paths.len() == 1) {
            for path in paths {
                self.mutation.delete(library, &path.0, folder).await?;
            }
            return Ok(DeleteResult::Completed);
        }
        let kind = if folder {
            models::OperationKind::DeleteFolders
        } else {
            models::OperationKind::DeleteFiles
        };
        let record = self.create(library, kind);
        for path in paths {
            record.lock().changes.lock().include_entry(library, &path.0)?;
        }
        let model = record.lock().model.clone();
        let service = self.clone();
        let library = library.to_string();
        let paths = paths.iter().map(|path| path.0.clone()).collect::<Vec<_>>();
        tokio::spawn(async move {
            let cancel = record.lock().cancel.clone();
            let permit = tokio::select! { permit = service.permits.clone().acquire_owned() => permit.ok(), _ = cancel.cancelled() => None };
            if permit.is_none() {
                service.finish(&record).await;
                return;
            }
            record.lock().model.status = models::OperationStatus::Running;
            for path in paths {
                if cancel.is_cancelled() {
                    break;
                }
                let result = async {
                    let source = service.mutation.source(&library, &path, folder, true).await?;
                    service.delete_tree(&library, source, &path, &record).await
                }
                .await;
                if let Err(error) = result {
                    Self::failure(&record, &path, error);
                }
            }
            service.finish(&record).await;
        });
        Ok(DeleteResult::Started(model))
    }

    async fn prepare(&self, library: &str, request: &models::EntryTransferRequest, folder: bool, moving: bool) -> Result<Transfer> {
        self.mutation.library(library, moving)?;
        let dst_library = request.dst_library_id.as_deref().unwrap_or(library);
        self.mutation.library(dst_library, true)?;
        let conflict = LibraryMutation::conflict(request.conflict);
        if request.src_paths.is_empty() || request.dst_path.is_some() == request.dst_folder_path.is_some() || (request.dst_path.is_some() && request.src_paths.len() != 1) {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("choose dstPath for one source or dstFolderPath"));
        }
        let mut pairs = Vec::new();
        for src in &request.src_paths {
            let relative = LibraryRelativePath::parse(&src.0)?;
            if relative.segments().is_empty() {
                return Err(Error::new(ErrorKind::InvalidInput).with_message("cannot transfer the library root"));
            }
            let src_path = format!("/{}", relative.segments().join("/"));
            let dst = match &request.dst_path {
                Some(path) => path.clone(),
                None => format!(
                    "{}/{}",
                    request.dst_folder_path.as_deref().unwrap_or_default().trim_end_matches('/'),
                    relative.segments().last().unwrap()
                ),
            };
            let dst = self.mutation.destination(dst_library, &dst, false).await?;
            let dst_path = self.mutation.relative(dst_library, &dst)?;
            let source = self.mutation.source(library, &src_path, folder, moving).await?;
            if source.path().rank() == 0 {
                let source_path = Path::new(source.path().iter().next().unwrap_or_default());
                let source_canonical = tokio::fs::canonicalize(source_path).await.map_err(LibraryMutation::io_error)?;
                let dst_local = LibraryMutation::local(&dst)?;
                let dst_canonical = match tokio::fs::canonicalize(dst_local).await {
                    Ok(path) => path,
                    Err(_) => tokio::fs::canonicalize(dst_local.parent().ok_or_else(|| Error::new(ErrorKind::InvalidInput))?)
                        .await
                        .map_err(LibraryMutation::io_error)?
                        .join(dst_local.file_name().unwrap_or_default()),
                };
                let same = dst_canonical == source_canonical;
                let duplicate = same && !moving && conflict == ConflictPolicy::Rename;
                // 別名候補は宛先の親に作る。ソース配下の symlink 経由でも自己配下へ作らない。
                let parent_inside_source = folder
                    && tokio::fs::canonicalize(dst_local.parent().ok_or_else(|| Error::new(ErrorKind::InvalidInput))?)
                        .await
                        .map_err(LibraryMutation::io_error)?
                        .starts_with(&source_canonical);
                if (same && !duplicate) || (folder && !same && dst_canonical.starts_with(&source_canonical)) || parent_inside_source {
                    return Err(Error::new(ErrorKind::InvalidInput).with_message("destination cannot be the source or a descendant"));
                }
            }
            pairs.push((src_path, dst_path));
        }
        let mut changes = ListingInvalidation::default();
        for (src, dst) in &pairs {
            changes.include_entry(library, src)?;
            changes.include_entry(dst_library, dst)?;
        }
        Ok(Transfer {
            src_library: library.to_string(),
            dst_library: dst_library.to_string(),
            pairs,
            folder,
            moving,
            conflict,
            swept: Mutex::new(HashSet::new()),
            changes: Arc::new(Mutex::new(changes)),
        })
    }

    fn start_transfer(self: &Arc<Self>, transfer: Transfer) -> models::Operation {
        let kind = match (transfer.folder, transfer.moving) {
            (false, false) => models::OperationKind::CopyFiles,
            (false, true) => models::OperationKind::MoveFiles,
            (true, false) => models::OperationKind::CopyFolders,
            (true, true) => models::OperationKind::MoveFolders,
        };
        let record = self.create(&transfer.src_library, kind);
        {
            let mut record = record.lock();
            record.changes = transfer.changes.clone();
            record.model.destination_library_id = Some(transfer.dst_library.clone());
            record.model.result_entries = Some(Vec::new());
        }
        let model = record.lock().model.clone();
        let service = self.clone();
        tokio::spawn(async move {
            let cancel = record.lock().cancel.clone();
            let permit = tokio::select! { permit = service.permits.clone().acquire_owned() => permit.ok(), _ = cancel.cancelled() => None };
            if permit.is_none() {
                service.finish(&record).await;
                return;
            }
            record.lock().model.status = models::OperationStatus::Running;
            for (src, dst) in &transfer.pairs {
                if cancel.is_cancelled() {
                    break;
                }
                let result = async {
                    let source = service.mutation.source(&transfer.src_library, src, transfer.folder, transfer.moving).await?;
                    let dst = service.mutation.destination(&transfer.dst_library, dst, false).await?;
                    service
                        .transfer_entry(&transfer, source, src.clone(), LibraryMutation::local(&dst)?.to_path_buf(), Some(record.clone()))
                        .await
                }
                .await;
                match result {
                    Ok(entries) => record.lock().model.result_entries.get_or_insert_with(Vec::new).extend(entries),
                    Err(error) => Self::failure(&record, src, error),
                }
            }
            service.finish(&record).await;
        });
        model
    }

    fn transfer_entry<'a>(
        &'a self,
        transfer: &'a Transfer,
        source: ResolvedStoragePath,
        relative: String,
        destination: PathBuf,
        progress: Option<Arc<Mutex<OperationRecord>>>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<models::FileSystemEntry>>> + Send + 'a>> {
        Box::pin(async move {
            Self::check_cancel(progress.as_ref())?;
            // 実行中に親が差し替わっても、各エントリを置く前に再度境界を検査する。
            let dst_relative = self.mutation.relative(&transfer.dst_library, &LibraryMutation::nested(&destination)?)?;
            {
                let mut changes = transfer.changes.lock();
                changes.include_entry(&transfer.src_library, &relative)?;
                changes.include_entry(&transfer.dst_library, &dst_relative)?;
            }
            self.mutation.destination(&transfer.dst_library, &dst_relative, false).await?;
            let entry = source.get_entry().await?;
            if matches!(entry.detail, FsEntryDetail::Dir { .. }) {
                let Some(destination) = self
                    .directory(&transfer.dst_library, &destination, transfer.conflict, progress.as_ref(), &transfer.changes)
                    .await?
                else {
                    Self::skipped(progress.as_ref());
                    return Ok(Vec::new());
                };
                let results = vec![self.mutation.entry(&transfer.dst_library, &LibraryMutation::nested(&destination)?).await?];
                Self::processed(progress.as_ref());
                for (child_relative, child) in self.children(&source, &relative).await? {
                    Self::check_cancel(progress.as_ref())?;
                    let result = async {
                        let child = child?;
                        let name = child.path().file_name().ok_or_else(|| Error::new(ErrorKind::InvalidInput))?;
                        self.transfer_entry(transfer, child.clone(), child_relative.clone(), destination.join(name), progress.clone())
                            .await
                    }
                    .await;
                    match result {
                        Ok(_) => {}
                        Err(error) if *error.kind() == ErrorKind::Cancelled => return Err(error),
                        Err(error) => match &progress {
                            Some(record) => Self::failure(record, &child_relative, error),
                            None => return Err(error),
                        },
                    }
                }
                if transfer.moving {
                    // skip や個別失敗で残ったソースは保持する。
                    let mut remaining = source.find_entries(false).await?;
                    if remaining.next().await.is_none() {
                        self.mutation.storage.remove_dir(source.path()).await?;
                    }
                }
                Ok(results)
            } else if matches!(entry.detail, FsEntryDetail::File { .. }) {
                let dst = LibraryMutation::nested(&destination)?;
                if transfer.conflict == ConflictPolicy::Overwrite && tokio::fs::symlink_metadata(&destination).await.is_ok_and(|meta| meta.is_dir()) {
                    self.remove_destination(&transfer.dst_library, &destination, progress.as_ref(), &transfer.changes).await?;
                }
                let parent = destination.parent().ok_or_else(|| Error::new(ErrorKind::InvalidInput))?;
                let sweep = transfer.swept.lock().insert(parent.to_path_buf());
                if sweep {
                    self.mutation.storage.sweep_staging(&LibraryMutation::nested(parent)?).await?;
                }
                let operation = async {
                    if transfer.moving && transfer.src_library == transfer.dst_library {
                        self.mutation.storage.rename(source.path(), &dst, transfer.conflict).await
                    } else if transfer.moving {
                        self.mutation.storage.move_file(source.path(), &dst, transfer.conflict).await
                    } else {
                        self.mutation.storage.copy_file(source.path(), &dst, transfer.conflict).await
                    }
                };
                let published = if let Some(record) = &progress {
                    let cancel = record.lock().cancel.clone();
                    tokio::select! { biased; result = operation => result?, _ = cancel.cancelled() => return Err(Error::new(ErrorKind::Cancelled)) }
                } else {
                    operation.await?
                };
                match published {
                    Publish::Published { dst } => {
                        // 同じ Library の中の移動は、公開された宛先へタイムラインの行を付け替える。別名で公開されたときも、実際の宛先を使う。
                        let published = self.mutation.relative(&transfer.dst_library, &LibraryMutation::nested(Path::new(&dst))?)?;
                        // 別名で公開されたときは、実際に公開した path を変更として知らせる。
                        transfer.changes.lock().include_entry(&transfer.dst_library, &published)?;
                        if transfer.moving && transfer.src_library == transfer.dst_library {
                            self.mutation.renamed(&transfer.src_library, &relative, &published).await;
                        } else if transfer.moving {
                            self.mutation.moved_across(&transfer.src_library, &relative, &transfer.dst_library, &published).await;
                        }
                        Self::processed(progress.as_ref());
                        Ok(vec![self.mutation.entry(&transfer.dst_library, &LibraryMutation::nested(Path::new(&dst))?).await?])
                    }
                    Publish::Skipped => {
                        Self::skipped(progress.as_ref());
                        Ok(Vec::new())
                    }
                }
            } else {
                Err(Error::new(ErrorKind::UnsupportedType).with_message("symbolic links cannot be traversed"))
            }
        })
    }

    async fn directory(
        &self,
        library: &str,
        destination: &Path,
        conflict: ConflictPolicy,
        progress: Option<&Arc<Mutex<OperationRecord>>>,
        changes: &Mutex<ListingInvalidation>,
    ) -> Result<Option<PathBuf>> {
        if let Ok(meta) = tokio::fs::symlink_metadata(destination).await {
            match conflict {
                ConflictPolicy::Fail => return Err(Error::new(ErrorKind::AlreadyExists).with_message("destination already exists")),
                ConflictPolicy::Skip => return Ok(None),
                ConflictPolicy::Overwrite if meta.is_dir() => return Ok(Some(destination.to_path_buf())),
                ConflictPolicy::Overwrite => {
                    self.remove_destination(library, destination, progress, changes).await?;
                }
                ConflictPolicy::Rename => {}
            }
        }
        for index in 0..=10_000 {
            let candidate = if index == 0 {
                destination.to_path_buf()
            } else {
                let name = destination.file_name().and_then(|name| name.to_str()).ok_or_else(|| Error::new(ErrorKind::InvalidInput))?;
                destination.with_file_name(format!("{name} ({index})"))
            };
            match tokio::fs::create_dir(&candidate).await {
                Ok(()) => return Ok(Some(candidate)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists && conflict == ConflictPolicy::Rename => {}
                Err(error) => return Err(LibraryMutation::io_error(error)),
            }
        }
        Err(Error::new(ErrorKind::LimitExceeded).with_message("no available destination name"))
    }

    fn remove_destination<'a>(
        &'a self,
        library: &'a str,
        path: &'a Path,
        progress: Option<&'a Arc<Mutex<OperationRecord>>>,
        changes: &'a Mutex<ListingInvalidation>,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            Self::check_cancel(progress)?;
            let relative = self.mutation.relative(library, &LibraryMutation::nested(path)?)?;
            changes.lock().include_entry(library, &relative)?;
            let meta = tokio::fs::symlink_metadata(path).await.map_err(LibraryMutation::io_error)?;
            if meta.is_symlink() {
                return Err(Error::new(ErrorKind::ReadOnly).with_message("cannot overwrite a symbolic link"));
            }
            if meta.is_dir() {
                let mut entries = tokio::fs::read_dir(path).await.map_err(LibraryMutation::io_error)?;
                while let Some(entry) = entries.next_entry().await.map_err(LibraryMutation::io_error)? {
                    self.remove_destination(library, &entry.path(), progress, changes).await?;
                }
                self.mutation.storage.remove_dir(&LibraryMutation::nested(path)?).await?;
            } else {
                self.mutation.storage.remove_file(&LibraryMutation::nested(path)?).await?;
            }
            Ok(())
        })
    }

    async fn children(&self, source: &ResolvedStoragePath, relative: &str) -> Result<Vec<(String, Result<ResolvedStoragePath>)>> {
        let mut children = Vec::new();
        if source.path().rank() == 0 {
            let mut entries = tokio::fs::read_dir(source.path().iter().next().unwrap_or_default())
                .await
                .map_err(LibraryMutation::io_error)?;
            while let Some(entry) = entries.next_entry().await.map_err(LibraryMutation::io_error)? {
                let path = entry.path();
                let name = entry
                    .file_name()
                    .to_str()
                    .map(str::to_string)
                    .ok_or_else(|| Error::new(ErrorKind::InvalidInput).with_message("entry name is not UTF-8"))?;
                let relative = format!("{}/{name}", relative.trim_end_matches('/'));
                let result = async {
                    let meta = tokio::fs::symlink_metadata(&path).await.map_err(LibraryMutation::io_error)?;
                    if meta.is_symlink() || (!meta.is_dir() && !meta.is_file()) {
                        return Err(Error::new(ErrorKind::UnsupportedType).with_message("cannot traverse a symbolic link or special entry"));
                    }
                    Ok(source.at(path.to_str().ok_or_else(|| Error::new(ErrorKind::InvalidInput))?)?)
                }
                .await;
                children.push((relative, result));
            }
        } else {
            let mut entries = source.find_entries(false).await?;
            while let Some(entry) = entries.next().await {
                let entry = entry?;
                let name = entry.path.file_name().ok_or_else(|| Error::new(ErrorKind::InvalidInput))?;
                let relative = format!("{}/{name}", relative.trim_end_matches('/'));
                let result = if matches!(entry.detail, FsEntryDetail::Link { .. }) {
                    Err(Error::new(ErrorKind::UnsupportedType).with_message("cannot traverse a symbolic link"))
                } else {
                    Ok(source.at(entry.path.iter().last().unwrap_or_default())?)
                };
                children.push((relative, result));
            }
        }
        children.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(children)
    }

    fn delete_tree<'a>(
        &'a self,
        library: &'a str,
        source: ResolvedStoragePath,
        relative: &'a str,
        record: &'a Arc<Mutex<OperationRecord>>,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            Self::check_cancel(Some(record))?;
            record.lock().changes.lock().include_entry(library, relative)?;
            self.mutation.destination(library, relative, false).await?;
            let entry = source.get_entry().await?;
            if matches!(entry.detail, FsEntryDetail::Dir { .. }) {
                let mut failed = false;
                for (relative, child) in self.children(&source, relative).await? {
                    Self::check_cancel(Some(record))?;
                    let result = async { self.delete_tree(library, child?, &relative, record).await }.await;
                    if let Err(error) = result {
                        if *error.kind() == ErrorKind::Cancelled {
                            return Err(error);
                        }
                        failed = true;
                        Self::failure(record, &relative, error);
                    }
                }
                if failed {
                    return Ok(());
                }
                self.mutation.storage.remove_dir(source.path()).await?;
            } else {
                self.mutation.storage.remove_file(source.path()).await?;
            }
            Self::processed(Some(record));
            Ok(())
        })
    }

    fn create(&self, library: &str, kind: models::OperationKind) -> Arc<Mutex<OperationRecord>> {
        self.reap();
        let id = format!("op_{}", self.ids.lock().create());
        let record = Arc::new(Mutex::new(OperationRecord {
            model: models::Operation::new(id.clone(), kind, models::OperationStatus::Pending, library.to_string(), 0, 0, Vec::new(), Utc::now()),
            cancel: CancellationToken::new(),
            changes: Arc::new(Mutex::new(ListingInvalidation::default())),
        }));
        self.operations.lock().insert(id, record.clone());
        record
    }

    fn record(&self, id: &str) -> Result<Arc<Mutex<OperationRecord>>> {
        self.operations
            .lock()
            .get(id)
            .cloned()
            .ok_or_else(|| Error::new(ErrorKind::NotFound).with_message("operation not found"))
    }

    fn check_cancel(record: Option<&Arc<Mutex<OperationRecord>>>) -> Result<()> {
        if record.is_some_and(|record| record.lock().cancel.is_cancelled()) {
            return Err(Error::new(ErrorKind::Cancelled));
        }
        Ok(())
    }

    fn processed(record: Option<&Arc<Mutex<OperationRecord>>>) {
        if let Some(record) = record {
            record.lock().model.processed_count += 1;
        }
    }
    fn skipped(record: Option<&Arc<Mutex<OperationRecord>>>) {
        if let Some(record) = record {
            record.lock().model.skipped_count += 1;
        }
    }

    fn failure(record: &Arc<Mutex<OperationRecord>>, path: &str, error: Error) {
        if *error.kind() == ErrorKind::Cancelled {
            return;
        }
        let api = ApiError::from(error);
        let mut failure = models::OperationFailure::new(path.to_string(), api.body.kind);
        failure.message = api.body.message;
        record.lock().model.failed_entries.push(failure);
    }

    /// 一覧とタイムラインへ変更を反映してから、終わった状態にする。
    async fn finish(&self, record: &Arc<Mutex<OperationRecord>>) {
        let changes = record.lock().changes.lock().clone();
        self.mutation.apply_changes(&changes).await;
        let mut record = record.lock();
        record.model.status = if record.cancel.is_cancelled() {
            models::OperationStatus::Cancelled
        } else if record.model.failed_entries.is_empty() {
            models::OperationStatus::Succeeded
        } else {
            models::OperationStatus::Failed
        };
        record.model.finished_at = Some(Nullable::Present(Utc::now()));
    }

    fn reap(&self) {
        let before = Utc::now() - Duration::hours(1);
        self.operations.lock().retain(|_, record| match &record.lock().model.finished_at {
            Some(Nullable::Present(time)) => *time > before,
            _ => true,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testresult::TestResult;

    async fn completed(state: &crate::state::DaemonState, id: &str) -> Result<models::Operation> {
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                let operation = state.operations.get(id)?;
                if !matches!(operation.status, models::OperationStatus::Pending | models::OperationStatus::Running) {
                    return Ok(operation);
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .map_err(|error| Error::from_error(error, ErrorKind::Timeout))?
    }

    #[tokio::test]
    async fn duplicate_in_same_folder_preserves_source_and_existing_candidates_test() -> TestResult {
        let (_dir, state) = crate::service::mutation::tests::state().await?;
        let root = &state.conf.libraries[0].path;
        std::fs::write(root.join("source.txt"), b"source")?;
        std::fs::write(root.join("source (1).txt"), b"existing")?;
        std::fs::create_dir(root.join("folder"))?;
        std::fs::create_dir(root.join("folder (1)"))?;
        std::fs::write(root.join("folder/nested.txt"), b"nested")?;
        std::fs::write(root.join("folder (1)/keep.txt"), b"keep")?;
        for (path, folder) in [("/source.txt", false), ("/folder", true)] {
            let request = serde_json::from_value(serde_json::json!({"srcPaths":[path],"dstFolderPath":"/","conflict":"rename"}))?;
            let started = state.operations.copy("test", &request, folder).await?;
            let operation = completed(&state, &started.id).await?;
            assert_eq!(operation.status, models::OperationStatus::Succeeded);
            assert!(operation.failed_entries.is_empty());
            let results = operation.result_entries.ok_or("missing result entries")?;
            assert_eq!(results.len(), 1);
            assert_ne!(results[0].path, path);
            let published = root.join(results[0].path.trim_start_matches('/'));
            assert_eq!(
                std::fs::read(if folder { published.join("nested.txt") } else { published })?,
                if folder { b"nested".as_slice() } else { b"source".as_slice() }
            );
        }
        assert_eq!(std::fs::read(root.join("source.txt"))?, b"source");
        assert_eq!(std::fs::read(root.join("source (1).txt"))?, b"existing");
        assert_eq!(std::fs::read(root.join("folder/nested.txt"))?, b"nested");
        assert_eq!(std::fs::read(root.join("folder (1)/keep.txt"))?, b"keep");
        Ok(())
    }

    #[tokio::test]
    async fn duplicate_does_not_allow_overwrite_move_or_descendants_test() -> TestResult {
        let (_dir, state) = crate::service::mutation::tests::state().await?;
        let root = &state.conf.libraries[0].path;
        std::fs::write(root.join("source.txt"), b"source")?;
        std::fs::create_dir(root.join("folder"))?;
        std::fs::create_dir(root.join("folder/child"))?;
        for folder in [false, true] {
            let source = if folder { "/folder" } else { "/source.txt" };
            for conflict in ["fail", "skip", "overwrite"] {
                let request = serde_json::from_value(serde_json::json!({"srcPaths":[source],"dstFolderPath":"/","conflict":conflict}))?;
                assert_eq!(state.operations.copy("test", &request, folder).await.unwrap_err().kind(), &ErrorKind::InvalidInput);
            }
            let request = serde_json::from_value(serde_json::json!({"srcPaths":[source],"dstFolderPath":"/","conflict":"rename"}))?;
            assert!(state.operations.move_entries("test", &request, folder).await.is_err());
        }
        for destination in ["/folder", "/folder/child"] {
            let request = serde_json::from_value(serde_json::json!({"srcPaths":["/folder"],"dstFolderPath":destination,"conflict":"rename"}))?;
            assert_eq!(state.operations.copy("test", &request, true).await.unwrap_err().kind(), &ErrorKind::InvalidInput);
        }
        assert_eq!(std::fs::read(root.join("source.txt"))?, b"source");
        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn duplicate_rejects_alias_parent_inside_source_test() -> TestResult {
        let (_dir, state) = crate::service::mutation::tests::state().await?;
        let root = &state.conf.libraries[0].path;
        std::fs::create_dir(root.join("folder"))?;
        std::os::unix::fs::symlink(root.join("folder"), root.join("folder/alias"))?;
        let request = serde_json::from_value(serde_json::json!({"srcPaths":["/folder"],"dstPath":"/folder/alias","conflict":"rename"}))?;
        assert_eq!(state.operations.copy("test", &request, true).await.unwrap_err().kind(), &ErrorKind::InvalidInput);
        assert!(!root.join("folder/alias (1)").exists());
        Ok(())
    }

    #[tokio::test]
    async fn running_cancel_keeps_completed_entries_and_discards_partial_file_test() -> TestResult {
        let (_dir, state) = crate::service::mutation::tests::state().await?;
        let root = &state.conf.libraries[0].path;
        std::fs::create_dir(root.join("source"))?;
        std::fs::write(root.join("source/000-done.txt"), b"done")?;
        std::fs::File::create(root.join("source/999-large.bin"))?.set_len(128 * 1024 * 1024)?;
        let request: models::EntryTransferRequest = serde_json::from_value(serde_json::json!({"srcPaths":["/source"],"dstPath":"/destination"}))?;
        let operation = state.operations.copy("test", &request, true).await?;
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            while state.operations.get(&operation.id)?.processed_count < 2 {
                tokio::task::yield_now().await;
            }
            Ok::<_, Error>(())
        })
        .await??;
        state.operations.cancel(&operation.id)?;
        let completed = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                let operation = state.operations.get(&operation.id)?;
                if operation.status == models::OperationStatus::Cancelled {
                    return Ok::<_, Error>(operation);
                }
                tokio::task::yield_now().await;
            }
        })
        .await??;
        assert_eq!(completed.processed_count, 2);
        assert_eq!(std::fs::read(root.join("destination/000-done.txt"))?, b"done");
        assert!(!root.join("destination/999-large.bin").exists());
        assert!(root.join("source/999-large.bin").exists());
        assert_eq!(std::fs::read_dir(root.join("destination"))?.count(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn pending_cancel_preserves_source_and_completed_retention_test() -> TestResult {
        let (_dir, state) = crate::service::mutation::tests::state().await?;
        let source = state.conf.libraries[0].path.join("source.txt");
        std::fs::write(&source, b"source")?;
        let permits = state.operations.permits.clone().acquire_many_owned(4).await?;
        let request: models::EntryTransferRequest = serde_json::from_value(serde_json::json!({"srcPaths":["/source.txt"],"dstPath":"/destination.txt"}))?;
        let operation = state.operations.copy("test", &request, false).await?;
        assert_eq!(operation.status, models::OperationStatus::Pending);
        let cancelled = state.operations.cancel(&operation.id)?;
        assert_eq!(cancelled.status, models::OperationStatus::Cancelled);
        drop(permits);
        // 取り消された操作の終了処理（一覧とタイムラインへの反映を含む）が終わるまで待つ。
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        assert!(source.exists());
        assert!(!state.conf.libraries[0].path.join("destination.txt").exists());
        assert_eq!(state.operations.cancel(&operation.id)?.status, models::OperationStatus::Cancelled);
        state.operations.record(&operation.id)?.lock().model.finished_at = Some(Nullable::Present(Utc::now() - Duration::hours(2)));
        assert_eq!(state.operations.get(&operation.id).unwrap_err().kind(), &ErrorKind::NotFound);
        Ok(())
    }
}
