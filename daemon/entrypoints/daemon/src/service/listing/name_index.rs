use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use futures::TryStreamExt as _;
use sqlx::{
    Row as _, Sqlite, SqlitePool, Transaction,
    query::Query,
    sqlite::{SqliteArguments, SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous},
};
use tokio::sync::{mpsc, oneshot, watch};

use super::NameFilter;
use crate::prelude::*;

const DATABASE_FILE_NAME: &str = "names.db";
/// Left next to the database when it failed while running, so that the next startup discards it.
const BROKEN_MARKER_NAME: &str = "names.db.broken";
/// Bump this when the schema changes. A file with another version is derived state and is discarded.
const FORMAT_VERSION: i64 = 1;
/// The number of stale Entries a sweep deletes in one transaction.
const SWEEP_BATCH: i64 = 2000;
/// How many queued changes wait for the writer. Aggregation waits when the queue is full, so memory stays bounded.
const QUEUE: usize = 64;
/// How many queued changes the writer takes in one pass. The folders among them share one transaction.
const BATCH_OPS: usize = 256;
/// How many new Entries one multi-row INSERT carries.
const INSERT_CHUNK: usize = 100;
/// How many matches a search hands over at once.
const SEARCH_BATCH: usize = 64;
/// How many ids one UPDATE carries.
const ID_CHUNK: usize = 500;

type SqliteQuery<'q> = Query<'q, Sqlite, SqliteArguments>;

/// The tables of the name index.
///
/// `name_entries` holds every Entry by its parent folder's id and its own name, and `name_folders` holds the path of each folder once,
/// so a path is not repeated for every Entry under it. A folder's id in `name_folders` is the id of its own row in `name_entries`.
/// A Library's root has a row of its own with parent 0 and the Library's id as its name.
/// `norm` is the normalized name, or NULL when it equals `name`. The trigram table is contentless: it holds only the trigrams of the normalized names.
const SCHEMA: &[&str] = &[
    r#"
CREATE TABLE IF NOT EXISTS name_entries (
    id INTEGER PRIMARY KEY,
    parent_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    norm TEXT,
    kind INTEGER NOT NULL,
    generation INTEGER NOT NULL
)
"#,
    "CREATE UNIQUE INDEX IF NOT EXISTS name_entries_key ON name_entries (parent_id, name)",
    r#"
CREATE TABLE IF NOT EXISTS name_folders (
    id INTEGER PRIMARY KEY,
    library_id TEXT NOT NULL,
    path TEXT NOT NULL
)
"#,
    "CREATE UNIQUE INDEX IF NOT EXISTS name_folders_path ON name_folders (library_id, path)",
    r#"
CREATE TABLE IF NOT EXISTS name_libraries (
    library_id TEXT NOT NULL PRIMARY KEY,
    usable_generation INTEGER NOT NULL
) WITHOUT ROWID
"#,
    r#"
CREATE TABLE IF NOT EXISTS name_scans (
    library_id TEXT NOT NULL PRIMARY KEY,
    generation INTEGER NOT NULL
) WITHOUT ROWID
"#,
    "CREATE VIRTUAL TABLE IF NOT EXISTS name_fts USING fts5(norm, content='', contentless_delete=1, tokenize='trigram case_sensitive 1')",
];

/// What an Entry outside archives is, as far as the name index tells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameKind {
    Folder,
    File,
    Archive,
    Image,
    Video,
}

impl NameKind {
    fn code(self) -> i64 {
        match self {
            Self::Folder => 0,
            Self::File => 1,
            Self::Archive => 2,
            Self::Image => 3,
            Self::Video => 4,
        }
    }

    fn from_code(code: i64) -> Self {
        match code {
            0 => Self::Folder,
            2 => Self::Archive,
            3 => Self::Image,
            4 => Self::Video,
            _ => Self::File,
        }
    }
}

/// One direct child of a folder, as aggregation enumerated it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameRow {
    pub name: String,
    pub kind: NameKind,
}

impl NameRow {
    pub fn new(name: &str, kind: NameKind) -> Self {
        Self { name: name.to_string(), kind }
    }

    /// The normalized name, or `None` when it equals the name.
    fn normalized(&self) -> (String, Option<String>) {
        let normalized = NameFilter::normalize(&self.name);
        let stored = (normalized != self.name).then(|| normalized.clone());
        (normalized, stored)
    }
}

/// A folder and everything under it, as a range of `name_folders.path`. The Library's root has no range: it is everything.
struct FolderRange {
    exact: String,
    lower: String,
    upper: String,
}

impl FolderRange {
    fn of(path: &str) -> Option<Self> {
        let path = path.trim_end_matches('/');
        if path.is_empty() {
            return None;
        }
        // The character after `/` is `0`, so every path that starts with `path/` is at least `path/` and below `path0`.
        Some(Self {
            exact: path.to_string(),
            lower: format!("{path}/"),
            upper: format!("{path}0"),
        })
    }

    /// The condition on `name_folders` for the Library and, unless it is the root, the range. Bind `library`, then the range with `bind`.
    fn condition(range: &Option<Self>) -> &'static str {
        match range {
            Some(_) => "library_id = ? AND (path = ? OR (path >= ? AND path < ?))",
            None => "library_id = ?",
        }
    }

    fn bind<'q>(range: &'q Option<Self>, library: &'q str, query: SqliteQuery<'q>) -> SqliteQuery<'q> {
        let query = query.bind(library);
        match range {
            Some(range) => query.bind(range.exact.as_str()).bind(range.lower.as_str()).bind(range.upper.as_str()),
            None => query,
        }
    }
}

/// What to look for under a folder of a Library.
pub struct NameQuery {
    pub library: String,
    /// The folder to search under, as a Library-relative path. The folder itself is not a match.
    pub start: String,
    /// Normalized terms. An Entry matches when its normalized name contains all of them.
    pub terms: Vec<String>,
    /// Only images and videos, which are the only Entries that can have ratings and tags.
    pub only_media: bool,
}

/// The matches of a search, handed over in batches while the query runs.
///
/// The query runs in a task of its own and stops as soon as the cursor is dropped, such as when every request waiting for the listing has gone.
pub struct NameCursor {
    receiver: mpsc::Receiver<Result<Vec<NameHit>>>,
    task: tokio::task::JoinHandle<()>,
    health: Arc<Health>,
}

impl NameCursor {
    /// The next batch of matches, or `None` once there are no more. An error means the index cannot answer, and then what came before is not to be used.
    pub async fn next(&mut self) -> Result<Option<Vec<NameHit>>> {
        let received = self.receiver.recv().await;
        match received {
            Some(Ok(batch)) if !self.health.is_disabled() => Ok(Some(batch)),
            Some(Ok(_)) => Err(Error::new(ErrorKind::Cancelled).with_message("the name index stopped while it was searched")),
            Some(Err(error)) => {
                self.health.settle::<()>(Err(Error::new(ErrorKind::IoError).with_message(format!("{error:?}"))));
                Err(Error::new(ErrorKind::IoError).with_message("the name index failed while it was searched"))
            }
            None if self.health.is_disabled() => Err(Error::new(ErrorKind::Cancelled).with_message("the name index stopped while it was searched")),
            None => Ok(None),
        }
    }

    #[cfg(test)]
    pub fn abort_handle(&self) -> tokio::task::AbortHandle {
        self.task.abort_handle()
    }
}

impl Drop for NameCursor {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// An Entry that a query found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameHit {
    /// The Library-relative path.
    pub path: String,
    pub kind: NameKind,
}

/// A Library-relative path split into the folder it is in and its own name.
struct EntryPath;

impl EntryPath {
    /// `/a/b` becomes `/a/b`, `a/b/` becomes `/a/b`, and the root is `/`.
    fn clean(path: &str) -> String {
        let trimmed = path.trim_matches('/');
        format!("/{trimmed}")
    }

    /// The folder and the name of an Entry. The root has no parent.
    fn split(path: &str) -> Option<(String, String)> {
        let clean = Self::clean(path);
        let (parent, name) = clean.rsplit_once('/')?;
        if name.is_empty() {
            return None;
        }
        Some((if parent.is_empty() { "/".to_string() } else { parent.to_string() }, name.to_string()))
    }

    /// Whether a segment of the path starts with `.`, which aggregation does not list or descend into.
    fn is_hidden(path: &str) -> bool {
        path.split('/').any(|segment| segment.starts_with('.'))
    }

    fn join(parent: &str, name: &str) -> String {
        if parent == "/" { format!("/{name}") } else { format!("{parent}/{name}") }
    }
}

/// Whether the index can be trusted. The writer task and the integrity check share it with the index.
struct Health {
    disabled: AtomicBool,
    /// The integrity check of the file has passed. Until it has, the index answers nothing.
    verified: AtomicBool,
    /// Whether the integrity check has finished, passed or not.
    checked: watch::Sender<bool>,
    directory: PathBuf,
}

impl Health {
    fn new(disabled: bool, directory: PathBuf) -> Self {
        Self {
            disabled: AtomicBool::new(disabled),
            verified: AtomicBool::new(false),
            // An index that was never opened has nothing to check.
            checked: watch::Sender::new(disabled),
            directory,
        }
    }

    fn is_disabled(&self) -> bool {
        self.disabled.load(Ordering::Acquire)
    }

    fn is_serving(&self) -> bool {
        !self.is_disabled() && self.verified.load(Ordering::Acquire)
    }

    /// Stops using the index for the process on an error, and marks the file so that the next startup discards it.
    fn settle<T>(&self, result: Result<T>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                if !self.disabled.swap(true, Ordering::AcqRel) {
                    warn!(?error, "name index failed; searching by walking until the next startup");
                    if !self.directory.as_os_str().is_empty() {
                        let _ = std::fs::write(self.directory.join(BROKEN_MARKER_NAME), b"");
                    }
                }
                None
            }
        }
    }
}

/// A change to the index. The writer applies them in the order they were queued.
enum Work {
    Children {
        library: String,
        parent: String,
        rows: Vec<NameRow>,
        generation: i64,
    },
    Entry {
        library: String,
        path: String,
        kind: NameKind,
        generation: i64,
    },
    Keep {
        library: String,
        folder: String,
        generation: i64,
    },
    Sweep {
        library: String,
        scope: String,
        generation: i64,
    },
    Remove {
        library: String,
        path: String,
    },
    Rename {
        library: String,
        src: String,
        dst: String,
    },
    Usable {
        library: String,
        generation: i64,
    },
    Begin {
        library: String,
        generation: i64,
    },
    /// Changes nothing; it is done when everything queued before it is. Tests use it to read what aggregation wrote.
    #[cfg(test)]
    Barrier,
}

struct Write {
    work: Work,
    /// Told when the change has been applied, for a caller that waits for it.
    done: Option<oneshot::Sender<()>>,
}

/// The index of the names of every Entry outside archives, one SQLite file under the listing state directory.
///
/// It is derived state. Any error while running makes the process stop using it, and the next startup discards the file.
///
/// Changes go through one writer task in the order they are made. Aggregation does not wait for its folders to be written, which lets
/// the writer batch them and lets writing overlap with walking, while the daemon's own changes and the end of a scan wait for theirs.
pub struct NameIndex {
    db: Option<SqlitePool>,
    writer: Option<mpsc::Sender<Write>>,
    health: Arc<Health>,
    /// The check of the file that runs after startup. Tests wait for it.
    #[cfg(test)]
    verification: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl NameIndex {
    /// An index that holds nothing and is never usable, for when the file cannot be opened.
    pub fn disabled() -> Self {
        Self {
            db: None,
            writer: None,
            health: Arc::new(Health::new(true, PathBuf::new())),
            #[cfg(test)]
            verification: Mutex::new(None),
        }
    }

    /// Opens the file under `dir`. A file that is corrupt, of another format, or marked broken is discarded first.
    /// If it still cannot be opened, the returned index is disabled so that everything else can start.
    pub async fn open(dir: &Path) -> Self {
        Self::open_with(dir, true).await
    }

    async fn open_with(dir: &Path, check: bool) -> Self {
        if let Err(error) = tokio::fs::create_dir_all(dir).await {
            warn!(?error, "name index unavailable; searching by walking");
            return Self::disabled();
        }
        if dir.join(BROKEN_MARKER_NAME).exists() {
            warn!("name index was marked broken; discarding it");
            Self::discard(dir).await;
        }
        let pool = match Self::connect(dir).await {
            Ok(pool) => pool,
            Err(error) => {
                warn!(?error, "name index is corrupt or of another format; discarding it");
                Self::discard(dir).await;
                match Self::connect(dir).await {
                    Ok(pool) => pool,
                    Err(error) => {
                        warn!(?error, "name index unavailable; searching by walking");
                        return Self::disabled();
                    }
                }
            }
        };
        let health = Arc::new(Health::new(false, dir.to_path_buf()));
        let (writer, receiver) = mpsc::channel(QUEUE);
        tokio::spawn(Self::write_loop(pool.clone(), health.clone(), receiver));
        // Reading every page of a large file takes a while, so the file is checked in the background and the index answers nothing until it
        // has passed. A file that fails stops the index for the process, and the next startup discards it.
        let verification = check.then(|| tokio::spawn(Self::verify(pool.clone(), health.clone())));
        #[cfg(not(test))]
        drop(verification);
        Self {
            db: Some(pool),
            writer: Some(writer),
            health,
            #[cfg(test)]
            verification: Mutex::new(verification),
        }
    }

    /// Runs SQLite's quick integrity check over the whole file.
    async fn verify(db: SqlitePool, health: Arc<Health>) {
        let checked = async {
            let verdict: String = sqlx::query_scalar("PRAGMA quick_check(1)").fetch_one(&db).await?;
            if verdict == "ok" {
                Ok(())
            } else {
                Err(Error::new(ErrorKind::InvalidFormat).with_message(format!("the name index failed its integrity check: {verdict}")))
            }
        }
        .await;
        if health.settle(checked).is_some() {
            health.verified.store(true, Ordering::Release);
        }
        health.checked.send_replace(true);
    }

    /// Runs the integrity check now, for a test that opened the index without it.
    #[cfg(test)]
    pub async fn verify_now(&self) {
        if let Some(db) = &self.db {
            Self::verify(db.clone(), self.health.clone()).await;
        }
    }

    /// Waits for the integrity check that startup began.
    #[cfg(test)]
    pub async fn verified(&self) {
        let handle = self.verification.lock().take();
        if let Some(handle) = handle {
            let _ = handle.await;
        }
    }

    async fn discard(dir: &Path) {
        for name in [DATABASE_FILE_NAME, "names.db-wal", "names.db-shm", BROKEN_MARKER_NAME] {
            let _ = tokio::fs::remove_file(dir.join(name)).await;
        }
    }

    async fn connect(dir: &Path) -> Result<SqlitePool> {
        let options = SqliteConnectOptions::new()
            .filename(dir.join(DATABASE_FILE_NAME))
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            // The index is rebuilt by aggregation, so a lost tail after a crash costs nothing.
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(std::time::Duration::from_secs(30));
        let db = SqlitePool::connect_with(options).await?;
        match Self::prepare(&db).await {
            Ok(()) => Ok(db),
            Err(error) => {
                db.close().await;
                Err(error)
            }
        }
    }

    /// Creates the tables of a new file, and rejects a file of another version or one whose schema cannot be read.
    async fn prepare(db: &SqlitePool) -> Result<()> {
        let version: i64 = sqlx::query_scalar("PRAGMA user_version").fetch_one(db).await?;
        let tables: i64 = sqlx::query_scalar("SELECT count(*) FROM sqlite_master").fetch_one(db).await?;
        if tables > 0 && version != FORMAT_VERSION {
            return Err(Error::new(ErrorKind::InvalidFormat).with_message("unknown name index format"));
        }
        for statement in SCHEMA {
            sqlx::query(*statement).execute(db).await?;
        }
        sqlx::query(sqlx::AssertSqlSafe(format!("PRAGMA user_version = {FORMAT_VERSION}"))).execute(db).await?;
        // Reads a row of each table, so a file that opens but is damaged fails here and not in the middle of aggregation.
        sqlx::query("SELECT 1 FROM name_entries LIMIT 1").fetch_optional(db).await?;
        sqlx::query("SELECT 1 FROM name_folders LIMIT 1").fetch_optional(db).await?;
        sqlx::query("SELECT 1 FROM name_libraries LIMIT 1").fetch_optional(db).await?;
        Ok(())
    }

    /// The database, unless the index is disabled.
    fn live(&self) -> Option<&SqlitePool> {
        if self.health.is_serving() { self.db.as_ref() } else { None }
    }

    /// Whether the index can answer: it has passed its integrity check and has not failed since.
    pub fn is_serving(&self) -> bool {
        self.health.is_serving()
    }

    /// Like `is_usable`, but waits for the integrity check that startup began, so that a Library is not taken for unaggregated only because the file
    /// has not been checked yet.
    pub async fn is_usable_checked(&self, library: &str) -> bool {
        let mut checked = self.health.checked.subscribe();
        let _ = checked.wait_for(|done| *done).await;
        self.is_usable(library).await
    }

    /// Stops the index as a failure of the file would. For tests of what happens when it fails while it is read.
    #[cfg(test)]
    pub fn fail_for_test(&self) {
        self.health.settle::<()>(Err(Error::new(ErrorKind::IoError).with_message("injected failure")));
    }

    /// Like `settle` for a read: the answer is not trusted either when the index failed while the read was running, such as in the writer.
    fn settle_read<T>(&self, result: Result<T>) -> Option<T> {
        self.health.settle(result).filter(|_| !self.health.is_disabled())
    }

    /// Queues a change, and waits until it has been applied when `wait` is set.
    async fn queue(&self, work: Work, wait: bool) {
        let Some(writer) = &self.writer else {
            return;
        };
        if self.health.is_disabled() {
            return;
        }
        if wait {
            let (done, finished) = oneshot::channel();
            if writer.send(Write { work, done: Some(done) }).await.is_ok() {
                let _ = finished.await;
            }
        } else {
            let _ = writer.send(Write { work, done: None }).await;
        }
    }

    /// Waits until everything queued so far has been applied. For tests, which read what aggregation wrote without waiting.
    #[cfg(test)]
    pub async fn flush(&self) {
        self.queue(Work::Barrier, true).await;
    }

    /// Every Entry of the Library as `path:kind`, sorted, without the root. For tests of the code that feeds the index.
    #[cfg(test)]
    pub async fn held(&self, library: &str) -> Vec<String> {
        self.flush().await;
        let db = self.db.as_ref().expect("the index is open");
        let rows = sqlx::query("SELECT f.path AS folder, e.name AS name, e.kind AS kind FROM name_entries e JOIN name_folders f ON e.parent_id = f.id WHERE f.library_id = ?")
            .bind(library)
            .fetch_all(db)
            .await
            .expect("the index can be read");
        let mut held = rows
            .iter()
            .map(|row| {
                format!(
                    "{}:{}",
                    EntryPath::join(&row.get::<String, _>("folder"), &row.get::<String, _>("name")),
                    row.get::<i64, _>("kind")
                )
            })
            .collect::<Vec<_>>();
        held.sort();
        held
    }

    // ---- State ----

    /// Whether a full aggregation of the Library has finished with this index once, and the index has not failed since.
    pub async fn is_usable(&self, library: &str) -> bool {
        let Some(db) = self.live() else {
            return false;
        };
        let found = sqlx::query_scalar::<_, i64>("SELECT 1 FROM name_libraries WHERE library_id = ?")
            .bind(library)
            .fetch_optional(db)
            .await
            .map_err(Error::from);
        self.settle_read(found).flatten().is_some()
    }

    /// Records that a full aggregation of `generation` has finished, so the Library's index can answer.
    pub async fn mark_usable(&self, library: &str, generation: i64) {
        let work = Work::Usable {
            library: library.to_string(),
            generation,
        };
        self.queue(work, true).await;
    }

    // ---- Reading ----

    /// Starts a search of the Entries under `query.start`. `None` means the index cannot answer.
    ///
    /// Terms of 3 or more characters go to the trigram table as quoted phrases, and shorter ones are compared with `instr` on the normalized name.
    pub fn search(&self, query: NameQuery) -> Option<NameCursor> {
        let db = self.live()?.clone();
        let (sender, receiver) = mpsc::channel(2);
        let task = tokio::spawn(async move {
            if let Err(error) = Self::run_search(&db, query, &sender).await {
                let _ = sender.send(Err(error)).await;
            }
        });
        Some(NameCursor {
            receiver,
            task,
            health: self.health.clone(),
        })
    }

    /// Of `paths`, those the index holds as images or videos (or any kind, unless `only_media`) whose names contain all the terms.
    pub async fn lookup(&self, library: &str, paths: &[String], terms: &[String], only_media: bool) -> Option<Vec<NameHit>> {
        let db = self.live()?;
        let result = Self::run_lookup(db, library, paths, terms, only_media).await;
        self.settle_read(result)
    }

    const MEDIA_KINDS: &'static str = "e.kind IN (3, 4)";

    async fn run_search(db: &SqlitePool, query: NameQuery, sender: &mpsc::Sender<Result<Vec<NameHit>>>) -> Result<()> {
        let start = EntryPath::clean(&query.start);
        let range = FolderRange::of(&start);
        let (long, short): (Vec<&String>, Vec<&String>) = query.terms.iter().partition(|term| term.chars().count() >= 3);
        let mut sql = if long.is_empty() {
            "SELECT f.path AS folder, e.name AS name, e.kind AS kind FROM name_entries e JOIN name_folders f ON f.id = e.parent_id WHERE ".to_string()
        } else {
            "SELECT f.path AS folder, e.name AS name, e.kind AS kind FROM name_fts t JOIN name_entries e ON e.id = t.rowid JOIN name_folders f ON f.id = e.parent_id WHERE name_fts MATCH ? AND ".to_string()
        };
        sql.push_str(&FolderRange::condition(&range).replace("library_id", "f.library_id").replace("path", "f.path"));
        for _ in &short {
            sql.push_str(" AND instr(COALESCE(e.norm, e.name), ?) > 0");
        }
        if query.only_media {
            sql.push_str(" AND ");
            sql.push_str(Self::MEDIA_KINDS);
        }
        let mut statement = sqlx::query(sqlx::AssertSqlSafe(sql));
        if !long.is_empty() {
            statement = statement.bind(long.iter().map(|term| format!("\"{}\"", term.replace('"', "\"\""))).collect::<Vec<_>>().join(" AND "));
        }
        let mut statement = FolderRange::bind(&range, &query.library, statement);
        for term in &short {
            statement = statement.bind(term.as_str());
        }
        let mut rows = statement.fetch(db);
        let mut batch = Vec::with_capacity(SEARCH_BATCH);
        while let Some(row) = rows.try_next().await? {
            batch.push(NameHit {
                path: EntryPath::join(&row.try_get::<String, _>("folder")?, &row.try_get::<String, _>("name")?),
                kind: NameKind::from_code(row.try_get("kind")?),
            });
            if batch.len() >= SEARCH_BATCH && sender.send(Ok(std::mem::take(&mut batch))).await.is_err() {
                return Ok(());
            }
        }
        if !batch.is_empty() {
            let _ = sender.send(Ok(batch)).await;
        }
        Ok(())
    }

    async fn run_lookup(db: &SqlitePool, library: &str, paths: &[String], terms: &[String], only_media: bool) -> Result<Vec<NameHit>> {
        let mut sql = "SELECT e.kind AS kind FROM name_entries e JOIN name_folders f ON f.id = e.parent_id WHERE f.library_id = ? AND f.path = ? AND e.name = ?".to_string();
        for _ in terms {
            sql.push_str(" AND instr(COALESCE(e.norm, e.name), ?) > 0");
        }
        if only_media {
            sql.push_str(" AND ");
            sql.push_str(Self::MEDIA_KINDS);
        }
        let mut hits = Vec::new();
        for path in paths {
            let Some((parent, name)) = EntryPath::split(path) else {
                continue;
            };
            let mut statement = sqlx::query(sqlx::AssertSqlSafe(sql.clone())).bind(library).bind(parent).bind(name);
            for term in terms {
                statement = statement.bind(term.as_str());
            }
            if let Some(row) = statement.fetch_optional(db).await? {
                hits.push(NameHit {
                    path: EntryPath::clean(path),
                    kind: NameKind::from_code(row.try_get("kind")?),
                });
            }
        }
        Ok(hits)
    }

    // ---- Writing ----

    /// Records the direct children that aggregation enumerated in the folder `parent`, stamping them with `generation`.
    ///
    /// Children missing from `children` are not deleted here: a later `sweep` deletes what no aggregation saw,
    /// so an Entry that appeared after this folder was enumerated is not lost.
    /// This returns when the change is queued; `sweep` and `flush` wait for it to be applied.
    pub async fn replace_children(&self, library: &str, parent: &str, children: &[NameRow], generation: i64) {
        let work = Work::Children {
            library: library.to_string(),
            parent: EntryPath::clean(parent),
            rows: children.to_vec(),
            generation,
        };
        self.queue(work, false).await;
    }

    /// Records that a full aggregation of `generation` begins. A generation that is not above the last one means the aggregation records
    /// restarted, such as when only the timeline index was recreated, so the rows are no longer comparable with it. They are deleted and the
    /// Library is not usable until this aggregation finishes.
    pub async fn begin_scan(&self, library: &str, generation: i64) {
        let work = Work::Begin {
            library: library.to_string(),
            generation,
        };
        self.queue(work, true).await;
    }

    /// Records one Entry, such as the start of a range aggregation, with the folders above it.
    pub async fn upsert(&self, library: &str, path: &str, kind: NameKind, generation: i64) {
        let work = Work::Entry {
            library: library.to_string(),
            path: EntryPath::clean(path),
            kind,
            generation,
        };
        self.queue(work, true).await;
    }

    /// Stamps the Entries under the folder with `generation` without enumerating it, for a folder aggregation could not read,
    /// so that the sweep does not delete what is probably still there.
    pub async fn keep(&self, library: &str, folder: &str, generation: i64) {
        let work = Work::Keep {
            library: library.to_string(),
            folder: EntryPath::clean(folder),
            generation,
        };
        self.queue(work, true).await;
    }

    /// Deletes, in the scanned range, the Entries and folders that no aggregation of `generation` or later has seen.
    pub async fn sweep(&self, library: &str, scope: &str, generation: i64) {
        let work = Work::Sweep {
            library: library.to_string(),
            scope: EntryPath::clean(scope),
            generation,
        };
        self.queue(work, true).await;
    }

    /// Deletes the Entry at `path` and everything under it. It returns after the deletion.
    pub async fn remove(&self, library: &str, path: &str) {
        let work = Work::Remove {
            library: library.to_string(),
            path: EntryPath::clean(path),
        };
        self.queue(work, true).await;
    }

    /// Moves the Entry at `src`, with everything under it, to `dst`. What was at `dst` is deleted first. It returns after the move.
    /// An Entry the index does not hold is left to the range aggregation of the destination, and a Library's root is not moved.
    pub async fn rename(&self, library: &str, src: &str, dst: &str) {
        let work = Work::Rename {
            library: library.to_string(),
            src: EntryPath::clean(src),
            dst: EntryPath::clean(dst),
        };
        self.queue(work, true).await;
    }

    async fn begin(db: &SqlitePool) -> Result<Transaction<'static, Sqlite>> {
        // Several Libraries write at once, so take the write lock from the start and wait for the others instead of failing.
        Ok(db.begin_with("BEGIN IMMEDIATE").await?)
    }

    /// Applies the queued changes. The folders that follow each other share one transaction, and any other change commits it first.
    async fn write_loop(db: SqlitePool, health: Arc<Health>, mut receiver: mpsc::Receiver<Write>) {
        let mut open: Option<Transaction<'static, Sqlite>> = None;
        while let Some(first) = receiver.recv().await {
            let mut batch = vec![first];
            while batch.len() < BATCH_OPS {
                match receiver.try_recv() {
                    Ok(next) => batch.push(next),
                    Err(_) => break,
                }
            }
            for Write { work, done } in batch {
                if !health.is_disabled() {
                    let result = Self::apply(&db, &mut open, work).await;
                    health.settle(result);
                }
                if let Some(done) = done {
                    let _ = done.send(());
                }
            }
            // Commit what the batch left open, so that it is visible as soon as the queue is idle.
            if let Some(transaction) = open.take() {
                health.settle(transaction.commit().await.map_err(Error::from));
            }
        }
    }

    async fn apply(db: &SqlitePool, open: &mut Option<Transaction<'static, Sqlite>>, work: Work) -> Result<()> {
        if let Work::Children {
            library,
            parent,
            rows,
            generation,
        } = work
        {
            if open.is_none() {
                *open = Some(Self::begin(db).await?);
            }
            let transaction = open.as_mut().expect("a transaction was opened above");
            let result = Self::write_children(transaction, &library, &parent, &rows, generation).await;
            if result.is_err() {
                // Dropping the transaction rolls it back.
                open.take();
            }
            return result;
        }
        if let Some(transaction) = open.take() {
            transaction.commit().await?;
        }
        match work {
            Work::Entry { library, path, kind, generation } => Self::write_entry(db, &library, &path, kind, generation).await,
            Work::Keep { library, folder, generation } => Self::stamp(db, &library, &folder, generation).await,
            Work::Sweep { library, scope, generation } => Self::delete_stale(db, &library, &scope, generation).await,
            Work::Remove { library, path } => Self::delete_path(db, &library, &path).await,
            Work::Rename { library, src, dst } => Self::move_path(db, &library, &src, &dst).await,
            Work::Usable { library, generation } => Self::write_usable(db, &library, generation).await,
            Work::Begin { library, generation } => Self::write_begin(db, &library, generation).await,
            Work::Children { .. } => Ok(()),
            #[cfg(test)]
            Work::Barrier => Ok(()),
        }
    }

    async fn write_usable(db: &SqlitePool, library: &str, generation: i64) -> Result<()> {
        sqlx::query(
            "INSERT INTO name_libraries (library_id, usable_generation) VALUES (?, ?) ON CONFLICT (library_id) DO UPDATE SET usable_generation = MAX(usable_generation, excluded.usable_generation)",
        )
        .bind(library)
        .bind(generation)
        .execute(db)
        .await?;
        Ok(())
    }

    async fn write_begin(db: &SqlitePool, library: &str, generation: i64) -> Result<()> {
        let mut tx = Self::begin(db).await?;
        let previous: Option<i64> = sqlx::query_scalar("SELECT generation FROM name_scans WHERE library_id = ?")
            .bind(library)
            .fetch_optional(&mut *tx)
            .await?;
        if previous.is_some_and(|previous| previous >= generation) {
            Self::delete_inside(&mut tx, library, "/").await?;
            sqlx::query("DELETE FROM name_libraries WHERE library_id = ?").bind(library).execute(&mut *tx).await?;
        }
        sqlx::query("INSERT INTO name_scans (library_id, generation) VALUES (?, ?) ON CONFLICT (library_id) DO UPDATE SET generation = excluded.generation")
            .bind(library)
            .bind(generation)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Writes the children of one folder into the open transaction. New ones go in with multi-row INSERTs.
    async fn write_children(tx: &mut Transaction<'_, Sqlite>, library: &str, parent: &str, children: &[NameRow], generation: i64) -> Result<()> {
        let parent_id = Self::ensure_folder(tx, library, parent, generation).await?;
        let existing: Vec<(i64, String, i64, i64)> = sqlx::query_as("SELECT id, name, kind, generation FROM name_entries WHERE parent_id = ?")
            .bind(parent_id)
            .fetch_all(&mut **tx)
            .await?;
        let existing = existing
            .into_iter()
            .map(|(id, name, kind, stamped)| (name, (id, NameKind::from_code(kind), stamped)))
            .collect::<HashMap<_, _>>();
        let mut seen = HashSet::new();
        let mut fresh = Vec::new();
        let mut restamp = Vec::new();
        for child in children {
            if !seen.insert(child.name.as_str()) {
                continue;
            }
            match existing.get(&child.name) {
                None => fresh.push(child),
                Some((id, kind, stamped)) => {
                    if *kind != child.kind {
                        Self::change_kind(tx, library, *id, *kind, parent, child).await?;
                    }
                    if *stamped < generation {
                        restamp.push(*id);
                    }
                }
            }
        }
        for ids in restamp.chunks(ID_CHUNK) {
            let sql = format!("UPDATE name_entries SET generation = ? WHERE id IN ({})", vec!["?"; ids.len()].join(", "));
            let mut statement = sqlx::query(sqlx::AssertSqlSafe(sql)).bind(generation);
            for id in ids {
                statement = statement.bind(*id);
            }
            statement.execute(&mut **tx).await?;
        }
        for rows in fresh.chunks(INSERT_CHUNK) {
            Self::insert_entries(tx, library, parent_id, parent, rows, generation).await?;
        }
        Ok(())
    }

    /// Adds Entries that are not in the index with three multi-row INSERTs: the Entries, their trigrams, and the folder rows of the folders among them.
    async fn insert_entries(tx: &mut Transaction<'_, Sqlite>, library: &str, parent_id: i64, parent: &str, rows: &[&NameRow], generation: i64) -> Result<()> {
        let normalized = rows.iter().map(|row| row.normalized()).collect::<Vec<_>>();
        let sql = format!(
            "INSERT INTO name_entries (parent_id, name, norm, kind, generation) VALUES {} RETURNING id, name",
            vec!["(?, ?, ?, ?, ?)"; rows.len()].join(", ")
        );
        let mut statement = sqlx::query(sqlx::AssertSqlSafe(sql));
        for (row, (_, stored)) in rows.iter().zip(&normalized) {
            statement = statement
                .bind(parent_id)
                .bind(row.name.as_str())
                .bind(stored.as_deref())
                .bind(row.kind.code())
                .bind(generation);
        }
        let mut ids = HashMap::with_capacity(rows.len());
        for inserted in statement.fetch_all(&mut **tx).await? {
            ids.insert(inserted.try_get::<String, _>("name")?, inserted.try_get::<i64, _>("id")?);
        }
        let sql = format!("INSERT INTO name_fts (rowid, norm) VALUES {}", vec!["(?, ?)"; rows.len()].join(", "));
        let mut statement = sqlx::query(sqlx::AssertSqlSafe(sql));
        for (row, (text, _)) in rows.iter().zip(&normalized) {
            let id = ids
                .get(&row.name)
                .ok_or_else(|| Error::new(ErrorKind::UnexpectedError).with_message("an inserted name index row was not returned"))?;
            statement = statement.bind(*id).bind(text.as_str());
        }
        statement.execute(&mut **tx).await?;
        let folders = rows.iter().filter(|row| row.kind == NameKind::Folder).collect::<Vec<_>>();
        if !folders.is_empty() {
            let sql = format!("INSERT INTO name_folders (id, library_id, path) VALUES {}", vec!["(?, ?, ?)"; folders.len()].join(", "));
            let mut statement = sqlx::query(sqlx::AssertSqlSafe(sql));
            for folder in folders {
                let id = ids
                    .get(&folder.name)
                    .ok_or_else(|| Error::new(ErrorKind::UnexpectedError).with_message("an inserted name index row was not returned"))?;
                statement = statement.bind(*id).bind(library).bind(EntryPath::join(parent, &folder.name));
            }
            statement.execute(&mut **tx).await?;
        }
        Ok(())
    }

    async fn write_entry(db: &SqlitePool, library: &str, path: &str, kind: NameKind, generation: i64) -> Result<()> {
        let path = EntryPath::clean(path);
        let Some((parent, name)) = EntryPath::split(&path) else {
            let mut tx = Self::begin(db).await?;
            Self::ensure_folder(&mut tx, library, "/", generation).await?;
            tx.commit().await?;
            return Ok(());
        };
        let mut tx = Self::begin(db).await?;
        let parent_id = Self::ensure_folder(&mut tx, library, &parent, generation).await?;
        let row = NameRow::new(&name, kind);
        let existing: Option<(i64, i64)> = sqlx::query_as("SELECT id, kind FROM name_entries WHERE parent_id = ? AND name = ?")
            .bind(parent_id)
            .bind(&name)
            .fetch_optional(&mut *tx)
            .await?;
        match existing {
            None => {
                Self::insert_entry(&mut tx, library, parent_id, &parent, &row, generation).await?;
            }
            Some((id, old)) => {
                let old = NameKind::from_code(old);
                if old != kind {
                    Self::change_kind(&mut tx, library, id, old, &parent, &row).await?;
                }
                sqlx::query("UPDATE name_entries SET generation = MAX(generation, ?) WHERE id = ?")
                    .bind(generation)
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }

    /// The id of the folder at `path`, creating it and the folders above it when the index does not know them yet.
    async fn ensure_folder(tx: &mut Transaction<'_, Sqlite>, library: &str, path: &str, generation: i64) -> Result<i64> {
        if let Some(id) = Self::folder_id(tx, library, path).await? {
            return Ok(id);
        }
        let mut parent_id = match Self::folder_id(tx, library, "/").await? {
            Some(id) => id,
            None => {
                // A Library's root is a row of its own, so that folder ids and Entry ids are one sequence.
                let id: i64 = sqlx::query_scalar(
                    "INSERT INTO name_entries (parent_id, name, norm, kind, generation) VALUES (0, ?, NULL, ?, 0) ON CONFLICT (parent_id, name) DO UPDATE SET kind = excluded.kind RETURNING id",
                )
                .bind(library)
                .bind(NameKind::Folder.code())
                .fetch_one(&mut **tx)
                .await?;
                sqlx::query("INSERT INTO name_folders (id, library_id, path) VALUES (?, ?, '/')")
                    .bind(id)
                    .bind(library)
                    .execute(&mut **tx)
                    .await?;
                id
            }
        };
        let mut folder = "/".to_string();
        for name in path.split('/').filter(|segment| !segment.is_empty()) {
            let child = EntryPath::join(&folder, name);
            parent_id = match Self::folder_id(tx, library, &child).await? {
                Some(id) => id,
                None => {
                    let row = NameRow::new(name, NameKind::Folder);
                    let existing: Option<(i64, i64)> = sqlx::query_as("SELECT id, kind FROM name_entries WHERE parent_id = ? AND name = ?")
                        .bind(parent_id)
                        .bind(name)
                        .fetch_optional(&mut **tx)
                        .await?;
                    match existing {
                        None => Self::insert_entry(tx, library, parent_id, &folder, &row, generation).await?,
                        Some((id, old)) => {
                            Self::change_kind(tx, library, id, NameKind::from_code(old), &folder, &row).await?;
                            id
                        }
                    }
                }
            };
            folder = child;
        }
        Ok(parent_id)
    }

    async fn folder_id(tx: &mut Transaction<'_, Sqlite>, library: &str, path: &str) -> Result<Option<i64>> {
        Ok(sqlx::query_scalar("SELECT id FROM name_folders WHERE library_id = ? AND path = ?")
            .bind(library)
            .bind(path)
            .fetch_optional(&mut **tx)
            .await?)
    }

    /// Adds an Entry that is not in the index, with its trigrams, and with its folder row when it is a folder.
    async fn insert_entry(tx: &mut Transaction<'_, Sqlite>, library: &str, parent_id: i64, parent: &str, row: &NameRow, generation: i64) -> Result<i64> {
        let (normalized, stored) = row.normalized();
        let id: i64 = sqlx::query_scalar("INSERT INTO name_entries (parent_id, name, norm, kind, generation) VALUES (?, ?, ?, ?, ?) RETURNING id")
            .bind(parent_id)
            .bind(&row.name)
            .bind(stored)
            .bind(row.kind.code())
            .bind(generation)
            .fetch_one(&mut **tx)
            .await?;
        sqlx::query("INSERT INTO name_fts (rowid, norm) VALUES (?, ?)")
            .bind(id)
            .bind(normalized)
            .execute(&mut **tx)
            .await?;
        if row.kind == NameKind::Folder {
            sqlx::query("INSERT INTO name_folders (id, library_id, path) VALUES (?, ?, ?)")
                .bind(id)
                .bind(library)
                .bind(EntryPath::join(parent, &row.name))
                .execute(&mut **tx)
                .await?;
        }
        Ok(id)
    }

    /// An Entry of the same name became another kind, such as a file replaced by a folder: what was under the old folder is gone.
    async fn change_kind(tx: &mut Transaction<'_, Sqlite>, library: &str, id: i64, old: NameKind, parent: &str, row: &NameRow) -> Result<()> {
        if old == NameKind::Folder
            && let Some(path) = Self::folder_path(tx, id).await?
        {
            Self::delete_inside(tx, library, &path).await?;
        }
        if row.kind == NameKind::Folder {
            sqlx::query("INSERT OR REPLACE INTO name_folders (id, library_id, path) VALUES (?, ?, ?)")
                .bind(id)
                .bind(library)
                .bind(EntryPath::join(parent, &row.name))
                .execute(&mut **tx)
                .await?;
        }
        sqlx::query("UPDATE name_entries SET kind = ? WHERE id = ?")
            .bind(row.kind.code())
            .bind(id)
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    async fn folder_path(tx: &mut Transaction<'_, Sqlite>, id: i64) -> Result<Option<String>> {
        Ok(sqlx::query_scalar("SELECT path FROM name_folders WHERE id = ?").bind(id).fetch_optional(&mut **tx).await?)
    }

    async fn stamp(db: &SqlitePool, library: &str, folder: &str, generation: i64) -> Result<()> {
        let mut tx = Self::begin(db).await?;
        Self::stamp_in(&mut tx, library, folder, generation).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Stamps the Entries under the folder, and nothing else, with `generation` unless they have a later one.
    async fn stamp_in(tx: &mut Transaction<'_, Sqlite>, library: &str, folder: &str, generation: i64) -> Result<()> {
        let range = FolderRange::of(folder);
        let sql = format!(
            "UPDATE name_entries SET generation = ? WHERE generation < ? AND parent_id IN (SELECT id FROM name_folders WHERE {})",
            FolderRange::condition(&range)
        );
        let query = sqlx::query(sqlx::AssertSqlSafe(sql)).bind(generation).bind(generation);
        FolderRange::bind(&range, library, query).execute(&mut **tx).await?;
        Ok(())
    }

    async fn delete_stale(db: &SqlitePool, library: &str, scope: &str, generation: i64) -> Result<()> {
        let range = FolderRange::of(scope);
        let sql = format!(
            "SELECT id, kind FROM name_entries WHERE generation < ? AND parent_id IN (SELECT id FROM name_folders WHERE {}) LIMIT {SWEEP_BATCH}",
            FolderRange::condition(&range)
        );
        loop {
            let query = sqlx::query(sqlx::AssertSqlSafe(sql.clone())).bind(generation);
            let stale = FolderRange::bind(&range, library, query).fetch_all(db).await?;
            if stale.is_empty() {
                return Ok(());
            }
            let mut tx = Self::begin(db).await?;
            for row in stale {
                Self::delete_entry(&mut tx, library, row.try_get("id")?, NameKind::from_code(row.try_get("kind")?)).await?;
            }
            tx.commit().await?;
        }
    }

    async fn delete_path(db: &SqlitePool, library: &str, path: &str) -> Result<()> {
        let mut tx = Self::begin(db).await?;
        if let Some((id, kind)) = Self::entry_at(&mut tx, library, path).await? {
            Self::delete_entry(&mut tx, library, id, kind).await?;
        } else if EntryPath::split(path).is_none() {
            // The Library's root: everything.
            Self::delete_inside(&mut tx, library, "/").await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// The id and kind of the Entry at `path`, if the index holds it.
    async fn entry_at(tx: &mut Transaction<'_, Sqlite>, library: &str, path: &str) -> Result<Option<(i64, NameKind)>> {
        let Some((parent, name)) = EntryPath::split(path) else {
            return Ok(None);
        };
        let Some(parent_id) = Self::folder_id(tx, library, &parent).await? else {
            return Ok(None);
        };
        let found: Option<(i64, i64)> = sqlx::query_as("SELECT id, kind FROM name_entries WHERE parent_id = ? AND name = ?")
            .bind(parent_id)
            .bind(name)
            .fetch_optional(&mut **tx)
            .await?;
        Ok(found.map(|(id, kind)| (id, NameKind::from_code(kind))))
    }

    async fn move_path(db: &SqlitePool, library: &str, src: &str, dst: &str) -> Result<()> {
        // A Library's root cannot be moved, and nothing asks for it, so it is not worth stopping the index for.
        let (Some(_), Some((dst_parent, dst_name))) = (EntryPath::split(src), EntryPath::split(dst)) else {
            return Ok(());
        };
        let mut tx = Self::begin(db).await?;
        if let Some((id, kind)) = Self::entry_at(&mut tx, library, dst).await? {
            Self::delete_entry(&mut tx, library, id, kind).await?;
        }
        if EntryPath::is_hidden(dst) {
            // Moved out of sight: the index holds no hidden Entries, and aggregation does not look at the destination.
            if let Some((id, kind)) = Self::entry_at(&mut tx, library, src).await? {
                Self::delete_entry(&mut tx, library, id, kind).await?;
            }
            tx.commit().await?;
            return Ok(());
        }
        let Some((id, kind)) = Self::entry_at(&mut tx, library, src).await? else {
            tx.commit().await?;
            return Ok(());
        };
        let generation: i64 = sqlx::query_scalar("SELECT generation FROM name_entries WHERE id = ?").bind(id).fetch_one(&mut *tx).await?;
        // A full aggregation in progress may already have listed the destination's folder and not the source's, and its sweep would delete what
        // it did not see. What is moved exists, and so does any folder made for it, so they carry the generation of the aggregation that is running.
        let running: Option<i64> = sqlx::query_scalar("SELECT generation FROM name_scans WHERE library_id = ?")
            .bind(library)
            .fetch_optional(&mut *tx)
            .await?;
        let parent_id = Self::ensure_folder(&mut tx, library, &dst_parent, generation.max(running.unwrap_or(0))).await?;
        let (normalized, stored) = NameRow::new(&dst_name, kind).normalized();
        sqlx::query("UPDATE name_entries SET parent_id = ?, name = ?, norm = ? WHERE id = ?")
            .bind(parent_id)
            .bind(&dst_name)
            .bind(stored)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM name_fts WHERE rowid = ?").bind(id).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO name_fts (rowid, norm) VALUES (?, ?)")
            .bind(id)
            .bind(normalized)
            .execute(&mut *tx)
            .await?;
        if kind == NameKind::Folder {
            let range = FolderRange::of(src);
            let offset = i64::try_from(src.chars().count() + 1)?;
            let sql = format!("UPDATE name_folders SET path = ? || substr(path, ?) WHERE {}", FolderRange::condition(&range));
            let query = sqlx::query(sqlx::AssertSqlSafe(sql)).bind(dst).bind(offset);
            FolderRange::bind(&range, library, query).execute(&mut *tx).await?;
        }
        if let Some(running) = running {
            sqlx::query("UPDATE name_entries SET generation = ? WHERE id = ? AND generation < ?")
                .bind(running)
                .bind(id)
                .bind(running)
                .execute(&mut *tx)
                .await?;
            if kind == NameKind::Folder {
                Self::stamp_in(&mut tx, library, dst, running).await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }

    /// Deletes an Entry and, for a folder, everything inside it.
    async fn delete_entry(tx: &mut Transaction<'_, Sqlite>, library: &str, id: i64, kind: NameKind) -> Result<()> {
        if kind == NameKind::Folder
            && let Some(path) = Self::folder_path(tx, id).await?
        {
            Self::delete_inside(tx, library, &path).await?;
        }
        sqlx::query("DELETE FROM name_fts WHERE rowid = ?").bind(id).execute(&mut **tx).await?;
        sqlx::query("DELETE FROM name_entries WHERE id = ?").bind(id).execute(&mut **tx).await?;
        Ok(())
    }

    /// Deletes everything inside the folder at `path`, and the folder rows of that range, including the folder's own row.
    /// The folder's own Entry is not deleted.
    async fn delete_inside(tx: &mut Transaction<'_, Sqlite>, library: &str, path: &str) -> Result<()> {
        let range = FolderRange::of(path);
        let condition = FolderRange::condition(&range);
        for sql in [
            format!("DELETE FROM name_fts WHERE rowid IN (SELECT id FROM name_entries WHERE parent_id IN (SELECT id FROM name_folders WHERE {condition}))"),
            format!("DELETE FROM name_entries WHERE parent_id IN (SELECT id FROM name_folders WHERE {condition})"),
        ] {
            FolderRange::bind(&range, library, sqlx::query(sqlx::AssertSqlSafe(sql))).execute(&mut **tx).await?;
        }
        let sql = format!("DELETE FROM name_folders WHERE {condition}");
        FolderRange::bind(&range, library, sqlx::query(sqlx::AssertSqlSafe(sql))).execute(&mut **tx).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use testresult::TestResult;

    use super::*;

    const LIB: &str = "photos";

    fn files(names: &[&str]) -> Vec<NameRow> {
        names.iter().map(|name| NameRow::new(name, NameKind::File)).collect()
    }

    fn folders(names: &[&str]) -> Vec<NameRow> {
        names.iter().map(|name| NameRow::new(name, NameKind::Folder)).collect()
    }

    async fn open(dir: &tempfile::TempDir) -> NameIndex {
        let index = NameIndex::open(&dir.path().join("listing")).await;
        // The index answers only after its integrity check.
        index.verified().await;
        index
    }

    fn pool(index: &NameIndex) -> &SqlitePool {
        index.db.as_ref().expect("the index is open")
    }

    async fn held(index: &NameIndex, library: &str) -> TestResult<BTreeSet<String>> {
        Ok(index.held(library).await.into_iter().collect())
    }

    async fn matches(index: &NameIndex, term: &str) -> TestResult<Vec<String>> {
        index.flush().await;
        let rows = sqlx::query(
            "SELECT f.path AS folder, e.name AS name FROM name_fts t JOIN name_entries e ON e.id = t.rowid JOIN name_folders f ON f.id = e.parent_id WHERE name_fts MATCH ? ORDER BY e.id",
        )
        .bind(format!("\"{term}\""))
        .fetch_all(pool(index))
        .await?;
        Ok(rows
            .iter()
            .map(|row| {
                let folder: String = row.get("folder");
                let name: String = row.get("name");
                EntryPath::join(&folder, &name)
            })
            .collect())
    }

    async fn count(index: &NameIndex, table: &str) -> TestResult<i64> {
        index.flush().await;
        Ok(sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
            .fetch_one(pool(index))
            .await?)
    }

    /// The trigram table has a row for exactly each Entry, and none for a Library's root.
    async fn assert_consistent(index: &NameIndex) -> TestResult {
        let entries: i64 = sqlx::query_scalar("SELECT count(*) FROM name_entries WHERE parent_id <> 0").fetch_one(pool(index)).await?;
        assert_eq!(count(index, "name_fts_docsize").await?, entries);
        let orphans: i64 = sqlx::query_scalar("SELECT count(*) FROM name_entries e WHERE e.parent_id <> 0 AND NOT EXISTS (SELECT 1 FROM name_folders f WHERE f.id = e.parent_id)")
            .fetch_one(pool(index))
            .await?;
        assert_eq!(orphans, 0, "every Entry is in a known folder");
        Ok(())
    }

    fn set(entries: &[&str]) -> BTreeSet<String> {
        entries.iter().map(|entry| entry.to_string()).collect()
    }

    #[tokio::test]
    async fn children_are_held_with_their_kinds_and_normalized_names_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = open(&dir).await;
        index
            .replace_children(
                LIB,
                "/",
                &[
                    NameRow::new("Trip", NameKind::Folder),
                    NameRow::new("IMG_1.JPG", NameKind::Image),
                    NameRow::new("notes.txt", NameKind::File),
                    NameRow::new("a.zip", NameKind::Archive),
                    NameRow::new("clip.mp4", NameKind::Video),
                ],
                1,
            )
            .await;
        index.replace_children(LIB, "/Trip", &files(&["Ｆｕｊｉ.png"]), 1).await;
        assert_eq!(
            held(&index, LIB).await?,
            set(&["/Trip:0", "/IMG_1.JPG:3", "/notes.txt:1", "/a.zip:2", "/clip.mp4:4", "/Trip/Ｆｕｊｉ.png:1"])
        );
        // Names are matched normalized (NFKC and lowercase) and shown as they are.
        assert_eq!(matches(&index, "fuji").await?, vec!["/Trip/Ｆｕｊｉ.png"]);
        assert_eq!(matches(&index, "img_1").await?, vec!["/IMG_1.JPG"]);
        assert!(matches(&index, "IMG_1").await?.is_empty(), "the trigram table holds normalized names only");
        let stored: Option<String> = sqlx::query_scalar("SELECT norm FROM name_entries WHERE name = 'notes.txt'").fetch_one(pool(&index)).await?;
        assert_eq!(stored, None, "a name equal to its normalized form stores no copy");
        assert_consistent(&index).await
    }

    #[tokio::test]
    async fn a_folder_the_index_does_not_know_is_created_with_the_folders_above_it_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = open(&dir).await;
        index.replace_children(LIB, "/a/b/c", &files(&["deep.txt"]), 1).await;
        assert_eq!(held(&index, LIB).await?, set(&["/a:0", "/a/b:0", "/a/b/c:0", "/a/b/c/deep.txt:1"]));
        // Another enumeration of the root finds them already held.
        index.replace_children(LIB, "/", &folders(&["a"]), 2).await;
        assert_eq!(count(&index, "name_folders").await?, 4);
        assert_consistent(&index).await
    }

    #[tokio::test]
    async fn a_rewrite_keeps_what_it_does_not_list_until_the_sweep_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = open(&dir).await;
        index.replace_children(LIB, "/", &[files(&["keep.txt", "gone.txt"]), folders(&["old"])].concat(), 1).await;
        index.replace_children(LIB, "/old", &files(&["inside.txt"]), 1).await;
        // The next full aggregation sees only `keep.txt`; an Entry that appeared meanwhile was stamped with the new generation.
        index.replace_children(LIB, "/", &files(&["keep.txt"]), 2).await;
        index.upsert(LIB, "/new.txt", NameKind::File, 2).await;
        assert_eq!(held(&index, LIB).await?.len(), 5, "nothing is deleted before the sweep");
        index.sweep(LIB, "/", 2).await;
        assert_eq!(held(&index, LIB).await?, set(&["/keep.txt:1", "/new.txt:1"]));
        assert_consistent(&index).await
    }

    #[tokio::test]
    async fn an_older_generation_does_not_lower_a_stamp_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = open(&dir).await;
        index.replace_children(LIB, "/", &files(&["a.txt"]), 5).await;
        // A range aggregation that read its generation before a full aggregation began writes with the older one.
        index.replace_children(LIB, "/", &files(&["a.txt"]), 3).await;
        index.upsert(LIB, "/a.txt", NameKind::File, 4).await;
        index.sweep(LIB, "/", 5).await;
        assert_eq!(held(&index, LIB).await?, set(&["/a.txt:1"]));
        Ok(())
    }

    #[tokio::test]
    async fn a_scoped_sweep_stays_inside_its_scope_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = open(&dir).await;
        index.replace_children(LIB, "/", &folders(&["a", "ab"]), 1).await;
        index.replace_children(LIB, "/a", &files(&["x.txt", "y.txt"]), 1).await;
        index.replace_children(LIB, "/ab", &files(&["z.txt"]), 1).await;
        index.replace_children(LIB, "/a", &files(&["x.txt"]), 2).await;
        index.sweep(LIB, "/a", 2).await;
        assert_eq!(held(&index, LIB).await?, set(&["/a:0", "/ab:0", "/a/x.txt:1", "/ab/z.txt:1"]));
        Ok(())
    }

    #[tokio::test]
    async fn a_swept_folder_takes_everything_under_it_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = open(&dir).await;
        index.replace_children(LIB, "/", &folders(&["gone", "stay"]), 1).await;
        index.replace_children(LIB, "/gone", &[folders(&["deep"]), files(&["a.txt"])].concat(), 1).await;
        index.replace_children(LIB, "/gone/deep", &files(&["b.txt"]), 1).await;
        // Something under the vanished folder was stamped by a range aggregation of the new generation.
        index.upsert(LIB, "/gone/deep/b.txt", NameKind::File, 2).await;
        index.replace_children(LIB, "/", &folders(&["stay"]), 2).await;
        index.sweep(LIB, "/", 2).await;
        assert_eq!(held(&index, LIB).await?, set(&["/stay:0"]));
        assert_eq!(count(&index, "name_folders").await?, 2, "the root and /stay");
        assert_consistent(&index).await
    }

    #[tokio::test]
    async fn keep_protects_a_folder_that_could_not_be_read_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = open(&dir).await;
        index.replace_children(LIB, "/", &folders(&["locked"]), 1).await;
        index.replace_children(LIB, "/locked", &files(&["a.txt"]), 1).await;
        index.replace_children(LIB, "/", &folders(&["locked"]), 2).await;
        index.keep(LIB, "/locked", 2).await;
        index.sweep(LIB, "/", 2).await;
        assert_eq!(held(&index, LIB).await?, set(&["/locked:0", "/locked/a.txt:1"]));
        Ok(())
    }

    #[tokio::test]
    async fn remove_deletes_the_path_and_everything_under_it_but_not_a_sibling_with_the_same_prefix_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = open(&dir).await;
        index.replace_children(LIB, "/", &[folders(&["a", "ab"]), files(&["a.txt"])].concat(), 1).await;
        index.replace_children(LIB, "/a", &[folders(&["sub"]), files(&["x.txt"])].concat(), 1).await;
        index.replace_children(LIB, "/a/sub", &files(&["y.txt"]), 1).await;
        index.replace_children(LIB, "/ab", &files(&["z.txt"]), 1).await;
        index.remove(LIB, "/a").await;
        assert_eq!(held(&index, LIB).await?, set(&["/ab:0", "/a.txt:1", "/ab/z.txt:1"]));
        assert!(matches(&index, "x.txt").await?.is_empty());
        index.remove(LIB, "/a.txt").await;
        index.remove(LIB, "/missing").await;
        assert_eq!(held(&index, LIB).await?, set(&["/ab:0", "/ab/z.txt:1"]));
        assert_consistent(&index).await
    }

    #[tokio::test]
    async fn rename_moves_an_entry_and_the_paths_under_it_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = open(&dir).await;
        index.replace_children(LIB, "/", &[folders(&["a", "ab", "dest"])].concat(), 1).await;
        index.replace_children(LIB, "/a", &[folders(&["sub"]), files(&["Old.txt"])].concat(), 1).await;
        index.replace_children(LIB, "/a/sub", &files(&["deep.txt"]), 1).await;
        index.replace_children(LIB, "/ab", &files(&["z.txt"]), 1).await;

        index.rename(LIB, "/a", "/dest/moved").await;
        assert_eq!(
            held(&index, LIB).await?,
            set(&[
                "/ab:0",
                "/dest:0",
                "/ab/z.txt:1",
                "/dest/moved:0",
                "/dest/moved/sub:0",
                "/dest/moved/Old.txt:1",
                "/dest/moved/sub/deep.txt:1"
            ])
        );
        assert_eq!(matches(&index, "deep").await?, vec!["/dest/moved/sub/deep.txt"]);

        index.rename(LIB, "/dest/moved/Old.txt", "/dest/New Name.txt").await;
        assert_eq!(matches(&index, "new name").await?, vec!["/dest/New Name.txt"]);
        assert!(matches(&index, "old.txt").await?.is_empty(), "the old name leaves the trigram table");
        assert_consistent(&index).await
    }

    #[tokio::test]
    async fn rename_replaces_what_was_at_the_destination_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = open(&dir).await;
        index.replace_children(LIB, "/", &folders(&["src", "dst"]), 1).await;
        index.replace_children(LIB, "/src", &files(&["new.txt"]), 1).await;
        index.replace_children(LIB, "/dst", &files(&["old.txt"]), 1).await;
        index.rename(LIB, "/src", "/dst").await;
        assert_eq!(held(&index, LIB).await?, set(&["/dst:0", "/dst/new.txt:1"]));
        assert_consistent(&index).await
    }

    #[tokio::test]
    async fn rename_of_an_entry_the_index_does_not_hold_changes_nothing_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = open(&dir).await;
        index.replace_children(LIB, "/", &files(&["a.txt"]), 1).await;
        index.mark_usable(LIB, 1).await;
        index.rename(LIB, "/missing", "/b.txt").await;
        index.rename(LIB, "/", "/b").await;
        index.rename(LIB, "/a.txt", "/").await;
        assert_eq!(held(&index, LIB).await?, set(&["/a.txt:1"]));
        assert!(index.is_usable(LIB).await, "an impossible rename does not stop the index");
        Ok(())
    }

    #[tokio::test]
    async fn a_file_replaced_by_a_folder_and_back_changes_kind_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = open(&dir).await;
        index.replace_children(LIB, "/", &files(&["thing"]), 1).await;
        index.replace_children(LIB, "/", &folders(&["thing"]), 2).await;
        index.replace_children(LIB, "/thing", &files(&["inside.txt"]), 2).await;
        assert_eq!(held(&index, LIB).await?, set(&["/thing:0", "/thing/inside.txt:1"]));
        index.replace_children(LIB, "/", &files(&["thing"]), 3).await;
        assert_eq!(held(&index, LIB).await?, set(&["/thing:1"]), "what was inside the folder is gone with it");
        assert_consistent(&index).await
    }

    #[tokio::test]
    async fn libraries_are_kept_apart_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = open(&dir).await;
        index.replace_children("one", "/", &files(&["a.txt"]), 1).await;
        index.replace_children("two", "/", &files(&["a.txt"]), 1).await;
        index.remove("one", "/").await;
        assert!(held(&index, "one").await?.is_empty());
        assert_eq!(held(&index, "two").await?, set(&["/a.txt:1"]));
        index.replace_children("two", "/", &files(&["b.txt"]), 2).await;
        index.sweep("one", "/", 9).await;
        assert_eq!(held(&index, "two").await?.len(), 2);
        assert_consistent(&index).await
    }

    #[tokio::test]
    async fn usability_is_recorded_per_library_and_survives_a_restart_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = open(&dir).await;
        assert!(!index.is_usable(LIB).await);
        index.mark_usable(LIB, 3).await;
        assert!(index.is_usable(LIB).await);
        assert!(!index.is_usable("other").await);
        pool(&index).close().await;
        let reopened = open(&dir).await;
        assert!(reopened.is_usable(LIB).await);
        Ok(())
    }

    #[tokio::test]
    async fn a_corrupt_file_is_discarded_and_the_next_aggregation_rebuilds_it_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let listing = dir.path().join("listing");
        tokio::fs::create_dir_all(&listing).await?;
        let auth = dir.path().join("pxna.db");
        tokio::fs::write(&auth, b"authentication").await?;
        tokio::fs::write(listing.join(DATABASE_FILE_NAME), b"this is not a sqlite file at all, just text".repeat(64)).await?;
        let index = open(&dir).await;
        assert!(!index.is_usable(LIB).await);
        index.replace_children(LIB, "/", &files(&["a.txt"]), 1).await;
        assert_eq!(held(&index, LIB).await?, set(&["/a.txt:1"]));
        assert_eq!(tokio::fs::read(&auth).await?, b"authentication");
        Ok(())
    }

    #[tokio::test]
    async fn a_file_of_another_format_is_discarded_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = open(&dir).await;
        index.replace_children(LIB, "/", &files(&["a.txt"]), 1).await;
        index.mark_usable(LIB, 1).await;
        sqlx::query("PRAGMA user_version = 99").execute(pool(&index)).await?;
        pool(&index).close().await;
        let reopened = open(&dir).await;
        assert!(!reopened.is_usable(LIB).await);
        assert!(held(&reopened, LIB).await?.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn an_error_stops_the_index_for_the_process_and_the_next_startup_discards_it_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = open(&dir).await;
        index.replace_children(LIB, "/", &files(&["a.txt"]), 1).await;
        index.mark_usable(LIB, 1).await;
        assert!(index.is_usable(LIB).await);
        // An I/O error while running: the pool can no longer be used.
        pool(&index).close().await;
        index.replace_children(LIB, "/", &files(&["b.txt"]), 2).await;
        index.flush().await;
        assert!(!index.is_usable(LIB).await, "the index stops answering");
        index.replace_children(LIB, "/", &files(&["c.txt"]), 3).await;
        assert!(dir.path().join("listing").join(BROKEN_MARKER_NAME).exists());

        let reopened = open(&dir).await;
        assert!(!reopened.is_usable(LIB).await, "a Library waits for a full aggregation again");
        assert!(held(&reopened, LIB).await?.is_empty());
        assert!(!dir.path().join("listing").join(BROKEN_MARKER_NAME).exists());
        Ok(())
    }

    #[tokio::test]
    async fn an_entry_renamed_to_a_hidden_path_leaves_the_index_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = open(&dir).await;
        index
            .replace_children(LIB, "/", &[files(&["visible.png", "other.png"]), folders(&["folder"])].concat(), 1)
            .await;
        index.replace_children(LIB, "/folder", &files(&["inside.png"]), 1).await;

        index.rename(LIB, "/visible.png", "/.visible.png").await;
        index.rename(LIB, "/folder", "/.folder").await;
        index.rename(LIB, "/other.png", "/.hidden-folder/other.png").await;
        assert_eq!(held(&index, LIB).await?, set(&[]));
        assert!(matches(&index, "visible").await?.is_empty() && matches(&index, "inside").await?.is_empty());
        assert_consistent(&index).await
    }

    #[tokio::test]
    async fn a_full_aggregation_whose_generation_went_back_starts_the_index_again_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = open(&dir).await;
        index.begin_scan(LIB, 4).await;
        index.replace_children(LIB, "/", &files(&["a.txt"]), 4).await;
        index.mark_usable(LIB, 4).await;
        index.begin_scan(LIB, 5).await;
        assert!(index.is_usable(LIB).await, "a later aggregation keeps the index usable while it runs");
        assert_eq!(held(&index, LIB).await?, set(&["/a.txt:1"]));

        // Only the timeline index was recreated, so its generations count from 1 again while the rows carry older ones.
        index.begin_scan(LIB, 1).await;
        assert!(!index.is_usable(LIB).await);
        assert!(held(&index, LIB).await?.is_empty(), "rows stamped by the earlier generations could never be swept");
        index.replace_children(LIB, "/", &files(&["b.txt"]), 1).await;
        index.sweep(LIB, "/", 1).await;
        index.mark_usable(LIB, 1).await;
        assert_eq!(held(&index, LIB).await?, set(&["/b.txt:1"]));
        assert_consistent(&index).await
    }

    #[tokio::test]
    async fn a_search_stops_when_its_cursor_is_dropped_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = open(&dir).await;
        let names = (0..3000).map(|number| format!("match_{number:04}.txt")).collect::<Vec<_>>();
        let rows = names.iter().map(|name| NameRow::new(name, NameKind::File)).collect::<Vec<_>>();
        index.replace_children(LIB, "/", &rows, 1).await;
        index.mark_usable(LIB, 1).await;

        let query = || NameQuery {
            library: LIB.to_string(),
            start: "/".to_string(),
            terms: vec!["match".to_string()],
            only_media: false,
        };
        let mut cursor = index.search(query()).expect("the index can search");
        let first = cursor.next().await?.expect("there are matches");
        assert_eq!(first.len(), SEARCH_BATCH);
        let task = cursor.abort_handle();
        drop(cursor);
        for _ in 0..200 {
            if task.is_finished() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert!(task.is_finished(), "the query stops with its cursor");
        // Nothing else is hurt by it.
        assert!(index.is_usable(LIB).await);
        let mut all = index.search(query()).expect("the index can search");
        let mut total = 0;
        while let Some(batch) = all.next().await? {
            total += batch.len();
        }
        assert_eq!(total, 3000);
        Ok(())
    }

    #[tokio::test]
    async fn a_structurally_corrupt_file_stops_the_index_and_is_discarded_next_time_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = open(&dir).await;
        let rows = (0..2000).map(|number| NameRow::new(&format!("entry_{number:05}.txt"), NameKind::File)).collect::<Vec<_>>();
        index.replace_children(LIB, "/", &rows, 1).await;
        index.mark_usable(LIB, 1).await;
        sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)").execute(pool(&index)).await?;
        pool(&index).close().await;
        drop(index);

        // A valid SQLite header with damaged pages behind it: the cheap checks of opening pass, the integrity check does not.
        let file = dir.path().join("listing").join(DATABASE_FILE_NAME);
        let mut bytes = std::fs::read(&file)?;
        assert!(bytes.len() > 4096 * 20);
        // The pages the cheap checks of opening read are near the front, so damage the second half.
        let (from, to) = (bytes.len() / 2 / 4096 * 4096, bytes.len() * 9 / 10 / 4096 * 4096);
        for byte in &mut bytes[from..to] {
            *byte = 0xFF;
        }
        std::fs::write(&file, bytes)?;
        for suffix in ["-wal", "-shm"] {
            let _ = std::fs::remove_file(dir.path().join("listing").join(format!("{DATABASE_FILE_NAME}{suffix}")));
        }

        let corrupt = open(&dir).await;
        corrupt.verified().await;
        assert!(!corrupt.is_usable(LIB).await, "the index stops answering");
        assert!(dir.path().join("listing").join(BROKEN_MARKER_NAME).exists());
        drop(corrupt);

        let rebuilt = open(&dir).await;
        rebuilt.verified().await;
        assert!(!rebuilt.is_usable(LIB).await, "a Library waits for a full aggregation again");
        assert!(held(&rebuilt, LIB).await?.is_empty());
        rebuilt.replace_children(LIB, "/", &files(&["a.txt"]), 1).await;
        assert_eq!(held(&rebuilt, LIB).await?, set(&["/a.txt:1"]));
        Ok(())
    }

    #[tokio::test]
    async fn the_index_answers_nothing_until_its_file_has_been_checked_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = NameIndex::open_with(&dir.path().join("listing"), false).await;
        index.replace_children(LIB, "/", &files(&["a.txt"]), 1).await;
        index.mark_usable(LIB, 1).await;
        let query = || NameQuery {
            library: LIB.to_string(),
            start: "/".to_string(),
            terms: vec!["a.t".to_string()],
            only_media: false,
        };
        assert!(!index.is_usable(LIB).await, "an unchecked file is not trusted");
        assert!(index.search(query()).is_none());
        assert!(index.lookup(LIB, &["/a.txt".to_string()], &[], false).await.is_none());

        index.verify_now().await;
        assert!(index.is_usable(LIB).await);
        assert!(index.search(query()).is_some());
        assert_eq!(index.lookup(LIB, &["/a.txt".to_string()], &[], false).await.map(|hits| hits.len()), Some(1));
        Ok(())
    }

    #[tokio::test]
    async fn an_entry_moved_during_a_full_aggregation_survives_its_sweep_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = open(&dir).await;
        index.begin_scan(LIB, 1).await;
        index.replace_children(LIB, "/", &folders(&["from", "into"]), 1).await;
        index
            .replace_children(LIB, "/from", &[files(&["moved.txt", "stays.txt"]), folders(&["tree"])].concat(), 1)
            .await;
        index.replace_children(LIB, "/from/tree", &files(&["leaf.txt"]), 1).await;
        index.sweep(LIB, "/", 1).await;
        index.mark_usable(LIB, 1).await;

        // The next full aggregation has listed the destination's folder but not the source's when the Entries are moved.
        index.begin_scan(LIB, 2).await;
        index.replace_children(LIB, "/", &folders(&["from", "into"]), 2).await;
        index.replace_children(LIB, "/into", &[], 2).await;
        index.rename(LIB, "/from/moved.txt", "/into/moved.txt").await;
        index.rename(LIB, "/from/tree", "/into/tree").await;
        index.replace_children(LIB, "/from", &files(&["stays.txt"]), 2).await;
        index.sweep(LIB, "/", 2).await;
        assert_eq!(
            held(&index, LIB).await?,
            set(&["/from:0", "/into:0", "/from/stays.txt:1", "/into/moved.txt:1", "/into/tree:0", "/into/tree/leaf.txt:1"]),
            "what was moved is not swept for having been listed at its old place"
        );
        assert_consistent(&index).await
    }

    #[tokio::test]
    async fn an_entry_moved_into_a_folder_the_index_did_not_hold_survives_the_sweep_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = open(&dir).await;
        index.begin_scan(LIB, 1).await;
        index.replace_children(LIB, "/", &folders(&["from"]), 1).await;
        index.replace_children(LIB, "/from", &files(&["moved.txt"]), 1).await;
        index.sweep(LIB, "/", 1).await;
        index.mark_usable(LIB, 1).await;

        // The new folders were made by the daemon after the aggregation listed the root, so the index has never seen them.
        index.begin_scan(LIB, 2).await;
        index.replace_children(LIB, "/", &folders(&["from"]), 2).await;
        index.rename(LIB, "/from/moved.txt", "/new/deeper/moved.txt").await;
        index.replace_children(LIB, "/from", &[], 2).await;
        index.sweep(LIB, "/", 2).await;
        assert_eq!(held(&index, LIB).await?, set(&["/from:0", "/new:0", "/new/deeper:0", "/new/deeper/moved.txt:1"]));
        assert_consistent(&index).await
    }

    #[tokio::test]
    async fn a_disabled_index_ignores_everything_test() -> TestResult {
        let index = NameIndex::disabled();
        index.replace_children(LIB, "/", &files(&["a.txt"]), 1).await;
        index.upsert(LIB, "/b.txt", NameKind::File, 1).await;
        index.remove(LIB, "/a.txt").await;
        index.rename(LIB, "/a.txt", "/c.txt").await;
        index.sweep(LIB, "/", 1).await;
        index.mark_usable(LIB, 1).await;
        assert!(!index.is_usable(LIB).await);
        Ok(())
    }

    #[tokio::test]
    async fn a_directory_that_cannot_be_created_gives_a_disabled_index_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let blocker = dir.path().join("blocker");
        tokio::fs::write(&blocker, b"a file where the directory should be").await?;
        let index = NameIndex::open(&blocker.join("listing")).await;
        assert!(!index.is_usable(LIB).await);
        Ok(())
    }
}
