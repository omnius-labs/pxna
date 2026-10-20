use chrono::{DateTime, Utc};
use sqlx::{SqliteExecutor, SqlitePool};

use omnius_core_migration::sqlite::{MigrationRequest, SqliteMigrator};

use crate::prelude::*;

/// `users` と `sessions` を読み書きする。テーブルは生成時に自分で migrate する。
pub struct AuthRepo {
    db: SqlitePool,
}

/// 失効させた時刻を含む、保存されたままのセッション。
pub struct StoredSession {
    pub user: User,
    pub session: Session,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(sqlx::FromRow)]
struct CredentialsRecord {
    id: String,
    username: String,
    role: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    password_hash: String,
}

#[derive(sqlx::FromRow)]
struct SessionRecord {
    user_id: String,
    username: String,
    role: String,
    user_created_at: DateTime<Utc>,
    user_updated_at: DateTime<Utc>,
    session_id: String,
    transport: String,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    last_seen_at: Option<DateTime<Utc>>,
    revoked_at: Option<DateTime<Utc>>,
}

impl AuthRepo {
    pub async fn new(db: SqlitePool) -> Result<Self> {
        Self::migrate(&db).await?;
        Ok(Self { db })
    }

    async fn migrate(db: &SqlitePool) -> Result<()> {
        let requests = vec![MigrationRequest {
            name: "2026-09-30_users_and_sessions".to_string(),
            queries: r#"
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('owner')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
-- オーナーは 1 人に限る。登録の競合を取り逃しても、2 人目は制約で弾かれる。
CREATE UNIQUE INDEX index_users_single_owner ON users(role) WHERE role = 'owner';

CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    token_hash BLOB NOT NULL UNIQUE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    transport TEXT NOT NULL CHECK (transport IN ('cookie', 'bearer')),
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    last_seen_at TEXT,
    revoked_at TEXT
);
CREATE INDEX index_sessions_user_id ON sessions(user_id);
"#
            .to_string(),
        }];

        SqliteMigrator::migrate(db, requests).await?;
        Ok(())
    }

    pub async fn has_users(&self) -> Result<bool> {
        Ok(sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM users)").fetch_one(&self.db).await?)
    }

    /// ユーザがいないときだけ、オーナーとそのセッションを 1 つのトランザクションで作る。作れなければ false を返す。
    pub async fn insert_owner(&self, user: &User, password_hash: &str, session: &Session, token_hash: &[u8]) -> Result<bool> {
        // 最初に書き込みの lock を取り、同時の登録が順に「ユーザがいないか」を評価するようにする。
        let mut tx = self.db.begin_with("BEGIN IMMEDIATE").await?;
        let inserted =
            sqlx::query("INSERT INTO users (id, username, password_hash, role, created_at, updated_at) SELECT ?1, ?2, ?3, ?4, ?5, ?6 WHERE NOT EXISTS (SELECT 1 FROM users)")
                .bind(&user.id)
                .bind(&user.username)
                .bind(password_hash)
                .bind(user.role.as_str())
                .bind(user.created_at)
                .bind(user.updated_at)
                .execute(&mut *tx)
                .await?
                .rows_affected();

        if inserted == 0 {
            tx.rollback().await?;
            return Ok(false);
        }

        Self::insert_session_with(&mut *tx, session, token_hash).await?;
        tx.commit().await?;
        Ok(true)
    }

    /// ユーザと、照合に使うパスワードのハッシュを返す。
    pub async fn find_credentials(&self, username: &str) -> Result<Option<(User, String)>> {
        let record: Option<CredentialsRecord> = sqlx::query_as("SELECT id, username, role, created_at, updated_at, password_hash FROM users WHERE username = ?1")
            .bind(username)
            .fetch_optional(&self.db)
            .await?;

        record
            .map(|record| {
                let user = User {
                    id: record.id,
                    username: record.username,
                    role: UserRole::parse(&record.role)?,
                    created_at: record.created_at,
                    updated_at: record.updated_at,
                };
                Ok((user, record.password_hash))
            })
            .transpose()
    }

    pub async fn insert_session(&self, session: &Session, token_hash: &[u8]) -> Result<()> {
        Self::insert_session_with(&self.db, session, token_hash).await
    }

    async fn insert_session_with<'e>(executor: impl SqliteExecutor<'e>, session: &Session, token_hash: &[u8]) -> Result<()> {
        sqlx::query("INSERT INTO sessions (id, token_hash, user_id, transport, created_at, expires_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)")
            .bind(&session.id)
            .bind(token_hash)
            .bind(&session.user_id)
            .bind(session.transport.as_str())
            .bind(session.created_at)
            .bind(session.expires_at)
            .execute(executor)
            .await?;
        Ok(())
    }

    pub async fn find_session(&self, token_hash: &[u8]) -> Result<Option<StoredSession>> {
        let record: Option<SessionRecord> = sqlx::query_as(
            "SELECT u.id AS user_id, u.username, u.role, u.created_at AS user_created_at, u.updated_at AS user_updated_at, \
             s.id AS session_id, s.transport, s.created_at, s.expires_at, s.last_seen_at, s.revoked_at \
             FROM sessions s JOIN users u ON u.id = s.user_id WHERE s.token_hash = ?1",
        )
        .bind(token_hash)
        .fetch_optional(&self.db)
        .await?;

        record
            .map(|record| {
                Ok(StoredSession {
                    session: Session {
                        id: record.session_id,
                        user_id: record.user_id.clone(),
                        transport: SessionTransport::parse(&record.transport)?,
                        created_at: record.created_at,
                        expires_at: record.expires_at,
                        last_seen_at: record.last_seen_at,
                    },
                    user: User {
                        id: record.user_id,
                        username: record.username,
                        role: UserRole::parse(&record.role)?,
                        created_at: record.user_created_at,
                        updated_at: record.user_updated_at,
                    },
                    revoked_at: record.revoked_at,
                })
            })
            .transpose()
    }

    pub async fn touch_session(&self, session_id: &str, now: DateTime<Utc>) -> Result<()> {
        sqlx::query("UPDATE sessions SET last_seen_at = ?2 WHERE id = ?1")
            .bind(session_id)
            .bind(now)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    /// 失効済みのセッションは、最初に失効させた時刻を保つ。
    pub async fn revoke_session(&self, session_id: &str, now: DateTime<Utc>) -> Result<()> {
        sqlx::query("UPDATE sessions SET revoked_at = ?2 WHERE id = ?1 AND revoked_at IS NULL")
            .bind(session_id)
            .bind(now)
            .execute(&self.db)
            .await?;
        Ok(())
    }
}
