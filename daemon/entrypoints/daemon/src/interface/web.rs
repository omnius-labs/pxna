use std::{convert::Infallible, path::Path};

use axum::{
    extract::Request,
    response::{IntoResponse as _, Response},
};
use tower::ServiceExt as _;
use tower_http::services::{ServeDir, ServeFile};

/// `[web] dir` の web の成果物を、API と同じポートで返す（design.md §10.1）。
#[derive(Clone)]
pub struct WebAssets {
    files: ServeDir<ServeFile>,
}

impl WebAssets {
    /// ファイルのない path には `index.html` を返し、web の route をリロードしても同じ画面へ戻れるようにする。
    pub fn new(dir: &Path) -> Self {
        Self {
            files: ServeDir::new(dir).fallback(ServeFile::new(dir.join("index.html"))),
        }
    }

    /// `/api` の下の要求は `api` へ渡し、それ以外の要求に成果物を返す外側のルータを作る。
    pub fn router(self, api: axum::Router) -> axum::Router {
        axum::Router::new().fallback(move |request: Request| self.clone().dispatch(api.clone(), request))
    }

    /// `/api` の下で経路がない要求は、`index.html` ではなく API の 404 にする。
    /// API の誤った呼び出しが HTML の 200 として成功して見えないようにするためである。
    async fn dispatch(self, api: axum::Router, request: Request) -> Response {
        let response: Result<Response, Infallible> = if Self::is_api_path(request.uri().path()) {
            api.oneshot(request).await
        } else {
            self.files.oneshot(request).await.map(|response| response.into_response())
        };
        match response {
            Ok(response) => response,
            Err(never) => match never {},
        }
    }

    fn is_api_path(path: &str) -> bool {
        path == "/api" || path.starts_with("/api/")
    }
}

#[cfg(test)]
mod tests {
    use http::{Method, StatusCode};
    use omnius_pxna_engine::service::ArchiveOptions;
    use testresult::TestResult;

    use crate::interface::server::tests::*;

    /// `index.html` と 1 つの asset を置いた web の成果物の directory を作る。
    fn web_dist() -> TestResult<tempfile::TempDir> {
        let dist = tempfile::tempdir()?;
        std::fs::write(dist.path().join("index.html"), b"<!doctype html><title>pxna</title>")?;
        std::fs::create_dir_all(dist.path().join("assets"))?;
        std::fs::write(dist.path().join("assets/app.js"), b"console.log(1);")?;
        Ok(dist)
    }

    #[tokio::test]
    async fn web_returns_index_for_web_routes_test() -> TestResult {
        let dist = web_dist()?;
        let fixture = fixture_with_web(ArchiveOptions::default(), Some(dist.path().to_path_buf())).await?;
        let router = router(&fixture);

        for uri in ["/", "/browse/photos", "/browse/photos/a%20b/c.7z/x"] {
            let response = send(&router, Method::GET, uri, &[], None).await?;
            assert_eq!(response.status, StatusCode::OK, "{uri}");
            assert!(response.header("content-type").is_some_and(|value| value.starts_with("text/html")), "{uri}");
            assert_eq!(response.bytes, b"<!doctype html><title>pxna</title>", "{uri}");
        }
        Ok(())
    }

    #[tokio::test]
    async fn web_returns_assets_test() -> TestResult {
        let dist = web_dist()?;
        let fixture = fixture_with_web(ArchiveOptions::default(), Some(dist.path().to_path_buf())).await?;

        let response = send(&router(&fixture), Method::GET, "/assets/app.js", &[], None).await?;
        assert_eq!(response.status, StatusCode::OK);
        assert!(response.header("content-type").is_some_and(|value| value.contains("javascript")));
        assert_eq!(response.bytes, b"console.log(1);");
        Ok(())
    }

    #[tokio::test]
    async fn web_does_not_fall_back_under_api_test() -> TestResult {
        let dist = web_dist()?;
        let fixture = fixture_with_web(ArchiveOptions::default(), Some(dist.path().to_path_buf())).await?;
        let router = router(&fixture);

        for uri in ["/api", "/api/v1/nope"] {
            let response = send(&router, Method::GET, uri, &[], None).await?;
            assert_eq!(response.status, StatusCode::NOT_FOUND, "{uri}");
            assert!(!response.header("content-type").is_some_and(|value| value.starts_with("text/html")), "{uri}");
        }
        assert_eq!(send(&router, Method::GET, "/api/v1/health", &[], None).await?.status, StatusCode::OK);
        Ok(())
    }

    #[tokio::test]
    async fn web_is_not_served_without_dir_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let response = send(&router(&fixture), Method::GET, "/", &[], None).await?;
        assert_eq!(response.status, StatusCode::NOT_FOUND);
        Ok(())
    }
}
