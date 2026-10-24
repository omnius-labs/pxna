use std::{collections::HashMap, path::Path};

use chrono::{DateTime, Datelike as _, NaiveDate, Utc};
use omnius_core_migration::sqlite::{MigrationRequest, SqliteMigrator};
use sqlx::{
    Row as _, Sqlite, SqlitePool, Transaction,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteRow},
};

use super::{TimelineDate, TimelineZone};
use crate::prelude::*;

const DATABASE_FILE_NAME: &str = "index.db";

/// 1 つの問い合わせに並べる値の数。SQLite の上限より十分小さくする。
const CHUNK: usize = 500;

/// 月ごとの件数を、Entry の行の追加、削除、日時の変更と同じトランザクションで増減する。
///
/// migrator は `;` で文を分けるため、本体に複数の文を持つ trigger は migration の外で作る。
const TRIGGERS: &[&str] = &[
    r#"
CREATE TRIGGER IF NOT EXISTS timeline_entries_insert AFTER INSERT ON timeline_entries BEGIN
    INSERT OR IGNORE INTO timeline_months (library_id, month, count) VALUES (NEW.library_id, substr(NEW.date, 1, 7), 0);
    UPDATE timeline_months SET count = count + 1 WHERE library_id = NEW.library_id AND month = substr(NEW.date, 1, 7);
END
"#,
    r#"
CREATE TRIGGER IF NOT EXISTS timeline_entries_delete AFTER DELETE ON timeline_entries BEGIN
    UPDATE timeline_months SET count = count - 1 WHERE library_id = OLD.library_id AND month = substr(OLD.date, 1, 7);
    DELETE FROM timeline_months WHERE library_id = OLD.library_id AND month = substr(OLD.date, 1, 7) AND count <= 0;
END
"#,
    r#"
CREATE TRIGGER IF NOT EXISTS timeline_entries_update AFTER UPDATE OF library_id, date ON timeline_entries
WHEN OLD.library_id <> NEW.library_id OR substr(OLD.date, 1, 7) <> substr(NEW.date, 1, 7) BEGIN
    UPDATE timeline_months SET count = count - 1 WHERE library_id = OLD.library_id AND month = substr(OLD.date, 1, 7);
    DELETE FROM timeline_months WHERE library_id = OLD.library_id AND month = substr(OLD.date, 1, 7) AND count <= 0;
    INSERT OR IGNORE INTO timeline_months (library_id, month, count) VALUES (NEW.library_id, substr(NEW.date, 1, 7), 0);
    UPDATE timeline_months SET count = count + 1 WHERE library_id = NEW.library_id AND month = substr(NEW.date, 1, 7);
END
"#,
];

/// タイムラインに並べる Entry の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineKind {
    Image,
    Video,
}

impl TimelineKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Video => "video",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "image" => Ok(Self::Image),
            "video" => Ok(Self::Video),
            _ => Err(Error::new(ErrorKind::InvalidFormat).with_message(format!("unknown timeline kind: {value}"))),
        }
    }
}

/// タイムラインに並べる Entry の行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineRow {
    /// Library 相対パス。archive の内側は archive の path に内側の path を連結する。
    pub path: String,
    /// 行が属する local ファイル（最外の archive か、Entry 自身）の Library 相対パス。
    pub file_path: String,
    pub kind: TimelineKind,
    pub date: TimelineDate,
    pub size: u64,
    pub modified: Option<DateTime<Utc>>,
    pub readonly: bool,
}

impl TimelineRow {
    pub fn cursor(&self) -> TimelineCursor {
        TimelineCursor {
            date: self.date.text(),
            path: self.path.clone(),
        }
    }

    fn from_row(row: &SqliteRow) -> Result<Self> {
        let date: String = row.try_get("date")?;
        let source: String = row.try_get("date_source")?;
        let instant: Option<i64> = row.try_get("date_instant")?;
        let kind: String = row.try_get("kind")?;
        let size: i64 = row.try_get("size")?;
        let modified: Option<i64> = row.try_get("modified_ns")?;
        Ok(Self {
            path: row.try_get("path")?,
            file_path: row.try_get("file_path")?,
            kind: TimelineKind::parse(&kind)?,
            date: TimelineDate::parse(&date, &source, instant)?,
            size: u64::try_from(size)?,
            modified: modified.map(DateTime::from_timestamp_nanos),
            readonly: row.try_get("readonly")?,
        })
    }
}

/// 読み直しを省く判定に使う、local のファイルの値。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelineFileVersion {
    /// 画像と動画のサイズと、更新時刻（UNIX 時刻のナノ秒）。
    Media { size: u64, modified_ns: i64 },
    /// archive の `ArchiveIdentity`。
    Archive(String),
}

/// 集計が 1 回の書き込みで索引へ反映する内容。上から順に反映する。
#[derive(Debug, Default)]
pub struct TimelineBatch {
    /// 読み直さなかったファイル。世代だけを記録する。
    pub touched: Vec<String>,
    /// 読み直したファイル。属する Entry の行をすべて消す。
    pub replaced: Vec<String>,
    /// ファイルの行と、読み直しを省く判定に使う値。今の世代を記録する。
    pub files: Vec<(String, TimelineFileVersion)>,
    /// 足す Entry の行。
    pub rows: Vec<TimelineRow>,
}

impl TimelineBatch {
    pub fn is_empty(&self) -> bool {
        self.touched.is_empty() && self.replaced.is_empty() && self.files.is_empty() && self.rows.is_empty()
    }
}

/// ページの位置。ページの端の Entry の並びの日時と Library 相対パスの組である。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineCursor {
    pub date: String,
    pub path: String,
}

impl TimelineCursor {
    const DATE_LENGTH: usize = "YYYY-MM-DDTHH:MM:SS".len();

    /// 並びの日時の 19 文字に Library 相対パスを続けた文字列から作る。
    pub fn parse(value: &str) -> Result<Self> {
        let invalid = || Error::new(ErrorKind::InvalidInput).with_message("invalid timeline cursor");
        let date = value.get(..Self::DATE_LENGTH).ok_or_else(invalid)?;
        let path = value.get(Self::DATE_LENGTH..).ok_or_else(invalid)?;
        TimelineDate::parse(date, "exif", None).map_err(|_| invalid())?;
        if !path.starts_with('/') {
            return Err(invalid());
        }
        Ok(Self {
            date: date.to_string(),
            path: path.to_string(),
        })
    }

    pub fn encode(&self) -> String {
        format!("{}{}", self.date, self.path)
    }
}

/// どこからページを読むか。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelineAnchor {
    /// 最も新しい Entry から。
    Newest,
    /// cursor より古い Entry。
    After(TimelineCursor),
    /// cursor より新しい Entry。
    Before(TimelineCursor),
    /// `YYYY-MM` の月の、最も新しい Entry から。その月が空なら、それより古い Entry から。
    Month(String),
    /// Library 相対パスの Entry を中央に含む範囲。
    Around(String),
}

/// 新しい順に並べたページと、その前後を読むための cursor。前後に Entry がなければ `None` にする。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelinePage {
    pub rows: Vec<TimelineRow>,
    pub previous: Option<TimelineCursor>,
    pub next: Option<TimelineCursor>,
}

/// Library 相対パスとその配下を指す範囲。Library のルートはすべてを指す。
struct PathRange {
    exact: String,
    lower: String,
    upper: String,
}

impl PathRange {
    fn of(path: &str) -> Option<Self> {
        let path = path.trim_end_matches('/');
        if path.is_empty() {
            return None;
        }
        // `/` の次の文字は `0` なので、`path/` で始まる文字列はすべて `path/` 以上 `path0` 未満に入る。
        Some(Self {
            exact: path.to_string(),
            lower: format!("{path}/"),
            upper: format!("{path}0"),
        })
    }

    /// `column` が範囲に入る条件。引数は `exact`、`lower`、`upper` の順に bind する。
    fn condition(column: &str) -> String {
        format!("({column} = ? OR ({column} >= ? AND {column} < ?))")
    }
}

/// タイムラインに並べる Entry、ファイル、月ごとの件数、集計の状態を持つ SQLite の索引。
pub struct TimelineIndex {
    db: SqlitePool,
}

impl TimelineIndex {
    pub async fn open(dir: &Path) -> Result<Self> {
        tokio::fs::create_dir_all(dir).await?;
        let options = SqliteConnectOptions::new()
            .filename(dir.join(DATABASE_FILE_NAME))
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(30));
        let db = SqlitePool::connect_with(options).await?;
        Self::migrate(&db).await?;
        Ok(Self { db })
    }

    async fn migrate(db: &SqlitePool) -> Result<()> {
        let requests = vec![MigrationRequest {
            name: "2026-10-14_timeline_index".to_string(),
            queries: r#"
CREATE TABLE timeline_files (
    library_id TEXT NOT NULL,
    path TEXT NOT NULL,
    size INTEGER,
    modified_ns INTEGER,
    archive_identity TEXT,
    generation INTEGER NOT NULL,
    PRIMARY KEY (library_id, path)
) WITHOUT ROWID;
CREATE TABLE timeline_entries (
    library_id TEXT NOT NULL,
    path TEXT NOT NULL,
    file_path TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('image', 'video')),
    date TEXT NOT NULL,
    date_source TEXT NOT NULL,
    date_instant INTEGER,
    size INTEGER NOT NULL,
    modified_ns INTEGER,
    readonly INTEGER NOT NULL,
    PRIMARY KEY (library_id, path)
) WITHOUT ROWID;
CREATE INDEX timeline_entries_order ON timeline_entries (library_id, date DESC, path);
CREATE INDEX timeline_entries_file ON timeline_entries (library_id, file_path);
CREATE TABLE timeline_months (
    library_id TEXT NOT NULL,
    month TEXT NOT NULL,
    count INTEGER NOT NULL,
    PRIMARY KEY (library_id, month)
) WITHOUT ROWID;
CREATE TABLE timeline_scans (
    library_id TEXT NOT NULL PRIMARY KEY,
    generation INTEGER NOT NULL,
    completed_at INTEGER
) WITHOUT ROWID;
CREATE TABLE timeline_settings (
    key TEXT NOT NULL PRIMARY KEY,
    value TEXT NOT NULL
) WITHOUT ROWID;
"#
            .to_string(),
        }];
        SqliteMigrator::migrate(db, requests).await?;
        for trigger in TRIGGERS {
            sqlx::query(sqlx::AssertSqlSafe(*trigger)).execute(db).await?;
        }
        Ok(())
    }

    /// 書き込みのトランザクションを始める。
    ///
    /// 複数の Library の集計が同じ索引へ同時に書くので、始めから書き込みのロックを取る。読み取りから始めると、
    /// ほかの書き込みの後に書き込みへ移るときに busy の待ちを経ずに `SQLITE_BUSY` で失敗し、その Library の集計が止まる。
    async fn write(&self) -> Result<Transaction<'static, Sqlite>> {
        Ok(self.db.begin_with("BEGIN IMMEDIATE").await?)
    }

    // ---- 集計の状態 ----

    /// 今の世代。全体の集計をまだ始めていなければ 0 である。
    pub async fn generation(&self, library: &str) -> Result<i64> {
        let generation: Option<i64> = sqlx::query_scalar("SELECT generation FROM timeline_scans WHERE library_id = ?")
            .bind(library)
            .fetch_optional(&self.db)
            .await?;
        Ok(generation.unwrap_or(0))
    }

    /// 全体の集計を始め、進めた世代を返す。
    pub async fn begin_scan(&self, library: &str) -> Result<i64> {
        let generation: i64 = sqlx::query_scalar(
            "INSERT INTO timeline_scans (library_id, generation) VALUES (?, 1) ON CONFLICT (library_id) DO UPDATE SET generation = generation + 1 RETURNING generation",
        )
        .bind(library)
        .fetch_one(&self.db)
        .await?;
        Ok(generation)
    }

    pub async fn complete_scan(&self, library: &str, at: DateTime<Utc>) -> Result<()> {
        sqlx::query("UPDATE timeline_scans SET completed_at = ? WHERE library_id = ?")
            .bind(at.timestamp_nanos_opt())
            .bind(library)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    /// 最後に全体の集計を終えた時刻。
    pub async fn completed_at(&self, library: &str) -> Result<Option<DateTime<Utc>>> {
        let completed: Option<Option<i64>> = sqlx::query_scalar("SELECT completed_at FROM timeline_scans WHERE library_id = ?")
            .bind(library)
            .fetch_optional(&self.db)
            .await?;
        Ok(completed.flatten().map(DateTime::from_timestamp_nanos))
    }

    /// ファイルの行を 1 つでも持つか。
    pub async fn has_files(&self, library: &str) -> Result<bool> {
        let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM timeline_files WHERE library_id = ?)")
            .bind(library)
            .fetch_one(&self.db)
            .await?;
        Ok(exists)
    }

    // ---- 書き込み ----

    /// ファイルの行の、読み直しを省く判定に使う値。行のないファイルは結果に含めない。
    pub async fn versions(&self, library: &str, paths: &[String]) -> Result<HashMap<String, TimelineFileVersion>> {
        let mut versions = HashMap::with_capacity(paths.len());
        for chunk in paths.chunks(CHUNK) {
            let sql = format!(
                "SELECT path, size, modified_ns, archive_identity FROM timeline_files WHERE library_id = ? AND path IN ({})",
                vec!["?"; chunk.len()].join(", ")
            );
            let mut query = sqlx::query(sqlx::AssertSqlSafe(sql)).bind(library);
            for path in chunk {
                query = query.bind(path);
            }
            for row in query.fetch_all(&self.db).await? {
                let path: String = row.try_get("path")?;
                let identity: Option<String> = row.try_get("archive_identity")?;
                let size: Option<i64> = row.try_get("size")?;
                let modified_ns: Option<i64> = row.try_get("modified_ns")?;
                let version = match (identity, size, modified_ns) {
                    (Some(identity), _, _) => TimelineFileVersion::Archive(identity),
                    (None, Some(size), Some(modified_ns)) => TimelineFileVersion::Media {
                        size: u64::try_from(size)?,
                        modified_ns,
                    },
                    _ => continue,
                };
                versions.insert(path, version);
            }
        }
        Ok(versions)
    }

    /// 集計の結果を 1 つのトランザクションで書き込む。
    ///
    /// 記録する世代は、渡された世代と今の世代の大きい方にする。範囲の集計が古い世代のまま書いても、行の世代が後退せず、
    /// 全体の集計の掃除で消されない。
    pub async fn apply(&self, library: &str, generation: i64, batch: &TimelineBatch) -> Result<()> {
        let mut tx = self.write().await?;
        let current: Option<i64> = sqlx::query_scalar("SELECT generation FROM timeline_scans WHERE library_id = ?")
            .bind(library)
            .fetch_optional(&mut *tx)
            .await?;
        let generation = generation.max(current.unwrap_or(0));
        for chunk in batch.touched.chunks(CHUNK) {
            let sql = format!(
                "UPDATE timeline_files SET generation = ? WHERE library_id = ? AND path IN ({})",
                vec!["?"; chunk.len()].join(", ")
            );
            let mut query = sqlx::query(sqlx::AssertSqlSafe(sql)).bind(generation).bind(library);
            for path in chunk {
                query = query.bind(path);
            }
            query.execute(&mut *tx).await?;
        }
        for path in &batch.replaced {
            sqlx::query("DELETE FROM timeline_entries WHERE library_id = ? AND file_path = ?")
                .bind(library)
                .bind(path)
                .execute(&mut *tx)
                .await?;
        }
        for (path, version) in &batch.files {
            let (size, modified_ns, identity) = match version {
                TimelineFileVersion::Media { size, modified_ns } => (Some(i64::try_from(*size)?), Some(*modified_ns), None),
                TimelineFileVersion::Archive(identity) => (None, None, Some(identity.as_str())),
            };
            sqlx::query(
                r#"
INSERT INTO timeline_files (library_id, path, size, modified_ns, archive_identity, generation) VALUES (?, ?, ?, ?, ?, ?)
ON CONFLICT (library_id, path) DO UPDATE SET
    size = excluded.size, modified_ns = excluded.modified_ns, archive_identity = excluded.archive_identity, generation = excluded.generation
"#,
            )
            .bind(library)
            .bind(path)
            .bind(size)
            .bind(modified_ns)
            .bind(identity)
            .bind(generation)
            .execute(&mut *tx)
            .await?;
        }
        for row in &batch.rows {
            Self::insert_row(&mut tx, library, row).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn insert_row(tx: &mut Transaction<'_, Sqlite>, library: &str, row: &TimelineRow) -> Result<()> {
        sqlx::query(
            r#"
INSERT INTO timeline_entries (library_id, path, file_path, kind, date, date_source, date_instant, size, modified_ns, readonly)
VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
ON CONFLICT (library_id, path) DO UPDATE SET
    file_path = excluded.file_path, kind = excluded.kind, date = excluded.date, date_source = excluded.date_source,
    date_instant = excluded.date_instant, size = excluded.size, modified_ns = excluded.modified_ns, readonly = excluded.readonly
"#,
        )
        .bind(library)
        .bind(&row.path)
        .bind(&row.file_path)
        .bind(row.kind.as_str())
        .bind(row.date.text())
        .bind(row.date.source.as_str())
        .bind(row.date.instant_nanos())
        .bind(i64::try_from(row.size)?)
        .bind(row.modified.and_then(|time| time.timestamp_nanos_opt()))
        .bind(row.readonly)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    /// `path` とその配下のファイルの行と、それらに属する Entry の行を消す。
    pub async fn remove(&self, library: &str, path: &str) -> Result<()> {
        let mut tx = self.write().await?;
        Self::remove_in(&mut tx, library, path).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn remove_in(tx: &mut Transaction<'_, Sqlite>, library: &str, path: &str) -> Result<()> {
        let Some(range) = PathRange::of(path) else {
            sqlx::query("DELETE FROM timeline_entries WHERE library_id = ?").bind(library).execute(&mut **tx).await?;
            sqlx::query("DELETE FROM timeline_files WHERE library_id = ?").bind(library).execute(&mut **tx).await?;
            return Ok(());
        };
        let sql = format!("DELETE FROM timeline_entries WHERE library_id = ? AND {}", PathRange::condition("file_path"));
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(library)
            .bind(&range.exact)
            .bind(&range.lower)
            .bind(&range.upper)
            .execute(&mut **tx)
            .await?;
        let sql = format!("DELETE FROM timeline_files WHERE library_id = ? AND {}", PathRange::condition("path"));
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(library)
            .bind(&range.exact)
            .bind(&range.lower)
            .bind(&range.upper)
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    /// `src` とその配下の行の path を `dst` へ付け替え、世代を、付け替えた時点の世代 N を表す `-(N + 1)` にする。`dst` に残っていた行は先に消す。
    ///
    /// 世代が負の行は、範囲の集計が対象かを判定し直して世代を記録するまで、未確認として扱う（`sweep`）。
    ///
    /// 付け替えた、読み終えた archive のファイルの Library 相対パスを返す。読み終えていない archive（identity が空）は含めず、読み直しの対象のまま残す。`ArchiveIdentity` は path から決まるため、呼び出し側が計算し直す。
    pub async fn rename(&self, library: &str, src: &str, dst: &str) -> Result<Vec<String>> {
        let (Some(src_range), Some(dst_range)) = (PathRange::of(src), PathRange::of(dst)) else {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("cannot rename the library root"));
        };
        let mut tx = self.write().await?;
        Self::remove_in(&mut tx, library, &dst_range.exact).await?;

        let offset = i64::try_from(src_range.exact.chars().count() + 1)?;
        let sql = format!(
            "UPDATE timeline_files SET path = ? || substr(path, ?), generation = -(COALESCE((SELECT generation FROM timeline_scans WHERE library_id = ?), 0) + 1) WHERE library_id = ? AND {}",
            PathRange::condition("path")
        );
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(&dst_range.exact)
            .bind(offset)
            .bind(library)
            .bind(library)
            .bind(&src_range.exact)
            .bind(&src_range.lower)
            .bind(&src_range.upper)
            .execute(&mut *tx)
            .await?;
        let sql = format!(
            "UPDATE timeline_entries SET path = ? || substr(path, ?), file_path = ? || substr(file_path, ?) WHERE library_id = ? AND {}",
            PathRange::condition("file_path")
        );
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(&dst_range.exact)
            .bind(offset)
            .bind(&dst_range.exact)
            .bind(offset)
            .bind(library)
            .bind(&src_range.exact)
            .bind(&src_range.lower)
            .bind(&src_range.upper)
            .execute(&mut *tx)
            .await?;

        let sql = format!(
            "SELECT path FROM timeline_files WHERE library_id = ? AND archive_identity IS NOT NULL AND archive_identity <> '' AND {}",
            PathRange::condition("path")
        );
        let archives: Vec<String> = sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
            .bind(library)
            .bind(&dst_range.exact)
            .bind(&dst_range.lower)
            .bind(&dst_range.upper)
            .fetch_all(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(archives)
    }

    pub async fn set_archive_identity(&self, library: &str, path: &str, identity: &str) -> Result<()> {
        sqlx::query("UPDATE timeline_files SET archive_identity = ? WHERE library_id = ? AND path = ? AND archive_identity IS NOT NULL AND archive_identity <> ''")
            .bind(identity)
            .bind(library)
            .bind(path)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    /// 走査した範囲のうち、古い世代のファイルの行と、それに属する Entry の行を消す。
    ///
    /// 世代が負の行は、名前の変更で付け替えた直後の、まだ判定し直していない行である。付け替えた時点の世代 N を `-(N + 1)` で持つ。
    /// 全体の集計は、集計を始める前に付け替えた行（N が `generation` より小さい行）だけを消し、集計の最中に付け替えた行を残す。
    /// 範囲の集計（`scoped`）は、範囲の中の判定し直しを担うので、集計を始めたときまでに付け替えた行（N が `generation` 以下の行）を消す。
    pub async fn sweep(&self, library: &str, scope: &str, generation: i64, scoped: bool) -> Result<()> {
        const STALE: &str = "((generation >= 0 AND generation < ?) OR (generation < 0 AND -generation - 1 < ?))";
        let pending = if scoped { generation + 1 } else { generation };
        let mut tx = self.write().await?;
        match PathRange::of(scope) {
            None => {
                let sql = format!("DELETE FROM timeline_entries WHERE library_id = ? AND file_path IN (SELECT path FROM timeline_files WHERE library_id = ? AND {STALE})");
                sqlx::query(sqlx::AssertSqlSafe(sql))
                    .bind(library)
                    .bind(library)
                    .bind(generation)
                    .bind(pending)
                    .execute(&mut *tx)
                    .await?;
                let sql = format!("DELETE FROM timeline_files WHERE library_id = ? AND {STALE}");
                sqlx::query(sqlx::AssertSqlSafe(sql)).bind(library).bind(generation).bind(pending).execute(&mut *tx).await?;
            }
            Some(range) => {
                let stale = format!("SELECT path FROM timeline_files WHERE library_id = ? AND {STALE} AND {}", PathRange::condition("path"));
                let sql = format!("DELETE FROM timeline_entries WHERE library_id = ? AND file_path IN ({stale})");
                sqlx::query(sqlx::AssertSqlSafe(sql))
                    .bind(library)
                    .bind(library)
                    .bind(generation)
                    .bind(pending)
                    .bind(&range.exact)
                    .bind(&range.lower)
                    .bind(&range.upper)
                    .execute(&mut *tx)
                    .await?;
                let sql = format!("DELETE FROM timeline_files WHERE library_id = ? AND {STALE} AND {}", PathRange::condition("path"));
                sqlx::query(sqlx::AssertSqlSafe(sql))
                    .bind(library)
                    .bind(generation)
                    .bind(pending)
                    .bind(&range.exact)
                    .bind(&range.lower)
                    .bind(&range.upper)
                    .execute(&mut *tx)
                    .await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }

    // ---- タイムゾーン ----

    /// 並びの日時を直したタイムゾーンが `zone` と違えば、元の瞬間を持つ並びの日時をすべて直し、記録を書き換える。
    pub async fn rezone(&self, zone: &TimelineZone) -> Result<()> {
        let recorded: Option<String> = sqlx::query_scalar("SELECT value FROM timeline_settings WHERE key = 'time_zone'")
            .fetch_optional(&self.db)
            .await?;
        if recorded.as_deref() == Some(zone.name()) {
            return Ok(());
        }
        if recorded.is_some() {
            let mut last: Option<(String, String)> = None;
            loop {
                let rows = match &last {
                    None => sqlx::query(
                        "SELECT library_id, path, date, date_source, date_instant FROM timeline_entries WHERE date_instant IS NOT NULL ORDER BY library_id, path LIMIT ?",
                    )
                    .bind(CHUNK as i64)
                    .fetch_all(&self.db)
                    .await?,
                    Some((library, path)) => sqlx::query(
                        "SELECT library_id, path, date, date_source, date_instant FROM timeline_entries WHERE date_instant IS NOT NULL AND (library_id > ?1 OR (library_id = ?1 AND path > ?2)) ORDER BY library_id, path LIMIT ?3",
                    )
                    .bind(library)
                    .bind(path)
                    .bind(CHUNK as i64)
                    .fetch_all(&self.db)
                    .await?,
                };
                let Some(tail) = rows.last() else {
                    break;
                };
                last = Some((tail.try_get("library_id")?, tail.try_get("path")?));
                let mut tx = self.write().await?;
                for row in &rows {
                    let library: String = row.try_get("library_id")?;
                    let path: String = row.try_get("path")?;
                    let date: String = row.try_get("date")?;
                    let source: String = row.try_get("date_source")?;
                    let instant: Option<i64> = row.try_get("date_instant")?;
                    let rezoned = TimelineDate::parse(&date, &source, instant)?.in_zone(zone).text();
                    if rezoned != date {
                        sqlx::query("UPDATE timeline_entries SET date = ? WHERE library_id = ? AND path = ?")
                            .bind(rezoned)
                            .bind(library)
                            .bind(path)
                            .execute(&mut *tx)
                            .await?;
                    }
                }
                tx.commit().await?;
            }
        }
        sqlx::query("INSERT INTO timeline_settings (key, value) VALUES ('time_zone', ?) ON CONFLICT (key) DO UPDATE SET value = excluded.value")
            .bind(zone.name())
            .execute(&self.db)
            .await?;
        Ok(())
    }

    // ---- 読み出し ----

    pub async fn page(&self, library: &str, anchor: &TimelineAnchor, limit: usize) -> Result<TimelinePage> {
        let limit = limit.max(1);
        let rows = match anchor {
            TimelineAnchor::Newest => self.older(library, None, limit).await?,
            TimelineAnchor::After(cursor) => self.older(library, Some(cursor), limit).await?,
            TimelineAnchor::Before(cursor) => self.newer(library, cursor, limit).await?,
            TimelineAnchor::Month(month) => {
                let end = Self::month_end(month)?;
                self.older_than_date(library, &end, limit).await?
            }
            TimelineAnchor::Around(path) => {
                let Some(center) = self.row(library, path).await? else {
                    return Err(Error::new(ErrorKind::NotFound).with_message("timeline entry not found"));
                };
                let cursor = center.cursor();
                let newer_count = (limit - 1) / 2;
                let mut rows = self.newer(library, &cursor, newer_count).await?;
                rows.push(center);
                rows.extend(self.older(library, Some(&cursor), limit - rows.len()).await?);
                rows
            }
        };
        let previous = match rows.first() {
            Some(first) if self.exists_newer(library, &first.cursor()).await? => Some(first.cursor()),
            _ => None,
        };
        let next = match rows.last() {
            Some(last) if self.exists_older(library, &last.cursor()).await? => Some(last.cursor()),
            _ => None,
        };
        Ok(TimelinePage { rows, previous, next })
    }

    pub async fn months(&self, library: &str) -> Result<Vec<(String, u64)>> {
        let rows = sqlx::query("SELECT month, count FROM timeline_months WHERE library_id = ? AND count > 0 ORDER BY month DESC")
            .bind(library)
            .fetch_all(&self.db)
            .await?;
        rows.iter()
            .map(|row| {
                let count: i64 = row.try_get("count")?;
                Ok((row.try_get("month")?, u64::try_from(count)?))
            })
            .collect()
    }

    const COLUMNS: &'static str = "path, file_path, kind, date, date_source, date_instant, size, modified_ns, readonly";

    async fn row(&self, library: &str, path: &str) -> Result<Option<TimelineRow>> {
        let sql = format!("SELECT {} FROM timeline_entries WHERE library_id = ? AND path = ?", Self::COLUMNS);
        let row = sqlx::query(sqlx::AssertSqlSafe(sql)).bind(library).bind(path).fetch_optional(&self.db).await?;
        row.as_ref().map(TimelineRow::from_row).transpose()
    }

    /// cursor より古い Entry を、新しい順に `limit` 件。cursor がなければ最も新しい Entry から。
    async fn older(&self, library: &str, cursor: Option<&TimelineCursor>, limit: usize) -> Result<Vec<TimelineRow>> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let rows = match cursor {
            None => {
                let sql = format!("SELECT {} FROM timeline_entries WHERE library_id = ? ORDER BY date DESC, path ASC LIMIT ?", Self::COLUMNS);
                sqlx::query(sqlx::AssertSqlSafe(sql)).bind(library).bind(i64::try_from(limit)?).fetch_all(&self.db).await?
            }
            Some(cursor) => {
                let sql = format!(
                    "SELECT {} FROM timeline_entries WHERE library_id = ?1 AND (date < ?2 OR (date = ?2 AND path > ?3)) ORDER BY date DESC, path ASC LIMIT ?4",
                    Self::COLUMNS
                );
                sqlx::query(sqlx::AssertSqlSafe(sql))
                    .bind(library)
                    .bind(&cursor.date)
                    .bind(&cursor.path)
                    .bind(i64::try_from(limit)?)
                    .fetch_all(&self.db)
                    .await?
            }
        };
        rows.iter().map(TimelineRow::from_row).collect()
    }

    async fn older_than_date(&self, library: &str, date: &str, limit: usize) -> Result<Vec<TimelineRow>> {
        let sql = format!(
            "SELECT {} FROM timeline_entries WHERE library_id = ? AND date < ? ORDER BY date DESC, path ASC LIMIT ?",
            Self::COLUMNS
        );
        let rows = sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(library)
            .bind(date)
            .bind(i64::try_from(limit)?)
            .fetch_all(&self.db)
            .await?;
        rows.iter().map(TimelineRow::from_row).collect()
    }

    /// cursor より新しい Entry のうち、cursor に近い `limit` 件を新しい順に。
    async fn newer(&self, library: &str, cursor: &TimelineCursor, limit: usize) -> Result<Vec<TimelineRow>> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT {} FROM timeline_entries WHERE library_id = ?1 AND (date > ?2 OR (date = ?2 AND path < ?3)) ORDER BY date ASC, path DESC LIMIT ?4",
            Self::COLUMNS
        );
        let rows = sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(library)
            .bind(&cursor.date)
            .bind(&cursor.path)
            .bind(i64::try_from(limit)?)
            .fetch_all(&self.db)
            .await?;
        let mut rows = rows.iter().map(TimelineRow::from_row).collect::<Result<Vec<_>>>()?;
        rows.reverse();
        Ok(rows)
    }

    async fn exists_newer(&self, library: &str, cursor: &TimelineCursor) -> Result<bool> {
        let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM timeline_entries WHERE library_id = ?1 AND (date > ?2 OR (date = ?2 AND path < ?3)))")
            .bind(library)
            .bind(&cursor.date)
            .bind(&cursor.path)
            .fetch_one(&self.db)
            .await?;
        Ok(exists)
    }

    async fn exists_older(&self, library: &str, cursor: &TimelineCursor) -> Result<bool> {
        let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM timeline_entries WHERE library_id = ?1 AND (date < ?2 OR (date = ?2 AND path > ?3)))")
            .bind(library)
            .bind(&cursor.date)
            .bind(&cursor.path)
            .fetch_one(&self.db)
            .await?;
        Ok(exists)
    }

    /// `YYYY-MM` の翌月の 1 日 0 時。その月の Entry はすべてこれより前に並ぶ。
    fn month_end(month: &str) -> Result<String> {
        let invalid = || Error::new(ErrorKind::InvalidInput).with_message("month must be YYYY-MM");
        let first = NaiveDate::parse_from_str(&format!("{month}-01"), "%Y-%m-%d").map_err(|_| invalid())?;
        if month.len() != 7 {
            return Err(invalid());
        }
        let next = if first.month() == 12 {
            NaiveDate::from_ymd_opt(first.year() + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(first.year(), first.month() + 1, 1)
        }
        .ok_or_else(invalid)?;
        Ok(format!("{}T00:00:00", next.format("%Y-%m-%d")))
    }
}

#[cfg(test)]
mod tests {
    use testresult::TestResult;

    use super::*;
    use crate::service::timeline::TimelineDateSource;

    fn row(path: &str, file_path: &str, date: &str) -> TimelineRow {
        TimelineRow {
            path: path.to_string(),
            file_path: file_path.to_string(),
            kind: TimelineKind::Image,
            date: TimelineDate::from_exif(date).unwrap(),
            size: 1,
            modified: None,
            readonly: false,
        }
    }

    fn media(size: u64) -> TimelineFileVersion {
        TimelineFileVersion::Media { size, modified_ns: 1 }
    }

    async fn index(dir: &tempfile::TempDir) -> Result<TimelineIndex> {
        TimelineIndex::open(dir.path()).await
    }

    async fn insert(index: &TimelineIndex, rows: &[TimelineRow]) -> Result<()> {
        let mut batch = TimelineBatch::default();
        for row in rows {
            if !batch.files.iter().any(|(path, _)| path == &row.file_path) {
                batch.replaced.push(row.file_path.clone());
                batch.files.push((row.file_path.clone(), media(1)));
            }
            batch.rows.push(row.clone());
        }
        index.apply("lib", 1, &batch).await
    }

    fn paths(page: &TimelinePage) -> Vec<&str> {
        page.rows.iter().map(|row| row.path.as_str()).collect()
    }

    async fn month_counts(index: &TimelineIndex) -> Result<Vec<(String, u64)>> {
        index.months("lib").await
    }

    #[tokio::test]
    async fn pages_do_not_overlap_or_skip_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = index(&dir).await?;
        let rows = (0..7)
            .map(|i| row(&format!("/p{i}.jpg"), &format!("/p{i}.jpg"), &format!("2024-01-{:02}T00:00:00", 1 + i % 3)))
            .collect::<Vec<_>>();
        insert(&index, &rows).await?;

        let mut seen = Vec::new();
        let mut page = index.page("lib", &TimelineAnchor::Newest, 3).await?;
        assert_eq!(page.previous, None);
        loop {
            seen.extend(paths(&page).into_iter().map(str::to_string));
            let Some(next) = page.next.clone() else { break };
            page = index.page("lib", &TimelineAnchor::After(next), 3).await?;
        }
        let all = index.page("lib", &TimelineAnchor::Newest, 100).await?;
        assert_eq!(seen, paths(&all));
        assert_eq!(seen.len(), 7);

        // 末尾から前へ読み戻しても、同じ並びになる。
        let last = all.rows.last().unwrap().cursor();
        let mut back = vec![last.path.clone()];
        let mut page = index.page("lib", &TimelineAnchor::Before(last), 2).await?;
        loop {
            let mut current = paths(&page).into_iter().map(str::to_string).collect::<Vec<_>>();
            current.extend(back);
            back = current;
            let Some(previous) = page.previous.clone() else { break };
            page = index.page("lib", &TimelineAnchor::Before(previous), 2).await?;
        }
        assert_eq!(back, seen);
        Ok(())
    }

    #[tokio::test]
    async fn month_and_around_anchors_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = index(&dir).await?;
        insert(
            &index,
            &[
                row("/a.jpg", "/a.jpg", "2024-03-10T00:00:00"),
                row("/b.jpg", "/b.jpg", "2024-02-20T00:00:00"),
                row("/c.jpg", "/c.jpg", "2024-02-01T00:00:00"),
                row("/d.jpg", "/d.jpg", "2023-12-31T23:59:59"),
            ],
        )
        .await?;

        let page = index.page("lib", &TimelineAnchor::Month("2024-02".to_string()), 2).await?;
        assert_eq!(paths(&page), ["/b.jpg", "/c.jpg"]);
        assert!(page.previous.is_some() && page.next.is_some());
        let page = index.page("lib", &TimelineAnchor::Month("2024-01".to_string()), 2).await?;
        assert_eq!(paths(&page), ["/d.jpg"], "空の月はそれより古い Entry から");

        let page = index.page("lib", &TimelineAnchor::Around("/c.jpg".to_string()), 3).await?;
        assert_eq!(paths(&page), ["/b.jpg", "/c.jpg", "/d.jpg"]);
        assert!(index.page("lib", &TimelineAnchor::Month("2024-13".to_string()), 2).await.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn month_counts_follow_rows_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = index(&dir).await?;
        insert(
            &index,
            &[
                row("/x.zip/1.jpg", "/x.zip", "2024-01-01T00:00:00"),
                row("/x.zip/2.jpg", "/x.zip", "2024-01-02T00:00:00"),
                row("/y.jpg", "/y.jpg", "2024-02-01T00:00:00"),
            ],
        )
        .await?;
        assert_eq!(month_counts(&index).await?, [("2024-02".to_string(), 1), ("2024-01".to_string(), 2)]);

        // archive を読み直すと、属する行がすべて置き換わる。
        let batch = TimelineBatch {
            replaced: vec!["/x.zip".to_string()],
            files: vec![("/x.zip".to_string(), TimelineFileVersion::Archive("id2".to_string()))],
            rows: vec![row("/x.zip/1.jpg", "/x.zip", "2024-03-01T00:00:00")],
            ..Default::default()
        };
        index.apply("lib", 1, &batch).await?;
        assert_eq!(month_counts(&index).await?, [("2024-03".to_string(), 1), ("2024-02".to_string(), 1)]);

        index.remove("lib", "/y.jpg").await?;
        assert_eq!(month_counts(&index).await?, [("2024-03".to_string(), 1)]);
        Ok(())
    }

    #[tokio::test]
    async fn rename_moves_rows_and_reports_archives_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = index(&dir).await?;
        insert(
            &index,
            &[
                row("/old/a.jpg", "/old/a.jpg", "2024-01-01T00:00:00"),
                row("/old/x.zip/b.jpg", "/old/x.zip", "2024-01-02T00:00:00"),
                row("/older.jpg", "/older.jpg", "2024-01-03T00:00:00"),
            ],
        )
        .await?;
        let batch = TimelineBatch {
            replaced: vec!["/old/x.zip".to_string()],
            files: vec![("/old/x.zip".to_string(), TimelineFileVersion::Archive("id".to_string()))],
            rows: vec![row("/old/x.zip/b.jpg", "/old/x.zip", "2024-01-02T00:00:00")],
            ..Default::default()
        };
        index.apply("lib", 1, &batch).await?;

        let archives = index.rename("lib", "/old", "/new").await?;
        assert_eq!(archives, ["/new/x.zip"]);
        let page = index.page("lib", &TimelineAnchor::Newest, 10).await?;
        assert_eq!(paths(&page), ["/older.jpg", "/new/x.zip/b.jpg", "/new/a.jpg"], "`/older` は範囲に入らない");
        assert_eq!(page.rows[1].file_path, "/new/x.zip");
        assert_eq!(page.rows[1].date.source, TimelineDateSource::Exif, "日時は読み直さない");

        // 付け替えた行は未確認なので、範囲の集計が触れなければ掃除で消える。
        let batch = TimelineBatch {
            touched: vec!["/new/a.jpg".to_string()],
            ..Default::default()
        };
        index.apply("lib", 5, &batch).await?;
        index.sweep("lib", "/new", 5, true).await?;
        assert_eq!(paths(&index.page("lib", &TimelineAnchor::Newest, 10).await?), ["/older.jpg", "/new/a.jpg"]);
        assert_eq!(month_counts(&index).await?, [("2024-01".to_string(), 2)]);
        Ok(())
    }

    #[tokio::test]
    async fn a_stale_generation_does_not_make_rows_sweepable_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = index(&dir).await?;
        let first = index.begin_scan("lib").await?;
        assert_eq!(first, 1);
        // 範囲の集計が世代 0 のまま、全体の集計の始まりの後に書いても、行は今の世代を持つ。
        insert(&index, &[row("/late.jpg", "/late.jpg", "2024-01-01T00:00:00")]).await?;
        let second = index.begin_scan("lib").await?;
        let batch = TimelineBatch {
            files: vec![("/late.jpg".to_string(), media(1))],
            rows: vec![row("/late.jpg", "/late.jpg", "2024-01-01T00:00:00")],
            replaced: vec!["/late.jpg".to_string()],
            ..Default::default()
        };
        index.apply("lib", first, &batch).await?;
        index.sweep("lib", "/", second, false).await?;
        assert_eq!(paths(&index.page("lib", &TimelineAnchor::Newest, 10).await?), ["/late.jpg"]);
        Ok(())
    }

    #[tokio::test]
    async fn renamed_rows_survive_a_full_sweep_started_before_and_die_in_the_scoped_one_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = index(&dir).await?;
        insert(&index, &[row("/a.jpg", "/a.jpg", "2024-01-01T00:00:00")]).await?;
        let scan = index.begin_scan("lib").await?;
        // 全体の集計が宛先を読み終えた後に、移動して付け替える。
        index.rename("lib", "/a.jpg", "/b.jpg").await?;
        index.sweep("lib", "/", scan, false).await?;
        assert_eq!(
            paths(&index.page("lib", &TimelineAnchor::Newest, 10).await?),
            ["/b.jpg"],
            "集計の最中に付け替えた行は、全体の掃除で消えない"
        );
        assert_eq!(month_counts(&index).await?, [("2024-01".to_string(), 1)]);

        // 範囲の集計が触れなかった行は、対象外の名前になったものとして消える。
        index.sweep("lib", "/b.jpg", scan, true).await?;
        assert!(paths(&index.page("lib", &TimelineAnchor::Newest, 10).await?).is_empty());

        // 次の全体の集計は、その前に付け替えて判定されなかった行を消す。
        insert(&index, &[row("/c.jpg", "/c.jpg", "2024-01-01T00:00:00")]).await?;
        index.rename("lib", "/c.jpg", "/d.jpg").await?;
        let next = index.begin_scan("lib").await?;
        index.sweep("lib", "/", next, false).await?;
        assert!(paths(&index.page("lib", &TimelineAnchor::Newest, 10).await?).is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn a_partly_published_archive_is_swept_and_read_again_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = index(&dir).await?;
        let first = index.begin_scan("lib").await?;
        // 読み終えていない archive の途中の公開。ファイルの行は、どの identity にも一致しない空の値で持つ。
        let partial = TimelineBatch {
            replaced: vec!["/x.zip".to_string()],
            files: vec![("/x.zip".to_string(), TimelineFileVersion::Archive(String::new()))],
            rows: vec![row("/x.zip/1.jpg", "/x.zip", "2024-01-01T00:00:00")],
            ..Default::default()
        };
        index.apply("lib", first, &partial).await?;
        let versions = index.versions("lib", &["/x.zip".to_string()]).await?;
        assert_ne!(versions.get("/x.zip"), Some(&TimelineFileVersion::Archive("real".to_string())), "読み直しを省かない");

        // 中断の後に archive がなくなったら、次の全体の集計の掃除で Entry と月の件数が消える。
        let next = index.begin_scan("lib").await?;
        index.sweep("lib", "/", next, false).await?;
        assert!(paths(&index.page("lib", &TimelineAnchor::Newest, 10).await?).is_empty());
        assert!(month_counts(&index).await?.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn renaming_a_partly_published_archive_keeps_it_unfinished_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = index(&dir).await?;
        let scan = index.begin_scan("lib").await?;
        let partial = TimelineBatch {
            replaced: vec!["/x.zip".to_string()],
            files: vec![("/x.zip".to_string(), TimelineFileVersion::Archive(String::new()))],
            rows: vec![row("/x.zip/1.jpg", "/x.zip", "2024-01-01T00:00:00")],
            ..Default::default()
        };
        index.apply("lib", scan, &partial).await?;
        let finished = TimelineBatch {
            replaced: vec!["/y.zip".to_string()],
            files: vec![("/y.zip".to_string(), TimelineFileVersion::Archive("done".to_string()))],
            rows: vec![row("/y.zip/1.jpg", "/y.zip", "2024-01-01T00:00:00")],
            ..Default::default()
        };
        index.apply("lib", scan, &finished).await?;

        // 読み終えていない archive は、identity の再計算の対象にならず、空のまま読み直しの対象に残る。
        assert!(index.rename("lib", "/x.zip", "/x2.zip").await?.is_empty());
        index.set_archive_identity("lib", "/x2.zip", "new").await?;
        let versions = index.versions("lib", &["/x2.zip".to_string()]).await?;
        assert_eq!(versions.get("/x2.zip"), Some(&TimelineFileVersion::Archive(String::new())));
        // 読み終えた archive は、新しい path の identity へ直す。
        assert_eq!(index.rename("lib", "/y.zip", "/y2.zip").await?, ["/y2.zip"]);
        Ok(())
    }

    #[tokio::test]
    async fn rename_over_an_existing_archive_removes_its_entries_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = index(&dir).await?;
        insert(
            &index,
            &[
                row("/src.zip/a.jpg", "/src.zip", "2024-01-01T00:00:00"),
                row("/dst.zip/b.jpg", "/dst.zip", "2024-02-01T00:00:00"),
            ],
        )
        .await?;
        index.rename("lib", "/src.zip", "/dst.zip").await?;
        assert_eq!(paths(&index.page("lib", &TimelineAnchor::Newest, 10).await?), ["/dst.zip/a.jpg"]);
        assert_eq!(month_counts(&index).await?, [("2024-01".to_string(), 1)]);
        Ok(())
    }

    #[tokio::test]
    async fn rezone_rewrites_instant_dates_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = index(&dir).await?;
        let utc = TimelineZone::new(Some("UTC"))?;
        index.rezone(&utc).await?;
        let instant = DateTime::parse_from_rfc3339("2024-01-31T20:00:00Z")?.with_timezone(&Utc);
        let mut modified = row("/m.jpg", "/m.jpg", "2000-01-01T00:00:00");
        modified.date = TimelineDate::choose(&utc, None, Some(instant), None).unwrap();
        insert(&index, &[modified, row("/e.jpg", "/e.jpg", "2024-01-15T00:00:00")]).await?;
        assert_eq!(month_counts(&index).await?, [("2024-01".to_string(), 2)]);

        let tokyo = TimelineZone::new(Some("Asia/Tokyo"))?;
        index.rezone(&tokyo).await?;
        let page = index.page("lib", &TimelineAnchor::Newest, 10).await?;
        assert_eq!(page.rows[0].date.text(), "2024-02-01T05:00:00");
        assert_eq!(page.rows[1].date.text(), "2024-01-15T00:00:00", "Exif の値は直さない");
        assert_eq!(month_counts(&index).await?, [("2024-02".to_string(), 1), ("2024-01".to_string(), 1)]);
        Ok(())
    }

    #[test]
    fn cursor_round_trips_test() -> TestResult {
        let cursor = TimelineCursor {
            date: "2024-01-01T00:00:00".to_string(),
            path: "/a b/c.jpg".to_string(),
        };
        assert_eq!(TimelineCursor::parse(&cursor.encode())?, cursor);
        assert!(TimelineCursor::parse("2024-01-01").is_err());
        assert!(TimelineCursor::parse("2024-01-01T00:00:00relative").is_err());
        Ok(())
    }
}
