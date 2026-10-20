use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Instant, SystemTime},
};

use async_trait::async_trait;
use futures::StreamExt as _;
use omnius_core_base::tsid::TsidProvider;
use tokio_util::sync::CancellationToken;

use crate::{
    core::nfs::{
        ConflictPolicy, FileStorageDriver, FsEntryStream, FsEntrySummaryStream, LiveStagingSet, MaterializedFile, Publish, STAGING_MAX_AGE, StagingFile, StagingPublisher,
        StagingSweeper,
    },
    prelude::*,
};

use super::archive::format::ArchiveFormat;

mod listing;
use listing::LocalListingRuntime;

/// ホスト filesystem 全体に bind された driver。
///
/// 受け取る path は UTF-8 の absolute path であり、`canonicalize` は要求しない。
/// source がまだ存在しない copy destination も同じ表現で扱う。
pub struct FileStorageLocal {
    tsid_provider: Arc<Mutex<dyn TsidProvider + Send + Sync>>,
    live_staging: LiveStagingSet,
    listing: Arc<LocalListingRuntime>,
}

impl FileStorageLocal {
    pub fn new(tsid_provider: Arc<Mutex<dyn TsidProvider + Send + Sync>>) -> Self {
        Self::with_budget(tsid_provider, Arc::new(crate::core::work::WorkBudget::default()))
    }

    pub fn with_budget(tsid_provider: Arc<Mutex<dyn TsidProvider + Send + Sync>>, budget: Arc<crate::core::work::WorkBudget>) -> Self {
        Self {
            tsid_provider,
            live_staging: Arc::new(Mutex::new(Default::default())),
            listing: Arc::new(LocalListingRuntime::with_budget(budget)),
        }
    }

    pub fn work_budget(&self) -> &Arc<crate::core::work::WorkBudget> {
        &self.listing.budget
    }

    /// `dir` に残った孤児 staging を回収する。
    ///
    /// staging を作るたびではなく、ツリー copy や upload といった Operation が destination
    /// directory ごとに 1 回だけ呼ぶ。1 file ごとに走査すると、entry 数の多いフォルダで
    /// 走査回数が file 数に比例して増えるためである。
    pub async fn sweep_staging(&self, dir: &str) -> Result<u64> {
        let older_than = SystemTime::now().checked_sub(STAGING_MAX_AGE).unwrap_or(SystemTime::UNIX_EPOCH);
        StagingSweeper::sweep(Path::new(dir), &self.live_staging, older_than).await
    }

    fn to_utf8(path: &Path) -> Result<String> {
        // lossy にすると表示名が実体と一致せず、正規化後の path が衝突し得る。
        path.to_str()
            .map(str::to_string)
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput).with_message(format!("path is not valid UTF-8: {}", path.to_string_lossy())))
    }

    fn to_fs_metadata(meta: &std::fs::Metadata) -> FsMetadata {
        // timestamp は best-effort とし、欠落だけで metadata 操作を失敗させない。
        FsMetadata {
            len: meta.len(),
            readonly: meta.permissions().readonly(),
            created: meta.created().ok().map(Into::into),
            modified: meta.modified().ok().map(Into::into),
            accessed: meta.accessed().ok().map(Into::into),
        }
    }

    fn to_file_or_dir_detail(meta: &std::fs::Metadata, path: &str) -> Result<FsEntryDetail> {
        if meta.is_dir() {
            Ok(FsEntryDetail::Dir { has_children: None })
        } else if meta.is_file() {
            Ok(FsEntryDetail::File {
                archive: ArchiveFormat::supports_path(path),
            })
        } else {
            Err(Error::new(ErrorKind::UnsupportedType).with_message(format!("unsupported entry: {path}")))
        }
    }

    fn io_error(error: std::io::Error, path: &str) -> Error {
        let kind = match error.kind() {
            std::io::ErrorKind::NotFound => ErrorKind::NotFound,
            std::io::ErrorKind::AlreadyExists => ErrorKind::AlreadyExists,
            _ => ErrorKind::IoError,
        };
        Error::from_error(error, kind).with_message(format!("io operation failed: {path}"))
    }
}

#[async_trait]
impl FileStorageDriver for FileStorageLocal {
    async fn get_entry(&self, path: &str) -> Result<FsEntry> {
        // 操作対象として直接指名された path なので symlink を追跡する。
        let meta = tokio::fs::metadata(path).await.map_err(|e| Self::io_error(e, path))?;

        Ok(FsEntry {
            path: path.to_string(),
            meta: Self::to_fs_metadata(&meta),
            detail: Self::to_file_or_dir_detail(&meta, path)?,
        })
    }

    async fn find_entries(&self, path: &str, recursive: bool) -> Result<FsEntryStream> {
        let root = PathBuf::from(path);

        let meta = tokio::fs::metadata(&root).await.map_err(|e| Self::io_error(e, path))?;
        if !meta.is_dir() {
            return Err(Error::new(ErrorKind::UnsupportedType).with_message(format!("not a directory: {path}")));
        }

        let runtime = self.listing.clone();
        let class = crate::core::work::WorkClass::current();
        Ok(Box::pin(async_stream::try_stream! {
            let cancellation = CancellationToken::new();
            let _cancel_on_drop = cancellation.clone().drop_guard();
            let _scan_permit = class.scope(runtime.enter()).await?;
            let started = Instant::now();
            let mut enumeration_sec = 0.0;
            let mut metadata_sec = 0.0;
            let mut entry_count = 0_u64;
            let mut stack = vec![root];
            while let Some(dir) = stack.pop() {
                let enumeration_started = Instant::now();
                let read_dir = class.scope(runtime.open_directory(dir, cancellation.clone())).await?;
                enumeration_sec += enumeration_started.elapsed().as_secs_f64();
                loop {
                    let mut paths = Vec::with_capacity(LocalListingRuntime::BATCH_SIZE * LocalListingRuntime::PARALLEL_BATCHES);
                    let enumeration_started = Instant::now();
                    for _ in 0..LocalListingRuntime::PARALLEL_BATCHES {
                        let batch = class.scope(runtime.read_paths(read_dir.clone(), cancellation.clone())).await?;
                        let count = batch.len();
                        paths.extend(batch);
                        if count < LocalListingRuntime::BATCH_SIZE { break; }
                    }
                    enumeration_sec += enumeration_started.elapsed().as_secs_f64();
                    if paths.is_empty() { break; }
                    let chunks: Vec<_> = paths.chunks(LocalListingRuntime::BATCH_SIZE).map(|chunk| chunk.to_vec()).collect();
                    let mut batches = futures::stream::iter(chunks)
                        .map(|paths| class.scope(runtime.read_batch(paths, cancellation.clone())))
                        .buffered(LocalListingRuntime::PARALLEL_BATCHES);
                    while let Some(batch) = batches.next().await {
                        let batch = batch?;
                        metadata_sec += batch.metadata_sec;
                        entry_count += batch.entries.len() as u64;
                        for entry in batch.entries {
                            if recursive && matches!(entry.detail, FsEntryDetail::Dir { .. }) {
                                stack.push(PathBuf::from(&entry.path));
                            }
                            yield entry;
                        }
                    }
                }
            }
            // metadata_sec は並列 worker の合計、elapsed_sec は消費側の待ちも含む壁時計時間。
            tracing::debug!(target: "pxna::performance", operation = "local_listing", recursive,
                entry_count, enumeration_sec, metadata_sec, elapsed_sec = started.elapsed().as_secs_f64(),
                "listing stream completed");
        }))
    }

    async fn get_listing_entry(&self, path: &str) -> Result<FsEntry> {
        self.listing.read_entry(path).await
    }

    async fn find_entry_summaries(&self, path: &str) -> Result<FsEntrySummaryStream> {
        let root = PathBuf::from(path);
        let meta = tokio::fs::metadata(&root).await.map_err(|error| Self::io_error(error, path))?;
        if !meta.is_dir() {
            return Err(Error::new(ErrorKind::UnsupportedType).with_message(format!("not a directory: {path}")));
        }
        let runtime = self.listing.clone();
        let class = crate::core::work::WorkClass::current();
        Ok(Box::pin(async_stream::try_stream! {
            let cancellation = CancellationToken::new();
            let _cancel_on_drop = cancellation.clone().drop_guard();
            let _scan_permit = class.scope(runtime.enter()).await?;
            let started = Instant::now();
            let directory = class.scope(runtime.open_directory(root, cancellation.clone())).await?;
            let mut entry_count = 0_u64;
            loop {
                let batch = class.scope(runtime.read_summaries(directory.clone(), cancellation.clone())).await?;
                if batch.is_empty() { break; }
                entry_count += batch.len() as u64;
                for entry in batch { yield entry; }
            }
            tracing::debug!(target: "pxna::performance", operation = "local_summary_listing", entry_count,
                elapsed_sec = started.elapsed().as_secs_f64(), "summary listing completed");
        }))
    }

    async fn exists_file(&self, path: &str) -> Result<bool> {
        match tokio::fs::metadata(path).await {
            Ok(meta) => Ok(meta.is_file()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(Self::io_error(e, path)),
        }
    }

    async fn exists_dir(&self, path: &str) -> Result<bool> {
        match tokio::fs::metadata(path).await {
            Ok(meta) => Ok(meta.is_dir()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(Self::io_error(e, path)),
        }
    }

    async fn materialize_file(&self, path: &str) -> Result<MaterializedFile> {
        let meta = tokio::fs::metadata(path).await.map_err(|e| Self::io_error(e, path))?;
        if !meta.is_file() {
            return Err(Error::new(ErrorKind::UnsupportedType).with_message(format!("not a file: {path}")));
        }

        // local file はすでに物理 file なので、lease は何もしない。
        Ok(MaterializedFile::borrowed(path))
    }

    async fn create_staging(&self, dst_dir: &str) -> Result<StagingFile> {
        let tsid = self.tsid_provider.lock().create().to_string();
        StagingFile::create(Path::new(dst_dir), &tsid, self.live_staging.clone()).await
    }

    async fn make_dir(&self, path: &str) -> Result<()> {
        tokio::fs::create_dir(path).await.map_err(|e| Self::io_error(e, path))
    }

    async fn remove_file(&self, path: &str) -> Result<()> {
        // symlink を追跡せず link entry 自体を削除する。
        tokio::fs::remove_file(path).await.map_err(|e| Self::io_error(e, path))
    }

    async fn remove_dir(&self, path: &str) -> Result<()> {
        tokio::fs::remove_dir(path).await.map_err(|e| Self::io_error(e, path))
    }

    async fn rename(&self, src: &str, dst: &str, conflict: ConflictPolicy) -> Result<Publish> {
        StagingPublisher::apply(Path::new(src).to_path_buf(), Path::new(dst).to_path_buf(), conflict).await
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt as _;
    use omnius_core_base::{clock::ClockUtc, tsid::TsidProviderImpl};
    use rand::{SeedableRng as _, rngs::ChaCha20Rng};
    use testresult::TestResult;

    use super::*;
    use crate::core::nfs::{ConflictPolicy, MAX_READ_FILE_BYTES, STAGING_PREFIX};

    fn driver() -> FileStorageLocal {
        let tsid_provider: Arc<Mutex<dyn TsidProvider + Send + Sync>> = Arc::new(Mutex::new(TsidProviderImpl::new(ClockUtc, ChaCha20Rng::seed_from_u64(0), 16)));
        FileStorageLocal::new(tsid_provider)
    }

    fn s(path: &Path) -> String {
        path.to_str().unwrap().to_string()
    }

    async fn collect(stream: FsEntryStream) -> Result<Vec<FsEntry>> {
        let mut entries: Vec<FsEntry> = stream.collect::<Vec<_>>().await.into_iter().collect::<Result<Vec<_>>>()?;
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(entries)
    }

    #[tokio::test]
    async fn rename_applies_conflict_without_losing_source_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let src = dir.path().join("source.txt");
        let dst = dir.path().join("destination.txt");
        tokio::fs::write(&src, b"source").await?;
        tokio::fs::write(&dst, b"existing").await?;
        let driver = driver();
        let error = driver.rename(&s(&src), &s(&dst), ConflictPolicy::Fail).await.unwrap_err();
        assert_eq!(*error.kind(), ErrorKind::AlreadyExists);
        assert_eq!(tokio::fs::read(&dst).await?, b"existing");
        assert_eq!(driver.rename(&s(&src), &s(&dst), ConflictPolicy::Skip).await?, crate::core::nfs::Publish::Skipped);
        assert_eq!(tokio::fs::read(&src).await?, b"source");
        let crate::core::nfs::Publish::Published { dst: renamed } = driver.rename(&s(&src), &s(&dst), ConflictPolicy::Rename).await? else {
            panic!("rename must publish");
        };
        assert_ne!(renamed, s(&dst));
        assert_eq!(tokio::fs::read(&renamed).await?, b"source");
        assert_eq!(tokio::fs::read(&dst).await?, b"existing");
        tokio::fs::write(&src, b"replacement").await?;
        driver.rename(&s(&src), &s(&dst), ConflictPolicy::Overwrite).await?;
        assert!(!src.exists());
        assert_eq!(tokio::fs::read(&dst).await?, b"replacement");
        Ok(())
    }

    #[tokio::test]
    async fn get_entry_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("a.zip");
        tokio::fs::write(&file, b"hello").await?;

        let driver = driver();

        let entry = driver.get_entry(&s(&file)).await?;
        assert_eq!(entry.path, s(&file));
        assert_eq!(entry.meta.len, 5);
        assert_eq!(entry.detail, FsEntryDetail::File { archive: true });

        let entry = driver.get_entry(&s(dir.path())).await?;
        assert_eq!(entry.detail, FsEntryDetail::Dir { has_children: None });

        let e = driver.get_entry(&s(&dir.path().join("missing"))).await.unwrap_err();
        assert_eq!(*e.kind(), ErrorKind::NotFound);

        Ok(())
    }

    #[tokio::test]
    async fn exists_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("a.txt");
        tokio::fs::write(&file, b"hello").await?;

        let driver = driver();

        assert!(driver.exists_file(&s(&file)).await?);
        assert!(!driver.exists_dir(&s(&file)).await?);
        assert!(driver.exists_dir(&s(dir.path())).await?);
        assert!(!driver.exists_file(&s(dir.path())).await?);
        assert!(!driver.exists_file(&s(&dir.path().join("missing"))).await?);

        Ok(())
    }

    #[tokio::test]
    async fn find_entries_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        tokio::fs::create_dir(dir.path().join("sub")).await?;
        tokio::fs::write(dir.path().join("a.txt"), b"a").await?;
        tokio::fs::write(dir.path().join("sub/b.txt"), b"b").await?;

        let driver = driver();

        let entries = collect(driver.find_entries(&s(dir.path()), false).await?).await?;
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, s(&dir.path().join("a.txt")));
        assert_eq!(entries[1].path, s(&dir.path().join("sub")));

        let entries = collect(driver.find_entries(&s(dir.path()), true).await?).await?;
        assert_eq!(entries.len(), 3, "全 descendant を返す");

        Ok(())
    }

    #[tokio::test]
    async fn read_file_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("a.txt");
        tokio::fs::write(&file, b"0123456789").await?;

        let driver = driver();
        let path = s(&file);

        assert_eq!(&driver.read_file(&path, 0, 10).await?[..], b"0123456789");
        assert_eq!(&driver.read_file(&path, 3, 4).await?[..], b"3456");
        assert_eq!(&driver.read_file(&path, 8, 100).await?[..], b"89", "EOF で打ち切る");
        assert!(driver.read_file(&path, 0, 0).await?.is_empty(), "limit = 0 は empty bytes");
        assert!(driver.read_file(&path, 10, 5).await?.is_empty(), "offset >= EOF は empty bytes");

        let e = driver.read_file(&path, 0, MAX_READ_FILE_BYTES + 1).await.unwrap_err();
        assert_eq!(*e.kind(), ErrorKind::LimitExceeded);

        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn summary_and_leaf_details_do_not_follow_links_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        std::fs::create_dir(dir.path().join("child"))?;
        std::fs::write(dir.path().join("image.zip"), b"not opened")?;
        std::os::unix::fs::symlink(dir.path().join("missing"), dir.path().join("link.zip"))?;
        let driver = driver();
        let summaries = driver
            .find_entry_summaries(&s(dir.path()))
            .await?
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
        assert_eq!(summaries.len(), 3);
        let link = summaries.iter().find(|entry| entry.path.ends_with("link.zip")).unwrap();
        assert_eq!(link.kind, FsEntryKind::Link);
        assert!(!link.archive);
        assert!(summaries.iter().any(|entry| entry.path.ends_with("image.zip") && entry.archive));
        let detail = driver.get_listing_entry(&link.path).await?;
        assert!(matches!(detail.detail, FsEntryDetail::Link { target_kind: None, .. }));
        assert!(driver.get_entry(&link.path).await.is_err());
        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn summary_rejects_non_utf8_and_special_entries_test() -> TestResult {
        use std::os::unix::ffi::OsStringExt as _;
        let dir = tempfile::tempdir()?;
        let driver = driver();
        let name = std::ffi::OsString::from_vec(vec![0xff]);
        std::fs::write(dir.path().join(&name), b"")?;
        let mut stream = driver.find_entry_summaries(&s(dir.path())).await?;
        assert!(stream.next().await.unwrap().is_err());
        drop(stream);
        std::fs::remove_file(dir.path().join(name))?;
        let socket = std::os::unix::net::UnixListener::bind(dir.path().join("socket"))?;
        let mut stream = driver.find_entry_summaries(&s(dir.path())).await?;
        assert_eq!(stream.next().await.unwrap().unwrap_err().kind(), &ErrorKind::UnsupportedType);
        drop(socket);
        Ok(())
    }

    #[tokio::test]
    async fn mutation_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let driver = driver();

        let sub = dir.path().join("sub");
        driver.make_dir(&s(&sub)).await?;
        assert!(sub.is_dir());

        let file = sub.join("a.txt");
        tokio::fs::write(&file, b"a").await?;

        let moved = sub.join("b.txt");
        driver.rename(&s(&file), &s(&moved), ConflictPolicy::Fail).await?;
        assert!(!file.exists());
        assert!(moved.exists());

        driver.remove_file(&s(&moved)).await?;
        assert!(!moved.exists());

        driver.remove_dir(&s(&sub)).await?;
        assert!(!sub.exists());

        Ok(())
    }

    #[tokio::test]
    async fn create_staging_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let driver = driver();

        let mut staging = driver.create_staging(&s(dir.path())).await?;
        staging.write_all(b"hello").await?;
        assert_eq!(staging.len(), 5);

        let dst = dir.path().join("a.txt");
        staging.publish(&s(&dst), ConflictPolicy::Fail).await?;
        assert_eq!(tokio::fs::read(&dst).await?, b"hello");

        Ok(())
    }

    #[tokio::test]
    async fn sweep_staging_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let driver = driver();

        let old = dir.path().join(format!("{STAGING_PREFIX}old"));
        tokio::fs::write(&old, b"x").await?;
        std::fs::File::options()
            .write(true)
            .open(&old)?
            .set_modified(SystemTime::now() - STAGING_MAX_AGE - std::time::Duration::from_secs(60))?;

        let live = driver.create_staging(&s(dir.path())).await?;
        let live_path = live.path().to_path_buf();

        assert_eq!(driver.sweep_staging(&s(dir.path())).await?, 1);
        assert!(!old.exists());
        assert!(live_path.exists(), "生存中の staging は回収しない");

        Ok(())
    }

    #[cfg(unix)]
    mod symlink {
        use super::*;

        #[tokio::test]
        async fn follows_named_target_but_not_during_traversal_test() -> TestResult {
            let dir = tempfile::tempdir()?;
            let target = dir.path().join("target.txt");
            tokio::fs::write(&target, b"hello").await?;

            let link = dir.path().join("link.txt");
            std::os::unix::fs::symlink(&target, &link)?;

            let driver = driver();

            // 直接指名した symlink は target を見る。
            let entry = driver.get_entry(&s(&link)).await?;
            assert_eq!(entry.detail, FsEntryDetail::File { archive: false });
            assert_eq!(entry.meta.len, 5);
            assert!(driver.exists_file(&s(&link)).await?);
            assert_eq!(&driver.read_file(&s(&link), 0, 5).await?[..], b"hello");

            // 走査中に見つけた symlink は追跡せず link entry として返す。
            let entries = collect(driver.find_entries(&s(dir.path()), false).await?).await?;
            let found = entries.iter().find(|n| n.path == s(&link)).unwrap();
            assert_eq!(
                found.detail,
                FsEntryDetail::Link {
                    target: Some(s(&target)),
                    target_kind: Some(FsEntryKind::File),
                }
            );

            Ok(())
        }

        #[tokio::test]
        async fn does_not_descend_into_linked_directory_test() -> TestResult {
            let dir = tempfile::tempdir()?;
            let sub = dir.path().join("sub");
            tokio::fs::create_dir(&sub).await?;
            tokio::fs::write(sub.join("inner.txt"), b"inner").await?;

            let link = dir.path().join("link");
            std::os::unix::fs::symlink(&sub, &link)?;

            let driver = driver();
            let entries = collect(driver.find_entries(&s(dir.path()), true).await?).await?;

            let paths: Vec<&str> = entries.iter().map(|n| n.path.as_str()).collect();
            assert!(paths.contains(&s(&link).as_str()));
            assert!(!paths.iter().any(|n| n.starts_with(&format!("{}/", s(&link)))), "link の先へ降りない");

            Ok(())
        }

        #[tokio::test]
        async fn broken_link_is_returned_without_target_kind_test() -> TestResult {
            let dir = tempfile::tempdir()?;
            let link = dir.path().join("broken");
            std::os::unix::fs::symlink(dir.path().join("missing"), &link)?;

            let driver = driver();

            let entries = collect(driver.find_entries(&s(dir.path()), false).await?).await?;
            assert_eq!(entries.len(), 1, "壊れた symlink を黙って除外しない");
            assert!(matches!(entries[0].detail, FsEntryDetail::Link { target_kind: None, .. }));

            assert!(!driver.exists_file(&s(&link)).await?);

            Ok(())
        }
    }
}
