use std::{
    collections::{HashMap, HashSet},
    io,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicI64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use sqlx::{Row as _, sqlite::SqlitePool};

use omnius_core_migration::sqlite::{MigrationRequest, SqliteMigrator};

use crate::{core::nfs::MaterializedFile, prelude::*};

use super::identity::{ArchiveIdentity, EntryCacheKey};

const DATABASE_FILE_NAME: &str = "cache.sqlite3";
const FILES_DIR_NAME: &str = "files";
const STAGING_DIR_NAME: &str = "staging";

/// archive から取り出した entry を daemon の再起動後も再利用するための永続 LRU cache。
///
/// metadata は SQLite に、file の実体は `files/<先頭 2 hex>/<key>` に置く。
/// `MaterializedFile` の clone が生存している entry は eviction の対象から外す。
pub struct ExtractionCache {
    root: PathBuf,
    db: SqlitePool,
    capacity_bytes: u64,
    leases: Arc<CacheLeaseTable>,
    // lookup、publish、eviction を直列にし、lease を取る前に file が消える競合を防ぐ。
    lock: TokioMutex<()>,
    last_accessed: AtomicI64,
}

impl ExtractionCache {
    /// 初期化に失敗したら `root` を作り直してもう一度だけ試し、それでも失敗したら `DatabaseError` を返す。
    ///
    /// cache の中身は再生成できるため、corruption で起動できなくなるより作り直すほうを選ぶ。
    pub async fn open(root: &Path, capacity_bytes: u64) -> Result<Self> {
        let first = match Self::try_open(root, capacity_bytes).await {
            Ok(cache) => return Ok(cache),
            Err(error) => error,
        };
        warn!(root = ?root, error = ?first, "archive extraction cache is broken, recreating it");

        if let Err(error) = tokio::fs::remove_dir_all(root).await
            && error.kind() != io::ErrorKind::NotFound
        {
            return Err(Self::init_error(error));
        }
        Self::try_open(root, capacity_bytes).await.map_err(Self::init_error)
    }

    pub fn staging_dir(&self) -> PathBuf {
        self.root.join(STAGING_DIR_NAME)
    }

    /// cache hit なら lease 付きの file を返し、`last_accessed` を更新する。
    pub async fn lookup(&self, key: &EntryCacheKey, archive: &ArchiveIdentity, inner_path: &str) -> Result<Option<MaterializedFile>> {
        let _lock = self.lock.lock().await;
        self.lookup_locked(key, archive, inner_path).await
    }

    /// 検証済みの staging file を cache へ登録し、lease 付きの file を返す。
    ///
    /// 容量が足りない場合や登録に失敗した場合も、staging file を lease 専用 temp として返し、
    /// 呼び出し元の読み取りは成功させる。
    pub async fn publish(&self, key: &EntryCacheKey, archive: &ArchiveIdentity, inner_path: &str, staged: PathBuf, size: u64) -> MaterializedFile {
        if size > self.capacity_bytes {
            return MaterializedFile::owned_temp(staged);
        }

        let _lock = self.lock.lock().await;
        match self.lookup_locked(key, archive, inner_path).await {
            Ok(Some(existing)) => {
                Self::remove_file_quietly(&staged).await;
                return existing;
            }
            Ok(None) => {}
            Err(error) => {
                warn!(error = ?error, "failed to look up archive extraction cache before publish");
                return MaterializedFile::owned_temp(staged);
            }
        }

        match self.make_room(size).await {
            Ok(true) => {}
            Ok(false) => return MaterializedFile::owned_temp(staged),
            Err(error) => {
                warn!(error = ?error, "failed to evict archive extraction cache entries");
                return MaterializedFile::owned_temp(staged);
            }
        }

        let path = self.file_path(key);
        let renamed = match path.parent() {
            Some(parent) => match tokio::fs::create_dir_all(parent).await {
                Ok(()) => tokio::fs::rename(&staged, &path).await,
                Err(error) => Err(error),
            },
            None => Err(io::Error::other("cache file path has no parent")),
        };
        if let Err(error) = renamed {
            warn!(error = ?error, "failed to move staged file into archive extraction cache");
            return MaterializedFile::owned_temp(staged);
        }

        if let Err(error) = self.insert_row(key, archive, inner_path, size).await {
            // row のない file は次の起動時の reconciliation でも回収されるが、ここで lease 専用 temp にして即座に消す。
            warn!(error = ?error, "failed to register archive extraction cache entry");
            return MaterializedFile::owned_temp(path);
        }

        self.lease(key, path)
    }
}

impl ExtractionCache {
    async fn try_open(root: &Path, capacity_bytes: u64) -> Result<Self> {
        tokio::fs::create_dir_all(root.join(FILES_DIR_NAME)).await?;

        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(root.join(DATABASE_FILE_NAME))
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(10));
        let db = SqlitePool::connect_with(options).await?;
        Self::migrate(&db).await?;

        let cache = Self {
            root: root.to_path_buf(),
            db,
            capacity_bytes,
            leases: Arc::new(CacheLeaseTable::default()),
            lock: TokioMutex::new(()),
            last_accessed: AtomicI64::new(0),
        };
        cache.reconcile().await?;

        let latest: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(last_accessed), 0) FROM entries").fetch_one(&cache.db).await?;
        cache.last_accessed.store(latest, Ordering::SeqCst);

        Ok(cache)
    }

    async fn migrate(db: &SqlitePool) -> Result<()> {
        let requests = vec![MigrationRequest {
            name: "2026-09-23_archive_extraction_cache_init".to_string(),
            queries: r#"
CREATE TABLE IF NOT EXISTS entries (
    key TEXT NOT NULL PRIMARY KEY,
    archive_identity TEXT NOT NULL,
    inner_path TEXT NOT NULL,
    size INTEGER NOT NULL,
    last_accessed INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS index_entries_last_accessed ON entries(last_accessed);
"#
            .to_string(),
        }];
        SqliteMigrator::migrate(db, requests).await?;
        Ok(())
    }

    /// rename と SQLite 更新をまたぐ atomicity がないため、片側だけ残った状態を起動時に回収する。
    async fn reconcile(&self) -> Result<()> {
        let staging = self.staging_dir();
        if let Err(error) = tokio::fs::remove_dir_all(&staging).await
            && error.kind() != io::ErrorKind::NotFound
        {
            return Err(error.into());
        }
        tokio::fs::create_dir_all(&staging).await?;

        let rows: Vec<String> = sqlx::query_scalar("SELECT key FROM entries").fetch_all(&self.db).await?;
        let mut known = HashSet::new();
        for hex in rows {
            let file_exists = match EntryCacheKey::from_hex(&hex) {
                Some(key) => tokio::fs::try_exists(self.file_path(&key)).await?,
                None => false,
            };
            if file_exists {
                known.insert(hex);
            } else {
                sqlx::query("DELETE FROM entries WHERE key = ?1").bind(&hex).execute(&self.db).await?;
            }
        }

        let files_dir = self.root.join(FILES_DIR_NAME);
        let mut shards = tokio::fs::read_dir(&files_dir).await?;
        while let Some(shard) = shards.next_entry().await? {
            if !shard.file_type().await?.is_dir() {
                tokio::fs::remove_file(shard.path()).await?;
                continue;
            }
            let shard_name = shard.file_name().to_string_lossy().into_owned();
            let mut files = tokio::fs::read_dir(shard.path()).await?;
            while let Some(file) = files.next_entry().await? {
                let name = file.file_name().to_string_lossy().into_owned();
                let registered = known.contains(&name) && name.starts_with(&shard_name) && file.file_type().await?.is_file();
                if !registered {
                    Self::remove_path(&file.path()).await?;
                }
            }
        }

        Ok(())
    }

    async fn lookup_locked(&self, key: &EntryCacheKey, archive: &ArchiveIdentity, inner_path: &str) -> Result<Option<MaterializedFile>> {
        let row = sqlx::query("SELECT archive_identity, inner_path FROM entries WHERE key = ?1")
            .bind(key.to_hex())
            .fetch_optional(&self.db)
            .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let stored_archive: String = row.try_get("archive_identity")?;
        let stored_inner_path: String = row.try_get("inner_path")?;
        if stored_archive != archive.to_hex() || stored_inner_path != inner_path {
            return Ok(None);
        }

        let path = self.file_path(key);
        if !tokio::fs::try_exists(&path).await? {
            sqlx::query("DELETE FROM entries WHERE key = ?1").bind(key.to_hex()).execute(&self.db).await?;
            return Ok(None);
        }

        sqlx::query("UPDATE entries SET last_accessed = ?2 WHERE key = ?1")
            .bind(key.to_hex())
            .bind(self.next_access())
            .execute(&self.db)
            .await?;

        Ok(Some(self.lease(key, path)))
    }

    /// `incoming` を加えても容量に収まるまで、lease のない古い entry から削除する。
    ///
    /// lease 中の entry しか残らず収まらない場合は `false` を返す。
    async fn make_room(&self, incoming: u64) -> Result<bool> {
        let used: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(size), 0) FROM entries").fetch_one(&self.db).await?;
        let mut used = used as u64;
        if used + incoming <= self.capacity_bytes {
            return Ok(true);
        }

        let rows = sqlx::query("SELECT key, size FROM entries ORDER BY last_accessed ASC").fetch_all(&self.db).await?;
        for row in rows {
            let hex: String = row.try_get("key")?;
            let size: i64 = row.try_get("size")?;
            let Some(key) = EntryCacheKey::from_hex(&hex) else {
                continue;
            };
            if self.leases.is_leased(&key) {
                continue;
            }

            // file を先に消す。row だけ残っても lookup と reconciliation が miss として扱う。
            Self::remove_path(&self.file_path(&key)).await?;
            sqlx::query("DELETE FROM entries WHERE key = ?1").bind(&hex).execute(&self.db).await?;
            used = used.saturating_sub(size as u64);
            if used + incoming <= self.capacity_bytes {
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn insert_row(&self, key: &EntryCacheKey, archive: &ArchiveIdentity, inner_path: &str, size: u64) -> Result<()> {
        sqlx::query(
            r#"
INSERT OR REPLACE INTO entries (key, archive_identity, inner_path, size, last_accessed)
VALUES (?1, ?2, ?3, ?4, ?5)
"#,
        )
        .bind(key.to_hex())
        .bind(archive.to_hex())
        .bind(inner_path)
        .bind(size as i64)
        .bind(self.next_access())
        .execute(&self.db)
        .await?;
        Ok(())
    }

    fn lease(&self, key: &EntryCacheKey, path: PathBuf) -> MaterializedFile {
        MaterializedFile::guarded(path, CacheLease::acquire(self.leases.clone(), *key))
    }

    fn file_path(&self, key: &EntryCacheKey) -> PathBuf {
        let hex = key.to_hex();
        self.root.join(FILES_DIR_NAME).join(&hex[..2]).join(hex)
    }

    /// 壁時計が戻っても LRU の順序が崩れないよう、単調増加させる。
    fn next_access(&self) -> i64 {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|duration| duration.as_nanos() as i64).unwrap_or(0);
        let mut current = self.last_accessed.load(Ordering::SeqCst);
        loop {
            let next = now.max(current + 1);
            match self.last_accessed.compare_exchange(current, next, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(_) => return next,
                Err(actual) => current = actual,
            }
        }
    }

    async fn remove_path(path: &Path) -> Result<()> {
        let result = match tokio::fs::symlink_metadata(path).await {
            Ok(metadata) if metadata.is_dir() => tokio::fs::remove_dir_all(path).await,
            Ok(_) => tokio::fs::remove_file(path).await,
            Err(error) => Err(error),
        };
        match result {
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error.into()),
            _ => Ok(()),
        }
    }

    async fn remove_file_quietly(path: &Path) {
        if let Err(error) = Self::remove_path(path).await {
            warn!(path = ?path, error = ?error, "failed to remove staged file");
        }
    }

    fn init_error<E>(error: E) -> Error
    where
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        Error::from_error(error, ErrorKind::DatabaseError).with_message("failed to initialize archive extraction cache")
    }
}

/// cache entry ごとに生存している lease の数を数える。
#[derive(Default)]
struct CacheLeaseTable {
    counts: Mutex<HashMap<EntryCacheKey, usize>>,
}

impl CacheLeaseTable {
    fn is_leased(&self, key: &EntryCacheKey) -> bool {
        self.counts.lock().contains_key(key)
    }
}

/// `MaterializedFile` の lease として保持され、drop されると lease を返す。
struct CacheLease {
    table: Arc<CacheLeaseTable>,
    key: EntryCacheKey,
}

impl CacheLease {
    fn acquire(table: Arc<CacheLeaseTable>, key: EntryCacheKey) -> Self {
        *table.counts.lock().entry(key).or_insert(0) += 1;
        Self { table, key }
    }
}

impl Drop for CacheLease {
    fn drop(&mut self) {
        let mut counts = self.table.counts.lock();
        if let Some(count) = counts.get_mut(&self.key) {
            *count -= 1;
            if *count == 0 {
                counts.remove(&self.key);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use testresult::TestResult;

    use super::*;
    use crate::core::nfs::SourceFingerprint;

    fn archive() -> ArchiveIdentity {
        ArchiveIdentity::from_local("/library/a.zip", &SourceFingerprint { size: 1, mtime_s: 0, mtime_ns: 0 })
    }

    async fn stage(cache: &ExtractionCache, content: &[u8]) -> TestResult<PathBuf> {
        let file = tempfile::NamedTempFile::new_in(cache.staging_dir())?;
        let path = file.into_temp_path().keep()?;
        tokio::fs::write(&path, content).await?;
        Ok(path)
    }

    async fn publish(cache: &ExtractionCache, inner_path: &str, content: &[u8]) -> TestResult<MaterializedFile> {
        let staged = stage(cache, content).await?;
        let key = EntryCacheKey::new(&archive(), inner_path);
        Ok(cache.publish(&key, &archive(), inner_path, staged, content.len() as u64).await)
    }

    async fn row_count(cache: &ExtractionCache) -> TestResult<i64> {
        Ok(sqlx::query_scalar("SELECT COUNT(*) FROM entries").fetch_one(&cache.db).await?)
    }

    #[tokio::test]
    async fn publish_and_lookup_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let cache = ExtractionCache::open(dir.path(), 1024).await?;

        let published = publish(&cache, "a.txt", b"alpha").await?;
        assert!(published.path().starts_with(dir.path().join(FILES_DIR_NAME)));
        drop(published);

        let key = EntryCacheKey::new(&archive(), "a.txt");
        let hit = cache.lookup(&key, &archive(), "a.txt").await?.expect("cache hit");
        assert_eq!(tokio::fs::read(hit.path()).await?, b"alpha");
        assert!(cache.lookup(&key, &archive(), "b.txt").await?.is_none(), "元 field が一致しなければ miss とする");
        Ok(())
    }

    #[tokio::test]
    async fn survives_reopen_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        {
            let cache = ExtractionCache::open(dir.path(), 1024).await?;
            drop(publish(&cache, "a.txt", b"alpha").await?);
            cache.db.close().await;
        }

        let cache = ExtractionCache::open(dir.path(), 1024).await?;
        let key = EntryCacheKey::new(&archive(), "a.txt");
        assert!(cache.lookup(&key, &archive(), "a.txt").await?.is_some());
        Ok(())
    }

    #[tokio::test]
    async fn lease_survives_eviction_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let cache = ExtractionCache::open(dir.path(), 10).await?;

        let leased = publish(&cache, "leased", b"aaaa").await?;
        let old = publish(&cache, "old", b"bbbb").await?;
        let old_path = old.path().to_path_buf();
        drop(old);

        // leased が最も古いが lease 中なので、次に古い old が消える。
        let newer = publish(&cache, "newer", b"cccc").await?;
        assert!(leased.path().exists(), "lease 中の entry は消さない");
        assert!(!old_path.exists(), "lease のない古い entry から消す");
        assert!(newer.path().starts_with(dir.path().join(FILES_DIR_NAME)));
        assert_eq!(row_count(&cache).await?, 2);
        Ok(())
    }

    #[tokio::test]
    async fn all_leased_returns_temp_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let cache = ExtractionCache::open(dir.path(), 8).await?;

        let first = publish(&cache, "first", b"aaaa").await?;
        let second = publish(&cache, "second", b"bbbb").await?;
        let third = publish(&cache, "third", b"cccc").await?;
        assert!(first.path().exists() && second.path().exists());
        assert!(third.path().starts_with(cache.staging_dir()), "全候補が lease 中なら lease 専用 temp にする");
        assert_eq!(tokio::fs::read(third.path()).await?, b"cccc");

        let third_path = third.path().to_path_buf();
        drop(third);
        assert!(!third_path.exists());
        assert_eq!(row_count(&cache).await?, 2);
        Ok(())
    }

    #[tokio::test]
    async fn oversized_entry_is_not_registered_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let cache = ExtractionCache::open(dir.path(), 4).await?;

        let materialized = publish(&cache, "large", b"too large").await?;
        assert_eq!(tokio::fs::read(materialized.path()).await?, b"too large");
        assert_eq!(row_count(&cache).await?, 0);
        Ok(())
    }

    #[tokio::test]
    async fn init_failure_recreates_once_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let root = dir.path().join("archive");
        {
            let cache = ExtractionCache::open(&root, 1024).await?;
            drop(publish(&cache, "a.txt", b"alpha").await?);
            cache.db.close().await;
        }
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(root.join(format!("{DATABASE_FILE_NAME}{suffix}")));
        }
        std::fs::write(root.join(DATABASE_FILE_NAME), b"this is not a sqlite database, just some garbage bytes to corrupt it")?;

        let cache = ExtractionCache::open(&root, 1024).await?;
        assert_eq!(row_count(&cache).await?, 0, "作り直した cache は空である");

        // root の親が file だと、作り直しても失敗する。
        let blocker = dir.path().join("blocker");
        std::fs::write(&blocker, b"")?;
        let error = ExtractionCache::open(&blocker.join("archive"), 1024).await.err().expect("open must fail");
        assert_eq!(error.kind(), &ErrorKind::DatabaseError);
        Ok(())
    }

    #[tokio::test]
    async fn reconciliation_removes_leftovers_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let (row_only, file_only, staged) = {
            let cache = ExtractionCache::open(dir.path(), 1024).await?;

            let row_only = publish(&cache, "row-only", b"row").await?.path().to_path_buf();
            std::fs::remove_file(&row_only)?;

            let orphan_key = EntryCacheKey::new(&archive(), "file-only");
            let file_only = cache.file_path(&orphan_key);
            std::fs::create_dir_all(file_only.parent().unwrap())?;
            std::fs::write(&file_only, b"file")?;

            let staged = stage(&cache, b"staged").await?;
            cache.db.close().await;
            (row_only, file_only, staged)
        };

        let cache = ExtractionCache::open(dir.path(), 1024).await?;
        assert_eq!(row_count(&cache).await?, 0, "file のない row を消す");
        assert!(!row_only.exists());
        assert!(!file_only.exists(), "row のない file を消す");
        assert!(!staged.exists(), "残った staging を消す");
        assert!(cache.staging_dir().exists());
        Ok(())
    }
}
