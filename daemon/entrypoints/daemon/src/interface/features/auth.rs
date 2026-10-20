use async_trait::async_trait;
use axum_extra::extract::CookieJar;
use headers::Host;
use http::Method;

use omnius_pxna_engine::model::{IssuedSession, Session, SessionTransport, User, UserRole};
use omnius_pxna_interface::{apis, models, types::Nullable};

use crate::{
    error::{ApiError, ApiStatus},
    interface::{
        ApiServer,
        session::{Claims, SessionCookie},
    },
    prelude::*,
};

#[allow(unused_variables)]
#[async_trait]
impl apis::auth::Auth for ApiServer {
    type Claims = Claims;

    async fn delete_current_session(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
    ) -> std::result::Result<apis::auth::DeleteCurrentSessionResponse, ()> {
        Ok(match self.state.auth.revoke(&claims.session.id).await.map_err(Error::from) {
            // cookie のセッションには、ブラウザに cookie を消させる失効済みの cookie を返す。
            Ok(()) => apis::auth::DeleteCurrentSessionResponse::Status204_SessionRevoked {
                set_cookie: (claims.session.transport == SessionTransport::Cookie).then(|| SessionCookie::from_config(&self.state.conf.web).clear()),
            },
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn get_current_user(&self, method: &Method, host: &Host, cookies: &CookieJar, claims: &Self::Claims) -> std::result::Result<apis::auth::GetCurrentUserResponse, ()> {
        Ok(apis::auth::GetCurrentUserResponse::Status200_CurrentAuthenticatedUserAndSession(AuthConverter::current(
            claims,
        )))
    }

    async fn login(&self, method: &Method, host: &Host, cookies: &CookieJar, body: &models::LoginRequest) -> std::result::Result<apis::auth::LoginResponse, ()> {
        let transport = AuthConverter::transport(body.session_transport);
        Ok(match self.state.auth.login(&body.username, &body.password, transport).await.map_err(Error::from) {
            Ok(issued) => {
                let (body, set_cookie) = AuthConverter::issued(&issued, SessionCookie::from_config(&self.state.conf.web));
                apis::auth::LoginResponse::Status200_LoginSucceeded { body, set_cookie }
            }
            Err(error) => ApiError::from(error).into(),
        })
    }
}

impl From<ApiError> for apis::auth::LoginResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::auth::DeleteCurrentSessionResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

/// engine のユーザとセッションを、API の型へ写す。
pub struct AuthConverter;

impl AuthConverter {
    pub fn transport(transport: models::SessionTransport) -> SessionTransport {
        match transport {
            models::SessionTransport::Cookie => SessionTransport::Cookie,
            models::SessionTransport::Bearer => SessionTransport::Bearer,
        }
    }

    /// `cookie` のセッションは token を `Set-Cookie` で返し、`bearer` のセッションは本文の `accessToken` で返す。
    pub fn issued(issued: &IssuedSession, cookie: SessionCookie) -> (models::AuthResult, Option<String>) {
        let mut result = models::AuthResult::new(Self::user(&issued.user), Self::session(&issued.session));
        match issued.session.transport {
            SessionTransport::Cookie => (result, Some(cookie.issue(&issued.token))),
            SessionTransport::Bearer => {
                result.access_token = Some(issued.token.clone());
                (result, None)
            }
        }
    }

    pub fn current(claims: &Claims) -> models::CurrentAuth {
        models::CurrentAuth::new(Self::user(&claims.user), Self::session(&claims.session))
    }

    fn user(user: &User) -> models::User {
        let role = match user.role {
            UserRole::Owner => models::UserRole::Owner,
        };
        models::User::new(user.id.clone(), user.username.clone(), role, user.created_at, user.updated_at)
    }

    /// 照合を通ったセッションだけを返すので、状態は常に `active` である。
    fn session(session: &Session) -> models::AuthSession {
        let transport = match session.transport {
            SessionTransport::Cookie => models::SessionTransport::Cookie,
            SessionTransport::Bearer => models::SessionTransport::Bearer,
        };
        let mut converted = models::AuthSession::new(
            session.id.clone(),
            session.user_id.clone(),
            models::SessionStatus::Active,
            transport,
            session.created_at,
            session.expires_at,
        );
        converted.last_seen_at = Some(session.last_seen_at.map(Nullable::Present).unwrap_or(Nullable::Null));
        converted
    }
}

#[cfg(test)]
mod tests {
    use http::StatusCode;
    use omnius_pxna_engine::service::ArchiveOptions;
    use testresult::TestResult;

    use super::*;
    use crate::interface::server::tests::*;

    #[tokio::test]
    async fn login_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let router = router(&fixture);
        send(&router, Method::POST, "/api/v1/setup", &[], credentials("owner", "password1", "cookie")).await?;

        let bearer_login = send(&router, Method::POST, "/api/v1/auth/login", &[], credentials("owner", "password1", "bearer")).await?;
        assert_eq!(bearer_login.status, StatusCode::OK);
        assert!(bearer_login.body["accessToken"].is_string());
        assert!(bearer_login.set_cookie().is_none());

        let cookie_login = send(&router, Method::POST, "/api/v1/auth/login", &[], credentials("owner", "password1", "cookie")).await?;
        assert!(cookie_login.set_cookie().is_some_and(|value| value.contains("HttpOnly; Secure; SameSite=Strict")));

        let wrong_password = send(&router, Method::POST, "/api/v1/auth/login", &[], credentials("owner", "password2", "cookie")).await?;
        let unknown_user = send(&router, Method::POST, "/api/v1/auth/login", &[], credentials("nobody", "password1", "cookie")).await?;
        assert_eq!(wrong_password.status, StatusCode::UNAUTHORIZED);
        assert_eq!(unknown_user.status, StatusCode::UNAUTHORIZED);
        assert_eq!(wrong_password.body, unknown_user.body, "ユーザ名の有無を本文から区別できない");
        Ok(())
    }

    #[tokio::test]
    async fn current_user_and_logout_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let router = router(&fixture);
        let setup = send(&router, Method::POST, "/api/v1/setup", &[], credentials("owner", "password1", "cookie")).await?;
        let cookie = cookie(&setup)?;

        let me = send(&router, Method::GET, "/api/v1/auth/me", &[("cookie", &cookie)], None).await?;
        assert_eq!(me.status, StatusCode::OK);
        assert_eq!(me.body["user"]["username"], "owner");
        assert_eq!(me.body["user"]["role"], "owner");
        assert_eq!(me.body["session"]["transport"], "cookie");
        assert_eq!(me.body["session"]["status"], "active");

        let logout = send(
            &router,
            Method::DELETE,
            "/api/v1/auth/session",
            &[("cookie", &cookie), ("origin", "http://localhost")],
            None,
        )
        .await?;
        assert_eq!(logout.status, StatusCode::NO_CONTENT);
        assert!(logout.set_cookie().is_some_and(|value| value.ends_with("Max-Age=0")), "ブラウザに cookie を消させる");
        assert_eq!(
            send(&router, Method::GET, "/api/v1/auth/me", &[("cookie", &cookie)], None).await?.status,
            StatusCode::UNAUTHORIZED
        );

        let login = send(&router, Method::POST, "/api/v1/auth/login", &[], credentials("owner", "password1", "bearer")).await?;
        let bearer = bearer(&login)?;
        let logout = send(&router, Method::DELETE, "/api/v1/auth/session", &[("authorization", &bearer)], None).await?;
        assert_eq!(logout.status, StatusCode::NO_CONTENT);
        assert!(logout.set_cookie().is_none(), "bearer のセッションには cookie を返さない");
        assert_eq!(
            send(&router, Method::GET, "/api/v1/auth/me", &[("authorization", &bearer)], None).await?.status,
            StatusCode::UNAUTHORIZED
        );
        Ok(())
    }
}
