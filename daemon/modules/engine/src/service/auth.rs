use std::sync::Arc;

use chrono::{Duration, Utc};
use sqlx::SqlitePool;

use omnius_core_base::{clock::Clock, tsid::TsidProvider};

use crate::prelude::*;

mod password;
mod repo;
mod token;

use password::PasswordDigest;
use repo::AuthRepo;
use token::SessionToken;

const USERNAME_MAX_LENGTH: usize = 128;
const PASSWORD_MIN_LENGTH: usize = 8;
const PASSWORD_MAX_LENGTH: usize = 256;

/// 初期セットアップ、ログイン、セッションの照合と失効を受け持つ。
pub struct AuthService {
    repo: AuthRepo,
    clock: Arc<dyn Clock<Utc> + Send + Sync>,
    tsid_provider: Arc<Mutex<dyn TsidProvider + Send + Sync>>,
    /// 存在しないユーザ名でもパスワードを照合し、応答時間からユーザ名の有無を推測されにくくする。
    dummy_password_hash: String,
}

impl AuthService {
    pub async fn new(db: SqlitePool, clock: Arc<dyn Clock<Utc> + Send + Sync>, tsid_provider: Arc<Mutex<dyn TsidProvider + Send + Sync>>) -> Result<Self> {
        let repo = AuthRepo::new(db).await?;
        let dummy_password_hash = PasswordDigest::hash(&SessionToken::generate()?).await?;
        Ok(Self {
            repo,
            clock,
            tsid_provider,
            dummy_password_hash,
        })
    }

    /// セッションの有効期間。作成から数え、延長しない。
    pub fn session_lifetime() -> Duration {
        Duration::days(30)
    }

    pub async fn is_initialized(&self) -> Result<bool> {
        self.repo.has_users().await
    }

    /// ユーザがいないときだけオーナーを登録し、そのセッションを作る。すでにいれば `AlreadyExists` になる。
    pub async fn create_owner(&self, username: &str, password: &str, transport: SessionTransport) -> Result<IssuedSession> {
        Self::validate_username(username)?;
        Self::validate_password(password)?;

        let now = self.clock.now();
        let user = User {
            id: self.new_id("usr"),
            username: username.to_string(),
            role: UserRole::Owner,
            created_at: now,
            updated_at: now,
        };
        let password_hash = PasswordDigest::hash(password).await?;
        let (session, token) = self.new_session(&user, transport)?;

        if !self.repo.insert_owner(&user, &password_hash, &session, &SessionToken::hash(&token)).await? {
            return Err(Error::new(ErrorKind::AlreadyExists).with_message("the owner is already registered"));
        }

        Ok(IssuedSession { user, session, token })
    }

    /// ユーザ名が存在しない場合もパスワードが違う場合も、同じ `Unauthenticated` を返す。
    pub async fn login(&self, username: &str, password: &str, transport: SessionTransport) -> Result<IssuedSession> {
        let credentials = self.repo.find_credentials(username).await?;
        let hash = credentials.as_ref().map_or(self.dummy_password_hash.as_str(), |(_, hash)| hash.as_str());
        let verified = PasswordDigest::verify(password, hash).await?;

        let Some((user, _)) = credentials.filter(|_| verified) else {
            return Err(Error::new(ErrorKind::Unauthenticated).with_message("invalid username or password"));
        };

        let (session, token) = self.new_session(&user, transport)?;
        self.repo.insert_session(&session, &SessionToken::hash(&token)).await?;
        Ok(IssuedSession { user, session, token })
    }

    /// 失効させたセッションと期限を過ぎたセッションは、存在しないセッションと区別せずに `None` を返す。
    pub async fn authenticate(&self, token: &str) -> Result<Option<AuthenticatedSession>> {
        let Some(stored) = self.repo.find_session(&SessionToken::hash(token)).await? else {
            return Ok(None);
        };

        let now = self.clock.now();
        if stored.revoked_at.is_some() || now >= stored.session.expires_at {
            return Ok(None);
        }

        // 要求のたびに書き込むと、一覧のサムネイルのように要求が集中する場面で書き込みが要求の数だけ増える。
        let mut session = stored.session;
        if session.last_seen_at.is_none_or(|last_seen_at| now - last_seen_at >= Duration::minutes(5)) {
            self.repo.touch_session(&session.id, now).await?;
            session.last_seen_at = Some(now);
        }

        Ok(Some(AuthenticatedSession { user: stored.user, session }))
    }

    pub async fn revoke(&self, session_id: &str) -> Result<()> {
        self.repo.revoke_session(session_id, self.clock.now()).await
    }

    fn new_session(&self, user: &User, transport: SessionTransport) -> Result<(Session, String)> {
        let now = self.clock.now();
        let session = Session {
            id: self.new_id("ses"),
            user_id: user.id.clone(),
            transport,
            created_at: now,
            expires_at: now + Self::session_lifetime(),
            last_seen_at: None,
        };
        Ok((session, SessionToken::generate()?))
    }

    fn new_id(&self, prefix: &str) -> String {
        format!("{prefix}_{}", self.tsid_provider.lock().create())
    }

    fn validate_username(username: &str) -> Result<()> {
        let valid = !username.is_empty() && username.len() <= USERNAME_MAX_LENGTH && username.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
        if !valid {
            return Err(Error::new(ErrorKind::InvalidInput).with_message(format!("username must be 1-{USERNAME_MAX_LENGTH} characters of [A-Za-z0-9._-]")));
        }
        Ok(())
    }

    fn validate_password(password: &str) -> Result<()> {
        let length = password.chars().count();
        if !(PASSWORD_MIN_LENGTH..=PASSWORD_MAX_LENGTH).contains(&length) {
            return Err(Error::new(ErrorKind::InvalidInput).with_message(format!("password must be {PASSWORD_MIN_LENGTH}-{PASSWORD_MAX_LENGTH} characters")));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone as _;
    use omnius_core_base::{
        clock::{ClockUtc, FakeClockUtc},
        tsid::TsidProviderImpl,
    };
    use rand::{SeedableRng as _, rngs::ChaCha20Rng};
    use testresult::TestResult;

    use super::*;

    struct Fixture {
        _dir: tempfile::TempDir,
        db: SqlitePool,
        clock: Arc<FakeClockUtc>,
        service: AuthService,
    }

    async fn fixture() -> TestResult<Fixture> {
        let dir = tempfile::tempdir()?;
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(dir.path().join("sqlite.db"))
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(10));
        let db = SqlitePool::connect_with(options).await?;
        let clock = Arc::new(FakeClockUtc::new(Utc.with_ymd_and_hms(2026, 9, 30, 0, 0, 0).single().ok_or("invalid time")?));
        let tsid_provider = Arc::new(Mutex::new(TsidProviderImpl::new(ClockUtc, ChaCha20Rng::seed_from_u64(0), 16)));
        let service = AuthService::new(db.clone(), clock.clone(), tsid_provider).await?;
        Ok(Fixture { _dir: dir, db, clock, service })
    }

    fn days(count: u64) -> std::time::Duration {
        std::time::Duration::from_secs(count * 24 * 60 * 60)
    }

    #[tokio::test]
    async fn creates_tables_test() -> TestResult {
        let fixture = fixture().await?;
        let tables: Vec<String> = sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type = 'table' AND name IN ('users', 'sessions') ORDER BY name")
            .fetch_all(&fixture.db)
            .await?;
        assert_eq!(tables, ["sessions", "users"]);
        Ok(())
    }

    #[tokio::test]
    async fn registers_only_one_owner_test() -> TestResult {
        let fixture = fixture().await?;
        assert!(!fixture.service.is_initialized().await?);

        let issued = fixture.service.create_owner("owner", "password1", SessionTransport::Cookie).await?;
        assert_eq!(issued.user.role, UserRole::Owner);
        assert!(issued.user.id.starts_with("usr_"));
        assert!(issued.session.id.starts_with("ses_"));
        assert!(fixture.service.is_initialized().await?);

        let error = fixture.service.create_owner("other", "password2", SessionTransport::Cookie).await.unwrap_err();
        assert_eq!(*error.kind(), ErrorKind::AlreadyExists);
        Ok(())
    }

    #[tokio::test]
    async fn concurrent_registrations_create_one_owner_test() -> TestResult {
        let fixture = fixture().await?;
        let (a, b) = tokio::join!(
            fixture.service.create_owner("alice", "password1", SessionTransport::Cookie),
            fixture.service.create_owner("bob", "password2", SessionTransport::Bearer),
        );
        let results = [a, b];
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert!(results.iter().any(|result| result.as_ref().is_err_and(|error| *error.kind() == ErrorKind::AlreadyExists)));

        let owners: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(&fixture.db).await?;
        assert_eq!(owners, 1);
        Ok(())
    }

    #[tokio::test]
    async fn rejects_invalid_credentials_alike_test() -> TestResult {
        let fixture = fixture().await?;
        fixture.service.create_owner("owner", "password1", SessionTransport::Cookie).await?;

        let wrong_password = fixture.service.login("owner", "password2", SessionTransport::Cookie).await.unwrap_err();
        let unknown_user = fixture.service.login("nobody", "password1", SessionTransport::Cookie).await.unwrap_err();
        assert_eq!(*wrong_password.kind(), ErrorKind::Unauthenticated);
        assert_eq!(*unknown_user.kind(), ErrorKind::Unauthenticated);
        assert_eq!(wrong_password.message(), unknown_user.message());

        let issued = fixture.service.login("owner", "password1", SessionTransport::Bearer).await?;
        assert_eq!(issued.session.transport, SessionTransport::Bearer);
        Ok(())
    }

    #[tokio::test]
    async fn rejects_invalid_registration_input_test() -> TestResult {
        let fixture = fixture().await?;
        for (username, password) in [("", "password1"), ("a/b", "password1"), ("owner", "short")] {
            let error = fixture.service.create_owner(username, password, SessionTransport::Cookie).await.unwrap_err();
            assert_eq!(*error.kind(), ErrorKind::InvalidInput, "{username:?} {password:?}");
        }
        Ok(())
    }

    #[tokio::test]
    async fn revoked_and_expired_sessions_are_not_found_test() -> TestResult {
        let fixture = fixture().await?;
        let owner = fixture.service.create_owner("owner", "password1", SessionTransport::Cookie).await?;
        assert!(fixture.service.authenticate(&owner.token).await?.is_some());
        assert!(fixture.service.authenticate("not a token").await?.is_none());

        fixture.service.revoke(&owner.session.id).await?;
        assert!(fixture.service.authenticate(&owner.token).await?.is_none());

        let login = fixture.service.login("owner", "password1", SessionTransport::Bearer).await?;
        fixture.clock.advance(days(29));
        assert!(fixture.service.authenticate(&login.token).await?.is_some());
        fixture.clock.advance(days(1));
        assert!(fixture.service.authenticate(&login.token).await?.is_none(), "作成から 30 日で失効する");
        Ok(())
    }

    #[tokio::test]
    async fn throttles_last_seen_updates_test() -> TestResult {
        let fixture = fixture().await?;
        let owner = fixture.service.create_owner("owner", "password1", SessionTransport::Cookie).await?;

        let first = fixture.service.authenticate(&owner.token).await?.ok_or("session")?.session.last_seen_at;
        fixture.clock.advance(std::time::Duration::from_secs(60));
        let second = fixture.service.authenticate(&owner.token).await?.ok_or("session")?.session.last_seen_at;
        assert_eq!(first, second, "5 分以内は書き込まない");

        fixture.clock.advance(std::time::Duration::from_secs(5 * 60));
        let third = fixture.service.authenticate(&owner.token).await?.ok_or("session")?.session.last_seen_at;
        assert_ne!(first, third);
        Ok(())
    }

    #[tokio::test]
    async fn stores_no_plaintext_secrets_test() -> TestResult {
        let fixture = fixture().await?;
        let owner = fixture.service.create_owner("owner", "password1", SessionTransport::Cookie).await?;

        let password_hash: String = sqlx::query_scalar("SELECT password_hash FROM users").fetch_one(&fixture.db).await?;
        assert!(password_hash.starts_with("$argon2id$"));
        assert!(!password_hash.contains("password1"));

        let token_hash: Vec<u8> = sqlx::query_scalar("SELECT token_hash FROM sessions").fetch_one(&fixture.db).await?;
        assert_ne!(token_hash, owner.token.as_bytes());
        assert_eq!(token_hash, SessionToken::hash(&owner.token));
        Ok(())
    }
}
