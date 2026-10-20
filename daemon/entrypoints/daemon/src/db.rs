use std::path::Path;

use sqlx::sqlite::SqlitePool;

use crate::prelude::*;

/// `state_dir/sqlite.db` に接続する。テーブルは、それを使う engine の component が自分で migrate する。
pub struct DatabaseBootstrap;

impl DatabaseBootstrap {
    pub async fn connect(state_dir: impl AsRef<Path>) -> Result<SqlitePool> {
        let path = state_dir.as_ref().join("sqlite.db");
        let path = path.to_str().ok_or_else(|| Error::new(ErrorKind::UnexpectedError).with_message("Invalid path"))?;

        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(10));

        Ok(SqlitePool::connect_with(options).await?)
    }
}
