use sqlx::SqlitePool;

use omnius_core_migration::sqlite::{MigrationRequest, SqliteMigrator};

use crate::{model::Preferences, prelude::*};

/// 表示設定を読み書きする。テーブルは生成時に自分で migrate する。
///
/// オーナーは 1 人なので、表示設定は 1 行だけを持つ。
pub struct PreferencesService {
    db: SqlitePool,
}

impl PreferencesService {
    pub async fn new(db: SqlitePool) -> Result<Self> {
        let requests = vec![MigrationRequest {
            name: "2026-10-05_preferences".to_string(),
            queries: r#"
CREATE TABLE preferences (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    value TEXT NOT NULL
);
"#
            .to_string(),
        }];
        SqliteMigrator::migrate(&db, requests).await?;
        Ok(Self { db })
    }

    /// 保存した値を返す。保存したことがなければ既定値を返す。
    pub async fn get(&self) -> Result<Preferences> {
        let value = sqlx::query_scalar::<_, String>("SELECT value FROM preferences WHERE id = 1")
            .fetch_optional(&self.db)
            .await?;
        match value {
            None => Ok(Preferences::default()),
            Some(value) => serde_json::from_str(&value).map_err(|error| Error::from_error(error, ErrorKind::SerdeError).with_message("stored preferences are unreadable")),
        }
    }

    /// 全体を検査して置き換える。範囲外の値は `InvalidInput` とし、保存済みの値を変えない。
    pub async fn put(&self, preferences: &Preferences) -> Result<()> {
        preferences.validate()?;
        let value = serde_json::to_string(preferences).map_err(|error| Error::from_error(error, ErrorKind::SerdeError))?;
        sqlx::query("INSERT INTO preferences (id, value) VALUES (1, ?) ON CONFLICT(id) DO UPDATE SET value = excluded.value")
            .bind(value)
            .execute(&self.db)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use testresult::TestResult;

    use super::*;
    use crate::model::{ListView, SortKey, SortOrder};

    async fn pool(dir: &tempfile::TempDir) -> TestResult<SqlitePool> {
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(dir.path().join("sqlite.db"))
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(10));
        Ok(SqlitePool::connect_with(options).await?)
    }

    #[tokio::test]
    async fn returns_the_defaults_until_something_is_saved_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let service = PreferencesService::new(pool(&dir).await?).await?;
        assert_eq!(service.get().await?, Preferences::default());
        Ok(())
    }

    #[tokio::test]
    async fn keeps_a_saved_value_across_a_new_service_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let mut preferences = Preferences::default();
        preferences.narrow.tile_size = 5;
        preferences.narrow.view = ListView::List;
        preferences.sort.key = SortKey::ModifiedAt;
        preferences.sort.order = SortOrder::Desc;

        PreferencesService::new(pool(&dir).await?).await?.put(&preferences).await?;
        let reopened = PreferencesService::new(pool(&dir).await?).await?;
        assert_eq!(reopened.get().await?, preferences);

        let mut replaced = preferences;
        replaced.wide.tile_size = 1;
        reopened.put(&replaced).await?;
        assert_eq!(reopened.get().await?, replaced, "保存は全体の置き換え");
        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM preferences").fetch_one(&pool(&dir).await?).await?;
        assert_eq!(rows, 1);
        Ok(())
    }

    #[tokio::test]
    async fn an_out_of_range_value_leaves_the_saved_value_unchanged_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let service = PreferencesService::new(pool(&dir).await?).await?;
        let mut saved = Preferences::default();
        saved.wide.tile_size = 4;
        service.put(&saved).await?;

        let mut invalid = saved;
        invalid.narrow.tile_size = 6;
        assert_eq!(*service.put(&invalid).await.unwrap_err().kind(), ErrorKind::InvalidInput);
        assert_eq!(service.get().await?, saved);
        Ok(())
    }
}
