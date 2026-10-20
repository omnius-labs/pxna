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
pub enum DeleteCurrentSessionResponse {
    /// Session revoked.
    Status204_SessionRevoked { set_cookie: Option<String> },
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum GetCurrentUserResponse {
    /// Current authenticated user and session.
    Status200_CurrentAuthenticatedUserAndSession(models::CurrentAuth),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum LoginResponse {
    /// Login succeeded.
    Status200_LoginSucceeded { body: models::AuthResult, set_cookie: Option<String> },
    /// Invalid request. `kind` is `InvalidRequest`, or `UnsupportedType` when the target cannot be opened as an archive, or `LimitExceeded` when an archive exceeds an entry, size, or nesting limit.
    Status400_InvalidRequest(models::ErrorResponse),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
}

/// Auth
#[async_trait]
#[allow(clippy::ptr_arg)]
pub trait Auth<E: std::fmt::Debug + Send + Sync + 'static = ()>: super::ErrorHandler<E> {
    type Claims;

    /// Revoke the current session.
    ///
    /// DeleteCurrentSession - DELETE /api/v1/auth/session
    async fn delete_current_session(&self, method: &Method, host: &Host, cookies: &CookieJar, claims: &Self::Claims) -> Result<DeleteCurrentSessionResponse, E>;

    /// Get the current user and session.
    ///
    /// GetCurrentUser - GET /api/v1/auth/me
    async fn get_current_user(&self, method: &Method, host: &Host, cookies: &CookieJar, claims: &Self::Claims) -> Result<GetCurrentUserResponse, E>;

    /// Log in with username and password.
    ///
    /// Login - POST /api/v1/auth/login
    async fn login(&self, method: &Method, host: &Host, cookies: &CookieJar, body: &models::LoginRequest) -> Result<LoginResponse, E>;
}
