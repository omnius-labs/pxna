use std::{
    io,
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use tokio::{
    fs::File as TokioFile,
    io::{AsyncRead, AsyncSeek, ReadBuf},
};

use crate::prelude::*;

/// 物理 file を指す handle と、その file を生かしておく lease を一体で所有する。
///
/// local driver は元 file の path と no-op lease を返し、archive driver は cache file または
/// lease 専用 temp を返す。lease は `Arc` の生存で表すため、clone が 1 つでも残る間は
/// 対象の file が消えない。
#[derive(Debug, Clone)]
pub struct MaterializedFile {
    path: PathBuf,
    #[allow(unused)]
    lease: Arc<Lease>,
}

impl MaterializedFile {
    /// 元 file をそのまま指す。lease は何もしない。
    pub fn borrowed<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            lease: Arc::new(Lease::None),
        }
    }

    /// この handle が所有する temp file を指す。最後の clone が drop された時点で削除する。
    pub fn owned_temp<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref().to_path_buf();
        Self {
            lease: Arc::new(Lease::Temp { path: path.clone() }),
            path,
        }
    }

    /// `guard` が生きている間だけ file を残すことを、呼び出し側が保証する file を指す。
    ///
    /// extraction cache は、guard の生存を eviction 対象から外す印として使う。
    pub(crate) fn guarded<P: AsRef<Path>, G: Send + Sync + 'static>(path: P, guard: G) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            lease: Arc::new(Lease::Guard(Box::new(guard))),
        }
    }

    pub(crate) fn keep_temp(file: tempfile::NamedTempFile) -> Result<Self> {
        let path = file.into_temp_path().keep().map_err(|error| Error::from(error.error))?;
        Ok(Self::owned_temp(path))
    }

    pub(crate) fn empty_temp() -> Result<Self> {
        Self::keep_temp(tempfile::NamedTempFile::new()?)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub async fn open(&self) -> Result<OpenedFile> {
        let file = TokioFile::open(&self.path).await?;
        Ok(OpenedFile { file, materialized: self.clone() })
    }
}

enum Lease {
    None,
    Temp { path: PathBuf },
    Guard(#[allow(unused)] Box<dyn std::any::Any + Send + Sync>),
}

impl std::fmt::Debug for Lease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Lease::None => write!(f, "None"),
            Lease::Temp { path } => f.debug_struct("Temp").field("path", path).finish(),
            Lease::Guard(_) => write!(f, "Guard"),
        }
    }
}

impl Drop for Lease {
    fn drop(&mut self) {
        let Lease::Temp { path } = self else {
            return;
        };
        if let Err(e) = std::fs::remove_file(&*path)
            && e.kind() != io::ErrorKind::NotFound
        {
            warn!(path = ?path, error = ?e, "failed to remove leased temp file");
        }
    }
}

/// 開いた物理 file と、その file の lease を同時に保持する。
///
/// file handle が開いている間は lease も残るため、Windows を含めて利用中の cache file を削除しない。
#[derive(Debug)]
pub struct OpenedFile {
    file: TokioFile,
    materialized: MaterializedFile,
}

impl OpenedFile {
    pub fn materialized(&self) -> &MaterializedFile {
        &self.materialized
    }

    pub async fn fingerprint(&self) -> Result<super::SourceFingerprint> {
        super::SourceFingerprint::from_metadata(&self.file.metadata().await?)
    }

    pub async fn len(&self) -> Result<u64> {
        Ok(self.file.metadata().await?.len())
    }
}

impl AsyncRead for OpenedFile {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.file).poll_read(cx, buf)
    }
}

impl AsyncSeek for OpenedFile {
    fn start_seek(mut self: Pin<&mut Self>, position: io::SeekFrom) -> io::Result<()> {
        Pin::new(&mut self.file).start_seek(position)
    }

    fn poll_complete(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<u64>> {
        Pin::new(&mut self.file).poll_complete(cx)
    }
}

#[cfg(test)]
mod tests {
    use testresult::TestResult;
    use tokio::io::AsyncReadExt as _;

    use super::*;

    #[tokio::test]
    async fn borrowed_lease_keeps_file_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("a.txt");
        tokio::fs::write(&path, b"hello").await?;

        let a = MaterializedFile::borrowed(&path);
        let b = a.clone();
        drop(a);
        assert!(path.exists());
        drop(b);
        assert!(path.exists());

        Ok(())
    }

    #[tokio::test]
    async fn owned_temp_is_removed_after_last_clone_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("a.tmp");
        tokio::fs::write(&path, b"hello").await?;

        let a = MaterializedFile::owned_temp(&path);
        let b = a.clone();
        drop(a);
        assert!(path.exists(), "clone が残っている間は削除しない");
        drop(b);
        assert!(!path.exists(), "最後の clone が drop されたら削除する");

        Ok(())
    }

    #[tokio::test]
    async fn opened_file_holds_lease_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("a.tmp");
        tokio::fs::write(&path, b"hello").await?;

        let materialized = MaterializedFile::owned_temp(&path);
        let mut opened = materialized.open().await?;
        drop(materialized);
        assert!(path.exists(), "OpenedFile が lease を保持している");

        let mut buf = String::new();
        opened.read_to_string(&mut buf).await?;
        assert_eq!(buf, "hello");

        drop(opened);
        assert!(!path.exists());

        Ok(())
    }
}
