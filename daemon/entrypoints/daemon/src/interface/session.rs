use axum_extra::extract::CookieJar;
use http::{HeaderMap, header::AUTHORIZATION};

use omnius_pxna_engine::{
    model::{AuthenticatedSession, Session, User},
    service::AuthService,
};

use crate::{config::WebConfig, prelude::*, state::DaemonState};

/// ブラウザがセッションの token を送る cookie の名前。`__Host-` 接頭辞により、`Secure` と `Path=/` が必須になる。
pub const SESSION_COOKIE_NAME: &str = "__Host-pxna_session";

/// `[web] insecure_cookie` を有効にしたときの cookie の名前。`__Host-` は `Secure` を要するので外す。
pub const INSECURE_SESSION_COOKIE_NAME: &str = "pxna_session";

/// 認証を要する経路のハンドラへ渡す、照合済みのユーザとセッション。
#[derive(Debug, Clone)]
pub struct Claims {
    pub user: User,
    pub session: Session,
}

impl From<AuthenticatedSession> for Claims {
    fn from(authenticated: AuthenticatedSession) -> Self {
        Self {
            user: authenticated.user,
            session: authenticated.session,
        }
    }
}

/// セッションの cookie の名前と `Set-Cookie` の値を組み立てる。
#[derive(Debug, Clone, Copy)]
pub struct SessionCookie {
    insecure: bool,
}

impl SessionCookie {
    pub fn new(insecure: bool) -> Self {
        Self { insecure }
    }

    pub fn from_config(web: &WebConfig) -> Self {
        Self::new(web.insecure_cookie)
    }

    pub fn name(&self) -> &'static str {
        if self.insecure { INSECURE_SESSION_COOKIE_NAME } else { SESSION_COOKIE_NAME }
    }

    pub fn issue(&self, token: &str) -> String {
        let max_age = AuthService::session_lifetime().num_seconds();
        format!("{}={token}; {}; Max-Age={max_age}", self.name(), self.attributes())
    }

    pub fn clear(&self) -> String {
        format!("{}=; {}; Max-Age=0", self.name(), self.attributes())
    }

    fn attributes(&self) -> &'static str {
        if self.insecure {
            "Path=/; HttpOnly; SameSite=Strict"
        } else {
            "Path=/; HttpOnly; Secure; SameSite=Strict"
        }
    }
}

/// 生成されたルータの外の route が、セッションを照合するために使う。
///
/// 生成されたルータと同じく、cookie を先に見て、なければ `Authorization: Bearer` を見る。
pub struct SessionAuth;

impl SessionAuth {
    pub async fn authenticate(state: &DaemonState, cookies: &CookieJar, headers: &HeaderMap) -> Option<Claims> {
        Self::authenticate_with_token(state, cookies, headers).await.map(|(_, claims)| claims)
    }

    /// `authenticate` と同じ照合をして、通った token も返す。接続が続く間、同じセッションがまだ有効かを確かめ直すために使う。
    pub async fn authenticate_with_token(state: &DaemonState, cookies: &CookieJar, headers: &HeaderMap) -> Option<(String, Claims)> {
        let tokens = cookies
            .get(SessionCookie::from_config(&state.conf.web).name())
            .map(|cookie| cookie.value())
            .into_iter()
            .chain(BearerToken::parse(headers));
        for token in tokens {
            if let Some(claims) = Self::authenticate_token(state, token).await {
                return Some((token.to_string(), claims));
            }
        }
        None
    }

    /// 照合に失敗したときは、呼び出し側が 401 を返せるよう `None` にし、原因は log にだけ残す。
    pub async fn authenticate_token(state: &DaemonState, token: &str) -> Option<Claims> {
        match state.auth.authenticate(token).await {
            Ok(authenticated) => authenticated.map(Claims::from),
            Err(error) => {
                warn!(error = ?error, "failed to authenticate a session");
                None
            }
        }
    }
}

/// `Authorization: Bearer <token>` から token を取り出す。
pub struct BearerToken;

impl BearerToken {
    pub fn parse(headers: &HeaderMap) -> Option<&str> {
        let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
        let (scheme, token) = value.split_once(' ')?;
        (scheme.eq_ignore_ascii_case("bearer") && !token.is_empty()).then_some(token.trim())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bearer_token_test() {
        let mut headers = HeaderMap::new();
        assert_eq!(BearerToken::parse(&headers), None);

        headers.insert(AUTHORIZATION, "Bearer abc".parse().unwrap());
        assert_eq!(BearerToken::parse(&headers), Some("abc"));

        headers.insert(AUTHORIZATION, "Basic abc".parse().unwrap());
        assert_eq!(BearerToken::parse(&headers), None);

        headers.insert(AUTHORIZATION, "Bearer ".parse().unwrap());
        assert_eq!(BearerToken::parse(&headers), None);
    }

    #[test]
    fn formats_session_cookies_test() {
        assert_eq!(
            SessionCookie::new(false).issue("token"),
            "__Host-pxna_session=token; Path=/; HttpOnly; Secure; SameSite=Strict; Max-Age=2592000"
        );
        assert_eq!(
            SessionCookie::new(false).clear(),
            "__Host-pxna_session=; Path=/; HttpOnly; Secure; SameSite=Strict; Max-Age=0"
        );
    }

    #[test]
    fn formats_insecure_session_cookies_test() {
        assert_eq!(
            SessionCookie::new(true).issue("token"),
            "pxna_session=token; Path=/; HttpOnly; SameSite=Strict; Max-Age=2592000"
        );
        assert_eq!(SessionCookie::new(true).clear(), "pxna_session=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0");
    }
}
