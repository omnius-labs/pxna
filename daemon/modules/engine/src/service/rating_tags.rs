use std::{
    collections::HashSet,
    sync::atomic::{AtomicU64, Ordering},
};

use sqlx::SqlitePool;
use tokio::sync::{RwLock, RwLockReadGuard};

use omnius_core_migration::sqlite::{MigrationRequest, SqliteMigrator};

use crate::{
    model::{Rating, TagName},
    prelude::*,
};

/// 利用者ごとの評価とタグを読み書きする。テーブルは生成時に自分で migrate する。
///
/// 対象の Entry は `(library_id, path)` で指す。Entry が存在するか、評価とタグの対象になる種類かは、呼び出し側が確かめる。
pub struct RatingTagService {
    db: SqlitePool,
    /// 評価とタグを書くたびに増える版。絞り込んだ一覧のページが、書き込みをまたいで食い違わないようにするために使う。再起動で 0 に戻る。
    version: AtomicU64,
    /// 評価とタグの書き込みは、データベースの更新と版の更新を、この錠を書き込みで持ったまま行う。
    /// 絞り込みの読み出しは、版と集合を、この錠を読み込みで持ったまま取る。これで、版と集合が同じ状態のものになる。
    consistency: RwLock<()>,
}

/// 評価とタグを、書き込みをはさまずに、版とあわせて読むための手段。持っている間、書き込みは待たされる。
pub struct RatingTagRead<'a> {
    service: &'a RatingTagService,
    _guard: RwLockReadGuard<'a, ()>,
}

impl RatingTagRead<'_> {
    /// この読み出しの間、変わらない版。
    pub fn version(&self) -> u64 {
        self.service.version()
    }

    pub async fn rated_paths(&self, user_id: &str, library_id: &str, rating: Rating) -> Result<HashSet<String>> {
        self.service.rated_paths(user_id, library_id, rating).await
    }

    pub async fn all_rated_paths(&self, user_id: &str, library_id: &str) -> Result<HashSet<String>> {
        self.service.all_rated_paths(user_id, library_id).await
    }

    pub async fn tagged_paths(&self, user_id: &str, library_id: &str, name: &str) -> Result<HashSet<String>> {
        self.service.tagged_paths(user_id, library_id, name).await
    }
}

impl RatingTagService {
    /// 1 つの Entry に付けられるタグの数の上限。
    pub const MAX_TAGS_PER_ENTRY: usize = 32;

    pub async fn new(db: SqlitePool) -> Result<Self> {
        let requests = vec![MigrationRequest {
            name: "2026-10-17_rating_tags".to_string(),
            queries: r#"
CREATE TABLE entry_ratings (
    user_id TEXT NOT NULL,
    library_id TEXT NOT NULL,
    path TEXT NOT NULL,
    rating TEXT NOT NULL CHECK (rating IN ('good', 'bad')),
    updated_at TEXT NOT NULL,
    PRIMARY KEY (user_id, library_id, path)
);

CREATE TABLE tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    UNIQUE (user_id, name)
);

CREATE TABLE entry_tags (
    user_id TEXT NOT NULL,
    library_id TEXT NOT NULL,
    path TEXT NOT NULL,
    tag_id INTEGER NOT NULL REFERENCES tags (id),
    PRIMARY KEY (user_id, library_id, path, tag_id)
);

CREATE INDEX entry_tags_tag ON entry_tags (tag_id);
"#
            .to_string(),
        }];
        SqliteMigrator::migrate(&db, requests).await?;
        Ok(Self {
            db,
            version: AtomicU64::new(0),
            consistency: RwLock::new(()),
        })
    }

    /// 版と集合を、書き込みにはさまれずにまとめて読む。返した値を持っている間、書き込みは待たされる。
    pub async fn read(&self) -> RatingTagRead<'_> {
        RatingTagRead {
            service: self,
            _guard: self.consistency.read().await,
        }
    }

    /// 評価とタグを最後に書き換えた版。
    pub fn version(&self) -> u64 {
        self.version.load(Ordering::SeqCst)
    }

    fn changed(&self) {
        self.version.fetch_add(1, Ordering::SeqCst);
    }

    /// 評価を返す。評価していなければ `None` を返す。
    pub async fn rating(&self, user_id: &str, library_id: &str, path: &str) -> Result<Option<Rating>> {
        let value = sqlx::query_scalar::<_, String>("SELECT rating FROM entry_ratings WHERE user_id = ? AND library_id = ? AND path = ?")
            .bind(user_id)
            .bind(library_id)
            .bind(path)
            .fetch_optional(&self.db)
            .await?;
        value.as_deref().map(Rating::parse).transpose()
    }

    /// 評価を置き換える。`None` は評価を外す。
    pub async fn set_rating(&self, user_id: &str, library_id: &str, path: &str, rating: Option<Rating>) -> Result<()> {
        let _write = self.consistency.write().await;
        match rating {
            Some(rating) => {
                sqlx::query(
                    "INSERT INTO entry_ratings (user_id, library_id, path, rating, updated_at) VALUES (?, ?, ?, ?, ?) \
                     ON CONFLICT(user_id, library_id, path) DO UPDATE SET rating = excluded.rating, updated_at = excluded.updated_at",
                )
                .bind(user_id)
                .bind(library_id)
                .bind(path)
                .bind(rating.as_str())
                .bind(chrono::Utc::now().to_rfc3339())
                .execute(&self.db)
                .await?;
            }
            None => {
                sqlx::query("DELETE FROM entry_ratings WHERE user_id = ? AND library_id = ? AND path = ?")
                    .bind(user_id)
                    .bind(library_id)
                    .bind(path)
                    .execute(&self.db)
                    .await?;
            }
        }
        self.changed();
        Ok(())
    }

    /// Entry に付いているタグを、語の順に返す。
    pub async fn entry_tags(&self, user_id: &str, library_id: &str, path: &str) -> Result<Vec<String>> {
        Ok(sqlx::query_scalar::<_, String>(
            "SELECT tags.name FROM entry_tags JOIN tags ON tags.id = entry_tags.tag_id \
             WHERE entry_tags.user_id = ? AND entry_tags.library_id = ? AND entry_tags.path = ? ORDER BY tags.name",
        )
        .bind(user_id)
        .bind(library_id)
        .bind(path)
        .fetch_all(&self.db)
        .await?)
    }

    /// Entry のタグを、与えた語の集合に置き換える。知らない語は作る。
    ///
    /// 語は整えて重複を除く。語が不正か多すぎるときは `InvalidInput` とし、保存済みのタグを変えない。置き換えた結果を、語の順に返す。
    pub async fn set_entry_tags(&self, user_id: &str, library_id: &str, path: &str, names: &[String]) -> Result<Vec<String>> {
        let mut tags = names.iter().map(|name| TagName::parse(name)).collect::<Result<Vec<_>>>()?;
        tags.sort();
        tags.dedup();
        if tags.len() > Self::MAX_TAGS_PER_ENTRY {
            return Err(Error::new(ErrorKind::InvalidInput).with_message(format!("an entry can have at most {} tags", Self::MAX_TAGS_PER_ENTRY)));
        }

        let _write = self.consistency.write().await;
        let mut transaction = self.db.begin().await?;
        sqlx::query("DELETE FROM entry_tags WHERE user_id = ? AND library_id = ? AND path = ?")
            .bind(user_id)
            .bind(library_id)
            .bind(path)
            .execute(&mut *transaction)
            .await?;
        for tag in &tags {
            sqlx::query("INSERT OR IGNORE INTO tags (user_id, name) VALUES (?, ?)")
                .bind(user_id)
                .bind(tag.as_str())
                .execute(&mut *transaction)
                .await?;
            sqlx::query(
                "INSERT INTO entry_tags (user_id, library_id, path, tag_id) \
                 SELECT ?, ?, ?, id FROM tags WHERE user_id = ? AND name = ?",
            )
            .bind(user_id)
            .bind(library_id)
            .bind(path)
            .bind(user_id)
            .bind(tag.as_str())
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        self.changed();
        Ok(tags.iter().map(|tag| tag.as_str().to_string()).collect())
    }

    /// 指す評価の Entry の path を、Library の中で集める。絞り込みの照合に使う。
    pub async fn rated_paths(&self, user_id: &str, library_id: &str, rating: Rating) -> Result<HashSet<String>> {
        Ok(
            sqlx::query_scalar::<_, String>("SELECT path FROM entry_ratings WHERE user_id = ? AND library_id = ? AND rating = ?")
                .bind(user_id)
                .bind(library_id)
                .bind(rating.as_str())
                .fetch_all(&self.db)
                .await?
                .into_iter()
                .collect(),
        )
    }

    /// 評価が付いている Entry の path を、good と bad のどちらでも、Library の中で集める。
    pub async fn all_rated_paths(&self, user_id: &str, library_id: &str) -> Result<HashSet<String>> {
        Ok(sqlx::query_scalar::<_, String>("SELECT path FROM entry_ratings WHERE user_id = ? AND library_id = ?")
            .bind(user_id)
            .bind(library_id)
            .fetch_all(&self.db)
            .await?
            .into_iter()
            .collect())
    }

    /// 語が付いている Entry の path を、Library の中で集める。語は整えてから照合し、使っていない語には空の集合を返す。
    pub async fn tagged_paths(&self, user_id: &str, library_id: &str, name: &str) -> Result<HashSet<String>> {
        let tag = TagName::parse(name)?;
        Ok(sqlx::query_scalar::<_, String>(
            "SELECT entry_tags.path FROM entry_tags JOIN tags ON tags.id = entry_tags.tag_id \
             WHERE entry_tags.user_id = ? AND entry_tags.library_id = ? AND tags.name = ?",
        )
        .bind(user_id)
        .bind(library_id)
        .bind(tag.as_str())
        .fetch_all(&self.db)
        .await?
        .into_iter()
        .collect())
    }

    /// Entry の移動に、評価とタグを追従させる。`from` の Entry と、それがフォルダか archive のときは配下の Entry の行を、`to` の下へ付け替える。
    ///
    /// すべての利用者の行が対象である。`to` に元からあった行は、移動した Entry に上書きされたものとして消す。
    pub async fn moved(&self, from_library: &str, from_path: &str, to_library: &str, to_path: &str) -> Result<()> {
        if from_library == to_library && from_path == to_path {
            return Ok(());
        }
        // 各文は、`path` が指定の path そのものか、その下（`path/` で始まる）の行を指す。名前が前方一致するだけの Entry は含めない。
        let _write = self.consistency.write().await;
        let mut transaction = self.db.begin().await?;
        sqlx::query("DELETE FROM entry_ratings WHERE library_id = ?1 AND (path = ?2 OR substr(path, 1, length(?2) + 1) = ?2 || '/')")
            .bind(to_library)
            .bind(to_path)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("DELETE FROM entry_tags WHERE library_id = ?1 AND (path = ?2 OR substr(path, 1, length(?2) + 1) = ?2 || '/')")
            .bind(to_library)
            .bind(to_path)
            .execute(&mut *transaction)
            .await?;
        sqlx::query(
            "UPDATE entry_ratings SET library_id = ?1, path = ?2 || substr(path, length(?4) + 1) \
             WHERE library_id = ?3 AND (path = ?4 OR substr(path, 1, length(?4) + 1) = ?4 || '/')",
        )
        .bind(to_library)
        .bind(to_path)
        .bind(from_library)
        .bind(from_path)
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            "UPDATE entry_tags SET library_id = ?1, path = ?2 || substr(path, length(?4) + 1) \
             WHERE library_id = ?3 AND (path = ?4 OR substr(path, 1, length(?4) + 1) = ?4 || '/')",
        )
        .bind(to_library)
        .bind(to_path)
        .bind(from_library)
        .bind(from_path)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        self.changed();
        Ok(())
    }

    /// 存在しなくなった Entry の評価とタグを消す。`path` がフォルダか archive のときは配下の行も消す。語彙は残す。
    pub async fn forget(&self, library_id: &str, path: &str) -> Result<()> {
        let _write = self.consistency.write().await;
        let mut transaction = self.db.begin().await?;
        sqlx::query("DELETE FROM entry_ratings WHERE library_id = ?1 AND (path = ?2 OR substr(path, 1, length(?2) + 1) = ?2 || '/')")
            .bind(library_id)
            .bind(path)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("DELETE FROM entry_tags WHERE library_id = ?1 AND (path = ?2 OR substr(path, 1, length(?2) + 1) = ?2 || '/')")
            .bind(library_id)
            .bind(path)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        self.changed();
        Ok(())
    }

    /// 利用者がこれまでに作った語を、語の順に返す。Entry から外した語も残る。
    pub async fn tags(&self, user_id: &str) -> Result<Vec<String>> {
        Ok(sqlx::query_scalar::<_, String>("SELECT name FROM tags WHERE user_id = ? ORDER BY name")
            .bind(user_id)
            .fetch_all(&self.db)
            .await?)
    }
}

#[cfg(test)]
mod tests {
    use testresult::TestResult;

    use super::*;

    async fn service(dir: &tempfile::TempDir) -> TestResult<RatingTagService> {
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(dir.path().join("sqlite.db"))
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(10));
        Ok(RatingTagService::new(SqlitePool::connect_with(options).await?).await?)
    }

    fn names(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[tokio::test]
    async fn a_rating_is_set_replaced_and_cleared_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let service = service(&dir).await?;
        assert_eq!(service.rating("u1", "photos", "/a.png").await?, None);
        service.set_rating("u1", "photos", "/a.png", Some(Rating::Good)).await?;
        assert_eq!(service.rating("u1", "photos", "/a.png").await?, Some(Rating::Good));
        service.set_rating("u1", "photos", "/a.png", Some(Rating::Bad)).await?;
        assert_eq!(service.rating("u1", "photos", "/a.png").await?, Some(Rating::Bad));
        service.set_rating("u1", "photos", "/a.png", None).await?;
        assert_eq!(service.rating("u1", "photos", "/a.png").await?, None);
        Ok(())
    }

    #[tokio::test]
    async fn ratings_are_kept_apart_by_user_and_library_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let service = service(&dir).await?;
        service.set_rating("u1", "photos", "/a.png", Some(Rating::Good)).await?;
        assert_eq!(service.rating("u2", "photos", "/a.png").await?, None);
        assert_eq!(service.rating("u1", "videos", "/a.png").await?, None);
        Ok(())
    }

    #[tokio::test]
    async fn tags_are_replaced_as_a_set_and_new_words_are_created_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let service = service(&dir).await?;
        let saved = service.set_entry_tags("u1", "photos", "/a.png", &names(&[" b ", "a", "b"])).await?;
        assert_eq!(saved, names(&["a", "b"]));
        assert_eq!(service.entry_tags("u1", "photos", "/a.png").await?, names(&["a", "b"]));

        service.set_entry_tags("u1", "photos", "/a.png", &names(&["c"])).await?;
        assert_eq!(service.entry_tags("u1", "photos", "/a.png").await?, names(&["c"]));
        assert_eq!(service.tags("u1").await?, names(&["a", "b", "c"]), "外した語も語彙に残る");

        service.set_entry_tags("u1", "photos", "/a.png", &[]).await?;
        assert!(service.entry_tags("u1", "photos", "/a.png").await?.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn words_are_shared_across_entries_and_kept_apart_by_user_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let service = service(&dir).await?;
        service.set_entry_tags("u1", "photos", "/a.png", &names(&["trip"])).await?;
        service.set_entry_tags("u1", "photos", "/b.png", &names(&["trip"])).await?;
        assert_eq!(service.tags("u1").await?, names(&["trip"]));
        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tags WHERE user_id = 'u1'").fetch_one(&service.db).await?;
        assert_eq!(rows, 1);
        assert!(service.tags("u2").await?.is_empty());
        assert!(service.entry_tags("u2", "photos", "/a.png").await?.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn an_invalid_set_leaves_the_saved_tags_unchanged_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let service = service(&dir).await?;
        service.set_entry_tags("u1", "photos", "/a.png", &names(&["keep"])).await?;
        assert_eq!(
            *service.set_entry_tags("u1", "photos", "/a.png", &names(&["ok", ""])).await.unwrap_err().kind(),
            ErrorKind::InvalidInput
        );
        let many = (0..=RatingTagService::MAX_TAGS_PER_ENTRY).map(|index| format!("t{index}")).collect::<Vec<_>>();
        assert_eq!(*service.set_entry_tags("u1", "photos", "/a.png", &many).await.unwrap_err().kind(), ErrorKind::InvalidInput);
        assert_eq!(service.entry_tags("u1", "photos", "/a.png").await?, names(&["keep"]));
        assert_eq!(service.tags("u1").await?, names(&["keep"]), "拒否した置き換えは語も作らない");
        Ok(())
    }

    #[tokio::test]
    async fn values_survive_a_new_service_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        {
            let first = service(&dir).await?;
            first.set_rating("u1", "photos", "/a.png", Some(Rating::Good)).await?;
            first.set_entry_tags("u1", "photos", "/a.png", &names(&["x"])).await?;
        }
        let reopened = service(&dir).await?;
        assert_eq!(reopened.rating("u1", "photos", "/a.png").await?, Some(Rating::Good));
        assert_eq!(reopened.entry_tags("u1", "photos", "/a.png").await?, names(&["x"]));
        Ok(())
    }

    #[tokio::test]
    async fn a_move_carries_ratings_and_tags_of_the_entry_and_its_descendants_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let service = service(&dir).await?;
        for user in ["u1", "u2"] {
            service.set_rating(user, "photos", "/a/x.png", Some(Rating::Good)).await?;
            service.set_entry_tags(user, "photos", "/a/x.png", &names(&["t"])).await?;
        }
        service.set_rating("u1", "photos", "/a.png", Some(Rating::Bad)).await?;
        service.set_rating("u1", "photos", "/ab/y.png", Some(Rating::Bad)).await?;

        service.moved("photos", "/a", "videos", "/b").await?;
        for user in ["u1", "u2"] {
            assert_eq!(service.rating(user, "videos", "/b/x.png").await?, Some(Rating::Good), "{user}");
            assert_eq!(service.entry_tags(user, "videos", "/b/x.png").await?, names(&["t"]));
            assert_eq!(service.rating(user, "photos", "/a/x.png").await?, None);
            assert!(service.entry_tags(user, "photos", "/a/x.png").await?.is_empty());
        }
        assert_eq!(
            service.rating("u1", "photos", "/a.png").await?,
            Some(Rating::Bad),
            "名前が前方一致するだけの Entry は動かさない"
        );
        assert_eq!(service.rating("u1", "photos", "/ab/y.png").await?, Some(Rating::Bad));

        service.moved("photos", "/a.png", "photos", "/c.png").await?;
        assert_eq!(service.rating("u1", "photos", "/c.png").await?, Some(Rating::Bad));
        assert_eq!(service.rating("u1", "photos", "/a.png").await?, None);
        Ok(())
    }

    #[tokio::test]
    async fn a_move_over_an_existing_entry_replaces_its_marks_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let service = service(&dir).await?;
        service.set_rating("u1", "photos", "/src.png", Some(Rating::Good)).await?;
        service.set_entry_tags("u1", "photos", "/src.png", &names(&["new"])).await?;
        service.set_rating("u1", "photos", "/dst.png", Some(Rating::Bad)).await?;
        service.set_entry_tags("u1", "photos", "/dst.png", &names(&["old"])).await?;

        service.moved("photos", "/src.png", "photos", "/dst.png").await?;
        assert_eq!(service.rating("u1", "photos", "/dst.png").await?, Some(Rating::Good));
        assert_eq!(service.entry_tags("u1", "photos", "/dst.png").await?, names(&["new"]));
        assert_eq!(service.tags("u1").await?, names(&["new", "old"]), "語彙は残る");
        Ok(())
    }

    #[tokio::test]
    async fn forgetting_removes_the_entry_and_its_descendants_only_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let service = service(&dir).await?;
        for path in ["/d", "/d/x.png", "/d.png", "/e/x.png"] {
            service.set_rating("u1", "photos", path, Some(Rating::Good)).await?;
            service.set_entry_tags("u1", "photos", path, &names(&["t"])).await?;
        }
        service.forget("photos", "/d").await?;
        for gone in ["/d", "/d/x.png"] {
            assert_eq!(service.rating("u1", "photos", gone).await?, None, "{gone}");
            assert!(service.entry_tags("u1", "photos", gone).await?.is_empty());
        }
        for kept in ["/d.png", "/e/x.png"] {
            assert_eq!(service.rating("u1", "photos", kept).await?, Some(Rating::Good), "{kept}");
        }
        assert_eq!(service.tags("u1").await?, names(&["t"]));
        Ok(())
    }

    #[tokio::test]
    async fn paths_are_collected_by_rating_and_by_tag_for_one_user_and_library_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let service = service(&dir).await?;
        service.set_rating("u1", "photos", "/a.png", Some(Rating::Good)).await?;
        service.set_rating("u1", "photos", "/b.png", Some(Rating::Bad)).await?;
        service.set_rating("u1", "videos", "/c.png", Some(Rating::Good)).await?;
        service.set_rating("u2", "photos", "/d.png", Some(Rating::Good)).await?;
        service.set_entry_tags("u1", "photos", "/a.png", &names(&["trip"])).await?;
        service.set_entry_tags("u2", "photos", "/d.png", &names(&["trip"])).await?;

        assert_eq!(service.rated_paths("u1", "photos", Rating::Good).await?, ["/a.png".to_string()].into());
        assert_eq!(service.all_rated_paths("u1", "photos").await?, ["/a.png".to_string(), "/b.png".to_string()].into());
        assert_eq!(service.tagged_paths("u1", "photos", " trip ").await?, ["/a.png".to_string()].into());
        assert!(service.tagged_paths("u1", "photos", "unused").await?.is_empty());
        assert_eq!(*service.tagged_paths("u1", "photos", "").await.unwrap_err().kind(), ErrorKind::InvalidInput);
        Ok(())
    }

    #[tokio::test]
    async fn the_version_advances_on_every_write_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let service = service(&dir).await?;
        let mut last = service.version();
        for step in 0..4 {
            match step {
                0 => service.set_rating("u1", "photos", "/a.png", Some(Rating::Good)).await?,
                1 => drop(service.set_entry_tags("u1", "photos", "/a.png", &names(&["t"])).await?),
                2 => service.moved("photos", "/a.png", "photos", "/b.png").await?,
                _ => service.forget("photos", "/b.png").await?,
            }
            assert!(service.version() > last, "step {step}");
            last = service.version();
        }
        Ok(())
    }

    #[tokio::test]
    async fn a_read_sees_one_state_while_a_write_waits_for_it_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let service = std::sync::Arc::new(service(&dir).await?);
        service.set_rating("u1", "photos", "/a.png", Some(Rating::Good)).await?;

        let read = service.read().await;
        let version = read.version();
        let before = read.rated_paths("u1", "photos", Rating::Good).await?;
        let writer = {
            let service = service.clone();
            tokio::spawn(async move { service.set_rating("u1", "photos", "/b.png", Some(Rating::Good)).await })
        };
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        assert!(!writer.is_finished(), "読み出しの間、書き込みは待たされる");
        assert_eq!(read.version(), version);
        assert_eq!(read.rated_paths("u1", "photos", Rating::Good).await?, before, "読み出しの間、集合は変わらない");
        drop(read);

        writer.await??;
        let read = service.read().await;
        assert!(read.version() > version);
        assert_eq!(read.rated_paths("u1", "photos", Rating::Good).await?.len(), 2);
        Ok(())
    }
}
