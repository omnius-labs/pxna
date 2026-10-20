mod probe;
pub use probe::*;
use std::{path::Path, pin::Pin, sync::Arc};

use futures::{Stream, StreamExt as _};
use tokio::io::AsyncReadExt as _;
use tokio_util::bytes::Bytes;

use crate::{
    core::nfs::{FileStorageDriver, FileStorageLocal, is_cross_device},
    prelude::*,
};

// storage の primitive は engine の公開 API に現れるため、service 層から名前を出す。
pub use crate::core::nfs::{
    ArchiveBlockGroup, ArchiveIdentity, ArchiveOptions, ArchiveRuntime, CACHE_FORMAT_VERSION, ConflictPolicy, EntryCacheKey, ExtractionCache, FileStorageArchive,
    FileStorageLocal as LocalStorageDriver, MaterializedFile, OpenedFile, Publish, SourceFingerprint, StagingFile, ZipLegacyEncoding,
};

pub use crate::core::work::{WorkBudget, WorkClass, WorkKind};

const COPY_CHUNK_BYTES: usize = 64 * 1024;

/// 一度解決した namespace と entry の位置。archive の driver と lease を要求の間保持する。
#[derive(Clone)]
pub struct ResolvedStoragePath {
    driver: Arc<dyn FileStorageDriver>,
    path: NestedPathBuf,
    inner: String,
    archive_identity: Option<ArchiveIdentity>,
}

impl ResolvedStoragePath {
    pub fn path(&self) -> &NestedPath {
        &self.path
    }

    /// 解決済みの archive namespace の identity。local の path では持たない。
    pub fn archive_identity(&self) -> Option<ArchiveIdentity> {
        self.archive_identity
    }

    /// 同じ namespace の別の entry を指す。archive 境界は変更しない。
    pub fn at(&self, inner: &str) -> Result<Self> {
        let path = NestedPathBuf::compose(&self.path.outer_segments(), inner)?;
        let inner = path.iter().last().unwrap_or_default().to_string();
        Ok(Self {
            driver: self.driver.clone(),
            path,
            inner,
            archive_identity: self.archive_identity,
        })
    }

    pub fn entry_cache_key(&self) -> Option<EntryCacheKey> {
        self.archive_identity.map(|identity| EntryCacheKey::new(&identity, &self.inner))
    }

    pub async fn get_entry(&self) -> Result<NestedFsEntry> {
        let entry = self.driver.get_entry(&self.inner).await?;
        Ok(NestedFsEntry {
            path: NestedPathBuf::compose(&self.path.outer_segments(), &entry.path)?,
            meta: entry.meta,
            detail: entry.detail,
        })
    }

    pub async fn get_listing_entry(&self) -> Result<NestedFsEntry> {
        let entry = self.driver.get_listing_entry(&self.inner).await?;
        Ok(NestedFsEntry {
            path: NestedPathBuf::compose(&self.path.outer_segments(), &entry.path)?,
            meta: entry.meta,
            detail: entry.detail,
        })
    }

    pub async fn find_entry_summaries(&self) -> Result<NestedFsEntrySummaryStream> {
        let outer = self.path.outer_segments();
        let stream = self.driver.find_entry_summaries(&self.inner).await?;
        Ok(Box::pin(stream.map(move |entry| {
            let entry = entry?;
            Ok(NestedFsEntrySummary {
                path: NestedPathBuf::compose(&outer, &entry.path)?,
                kind: entry.kind,
                archive: entry.archive,
            })
        })))
    }

    pub async fn find_entries(&self, recursive: bool) -> Result<NestedFsEntryStream> {
        let outer = self.path.outer_segments();
        let stream = self.driver.find_entries(&self.inner, recursive).await?;
        Ok(Box::pin(stream.map(move |entry| {
            let entry = entry?;
            Ok(NestedFsEntry {
                path: NestedPathBuf::compose(&outer, &entry.path)?,
                meta: entry.meta,
                detail: entry.detail,
            })
        })))
    }

    pub async fn materialize_file(&self) -> Result<MaterializedFile> {
        self.driver.materialize_file(&self.inner).await
    }
    pub async fn open_file(&self) -> Result<OpenedFile> {
        self.driver.open_file(&self.inner).await
    }
}

/// `NestedPath` を解決した結果として上位層へ返す entry。
///
/// driver が返す `FsEntry` の `path` は bind 済み namespace 内の path であり、そのままでは
/// 外側の segment を持たない。両者を型で分けることで、driver 内 path を上位層へ漏らす取り違えを
/// コンパイル時に止める。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedFsEntry {
    pub path: NestedPathBuf,
    pub meta: FsMetadata,
    pub detail: FsEntryDetail,
}

/// 解決済み namespace の軽量な列挙結果。属性の有無を FsMetadata の仮値で表さない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedFsEntrySummary {
    pub path: NestedPathBuf,
    pub kind: FsEntryKind,
    pub archive: bool,
}

pub type NestedFsEntrySummaryStream = Pin<Box<dyn Stream<Item = Result<NestedFsEntrySummary>> + Send + 'static>>;

pub type NestedFsEntryStream = Pin<Box<dyn Stream<Item = Result<NestedFsEntry>> + Send + 'static>>;

/// engine 内で `NestedPath` を知る唯一の storage component。
///
/// 外側の local path から内側へ順番に driver を構築し、最終 segment を driver-local な `&str`
/// として渡す。ツリー走査、entry ごとの conflict 適用、進捗報告、キャンセルは持たない。
pub struct StorageService {
    local: Arc<FileStorageLocal>,
    archive: Arc<ArchiveRuntime>,
    browse_generation: std::sync::atomic::AtomicU64,
}

impl StorageService {
    pub fn work_budget(&self) -> &Arc<WorkBudget> {
        self.local.work_budget()
    }
    pub fn new(local: Arc<FileStorageLocal>, archive: Arc<ArchiveRuntime>) -> Self {
        Self {
            local,
            archive,
            browse_generation: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// 自分の変更が、古い代表選択を再公開しないための世代。
    pub fn browse_generation(&self) -> u64 {
        self.browse_generation.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn invalidate_browse_caches(&self) {
        self.browse_generation.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    // ---- 読み取り ----

    pub async fn get_entry(&self, path: &NestedPath) -> Result<NestedFsEntry> {
        self.resolve_entry(path).await?.get_entry().await
    }

    pub async fn find_entries(&self, path: &NestedPath, recursive: bool) -> Result<NestedFsEntryStream> {
        self.resolve_entry(path).await?.find_entries(recursive).await
    }

    pub async fn exists_file(&self, path: &NestedPath) -> Result<bool> {
        let ResolvedStoragePath { driver, inner, .. } = self.resolve_entry(path).await?;
        driver.exists_file(&inner).await
    }

    pub async fn exists_dir(&self, path: &NestedPath) -> Result<bool> {
        let ResolvedStoragePath { driver, inner, .. } = self.resolve_entry(path).await?;
        driver.exists_dir(&inner).await
    }

    /// archive 内の entry を指す `EntryCacheKey` を返す。archive を含まない local の path には `None` を返す。
    ///
    /// archive は開かず、最外の local file の fingerprint と、入れ子の archive の path だけから導く。
    /// 最外の file が変われば、内側のすべての entry のキーが変わる（[`ArchiveIdentity`] の定義）。
    pub async fn entry_cache_key(&self, path: &NestedPath) -> Result<Option<EntryCacheKey>> {
        let segments = path.iter().collect::<Vec<_>>();
        let Some((last, outer)) = segments.split_last() else {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("empty nested path"));
        };
        let Some((local_path, inner_archives)) = outer.split_first() else {
            return Ok(None);
        };

        let fingerprint = SourceFingerprint::read(Path::new(local_path)).await?;
        let mut identity = ArchiveIdentity::from_local(local_path, &fingerprint);
        for inner in inner_archives {
            identity = ArchiveIdentity::from_parent(&identity, inner);
        }
        Ok(Some(EntryCacheKey::new(&identity, last)))
    }

    pub async fn materialize_file(&self, path: &NestedPath) -> Result<MaterializedFile> {
        let ResolvedStoragePath { driver, inner, .. } = self.resolve_entry(path).await?;
        driver.materialize_file(&inner).await
    }

    pub async fn open_file(&self, path: &NestedPath) -> Result<OpenedFile> {
        let ResolvedStoragePath { driver, inner, .. } = self.resolve_entry(path).await?;
        driver.open_file(&inner).await
    }

    pub async fn read_file(&self, path: &NestedPath, offset: u64, limit: u64) -> Result<Bytes> {
        let ResolvedStoragePath { driver, inner, .. } = self.resolve_entry(path).await?;
        driver.read_file(&inner, offset, limit).await
    }

    // ---- 書き込み ----

    pub async fn create_staging(&self, dst_dir: &NestedPath) -> Result<StagingFile> {
        let (driver, inner) = self.resolve_writable(dst_dir)?;
        driver.create_staging(inner).await
    }

    pub async fn make_dir(&self, path: &NestedPath) -> Result<()> {
        let (driver, inner) = self.resolve_writable(path)?;
        driver.make_dir(inner).await
    }

    pub async fn remove_file(&self, path: &NestedPath) -> Result<()> {
        let (driver, inner) = self.resolve_writable(path)?;
        driver.remove_file(inner).await
    }

    pub async fn remove_dir(&self, path: &NestedPath) -> Result<()> {
        let (driver, inner) = self.resolve_writable(path)?;
        driver.remove_dir(inner).await
    }

    pub async fn rename(&self, src: &NestedPath, dst: &NestedPath, conflict: ConflictPolicy) -> Result<Publish> {
        let (driver, src_inner) = self.resolve_writable(src)?;
        let (_, dst_inner) = self.resolve_writable(dst)?;
        driver.rename(src_inner, dst_inner, conflict).await
    }

    /// destination directory に残った孤児 staging を回収する。
    ///
    /// 1 file ごとではなく、Operation が directory ごとに 1 回だけ呼ぶ。
    pub async fn sweep_staging(&self, dir: &NestedPath) -> Result<u64> {
        let (_, inner) = self.resolve_writable(dir)?;
        self.local.sweep_staging(inner).await
    }

    // ---- 合成 ----

    /// 1 file の copy。
    ///
    /// source が local か archive かに関係なく、destination と同じ directory の staging へ
    /// chunk copy し、内容と `modified` を設定してから publish する。
    pub async fn copy_file(&self, src: &NestedPath, dst: &NestedPath, conflict: ConflictPolicy) -> Result<Publish> {
        let ResolvedStoragePath {
            driver: src_driver,
            inner: src_inner,
            ..
        } = self.resolve_entry(src).await?;
        let (dst_driver, dst_inner) = self.resolve_writable(dst)?;
        let dst_dir = dst
            .parent()
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput).with_message(format!("destination has no parent directory: {dst}")))?;

        let modified = src_driver.get_entry(&src_inner).await?.meta.modified;
        let mut opened = src_driver.open_file(&src_inner).await?;

        let mut staging = dst_driver.create_staging(&dst_dir).await?;
        let mut buf = vec![0u8; COPY_CHUNK_BYTES];
        loop {
            let read = opened.read(&mut buf).await?;
            if read == 0 {
                break;
            }
            staging.write_all(&buf[..read]).await?;
        }

        if let Some(modified) = modified {
            staging.set_modified(modified).await?;
        }

        // 失敗、timeout、cancellation では staging が drop で削除される。
        staging.publish(dst_inner, conflict).await
    }

    /// 1 file の move。
    ///
    /// 同一 driver 内ではまず OS rename を試し、別 filesystem をまたぐ場合だけ copy 完了後に
    /// source を削除する方式へ退避する。OS rename が成功した場合だけ atomic move を保証する。
    pub async fn move_file(&self, src: &NestedPath, dst: &NestedPath, conflict: ConflictPolicy) -> Result<Publish> {
        let (src_driver, src_inner) = self.resolve_writable(src)?;
        let (_, dst_inner) = self.resolve_writable(dst)?;

        match src_driver.rename(src_inner, dst_inner, conflict).await {
            Ok(published) => Ok(published),
            Err(e) if is_cross_device(&e) => {
                let published = self.copy_file(src, dst, conflict).await?;

                // copy 成功後の source 削除に失敗した場合は両方を残して error にし、
                // destination の rollback は行わない。
                if matches!(published, Publish::Published { .. }) {
                    src_driver.remove_file(src_inner).await?;
                }

                Ok(published)
            }
            Err(e) => Err(e),
        }
    }

    // ---- 解決 ----

    /// `NestedPath` を driver と driver-local な path へ落とす。
    ///
    /// 外側の segment から順に archive を開く。最外の identity は local file の path と fingerprint から、
    /// 内側の identity は親の identity と内側 path から導く。中間 archive の `MaterializedFile` は
    /// 次の driver が保持するため、利用中に消えない。
    pub async fn resolve_entry(&self, path: &NestedPath) -> Result<ResolvedStoragePath> {
        let segments = path.iter().collect::<Vec<_>>();
        let Some((last, outer)) = segments.split_last() else {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("empty nested path"));
        };
        let Some((local_path, inner_archives)) = outer.split_first() else {
            return Ok(ResolvedStoragePath {
                driver: self.local.clone(),
                path: path.to_nested_path_buf(),
                inner: last.to_string(),
                archive_identity: None,
            });
        };

        let max_nested_depth = self.archive.options().max_nested_depth;
        if outer.len() > max_nested_depth {
            return Err(Error::new(ErrorKind::LimitExceeded).with_message(format!("archive nesting exceeds {max_nested_depth}: {path}")));
        }

        let mut driver = self.open_archive(local_path).await?;
        let mut identity = driver.identity();
        for inner in inner_archives {
            let source = driver.materialize_file(inner).await?;
            identity = ArchiveIdentity::from_parent(&identity, inner);
            driver = FileStorageArchive::open(source, inner, identity, self.archive.clone())
                .await
                .map_err(|error| Self::archive_open_error(error, inner))?;
        }

        Ok(ResolvedStoragePath {
            driver: Arc::new(driver),
            path: path.to_nested_path_buf(),
            inner: last.to_string(),
            archive_identity: Some(identity),
        })
    }

    /// 最外の local の archive を開く。返す driver は、extract の前に source の fingerprint を照合する。
    pub async fn open_archive(&self, local_path: &str) -> Result<FileStorageArchive> {
        let source_entry = self.local.get_entry(local_path).await?;
        if !source_entry.is_file() {
            return Err(Error::new(ErrorKind::UnsupportedType).with_message(format!("not an archive file: {local_path}")));
        }
        let fingerprint = SourceFingerprint::read(Path::new(local_path)).await?;
        let identity = ArchiveIdentity::from_local(local_path, &fingerprint);
        let source = self.local.materialize_file(local_path).await?;
        Ok(FileStorageArchive::open(source, local_path, identity, self.archive.clone())
            .await
            .map_err(|error| Self::archive_open_error(error, local_path))?
            .verify_source(fingerprint))
    }

    pub fn archive_options(&self) -> &ArchiveOptions {
        self.archive.options()
    }

    /// archive として開けなかった segment を message に含める。
    fn archive_open_error(error: Error, segment: &str) -> Error {
        let message = format!("cannot open as archive: {segment}: {}", error.message().unwrap_or_default());
        let kind = error.kind().clone();
        Error::from_error(error, kind).with_message(message)
    }

    /// 変更系のための解決。archive の内側は書き込めない。
    fn resolve_writable<'a>(&'a self, path: &'a NestedPath) -> Result<(&'a dyn FileStorageDriver, &'a str)> {
        if path.rank() != 0 {
            return Err(Error::new(ErrorKind::ReadOnly).with_message(format!("path is inside an archive: {path}")));
        }
        Ok((self.local.as_ref(), path.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use omnius_core_base::{
        clock::ClockUtc,
        tsid::{TsidProvider, TsidProviderImpl},
    };
    use rand::{SeedableRng as _, rngs::ChaCha20Rng};
    use testresult::TestResult;

    use super::*;

    fn service() -> StorageService {
        let tsid_provider: Arc<Mutex<dyn TsidProvider + Send + Sync>> = Arc::new(Mutex::new(TsidProviderImpl::new(ClockUtc, ChaCha20Rng::seed_from_u64(0), 16)));
        StorageService::new(Arc::new(FileStorageLocal::new(tsid_provider)), Arc::new(ArchiveRuntime::default()))
    }

    fn nested(path: &Path) -> NestedPathBuf {
        NestedPathBuf::try_from(path).unwrap()
    }

    #[tokio::test]
    async fn get_entry_returns_nested_path_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("a.txt");
        tokio::fs::write(&file, b"hello").await?;

        let service = service();
        let entry = service.get_entry(&nested(&file)).await?;
        assert_eq!(entry.path, nested(&file));
        assert_eq!(entry.meta.len, 5);

        Ok(())
    }

    #[tokio::test]
    async fn copy_file_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let src = dir.path().join("a.txt");
        let dst = dir.path().join("b.txt");
        tokio::fs::write(&src, b"hello").await?;

        let service = service();
        let published = service.copy_file(&nested(&src), &nested(&dst), ConflictPolicy::Fail).await?;
        assert_eq!(
            published,
            Publish::Published {
                dst: dst.to_str().unwrap().to_string()
            }
        );
        assert_eq!(tokio::fs::read(&dst).await?, b"hello");
        assert!(src.exists(), "copy では source を残す");

        Ok(())
    }

    #[tokio::test]
    async fn copy_file_applies_conflict_policy_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let src = dir.path().join("a.txt");
        let dst = dir.path().join("b.txt");
        tokio::fs::write(&src, b"hello").await?;
        tokio::fs::write(&dst, b"existing").await?;

        let service = service();

        let e = service.copy_file(&nested(&src), &nested(&dst), ConflictPolicy::Fail).await.unwrap_err();
        assert_eq!(*e.kind(), ErrorKind::AlreadyExists);
        assert_eq!(tokio::fs::read(&dst).await?, b"existing");

        assert_eq!(service.copy_file(&nested(&src), &nested(&dst), ConflictPolicy::Skip).await?, Publish::Skipped);
        assert_eq!(tokio::fs::read(&dst).await?, b"existing");

        service.copy_file(&nested(&src), &nested(&dst), ConflictPolicy::Overwrite).await?;
        assert_eq!(tokio::fs::read(&dst).await?, b"hello");

        Ok(())
    }

    #[tokio::test]
    async fn move_file_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let src = dir.path().join("a.txt");
        let dst = dir.path().join("b.txt");
        tokio::fs::write(&src, b"hello").await?;

        let service = service();
        let published = service.move_file(&nested(&src), &nested(&dst), ConflictPolicy::Fail).await?;
        assert_eq!(
            published,
            Publish::Published {
                dst: dst.to_str().unwrap().to_string()
            }
        );
        assert!(!src.exists(), "move では source を残さない");
        assert_eq!(tokio::fs::read(&dst).await?, b"hello");

        Ok(())
    }

    #[tokio::test]
    async fn nested_path_is_rejected_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let mut path = nested(&dir.path().join("a.zip"));
        path.push_str("inner.txt")?;

        let service = service();

        let e = service.get_entry(&path).await.unwrap_err();
        assert_eq!(*e.kind(), ErrorKind::NotFound, "存在しない archive の内側は NotFound");

        let e = service.make_dir(&path).await.unwrap_err();
        assert_eq!(*e.kind(), ErrorKind::ReadOnly, "archive の内側への変更は ReadOnly");

        Ok(())
    }

    fn write_7z(path: &Path, entries: &[(&str, &[u8])]) -> TestResult {
        let mut writer = sevenz_rust2::ArchiveWriter::create(path)?;
        for (name, content) in entries {
            writer.push_archive_entry(sevenz_rust2::ArchiveEntry::new_file(name), Some(std::io::Cursor::new(content.to_vec())))?;
        }
        writer.finish()?;
        Ok(())
    }

    async fn cached_service(cache_dir: &Path) -> TestResult<(StorageService, Arc<ArchiveRuntime>)> {
        let tsid_provider: Arc<Mutex<dyn TsidProvider + Send + Sync>> = Arc::new(Mutex::new(TsidProviderImpl::new(ClockUtc, ChaCha20Rng::seed_from_u64(0), 16)));
        let runtime = Arc::new(ArchiveRuntime::open(ArchiveOptions::default(), cache_dir).await?);
        Ok((StorageService::new(Arc::new(FileStorageLocal::new(tsid_provider)), runtime.clone()), runtime))
    }

    #[tokio::test]
    async fn resolved_namespace_reuses_driver_and_preserves_paths_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let archive = dir.path().join("a.7z");
        write_7z(&archive, &[("x.txt", b"one"), ("dir/y.txt", b"two")])?;
        let service = service();
        let mut root = nested(&archive);
        root.push_archive_root()?;
        let namespace = service.resolve_entry(&root).await?;
        let x = namespace.at("x.txt")?;
        let y = namespace.at("dir/y.txt")?;
        assert!(Arc::ptr_eq(&x.driver, &y.driver));
        let x_entry = x.get_entry().await?;
        assert_eq!(x_entry.path, x.path);
        assert_eq!(x.entry_cache_key(), service.entry_cache_key(x.path()).await?);
        assert_ne!(x.entry_cache_key(), y.entry_cache_key());
        let mut file = y.open_file().await?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).await?;
        assert_eq!(bytes, b"two");
        let mut entries = namespace.find_entries(true).await?;
        while let Some(entry) = entries.next().await {
            assert_eq!(entry?.path.outer_segments(), root.outer_segments());
        }
        Ok(())
    }

    #[tokio::test]
    async fn nested_archive_reuses_inner_cache_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let inner = dir.path().join("inner.7z");
        write_7z(&inner, &[("x.png", b"pixel")])?;
        let outer = dir.path().join("outer.7z");
        let inner_bytes = std::fs::read(&inner)?;
        write_7z(&outer, &[("inner.7z", &inner_bytes)])?;

        let cache_dir = dir.path().join("cache");
        let (service, runtime) = cached_service(&cache_dir).await?;
        let mut path = nested(&outer);
        path.push_str("inner.7z")?;
        path.push_str("x.png")?;

        let materialized = service.materialize_file(&path).await?;
        assert_eq!(std::fs::read(materialized.path())?, b"pixel");
        assert_eq!(runtime.decoded_blocks(), 2);
        drop(materialized);

        // 外側の archive から取り出した inner.7z の cache を消し、中間 file の世代を変える。
        let fingerprint = SourceFingerprint::read(&outer).await?;
        let outer_identity = ArchiveIdentity::from_local(outer.to_str().unwrap(), &fingerprint);
        let key = EntryCacheKey::new(&outer_identity, "inner.7z").to_hex();
        std::fs::remove_file(cache_dir.join("files").join(&key[..2]).join(&key))?;

        let materialized = service.materialize_file(&path).await?;
        assert_eq!(std::fs::read(materialized.path())?, b"pixel");
        assert_eq!(runtime.decoded_blocks(), 3, "inner.7z だけを取り出し直し、x.png は cache から返す");
        Ok(())
    }

    #[tokio::test]
    async fn nested_path_lists_archive_root_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let archive = dir.path().join("a.7z");
        write_7z(&archive, &[("dir/x.txt", b"x"), ("y.txt", b"y")])?;
        let (service, _runtime) = cached_service(&dir.path().join("cache")).await?;

        let mut root = nested(&archive);
        root.push_archive_root()?;
        let entries = service.find_entries(&root, false).await?.collect::<Vec<_>>().await;
        let paths = entries
            .into_iter()
            .map(|entry| entry.map(|entry| entry.path.iter().last().unwrap_or_default().to_string()))
            .collect::<Result<Vec<_>>>()?;
        assert_eq!(paths, ["dir", "y.txt"]);
        Ok(())
    }

    #[tokio::test]
    async fn nested_depth_limit_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let (service, _runtime) = cached_service(&dir.path().join("cache")).await?;
        let mut path = nested(&dir.path().join("a.7z"));
        for _ in 0..=ArchiveOptions::default().max_nested_depth {
            path.push_str("inner.7z")?;
        }
        let e = service.get_entry(&path).await.unwrap_err();
        assert_eq!(*e.kind(), ErrorKind::LimitExceeded);
        Ok(())
    }

    #[tokio::test]
    async fn not_an_archive_is_unsupported_type_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("notes.txt");
        tokio::fs::write(&file, b"plain text").await?;
        let (service, _runtime) = cached_service(&dir.path().join("cache")).await?;

        let mut path = nested(&file);
        path.push_str("x")?;
        let e = service.get_entry(&path).await.unwrap_err();
        assert_eq!(*e.kind(), ErrorKind::UnsupportedType);
        assert!(e.message().unwrap_or_default().contains("notes.txt"), "開けなかった segment を message に含める");
        Ok(())
    }

    #[tokio::test]
    async fn entry_cache_key_follows_the_source_and_inner_path_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let archive = dir.path().join("a.7z");
        write_7z(&archive, &[("x.png", b"one"), ("y.png", b"two")])?;
        let (service, _runtime) = cached_service(&dir.path().join("cache")).await?;

        let local = nested(&archive);
        assert_eq!(service.entry_cache_key(&local).await?, None, "local の path にはキーがない");

        let mut x = nested(&archive);
        x.push_str("x.png")?;
        let mut y = nested(&archive);
        y.push_str("y.png")?;
        let key_x = service.entry_cache_key(&x).await?.ok_or("x")?;
        assert_eq!(service.entry_cache_key(&x).await?, Some(key_x.clone()), "同じ entry には同じキー");
        assert_ne!(service.entry_cache_key(&y).await?, Some(key_x.clone()), "inner path が違えばキーも違う");

        // 最外の file の size が変われば、内側のキーも変わる。
        write_7z(&archive, &[("x.png", b"one and more"), ("y.png", b"two")])?;
        assert_ne!(service.entry_cache_key(&x).await?, Some(key_x));
        Ok(())
    }
}
