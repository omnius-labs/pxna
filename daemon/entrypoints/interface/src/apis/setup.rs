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
pub enum CreateInitialSetupResponse {
    /// Owner and authenticated session created.
    Status201_OwnerAndAuthenticatedSessionCreated { body: models::AuthResult, set_cookie: Option<String> },
    /// Invalid request. `kind` is `InvalidRequest`, or `UnsupportedType` when the target cannot be opened as an archive, or `LimitExceeded` when an archive exceeds an entry, size, or nesting limit.
    Status400_InvalidRequest(models::ErrorResponse),
    /// The initial owner has already been registered.
    Status409_TheInitialOwnerHasAlreadyBeenRegistered(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum GetSetupStatusResponse {
    /// Current setup status.
    Status200_CurrentSetupStatus(models::SetupStatus),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
}

/// Setup
#[async_trait]
#[allow(clippy::ptr_arg)]
pub trait Setup<E: std::fmt::Debug + Send + Sync + 'static = ()>: super::ErrorHandler<E> {
    /// Register the initial owner.
    ///
    /// CreateInitialSetup - POST /api/v1/setup
    async fn create_initial_setup(&self, method: &Method, host: &Host, cookies: &CookieJar, body: &models::InitialSetupRequest) -> Result<CreateInitialSetupResponse, E>;

    /// Get initial setup status.
    ///
    /// GetSetupStatus - GET /api/v1/setup/status
    async fn get_setup_status(&self, method: &Method, host: &Host, cookies: &CookieJar) -> Result<GetSetupStatusResponse, E>;
}
