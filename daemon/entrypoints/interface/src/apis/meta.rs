use async_trait::async_trait;
use axum::extract::*;
use axum_extra::extract::CookieJar;
use bytes::Bytes;
use headers::Host;
use http::Method;
use serde::{Deserialize, Serialize};

use crate::{models, types::*};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum GetCapabilitiesResponse {
    /// Optional browsing features implemented by this daemon.
    Status200_OptionalBrowsingFeaturesImplementedByThisDaemon(models::Capabilities),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::Problem),
    /// Internal server error.
    Status500_InternalServerError(models::Problem),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum GetHealthResponse {
    /// The daemon is running.
    Status200_TheDaemonIsRunning(models::HealthResponse),
    /// Internal server error.
    Status500_InternalServerError(models::Problem),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum GetSetupStatusResponse {
    /// Current setup status.
    Status200_CurrentSetupStatus(models::SetupStatus),
    /// Internal server error.
    Status500_InternalServerError(models::Problem),
}

/// Meta
#[async_trait]
#[allow(clippy::ptr_arg)]
pub trait Meta<E: std::fmt::Debug + Send + Sync + 'static = ()>: super::ErrorHandler<E> {
    type Claims;

    /// Get optional browsing capabilities.
    ///
    /// GetCapabilities - GET /api/v1/capabilities
    async fn get_capabilities(&self, method: &Method, host: &Host, cookies: &CookieJar, claims: &Self::Claims) -> Result<GetCapabilitiesResponse, E>;

    /// Check daemon health.
    ///
    /// GetHealth - GET /api/v1/health
    async fn get_health(&self, method: &Method, host: &Host, cookies: &CookieJar) -> Result<GetHealthResponse, E>;

    /// Get initial setup status.
    ///
    /// GetSetupStatus - GET /api/v1/setup/status
    async fn get_setup_status(&self, method: &Method, host: &Host, cookies: &CookieJar) -> Result<GetSetupStatusResponse, E>;
}
