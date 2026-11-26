use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use chrono::Utc;
use sqlx::{Sqlite, SqlitePool, Transaction};

use omnius_core_base::{clock::Clock, tsid::TsidProvider};
use omnius_core_migration::sqlite::{MigrationRequest, SqliteMigrator};

use crate::prelude::*;

/// Reads and writes each user's Shelves. It migrates its tables itself.
///
/// The migration alters `users`, so `AuthService` must be created on the same database first.
/// The Scope, the Filter, and the paths of the starts are checked by the caller, which gives the paths in normal form.
pub struct ShelfService {
    db: SqlitePool,
    clock: Arc<dyn Clock<Utc> + Send + Sync>,
    tsid_provider: Arc<Mutex<dyn TsidProvider + Send + Sync>>,
}

#[derive(sqlx::FromRow)]
struct ShelfRecord {
    id: String,
    name: String,
    recursive: bool,
    inside_archives: bool,
    filter: String,
    display: String,
}

#[derive(sqlx::FromRow)]
struct StartRecord {
    shelf_id: String,
    library_id: String,
    path: String,
}

impl ShelfService {
    /// How many Shelves a user has at most.
    pub const MAX_SHELVES: usize = 200;

    pub async fn new(db: SqlitePool, clock: Arc<dyn Clock<Utc> + Send + Sync>, tsid_provider: Arc<Mutex<dyn TsidProvider + Send + Sync>>) -> Result<Self> {
        let requests = vec![MigrationRequest {
            name: "2026-11-26_shelves_and_folder_settings".to_string(),
            queries: r#"
CREATE TABLE shelves (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    position INTEGER NOT NULL,
    recursive INTEGER NOT NULL,
    inside_archives INTEGER NOT NULL,
    filter TEXT NOT NULL,
    display TEXT NOT NULL
);
CREATE INDEX shelves_user ON shelves (user_id, position);

CREATE TABLE shelf_starts (
    shelf_id TEXT NOT NULL REFERENCES shelves (id),
    position INTEGER NOT NULL,
    library_id TEXT NOT NULL,
    path TEXT NOT NULL,
    PRIMARY KEY (shelf_id, position)
);
CREATE INDEX shelf_starts_path ON shelf_starts (library_id, path);

CREATE TABLE folder_settings (
    user_id TEXT NOT NULL,
    library_id TEXT NOT NULL,
    path TEXT NOT NULL,
    display TEXT NOT NULL,
    PRIMARY KEY (user_id, library_id, path)
);

ALTER TABLE users ADD COLUMN default_shelf_created_at TEXT;
"#
            .to_string(),
        }];
        SqliteMigrator::migrate(&db, requests).await?;
        Ok(Self { db, clock, tsid_provider })
    }

    /// The user's Shelves in their order.
    ///
    /// The first call that gives `default_starts` for a user creates the default Shelf over them and records it on the user, so it is not created again.
    /// With no start, nothing is created or recorded.
    pub async fn list(&self, user_id: &str, default_starts: &[ShelfStart]) -> Result<Vec<Shelf>> {
        self.create_default(user_id, default_starts).await?;
        let mut transaction = self.db.begin().await?;
        let shelves = Self::read(&mut transaction, user_id).await?;
        transaction.commit().await?;
        Ok(shelves)
    }

    /// Creates a Shelf and puts it last.
    pub async fn create(&self, user_id: &str, input: &ShelfInput) -> Result<Shelf> {
        let input = input.normalized()?;
        let mut transaction = self.db.begin_with("BEGIN IMMEDIATE").await?;
        if Self::count(&mut transaction, user_id).await? >= Self::MAX_SHELVES {
            return Err(Error::new(ErrorKind::InvalidInput).with_message(format!("a user can have at most {} shelves", Self::MAX_SHELVES)));
        }
        let shelf = self.insert(&mut transaction, user_id, input).await?;
        transaction.commit().await?;
        Ok(shelf)
    }

    /// Replaces a Shelf whole, keeping its id and position.
    pub async fn replace(&self, user_id: &str, shelf_id: &str, input: &ShelfInput) -> Result<Shelf> {
        let input = input.normalized()?;
        let mut transaction = self.db.begin_with("BEGIN IMMEDIATE").await?;
        let updated = sqlx::query("UPDATE shelves SET name = ?, recursive = ?, inside_archives = ?, filter = ?, display = ? WHERE id = ? AND user_id = ?")
            .bind(&input.name)
            .bind(input.scope.recursive)
            .bind(input.scope.inside_archives)
            .bind(Self::encode(&input.filter)?)
            .bind(Self::encode(&input.display)?)
            .bind(shelf_id)
            .bind(user_id)
            .execute(&mut *transaction)
            .await?
            .rows_affected();
        if updated == 0 {
            return Err(Self::not_found());
        }
        sqlx::query("DELETE FROM shelf_starts WHERE shelf_id = ?").bind(shelf_id).execute(&mut *transaction).await?;
        Self::insert_starts(&mut transaction, shelf_id, &input.scope.starts).await?;
        transaction.commit().await?;
        Ok(Shelf {
            id: shelf_id.to_string(),
            name: input.name,
            scope: input.scope,
            filter: input.filter,
            display: input.display,
        })
    }

    pub async fn delete(&self, user_id: &str, shelf_id: &str) -> Result<()> {
        let mut transaction = self.db.begin_with("BEGIN IMMEDIATE").await?;
        sqlx::query("DELETE FROM shelf_starts WHERE shelf_id IN (SELECT id FROM shelves WHERE id = ? AND user_id = ?)")
            .bind(shelf_id)
            .bind(user_id)
            .execute(&mut *transaction)
            .await?;
        let deleted = sqlx::query("DELETE FROM shelves WHERE id = ? AND user_id = ?")
            .bind(shelf_id)
            .bind(user_id)
            .execute(&mut *transaction)
            .await?
            .rows_affected();
        if deleted == 0 {
            return Err(Self::not_found());
        }
        transaction.commit().await?;
        Ok(())
    }

    /// Orders the user's Shelves as `ids` names them, which must be every one of them once, and returns them.
    pub async fn reorder(&self, user_id: &str, ids: &[String]) -> Result<Vec<Shelf>> {
        let mut transaction = self.db.begin_with("BEGIN IMMEDIATE").await?;
        let held = sqlx::query_scalar::<_, String>("SELECT id FROM shelves WHERE user_id = ?")
            .bind(user_id)
            .fetch_all(&mut *transaction)
            .await?
            .into_iter()
            .collect::<HashSet<_>>();
        let named = ids.iter().cloned().collect::<HashSet<_>>();
        if ids.len() != held.len() || named != held {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("the order must name every shelf exactly once"));
        }
        for (position, id) in ids.iter().enumerate() {
            sqlx::query("UPDATE shelves SET position = ? WHERE id = ?")
                .bind(position as i64)
                .bind(id)
                .execute(&mut *transaction)
                .await?;
        }
        let shelves = Self::read(&mut transaction, user_id).await?;
        transaction.commit().await?;
        Ok(shelves)
    }

    async fn create_default(&self, user_id: &str, starts: &[ShelfStart]) -> Result<()> {
        if starts.is_empty() {
            return Ok(());
        }
        // Every read passes here, so a user who has the record does not take the write lock.
        let pending = sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM users WHERE id = ? AND default_shelf_created_at IS NULL)")
            .bind(user_id)
            .fetch_one(&self.db)
            .await?;
        if !pending {
            return Ok(());
        }

        // The update comes first, so that of two first reads, the later one finds the record set.
        let mut transaction = self.db.begin().await?;
        let recorded = sqlx::query("UPDATE users SET default_shelf_created_at = ? WHERE id = ? AND default_shelf_created_at IS NULL")
            .bind(self.clock.now())
            .bind(user_id)
            .execute(&mut *transaction)
            .await?
            .rows_affected();
        // A user who already has the most Shelves gets the default one at a later read, once there is room.
        if recorded == 0 || Self::count(&mut transaction, user_id).await? >= Self::MAX_SHELVES {
            transaction.rollback().await?;
            return Ok(());
        }
        self.insert(&mut transaction, user_id, ShelfInput::default_shelf(starts)).await?;
        transaction.commit().await?;
        Ok(())
    }

    async fn insert(&self, transaction: &mut Transaction<'_, Sqlite>, user_id: &str, input: ShelfInput) -> Result<Shelf> {
        let id = format!("shf_{}", self.tsid_provider.lock().create());
        sqlx::query(
            "INSERT INTO shelves (id, user_id, name, position, recursive, inside_archives, filter, display) \
             SELECT ?1, ?2, ?3, COALESCE(MAX(position) + 1, 0), ?4, ?5, ?6, ?7 FROM shelves WHERE user_id = ?2",
        )
        .bind(&id)
        .bind(user_id)
        .bind(&input.name)
        .bind(input.scope.recursive)
        .bind(input.scope.inside_archives)
        .bind(Self::encode(&input.filter)?)
        .bind(Self::encode(&input.display)?)
        .execute(&mut **transaction)
        .await?;
        Self::insert_starts(transaction, &id, &input.scope.starts).await?;
        Ok(Shelf {
            id,
            name: input.name,
            scope: input.scope,
            filter: input.filter,
            display: input.display,
        })
    }

    async fn insert_starts(transaction: &mut Transaction<'_, Sqlite>, shelf_id: &str, starts: &[ShelfStart]) -> Result<()> {
        for (position, start) in starts.iter().enumerate() {
            sqlx::query("INSERT INTO shelf_starts (shelf_id, position, library_id, path) VALUES (?, ?, ?, ?)")
                .bind(shelf_id)
                .bind(position as i64)
                .bind(&start.library_id)
                .bind(&start.path)
                .execute(&mut **transaction)
                .await?;
        }
        Ok(())
    }

    async fn count(transaction: &mut Transaction<'_, Sqlite>, user_id: &str) -> Result<usize> {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM shelves WHERE user_id = ?")
            .bind(user_id)
            .fetch_one(&mut **transaction)
            .await?;
        Ok(usize::try_from(count)?)
    }

    async fn read(transaction: &mut Transaction<'_, Sqlite>, user_id: &str) -> Result<Vec<Shelf>> {
        let records: Vec<ShelfRecord> = sqlx::query_as("SELECT id, name, recursive, inside_archives, filter, display FROM shelves WHERE user_id = ? ORDER BY position")
            .bind(user_id)
            .fetch_all(&mut **transaction)
            .await?;
        let starts: Vec<StartRecord> = sqlx::query_as(
            "SELECT shelf_starts.shelf_id, shelf_starts.library_id, shelf_starts.path FROM shelf_starts \
             JOIN shelves ON shelves.id = shelf_starts.shelf_id WHERE shelves.user_id = ? ORDER BY shelf_starts.shelf_id, shelf_starts.position",
        )
        .bind(user_id)
        .fetch_all(&mut **transaction)
        .await?;
        let mut by_shelf = HashMap::<String, Vec<ShelfStart>>::new();
        for start in starts {
            by_shelf.entry(start.shelf_id).or_default().push(ShelfStart {
                library_id: start.library_id,
                path: start.path,
            });
        }
        records
            .into_iter()
            .map(|record| {
                Ok(Shelf {
                    scope: ShelfScope {
                        starts: by_shelf.remove(&record.id).unwrap_or_default(),
                        recursive: record.recursive,
                        inside_archives: record.inside_archives,
                    },
                    filter: Self::decode(&record.filter)?,
                    display: Self::decode(&record.display)?,
                    id: record.id,
                    name: record.name,
                })
            })
            .collect()
    }

    fn encode<T: serde::Serialize>(value: &T) -> Result<String> {
        serde_json::to_string(value).map_err(|error| Error::from_error(error, ErrorKind::SerdeError))
    }

    fn decode<T: serde::de::DeserializeOwned>(value: &str) -> Result<T> {
        serde_json::from_str(value).map_err(|error| Error::from_error(error, ErrorKind::SerdeError).with_message("a stored value is unreadable"))
    }

    fn not_found() -> Error {
        Error::new(ErrorKind::NotFound).with_message("shelf not found")
    }
}

#[cfg(test)]
mod tests {
    use omnius_core_base::{clock::ClockUtc, tsid::TsidProviderImpl};
    use rand::{SeedableRng as _, rngs::ChaCha20Rng};
    use testresult::TestResult;

    use super::*;
    use crate::service::AuthService;

    struct Fixture {
        _dir: tempfile::TempDir,
        service: ShelfService,
        /// The registered owner, whose row in `users` records the default Shelf.
        owner: String,
    }

    async fn fixture() -> TestResult<Fixture> {
        let dir = tempfile::tempdir()?;
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(dir.path().join("sqlite.db"))
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(10));
        let db = SqlitePool::connect_with(options).await?;
        let clock = Arc::new(ClockUtc);
        let tsid_provider = Arc::new(Mutex::new(TsidProviderImpl::new(ClockUtc, ChaCha20Rng::seed_from_u64(0), 16)));
        let auth = AuthService::new(db.clone(), clock.clone(), tsid_provider.clone()).await?;
        let owner = auth.create_owner("owner", "password1", SessionTransport::Bearer).await?.user.id;
        let service = ShelfService::new(db, clock, tsid_provider).await?;
        Ok(Fixture { _dir: dir, service, owner })
    }

    fn start(library_id: &str, path: &str) -> ShelfStart {
        ShelfStart {
            library_id: library_id.to_string(),
            path: path.to_string(),
        }
    }

    fn input(name: &str, starts: &[ShelfStart]) -> ShelfInput {
        ShelfInput {
            name: name.to_string(),
            scope: ShelfScope {
                starts: starts.to_vec(),
                recursive: false,
                inside_archives: false,
            },
            filter: ShelfFilter::default(),
            display: DisplayValues::default(),
        }
    }

    fn names(shelves: &[Shelf]) -> Vec<&str> {
        shelves.iter().map(|shelf| shelf.name.as_str()).collect()
    }

    #[tokio::test]
    async fn the_default_shelf_is_created_once_from_the_given_starts_test() -> TestResult {
        let Fixture { _dir, service, owner } = fixture().await?;
        let starts = [start("photos", "/"), start("videos", "/")];

        let shelves = service.list(&owner, &starts).await?;
        assert_eq!(names(&shelves), ["Timeline"]);
        let default = &shelves[0];
        assert!(default.id.starts_with("shf_"));
        assert_eq!(
            default.scope,
            ShelfScope {
                starts: starts.to_vec(),
                recursive: true,
                inside_archives: true
            }
        );
        assert_eq!(default.filter.kinds, Some(vec!["image".to_string(), "video".to_string()]));
        assert_eq!(default.display.wide.and_then(|set| set.view), Some(DisplayLayout::Timeline));
        assert_eq!(default.display.narrow.and_then(|set| set.view), Some(DisplayLayout::Timeline));

        assert_eq!(service.list(&owner, &[start("other", "/")]).await?, shelves, "a later read changes nothing");
        service.delete(&owner, &default.id).await?;
        assert!(service.list(&owner, &starts).await?.is_empty(), "a deleted default Shelf is not created again");
        Ok(())
    }

    #[tokio::test]
    async fn a_read_with_no_start_leaves_the_default_shelf_to_a_later_read_test() -> TestResult {
        let Fixture { _dir, service, owner } = fixture().await?;
        assert!(service.list(&owner, &[]).await?.is_empty());
        assert_eq!(names(&service.list(&owner, &[start("photos", "/")]).await?), ["Timeline"]);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn two_first_reads_at_once_create_one_default_shelf_test() -> TestResult {
        for _ in 0..10 {
            let Fixture { _dir, service, owner } = fixture().await?;
            let service = Arc::new(service);
            let reads = (0..4)
                .map(|_| {
                    let (service, owner) = (service.clone(), owner.clone());
                    tokio::spawn(async move { service.list(&owner, &[start("photos", "/")]).await })
                })
                .collect::<Vec<_>>();
            for read in reads {
                assert_eq!(names(&read.await??), ["Timeline"]);
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn shelves_are_created_last_replaced_whole_and_deleted_test() -> TestResult {
        let Fixture { _dir, service, .. } = fixture().await?;
        let mut first = input("  First  ", &[start("photos", "/a"), start("photos", "/a"), start("gone", "/missing")]);
        first.scope.recursive = true;
        first.filter = ShelfFilter {
            query: Some("sea".to_string()),
            rating: Some("good".to_string()),
            tags: Some(vec!["trip".to_string()]),
            kinds: Some(vec!["folder".to_string()]),
        };
        first.display = DisplayValues {
            narrow: Some(DisplayValueSet {
                view: Some(DisplayLayout::Timeline),
                tile_size: Some(5),
                thumbnail_shape: Some(ThumbnailShape::Fit),
            }),
            sort: Some(SortPreference {
                key: SortKey::Size,
                order: SortOrder::Desc,
            }),
            archive_open: Some(ArchiveOpen::Viewer),
            ..DisplayValues::default()
        };

        let created = service.create("u1", &first).await?;
        assert_eq!(created.name, "First", "the name is trimmed");
        assert_eq!((&created.scope, &created.filter, &created.display), (&first.scope, &first.filter, &first.display));
        let second = service.create("u1", &input("Second", &[start("photos", "/")])).await?;
        assert_eq!(service.list("u1", &[]).await?, vec![created.clone(), second.clone()]);

        let replaced = service.replace("u1", &created.id, &input("Renamed", &[start("videos", "/b")])).await?;
        assert_eq!(replaced.id, created.id);
        assert_eq!(replaced.filter, ShelfFilter::default(), "a replacement keeps nothing of the old value");
        assert_eq!(service.list("u1", &[]).await?, vec![replaced, second.clone()], "the position is kept");

        service.delete("u1", &created.id).await?;
        assert_eq!(service.list("u1", &[]).await?, vec![second]);
        let starts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM shelf_starts").fetch_one(&service.db).await?;
        assert_eq!(starts, 1, "a deleted Shelf's starts are gone");
        let third = service.create("u1", &input("Third", &[start("photos", "/")])).await?;
        assert_eq!(service.list("u1", &[]).await?.last(), Some(&third));
        Ok(())
    }

    #[tokio::test]
    async fn the_order_must_name_every_shelf_once_test() -> TestResult {
        let Fixture { _dir, service, .. } = fixture().await?;
        let mut ids = Vec::new();
        for name in ["a", "b", "c"] {
            ids.push(service.create("u1", &input(name, &[start("photos", "/")])).await?.id);
        }
        let other = service.create("u2", &input("other", &[start("photos", "/")])).await?.id;

        let reordered = service.reorder("u1", &[ids[2].clone(), ids[0].clone(), ids[1].clone()]).await?;
        assert_eq!(names(&reordered), ["c", "a", "b"]);
        assert_eq!(service.list("u1", &[]).await?, reordered);
        service.create("u1", &input("d", &[start("photos", "/")])).await?;
        assert_eq!(names(&service.list("u1", &[]).await?), ["c", "a", "b", "d"]);

        let missing = vec![ids[0].clone(), ids[1].clone(), ids[2].clone()];
        let twice = vec![ids[0].clone(), ids[0].clone(), ids[1].clone(), ids[2].clone()];
        let unknown = vec![ids[0].clone(), ids[1].clone(), ids[2].clone(), "shf_unknown".to_string()];
        let foreign = vec![ids[0].clone(), ids[1].clone(), ids[2].clone(), other];
        for order in [missing, twice, unknown, foreign, Vec::new()] {
            let error = service.reorder("u1", &order).await.unwrap_err();
            assert_eq!(*error.kind(), ErrorKind::InvalidInput, "{order:?}");
        }
        assert_eq!(names(&service.list("u1", &[]).await?), ["c", "a", "b", "d"], "a refused order changes nothing");
        assert!(service.reorder("u3", &[]).await?.is_empty(), "a user with no Shelf names none");
        Ok(())
    }

    #[tokio::test]
    async fn the_name_and_the_tile_size_are_limited_test() -> TestResult {
        let Fixture { _dir, service, .. } = fixture().await?;
        let starts = [start("photos", "/")];
        let longest = "あ".repeat(ShelfInput::MAX_NAME_LENGTH);
        let saved = service.create("u1", &input(&format!(" {longest} "), &starts)).await?;
        assert_eq!(saved.name, longest, "the length counts characters after trimming");

        let mut oversized = input("tile", &starts);
        oversized.display.wide = Some(DisplayValueSet {
            tile_size: Some(6),
            ..DisplayValueSet::default()
        });
        let mut undersized = oversized.clone();
        undersized.display = DisplayValues {
            narrow: Some(DisplayValueSet {
                tile_size: Some(0),
                ..DisplayValueSet::default()
            }),
            ..DisplayValues::default()
        };
        let long = "x".repeat(ShelfInput::MAX_NAME_LENGTH + 1);
        for invalid in [input("", &starts), input(" \t ", &starts), input(&long, &starts), oversized, undersized] {
            assert_eq!(*service.create("u1", &invalid).await.unwrap_err().kind(), ErrorKind::InvalidInput, "{invalid:?}");
            assert_eq!(
                *service.replace("u1", &saved.id, &invalid).await.unwrap_err().kind(),
                ErrorKind::InvalidInput,
                "{invalid:?}"
            );
        }
        assert_eq!(service.list("u1", &[]).await?, vec![saved]);
        Ok(())
    }

    #[tokio::test]
    async fn a_user_has_at_most_the_limit_of_shelves_test() -> TestResult {
        let Fixture { _dir, service, owner } = fixture().await?;
        let value = input("shelf", &[start("photos", "/")]);
        for _ in 0..ShelfService::MAX_SHELVES {
            service.create(&owner, &value).await?;
        }
        assert_eq!(*service.create(&owner, &value).await.unwrap_err().kind(), ErrorKind::InvalidInput);
        service.create("u2", &value).await?;

        let shelves = service.list(&owner, &[start("photos", "/")]).await?;
        assert_eq!(shelves.len(), ShelfService::MAX_SHELVES, "the default Shelf does not exceed the limit");
        service.delete(&owner, &shelves[0].id).await?;
        let shelves = service.list(&owner, &[start("photos", "/")]).await?;
        assert_eq!(shelves.last().map(|shelf| shelf.name.as_str()), Some("Timeline"), "it is created once there is room");
        Ok(())
    }

    #[tokio::test]
    async fn another_users_shelf_is_not_found_test() -> TestResult {
        let Fixture { _dir, service, .. } = fixture().await?;
        let value = input("mine", &[start("photos", "/")]);
        let mine = service.create("u1", &value).await?;
        assert!(service.list("u2", &[]).await?.is_empty());
        for (user, id) in [("u2", mine.id.as_str()), ("u1", "shf_unknown")] {
            assert_eq!(
                *service.replace(user, id, &input("theirs", &[start("videos", "/")])).await.unwrap_err().kind(),
                ErrorKind::NotFound
            );
            assert_eq!(*service.delete(user, id).await.unwrap_err().kind(), ErrorKind::NotFound);
        }
        assert_eq!(service.list("u1", &[]).await?, vec![mine]);
        Ok(())
    }
}
