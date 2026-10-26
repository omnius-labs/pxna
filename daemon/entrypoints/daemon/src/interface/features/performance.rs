use async_trait::async_trait;
use axum_extra::extract::CookieJar;
use headers::Host;
use http::Method;

use omnius_pxna_engine::telemetry::PERFORMANCE_TARGET;
use omnius_pxna_interface::{apis, models};

use crate::interface::{ApiServer, session::Claims};

#[allow(unused_variables)]
#[async_trait]
impl apis::performance::Performance for ApiServer {
    type Claims = Claims;

    /// クライアントが測った時間を、daemon の計測と同じログへ `client_timing` として出す。計測が無効なら捨てる。
    async fn report_client_timings(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        body: &models::ClientTimingReport,
    ) -> std::result::Result<apis::performance::ReportClientTimingsResponse, ()> {
        if self.state.conf.logging.performance {
            for timing in &body.timings {
                tracing::debug!(
                    target: PERFORMANCE_TARGET,
                    operation = "client_timing",
                    client_operation = %timing.operation,
                    library_id = timing.library_id.as_deref(),
                    path = timing.path.as_deref(),
                    duration_ms = timing.duration_ms,
                    queued_ms = timing.queued_ms,
                    waiting_ms = timing.waiting_ms,
                    download_ms = timing.download_ms,
                    after_response_ms = timing.after_response_ms,
                    bytes = timing.bytes,
                    cached = timing.cached,
                    "client timing"
                );
            }
        }
        Ok(apis::performance::ReportClientTimingsResponse::Status204_TheTimingsWereAccepted)
    }
}

#[cfg(test)]
mod tests {
    use http::StatusCode;
    use omnius_pxna_engine::service::ArchiveOptions;
    use serde_json::json;
    use testresult::TestResult;

    use super::*;
    use crate::interface::server::tests::*;

    #[tokio::test]
    async fn client_timings_are_accepted_and_capabilities_report_the_setting_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let router = router(&fixture);
        let setup = send(&router, Method::POST, "/api/v1/setup", &[], credentials("owner", "password1", "cookie")).await?;
        let cookie = cookie(&setup)?;
        let headers = [("cookie", cookie.as_str()), ("origin", "http://localhost")];

        let capabilities = send(&router, Method::GET, "/api/v1/capabilities", &headers, None).await?;
        assert_eq!(capabilities.body["performanceLogging"], json!(false), "既定では計測を有効にしない");

        let timing = json!({ "timings": [{ "operation": "display_image", "libraryId": "photos", "path": "/a.jpg", "durationMs": 12.5, "waitingMs": 10.0, "cached": false }] });
        let accepted = send(&router, Method::POST, "/api/v1/performance/client-timings", &headers, Some(timing.clone())).await?;
        assert_eq!(accepted.status, StatusCode::NO_CONTENT);

        let empty = send(&router, Method::POST, "/api/v1/performance/client-timings", &headers, Some(json!({ "timings": [] }))).await?;
        assert_eq!(empty.status, StatusCode::BAD_REQUEST);
        let unauthenticated = send(&router, Method::POST, "/api/v1/performance/client-timings", &[], Some(timing)).await?;
        assert_eq!(unauthenticated.status, StatusCode::UNAUTHORIZED);
        Ok(())
    }
}
