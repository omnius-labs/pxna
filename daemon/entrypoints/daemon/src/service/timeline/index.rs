use std::{collections::HashMap, path::Path};

use chrono::{DateTime, Datelike as _, NaiveDate, Utc};
use omnius_core_migration::sqlite::{MigrationRequest, SqliteMigrator};
use sqlx::{
    Row as _, Sqlite, SqlitePool, Transaction,
    query::Query,
    sqlite::{SqliteArguments, SqliteConnectOptions, SqliteJournalMode, SqliteRow},
};

use super::{TimelineDate, TimelineZone};
use crate::prelude::*;

const DATABASE_FILE_NAME: &str = "index.db";

type SqliteQuery<'q> = Query<'q, Sqlite, SqliteArguments>;

/// 1 つの問い合わせに並べる値の数。SQLite の上限より十分小さくする。
const CHUNK: usize = 500;

/// Keep the per-month counts in step with the rows of `timeline_entries` in the same transaction that adds, removes, or changes them.
///
/// A count is held per Library, month, kind, and whether the Entry is inside an archive, that is, whether its path differs from its
/// local file's. A count is raised by an upsert, since a conflict clause of a statement that fires a trigger overrides `INSERT OR IGNORE` in its
/// body, and the upsert of a row is such a statement. The migrator splits statements on `;`, so triggers, whose bodies hold several statements,
/// are created outside the migrations. They are created with `IF NOT EXISTS` after the migrations, and the migration that changed the key dropped the old ones.
const TRIGGERS: &[&str] = &[
    r#"
CREATE TRIGGER IF NOT EXISTS timeline_entries_insert AFTER INSERT ON timeline_entries BEGIN
    INSERT INTO timeline_months (library_id, month, kind, inside_archive, count)
    VALUES (NEW.library_id, substr(NEW.date, 1, 7), NEW.kind, NEW.path <> NEW.file_path, 1)
    ON CONFLICT (library_id, month, kind, inside_archive) DO UPDATE SET count = count + 1;
END
"#,
    r#"
CREATE TRIGGER IF NOT EXISTS timeline_entries_delete AFTER DELETE ON timeline_entries BEGIN
    UPDATE timeline_months SET count = count - 1
    WHERE library_id = OLD.library_id AND month = substr(OLD.date, 1, 7) AND kind = OLD.kind AND inside_archive = (OLD.path <> OLD.file_path);
    DELETE FROM timeline_months
    WHERE library_id = OLD.library_id AND month = substr(OLD.date, 1, 7) AND kind = OLD.kind AND inside_archive = (OLD.path <> OLD.file_path) AND count <= 0;
END
"#,
    r#"
CREATE TRIGGER IF NOT EXISTS timeline_entries_update AFTER UPDATE OF library_id, path, file_path, kind, date ON timeline_entries
WHEN OLD.library_id <> NEW.library_id OR substr(OLD.date, 1, 7) <> substr(NEW.date, 1, 7) OR OLD.kind <> NEW.kind
    OR (OLD.path <> OLD.file_path) <> (NEW.path <> NEW.file_path) BEGIN
    UPDATE timeline_months SET count = count - 1
    WHERE library_id = OLD.library_id AND month = substr(OLD.date, 1, 7) AND kind = OLD.kind AND inside_archive = (OLD.path <> OLD.file_path);
    DELETE FROM timeline_months
    WHERE library_id = OLD.library_id AND month = substr(OLD.date, 1, 7) AND kind = OLD.kind AND inside_archive = (OLD.path <> OLD.file_path) AND count <= 0;
    INSERT INTO timeline_months (library_id, month, kind, inside_archive, count)
    VALUES (NEW.library_id, substr(NEW.date, 1, 7), NEW.kind, NEW.path <> NEW.file_path, 1)
    ON CONFLICT (library_id, month, kind, inside_archive) DO UPDATE SET count = count + 1;
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

/// ページに並べる Entry の行と、その行の Library。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineListedRow {
    pub library: String,
    pub row: TimelineRow,
}

impl TimelineListedRow {
    pub fn cursor(&self) -> TimelineCursor {
        TimelineCursor {
            date: self.row.date.text(),
            library: self.library.clone(),
            path: self.row.path.clone(),
        }
    }

    /// Puts `rows` in the order of the sequence, newest timeline date first and then by Library id and path, keeps a row that several sources
    /// read once, and keeps at most `limit` rows.
    fn merge(mut rows: Vec<Self>, limit: usize) -> Vec<Self> {
        rows.sort_by_cached_key(|listed| (std::cmp::Reverse(listed.row.date.text()), listed.library.clone(), listed.row.path.clone()));
        rows.dedup_by(|later, earlier| later.library == earlier.library && later.row.path == earlier.row.path);
        rows.truncate(limit);
        rows
    }

    fn from_row(row: &SqliteRow) -> Result<Self> {
        Ok(Self {
            library: row.try_get("library_id")?,
            row: TimelineRow::from_row(row)?,
        })
    }
}

/// ページの位置。ページの端の Entry の並びの日時、Library、Library 相対パスの組である。
///
/// 並びの日時が同じ Entry は、Library の id、Library 相対パスの順に並べる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineCursor {
    pub date: String,
    pub library: String,
    pub path: String,
}

impl TimelineCursor {
    const DATE_LENGTH: usize = "YYYY-MM-DDTHH:MM:SS".len();

    /// 並びの日時の 19 文字に Library の id と Library 相対パスを続けた文字列から作る。
    ///
    /// Library の id は `/` を含まないので、日時の後の最初の `/` から後が Library 相対パスである。
    pub fn parse(value: &str) -> Result<Self> {
        let invalid = || Error::new(ErrorKind::InvalidInput).with_message("invalid timeline cursor");
        let date = value.get(..Self::DATE_LENGTH).ok_or_else(invalid)?;
        let rest = value.get(Self::DATE_LENGTH..).ok_or_else(invalid)?;
        TimelineDate::parse(date, "exif", None).map_err(|_| invalid())?;
        let slash = rest.find('/').ok_or_else(invalid)?;
        let (library, path) = rest.split_at(slash);
        if library.is_empty() {
            return Err(invalid());
        }
        Ok(Self {
            date: date.to_string(),
            library: library.to_string(),
            path: path.to_string(),
        })
    }

    pub fn encode(&self) -> String {
        format!("{}{}{}", self.date, self.library, self.path)
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
    /// Library の、Library 相対パスの Entry を中央に含む範囲。
    Around { library: String, path: String },
}

/// 新しい順に並べたページと、その前後を読むための cursor。前後に Entry がなければ `None` にする。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelinePage {
    pub rows: Vec<TimelineListedRow>,
    pub previous: Option<TimelineCursor>,
    pub next: Option<TimelineCursor>,
}

/// One start of a reading: a Library with its root or a path in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineStart {
    library: String,
    /// The Library-relative path with its leading `/` and without a trailing one, or `/` for the root.
    path: String,
    /// Whether the start is an archive or a path inside one, which reads every row under it.
    in_archive: bool,
}

impl TimelineStart {
    pub fn new(library: &str, path: &str, in_archive: bool) -> Self {
        let trimmed = path.trim_matches('/');
        Self {
            library: library.to_string(),
            path: if trimmed.is_empty() { "/".to_string() } else { format!("/{trimmed}") },
            in_archive,
        }
    }

    fn is_root(&self) -> bool {
        self.path == "/"
    }
}

/// What a reading of the index keeps from the starts' rows: the Scope's starts and choices, and the kinds of the Filter.
///
/// A start that is the root of a Library reads the Library's rows, and any other start the rows under its path. Starts are not expected to
/// overlap, since the caller drops the starts that another start covers; a start given twice is read once.
#[derive(Debug, Clone)]
pub struct TimelineReading {
    starts: Vec<TimelineStart>,
    /// Whether the descendants of the starts are read, and not only their direct children.
    recursive: bool,
    /// Whether the archives found under the starts are read. A start that is in an archive reads that archive's inside regardless.
    inside_archives: bool,
    /// The kinds to keep. None given keeps both images and videos.
    kinds: Vec<TimelineKind>,
}

impl TimelineReading {
    #[cfg_attr(not(test), expect(dead_code, reason = "listEntries reads a Scope in timeline-date order through this from step 5"))]
    pub fn new(starts: Vec<TimelineStart>, recursive: bool, inside_archives: bool, kinds: Vec<TimelineKind>) -> Self {
        Self {
            starts,
            recursive,
            inside_archives,
            kinds,
        }
    }

    /// Every row of the Libraries `ids`, of both kinds, the insides of archives included.
    fn of_libraries(ids: &[String]) -> Self {
        Self {
            starts: ids.iter().map(|id| TimelineStart::new(id, "/", false)).collect(),
            recursive: true,
            inside_archives: true,
            kinds: Vec::new(),
        }
    }

    /// Whether every start is the root of a Library and the descendants are read, so that the counts held per Library can answer.
    fn is_whole_libraries(&self) -> bool {
        self.recursive && self.starts.iter().all(TimelineStart::is_root)
    }

    /// The queries that read this: the Library roots together, which the date order serves, and every other start by its path.
    ///
    /// A start that is a root reads by the date order only when the descendants are read, since direct children are found by the path.
    fn sources(&self) -> Result<Vec<TimelineSource>> {
        if self.starts.is_empty() {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("at least one start is needed"));
        }
        let mut roots = Vec::<String>::new();
        let mut paths = Vec::<&TimelineStart>::new();
        for start in &self.starts {
            if start.is_root() && self.recursive {
                if !roots.contains(&start.library) {
                    roots.push(start.library.clone());
                }
            } else if !paths.contains(&start) {
                paths.push(start);
            }
        }
        let mut sources = Vec::new();
        if !roots.is_empty() {
            sources.push(TimelineSource::new(roots, None, self, false));
        }
        for start in paths {
            sources.push(TimelineSource::new(vec![start.library.clone()], Some(&start.path), self, start.in_archive));
        }
        Ok(sources)
    }
}

/// One query's worth of a reading: the Libraries it reads, the range of paths under a start, and the conditions on the rows.
struct TimelineSource {
    libraries: Vec<String>,
    /// The lower and upper bounds of the paths under the start's path, which the start itself is not among. None for a whole Library.
    range: Option<(String, String)>,
    /// With direct children only, the position in a path, counting from 1, where the part after the start's path begins.
    direct_from: Option<i64>,
    /// Whether only rows whose path is their local file's are read, which leaves out the insides of archives.
    outside_only: bool,
    kinds: Vec<TimelineKind>,
}

impl TimelineSource {
    fn new(libraries: Vec<String>, path: Option<&str>, reading: &TimelineReading, in_archive: bool) -> Self {
        let path = path.map(|path| path.trim_end_matches('/')).filter(|path| !path.is_empty());
        let mut kinds = reading.kinds.clone();
        kinds.dedup();
        Self {
            libraries,
            // `/` is followed by `0` in the order of bytes, so the paths under `path` are exactly those from `path/` up to `path0`.
            range: path.map(|path| (format!("{path}/"), format!("{path}0"))),
            direct_from: (!reading.recursive).then(|| path.map_or(2, |path| path.chars().count() as i64 + 2)),
            // A start in an archive reads every row under it, whether or not the Scope reads the insides of archives.
            outside_only: !reading.inside_archives && !in_archive,
            kinds: if kinds.len() >= 2 { Vec::new() } else { kinds },
        }
    }

    /// The condition on the rows. The values are bound by `bind`, in the same order.
    fn condition(&self) -> String {
        let mut parts = vec![format!("library_id IN ({})", vec!["?"; self.libraries.len()].join(", "))];
        if self.range.is_some() {
            parts.push("(path >= ? AND path < ?)".to_string());
        }
        if self.direct_from.is_some() {
            parts.push("instr(substr(path, ?), '/') = 0".to_string());
        }
        if self.outside_only {
            parts.push("path = file_path".to_string());
        }
        if let [kind] = self.kinds.as_slice() {
            parts.push(format!("kind = '{}'", kind.as_str()));
        }
        parts.join(" AND ")
    }

    fn bind<'q>(&'q self, query: SqliteQuery<'q>) -> SqliteQuery<'q> {
        let mut query = self.libraries.iter().fold(query, |query, id| query.bind(id.as_str()));
        if let Some((lower, upper)) = &self.range {
            query = query.bind(lower.as_str()).bind(upper.as_str());
        }
        if let Some(from) = self.direct_from {
            query = query.bind(from);
        }
        query
    }
}

impl TimelineSource {
    /// The rows older than `cursor`, or the newest rows without one, newest first, at most `limit`.
    async fn older(&self, db: &SqlitePool, cursor: Option<&TimelineCursor>, limit: usize) -> Result<Vec<TimelineListedRow>> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let mut sql = format!("SELECT {} FROM timeline_entries WHERE {}", TimelineIndex::COLUMNS, self.condition());
        if cursor.is_some() {
            sql = format!("{sql} AND {}", TimelineIndex::OLDER_THAN_CURSOR);
        }
        let mut query = self.bind(sqlx::query(sqlx::AssertSqlSafe(format!("{sql} {} LIMIT ?", TimelineIndex::NEWEST_FIRST))));
        if let Some(cursor) = cursor {
            query = TimelineIndex::bind_cursor(query, cursor);
        }
        let rows = query.bind(i64::try_from(limit)?).fetch_all(db).await?;
        rows.iter().map(TimelineListedRow::from_row).collect()
    }

    /// The rows before `date`, newest first, at most `limit`.
    async fn older_than_date(&self, db: &SqlitePool, date: &str, limit: usize) -> Result<Vec<TimelineListedRow>> {
        let sql = format!(
            "SELECT {} FROM timeline_entries WHERE {} AND date < ? {} LIMIT ?",
            TimelineIndex::COLUMNS,
            self.condition(),
            TimelineIndex::NEWEST_FIRST
        );
        let rows = self
            .bind(sqlx::query(sqlx::AssertSqlSafe(sql)))
            .bind(date)
            .bind(i64::try_from(limit)?)
            .fetch_all(db)
            .await?;
        rows.iter().map(TimelineListedRow::from_row).collect()
    }

    /// The rows newer than `cursor` that are nearest to it, at most `limit`, newest first.
    async fn newer(&self, db: &SqlitePool, cursor: &TimelineCursor, limit: usize) -> Result<Vec<TimelineListedRow>> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT {} FROM timeline_entries WHERE {} AND {} {} LIMIT ?",
            TimelineIndex::COLUMNS,
            self.condition(),
            TimelineIndex::NEWER_THAN_CURSOR,
            TimelineIndex::OLDEST_FIRST
        );
        let query = TimelineIndex::bind_cursor(self.bind(sqlx::query(sqlx::AssertSqlSafe(sql))), cursor);
        let rows = query.bind(i64::try_from(limit)?).fetch_all(db).await?;
        let mut rows = rows.iter().map(TimelineListedRow::from_row).collect::<Result<Vec<_>>>()?;
        rows.reverse();
        Ok(rows)
    }

    /// Whether the source has a row on the side of `cursor` that `than_cursor` names.
    async fn exists(&self, db: &SqlitePool, than_cursor: &str, cursor: &TimelineCursor) -> Result<bool> {
        let sql = format!("SELECT EXISTS (SELECT 1 FROM timeline_entries WHERE {} AND {than_cursor})", self.condition());
        let row = TimelineIndex::bind_cursor(self.bind(sqlx::query(sqlx::AssertSqlSafe(sql))), cursor).fetch_one(db).await?;
        Ok(row.try_get(0)?)
    }

    /// The row at `path` of `library` when this source reads it.
    async fn row(&self, db: &SqlitePool, library: &str, path: &str) -> Result<Option<TimelineListedRow>> {
        let sql = format!(
            "SELECT {} FROM timeline_entries WHERE {} AND library_id = ? AND path = ?",
            TimelineIndex::COLUMNS,
            self.condition()
        );
        let row = self.bind(sqlx::query(sqlx::AssertSqlSafe(sql))).bind(library).bind(path).fetch_optional(db).await?;
        row.as_ref().map(TimelineListedRow::from_row).transpose()
    }
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

/// 読み出しの対象にする Library の組。空の組は作れない。
struct TimelineLibraries<'a> {
    ids: &'a [String],
}

impl<'a> TimelineLibraries<'a> {
    fn new(ids: &'a [String]) -> Result<Self> {
        if ids.is_empty() {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("at least one library is needed"));
        }
        Ok(Self { ids })
    }

    /// `library_id` が組に入る条件。引数は `bind` で bind する。
    fn condition(&self) -> String {
        format!("library_id IN ({})", vec!["?"; self.ids.len()].join(", "))
    }

    fn bind<'q>(&self, query: SqliteQuery<'q>) -> SqliteQuery<'q>
    where
        'a: 'q,
    {
        self.ids.iter().fold(query, |query, id| query.bind(id.as_str()))
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
        SqliteMigrator::migrate(db, Self::migrations()).await?;
        for trigger in TRIGGERS {
            sqlx::query(sqlx::AssertSqlSafe(*trigger)).execute(db).await?;
        }
        Ok(())
    }

    fn migrations() -> Vec<MigrationRequest> {
        vec![
            MigrationRequest {
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
            },
            MigrationRequest {
                // Library を横断して並びの日時の順に読むので、並びの索引の先頭を日時にする。
                name: "2026-10-24_timeline_cross_library_order".to_string(),
                queries: r#"
DROP INDEX timeline_entries_order;
CREATE INDEX timeline_entries_date ON timeline_entries (date DESC, library_id, path);
"#
                .to_string(),
            },
            MigrationRequest {
                // Hold the per-month counts per kind and whether the Entry is inside an archive, recounted from the rows without
                // aggregating again. The triggers of the old key are dropped here, including on a new file, which has none yet,
                // and `TRIGGERS` creates the new ones after the migrations.
                name: "2026-11-23_timeline_month_counts_by_kind".to_string(),
                queries: r#"
DROP TRIGGER IF EXISTS timeline_entries_insert;
DROP TRIGGER IF EXISTS timeline_entries_delete;
DROP TRIGGER IF EXISTS timeline_entries_update;
CREATE TABLE timeline_month_counts (
    library_id TEXT NOT NULL,
    month TEXT NOT NULL,
    kind TEXT NOT NULL,
    inside_archive INTEGER NOT NULL,
    count INTEGER NOT NULL,
    PRIMARY KEY (library_id, month, kind, inside_archive)
) WITHOUT ROWID;
INSERT INTO timeline_month_counts (library_id, month, kind, inside_archive, count)
SELECT library_id, substr(date, 1, 7), kind, path <> file_path, COUNT(*) FROM timeline_entries
GROUP BY library_id, substr(date, 1, 7), kind, path <> file_path;
DROP TABLE timeline_months;
ALTER TABLE timeline_month_counts RENAME TO timeline_months;
"#
                .to_string(),
            },
        ]
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

    /// When the date detection rules differ from the recorded ones, delete every Library's rows so the next aggregation rereads them.
    ///
    /// A missing record means the rows were indexed by a version without date rules. If rules are configured now,
    /// those rows must be reread even though there is nothing to compare the new list against.
    pub async fn reset_for_date_rules(&self, rules: &str, has_rules: bool) -> Result<()> {
        let recorded: Option<String> = sqlx::query_scalar("SELECT value FROM timeline_settings WHERE key = 'date_rules'")
            .fetch_optional(&self.db)
            .await?;
        if recorded.as_deref() == Some(rules) {
            return Ok(());
        }

        let mut tx = self.write().await?;
        if recorded.is_some() || has_rules {
            sqlx::query("DELETE FROM timeline_entries").execute(&mut *tx).await?;
            sqlx::query("DELETE FROM timeline_files").execute(&mut *tx).await?;
            sqlx::query("DELETE FROM timeline_months").execute(&mut *tx).await?;
            sqlx::query("UPDATE timeline_scans SET completed_at = NULL").execute(&mut *tx).await?;
        }
        sqlx::query("INSERT INTO timeline_settings (key, value) VALUES ('date_rules', ?) ON CONFLICT (key) DO UPDATE SET value = excluded.value")
            .bind(rules)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

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

    /// `libraries` の Entry を 1 本の並びにしたページ。`libraries` は空でない。
    pub async fn page(&self, libraries: &[String], anchor: &TimelineAnchor, limit: usize) -> Result<TimelinePage> {
        self.read(&TimelineReading::of_libraries(libraries), anchor, limit).await
    }

    /// The page of the rows of `reading`'s starts as one sequence, newest timeline date first, with the Entries of the same timeline date
    /// ordered by Library id and then by path. Starts that overlap list a row once. `reading` has at least one start.
    ///
    /// Each source reads at most `limit` rows from `anchor` and the merge keeps the first `limit` of them, which are the first `limit` of
    /// the whole sequence, since a row that fewer than `limit` rows precede in the sequence is preceded by fewer in every source holding it.
    pub async fn read(&self, reading: &TimelineReading, anchor: &TimelineAnchor, limit: usize) -> Result<TimelinePage> {
        let sources = reading.sources()?;
        let limit = limit.max(1);
        let rows = match anchor {
            TimelineAnchor::Newest => self.older(&sources, None, limit).await?,
            TimelineAnchor::After(cursor) => self.older(&sources, Some(cursor), limit).await?,
            TimelineAnchor::Before(cursor) => self.newer(&sources, cursor, limit).await?,
            TimelineAnchor::Month(month) => {
                let end = Self::month_end(month)?;
                let mut rows = Vec::new();
                for source in &sources {
                    rows.extend(source.older_than_date(&self.db, &end, limit).await?);
                }
                TimelineListedRow::merge(rows, limit)
            }
            TimelineAnchor::Around { library, path } => {
                // The named row must be one that the starts read, with their conditions applied.
                let mut center = None;
                for source in &sources {
                    if let Some(row) = source.row(&self.db, library, path).await? {
                        center = Some(row);
                        break;
                    }
                }
                let Some(center) = center else {
                    return Err(Error::new(ErrorKind::NotFound).with_message("timeline entry not found"));
                };
                let cursor = center.cursor();
                let newer_count = (limit - 1) / 2;
                let mut rows = self.newer(&sources, &cursor, newer_count).await?;
                rows.push(center);
                let older_count = limit - rows.len();
                rows.extend(self.older(&sources, Some(&cursor), older_count).await?);
                rows
            }
        };
        let previous = match rows.first() {
            Some(first) if self.exists(&sources, Self::NEWER_THAN_CURSOR, &first.cursor()).await? => Some(first.cursor()),
            _ => None,
        };
        let next = match rows.last() {
            Some(last) if self.exists(&sources, Self::OLDER_THAN_CURSOR, &last.cursor()).await? => Some(last.cursor()),
            _ => None,
        };
        Ok(TimelinePage { rows, previous, next })
    }

    /// `libraries` の月ごとの件数を足し合わせ、新しい月から並べる。`libraries` は空でない。
    pub async fn months(&self, libraries: &[String]) -> Result<Vec<(String, u64)>> {
        let libraries = TimelineLibraries::new(libraries)?;
        let sql = format!(
            "SELECT month, SUM(count) AS count FROM timeline_months WHERE {} AND count > 0 GROUP BY month ORDER BY month DESC",
            libraries.condition()
        );
        let rows = libraries.bind(sqlx::query(sqlx::AssertSqlSafe(sql))).fetch_all(&self.db).await?;
        rows.iter()
            .map(|row| {
                let count: i64 = row.try_get("count")?;
                Ok((row.try_get("month")?, u64::try_from(count)?))
            })
            .collect()
    }

    /// The number of rows of `reading` in each month, newest month first, without the months that have none. `reading` has at least one start.
    ///
    /// When every start is a Library root and the descendants are read, these are the sums of the counts held per Library, month, kind, and
    /// inside-archive, over the kinds kept and, unless the insides of archives are read, over rows outside archives. Otherwise the rows that
    /// the same reading returns are counted, which costs the rows under the starts, so the starts must not overlap, since a row two starts read
    /// would be counted twice.
    #[cfg_attr(not(test), expect(dead_code, reason = "listEntryMonths counts the months of a Scope through this from step 5"))]
    pub async fn count_months(&self, reading: &TimelineReading) -> Result<Vec<(String, u64)>> {
        let sources = reading.sources()?;
        let rows = if reading.is_whole_libraries() {
            let [source] = sources.as_slice() else {
                return Err(Error::new(ErrorKind::UnexpectedError).with_message("the roots of a reading are one source"));
            };
            let mut conditions = vec![format!("library_id IN ({})", vec!["?"; source.libraries.len()].join(", ")), "count > 0".to_string()];
            if let [kind] = source.kinds.as_slice() {
                conditions.push(format!("kind = '{}'", kind.as_str()));
            }
            if source.outside_only {
                conditions.push("inside_archive = 0".to_string());
            }
            let sql = format!("SELECT month, SUM(count) AS count FROM timeline_months WHERE {} GROUP BY month", conditions.join(" AND "));
            let query = source.libraries.iter().fold(sqlx::query(sqlx::AssertSqlSafe(sql)), |query, id| query.bind(id.as_str()));
            query.fetch_all(&self.db).await?
        } else {
            let mut rows = Vec::new();
            for source in &sources {
                let sql = format!(
                    "SELECT substr(date, 1, 7) AS month, COUNT(*) AS count FROM timeline_entries WHERE {} GROUP BY substr(date, 1, 7)",
                    source.condition()
                );
                rows.extend(source.bind(sqlx::query(sqlx::AssertSqlSafe(sql))).fetch_all(&self.db).await?);
            }
            rows
        };
        let mut months = std::collections::BTreeMap::<String, u64>::new();
        for row in &rows {
            let count: i64 = row.try_get("count")?;
            *months.entry(row.try_get("month")?).or_default() += u64::try_from(count)?;
        }
        Ok(months.into_iter().rev().filter(|(_, count)| *count > 0).collect())
    }

    /// The rows of `paths` in `library`, found by the primary key in batches. A path without a row is not in the result.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "the timeline-date order of a snapshot reads its matches' rows through this from step 5")
    )]
    pub async fn rows_of(&self, library: &str, paths: &[String]) -> Result<HashMap<String, TimelineRow>> {
        let mut rows = HashMap::with_capacity(paths.len());
        for chunk in paths.chunks(CHUNK) {
            let sql = format!(
                "SELECT {} FROM timeline_entries WHERE library_id = ? AND path IN ({})",
                Self::COLUMNS,
                vec!["?"; chunk.len()].join(", ")
            );
            let query = chunk
                .iter()
                .fold(sqlx::query(sqlx::AssertSqlSafe(sql)).bind(library), |query, path| query.bind(path.as_str()));
            for row in query.fetch_all(&self.db).await? {
                let row = TimelineRow::from_row(&row)?;
                rows.insert(row.path.clone(), row);
            }
        }
        Ok(rows)
    }

    const COLUMNS: &'static str = "library_id, path, file_path, kind, date, date_source, date_instant, size, modified_ns, readonly";

    /// 並びの順。索引 `timeline_entries_date` の順と同じにする。
    const NEWEST_FIRST: &'static str = "ORDER BY date DESC, library_id ASC, path ASC";
    const OLDEST_FIRST: &'static str = "ORDER BY date ASC, library_id DESC, path DESC";

    /// cursor より古い Entry の条件。引数は日時、日時、Library、Library 相対パスの順に bind する。
    const OLDER_THAN_CURSOR: &'static str = "(date < ? OR (date = ? AND (library_id, path) > (?, ?)))";
    /// cursor より新しい Entry の条件。引数は `OLDER_THAN_CURSOR` と同じ順に bind する。
    const NEWER_THAN_CURSOR: &'static str = "(date > ? OR (date = ? AND (library_id, path) < (?, ?)))";

    /// Rows older than `cursor`, or the newest rows without one, merged from every source, newest first.
    async fn older(&self, sources: &[TimelineSource], cursor: Option<&TimelineCursor>, limit: usize) -> Result<Vec<TimelineListedRow>> {
        let mut rows = Vec::new();
        for source in sources {
            rows.extend(source.older(&self.db, cursor, limit).await?);
        }
        Ok(TimelineListedRow::merge(rows, limit))
    }

    /// The `limit` rows newer than `cursor` that are nearest to it, merged from every source, newest first.
    async fn newer(&self, sources: &[TimelineSource], cursor: &TimelineCursor, limit: usize) -> Result<Vec<TimelineListedRow>> {
        let mut rows = Vec::new();
        for source in sources {
            rows.extend(source.newer(&self.db, cursor, limit).await?);
        }
        let merged = TimelineListedRow::merge(rows, usize::MAX);
        let skipped = merged.len().saturating_sub(limit);
        Ok(merged.into_iter().skip(skipped).collect())
    }

    /// Whether any source has a row on the side of `cursor` that `than_cursor` names.
    async fn exists(&self, sources: &[TimelineSource], than_cursor: &str, cursor: &TimelineCursor) -> Result<bool> {
        for source in sources {
            if source.exists(&self.db, than_cursor, cursor).await? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn bind_cursor<'q>(query: SqliteQuery<'q>, cursor: &'q TimelineCursor) -> SqliteQuery<'q> {
        query.bind(&cursor.date).bind(&cursor.date).bind(&cursor.library).bind(&cursor.path)
    }

    /// `YYYY-MM` の翌月の 1 日 0 時。その月の Entry はすべてこれより前に並ぶ。
    pub fn month_end(month: &str) -> Result<String> {
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

    fn lib() -> Vec<String> {
        vec!["lib".to_string()]
    }

    fn media(size: u64) -> TimelineFileVersion {
        TimelineFileVersion::Media { size, modified_ns: 1 }
    }

    async fn index(dir: &tempfile::TempDir) -> Result<TimelineIndex> {
        TimelineIndex::open(dir.path()).await
    }

    async fn insert(index: &TimelineIndex, rows: &[TimelineRow]) -> Result<()> {
        insert_into(index, "lib", rows).await
    }

    async fn insert_into(index: &TimelineIndex, library: &str, rows: &[TimelineRow]) -> Result<()> {
        let mut batch = TimelineBatch::default();
        for row in rows {
            if !batch.files.iter().any(|(path, _)| path == &row.file_path) {
                batch.replaced.push(row.file_path.clone());
                batch.files.push((row.file_path.clone(), media(1)));
            }
            batch.rows.push(row.clone());
        }
        index.apply(library, 1, &batch).await
    }

    fn paths(page: &TimelinePage) -> Vec<&str> {
        page.rows.iter().map(|listed| listed.row.path.as_str()).collect()
    }

    async fn month_counts(index: &TimelineIndex) -> Result<Vec<(String, u64)>> {
        index.months(&lib()).await
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
        let mut page = index.page(&lib(), &TimelineAnchor::Newest, 3).await?;
        assert_eq!(page.previous, None);
        loop {
            seen.extend(paths(&page).into_iter().map(str::to_string));
            let Some(next) = page.next.clone() else { break };
            page = index.page(&lib(), &TimelineAnchor::After(next), 3).await?;
        }
        let all = index.page(&lib(), &TimelineAnchor::Newest, 100).await?;
        assert_eq!(seen, paths(&all));
        assert_eq!(seen.len(), 7);

        // 末尾から前へ読み戻しても、同じ並びになる。
        let last = all.rows.last().unwrap().cursor();
        let mut back = vec![last.path.clone()];
        let mut page = index.page(&lib(), &TimelineAnchor::Before(last), 2).await?;
        loop {
            let mut current = paths(&page).into_iter().map(str::to_string).collect::<Vec<_>>();
            current.extend(back);
            back = current;
            let Some(previous) = page.previous.clone() else { break };
            page = index.page(&lib(), &TimelineAnchor::Before(previous), 2).await?;
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

        let page = index.page(&lib(), &TimelineAnchor::Month("2024-02".to_string()), 2).await?;
        assert_eq!(paths(&page), ["/b.jpg", "/c.jpg"]);
        assert!(page.previous.is_some() && page.next.is_some());
        let page = index.page(&lib(), &TimelineAnchor::Month("2024-01".to_string()), 2).await?;
        assert_eq!(paths(&page), ["/d.jpg"], "空の月はそれより古い Entry から");

        let page = index
            .page(
                &lib(),
                &TimelineAnchor::Around {
                    library: "lib".to_string(),
                    path: "/c.jpg".to_string(),
                },
                3,
            )
            .await?;
        assert_eq!(paths(&page), ["/b.jpg", "/c.jpg", "/d.jpg"]);
        assert!(index.page(&lib(), &TimelineAnchor::Month("2024-13".to_string()), 2).await.is_err());
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
        let page = index.page(&lib(), &TimelineAnchor::Newest, 10).await?;
        assert_eq!(paths(&page), ["/older.jpg", "/new/x.zip/b.jpg", "/new/a.jpg"], "`/older` は範囲に入らない");
        assert_eq!(page.rows[1].row.file_path, "/new/x.zip");
        assert_eq!(page.rows[1].row.date.source, TimelineDateSource::Exif, "日時は読み直さない");

        // 付け替えた行は未確認なので、範囲の集計が触れなければ掃除で消える。
        let batch = TimelineBatch {
            touched: vec!["/new/a.jpg".to_string()],
            ..Default::default()
        };
        index.apply("lib", 5, &batch).await?;
        index.sweep("lib", "/new", 5, true).await?;
        assert_eq!(paths(&index.page(&lib(), &TimelineAnchor::Newest, 10).await?), ["/older.jpg", "/new/a.jpg"]);
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
        assert_eq!(paths(&index.page(&lib(), &TimelineAnchor::Newest, 10).await?), ["/late.jpg"]);
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
            paths(&index.page(&lib(), &TimelineAnchor::Newest, 10).await?),
            ["/b.jpg"],
            "集計の最中に付け替えた行は、全体の掃除で消えない"
        );
        assert_eq!(month_counts(&index).await?, [("2024-01".to_string(), 1)]);

        // 範囲の集計が触れなかった行は、対象外の名前になったものとして消える。
        index.sweep("lib", "/b.jpg", scan, true).await?;
        assert!(paths(&index.page(&lib(), &TimelineAnchor::Newest, 10).await?).is_empty());

        // 次の全体の集計は、その前に付け替えて判定されなかった行を消す。
        insert(&index, &[row("/c.jpg", "/c.jpg", "2024-01-01T00:00:00")]).await?;
        index.rename("lib", "/c.jpg", "/d.jpg").await?;
        let next = index.begin_scan("lib").await?;
        index.sweep("lib", "/", next, false).await?;
        assert!(paths(&index.page(&lib(), &TimelineAnchor::Newest, 10).await?).is_empty());
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
        assert!(paths(&index.page(&lib(), &TimelineAnchor::Newest, 10).await?).is_empty());
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
        assert_eq!(paths(&index.page(&lib(), &TimelineAnchor::Newest, 10).await?), ["/dst.zip/a.jpg"]);
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
        modified.date = TimelineDate::choose(&utc, None, None, Some(instant), None).unwrap();
        insert(&index, &[modified, row("/e.jpg", "/e.jpg", "2024-01-15T00:00:00")]).await?;
        assert_eq!(month_counts(&index).await?, [("2024-01".to_string(), 2)]);

        let tokyo = TimelineZone::new(Some("Asia/Tokyo"))?;
        index.rezone(&tokyo).await?;
        let page = index.page(&lib(), &TimelineAnchor::Newest, 10).await?;
        assert_eq!(page.rows[0].row.date.text(), "2024-02-01T05:00:00");
        assert_eq!(page.rows[1].row.date.text(), "2024-01-15T00:00:00", "Exif の値は直さない");
        assert_eq!(month_counts(&index).await?, [("2024-02".to_string(), 1), ("2024-01".to_string(), 1)]);
        Ok(())
    }

    #[tokio::test]
    async fn reset_for_date_rules_clears_rows_when_the_record_is_missing_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = index(&dir).await?;
        insert(&index, &[row("/a.jpg", "/a.jpg", "2024-01-01T00:00:00")]).await?;

        index.reset_for_date_rules("[]", false).await?;
        assert_eq!(
            paths(&index.page(&lib(), &TimelineAnchor::Newest, 10).await?),
            ["/a.jpg"],
            "no rules and no record keeps the rows"
        );

        // An index written before date rules existed has no record at all.
        sqlx::query("DELETE FROM timeline_settings WHERE key = 'date_rules'").execute(&index.db).await?;
        index.reset_for_date_rules(r#"[{"path":"**"}]"#, true).await?;
        assert!(paths(&index.page(&lib(), &TimelineAnchor::Newest, 10).await?).is_empty());
        assert!(month_counts(&index).await?.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn libraries_are_merged_into_one_order_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = index(&dir).await?;
        insert_into(
            &index,
            "a",
            &[row("/1.jpg", "/1.jpg", "2024-03-01T00:00:00"), row("/same.jpg", "/same.jpg", "2024-02-01T00:00:00")],
        )
        .await?;
        insert_into(
            &index,
            "b",
            &[row("/2.jpg", "/2.jpg", "2024-02-15T00:00:00"), row("/same.jpg", "/same.jpg", "2024-02-01T00:00:00")],
        )
        .await?;
        insert_into(&index, "c", &[row("/3.jpg", "/3.jpg", "2024-02-20T00:00:00")]).await?;
        let both = vec!["b".to_string(), "a".to_string()];
        let listed = |page: &TimelinePage| page.rows.iter().map(|listed| format!("{}{}", listed.library, listed.row.path)).collect::<Vec<_>>();

        let all = index.page(&both, &TimelineAnchor::Newest, 10).await?;
        assert_eq!(listed(&all), ["a/1.jpg", "b/2.jpg", "a/same.jpg", "b/same.jpg"], "同じ日時は Library の id の順");

        // 同じ日時、同じ path の Entry が Library の境目にあっても、ページは重ならず飛ばさない。
        let mut seen = Vec::new();
        let mut page = index.page(&both, &TimelineAnchor::Newest, 1).await?;
        loop {
            seen.extend(listed(&page));
            let Some(next) = page.next.clone() else { break };
            page = index.page(&both, &TimelineAnchor::After(next), 1).await?;
        }
        assert_eq!(seen, listed(&all));
        let last = all.rows.last().unwrap().cursor();
        let previous = index.page(&both, &TimelineAnchor::Before(last), 1).await?;
        assert_eq!(listed(&previous), ["a/same.jpg"]);

        let around = TimelineAnchor::Around {
            library: "b".to_string(),
            path: "/same.jpg".to_string(),
        };
        assert_eq!(listed(&index.page(&both, &around, 3).await?), ["a/same.jpg", "b/same.jpg"]);
        let outside = TimelineAnchor::Around {
            library: "c".to_string(),
            path: "/3.jpg".to_string(),
        };
        assert_eq!(*index.page(&both, &outside, 3).await.unwrap_err().kind(), ErrorKind::NotFound, "組の外の Library の Entry");

        assert_eq!(index.months(&both).await?, [("2024-03".to_string(), 1), ("2024-02".to_string(), 3)]);
        assert!(index.page(&[], &TimelineAnchor::Newest, 10).await.is_err());
        Ok(())
    }

    /// The triggers of the counts as they were before the counts were held per kind and inside-archive.
    const OLD_TRIGGERS: &[&str] = &[
        r#"
CREATE TRIGGER timeline_entries_insert AFTER INSERT ON timeline_entries BEGIN
    INSERT OR IGNORE INTO timeline_months (library_id, month, count) VALUES (NEW.library_id, substr(NEW.date, 1, 7), 0);
    UPDATE timeline_months SET count = count + 1 WHERE library_id = NEW.library_id AND month = substr(NEW.date, 1, 7);
END
"#,
        r#"
CREATE TRIGGER timeline_entries_delete AFTER DELETE ON timeline_entries BEGIN
    UPDATE timeline_months SET count = count - 1 WHERE library_id = OLD.library_id AND month = substr(OLD.date, 1, 7);
    DELETE FROM timeline_months WHERE library_id = OLD.library_id AND month = substr(OLD.date, 1, 7) AND count <= 0;
END
"#,
        r#"
CREATE TRIGGER timeline_entries_update AFTER UPDATE OF library_id, date ON timeline_entries
WHEN OLD.library_id <> NEW.library_id OR substr(OLD.date, 1, 7) <> substr(NEW.date, 1, 7) BEGIN
    UPDATE timeline_months SET count = count - 1 WHERE library_id = OLD.library_id AND month = substr(OLD.date, 1, 7);
    DELETE FROM timeline_months WHERE library_id = OLD.library_id AND month = substr(OLD.date, 1, 7) AND count <= 0;
    INSERT OR IGNORE INTO timeline_months (library_id, month, count) VALUES (NEW.library_id, substr(NEW.date, 1, 7), 0);
    UPDATE timeline_months SET count = count + 1 WHERE library_id = NEW.library_id AND month = substr(NEW.date, 1, 7);
END
"#,
    ];

    type MonthCount = (String, String, String, i64, i64);

    /// The counts the index holds, as (library, month, kind, inside-archive, count).
    async fn held_counts(index: &TimelineIndex) -> Result<Vec<MonthCount>> {
        let rows = sqlx::query("SELECT library_id, month, kind, inside_archive, count FROM timeline_months ORDER BY library_id, month, kind, inside_archive")
            .fetch_all(&index.db)
            .await?;
        rows.iter()
            .map(|row| {
                Ok((
                    row.try_get("library_id")?,
                    row.try_get("month")?,
                    row.try_get("kind")?,
                    row.try_get("inside_archive")?,
                    row.try_get("count")?,
                ))
            })
            .collect()
    }

    /// The rows counted by Library, month, kind, and whether their path differs from their local file's.
    async fn counted_counts(index: &TimelineIndex) -> Result<Vec<MonthCount>> {
        let rows = sqlx::query(
            "SELECT library_id, substr(date, 1, 7) AS month, kind, path <> file_path AS inside_archive, COUNT(*) AS count FROM timeline_entries GROUP BY 1, 2, 3, 4 ORDER BY 1, 2, 3, 4",
        )
        .fetch_all(&index.db)
        .await?;
        rows.iter()
            .map(|row| {
                Ok((
                    row.try_get("library_id")?,
                    row.try_get("month")?,
                    row.try_get("kind")?,
                    row.try_get("inside_archive")?,
                    row.try_get("count")?,
                ))
            })
            .collect()
    }

    async fn assert_counts_equal_rows(index: &TimelineIndex) -> Result<()> {
        assert_eq!(held_counts(index).await?, counted_counts(index).await?);
        Ok(())
    }

    fn video(path: &str, file_path: &str, date: &str) -> TimelineRow {
        TimelineRow {
            kind: TimelineKind::Video,
            ..row(path, file_path, date)
        }
    }

    /// Writes one row of Library `lib` as the upsert of aggregation does, without deleting the rows of its local file first.
    async fn upsert(index: &TimelineIndex, row: TimelineRow) -> Result<()> {
        let mut tx = index.write().await?;
        TimelineIndex::insert_row(&mut tx, "lib", &row).await?;
        tx.commit().await?;
        Ok(())
    }

    fn mixed_rows() -> Vec<TimelineRow> {
        vec![
            row("/a.jpg", "/a.jpg", "2024-01-01T00:00:00"),
            row("/b.jpg", "/b.jpg", "2024-01-02T00:00:00"),
            video("/c.mp4", "/c.mp4", "2024-01-03T00:00:00"),
            row("/x.zip/1.jpg", "/x.zip", "2024-01-04T00:00:00"),
            video("/x.zip/2.mp4", "/x.zip", "2024-02-05T00:00:00"),
            row("/x.zip/3.jpg", "/x.zip", "2024-02-06T00:00:00"),
            row("/d.jpg", "/d.jpg", "2024-02-07T00:00:00"),
        ]
    }

    #[tokio::test]
    async fn an_index_with_the_old_counts_is_migrated_to_counts_equal_to_the_rows_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        // An index created by the code before the counts held the kind and inside-archive: the first two migrations and the old triggers.
        let options = SqliteConnectOptions::new()
            .filename(dir.path().join(DATABASE_FILE_NAME))
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal);
        let old = SqlitePool::connect_with(options).await?;
        SqliteMigrator::migrate(&old, TimelineIndex::migrations().into_iter().take(2).collect()).await?;
        for trigger in OLD_TRIGGERS {
            sqlx::query(sqlx::AssertSqlSafe(*trigger)).execute(&old).await?;
        }
        for (library, row) in [("a", mixed_rows()), ("b", vec![row("/e.jpg", "/e.jpg", "2024-02-08T00:00:00")])]
            .into_iter()
            .flat_map(|(library, rows)| rows.into_iter().map(move |row| (library, row)))
        {
            sqlx::query("INSERT INTO timeline_entries (library_id, path, file_path, kind, date, date_source, size, readonly) VALUES (?, ?, ?, ?, ?, 'exif', 1, 0)")
                .bind(library)
                .bind(&row.path)
                .bind(&row.file_path)
                .bind(row.kind.as_str())
                .bind(row.date.text())
                .execute(&old)
                .await?;
        }
        let before: Vec<(String, i64)> = sqlx::query_as("SELECT month, SUM(count) FROM timeline_months GROUP BY month ORDER BY month DESC")
            .fetch_all(&old)
            .await?;
        assert_eq!(before, [("2024-02".to_string(), 4), ("2024-01".to_string(), 4)]);
        old.close().await;

        let index = TimelineIndex::open(dir.path()).await?;
        assert_counts_equal_rows(&index).await?;
        assert_eq!(held_counts(&index).await?.len(), 7, "per Library, month, kind, and inside-archive");
        assert_eq!(
            index.months(&["a".to_string(), "b".to_string()]).await?,
            [("2024-02".to_string(), 4), ("2024-01".to_string(), 4)],
            "listTimelineMonths returns what it returned before the migration"
        );
        assert_eq!(index.months(&["a".to_string()]).await?, [("2024-02".to_string(), 3), ("2024-01".to_string(), 4)]);

        // The new triggers replaced the old ones and keep the counts afterwards.
        insert_into(&index, "a", &[video("/y.zip/1.mp4", "/y.zip", "2024-03-01T00:00:00")]).await?;
        index.remove("a", "/x.zip").await?;
        assert_counts_equal_rows(&index).await?;
        let triggers: Vec<String> = sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type = 'trigger' ORDER BY name")
            .fetch_all(&index.db)
            .await?;
        assert_eq!(triggers, ["timeline_entries_delete", "timeline_entries_insert", "timeline_entries_update"]);

        // Opening it again does not migrate again.
        drop(index);
        let index = TimelineIndex::open(dir.path()).await?;
        assert_counts_equal_rows(&index).await?;
        Ok(())
    }

    #[tokio::test]
    async fn a_new_index_holds_counts_per_kind_and_inside_archive_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = index(&dir).await?;
        insert(&index, &mixed_rows()).await?;
        assert_counts_equal_rows(&index).await?;
        assert_eq!(
            held_counts(&index).await?,
            [
                ("lib".to_string(), "2024-01".to_string(), "image".to_string(), 0, 2),
                ("lib".to_string(), "2024-01".to_string(), "image".to_string(), 1, 1),
                ("lib".to_string(), "2024-01".to_string(), "video".to_string(), 0, 1),
                ("lib".to_string(), "2024-02".to_string(), "image".to_string(), 0, 1),
                ("lib".to_string(), "2024-02".to_string(), "image".to_string(), 1, 1),
                ("lib".to_string(), "2024-02".to_string(), "video".to_string(), 1, 1),
            ]
        );
        assert_eq!(index.months(&lib()).await?, [("2024-02".to_string(), 3), ("2024-01".to_string(), 4)]);
        Ok(())
    }

    #[tokio::test]
    async fn counts_follow_deletes_redating_and_upserts_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = index(&dir).await?;
        insert(&index, &mixed_rows()).await?;

        // Re-dating moves a count to another month, whose count may exist already.
        upsert(&index, row("/a.jpg", "/a.jpg", "2024-02-20T00:00:00")).await?;
        assert_counts_equal_rows(&index).await?;
        // An upsert that changes only the kind moves a count to the other kind.
        upsert(&index, video("/b.jpg", "/b.jpg", "2024-01-02T00:00:00")).await?;
        assert_counts_equal_rows(&index).await?;
        // An upsert that changes only the local file moves a count between outside and inside an archive, in both directions.
        upsert(&index, row("/d.jpg", "/d.zip", "2024-02-07T00:00:00")).await?;
        assert_counts_equal_rows(&index).await?;
        upsert(&index, row("/x.zip/1.jpg", "/x.zip/1.jpg", "2024-01-04T00:00:00")).await?;
        assert_counts_equal_rows(&index).await?;
        // Changing the date, kind, and local file at once, and changing nothing, both keep the counts.
        upsert(&index, video("/c.mp4", "/c.zip", "2023-12-31T23:59:59")).await?;
        upsert(&index, row("/x.zip/3.jpg", "/x.zip", "2024-02-06T00:00:00")).await?;
        assert_counts_equal_rows(&index).await?;
        // Rows that a re-read replaces through the batch are counted too.
        insert(&index, &[row("/a.jpg", "/a.jpg", "2024-01-20T00:00:00"), video("/b.jpg", "/b.jpg", "2024-03-02T00:00:00")]).await?;
        assert_counts_equal_rows(&index).await?;

        // A rename keeps the Library, month, kind, and inside-archive of every row, and a Library change moves its rows.
        index.rename("lib", "/x.zip", "/z.zip").await?;
        assert_counts_equal_rows(&index).await?;
        sqlx::query("UPDATE timeline_entries SET library_id = 'other' WHERE path = '/e.jpg' OR path = '/a.jpg'")
            .execute(&index.db)
            .await?;
        assert_counts_equal_rows(&index).await?;

        // A replaced archive and removed paths take their counts away, and a count that reaches 0 leaves no row.
        let batch = TimelineBatch {
            replaced: vec!["/z.zip".to_string()],
            files: vec![("/z.zip".to_string(), TimelineFileVersion::Archive("id".to_string()))],
            rows: vec![video("/z.zip/9.mp4", "/z.zip", "2022-05-05T00:00:00")],
            ..Default::default()
        };
        index.apply("lib", 1, &batch).await?;
        index.remove("lib", "/b.jpg").await?;
        assert_counts_equal_rows(&index).await?;
        assert!(held_counts(&index).await?.iter().all(|count| count.4 > 0));
        index.remove("lib", "/").await?;
        assert_counts_equal_rows(&index).await?;
        Ok(())
    }

    fn start(library: &str, path: &str) -> TimelineStart {
        TimelineStart::new(library, path, false)
    }

    fn archive_start(library: &str, path: &str) -> TimelineStart {
        TimelineStart::new(library, path, true)
    }

    /// The Library and the path of every row of a page, as `library:path`.
    fn listed(page: &TimelinePage) -> Vec<String> {
        page.rows.iter().map(|listed| format!("{}:{}", listed.library, listed.row.path)).collect()
    }

    fn reading(starts: Vec<TimelineStart>, recursive: bool, inside_archives: bool, kinds: Vec<TimelineKind>) -> TimelineReading {
        TimelineReading::new(starts, recursive, inside_archives, kinds)
    }

    /// Two Libraries with folders, an archive that holds a nested archive, images, and videos. `a` and `b` have rows of the same path and
    /// timeline date, and `/f2` is a folder whose name starts with `/f`.
    async fn scope_index(dir: &tempfile::TempDir) -> Result<TimelineIndex> {
        let index = index(dir).await?;
        insert_into(
            &index,
            "a",
            &[
                row("/top1.jpg", "/top1.jpg", "2024-03-10T00:00:00"),
                video("/top2.mp4", "/top2.mp4", "2024-03-09T00:00:00"),
                row("/f/p1.jpg", "/f/p1.jpg", "2024-03-08T00:00:00"),
                video("/f/q.mp4", "/f/q.mp4", "2024-03-07T00:00:00"),
                row("/f/sub/p2.jpg", "/f/sub/p2.jpg", "2024-03-06T00:00:00"),
                row("/f/pack.zip/i1.jpg", "/f/pack.zip", "2024-03-05T00:00:00"),
                row("/f/pack.zip/nested.7z/n1.jpg", "/f/pack.zip", "2024-03-04T00:00:00"),
                row("/g/x.jpg", "/g/x.jpg", "2024-03-03T00:00:00"),
                video("/f/pack.zip/dir/i2.mp4", "/f/pack.zip", "2024-03-02T00:00:00"),
                row("/f2/other.jpg", "/f2/other.jpg", "2024-03-01T00:00:00"),
            ],
        )
        .await?;
        insert_into(
            &index,
            "b",
            &[
                row("/top1.jpg", "/top1.jpg", "2024-03-10T00:00:00"),
                row("/f/p1.jpg", "/f/p1.jpg", "2024-03-08T00:00:00"),
                row("/y.zip/z.jpg", "/y.zip", "2024-02-20T00:00:00"),
            ],
        )
        .await?;
        Ok(index)
    }

    async fn read_all(index: &TimelineIndex, reading: &TimelineReading) -> Result<Vec<String>> {
        Ok(listed(&index.read(reading, &TimelineAnchor::Newest, 100).await?))
    }

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[tokio::test]
    async fn library_roots_are_read_together_by_the_date_order_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = scope_index(&dir).await?;
        let roots = vec![start("b", "/"), start("a", "/")];

        let all = read_all(&index, &reading(roots.clone(), true, true, vec![])).await?;
        assert_eq!(
            all,
            strings(&[
                "a:/top1.jpg",
                "b:/top1.jpg",
                "a:/top2.mp4",
                "a:/f/p1.jpg",
                "b:/f/p1.jpg",
                "a:/f/q.mp4",
                "a:/f/sub/p2.jpg",
                "a:/f/pack.zip/i1.jpg",
                "a:/f/pack.zip/nested.7z/n1.jpg",
                "a:/g/x.jpg",
                "a:/f/pack.zip/dir/i2.mp4",
                "a:/f2/other.jpg",
                "b:/y.zip/z.jpg",
            ]),
            "the same timeline date by Library id, then by path"
        );
        // The page of whole Libraries that `listTimelineEntries` reads is this reading.
        let page = index.page(&strings(&["b", "a"]), &TimelineAnchor::Newest, 100).await?;
        assert_eq!(listed(&page), all);

        // Without the insides of archives only the rows whose path is their local file's are read.
        let outside = read_all(&index, &reading(roots.clone(), true, false, vec![])).await?;
        assert_eq!(outside.len(), 9);
        assert!(outside.iter().all(|row| !row.contains(".zip/") && !row.contains(".7z/")));

        // The kinds.
        let videos = read_all(&index, &reading(roots.clone(), true, true, vec![TimelineKind::Video])).await?;
        assert_eq!(videos, strings(&["a:/top2.mp4", "a:/f/q.mp4", "a:/f/pack.zip/dir/i2.mp4"]));
        let images = read_all(&index, &reading(roots.clone(), true, false, vec![TimelineKind::Image])).await?;
        assert_eq!(images.len(), 7);
        let both = read_all(&index, &reading(roots, true, true, vec![TimelineKind::Image, TimelineKind::Video])).await?;
        assert_eq!(both, all);
        Ok(())
    }

    #[tokio::test]
    async fn a_folder_start_reads_the_rows_under_its_path_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = scope_index(&dir).await?;
        let folder = vec![start("a", "/f")];

        assert_eq!(
            read_all(&index, &reading(folder.clone(), true, false, vec![])).await?,
            strings(&["a:/f/p1.jpg", "a:/f/q.mp4", "a:/f/sub/p2.jpg"]),
            "no Entry inside an archive without insideArchives, and nothing of `/f2`"
        );
        assert_eq!(
            read_all(&index, &reading(folder.clone(), true, true, vec![])).await?,
            strings(&[
                "a:/f/p1.jpg",
                "a:/f/q.mp4",
                "a:/f/sub/p2.jpg",
                "a:/f/pack.zip/i1.jpg",
                "a:/f/pack.zip/nested.7z/n1.jpg",
                "a:/f/pack.zip/dir/i2.mp4",
            ])
        );
        // Direct children only: the insides of an archive among them are descendants, with or without insideArchives.
        for inside_archives in [false, true] {
            assert_eq!(
                read_all(&index, &reading(folder.clone(), false, inside_archives, vec![])).await?,
                strings(&["a:/f/p1.jpg", "a:/f/q.mp4"])
            );
        }
        assert_eq!(read_all(&index, &reading(folder, false, true, vec![TimelineKind::Video])).await?, strings(&["a:/f/q.mp4"]));
        // The direct children of a root, and a start given with a trailing `/` in another Library.
        assert_eq!(
            read_all(&index, &reading(vec![start("a", "/")], false, true, vec![])).await?,
            strings(&["a:/top1.jpg", "a:/top2.mp4"])
        );
        assert_eq!(read_all(&index, &reading(vec![start("b", "/f/")], true, false, vec![])).await?, strings(&["b:/f/p1.jpg"]));
        Ok(())
    }

    #[tokio::test]
    async fn an_archive_start_reads_every_row_under_it_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = scope_index(&dir).await?;
        let archive = vec![archive_start("a", "/f/pack.zip")];
        let everything = strings(&["a:/f/pack.zip/i1.jpg", "a:/f/pack.zip/nested.7z/n1.jpg", "a:/f/pack.zip/dir/i2.mp4"]);

        for inside_archives in [false, true] {
            assert_eq!(
                read_all(&index, &reading(archive.clone(), true, inside_archives, vec![])).await?,
                everything,
                "the nested archive's Entries too, whether or not the Scope reads the insides of archives"
            );
        }
        assert_eq!(read_all(&index, &reading(archive.clone(), false, false, vec![])).await?, strings(&["a:/f/pack.zip/i1.jpg"]));
        assert_eq!(
            read_all(&index, &reading(archive, true, false, vec![TimelineKind::Video])).await?,
            strings(&["a:/f/pack.zip/dir/i2.mp4"])
        );
        // A path inside an archive.
        assert_eq!(
            read_all(&index, &reading(vec![archive_start("a", "/f/pack.zip/dir")], true, false, vec![])).await?,
            strings(&["a:/f/pack.zip/dir/i2.mp4"])
        );
        // A folder start and an archive start under it read disjoint rows when the Scope does not read the insides of archives.
        let both = vec![start("a", "/f"), archive_start("a", "/f/pack.zip")];
        assert_eq!(read_all(&index, &reading(both, true, false, vec![])).await?.len(), 3 + 3);
        Ok(())
    }

    #[tokio::test]
    async fn starts_of_both_shapes_are_merged_and_a_repeated_start_is_read_once_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = scope_index(&dir).await?;
        let starts = vec![start("b", "/"), start("a", "/g"), start("a", "/f"), start("a", "/g")];
        assert_eq!(
            read_all(&index, &reading(starts, true, false, vec![])).await?,
            strings(&["b:/top1.jpg", "a:/f/p1.jpg", "b:/f/p1.jpg", "a:/f/q.mp4", "a:/f/sub/p2.jpg", "a:/g/x.jpg"])
        );
        // Starts that overlap list a row once.
        let overlapping = vec![start("a", "/"), start("a", "/f")];
        assert_eq!(read_all(&index, &reading(overlapping, true, true, vec![])).await?.len(), 10);
        assert!(
            index.read(&reading(vec![], true, true, vec![]), &TimelineAnchor::Newest, 10).await.is_err(),
            "a reading has a start"
        );
        Ok(())
    }

    #[tokio::test]
    async fn cursors_join_without_a_repeat_or_a_gap_over_every_shape_of_start_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = scope_index(&dir).await?;
        let readings = [
            reading(vec![start("b", "/"), start("a", "/f"), start("a", "/g")], true, true, vec![]),
            reading(vec![start("b", "/"), start("a", "/")], true, false, vec![]),
            reading(vec![archive_start("a", "/f/pack.zip"), start("b", "/")], true, false, vec![TimelineKind::Image]),
            reading(vec![start("a", "/f"), start("b", "/f")], false, true, vec![]),
        ];
        for reading in &readings {
            let all = read_all(&index, reading).await?;
            for limit in 1..=4 {
                let mut forward = Vec::new();
                let mut page = index.read(reading, &TimelineAnchor::Newest, limit).await?;
                assert_eq!(page.previous, None);
                loop {
                    forward.extend(listed(&page));
                    let Some(next) = page.next.clone() else { break };
                    page = index.read(reading, &TimelineAnchor::After(next), limit).await?;
                }
                assert_eq!(forward, all, "forward by {limit}");

                let last = index.read(reading, &TimelineAnchor::Newest, 100).await?.rows.last().unwrap().cursor();
                let mut backward = vec![format!("{}:{}", last.library, last.path)];
                let mut page = index.read(reading, &TimelineAnchor::Before(last), limit).await?;
                loop {
                    let mut current = listed(&page);
                    current.extend(backward);
                    backward = current;
                    let Some(previous) = page.previous.clone() else { break };
                    page = index.read(reading, &TimelineAnchor::Before(previous), limit).await?;
                }
                assert_eq!(backward, all, "backward by {limit}");
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn month_and_around_start_where_the_reading_has_rows_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = scope_index(&dir).await?;
        let all = reading(vec![start("a", "/"), start("b", "/")], true, true, vec![]);

        let page = index.read(&all, &TimelineAnchor::Month("2024-02".to_string()), 5).await?;
        assert_eq!(listed(&page), strings(&["b:/y.zip/z.jpg"]));
        assert!(page.previous.is_some() && page.next.is_none());
        let page = index.read(&all, &TimelineAnchor::Month("2024-04".to_string()), 2).await?;
        assert_eq!(
            listed(&page),
            strings(&["a:/top1.jpg", "b:/top1.jpg"]),
            "an empty month starts at the next older month that has Entries"
        );
        let empty = index.read(&all, &TimelineAnchor::Month("2024-01".to_string()), 2).await?;
        assert!(empty.rows.is_empty() && empty.previous.is_none() && empty.next.is_none());
        assert!(index.read(&all, &TimelineAnchor::Month("2024-13".to_string()), 2).await.is_err());

        let around = |library: &str, path: &str| TimelineAnchor::Around {
            library: library.to_string(),
            path: path.to_string(),
        };
        let page = index.read(&all, &around("a", "/f/q.mp4"), 5).await?;
        assert_eq!(
            listed(&page),
            strings(&["a:/f/p1.jpg", "b:/f/p1.jpg", "a:/f/q.mp4", "a:/f/sub/p2.jpg", "a:/f/pack.zip/i1.jpg"])
        );
        assert!(page.previous.is_some() && page.next.is_some());

        // The named Entry must be one the starts read, with their conditions applied.
        let folder = reading(vec![start("a", "/f")], true, false, vec![TimelineKind::Image]);
        assert_eq!(
            listed(&index.read(&folder, &around("a", "/f/sub/p2.jpg"), 3).await?),
            strings(&["a:/f/p1.jpg", "a:/f/sub/p2.jpg"])
        );
        for (library, path) in [
            ("a", "/f/q.mp4"),           // another kind
            ("a", "/f/pack.zip/i1.jpg"), // inside an archive the Scope does not read
            ("a", "/top1.jpg"),          // outside the start
            ("b", "/f/p1.jpg"),          // in another Library
            ("a", "/missing.jpg"),       // not a row
        ] {
            let error = index.read(&folder, &around(library, path), 3).await.unwrap_err();
            assert_eq!(*error.kind(), ErrorKind::NotFound, "{library}{path}");
        }
        let archive = reading(vec![archive_start("a", "/f/pack.zip")], true, false, vec![]);
        assert_eq!(index.read(&archive, &around("a", "/f/pack.zip/nested.7z/n1.jpg"), 1).await?.rows.len(), 1);
        Ok(())
    }

    /// The number of rows of each month, counted from the rows that `reading` returns.
    async fn months_of_rows(index: &TimelineIndex, reading: &TimelineReading) -> Result<Vec<(String, u64)>> {
        let page = index.read(reading, &TimelineAnchor::Newest, 1000).await?;
        let mut months = Vec::<(String, u64)>::new();
        for listed in &page.rows {
            let month = listed.row.date.text()[..7].to_string();
            match months.last_mut() {
                Some((last, count)) if *last == month => *count += 1,
                _ => months.push((month, 1)),
            }
        }
        Ok(months)
    }

    #[tokio::test]
    async fn the_months_over_starts_equal_the_rows_every_way_of_reading_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = scope_index(&dir).await?;
        let kinds = [vec![], vec![TimelineKind::Image], vec![TimelineKind::Video], vec![TimelineKind::Image, TimelineKind::Video]];
        let start_sets = [
            vec![start("a", "/"), start("b", "/")],
            vec![start("b", "/")],
            vec![start("a", "/f")],
            vec![start("a", "/f"), start("a", "/g"), start("b", "/f")],
            vec![start("b", "/"), start("a", "/f")],
            vec![archive_start("a", "/f/pack.zip")],
            vec![archive_start("a", "/f/pack.zip/dir"), start("b", "/")],
        ];
        let mut distinct = std::collections::HashSet::new();
        for starts in &start_sets {
            for recursive in [true, false] {
                for inside_archives in [false, true] {
                    for kinds in &kinds {
                        let reading = reading(starts.clone(), recursive, inside_archives, kinds.clone());
                        let months = index.count_months(&reading).await?;
                        assert_eq!(months, months_of_rows(&index, &reading).await?, "{starts:?} {recursive} {inside_archives} {kinds:?}");
                        distinct.insert(months);
                    }
                }
            }
        }
        assert!(distinct.len() > 6, "the readings differ in what they count");
        // A folder start and an archive start under it are disjoint only when the insides of archives are not read, which is when neither covers the other.
        for recursive in [true, false] {
            let reading = reading(vec![start("a", "/f"), archive_start("a", "/f/pack.zip")], recursive, false, vec![]);
            assert_eq!(index.count_months(&reading).await?, months_of_rows(&index, &reading).await?);
        }
        assert_eq!(
            index.count_months(&reading(vec![start("a", "/"), start("b", "/")], true, true, vec![])).await?,
            [("2024-03".to_string(), 12), ("2024-02".to_string(), 1)]
        );
        assert_eq!(
            index.count_months(&reading(vec![start("a", "/f")], false, false, vec![])).await?,
            [("2024-03".to_string(), 2)],
            "the direct children of a folder"
        );
        assert!(index.count_months(&reading(vec![], true, true, vec![])).await.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn the_months_of_whole_libraries_are_summed_from_the_held_counts_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = scope_index(&dir).await?;
        // Make the held count of one month differ from its rows to see which reading takes it.
        sqlx::query("UPDATE timeline_months SET count = count + 100 WHERE library_id = 'b' AND month = '2024-02'")
            .execute(&index.db)
            .await?;

        let roots = reading(vec![start("b", "/")], true, true, vec![]);
        assert_eq!(
            index.count_months(&roots).await?,
            [("2024-03".to_string(), 2), ("2024-02".to_string(), 101)],
            "summed, not counted"
        );
        let images = reading(vec![start("b", "/")], true, true, vec![TimelineKind::Image]);
        assert_eq!(index.count_months(&images).await?, [("2024-03".to_string(), 2), ("2024-02".to_string(), 101)]);
        let videos = reading(vec![start("b", "/")], true, true, vec![TimelineKind::Video]);
        assert!(index.count_months(&videos).await?.is_empty(), "b has no video");
        let outside = reading(vec![start("b", "/")], true, false, vec![]);
        assert_eq!(
            index.count_months(&outside).await?,
            [("2024-03".to_string(), 2)],
            "the insides of archives are left out by their count"
        );
        // A folder start of the same Library is counted from the rows.
        let folder = reading(vec![start("b", "/f")], true, true, vec![]);
        assert_eq!(index.count_months(&folder).await?, [("2024-03".to_string(), 1)]);
        // Direct children of a root are counted from the rows as well.
        let direct = reading(vec![start("b", "/")], false, true, vec![]);
        assert_eq!(index.count_months(&direct).await?, [("2024-03".to_string(), 1)]);
        Ok(())
    }

    #[test]
    fn cursor_round_trips_test() -> TestResult {
        let cursor = TimelineCursor {
            date: "2024-01-01T00:00:00".to_string(),
            library: "my.photos".to_string(),
            path: "/a b/c.jpg".to_string(),
        };
        assert_eq!(cursor.encode(), "2024-01-01T00:00:00my.photos/a b/c.jpg");
        assert_eq!(TimelineCursor::parse(&cursor.encode())?, cursor);
        assert!(TimelineCursor::parse("2024-01-01").is_err());
        assert!(TimelineCursor::parse("2024-01-01T00:00:00relative").is_err());
        assert!(TimelineCursor::parse("2024-01-01T00:00:00/a.jpg").is_err(), "Library のない cursor");
        Ok(())
    }
}
