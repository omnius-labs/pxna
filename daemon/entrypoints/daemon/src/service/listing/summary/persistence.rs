use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use chrono::{DateTime, Utc};
use omnius_core_base::clock::Clock;
use omnius_pxna_engine::{
    perf_span,
    service::{WorkBudget, WorkClass, WorkKind},
};
use ring::digest::{SHA256, digest};
use sqlx::{
    Row as _, SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use tokio::sync::{Mutex as AsyncMutex, mpsc};
use tracing::Instrument as _;

use super::{SummaryEpochs, SummaryKey, SummarySnapshot};
use crate::{config::ListingConfig, prelude::*, service::listing::ListingInvalidation};

const MAX_PAYLOAD: usize = 64 * 1024 * 1024;
const FORMAT_VERSION: i64 = 1;

/// JSON の生成中にも上限を守り、大きい一覧で無制限の一時 Vec を作らない。
struct PayloadWriter {
    bytes: Vec<u8>,
    limit: usize,
}

impl Write for PayloadWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
            return Err(std::io::Error::other("summary payload exceeds capacity"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

enum PendingWrite {
    Save {
        snapshot: Arc<SummarySnapshot>,
        epoch: u64,
    },
    Touch {
        key: SummaryKey,
        epoch: u64,
    },
    #[cfg(test)]
    Barrier(tokio::sync::oneshot::Sender<()>),
}

struct PersistenceState {
    budget: Arc<WorkBudget>,
    pool: SqlitePool,
    file: PathBuf,
    usable: AtomicBool,
    config: ListingConfig,
    clock: Arc<dyn Clock<Utc> + Send + Sync>,
    epochs: Arc<SummaryEpochs>,
    gate: AsyncMutex<()>,
    #[cfg(test)]
    started: tokio::sync::Notify,
}

/// 1 本の writer と待機 1 件だけを所有する。満杯なら派生状態の保存を省略する。
pub(super) struct SummaryPersistence {
    state: Arc<PersistenceState>,
    writer: mpsc::Sender<PendingWrite>,
}

impl SummaryPersistence {
    #[cfg(test)]
    pub async fn open(dir: &Path, config: ListingConfig, clock: Arc<dyn Clock<Utc> + Send + Sync>, epochs: Arc<SummaryEpochs>) -> Option<Self> {
        Self::open_with_budget(dir, config, clock, epochs, Arc::new(WorkBudget::default())).await
    }
    pub async fn open_with_budget(
        dir: &Path,
        config: ListingConfig,
        clock: Arc<dyn Clock<Utc> + Send + Sync>,
        epochs: Arc<SummaryEpochs>,
        budget: Arc<WorkBudget>,
    ) -> Option<Self> {
        let file = dir.join("summary.db");
        let pool = match PersistenceState::connect(dir, &file).await {
            Ok(pool) => pool,
            Err(error) => {
                warn!(?error, "summary index unavailable; using directory scan");
                return None;
            }
        };
        let state = Arc::new(PersistenceState {
            budget,
            pool,
            file,
            usable: AtomicBool::new(true),
            config,
            clock,
            epochs,
            gate: AsyncMutex::new(()),
            #[cfg(test)]
            started: tokio::sync::Notify::new(),
        });
        let initialized = async {
            let mut transaction = state.pool.begin().await?;
            state.trim(&mut transaction).await?;
            transaction.commit().await?;
            Ok::<_, Error>(())
        }
        .await;
        if let Err(error) = initialized {
            state.disable(&error).await;
            return None;
        }
        let (writer, mut receiver) = mpsc::channel(1);
        let weak = Arc::downgrade(&state);
        tokio::spawn(async move {
            while let Some(job) = receiver.recv().await {
                let Some(state) = weak.upgrade() else {
                    break;
                };
                #[cfg(test)]
                state.started.notify_one();
                #[cfg(test)]
                if let PendingWrite::Barrier(done) = job {
                    let _ = done.send(());
                    continue;
                }
                if let Err(error) = state.write(job).await {
                    state.disable(&error).await;
                }
            }
        });
        Some(Self { state, writer })
    }

    pub fn save(&self, snapshot: Arc<SummarySnapshot>, epoch: u64) -> bool {
        self.state.usable.load(Ordering::Acquire)
            && snapshot.fresh(&snapshot.version, self.state.clock.now(), &self.state.config)
            && snapshot.data.entries.len().max(1) <= self.state.config.snapshot_capacity_entries
            && self.writer.try_send(PendingWrite::Save { snapshot, epoch }).is_ok()
    }

    pub fn touch(&self, key: SummaryKey, epoch: u64) {
        if self.state.usable.load(Ordering::Acquire) {
            let _ = self.writer.try_send(PendingWrite::Touch { key, epoch });
        }
    }

    pub async fn load(&self, key: &SummaryKey, version: &Option<String>, now: DateTime<Utc>) -> Option<SummarySnapshot> {
        if version.is_none() || !self.state.usable.load(Ordering::Acquire) {
            return None;
        }
        match self.state.load(key, version, now).await {
            Ok(value) => value,
            Err(error) => {
                self.state.disable(&error).await;
                None
            }
        }
    }

    pub async fn invalidate(&self, invalidation: &ListingInvalidation) {
        if !self.state.usable.load(Ordering::Acquire) {
            return;
        }
        let result = async {
            let _gate = self.state.gate.lock().await;
            let mut transaction = self.state.pool.begin().await?;
            for library in invalidation.entries().keys() {
                let rows = sqlx::query("SELECT cache_key, path FROM summaries WHERE library = ?")
                    .bind(library)
                    .fetch_all(&mut *transaction)
                    .await?;
                for row in rows {
                    let path: String = row.try_get("path")?;
                    if invalidation.affects(library, &path) {
                        let key: String = row.try_get("cache_key")?;
                        sqlx::query("DELETE FROM summaries WHERE cache_key = ?").bind(key).execute(&mut *transaction).await?;
                    }
                }
            }
            transaction.commit().await?;
            Ok::<_, Error>(())
        }
        .await;
        if let Err(error) = result {
            self.state.disable(&error).await;
        }
    }

    #[cfg(test)]
    pub async fn flush(&self) {
        let (send, receive) = tokio::sync::oneshot::channel();
        self.writer.send(PendingWrite::Barrier(send)).await.unwrap();
        receive.await.unwrap();
    }
}

impl PersistenceState {
    fn sql_error(error: sqlx::Error) -> Error {
        let corrupt = matches!(&error, sqlx::Error::Database(error) if error.code().and_then(|code| code.parse::<i32>().ok()).is_some_and(|code| matches!(code & 255, 11 | 26)));
        Error::from_error(error, if corrupt { ErrorKind::InvalidFormat } else { ErrorKind::DatabaseError })
    }

    async fn connect(dir: &Path, file: &Path) -> Result<SqlitePool> {
        tokio::fs::create_dir_all(dir).await?;
        match Self::connect_once(file).await {
            Ok(pool) => Ok(pool),
            Err(error) if *error.kind() == ErrorKind::InvalidFormat => {
                warn!(?error, "discarding invalid derived summary index");
                Self::remove_files(file).await?;
                Self::connect_once(file).await
            }
            Err(error) => Err(error),
        }
    }

    async fn connect_once(file: &Path) -> Result<SqlitePool> {
        let options = SqliteConnectOptions::new().filename(file).create_if_missing(true).journal_mode(SqliteJournalMode::Wal);
        let pool = SqlitePoolOptions::new().max_connections(2).connect_with(options).await.map_err(Self::sql_error)?;
        let initialized = async {
            let result: String = sqlx::query_scalar("PRAGMA quick_check").fetch_one(&pool).await.map_err(Self::sql_error)?;
            if result != "ok" { return Err(Error::new(ErrorKind::InvalidFormat).with_message("summary index integrity check failed")); }
            let version: i64 = sqlx::query_scalar("PRAGMA user_version").fetch_one(&pool).await.map_err(Self::sql_error)?;
            if !matches!(version, 0 | FORMAT_VERSION) { return Err(Error::new(ErrorKind::InvalidFormat).with_message("unknown summary index format")); }
            sqlx::query("CREATE TABLE IF NOT EXISTS summaries (cache_key TEXT PRIMARY KEY, library TEXT NOT NULL, path TEXT NOT NULL, payload BLOB NOT NULL, checksum TEXT NOT NULL, entry_count INTEGER NOT NULL, used_at INTEGER NOT NULL)")
                .execute(&pool).await.map_err(Self::sql_error)?;
            sqlx::query("SELECT cache_key, library, path, payload, checksum, entry_count, used_at FROM summaries LIMIT 0").fetch_all(&pool).await
                .map_err(|error| Error::from_error(error, ErrorKind::InvalidFormat).with_message("unexpected summary index columns"))?;
            // 同じ DB が書き込み可能かも確認し、読めるだけの索引を次の変更後にも信頼しない。
            let mut transaction = pool.begin().await.map_err(Self::sql_error)?;
            sqlx::query("INSERT OR REPLACE INTO summaries VALUES ('write-check', '', '', x'', '', 0, 0)").execute(&mut *transaction).await.map_err(Self::sql_error)?;
            sqlx::query("DELETE FROM summaries WHERE cache_key = 'write-check'").execute(&mut *transaction).await.map_err(Self::sql_error)?;
            sqlx::query("PRAGMA user_version = 1").execute(&mut *transaction).await.map_err(Self::sql_error)?;
            transaction.commit().await.map_err(Self::sql_error)?;
            Ok::<_, Error>(())
        }.await;
        if let Err(error) = initialized {
            pool.close().await;
            return Err(error);
        }
        Ok(pool)
    }

    async fn remove_files(file: &Path) -> Result<()> {
        for suffix in ["", "-wal", "-shm"] {
            let path = PathBuf::from(format!("{}{suffix}", file.display()));
            match tokio::fs::remove_file(path).await {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    }

    async fn disable(&self, error: &Error) {
        if self.usable.swap(false, Ordering::AcqRel) {
            warn!(?error, "summary persistence disabled; using directory scan");
            // 閉じた後に専用 DB だけを捨て、失効に失敗した行を次回の起動へ残さない。
            self.pool.close().await;
            if let Err(error) = Self::remove_files(&self.file).await {
                warn!(?error, "cannot remove derived summary index");
            }
        }
    }

    fn key(key: &SummaryKey) -> Option<String> {
        serde_json::to_vec(key).ok().map(|bytes| hex::encode(digest(&SHA256, &bytes).as_ref()))
    }

    async fn load(&self, key: &SummaryKey, version: &Option<String>, now: DateTime<Utc>) -> Result<Option<SummarySnapshot>> {
        let Some(encoded_key) = Self::key(key) else {
            return Ok(None);
        };
        let row = sqlx::query("SELECT payload, checksum FROM summaries WHERE cache_key = ? AND length(payload) <= ? AND entry_count <= ?")
            .bind(&encoded_key)
            .bind(self.config.summary_index_capacity_bytes.min(MAX_PAYLOAD as u64) as i64)
            .bind(self.config.snapshot_capacity_entries as i64)
            .fetch_optional(&self.pool)
            .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let payload: Vec<u8> = row.try_get("payload")?;
        let checksum: String = row.try_get("checksum")?;
        let expected_key = key.clone();
        let capacity = self.config.snapshot_capacity_entries;
        let snapshot = self
            .budget
            .io()
            .run_blocking(WorkKind::Summary, move || -> Result<SummarySnapshot> {
                if hex::encode(digest(&SHA256, &payload).as_ref()) != checksum {
                    return Err(Error::new(ErrorKind::InvalidFormat).with_message("summary index checksum mismatch"));
                }
                let snapshot: SummarySnapshot =
                    serde_json::from_slice(&payload).map_err(|error| Error::from_error(error, ErrorKind::InvalidFormat).with_message("invalid summary index payload"))?;
                if snapshot.schema != 1 || snapshot.key != expected_key || !snapshot.data.valid_for(&expected_key) {
                    return Err(Error::new(ErrorKind::InvalidFormat).with_message("invalid summary index contents"));
                }
                Ok(snapshot)
            })
            .await
            .map_err(|error| error.with_message("cannot decode summary index"))??;
        if snapshot.data.entries.len().max(1) <= capacity && snapshot.fresh(version, now, &self.config) {
            Ok(Some(snapshot))
        } else {
            let _gate = self.gate.lock().await;
            sqlx::query("DELETE FROM summaries WHERE cache_key = ?").bind(encoded_key).execute(&self.pool).await?;
            Ok(None)
        }
    }

    async fn write(&self, job: PendingWrite) -> Result<()> {
        if !self.usable.load(Ordering::Acquire) {
            return Ok(());
        }
        let (key, epoch) = match &job {
            PendingWrite::Save { snapshot, epoch } => (&snapshot.key, *epoch),
            PendingWrite::Touch { key, epoch } => (key, *epoch),
            #[cfg(test)]
            PendingWrite::Barrier(_) => unreachable!(),
        };
        if self.epochs.current(&key.library) != epoch {
            return Ok(());
        }
        let Some(encoded_key) = Self::key(key) else {
            return Ok(());
        };
        let Some(payload) = self.encode(&job).await? else {
            return Ok(());
        };
        let _background = self.budget.acquire(WorkClass::Background, WorkKind::Summary).await?;
        let _gate = self.gate.lock().await;
        if !self.usable.load(Ordering::Acquire) || self.epochs.current(&key.library) != epoch {
            return Ok(());
        }
        let now = self.clock.now().timestamp_millis();
        match &job {
            PendingWrite::Save { snapshot, .. } => {
                if snapshot.version.is_none() || !snapshot.fresh(&snapshot.version, self.clock.now(), &self.config) {
                    return Ok(());
                }
                let cost = snapshot.data.entries.len().max(1);
                if cost > self.config.snapshot_capacity_entries {
                    return Ok(());
                }
                let checksum = hex::encode(digest(&SHA256, &payload).as_ref());
                let span = perf_span!("summary_persist", entry_count = snapshot.data.entries.len());
                async {
                    let mut transaction = self.pool.begin().await?;
                    sqlx::query("INSERT OR REPLACE INTO summaries (cache_key, library, path, payload, checksum, entry_count, used_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
                        .bind(encoded_key)
                        .bind(&key.library)
                        .bind(&key.path)
                        .bind(payload)
                        .bind(checksum)
                        .bind(cost as i64)
                        .bind(now)
                        .execute(&mut *transaction)
                        .await?;
                    self.trim(&mut transaction).await?;
                    transaction.commit().await?;
                    Ok::<_, Error>(())
                }
                .instrument(span)
                .await?;
            }
            PendingWrite::Touch { .. } => {
                sqlx::query("UPDATE summaries SET used_at = ? WHERE cache_key = ?")
                    .bind(now)
                    .bind(encoded_key)
                    .execute(&self.pool)
                    .await?;
            }
            #[cfg(test)]
            PendingWrite::Barrier(_) => unreachable!(),
        }
        Ok(())
    }

    async fn trim(&self, transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>) -> Result<()> {
        loop {
            let (entries, bytes): (i64, i64) = sqlx::query_as("SELECT COALESCE(SUM(entry_count), 0), COALESCE(SUM(length(payload)), 0) FROM summaries")
                .fetch_one(&mut **transaction)
                .await?;
            if entries <= self.config.snapshot_capacity_entries as i64 && bytes <= self.config.summary_index_capacity_bytes.min(i64::MAX as u64) as i64 {
                break;
            }
            sqlx::query("DELETE FROM summaries WHERE cache_key = (SELECT cache_key FROM summaries ORDER BY used_at, cache_key LIMIT 1)")
                .execute(&mut **transaction)
                .await?;
        }
        Ok(())
    }

    async fn encode(&self, job: &PendingWrite) -> Result<Option<Vec<u8>>> {
        match job {
            PendingWrite::Save { snapshot, .. } => {
                let snapshot = snapshot.clone();
                let limit = self.config.summary_index_capacity_bytes.min(MAX_PAYLOAD as u64) as usize;
                let encoded = self
                    .budget
                    .io()
                    .run_blocking_as(WorkClass::Background, WorkKind::Summary, move || {
                        let mut output = PayloadWriter {
                            bytes: Vec::with_capacity(limit.min(32768)),
                            limit,
                        };
                        serde_json::to_writer(&mut output, &*snapshot).ok().map(|()| output.bytes)
                    })
                    .await
                    .map_err(|error| error.with_message("cannot encode summary index"))?;
                Ok(encoded)
            }
            PendingWrite::Touch { .. } => Ok(Some(Vec::new())),
            #[cfg(test)]
            PendingWrite::Barrier(_) => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use omnius_core_base::clock::FakeClockUtc;
    use omnius_pxna_interface::models;
    use testresult::TestResult;

    use super::super::{SummaryData, SummaryEntry, SummaryIndex, SummaryOptions};
    use super::*;

    fn key(path: &str) -> SummaryKey {
        SummaryKey::new(
            "test",
            Path::new("/library"),
            path,
            &SummaryOptions::from_query(&serde_json::from_value(serde_json::json!({"path":path})).unwrap()).unwrap(),
        )
        .unwrap()
    }
    fn data(path: &str) -> SummaryData {
        SummaryData::new(vec![SummaryEntry {
            entry: models::FileSystemEntrySummary::new(format!("{}/a", path.trim_end_matches('/')), "a".into(), models::FileSystemEntryKind::File, false),
            normalized: "a".into(),
        }])
    }
    fn snapshot(path: &str, now: DateTime<Utc>) -> Arc<SummarySnapshot> {
        Arc::new(SummarySnapshot {
            schema: 1,
            id: uuid::Uuid::new_v4().to_string(),
            key: key(path),
            version: Some("test-version".into()),
            created_at: now,
            data: data(path),
        })
    }

    #[tokio::test]
    async fn persistence_encode_and_write_release_background_between_units_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let clock = Arc::new(FakeClockUtc::new(Utc::now()));
        let budget = Arc::new(WorkBudget::default());
        let persistence = SummaryPersistence::open_with_budget(dir.path(), Default::default(), clock.clone(), Arc::new(SummaryEpochs::default()), budget.clone())
            .await
            .unwrap();
        let gate = persistence.state.gate.lock().await;
        assert!(persistence.save(snapshot("/", clock.now()), 0));
        // Encoding completes, then writing acquires BG before the DB gate. No nested BG wait.
        while budget.counts(WorkKind::Summary).0 < 2 {
            tokio::task::yield_now().await;
        }
        assert_eq!(budget.counts(WorkKind::Summary), (2, 1));
        assert!(budget.try_probe().is_none());
        let _foreground = budget.acquire(WorkClass::Foreground, WorkKind::Listing).await?;
        drop(gate);
        persistence.flush().await;
        assert_eq!(budget.counts(WorkKind::Summary), (2, 2));
        Ok(())
    }

    #[tokio::test]
    async fn restart_reuses_index_and_mutation_invalidates_disk_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let clock = Arc::new(FakeClockUtc::new(Utc::now()));
        let version = Some(super::super::SnapshotVersion::Local(Some(clock.now() - chrono::Duration::seconds(10))));
        let first = SummaryIndex::open(Default::default(), clock.clone(), dir.path()).await;
        let initial = first.get_or_scan(key("/"), version.clone(), || async { Ok(data("/")) }).await?;
        first.persistence.as_ref().unwrap().flush().await;
        drop(first);
        let second = SummaryIndex::open(Default::default(), clock.clone(), dir.path()).await;
        let reused = second
            .get_or_scan(key("/"), version.clone(), || async { panic!("persisted snapshot must avoid scan") })
            .await?;
        assert_eq!(initial.id, reused.id);
        second.invalidate(&ListingInvalidation::for_entry("test", "/changed")?).await;
        second.persistence.as_ref().unwrap().flush().await;
        drop(second);
        let third = SummaryIndex::open(Default::default(), clock, dir.path()).await;
        let rebuilt = third.get_or_scan(key("/"), version, || async { Ok(data("/")) }).await?;
        assert_ne!(initial.id, rebuilt.id);
        third.persistence.as_ref().unwrap().flush().await;
        Ok(())
    }

    #[tokio::test]
    async fn freshness_root_unknown_and_recent_versions_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let clock = Arc::new(FakeClockUtc::new(Utc::now()));
        let config = ListingConfig {
            snapshot_max_age_secs: 5,
            ..Default::default()
        };
        let persistence = SummaryPersistence::open(dir.path(), config.clone(), clock.clone(), Arc::new(SummaryEpochs::default()))
            .await
            .unwrap();
        let saved = snapshot("/", clock.now());
        assert!(persistence.save(saved.clone(), 0));
        persistence.flush().await;
        assert!(persistence.load(&key("/"), &saved.version, clock.now()).await.is_some());
        let mut other_root = key("/");
        other_root.root = PathBuf::from("/other");
        assert!(persistence.load(&other_root, &saved.version, clock.now()).await.is_none());
        assert!(persistence.load(&key("/"), &None, clock.now()).await.is_none());
        assert!(persistence.load(&key("/"), &Some("different-version".into()), clock.now()).await.is_none());
        assert!(persistence.save(saved.clone(), 0));
        persistence.flush().await;
        clock.advance(std::time::Duration::from_secs(5));
        assert!(persistence.load(&key("/"), &saved.version, clock.now()).await.is_none());
        let now = clock.now();
        for version in [
            None,
            Some(super::super::SnapshotVersion::Local(None)),
            Some(super::super::SnapshotVersion::Local(Some(now - chrono::Duration::seconds(1)))),
            Some(super::super::SnapshotVersion::Local(Some(now + chrono::Duration::seconds(1)))),
        ] {
            assert!(SummaryIndex::version(&version, now).is_none());
        }
        Ok(())
    }

    #[tokio::test]
    async fn corrupt_payload_unknown_format_and_unwritable_location_fall_back_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let marker = dir.path().join("auth.db");
        std::fs::write(&marker, b"untouched")?;
        let clock = Arc::new(FakeClockUtc::new(Utc::now()));
        let persistence = SummaryPersistence::open(dir.path(), Default::default(), clock.clone(), Arc::new(SummaryEpochs::default()))
            .await
            .unwrap();
        let saved = snapshot("/", clock.now());
        persistence.save(saved.clone(), 0);
        persistence.flush().await;
        sqlx::query("UPDATE summaries SET checksum = 'bad'").execute(&persistence.state.pool).await?;
        assert!(persistence.load(&key("/"), &saved.version, clock.now()).await.is_none());
        assert!(!persistence.state.usable.load(Ordering::Acquire));
        assert!(!dir.path().join("summary.db").exists());
        drop(persistence);
        let persistence = SummaryPersistence::open(dir.path(), Default::default(), clock.clone(), Arc::new(SummaryEpochs::default()))
            .await
            .unwrap();
        sqlx::query("PRAGMA user_version = 999").execute(&persistence.state.pool).await?;
        persistence.state.pool.close().await;
        drop(persistence);
        let rebuilt = SummaryPersistence::open(dir.path(), Default::default(), clock.clone(), Arc::new(SummaryEpochs::default()))
            .await
            .unwrap();
        assert_eq!(sqlx::query_scalar::<_, i64>("PRAGMA user_version").fetch_one(&rebuilt.state.pool).await?, 1);
        rebuilt.state.pool.close().await;
        drop(rebuilt);
        std::fs::write(dir.path().join("summary.db"), b"not a SQLite database")?;
        let repaired = SummaryPersistence::open(dir.path(), Default::default(), clock.clone(), Arc::new(SummaryEpochs::default()))
            .await
            .unwrap();
        repaired.state.pool.close().await;
        let unavailable = dir.path().join("file-instead-of-directory");
        std::fs::write(&unavailable, b"x")?;
        let fallback = SummaryIndex::open(Default::default(), clock, &unavailable).await;
        assert!(fallback.persistence.is_none());
        assert!(fallback.get_or_scan(key("/"), None, || async { Ok(data("/")) }).await.is_ok());
        assert_eq!(std::fs::read(marker)?, b"untouched");
        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn read_only_index_is_not_trusted_after_startup_test() -> TestResult {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = tempfile::tempdir()?;
        let clock = Arc::new(FakeClockUtc::new(Utc::now()));
        let persistence = SummaryPersistence::open(dir.path(), Default::default(), clock.clone(), Arc::new(SummaryEpochs::default()))
            .await
            .unwrap();
        persistence.save(snapshot("/", clock.now()), 0);
        persistence.flush().await;
        persistence.state.pool.close().await;
        drop(persistence);
        let file = dir.path().join("summary.db");
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o400))?;
        let reopened = SummaryPersistence::open(dir.path(), Default::default(), clock, Arc::new(SummaryEpochs::default())).await;
        std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o600))?;
        assert!(reopened.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn count_and_byte_capacity_use_lru_including_empty_snapshots_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let clock = Arc::new(FakeClockUtc::new(Utc::now()));
        let config = ListingConfig {
            snapshot_capacity_entries: 2,
            ..Default::default()
        };
        let persistence = SummaryPersistence::open(dir.path(), config, clock.clone(), Arc::new(SummaryEpochs::default()))
            .await
            .unwrap();
        let a = snapshot("/a", clock.now());
        let b = snapshot("/b", clock.now());
        let c = snapshot("/c", clock.now());
        for value in [&a, &b] {
            persistence.save(value.clone(), 0);
            persistence.flush().await;
            clock.advance(std::time::Duration::from_secs(1));
        }
        persistence.touch(a.key.clone(), 0);
        persistence.flush().await;
        clock.advance(std::time::Duration::from_secs(1));
        persistence.save(c.clone(), 0);
        persistence.flush().await;
        assert!(persistence.load(&a.key, &a.version, clock.now()).await.is_some());
        assert!(persistence.load(&b.key, &b.version, clock.now()).await.is_none());
        assert!(persistence.load(&c.key, &c.version, clock.now()).await.is_some());
        let mut empty = (*snapshot("/empty", clock.now())).clone();
        empty.data = SummaryData::new(vec![]);
        persistence.save(Arc::new(empty), 0);
        persistence.flush().await;
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT SUM(entry_count) FROM summaries")
                .fetch_one(&persistence.state.pool)
                .await?,
            2
        );
        let bytes_dir = dir.path().join("bytes");
        let bytes_config = ListingConfig {
            summary_index_capacity_bytes: serde_json::to_vec(&*a)?.len() as u64 + 16,
            ..Default::default()
        };
        let byte_limit = bytes_config.summary_index_capacity_bytes;
        let by_bytes = SummaryPersistence::open(&bytes_dir, bytes_config, clock.clone(), Arc::new(SummaryEpochs::default()))
            .await
            .unwrap();
        for path in ["/a", "/b", "/c"] {
            by_bytes.save(snapshot(path, clock.now()), 0);
            by_bytes.flush().await;
            clock.advance(std::time::Duration::from_secs(1));
        }
        let (count, bytes): (i64, i64) = sqlx::query_as("SELECT COUNT(*), SUM(length(payload)) FROM summaries")
            .fetch_one(&by_bytes.state.pool)
            .await?;
        assert_eq!(count, 1);
        assert!(bytes as u64 <= byte_limit);
        Ok(())
    }

    #[tokio::test]
    async fn reduced_capacity_is_applied_when_reopening_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let clock = Arc::new(FakeClockUtc::new(Utc::now()));
        let original = SummaryPersistence::open(dir.path(), Default::default(), clock.clone(), Arc::new(SummaryEpochs::default()))
            .await
            .unwrap();
        for path in ["/a", "/b", "/c"] {
            original.save(snapshot(path, clock.now()), 0);
            original.flush().await;
        }
        original.state.pool.close().await;
        drop(original);
        let config = ListingConfig {
            snapshot_capacity_entries: 1,
            ..Default::default()
        };
        let reduced = SummaryPersistence::open(dir.path(), config, clock, Arc::new(SummaryEpochs::default())).await.unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT SUM(entry_count) FROM summaries")
                .fetch_one(&reduced.state.pool)
                .await?,
            1
        );
        Ok(())
    }

    #[tokio::test]
    async fn writer_has_one_running_one_queued_and_old_epochs_cannot_restore_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let clock = Arc::new(FakeClockUtc::new(Utc::now()));
        let epochs = Arc::new(SummaryEpochs::default());
        let persistence = SummaryPersistence::open(dir.path(), Default::default(), clock.clone(), epochs.clone()).await.unwrap();
        let gate = persistence.state.gate.lock().await;
        assert!(persistence.save(snapshot("/", clock.now()), 0));
        persistence.state.started.notified().await;
        assert!(persistence.save(snapshot("/", clock.now()), 0));
        assert!(!persistence.save(snapshot("/", clock.now()), 0));
        let invalidation = ListingInvalidation::for_entry("test", "/changed")?;
        epochs.advance(&invalidation);
        drop(gate);
        persistence.invalidate(&invalidation).await;
        persistence.flush().await;
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM summaries").fetch_one(&persistence.state.pool).await?, 0);
        assert!(persistence.save(snapshot("/", clock.now()), 1));
        persistence.flush().await;
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM summaries").fetch_one(&persistence.state.pool).await?, 1);
        Ok(())
    }

    #[tokio::test]
    async fn unknown_or_oversized_snapshots_do_not_enter_writer_queue_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let clock = Arc::new(FakeClockUtc::new(Utc::now()));
        let config = ListingConfig {
            snapshot_capacity_entries: 1,
            ..Default::default()
        };
        let persistence = SummaryPersistence::open(dir.path(), config, clock.clone(), Arc::new(SummaryEpochs::default()))
            .await
            .unwrap();
        let mut unknown = (*snapshot("/", clock.now())).clone();
        unknown.version = None;
        assert!(!persistence.save(Arc::new(unknown), 0));
        let mut oversized = (*snapshot("/", clock.now())).clone();
        oversized.data.entries.push(oversized.data.entries[0].clone());
        assert!(!persistence.save(Arc::new(oversized), 0));
        assert_eq!(persistence.writer.capacity(), 1);
        Ok(())
    }

    #[test]
    fn decoded_summary_rejects_invalid_positions_and_paths_test() {
        let valid = data("/");
        assert!(valid.valid_for(&key("/")));
        let mut corrupt = valid.clone();
        corrupt.name_order[0] = usize::MAX;
        assert!(!corrupt.valid_for(&key("/")));
        let mut corrupt = valid.clone();
        corrupt.entries[0].entry.path = "/../escape".into();
        assert!(!corrupt.valid_for(&key("/")));
        let mut corrupt = valid.clone();
        corrupt.entries[0].normalized = "different".into();
        assert!(!corrupt.valid_for(&key("/")));
        assert!(!valid.valid_for(&key("/elsewhere")));
    }
}
