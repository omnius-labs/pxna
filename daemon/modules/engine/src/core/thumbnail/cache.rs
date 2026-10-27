use std::{
    collections::{HashMap, HashSet},
    io,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex as StdMutex,
        atomic::{AtomicBool, AtomicI64, Ordering},
    },
    time::Duration,
};

use sqlx::sqlite::SqlitePool;
use tokio_util::bytes::Bytes;
use tracing::Instrument as _;

use omnius_core_migration::sqlite::{MigrationRequest, SqliteMigrator};

use crate::{perf_span, prelude::*};

use super::key::ThumbnailKey;

const DATABASE_FILE_NAME: &str = "cache.sqlite3";
const FILES_DIR_NAME: &str = "files";
const STAGING_DIR_NAME: &str = "staging";
/// 利用順の更新を溜めてから索引へ書くまでの待ち時間。
const TOUCH_FLUSH_DELAY: Duration = Duration::from_millis(500);

/// キャッシュの容量枠。サムネイルとスプライトシートは小さく、表示用画像は 1 桁大きいため、枠を分けて互いを追い出さないようにする。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheBucket {
    Thumbnail,
    Display,
}

impl CacheBucket {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Thumbnail => "thumbnail",
            Self::Display => "display",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThumbnailCacheOptions {
    pub thumbnail_capacity_bytes: u64,
    pub display_capacity_bytes: u64,
}

impl ThumbnailCacheOptions {
    fn capacity(&self, bucket: CacheBucket) -> u64 {
        match bucket {
            CacheBucket::Thumbnail => self.thumbnail_capacity_bytes,
            CacheBucket::Display => self.display_capacity_bytes,
        }
    }

    fn validate(&self) -> Result<()> {
        if self.thumbnail_capacity_bytes == 0 || self.display_capacity_bytes == 0 {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("thumbnail cache capacities must be greater than zero"));
        }
        Ok(())
    }
}

/// キャッシュから読み出した画像。スプライトシートだけが `frame_count` を持つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedImage {
    pub bytes: Bytes,
    pub frame_count: Option<u32>,
}

/// 生成したサムネイルを再起動後も再利用するための永続 LRU キャッシュ。
///
/// 索引は SQLite に、画像の実体は `files/<先頭 2 hex>/<key>` に置く。
/// 古いものは容量枠ごとに `last_accessed` の小さい順に削除し、TTL は持たない。
pub struct ThumbnailCache {
    root: PathBuf,
    db: SqlitePool,
    options: ThumbnailCacheOptions,
    // 書き込みと eviction を直列にし、索引と実ファイルの食い違いを作らない。
    lock: Arc<TokioMutex<()>>,
    last_accessed: AtomicI64,
    // 読み出しの利用順は応答を待たせず、まとめて索引へ反映する。
    pending_touches: Arc<PendingTouches>,
}

#[derive(Default)]
struct PendingTouches {
    entries: StdMutex<HashMap<String, i64>>,
    scheduled: AtomicBool,
}

impl ThumbnailCache {
    /// 初期化に失敗したら `root` を作り直してもう一度だけ試し、それでも失敗したら `DatabaseError` を返す。
    ///
    /// キャッシュの中身は再生成できるため、破損で起動できなくなるより作り直すほうを選ぶ。
    pub async fn open(root: &Path, options: ThumbnailCacheOptions) -> Result<Self> {
        options.validate()?;

        let first = match Self::try_open(root, options).await {
            Ok(cache) => return Ok(cache),
            Err(error) => error,
        };
        warn!(root = ?root, error = ?first, "thumbnail cache is broken, recreating it");

        if let Err(error) = tokio::fs::remove_dir_all(root).await
            && error.kind() != io::ErrorKind::NotFound
        {
            return Err(Self::init_error(error));
        }
        Self::try_open(root, options).await.map_err(Self::init_error)
    }

    pub async fn get(&self, key: &ThumbnailKey) -> Result<Option<CachedImage>> {
        let row: Option<(Option<i64>,)> = sqlx::query_as("SELECT frame_count FROM entries WHERE key = ?1")
            .bind(key.hex())
            .fetch_optional(&self.db)
            .instrument(perf_span!("thumbnail_cache_lookup"))
            .await?;
        let Some((frame_count,)) = row else {
            return Ok(None);
        };

        let bytes = match tokio::fs::read(self.file_path(key)).instrument(perf_span!("thumbnail_cache_read")).await {
            Ok(bytes) => Bytes::from(bytes),
            // eviction と競合して実ファイルが先に消えた場合は、ない扱いにする。
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };

        self.touch_later(key);
        Ok(Some(CachedImage {
            bytes,
            frame_count: frame_count.map(|count| count as u32),
        }))
    }

    /// Whether the index has the key. It does not read the file or update the recency, so a caller can skip work that is already cached.
    pub async fn contains(&self, key: &ThumbnailKey) -> Result<bool> {
        let row: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM entries WHERE key = ?1")
            .bind(key.hex())
            .fetch_optional(&self.db)
            .instrument(perf_span!("thumbnail_cache_contains"))
            .await?;
        Ok(row.is_some())
    }

    /// Removes the index row of a key, so that a test can make a cached entry missing.
    #[cfg(test)]
    pub(super) async fn forget(&self, key: &ThumbnailKey) -> Result<()> {
        sqlx::query("DELETE FROM entries WHERE key = ?1").bind(key.hex()).execute(&self.db).await?;
        Ok(())
    }

    /// 枠の容量を 1 件で超える画像は、キャッシュせずに `false` を返す。
    /// The generator stores in batches with `put_many`, so only tests use this single-entry form.
    #[cfg(test)]
    pub async fn put(&self, key: &ThumbnailKey, bucket: CacheBucket, image: &CachedImage) -> Result<bool> {
        Ok(self.put_many(&[(key, bucket, image)]).await?[0])
    }

    /// Stores several entries with one lock, one recency flush, one transaction, and one eviction per bucket.
    ///
    /// `last_accessed` follows the input order, so the last entries are the newest and a small bucket evicts the first ones.
    /// The result has one flag per input entry: `false` for an entry that alone exceeds its bucket, which is skipped.
    /// An I/O or SQL error fails the call; files already staged are removed, and files already published are reconciled on the next open.
    pub async fn put_many(&self, entries: &[(&ThumbnailKey, CacheBucket, &CachedImage)]) -> Result<Vec<bool>> {
        let storable: Vec<bool> = entries
            .iter()
            .map(|(_, bucket, image)| image.bytes.len() as u64 <= self.options.capacity(*bucket))
            .collect();

        // 実ファイルは lock の外で staging に書き、公開と索引の更新だけを lock の中で行う。
        let mut staged = Vec::new();
        for ((_, _, image), storable) in entries.iter().zip(&storable) {
            if !storable {
                continue;
            }
            let path = self.root.join(STAGING_DIR_NAME).join(uuid::Uuid::new_v4().to_string());
            staged.push(path.clone());
            if let Err(error) = tokio::fs::write(&path, &image.bytes).await {
                Self::remove_staged(&staged).await;
                return Err(error.into());
            }
        }
        if staged.is_empty() {
            return Ok(storable);
        }

        let published = self.publish_staged(entries, &storable, &staged).await;
        if published.is_err() {
            Self::remove_staged(&staged).await;
        }
        published?;
        Ok(storable)
    }

    async fn remove_staged(staged: &[PathBuf]) {
        for path in staged {
            let _ = tokio::fs::remove_file(path).await;
        }
    }

    async fn publish_staged(&self, entries: &[(&ThumbnailKey, CacheBucket, &CachedImage)], storable: &[bool], staged: &[PathBuf]) -> Result<()> {
        let _guard = self.lock.lock().await;
        // eviction が最新の利用順で選ぶよう、溜めた更新を先に反映する。
        Self::flush_touches(&self.db, &self.pending_touches).await?;

        let stored = entries.iter().zip(storable).filter(|(_, storable)| **storable).map(|(entry, _)| entry);
        for ((key, _, _), staged) in stored.clone().zip(staged) {
            let published = self.file_path(key);
            if let Some(parent) = published.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::rename(staged, &published).await?;
        }

        let mut transaction = self.db.begin().await?;
        let mut buckets = Vec::new();
        for (key, bucket, image) in stored {
            sqlx::query(
                "INSERT INTO entries (key, bucket, size, frame_count, last_accessed) VALUES (?1, ?2, ?3, ?4, ?5) \
                 ON CONFLICT(key) DO UPDATE SET bucket = ?2, size = ?3, frame_count = ?4, last_accessed = ?5",
            )
            .bind(key.hex())
            .bind(bucket.as_str())
            .bind(image.bytes.len() as i64)
            .bind(image.frame_count.map(i64::from))
            .bind(self.next_access())
            .execute(&mut *transaction)
            .await?;
            if !buckets.contains(bucket) {
                buckets.push(*bucket);
            }
        }
        transaction.commit().await?;

        for bucket in buckets {
            self.evict(bucket).await?;
        }
        Ok(())
    }
}

impl ThumbnailCache {
    async fn try_open(root: &Path, options: ThumbnailCacheOptions) -> Result<Self> {
        tokio::fs::create_dir_all(root.join(FILES_DIR_NAME)).await?;

        let connect_options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(root.join(DATABASE_FILE_NAME))
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            // The cache is regenerable, so a power loss may lose the latest entries; reconcile drops the rows whose files are gone.
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
            .busy_timeout(std::time::Duration::from_secs(10));
        let db = SqlitePool::connect_with(connect_options).await?;
        Self::migrate(&db).await?;

        let cache = Self {
            root: root.to_path_buf(),
            db,
            options,
            lock: Arc::new(TokioMutex::new(())),
            last_accessed: AtomicI64::new(0),
            pending_touches: Arc::default(),
        };
        cache.reconcile().await?;

        let latest: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(last_accessed), 0) FROM entries").fetch_one(&cache.db).await?;
        cache.last_accessed.store(latest, Ordering::SeqCst);

        Ok(cache)
    }

    async fn migrate(db: &SqlitePool) -> Result<()> {
        let requests = vec![MigrationRequest {
            name: "2026-10-01_thumbnail_cache_init".to_string(),
            queries: r#"
CREATE TABLE IF NOT EXISTS entries (
    key TEXT NOT NULL PRIMARY KEY,
    bucket TEXT NOT NULL CHECK (bucket IN ('thumbnail', 'display')),
    size INTEGER NOT NULL,
    frame_count INTEGER,
    last_accessed INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS index_entries_bucket_last_accessed ON entries(bucket, last_accessed);
"#
            .to_string(),
        }];
        SqliteMigrator::migrate(db, requests).await?;
        Ok(())
    }

    /// rename と SQLite 更新をまたぐ atomicity がないため、片側だけ残った状態を起動時に回収する。
    async fn reconcile(&self) -> Result<()> {
        let staging = self.root.join(STAGING_DIR_NAME);
        if let Err(error) = tokio::fs::remove_dir_all(&staging).await
            && error.kind() != io::ErrorKind::NotFound
        {
            return Err(error.into());
        }
        tokio::fs::create_dir_all(&staging).await?;

        let rows: Vec<String> = sqlx::query_scalar("SELECT key FROM entries").fetch_all(&self.db).await?;
        let mut known = HashSet::new();
        for hex in rows {
            let file_exists = match ThumbnailKey::from_hex(&hex) {
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
            let mut files = tokio::fs::read_dir(shard.path()).await?;
            while let Some(file) = files.next_entry().await? {
                let name = file.file_name().to_string_lossy().into_owned();
                if !known.contains(&name) {
                    tokio::fs::remove_file(file.path()).await?;
                }
            }
        }
        Ok(())
    }

    /// 枠の合計が容量に収まるまで、その枠の古いものから削除する。呼び出し元が `lock` を持つ。
    async fn evict(&self, bucket: CacheBucket) -> Result<()> {
        let capacity = self.options.capacity(bucket) as i64;
        // The total is read once, and the size of each deleted entry is subtracted from it.
        let mut total: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(size), 0) FROM entries WHERE bucket = ?1")
            .bind(bucket.as_str())
            .fetch_one(&self.db)
            .await?;
        while total > capacity {
            let oldest: Option<(String, i64)> = sqlx::query_as("SELECT key, size FROM entries WHERE bucket = ?1 ORDER BY last_accessed ASC LIMIT 1")
                .bind(bucket.as_str())
                .fetch_optional(&self.db)
                .await?;
            let Some((hex, size)) = oldest else {
                return Ok(());
            };

            sqlx::query("DELETE FROM entries WHERE key = ?1").bind(&hex).execute(&self.db).await?;
            total -= size;
            if let Some(key) = ThumbnailKey::from_hex(&hex)
                && let Err(error) = tokio::fs::remove_file(self.file_path(&key)).await
                && error.kind() != io::ErrorKind::NotFound
            {
                return Err(error.into());
            }
        }
        Ok(())
    }

    fn file_path(&self, key: &ThumbnailKey) -> PathBuf {
        self.root.join(FILES_DIR_NAME).join(&key.hex()[..2]).join(key.hex())
    }

    /// 利用順の更新を溜め、遅延後にまとめて書く。応答はこの SQLite 書き込みを待たない。
    fn touch_later(&self, key: &ThumbnailKey) {
        self.pending_touches
            .entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(key.hex().to_string(), self.next_access());
        if self.pending_touches.scheduled.swap(true, Ordering::SeqCst) {
            return;
        }

        let db = self.db.clone();
        let lock = self.lock.clone();
        let pending = self.pending_touches.clone();
        tokio::spawn(async move {
            tokio::time::sleep(TOUCH_FLUSH_DELAY).await;
            let _guard = lock.lock().await;
            pending.scheduled.store(false, Ordering::SeqCst);
            if let Err(error) = Self::flush_touches(&db, &pending).await {
                warn!(error = ?error, "failed to update thumbnail cache recency");
            }
        });
    }

    /// 溜めた利用順を 1 つのトランザクションで反映する。呼び出し元が `lock` を持つ。
    async fn flush_touches(db: &SqlitePool, pending: &PendingTouches) -> Result<()> {
        let touches = std::mem::take(&mut *pending.entries.lock().unwrap_or_else(|e| e.into_inner()));
        if touches.is_empty() {
            return Ok(());
        }

        let mut transaction = db.begin().await?;
        for (hex, access) in &touches {
            sqlx::query("UPDATE entries SET last_accessed = MAX(last_accessed, ?2) WHERE key = ?1")
                .bind(hex)
                .bind(access)
                .execute(&mut *transaction)
                .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    fn next_access(&self) -> i64 {
        self.last_accessed.fetch_add(1, Ordering::SeqCst) + 1
    }

    fn init_error(error: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Error {
        Error::from_error(error, ErrorKind::DatabaseError).with_message("failed to initialize the thumbnail cache")
    }
}

#[cfg(test)]
mod tests {
    use testresult::TestResult;

    use super::*;

    const OPTIONS: ThumbnailCacheOptions = ThumbnailCacheOptions {
        thumbnail_capacity_bytes: 10,
        display_capacity_bytes: 100,
    };

    fn key(seed: u8) -> ThumbnailKey {
        ThumbnailKey::from_digest(&[seed; 32])
    }

    fn image(len: usize, frame_count: Option<u32>) -> CachedImage {
        CachedImage {
            bytes: Bytes::from(vec![7_u8; len]),
            frame_count,
        }
    }

    async fn open(dir: &Path) -> Result<ThumbnailCache> {
        ThumbnailCache::open(&dir.join("thumbnail"), OPTIONS).await
    }

    #[tokio::test]
    async fn stores_and_reads_images_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let cache = open(dir.path()).await?;
        assert_eq!(cache.get(&key(1)).await?, None);

        assert!(cache.put(&key(1), CacheBucket::Thumbnail, &image(4, Some(8))).await?);
        assert!(cache.put(&key(2), CacheBucket::Thumbnail, &image(3, None)).await?);
        assert_eq!(cache.get(&key(1)).await?, Some(image(4, Some(8))));
        assert_eq!(cache.get(&key(2)).await?, Some(image(3, None)));

        drop(cache);
        let reopened = open(dir.path()).await?;
        assert_eq!(reopened.get(&key(1)).await?, Some(image(4, Some(8))), "再起動後も残る");
        Ok(())
    }

    #[tokio::test]
    async fn contains_answers_from_the_index_only_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let cache = open(dir.path()).await?;
        assert!(!cache.contains(&key(1)).await?);

        assert!(cache.put(&key(1), CacheBucket::Thumbnail, &image(4, None)).await?);
        assert!(cache.contains(&key(1)).await?);

        tokio::fs::remove_file(cache.file_path(&key(1))).await?;
        assert!(cache.contains(&key(1)).await?, "the index still has the key, and the file is not read");
        assert_eq!(cache.get(&key(1)).await?, None, "a read finds the missing file");
        Ok(())
    }

    #[tokio::test]
    async fn the_cache_database_uses_synchronous_normal_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let cache = open(dir.path()).await?;
        let mode: i64 = sqlx::query_scalar("PRAGMA synchronous").fetch_one(&cache.db).await?;
        assert_eq!(mode, 1, "NORMAL");
        Ok(())
    }

    #[tokio::test]
    async fn put_many_stores_several_and_skips_the_oversized_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let cache = open(dir.path()).await?;
        let (a, b, c) = (image(3, None), image(11, None), image(4, Some(8)));
        let stored = cache
            .put_many(&[
                (&key(1), CacheBucket::Thumbnail, &a),
                (&key(2), CacheBucket::Thumbnail, &b),
                (&key(3), CacheBucket::Display, &c),
            ])
            .await?;
        assert_eq!(stored, [true, false, true], "an entry larger than its bucket is skipped");
        assert_eq!(cache.get(&key(1)).await?, Some(a));
        assert_eq!(cache.get(&key(2)).await?, None);
        assert_eq!(cache.get(&key(3)).await?, Some(c));
        assert_eq!(cache.put_many(&[]).await?, Vec::<bool>::new());
        Ok(())
    }

    #[tokio::test]
    async fn put_many_evicts_the_first_entries_and_keeps_the_last_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let cache = open(dir.path()).await?;
        // The thumbnail bucket holds 10 bytes: the three companions (4 each) cannot stay with the primary (6).
        let (c1, c2, c3, primary) = (image(4, None), image(4, None), image(4, None), image(6, None));
        cache
            .put_many(&[
                (&key(1), CacheBucket::Thumbnail, &c1),
                (&key(2), CacheBucket::Thumbnail, &c2),
                (&key(3), CacheBucket::Thumbnail, &c3),
                (&key(4), CacheBucket::Thumbnail, &primary),
            ])
            .await?;
        assert_eq!(cache.get(&key(4)).await?, Some(primary), "the last entry is the newest and stays");
        assert_eq!(cache.get(&key(1)).await?, None);
        assert_eq!(cache.get(&key(2)).await?, None);
        assert_eq!(cache.get(&key(3)).await?, Some(c3), "10 bytes: the primary and the newest companion fit");
        Ok(())
    }

    #[tokio::test]
    async fn put_many_fails_without_staging_and_leaves_nothing_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let cache = open(dir.path()).await?;
        tokio::fs::remove_dir_all(dir.path().join("thumbnail").join(STAGING_DIR_NAME)).await?;
        assert!(cache.put_many(&[(&key(1), CacheBucket::Thumbnail, &image(3, None))]).await.is_err());
        assert!(!cache.contains(&key(1)).await?);
        Ok(())
    }

    #[tokio::test]
    async fn evicts_least_recently_used_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let cache = open(dir.path()).await?;
        cache.put(&key(1), CacheBucket::Thumbnail, &image(4, None)).await?;
        cache.put(&key(2), CacheBucket::Thumbnail, &image(4, None)).await?;

        // 1 を読んで最近使ったことにし、2 を最も古くする。
        assert!(cache.get(&key(1)).await?.is_some());
        cache.put(&key(3), CacheBucket::Thumbnail, &image(4, None)).await?;

        assert!(cache.get(&key(1)).await?.is_some());
        assert_eq!(cache.get(&key(2)).await?, None, "容量 10 を超えたので最も古い 2 が消える");
        assert!(cache.get(&key(3)).await?.is_some());
        Ok(())
    }

    #[tokio::test]
    async fn buckets_do_not_evict_each_other_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let cache = open(dir.path()).await?;
        cache.put(&key(1), CacheBucket::Thumbnail, &image(8, None)).await?;
        // 表示用の枠は 100 byte なので、10 byte を 11 件入れると最初の 1 件が消える。
        for seed in 10..21 {
            cache.put(&key(seed), CacheBucket::Display, &image(10, None)).await?;
        }

        assert!(cache.get(&key(1)).await?.is_some(), "表示用画像の書き込みでサムネイルが消えない");
        assert_eq!(cache.get(&key(10)).await?, None, "表示用の枠の中では古いものが消える");
        assert!(cache.get(&key(20)).await?.is_some());
        Ok(())
    }

    #[tokio::test]
    async fn skips_images_larger_than_the_capacity_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let cache = open(dir.path()).await?;
        assert!(!cache.put(&key(1), CacheBucket::Thumbnail, &image(11, None)).await?);
        assert_eq!(cache.get(&key(1)).await?, None);
        Ok(())
    }

    #[tokio::test]
    async fn reconciles_leftovers_on_open_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let root = dir.path().join("thumbnail");
        let cache = open(dir.path()).await?;
        cache.put(&key(1), CacheBucket::Thumbnail, &image(4, None)).await?;
        cache.put(&key(2), CacheBucket::Thumbnail, &image(4, None)).await?;
        let stale_file = cache.file_path(&key(2));
        drop(cache);

        // 途中で止まった公開を再現する。staging の残り、索引のない実ファイル、実ファイルのない索引を作る。
        std::fs::write(root.join("staging/partial"), b"partial")?;
        std::fs::create_dir_all(root.join("files/ff"))?;
        let orphan = root.join("files/ff").join(key(0xff).hex());
        std::fs::write(&orphan, b"orphan")?;
        std::fs::remove_file(&stale_file)?;

        let cache = open(dir.path()).await?;
        assert_eq!(std::fs::read_dir(root.join("staging"))?.count(), 0);
        assert!(!orphan.exists());
        assert_eq!(cache.get(&key(2)).await?, None, "実ファイルのない索引は消える");
        assert!(cache.get(&key(1)).await?.is_some());
        Ok(())
    }

    #[tokio::test]
    async fn recreates_a_broken_cache_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let root = dir.path().join("thumbnail");
        std::fs::create_dir_all(&root)?;
        std::fs::write(root.join("cache.sqlite3"), b"this is not a sqlite database")?;

        let cache = open(dir.path()).await?;
        assert!(cache.put(&key(1), CacheBucket::Thumbnail, &image(4, None)).await?);
        assert!(cache.get(&key(1)).await?.is_some());
        Ok(())
    }

    #[tokio::test]
    async fn rejects_zero_capacity_test() {
        let dir = tempfile::tempdir().unwrap();
        let options = ThumbnailCacheOptions {
            thumbnail_capacity_bytes: 0,
            display_capacity_bytes: 1,
        };
        let error = ThumbnailCache::open(&dir.path().join("thumbnail"), options)
            .await
            .err()
            .expect("zero capacity must be rejected");
        assert_eq!(*error.kind(), ErrorKind::InvalidInput);
    }

    #[tokio::test]
    async fn read_recency_is_flushed_after_the_delay_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let cache = ThumbnailCache::open(dir.path(), OPTIONS).await?;
        cache.put(&key(1), CacheBucket::Thumbnail, &image(4, None)).await?;
        let before: i64 = sqlx::query_scalar("SELECT last_accessed FROM entries").fetch_one(&cache.db).await?;

        assert!(cache.get(&key(1)).await?.is_some());
        let immediately: i64 = sqlx::query_scalar("SELECT last_accessed FROM entries").fetch_one(&cache.db).await?;
        assert_eq!(immediately, before, "読み出しは索引の書き込みを待たない");

        tokio::time::sleep(TOUCH_FLUSH_DELAY + Duration::from_millis(500)).await;
        let flushed: i64 = sqlx::query_scalar("SELECT last_accessed FROM entries").fetch_one(&cache.db).await?;
        assert!(flushed > before, "遅延後に利用順が反映される");
        Ok(())
    }
}
