use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use tokio::{io::AsyncReadExt as _, sync::Semaphore};

use crate::{
    core::{
        nfs::{FileStorageDriver, FsEntryStream, MaterializedFile, StagingFile},
        schedule::Scheduler,
    },
    prelude::*,
};

mod cache;
mod extractor;
pub(crate) mod format;
mod identity;
mod index;
mod index_cache;
mod options;
mod path;
mod sevenz;
mod zip;

pub use cache::ExtractionCache;
pub use identity::{ArchiveIdentity, CACHE_FORMAT_VERSION, EntryCacheKey, SourceFingerprint};
pub use options::{ArchiveOptions, ArchiveRuntime, ZipLegacyEncoding};

use extractor::ArchiveExtractor;
use format::ArchiveFormat;
use index::{ArchiveBlockId, ArchiveIndex};
use index_cache::EstimatedSize;

#[derive(Clone)]
enum ArchiveReader {
    Zip(zip::ZipArchive),
    SevenZip(sevenz::SevenZipArchive),
}

/// inspect で得た検証済み index と、entry を取り出すための reader の組。
///
/// 同じ `ArchiveIdentity` の archive を開く driver 間で共有する。
pub(super) struct InspectedArchive {
    reader: ArchiveReader,
    index: ArchiveIndex,
}

impl InspectedArchive {
    async fn inspect(path: PathBuf, name: String, options: ArchiveOptions, budget: Arc<crate::core::work::WorkBudget>) -> Result<Self> {
        let background = budget.background_if_needed(crate::core::work::WorkKind::Archive).await?;
        let mut file = tokio::fs::File::open(&path).await?;
        let mut head = [0_u8; 6];
        let read = file.read(&mut head).await?;
        let format = ArchiveFormat::detect(&head[..read])?;
        format.validate_extension(Path::new(&name))?;

        match format {
            ArchiveFormat::Zip => {
                let (reader, index) = zip::ZipArchive::inspect(&path, &options).await?;
                Ok(Self {
                    reader: ArchiveReader::Zip(reader),
                    index,
                })
            }
            ArchiveFormat::SevenZip => {
                let (reader, index) = Scheduler::run_blocking_with(background, move || sevenz::SevenZipArchive::inspect(&path, &options))
                    .await
                    .map_err(|error| error.with_message("join failed"))??;
                Ok(Self {
                    reader: ArchiveReader::SevenZip(reader),
                    index,
                })
            }
        }
    }
}

/// 探査は既存の検証済み索引だけを保持する。新規 inspect や materialize を行わない。
pub struct CachedArchiveFolder {
    inspected: Arc<InspectedArchive>,
    identity: ArchiveIdentity,
    path: String,
}
impl CachedArchiveFolder {
    pub fn identity(&self) -> ArchiveIdentity {
        self.identity
    }
    pub fn has_children(&self, include_hidden: bool, alive: impl Fn() -> bool) -> Option<bool> {
        let mut entries = self.inspected.index.descendants(&self.path, false);
        for _ in 0..128 {
            if !alive() {
                return None;
            }
            let Some(entry) = entries.next() else {
                return Some(false);
            };
            let entry = &entry.fs_entry;
            if !include_hidden && entry.path.rsplit('/').next()?.starts_with('.') {
                continue;
            }
            if entry.is_dir() || matches!(entry.detail, FsEntryDetail::File { archive: true }) {
                return Some(true);
            }
        }
        // EOF の確認も対象上限に含め、128 件で打ち切った場合は unknown とする。
        None
    }
}
impl ArchiveRuntime {
    pub fn cached_folder(&self, mut identity: ArchiveIdentity, segments: &[String]) -> Option<CachedArchiveFolder> {
        let mut rest = segments;
        for _ in 0..self.options().max_nested_depth {
            let inspected = self.index_cache().peek(&identity)?;
            let path = rest.join("/");
            if inspected.index.get(&path).is_some_and(|entry| entry.fs_entry.is_dir()) {
                return Some(CachedArchiveFolder { inspected, identity, path });
            }
            let mut boundary = None;
            for end in 1..=rest.len() {
                let prefix = rest[..end].join("/");
                let entry = inspected.index.get(&prefix)?;
                if matches!(entry.fs_entry.detail, FsEntryDetail::File { archive: true }) {
                    boundary = Some((prefix, end));
                    break;
                }
            }
            let (prefix, end) = boundary?;
            identity = ArchiveIdentity::from_parent(&identity, &prefix);
            rest = &rest[end..];
        }
        None
    }
}

impl EstimatedSize for InspectedArchive {
    fn estimated_size(&self) -> u64 {
        // reader も entry 名を保持するため、index の 2 倍で見積もる。
        const FIXED_BYTES: u64 = 1024;
        let entry_bytes = self.index.estimated_size();
        FIXED_BYTES + entry_bytes * 2
    }
}

/// 1 回の展開で取り出せる file の組。ZIP では 1 つの file、solid 7z では同じ block の file である。
#[derive(Debug, Clone)]
pub struct ArchiveBlockGroup {
    block_id: Option<ArchiveBlockId>,
    paths: Vec<String>,
}

impl ArchiveBlockGroup {
    pub fn paths(&self) -> &[String] {
        &self.paths
    }

    /// archive の中の block の順。block を持たない空の file を先にする。
    fn order(&self) -> usize {
        match self.block_id {
            None => 0,
            Some(ArchiveBlockId::Zip(index) | ArchiveBlockId::SevenZip(index)) => index + 1,
        }
    }
}

pub struct FileStorageArchive {
    source: MaterializedFile,
    identity: ArchiveIdentity,
    source_check: Option<SourceFingerprint>,
    runtime: Arc<ArchiveRuntime>,
    inspected: Arc<InspectedArchive>,
}

impl FileStorageArchive {
    /// `identity` が同じ archive の inspect 結果は `runtime` の index cache から再利用する。
    ///
    /// `name` は拡張子と signature の照合に使う論理 path である。
    /// 内側の archive の実体は extraction cache 上の拡張子のない file なので、`source` の path では照合できない。
    pub async fn open(source: MaterializedFile, name: &str, identity: ArchiveIdentity, runtime: Arc<ArchiveRuntime>) -> Result<Self> {
        let path = source.path().to_path_buf();
        let name = name.to_string();
        let options = runtime.options().clone();
        let class = crate::core::work::WorkClass::current();
        let budget = runtime.work_budget().clone();
        let inspected = runtime
            .index_cache()
            .get_or_inspect(identity, move || class.scope(InspectedArchive::inspect(path, name, options, budget)))
            .await?;

        Ok(Self {
            source,
            identity,
            source_check: None,
            runtime,
            inspected,
        })
    }

    /// extract の前と cache への登録の前に、source が `fingerprint` から変わっていないことを確かめる。
    ///
    /// 最外の local file だけに使う。内側の archive の identity は親から導かれるため、照合は要らない。
    pub fn verify_source(mut self, fingerprint: SourceFingerprint) -> Self {
        self.source_check = Some(fingerprint);
        self
    }

    pub fn identity(&self) -> ArchiveIdentity {
        self.identity
    }

    /// 中のすべての file の entry。ディレクトリと symlink は含めない。
    pub fn files(&self) -> Vec<FsEntry> {
        self.inspected
            .index
            .descendants("", true)
            .filter(|entry| entry.fs_entry.is_file())
            .map(|entry| entry.fs_entry.clone())
            .collect()
    }

    /// `paths` の file を、1 回の展開で取り出せる組にまとめる。組は archive の中の block の順に並ぶ。
    pub fn block_groups(&self, paths: impl IntoIterator<Item = String>) -> Vec<ArchiveBlockGroup> {
        let mut groups: Vec<ArchiveBlockGroup> = Vec::new();
        let mut positions: HashMap<Option<ArchiveBlockId>, usize> = HashMap::new();
        for path in paths {
            let Some(entry) = self.inspected.index.get(&path) else {
                continue;
            };
            if !entry.fs_entry.is_file() {
                continue;
            }
            let position = *positions.entry(entry.block_id).or_insert_with(|| {
                groups.push(ArchiveBlockGroup {
                    block_id: entry.block_id,
                    paths: Vec::new(),
                });
                groups.len() - 1
            });
            groups[position].paths.push(path);
        }
        groups.sort_by_key(ArchiveBlockGroup::order);
        groups
    }

    /// 組の block を 1 回だけ展開し、組の file を一時 file として返す。
    ///
    /// extraction cache を引かず登録もせず、`permits` の上限で decoder を動かす。1 つの file の上限を超える file は返さない。
    pub async fn extract_group(&self, group: &ArchiveBlockGroup, permits: Arc<Semaphore>) -> Result<HashMap<String, MaterializedFile>> {
        ArchiveExtractor::extract_uncached(
            self.source.path().to_path_buf(),
            self.source_check,
            self.runtime.clone(),
            self.inspected.clone(),
            group.block_id,
            &group.paths,
            permits,
        )
        .await
    }

    /// 中の `path` の archive を、取り出した `source` から開く。identity は自身の identity と `path` から導く。
    pub async fn open_inner(&self, path: &str, source: MaterializedFile) -> Result<Self> {
        Self::open(source, path, ArchiveIdentity::from_parent(&self.identity, path), self.runtime.clone()).await
    }

    fn not_found(path: &str) -> Error {
        Error::new(ErrorKind::NotFound).with_message(format!("archive entry not found: {path}"))
    }

    fn read_only() -> Error {
        Error::new(ErrorKind::ReadOnly).with_message("archive storage is read-only")
    }
}

#[async_trait]
impl FileStorageDriver for FileStorageArchive {
    async fn get_entry(&self, path: &str) -> Result<FsEntry> {
        self.inspected.index.get(path).map(|entry| entry.fs_entry.clone()).ok_or_else(|| Self::not_found(path))
    }

    async fn find_entries(&self, path: &str, recursive: bool) -> Result<FsEntryStream> {
        let entry = self.inspected.index.get(path).ok_or_else(|| Self::not_found(path))?;
        if !entry.fs_entry.is_dir() {
            return Err(Error::new(ErrorKind::UnsupportedType).with_message(format!("not a directory: {path}")));
        }
        let entries = self.inspected.index.descendants(path, recursive).map(|entry| entry.fs_entry.clone()).collect::<Vec<_>>();
        Ok(Box::pin(futures::stream::iter(entries.into_iter().map(Ok))))
    }

    async fn exists_file(&self, path: &str) -> Result<bool> {
        Ok(self.inspected.index.get(path).is_some_and(|entry| entry.fs_entry.is_file()))
    }

    async fn exists_dir(&self, path: &str) -> Result<bool> {
        Ok(self.inspected.index.get(path).is_some_and(|entry| entry.fs_entry.is_dir()))
    }

    async fn materialize_file(&self, path: &str) -> Result<MaterializedFile> {
        let entry = self.inspected.index.get(path).ok_or_else(|| Self::not_found(path))?;
        if entry.fs_entry.is_symlink() {
            return Err(Error::new(ErrorKind::UnsupportedType).with_message(format!("archive symlink cannot be materialized: {path}")));
        }
        if !entry.fs_entry.is_file() {
            return Err(Error::new(ErrorKind::UnsupportedType).with_message(format!("not a file: {path}")));
        }
        ArchiveExtractor::materialize(
            self.identity,
            self.source.path().to_path_buf(),
            self.source_check,
            self.runtime.clone(),
            self.inspected.clone(),
            entry.clone(),
        )
        .await
    }

    async fn create_staging(&self, _dst_dir: &str) -> Result<StagingFile> {
        Err(Self::read_only())
    }

    async fn make_dir(&self, _path: &str) -> Result<()> {
        Err(Self::read_only())
    }

    async fn remove_file(&self, _path: &str) -> Result<()> {
        Err(Self::read_only())
    }

    async fn remove_dir(&self, _path: &str) -> Result<()> {
        Err(Self::read_only())
    }

    async fn rename(&self, _src: &str, _dst: &str, _conflict: crate::core::nfs::ConflictPolicy) -> Result<crate::core::nfs::Publish> {
        Err(Self::read_only())
    }
}

pub(super) struct ArchiveError;

impl ArchiveError {
    pub(super) fn invalid_format(message: impl Into<String>) -> Error {
        Error::new(ErrorKind::InvalidFormat).with_message(message.into())
    }

    pub(super) fn invalid_format_from<E>(error: E) -> Error
    where
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        Error::from_error(error, ErrorKind::InvalidFormat)
    }

    pub(super) fn limit_exceeded(message: impl Into<String>) -> Error {
        Error::new(ErrorKind::LimitExceeded).with_message(message.into())
    }

    pub(super) fn limit_exceeded_from<E>(error: E) -> Error
    where
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        Error::from_error(error, ErrorKind::LimitExceeded)
    }

    pub(super) fn source_path(path: PathBuf, error: std::io::Error) -> Error {
        Error::from_error(error, ErrorKind::IoError).with_message(path.display().to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::{io::Cursor, sync::Arc};

    use async_zip::{Compression, StringEncoding, ZipEntryBuilder, ZipString, base::write::ZipFileWriter};
    use futures::TryStreamExt as _;
    use sevenz_rust2::{
        ArchiveEntry as SevenZipEntry, ArchiveWriter, Password, SourceReader,
        encoder_options::{AesEncoderOptions, Lzma2Options},
    };
    use testresult::TestResult;
    use tokio::io::AsyncWriteExt as _;
    use tokio_util::compat::TokioAsyncWriteCompatExt as _;

    use super::*;

    struct ZipFixtureEntry {
        name: ZipString,
        mode: Option<u16>,
        content: Vec<u8>,
    }

    async fn write_zip(path: &std::path::Path, entries: Vec<ZipFixtureEntry>) -> TestResult {
        let file = tokio::fs::File::create(path).await?;
        let mut writer = ZipFileWriter::new(file.compat_write());
        for entry in entries {
            let mut builder = ZipEntryBuilder::new(entry.name, Compression::Stored);
            if let Some(mode) = entry.mode {
                builder = builder.unix_permissions(mode);
            }
            writer.write_entry_whole(builder, &entry.content).await?;
        }
        // tokio の File は drop しても書き込みの完了を待たないため、閉じる前に flush する。
        let mut file = writer.close().await?.into_inner();
        file.flush().await?;
        Ok(())
    }

    fn zip_file(name: impl Into<ZipString>, content: impl Into<Vec<u8>>) -> ZipFixtureEntry {
        ZipFixtureEntry {
            name: name.into(),
            mode: Some(0o100644),
            content: content.into(),
        }
    }

    fn write_7z(path: &std::path::Path, solid: bool) -> TestResult {
        let mut writer = ArchiveWriter::create(path)?;
        if solid {
            writer.push_archive_entries(
                vec![SevenZipEntry::new_file("a.txt"), SevenZipEntry::new_file("b.txt")],
                vec![SourceReader::new(Cursor::new(b"alpha".to_vec())), SourceReader::new(Cursor::new(b"bravo".to_vec()))],
            )?;
        } else {
            writer.push_archive_entry(SevenZipEntry::new_file("a.txt"), Some(Cursor::new(b"alpha".to_vec())))?;
            writer.push_archive_entry(SevenZipEntry::new_file("b.txt"), Some(Cursor::new(b"bravo".to_vec())))?;
        }
        writer.finish()?;
        Ok(())
    }

    async fn identity(path: &std::path::Path) -> Result<ArchiveIdentity> {
        let fingerprint = SourceFingerprint::read(path).await?;
        Ok(ArchiveIdentity::from_local(&path.to_string_lossy(), &fingerprint))
    }

    async fn open(path: &std::path::Path, options: ArchiveOptions) -> Result<FileStorageArchive> {
        let identity = identity(path).await?;
        FileStorageArchive::open(MaterializedFile::borrowed(path), &path.to_string_lossy(), identity, Arc::new(ArchiveRuntime::new(options)?)).await
    }

    fn patch_first_central_directory(path: &std::path::Path, field_offset: usize, patch: impl FnOnce(&mut [u8])) -> TestResult {
        let mut bytes = std::fs::read(path)?;
        let offset = bytes
            .windows(4)
            .position(|window| window == b"PK\x01\x02")
            .ok_or_else(|| std::io::Error::other("central directory not found"))?;
        patch(&mut bytes[offset + field_offset..]);
        std::fs::write(path, bytes)?;
        Ok(())
    }

    #[tokio::test]
    async fn zip_driver_root_listing_materialize_and_readonly_test() -> TestResult {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("sample.zip");
        write_zip(&path, vec![zip_file("a/b.txt", b"hello".to_vec()), zip_file("root.txt", b"root".to_vec())]).await?;
        let driver = open(&path, ArchiveOptions::default()).await?;

        assert!(driver.exists_dir("").await?);
        assert!(driver.exists_dir("a").await?);
        assert!(driver.exists_file("a/b.txt").await?);
        let root = driver.find_entries("", false).await?.try_collect::<Vec<_>>().await?;
        assert_eq!(root.iter().map(|entry| entry.path.as_str()).collect::<Vec<_>>(), ["a", "root.txt"]);
        let recursive = driver.find_entries("", true).await?.try_collect::<Vec<_>>().await?;
        assert_eq!(recursive.iter().map(|entry| entry.path.as_str()).collect::<Vec<_>>(), ["a", "a/b.txt", "root.txt"]);

        let materialized = driver.materialize_file("a/b.txt").await?;
        let target_path = materialized.path().to_path_buf();
        assert_eq!(tokio::fs::read(&target_path).await?, b"hello");
        assert!(target_path.exists());
        drop(materialized);
        assert!(!target_path.exists());

        assert_eq!(driver.get_entry("missing").await.unwrap_err().kind(), &ErrorKind::NotFound);
        assert_eq!(driver.create_staging("").await.err().unwrap().kind(), &ErrorKind::ReadOnly);
        assert_eq!(driver.make_dir("x").await.unwrap_err().kind(), &ErrorKind::ReadOnly);
        assert_eq!(driver.remove_file("x").await.unwrap_err().kind(), &ErrorKind::ReadOnly);
        assert_eq!(driver.remove_dir("x").await.unwrap_err().kind(), &ErrorKind::ReadOnly);
        assert_eq!(
            driver.rename("x", "y", crate::core::nfs::ConflictPolicy::Fail).await.unwrap_err().kind(),
            &ErrorKind::ReadOnly
        );
        Ok(())
    }

    #[tokio::test]
    async fn zip_cp932_and_none_test() -> TestResult {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("legacy.zip");
        let (name, _, _) = encoding_rs::SHIFT_JIS.encode("画像/猫.txt");
        write_zip(&path, vec![zip_file(ZipString::new(name.into_owned(), StringEncoding::Raw), b"cat".to_vec())]).await?;

        let driver = open(&path, ArchiveOptions::default()).await?;
        assert!(driver.exists_file("画像/猫.txt").await?);

        let error = open(
            &path,
            ArchiveOptions {
                zip_legacy_encoding: ZipLegacyEncoding::None,
                ..ArchiveOptions::default()
            },
        )
        .await
        .err()
        .unwrap();
        assert_eq!(error.kind(), &ErrorKind::InvalidFormat);
        Ok(())
    }

    #[tokio::test]
    async fn zip_symlink_has_no_target_and_cannot_materialize_test() -> TestResult {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("symlink.zip");
        write_zip(
            &path,
            vec![ZipFixtureEntry {
                name: "link".into(),
                mode: Some(0o120777),
                content: b"target.txt".to_vec(),
            }],
        )
        .await?;
        let driver = open(&path, ArchiveOptions::default()).await?;
        assert_eq!(driver.get_entry("link").await?.detail, FsEntryDetail::Link { target: None, target_kind: None });
        assert_eq!(driver.materialize_file("link").await.unwrap_err().kind(), &ErrorKind::UnsupportedType);
        Ok(())
    }

    #[tokio::test]
    async fn zip_encryption_crc_and_format_errors_test() -> TestResult {
        let temp = tempfile::tempdir()?;

        let encrypted = temp.path().join("encrypted.zip");
        write_zip(&encrypted, vec![zip_file("a.txt", b"alpha".to_vec())]).await?;
        patch_first_central_directory(&encrypted, 8, |bytes| {
            let flags = u16::from_le_bytes(bytes[..2].try_into().unwrap()) | 1;
            bytes[..2].copy_from_slice(&flags.to_le_bytes());
        })?;
        let mut encrypted_bytes = std::fs::read(&encrypted)?;
        let local_flags = u16::from_le_bytes(encrypted_bytes[6..8].try_into().unwrap()) | 1;
        encrypted_bytes[6..8].copy_from_slice(&local_flags.to_le_bytes());
        std::fs::write(&encrypted, encrypted_bytes)?;
        assert_eq!(open(&encrypted, ArchiveOptions::default()).await.err().unwrap().kind(), &ErrorKind::PasswordRequired);

        let corrupt = temp.path().join("corrupt.zip");
        write_zip(&corrupt, vec![zip_file("a.txt", b"alpha".to_vec())]).await?;
        patch_first_central_directory(&corrupt, 16, |bytes| bytes[..4].copy_from_slice(&0_u32.to_le_bytes()))?;
        let driver = open(&corrupt, ArchiveOptions::default()).await?;
        assert_eq!(driver.materialize_file("a.txt").await.unwrap_err().kind(), &ErrorKind::InvalidFormat);

        let mismatch = temp.path().join("mismatch.7z");
        write_zip(&mismatch, vec![zip_file("a.txt", b"alpha".to_vec())]).await?;
        assert_eq!(open(&mismatch, ArchiveOptions::default()).await.err().unwrap().kind(), &ErrorKind::InvalidFormat);

        let unknown = temp.path().join("unknown.bin");
        std::fs::write(&unknown, b"not an archive")?;
        assert_eq!(open(&unknown, ArchiveOptions::default()).await.err().unwrap().kind(), &ErrorKind::UnsupportedType);

        let malformed = temp.path().join("malformed.7z");
        std::fs::write(&malformed, b"7z\xBC\xAF\x27\x1C")?;
        assert_eq!(open(&malformed, ArchiveOptions::default()).await.err().unwrap().kind(), &ErrorKind::InvalidFormat);
        Ok(())
    }

    #[tokio::test]
    async fn entry_file_and_block_limits_test() -> TestResult {
        let temp = tempfile::tempdir()?;
        let zip = temp.path().join("limits.zip");
        write_zip(&zip, vec![zip_file("a.txt", b"alpha".to_vec())]).await?;

        let error = open(
            &zip,
            ArchiveOptions {
                max_entry_count: 1,
                ..ArchiveOptions::default()
            },
        )
        .await
        .err()
        .unwrap();
        assert_eq!(error.kind(), &ErrorKind::LimitExceeded);

        let driver = open(
            &zip,
            ArchiveOptions {
                max_materialized_file_size: 4,
                ..ArchiveOptions::default()
            },
        )
        .await?;
        assert_eq!(driver.materialize_file("a.txt").await.unwrap_err().kind(), &ErrorKind::LimitExceeded);

        let driver = open(
            &zip,
            ArchiveOptions {
                max_block_extract_bytes: 4,
                ..ArchiveOptions::default()
            },
        )
        .await?;
        assert_eq!(driver.materialize_file("a.txt").await.unwrap_err().kind(), &ErrorKind::LimitExceeded);
        Ok(())
    }

    #[tokio::test]
    async fn sevenz_solid_and_non_solid_blocks_extract_test() -> TestResult {
        let temp = tempfile::tempdir()?;
        let solid = temp.path().join("solid.7z");
        write_7z(&solid, true)?;
        let driver = open(&solid, ArchiveOptions::default()).await?;
        let a_block = driver.inspected.index.get("a.txt").unwrap().block_id;
        let b_block = driver.inspected.index.get("b.txt").unwrap().block_id;
        assert_eq!(a_block, b_block);
        let materialized = driver.materialize_file("b.txt").await?;
        assert_eq!(std::fs::read(materialized.path())?, b"bravo");
        let decode_count = match &driver.inspected.reader {
            ArchiveReader::SevenZip(reader) => reader.decode_count(),
            ArchiveReader::Zip(_) => unreachable!(),
        };
        assert_eq!(decode_count, 1);

        let non_solid = temp.path().join("non-solid.7z");
        write_7z(&non_solid, false)?;
        let driver = open(&non_solid, ArchiveOptions::default()).await?;
        let a_block = driver.inspected.index.get("a.txt").unwrap().block_id;
        let b_block = driver.inspected.index.get("b.txt").unwrap().block_id;
        assert_ne!(a_block, b_block);
        Ok(())
    }

    #[tokio::test]
    async fn sevenz_encryption_anti_item_and_symlink_test() -> TestResult {
        let temp = tempfile::tempdir()?;

        let encrypted = temp.path().join("encrypted.7z");
        let mut writer = ArchiveWriter::create(&encrypted)?;
        writer.set_content_methods(vec![AesEncoderOptions::new(Password::new("secret")).into(), Lzma2Options::default().into()]);
        writer.push_archive_entry(SevenZipEntry::new_file("a.txt"), Some(Cursor::new(b"alpha".to_vec())))?;
        writer.finish()?;
        assert_eq!(open(&encrypted, ArchiveOptions::default()).await.err().unwrap().kind(), &ErrorKind::PasswordRequired);

        let anti = temp.path().join("anti.7z");
        let mut writer = ArchiveWriter::create(&anti)?;
        let mut entry = SevenZipEntry::new_file("deleted.txt");
        entry.is_anti_item = true;
        writer.push_archive_entry::<Cursor<Vec<u8>>>(entry, None)?;
        writer.finish()?;
        assert_eq!(open(&anti, ArchiveOptions::default()).await.err().unwrap().kind(), &ErrorKind::InvalidFormat);

        let symlink = temp.path().join("symlink.7z");
        let mut writer = ArchiveWriter::create(&symlink)?;
        let mut entry = SevenZipEntry::new_file("link");
        entry.has_windows_attributes = true;
        entry.windows_attributes = 0x8000 | (0o120777 << 16);
        writer.push_archive_entry(entry, Some(Cursor::new(b"target.txt".to_vec())))?;
        writer.finish()?;
        let driver = open(&symlink, ArchiveOptions::default()).await?;
        assert_eq!(driver.get_entry("link").await?.detail, FsEntryDetail::Link { target: None, target_kind: None });
        assert_eq!(driver.materialize_file("link").await.unwrap_err().kind(), &ErrorKind::UnsupportedType);
        Ok(())
    }

    #[tokio::test]
    async fn archive_driver_keeps_source_lease_test() -> TestResult {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("leased.zip");
        write_zip(&path, vec![zip_file("a.txt", b"alpha".to_vec())]).await?;
        let source = MaterializedFile::owned_temp(&path);
        let driver = FileStorageArchive::open(source.clone(), &path.to_string_lossy(), identity(&path).await?, Arc::new(ArchiveRuntime::default())).await?;
        drop(source);
        assert!(path.exists());
        drop(driver);
        assert!(!path.exists());
        Ok(())
    }

    fn write_7z_entries(path: &std::path::Path, entries: &[(&str, &[u8])], solid: bool) -> TestResult {
        let mut writer = ArchiveWriter::create(path)?;
        if solid {
            writer.push_archive_entries(
                entries.iter().map(|(name, _)| SevenZipEntry::new_file(name)).collect(),
                entries.iter().map(|(_, content)| SourceReader::new(Cursor::new(content.to_vec()))).collect(),
            )?;
        } else {
            for (name, content) in entries {
                writer.push_archive_entry(SevenZipEntry::new_file(name), Some(Cursor::new(content.to_vec())))?;
            }
        }
        writer.finish()?;
        Ok(())
    }

    async fn cached_runtime(dir: &std::path::Path, options: ArchiveOptions) -> Result<Arc<ArchiveRuntime>> {
        Ok(Arc::new(ArchiveRuntime::open(options, dir).await?))
    }

    async fn open_with(path: &std::path::Path, runtime: &Arc<ArchiveRuntime>) -> Result<FileStorageArchive> {
        FileStorageArchive::open(MaterializedFile::borrowed(path), &path.to_string_lossy(), identity(path).await?, runtime.clone()).await
    }

    fn count_files(dir: &std::path::Path) -> std::io::Result<usize> {
        let mut count = 0;
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                count += count_files(&entry.path())?;
            } else {
                count += 1;
            }
        }
        Ok(count)
    }

    #[tokio::test]
    async fn extract_group_decodes_solid_block_once_without_cache_test() -> TestResult {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("solid.7z");
        write_7z_entries(&path, &[("a.jpg", b"alpha"), ("b.jpg", b"bravo"), ("c.jpg", b"charlie")], true)?;
        let cache_dir = temp.path().join("cache");
        let options = ArchiveOptions {
            max_concurrent_extractions: 1,
            extract_timeout: std::time::Duration::from_secs(5),
            ..ArchiveOptions::default()
        };
        let runtime = cached_runtime(&cache_dir, options).await?;
        let driver = open_with(&path, &runtime).await?;
        let groups = driver.block_groups(["a.jpg", "b.jpg", "c.jpg"].map(String::from));
        assert_eq!(groups.len(), 1, "solid 7z の同じ block の file は 1 つの組になる");

        // 閲覧の枠を塞いでも、集計の展開は渡した枠で進む。
        let held = runtime.hold_all_permits().await;
        let files = driver.extract_group(&groups[0], Arc::new(Semaphore::new(1))).await?;
        drop(held);
        assert_eq!(files.len(), 3);
        assert_eq!(std::fs::read(files["b.jpg"].path())?, b"bravo");
        assert_eq!(runtime.decoded_blocks(), 1, "block を 1 回だけ decode する");
        assert_eq!(count_files(&cache_dir.join("files"))?, 0, "extraction cache に登録しない");

        let paths = files.values().map(|file| file.path().to_path_buf()).collect::<Vec<_>>();
        drop(files);
        assert!(paths.iter().all(|path| !path.exists()), "読み終えた一時 file を消す");
        Ok(())
    }

    #[tokio::test]
    async fn extract_group_reads_zip_entries_test() -> TestResult {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("sample.zip");
        write_zip(&path, vec![zip_file("a.jpg", b"alpha".to_vec()), zip_file("dir/b.jpg", b"bravo".to_vec())]).await?;
        let driver = open(&path, ArchiveOptions::default()).await?;
        let groups = driver.block_groups(driver.files().into_iter().map(|entry| entry.path));
        assert_eq!(groups.iter().map(|group| group.paths().to_vec()).collect::<Vec<_>>(), [vec!["a.jpg"], vec!["dir/b.jpg"]]);

        let files = driver.extract_group(&groups[1], Arc::new(Semaphore::new(1))).await?;
        assert_eq!(files.len(), 1);
        assert_eq!(std::fs::read(files["dir/b.jpg"].path())?, b"bravo");
        Ok(())
    }

    #[tokio::test]
    async fn cache_hit_skips_decode_test() -> TestResult {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("sample.7z");
        write_7z(&path, false)?;
        let runtime = cached_runtime(&temp.path().join("cache"), ArchiveOptions::default()).await?;
        let driver = open_with(&path, &runtime).await?;

        drop(driver.materialize_file("a.txt").await?);
        let second = driver.materialize_file("a.txt").await?;
        assert_eq!(std::fs::read(second.path())?, b"alpha");
        assert!(second.path().starts_with(temp.path().join("cache")));
        assert_eq!(runtime.decoded_blocks(), 1, "2 回目は decode しない");
        Ok(())
    }

    #[tokio::test]
    async fn cache_survives_restart_test() -> TestResult {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("sample.7z");
        write_7z(&path, false)?;
        let cache_dir = temp.path().join("cache");
        {
            let runtime = cached_runtime(&cache_dir, ArchiveOptions::default()).await?;
            let driver = open_with(&path, &runtime).await?;
            drop(driver.materialize_file("a.txt").await?);
        }

        let runtime = cached_runtime(&cache_dir, ArchiveOptions::default()).await?;
        let driver = open_with(&path, &runtime).await?;
        let materialized = driver.materialize_file("a.txt").await?;
        assert_eq!(std::fs::read(materialized.path())?, b"alpha");
        assert_eq!(runtime.decoded_blocks(), 0, "再起動前に展開した entry を再利用する");
        Ok(())
    }

    #[tokio::test]
    async fn source_change_decodes_again_test() -> TestResult {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("sample.7z");
        write_7z_entries(&path, &[("a.txt", b"alpha")], false)?;
        let runtime = cached_runtime(&temp.path().join("cache"), ArchiveOptions::default()).await?;
        drop(open_with(&path, &runtime).await?.materialize_file("a.txt").await?);
        assert_eq!(runtime.decoded_blocks(), 1);

        // 更新時刻だけを変える。
        let modified = std::fs::metadata(&path)?.modified()? + std::time::Duration::from_secs(3600);
        std::fs::File::options().write(true).open(&path)?.set_modified(modified)?;
        drop(open_with(&path, &runtime).await?.materialize_file("a.txt").await?);
        assert_eq!(runtime.decoded_blocks(), 2, "更新時刻が変われば decode し直す");

        // size だけを変える。
        write_7z_entries(&path, &[("a.txt", b"alpha-2")], false)?;
        std::fs::File::options().write(true).open(&path)?.set_modified(modified)?;
        let materialized = open_with(&path, &runtime).await?.materialize_file("a.txt").await?;
        assert_eq!(std::fs::read(materialized.path())?, b"alpha-2");
        assert_eq!(runtime.decoded_blocks(), 3, "size が変われば decode し直す");
        Ok(())
    }

    #[tokio::test]
    async fn oversized_entry_is_not_cached_test() -> TestResult {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("sample.7z");
        write_7z(&path, false)?;
        let cache_dir = temp.path().join("cache");
        let runtime = cached_runtime(
            &cache_dir,
            ArchiveOptions {
                cache_capacity_bytes: 4,
                ..ArchiveOptions::default()
            },
        )
        .await?;
        let driver = open_with(&path, &runtime).await?;

        let materialized = driver.materialize_file("a.txt").await?;
        assert_eq!(std::fs::read(materialized.path())?, b"alpha");
        assert_eq!(count_files(&cache_dir.join("files"))?, 0, "容量を超える entry は登録しない");
        Ok(())
    }

    #[tokio::test]
    async fn concurrent_requests_in_one_block_decode_once_test() -> TestResult {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("solid.7z");
        write_7z(&path, true)?;
        let runtime = cached_runtime(&temp.path().join("cache"), ArchiveOptions::default()).await?;
        let driver = open_with(&path, &runtime).await?;

        let (a, b) = tokio::join!(driver.materialize_file("a.txt"), driver.materialize_file("b.txt"));
        assert_eq!(std::fs::read(a?.path())?, b"alpha");
        assert_eq!(std::fs::read(b?.path())?, b"bravo");
        assert_eq!(runtime.decoded_blocks(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn solid_7z_cumulative_decode_is_bounded_test() -> TestResult {
        const ENTRY_COUNT: usize = 300;
        const ENTRY_SIZE: usize = 1024;
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("solid.7z");
        let names = (0..ENTRY_COUNT).map(|index| format!("{index:03}.bin")).collect::<Vec<_>>();
        let contents = (0..ENTRY_COUNT).map(|index| vec![index as u8; ENTRY_SIZE]).collect::<Vec<_>>();
        let entries = names.iter().zip(&contents).map(|(name, content)| (name.as_str(), content.as_slice())).collect::<Vec<_>>();
        write_7z_entries(&path, &entries, true)?;

        let runtime = cached_runtime(&temp.path().join("cache"), ArchiveOptions::default()).await?;
        let driver = open_with(&path, &runtime).await?;
        for (name, content) in &entries {
            let materialized = driver.materialize_file(name).await?;
            assert_eq!(&std::fs::read(materialized.path())?, content);
        }

        let decoded_bytes = match &driver.inspected.reader {
            ArchiveReader::SevenZip(reader) => reader.decoded_bytes(),
            ArchiveReader::Zip(_) => unreachable!(),
        };
        let uncompressed = (ENTRY_COUNT * ENTRY_SIZE) as u64;
        assert!(decoded_bytes <= uncompressed * 2, "decoded {decoded_bytes} bytes for {uncompressed} bytes");
        Ok(())
    }

    #[tokio::test]
    async fn source_change_is_rejected_without_leaving_staging_test() -> TestResult {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("sample.7z");
        write_7z(&path, false)?;
        let cache_dir = temp.path().join("cache");
        let runtime = cached_runtime(&cache_dir, ArchiveOptions::default()).await?;
        let fingerprint = SourceFingerprint::read(&path).await?;
        let driver = open_with(&path, &runtime).await?.verify_source(fingerprint);

        let modified = std::fs::metadata(&path)?.modified()? + std::time::Duration::from_secs(3600);
        std::fs::File::options().write(true).open(&path)?.set_modified(modified)?;

        let error = driver.materialize_file("a.txt").await.unwrap_err();
        assert_eq!(error.kind(), &ErrorKind::SourceChanged);
        assert_eq!(count_files(&cache_dir.join("staging"))?, 0, "staging を残さない");
        assert_eq!(count_files(&cache_dir.join("files"))?, 0, "変更前の source の結果を登録しない");
        Ok(())
    }
}
