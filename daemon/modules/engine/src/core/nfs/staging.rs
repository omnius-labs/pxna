use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};

use chrono::{DateTime, Utc};
use tokio::{
    fs::File as TokioFile,
    io::{AsyncWriteExt as _, BufWriter},
};

use crate::{
    core::nfs::rename::{RenameOutcome, rename_no_replace},
    prelude::*,
};

/// staging file の名前の接頭辞。先頭がドットであり、`includeHidden=false` の既定では listing に現れない。
pub const STAGING_PREFIX: &str = ".pxna-tmp-";

/// 孤児と見なすまでの経過時間。
pub const STAGING_MAX_AGE: std::time::Duration = std::time::Duration::from_secs(24 * 60 * 60);

/// 候補名の上限。これを超えたら `LimitExceeded` とする。
const MAX_RENAME_CANDIDATES: u32 = 10_000;

/// このプロセスが保持している生存 staging の集合。
///
/// 孤児回収は、この集合に無いものだけを対象にする。
pub type LiveStagingSet = Arc<Mutex<HashSet<PathBuf>>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictPolicy {
    Fail,
    Skip,
    Overwrite,
    Rename,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Publish {
    Published { dst: String },
    Skipped,
}

/// destination と同じ directory に作られた未公開の書き込み先。
///
/// 同じ directory に置くのは、Library 内部に別 filesystem の mount があっても rename が
/// 同一 filesystem 内で完結するためである。drop で削除されるため、`publish` を通らなかった
/// 内容が destination に現れることはない。
pub struct StagingFile {
    path: PathBuf,
    dir: PathBuf,
    file: Option<BufWriter<TokioFile>>,
    len: u64,
    live: LiveStagingSet,
    published: bool,
}

impl StagingFile {
    pub async fn create(dir: &Path, tsid: &str, live: LiveStagingSet) -> Result<Self> {
        let path = dir.join(format!("{STAGING_PREFIX}{tsid}"));

        let file = TokioFile::options().write(true).create_new(true).open(&path).await?;
        live.lock().insert(path.clone());

        Ok(Self {
            path,
            dir: dir.to_path_buf(),
            file: Some(BufWriter::new(file)),
            len: 0,
            live,
            published: false,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// 追記のみ。offset 指定のランダム書き込みは持たない。
    pub async fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        let file = self
            .file
            .as_mut()
            .ok_or_else(|| Error::new(ErrorKind::UnexpectedError).with_message("staging file is closed"))?;
        file.write_all(buf).await?;
        self.len += buf.len() as u64;
        Ok(())
    }

    pub async fn set_modified(&mut self, t: DateTime<Utc>) -> Result<()> {
        let file = self
            .file
            .as_mut()
            .ok_or_else(|| Error::new(ErrorKind::UnexpectedError).with_message("staging file is closed"))?;
        file.flush().await?;

        let path = self.path.clone();
        let time = SystemTime::from(t);
        tokio::task::spawn_blocking(move || -> std::io::Result<()> { std::fs::File::options().write(true).open(&path)?.set_modified(time) }).await??;

        Ok(())
    }

    /// conflict policy を適用して destination へ移す唯一の経路。
    pub async fn publish(mut self, dst: &str, conflict: ConflictPolicy) -> Result<Publish> {
        let dst_path = PathBuf::from(dst);
        if dst_path.parent() != Some(self.dir.as_path()) {
            return Err(Error::new(ErrorKind::InvalidInput).with_message(format!("destination is outside of the staging directory: {dst}")));
        }

        self.close().await?;

        let src = self.path.clone();
        let result = StagingPublisher::apply(src, dst_path, conflict).await?;

        // rename が成功した時点で staging file は存在しない。drop による削除を止める。
        if !matches!(result, Publish::Skipped) {
            self.published = true;
        }

        Ok(result)
    }

    async fn close(&mut self) -> Result<()> {
        let Some(mut file) = self.file.take() else {
            return Ok(());
        };
        file.flush().await?;
        Ok(())
    }
}

impl Drop for StagingFile {
    fn drop(&mut self) {
        // Windows では open な handle が残っていると削除できないため、先に閉じる。
        drop(self.file.take());

        if !self.published
            && let Err(e) = std::fs::remove_file(&self.path)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            warn!(path = ?self.path, error = ?e, "failed to remove staging file");
        }

        self.live.lock().remove(&self.path);
    }
}

/// conflict policy に従って staging または同一 filesystem 内の file を公開する。
pub struct StagingPublisher;

impl StagingPublisher {
    pub async fn apply(src: PathBuf, dst: PathBuf, conflict: ConflictPolicy) -> Result<Publish> {
        tokio::task::spawn_blocking(move || Self::apply_blocking(&src, &dst, conflict)).await?
    }

    fn apply_blocking(src: &Path, dst: &Path, conflict: ConflictPolicy) -> Result<Publish> {
        match conflict {
            ConflictPolicy::Fail => match rename_no_replace(src, dst)? {
                RenameOutcome::Renamed => Ok(Publish::Published { dst: Self::to_string(dst)? }),
                RenameOutcome::AlreadyExists => Err(Error::new(ErrorKind::AlreadyExists).with_message(format!("destination already exists: {}", dst.display()))),
            },
            ConflictPolicy::Skip => match rename_no_replace(src, dst)? {
                RenameOutcome::Renamed => Ok(Publish::Published { dst: Self::to_string(dst)? }),
                RenameOutcome::AlreadyExists => Ok(Publish::Skipped),
            },
            ConflictPolicy::Overwrite => {
                // 既存 destination を先に削除せず、OS の atomic replace に任せる。
                std::fs::rename(src, dst)?;
                Ok(Publish::Published { dst: Self::to_string(dst)? })
            }
            ConflictPolicy::Rename => Self::publish_with_rename(src, dst),
        }
    }

    fn publish_with_rename(src: &Path, dst: &Path) -> Result<Publish> {
        if rename_no_replace(src, dst)? == RenameOutcome::Renamed {
            return Ok(Publish::Published { dst: Self::to_string(dst)? });
        }

        let dir = dst
            .parent()
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput).with_message(format!("destination has no parent: {}", dst.display())))?;
        let name = dst
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput).with_message(format!("destination has no file name: {}", dst.display())))?;

        for n in 1..=MAX_RENAME_CANDIDATES {
            let candidate = dir.join(Self::candidate_name(name, n));
            if rename_no_replace(src, &candidate)? == RenameOutcome::Renamed {
                return Ok(Publish::Published {
                    dst: Self::to_string(&candidate)?,
                });
            }
        }

        Err(Error::new(ErrorKind::LimitExceeded).with_message(format!("exhausted {MAX_RENAME_CANDIDATES} rename candidates for {}", dst.display())))
    }

    /// `photo.jpg` から `photo (1).jpg` を作る。
    fn candidate_name(name: &str, n: u32) -> String {
        match name.rfind('.').filter(|&i| i > 0) {
            Some(i) => format!("{} ({}){}", &name[..i], n, &name[i..]),
            None => format!("{name} ({n})"),
        }
    }

    fn to_string(path: &Path) -> Result<String> {
        path.to_str()
            .map(str::to_string)
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput).with_message(format!("path is not valid UTF-8: {}", path.display())))
    }
}

/// `dir` に残った孤児 staging を削除する。
///
/// このプロセスが保持している staging と、`older_than` より新しいものは対象にしない。
/// 走査は 1 directory につき 1 回であり、staging を作るたびには行わない。
pub struct StagingSweeper;

impl StagingSweeper {
    pub async fn sweep(dir: &Path, live: &LiveStagingSet, older_than: SystemTime) -> Result<u64> {
        let mut read_dir = match tokio::fs::read_dir(dir).await {
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(e) => return Err(e.into()),
        };

        let mut removed = 0;
        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();

            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !name.starts_with(STAGING_PREFIX) {
                continue;
            }
            if live.lock().contains(&path) {
                continue;
            }

            let meta = match entry.metadata().await {
                Ok(n) => n,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e.into()),
            };
            if !meta.is_file() {
                continue;
            }
            if meta.modified().map(|m| m >= older_than).unwrap_or(true) {
                continue;
            }

            match tokio::fs::remove_file(&path).await {
                Ok(()) => removed += 1,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
        }

        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use testresult::TestResult;

    use super::*;

    fn live() -> LiveStagingSet {
        Arc::new(Mutex::new(HashSet::new()))
    }

    async fn staged(dir: &Path, content: &[u8]) -> Result<StagingFile> {
        let mut staging = StagingFile::create(dir, "0001", live()).await?;
        staging.write_all(content).await?;
        Ok(staging)
    }

    #[tokio::test]
    async fn publish_fail_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let dst = dir.path().join("photo.jpg");

        let staging = staged(dir.path(), b"new").await?;
        let published = staging.publish(dst.to_str().unwrap(), ConflictPolicy::Fail).await?;
        assert_eq!(
            published,
            Publish::Published {
                dst: dst.to_str().unwrap().to_string()
            }
        );
        assert_eq!(tokio::fs::read(&dst).await?, b"new");

        let staging = staged(dir.path(), b"newer").await?;
        let e = staging.publish(dst.to_str().unwrap(), ConflictPolicy::Fail).await.unwrap_err();
        assert_eq!(*e.kind(), ErrorKind::AlreadyExists);
        assert_eq!(tokio::fs::read(&dst).await?, b"new", "既存を壊さない");

        Ok(())
    }

    #[tokio::test]
    async fn publish_skip_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let dst = dir.path().join("photo.jpg");

        let staging = staged(dir.path(), b"new").await?;
        assert_eq!(
            staging.publish(dst.to_str().unwrap(), ConflictPolicy::Skip).await?,
            Publish::Published {
                dst: dst.to_str().unwrap().to_string()
            }
        );

        let staging = staged(dir.path(), b"newer").await?;
        let staging_path = staging.path().to_path_buf();
        assert_eq!(staging.publish(dst.to_str().unwrap(), ConflictPolicy::Skip).await?, Publish::Skipped);
        assert_eq!(tokio::fs::read(&dst).await?, b"new", "既存に触れない");
        assert!(!staging_path.exists(), "skip した staging は削除される");

        Ok(())
    }

    #[tokio::test]
    async fn publish_overwrite_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let dst = dir.path().join("photo.jpg");

        let staging = staged(dir.path(), b"new").await?;
        staging.publish(dst.to_str().unwrap(), ConflictPolicy::Overwrite).await?;

        let staging = staged(dir.path(), b"newer").await?;
        assert_eq!(
            staging.publish(dst.to_str().unwrap(), ConflictPolicy::Overwrite).await?,
            Publish::Published {
                dst: dst.to_str().unwrap().to_string()
            }
        );
        assert_eq!(tokio::fs::read(&dst).await?, b"newer");

        Ok(())
    }

    #[tokio::test]
    async fn publish_rename_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let dst = dir.path().join("photo.jpg");

        let staging = staged(dir.path(), b"a").await?;
        assert_eq!(
            staging.publish(dst.to_str().unwrap(), ConflictPolicy::Rename).await?,
            Publish::Published {
                dst: dst.to_str().unwrap().to_string()
            }
        );

        let staging = staged(dir.path(), b"b").await?;
        let expected = dir.path().join("photo (1).jpg");
        assert_eq!(
            staging.publish(dst.to_str().unwrap(), ConflictPolicy::Rename).await?,
            Publish::Published {
                dst: expected.to_str().unwrap().to_string()
            }
        );
        assert_eq!(tokio::fs::read(&dst).await?, b"a");
        assert_eq!(tokio::fs::read(&expected).await?, b"b");

        let staging = staged(dir.path(), b"c").await?;
        let expected = dir.path().join("photo (2).jpg");
        assert_eq!(
            staging.publish(dst.to_str().unwrap(), ConflictPolicy::Rename).await?,
            Publish::Published {
                dst: expected.to_str().unwrap().to_string()
            }
        );

        Ok(())
    }

    #[test]
    fn candidate_name_test() {
        assert_eq!(StagingPublisher::candidate_name("photo.jpg", 1), "photo (1).jpg");
        assert_eq!(StagingPublisher::candidate_name("folder", 1), "folder (1)");
        assert_eq!(StagingPublisher::candidate_name(".profile", 1), ".profile (1)");
        assert_eq!(StagingPublisher::candidate_name("archive.tar.gz", 3), "archive.tar (3).gz");
    }

    #[tokio::test]
    async fn staging_location_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let dst = dir.path().join("photo.jpg");

        let mut staging = StagingFile::create(dir.path(), "0001", live()).await?;
        staging.write_all(b"new").await?;

        let staging_path = staging.path().to_path_buf();
        assert_eq!(staging_path.parent(), Some(dir.path()), "destination と同じ directory に作る");
        assert!(staging_path.file_name().and_then(|n| n.to_str()).unwrap().starts_with(STAGING_PREFIX));
        assert!(!dst.exists(), "publish 前に destination へ現れない");

        staging.publish(dst.to_str().unwrap(), ConflictPolicy::Fail).await?;
        assert!(dst.exists());
        assert!(!staging_path.exists());

        Ok(())
    }

    #[tokio::test]
    async fn staging_is_removed_on_drop_test() -> TestResult {
        let dir = tempfile::tempdir()?;

        let staging = staged(dir.path(), b"new").await?;
        let staging_path = staging.path().to_path_buf();
        assert!(staging_path.exists());

        drop(staging);
        assert!(!staging_path.exists());

        Ok(())
    }

    #[tokio::test]
    async fn publish_rejects_destination_outside_of_directory_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let other = dir.path().join("sub");
        tokio::fs::create_dir(&other).await?;

        let staging = staged(dir.path(), b"new").await?;
        let dst = other.join("photo.jpg");
        let e = staging.publish(dst.to_str().unwrap(), ConflictPolicy::Fail).await.unwrap_err();
        assert_eq!(*e.kind(), ErrorKind::InvalidInput);

        Ok(())
    }

    #[tokio::test]
    async fn sweep_removes_only_old_orphans_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let live = live();

        // 生存中の staging
        let alive = StagingFile::create(dir.path(), "alive", live.clone()).await?;
        let alive_path = alive.path().to_path_buf();

        // 古い孤児
        let old = dir.path().join(format!("{STAGING_PREFIX}old"));
        tokio::fs::write(&old, b"x").await?;
        std::fs::File::options()
            .write(true)
            .open(&old)?
            .set_modified(SystemTime::now() - STAGING_MAX_AGE - Duration::from_secs(60))?;

        // 新しい孤児
        let fresh = dir.path().join(format!("{STAGING_PREFIX}fresh"));
        tokio::fs::write(&fresh, b"x").await?;

        // staging ではない file
        let normal = dir.path().join("photo.jpg");
        tokio::fs::write(&normal, b"x").await?;

        let removed = StagingSweeper::sweep(dir.path(), &live, SystemTime::now() - STAGING_MAX_AGE).await?;
        assert_eq!(removed, 1);
        assert!(!old.exists(), "古い孤児を削除する");
        assert!(fresh.exists(), "新しい孤児は残す");
        assert!(alive_path.exists(), "生存中の staging は残す");
        assert!(normal.exists(), "staging でない file には触れない");

        drop(alive);
        Ok(())
    }
}
