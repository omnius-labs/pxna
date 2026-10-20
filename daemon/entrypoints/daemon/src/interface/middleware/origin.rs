use axum::{
    Json,
    extract::Request,
    middleware::Next,
    response::{IntoResponse as _, Response},
};
use http::{
    HeaderMap, Method, StatusCode,
    header::{COOKIE, HOST, ORIGIN},
};

use omnius_pxna_interface::models;

use crate::interface::session::{INSECURE_SESSION_COOKIE_NAME, SESSION_COOKIE_NAME};

/// セッションの cookie を持つ変更系の要求で、`Origin` が `Host` と一致するかを検証する（design.md §8.5）。
pub struct OriginCheck;

impl OriginCheck {
    /// `axum::middleware::from_fn` に渡す middleware。検証に通らない要求は、生成されたルータへ渡さずに 403 を返す。
    pub async fn middleware(request: Request, next: Next) -> Response {
        if Self::is_allowed(request.method(), request.headers()) {
            next.run(request).await
        } else {
            let mut body = models::ErrorResponse::new(models::ErrorKind::Forbidden);
            body.message = Some("origin does not match host".to_string());
            (StatusCode::FORBIDDEN, Json(body)).into_response()
        }
    }

    /// cookie を持たない要求は、ブラウザが勝手に資格情報を付けて送ることがないため検証しない。
    fn is_allowed(method: &Method, headers: &HeaderMap) -> bool {
        if matches!(*method, Method::GET | Method::HEAD) || !Self::has_session_cookie(headers) {
            return true;
        }

        // `Origin` がない要求を通すと、`Origin` を付けずに送らせる手段が見つかった時点で検証全体を迂回される。
        let host = headers.get(HOST).and_then(|value| value.to_str().ok());
        let origin = headers.get(ORIGIN).and_then(|value| value.to_str().ok()).and_then(Self::authority);
        matches!((host, origin), (Some(host), Some(origin)) if host.eq_ignore_ascii_case(origin))
    }

    fn has_session_cookie(headers: &HeaderMap) -> bool {
        headers
            .get_all(COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .flat_map(|value| value.split(';'))
            .map(str::trim_start)
            .any(|pair| {
                [SESSION_COOKIE_NAME, INSECURE_SESSION_COOKIE_NAME]
                    .iter()
                    .any(|name| pair.strip_prefix(name).is_some_and(|rest| rest.starts_with('=')))
            })
    }

    /// `scheme://host[:port]` の `host[:port]` を返す。`null` のような scheme のない値は一致させない。
    fn authority(origin: &str) -> Option<&str> {
        let (_, authority) = origin.split_once("://")?;
        (!authority.is_empty() && !authority.contains('/')).then_some(authority)
    }
}

#[cfg(test)]
mod tests {
    use axum::{Router, body::Body, routing::any};
    use testresult::TestResult;
    use tower::ServiceExt as _;

    use super::*;

    const SESSION_COOKIE: &str = "__Host-pxna_session=token";

    async fn status(method: Method, headers: &[(&str, &str)]) -> TestResult<StatusCode> {
        let router = Router::new()
            .route("/", any(|| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn(OriginCheck::middleware));
        let mut request = http::Request::builder().method(method).uri("/").header(HOST, "localhost:6050");
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        Ok(router.oneshot(request.body(Body::empty())?).await?.status())
    }

    #[tokio::test]
    async fn rejects_unsafe_requests_with_session_cookie_from_other_origins_test() -> TestResult {
        assert_eq!(status(Method::POST, &[("cookie", SESSION_COOKIE)]).await?, StatusCode::FORBIDDEN, "Origin がない");
        assert_eq!(
            status(Method::POST, &[("cookie", SESSION_COOKIE), ("origin", "http://evil.example")]).await?,
            StatusCode::FORBIDDEN
        );
        assert_eq!(status(Method::DELETE, &[("cookie", SESSION_COOKIE), ("origin", "null")]).await?, StatusCode::FORBIDDEN);
        assert_eq!(
            status(Method::POST, &[("cookie", "pxna_session=token"), ("origin", "http://evil.example")]).await?,
            StatusCode::FORBIDDEN,
            "insecure_cookie の cookie 名"
        );
        assert_eq!(
            status(Method::PUT, &[("cookie", "theme=dark; __Host-pxna_session=token"), ("origin", "http://localhost")]).await?,
            StatusCode::FORBIDDEN,
            "port が違う"
        );
        Ok(())
    }

    #[tokio::test]
    async fn allows_same_origin_safe_and_cookieless_requests_test() -> TestResult {
        assert_eq!(
            status(Method::POST, &[("cookie", SESSION_COOKIE), ("origin", "http://localhost:6050")]).await?,
            StatusCode::OK
        );
        assert_eq!(status(Method::GET, &[("cookie", SESSION_COOKIE), ("origin", "http://evil.example")]).await?, StatusCode::OK);
        assert_eq!(status(Method::HEAD, &[("cookie", SESSION_COOKIE)]).await?, StatusCode::OK);
        assert_eq!(status(Method::POST, &[]).await?, StatusCode::OK, "setup とログインの前、bearer の要求");
        assert_eq!(
            status(Method::POST, &[("authorization", "Bearer token"), ("origin", "http://evil.example")]).await?,
            StatusCode::OK
        );
        assert_eq!(status(Method::POST, &[("cookie", "theme=dark")]).await?, StatusCode::OK, "セッション以外の cookie");
        Ok(())
    }
}
