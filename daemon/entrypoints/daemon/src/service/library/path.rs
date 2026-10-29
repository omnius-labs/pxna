use std::{io, path::Path, sync::Arc};

use omnius_pxna_engine::{
    model::NestedPathBuf,
    service::{ResolvedStoragePath, StorageService},
};

use crate::prelude::*;
use omnius_pxna_engine::service::WorkClass;

/// クライアントが送った Library 相対 path を検査し、segment に分けた値。
///
/// `.` を除き、`..` を 1 つ上の segment との相殺で消す。
/// Library の外を指す `..` と、結合すると Library ルートを置き換える絶対 path は `InvalidInput` にする。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryRelativePath {
    segments: Vec<String>,
}

impl LibraryRelativePath {
    pub fn parse(value: &str) -> Result<Self> {
        if value.contains('\0') || value.contains('\\') {
            return Err(Self::invalid(value, "contains a NUL or backslash"));
        }

        let mut segments: Vec<String> = Vec::new();
        for segment in value.split('/') {
            match segment {
                "" | "." => {}
                ".." => {
                    if segments.pop().is_none() {
                        return Err(Self::invalid(value, "escapes the library root"));
                    }
                }
                _ => {
                    if Self::is_drive_prefix(segment) {
                        return Err(Self::invalid(value, "is an absolute path"));
                    }
                    segments.push(segment.to_string());
                }
            }
        }

        Ok(Self { segments })
    }

    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    // `C:` のような segment は、Windows で `Path::join` すると Library ルートを置き換える。
    fn is_drive_prefix(segment: &str) -> bool {
        let bytes = segment.as_bytes();
        bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
    }

    fn invalid(value: &str, reason: &str) -> Error {
        Error::new(ErrorKind::InvalidInput).with_message(format!("path {reason}: {value:?}"))
    }
}

/// `libraryId` と Library 相対 path から、検査済みの `NestedPath` を作る唯一の境界。
///
/// archive を含まない path は完全 path の `stat` 1 回で確定する。
/// `stat` が `ENOENT` か `ENOTDIR` を返したときだけ、先頭の segment から順に archive 境界を探す。
/// 探索結果は cache しない。境界の位置は size と更新時刻に現れないためである。
pub struct LibraryPathResolver {
    storage: Arc<StorageService>,
    max_nested_depth: usize,
    #[cfg(test)]
    stat_calls: std::sync::atomic::AtomicUsize,
}

impl LibraryPathResolver {
    pub fn new(storage: Arc<StorageService>, max_nested_depth: usize) -> Self {
        Self {
            storage,
            max_nested_depth,
            #[cfg(test)]
            stat_calls: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    #[cfg(test)]
    pub async fn resolve(&self, library_root: &Path, relative: &LibraryRelativePath) -> Result<NestedPathBuf> {
        Ok(self.resolve_entry(library_root, relative).await?.path().to_nested_path_buf())
    }

    /// 詳細一覧の leaf は lstat で存在を確認し、壊れた symlink も解決する。
    pub async fn resolve_listing_entry(&self, library_root: &Path, relative: &LibraryRelativePath) -> Result<ResolvedStoragePath> {
        let mut full = library_root.to_path_buf();
        full.extend(relative.segments());
        match tokio::fs::symlink_metadata(&full).await {
            Ok(_) => Ok(self.storage.resolve_entry(&Self::local(&full)?, WorkClass::Foreground).await?),
            Err(error) if Self::is_missing(&error) => self.resolve_entry(library_root, relative).await,
            Err(error) => Err(error.into()),
        }
    }

    pub async fn resolve_entry(&self, library_root: &Path, relative: &LibraryRelativePath) -> Result<ResolvedStoragePath> {
        let segments = relative.segments();
        let mut full = library_root.to_path_buf();
        full.extend(segments);

        match self.stat(&full).await {
            Ok(_) => return Ok(self.storage.resolve_entry(&Self::local(&full)?, WorkClass::Foreground).await?),
            Err(error) if Self::is_missing(&error) => {}
            Err(error) => return Err(error.into()),
        }

        let mut current = library_root.to_path_buf();
        for (index, segment) in segments.iter().enumerate() {
            current.push(segment);
            match self.stat(&current).await {
                Ok(metadata) if metadata.is_dir() => {}
                Ok(_) => return self.resolve_inside(Self::local(&current)?, &segments[index + 1..]).await,
                Err(error) if Self::is_missing(&error) => break,
                Err(error) => return Err(error.into()),
            }
        }

        Err(Self::not_found(relative))
    }
}

impl LibraryPathResolver {
    /// archive file の内側を、その archive の索引に対して同じ手順で探す。
    async fn resolve_inside(&self, mut outer: NestedPathBuf, mut rest: &[String]) -> Result<ResolvedStoragePath> {
        let mut depth = 1;
        loop {
            if depth > self.max_nested_depth {
                return Err(Error::new(ErrorKind::LimitExceeded).with_message(format!("archive nesting exceeds {}", self.max_nested_depth)));
            }

            let mut root = outer.clone();
            root.push_archive_root().map_err(Self::path_error)?;
            let namespace = self.storage.resolve_entry(&root, WorkClass::Foreground).await?;
            let candidate = namespace.at(&rest.join("/"))?;
            match candidate.get_entry(WorkClass::Foreground).await {
                Ok(_) => return Ok(candidate),
                Err(error) if error.kind() == &omnius_pxna_engine::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }

            let mut boundary = None;
            for index in 0..rest.len() {
                let prefix = rest[..=index].join("/");
                let candidate = namespace.at(&prefix)?;
                match candidate.get_entry(WorkClass::Foreground).await {
                    Ok(entry) if entry.detail.kind() == omnius_pxna_engine::model::FsEntryKind::Dir => {}
                    Ok(_) => {
                        boundary = Some((candidate, index));
                        break;
                    }
                    Err(error) if error.kind() == &omnius_pxna_engine::ErrorKind::NotFound => break,
                    Err(error) => return Err(error.into()),
                }
            }

            let Some((next, index)) = boundary else {
                return Err(Error::new(ErrorKind::NotFound).with_message(format!("entry not found in archive: {}", rest.join("/"))));
            };
            if index + 1 == rest.len() {
                return Ok(next);
            }
            outer = next.path().to_nested_path_buf();
            rest = &rest[index + 1..];
            depth += 1;
        }
    }

    async fn stat(&self, path: &Path) -> io::Result<std::fs::Metadata> {
        #[cfg(test)]
        self.stat_calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        tokio::fs::metadata(path).await
    }

    fn is_missing(error: &io::Error) -> bool {
        matches!(error.kind(), io::ErrorKind::NotFound | io::ErrorKind::NotADirectory)
    }

    fn local(path: &Path) -> Result<NestedPathBuf> {
        NestedPathBuf::try_from(path).map_err(Self::path_error)
    }

    fn path_error(error: omnius_pxna_engine::model::NestedPathError) -> Error {
        Error::from_error(error, ErrorKind::InvalidInput).with_message("path cannot be represented as a nested path")
    }

    fn not_found(relative: &LibraryRelativePath) -> Error {
        Error::new(ErrorKind::NotFound).with_message(format!("entry not found: /{}", relative.segments().join("/")))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

    use omnius_core_base::{
        clock::ClockUtc,
        tsid::{TsidProvider, TsidProviderImpl},
    };
    use omnius_pxna_engine::service::{ArchiveOptions, ArchiveRuntime, LocalStorageDriver};
    use rand::{SeedableRng as _, rngs::ChaCha20Rng};
    use testresult::TestResult;

    use super::*;

    fn write_7z(path: &Path, entries: &[(&str, &[u8])]) -> TestResult {
        let mut writer = sevenz_rust2::ArchiveWriter::create(path)?;
        for (name, content) in entries {
            writer.push_archive_entry(sevenz_rust2::ArchiveEntry::new_file(name), Some(std::io::Cursor::new(content.to_vec())))?;
        }
        writer.finish()?;
        Ok(())
    }

    fn resolver(max_nested_depth: usize) -> LibraryPathResolver {
        let tsid_provider: Arc<Mutex<dyn TsidProvider + Send + Sync>> = Arc::new(Mutex::new(TsidProviderImpl::new(ClockUtc, ChaCha20Rng::seed_from_u64(0), 16)));
        let runtime = Arc::new(
            ArchiveRuntime::new(ArchiveOptions {
                max_nested_depth,
                ..ArchiveOptions::default()
            })
            .unwrap(),
        );
        let storage = Arc::new(StorageService::new(Arc::new(LocalStorageDriver::new(tsid_provider)), runtime));
        LibraryPathResolver::new(storage, max_nested_depth)
    }

    async fn resolve(resolver: &LibraryPathResolver, root: &Path, relative: &str) -> Result<Vec<String>> {
        let path = resolver.resolve(root, &LibraryRelativePath::parse(relative)?).await?;
        Ok(path.iter().map(str::to_string).collect())
    }

    #[test]
    fn library_boundary_test() {
        assert_eq!(LibraryRelativePath::parse("/a/./b/../c").unwrap().segments(), ["a", "c"]);
        assert_eq!(LibraryRelativePath::parse("/").unwrap().segments(), Vec::<String>::new());
        for escaping in ["/..", "/a/../../etc/passwd", "../x", "/C:/Windows", "/a\\b", "/a\0b"] {
            let error = LibraryRelativePath::parse(escaping).unwrap_err();
            assert_eq!(*error.kind(), ErrorKind::InvalidInput, "{escaping:?}");
        }
    }

    #[tokio::test]
    async fn fast_path_uses_one_stat_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        std::fs::create_dir_all(dir.path().join("photos").join("2026"))?;
        std::fs::write(dir.path().join("photos").join("2026").join("a.png"), b"png")?;
        let resolver = resolver(4);

        let path = resolve(&resolver, dir.path(), "/photos/2026/a.png").await?;
        assert_eq!(path, [dir.path().join("photos").join("2026").join("a.png").to_str().unwrap()]);
        assert_eq!(resolver.stat_calls.load(Ordering::SeqCst), 1);
        Ok(())
    }

    #[tokio::test]
    async fn resolves_archive_boundaries_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let inner = dir.path().join("inner.7z");
        write_7z(&inner, &[("dir/x.png", b"pixel")])?;
        let inner_bytes = std::fs::read(&inner)?;
        std::fs::create_dir_all(dir.path().join("photos"))?;
        let outer = dir.path().join("photos").join("outer.7z");
        write_7z(&outer, &[("inner.7z", &inner_bytes)])?;
        let resolver = resolver(4);

        assert_eq!(resolve(&resolver, dir.path(), "/photos/outer.7z").await?, [outer.to_str().unwrap()]);
        assert_eq!(
            resolve(&resolver, dir.path(), "/photos/outer.7z/inner.7z/dir/x.png").await?,
            [outer.to_str().unwrap(), "inner.7z", "dir/x.png"]
        );
        assert_eq!(
            resolve(&resolver, dir.path(), "/photos/outer.7z/inner.7z/dir").await?,
            [outer.to_str().unwrap(), "inner.7z", "dir"]
        );
        assert_eq!(
            resolve(&resolver, dir.path(), "/photos/outer.7z/missing.png").await.unwrap_err().kind(),
            &ErrorKind::NotFound
        );
        Ok(())
    }

    #[tokio::test]
    async fn zip_named_directory_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        std::fs::create_dir_all(dir.path().join("photos").join("a.zip"))?;
        std::fs::write(dir.path().join("photos").join("a.zip").join("x.png"), b"png")?;
        let resolver = resolver(4);

        let path = resolve(&resolver, dir.path(), "/photos/a.zip/x.png").await?;
        assert_eq!(path, [dir.path().join("photos").join("a.zip").join("x.png").to_str().unwrap()]);
        Ok(())
    }

    #[tokio::test]
    async fn max_depth_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let inner = dir.path().join("inner.7z");
        write_7z(&inner, &[("x.png", b"pixel")])?;
        let inner_bytes = std::fs::read(&inner)?;
        write_7z(&dir.path().join("outer.7z"), &[("inner.7z", &inner_bytes)])?;
        let resolver = resolver(1);

        let error = resolve(&resolver, dir.path(), "/outer.7z/inner.7z/x.png").await.unwrap_err();
        assert_eq!(*error.kind(), ErrorKind::LimitExceeded);
        Ok(())
    }

    #[tokio::test]
    async fn not_an_archive_boundary_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("notes.txt"), b"plain")?;
        let resolver = resolver(4);

        let error = resolve(&resolver, dir.path(), "/notes.txt/x").await.unwrap_err();
        assert_eq!(*error.kind(), ErrorKind::UnsupportedType);
        Ok(())
    }
}
