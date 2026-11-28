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
    /// Invalid request. `code` is `InvalidRequest`, or `UnsupportedType` when the target cannot be opened as an archive, or `LimitExceeded` when an archive exceeds an entry, size, or nesting limit.
    Status400_InvalidRequest(models::Problem),
    /// The initial owner has already been registered.
    Status409_TheInitialOwnerHasAlreadyBeenRegistered(models::Problem),
    /// Internal server error.
    Status500_InternalServerError(models::Problem),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum DeleteCurrentSessionResponse {
    /// Session revoked.
    Status204_SessionRevoked { set_cookie: Option<String> },
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::Problem),
    /// Internal server error.
    Status500_InternalServerError(models::Problem),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum GetCurrentSessionResponse {
    /// Current authenticated user and session.
    Status200_CurrentAuthenticatedUserAndSession(models::CurrentAuth),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::Problem),
    /// Internal server error.
    Status500_InternalServerError(models::Problem),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum LoginResponse {
    /// Login succeeded.
    Status200_LoginSucceeded { body: models::AuthResult, set_cookie: Option<String> },
    /// Invalid request. `code` is `InvalidRequest`, or `UnsupportedType` when the target cannot be opened as an archive, or `LimitExceeded` when an archive exceeds an entry, size, or nesting limit.
    Status400_InvalidRequest(models::Problem),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::Problem),
    /// Internal server error.
    Status500_InternalServerError(models::Problem),
}

/// Auth
#[async_trait]
#[allow(clippy::ptr_arg)]
pub trait Auth<E: std::fmt::Debug + Send + Sync + 'static = ()>: super::ErrorHandler<E> {
    type Claims;

    /// Register the initial owner.
    ///
    /// CreateInitialSetup - POST /api/v1/setup
    async fn create_initial_setup(&self, method: &Method, host: &Host, cookies: &CookieJar, body: &models::InitialSetupRequest) -> Result<CreateInitialSetupResponse, E>;

    /// Revoke the current session.
    ///
    /// DeleteCurrentSession - DELETE /api/v1/auth/session
    async fn delete_current_session(&self, method: &Method, host: &Host, cookies: &CookieJar, claims: &Self::Claims) -> Result<DeleteCurrentSessionResponse, E>;

    /// Get the current user and session.
    ///
    /// GetCurrentSession - GET /api/v1/auth/session
    async fn get_current_session(&self, method: &Method, host: &Host, cookies: &CookieJar, claims: &Self::Claims) -> Result<GetCurrentSessionResponse, E>;

    /// Log in with username and password.
    ///
    /// Login - POST /api/v1/auth/login
    async fn login(&self, method: &Method, host: &Host, cookies: &CookieJar, body: &models::LoginRequest) -> Result<LoginResponse, E>;
}
