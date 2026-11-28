//! The daemon's liveness, the optional features it implements, and whether its owner is registered.
use async_trait::async_trait;
use axum_extra::extract::CookieJar;
use headers::Host;
use http::Method;

use omnius_pxna_interface::{apis, models};

use crate::{
    error::ApiError,
    interface::{ApiServer, session::Claims},
    prelude::*,
};

#[allow(unused_variables)]
#[async_trait]
impl apis::meta::Meta for ApiServer {
    type Claims = Claims;

    async fn get_capabilities(&self, method: &Method, host: &Host, cookies: &CookieJar, claims: &Self::Claims) -> std::result::Result<apis::meta::GetCapabilitiesResponse, ()> {
        Ok(apis::meta::GetCapabilitiesResponse::Status200_OptionalBrowsingFeaturesImplementedByThisDaemon(
            models::Capabilities::new(true, true, true, self.state.conf.logging.performance),
        ))
    }

    async fn get_health(&self, method: &Method, host: &Host, cookies: &CookieJar) -> std::result::Result<apis::meta::GetHealthResponse, ()> {
        Ok(apis::meta::GetHealthResponse::Status200_TheDaemonIsRunning(models::HealthResponse::new(true)))
    }

    async fn get_setup_status(&self, method: &Method, host: &Host, cookies: &CookieJar) -> std::result::Result<apis::meta::GetSetupStatusResponse, ()> {
        Ok(match self.state.auth.is_initialized().await.map_err(Error::from) {
            Ok(initialized) => apis::meta::GetSetupStatusResponse::Status200_CurrentSetupStatus(models::SetupStatus::new(initialized)),
            Err(error) => ApiError::from(error).into(),
        })
    }
}

impl From<ApiError> for apis::meta::GetSetupStatusResponse {
    fn from(error: ApiError) -> Self {
        Self::Status500_InternalServerError(error.body)
    }
}

#[cfg(test)]
mod tests {
    use http::StatusCode;
    use omnius_pxna_engine::service::ArchiveOptions;
    use testresult::TestResult;

    use super::*;
    use crate::interface::server::tests::*;

    #[test]
    fn generated_input_validator_accepts_valid_and_rejects_empty_credentials_test() {
        use validator::Validate;

        let valid: models::InitialSetupRequest = serde_json::from_value(serde_json::json!({
            "username": "alice", "password": "password1", "sessionTransport": "cookie"
        }))
        .unwrap();
        assert!(valid.validate().is_ok());
        let invalid: models::InitialSetupRequest = serde_json::from_value(serde_json::json!({
            "username": "", "password": "", "sessionTransport": "cookie"
        }))
        .unwrap();
        let errors = invalid.validate().unwrap_err();
        assert!(errors.field_errors().contains_key("username"));
        assert!(errors.field_errors().contains_key("password"));
    }

    #[tokio::test]
    async fn setup_registers_one_owner_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let router = router(&fixture);

        let status = send(&router, Method::GET, "/api/v1/setup/status", &[], None).await?;
        assert_eq!((status.status, status.body["initialized"].as_bool()), (StatusCode::OK, Some(false)));

        let (a, b) = tokio::join!(
            send(&router, Method::POST, "/api/v1/setup", &[], credentials("alice", "password1", "cookie")),
            send(&router, Method::POST, "/api/v1/setup", &[], credentials("bob", "password2", "cookie")),
        );
        let mut responses = [a?, b?];
        responses.sort_by_key(|response| response.status);
        assert_eq!(responses.iter().map(|response| response.status.as_u16()).collect::<Vec<_>>(), [201, 409]);
        let [created, conflict] = &responses;
        assert!(created.set_cookie().is_some_and(|value| value.starts_with("__Host-pxna_session=")));
        assert!(created.body.get("accessToken").is_none(), "cookie のセッションは token を本文に載せない");
        assert_eq!(conflict.body["code"], "AlreadyInitialized");

        let status = send(&router, Method::GET, "/api/v1/setup/status", &[], None).await?;
        assert_eq!(status.body["initialized"].as_bool(), Some(true));
        Ok(())
    }
}
