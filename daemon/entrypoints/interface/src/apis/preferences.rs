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
pub enum GetPreferencesResponse {
    /// Display preferences.
    Status200_DisplayPreferences(models::Preferences),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum PutPreferencesResponse {
    /// The saved display preferences.
    Status200_TheSavedDisplayPreferences(models::Preferences),
    /// Invalid request. `kind` is `InvalidRequest`, or `UnsupportedType` when the target cannot be opened as an archive, or `LimitExceeded` when an archive exceeds an entry, size, or nesting limit.
    Status400_InvalidRequest(models::ErrorResponse),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
}

/// Preferences
#[async_trait]
#[allow(clippy::ptr_arg)]
pub trait Preferences<E: std::fmt::Debug + Send + Sync + 'static = ()>: super::ErrorHandler<E> {
    type Claims;

    /// Get the display preferences.
    ///
    /// GetPreferences - GET /api/v1/preferences
    async fn get_preferences(&self, method: &Method, host: &Host, cookies: &CookieJar, claims: &Self::Claims) -> Result<GetPreferencesResponse, E>;

    /// Replace the display preferences.
    ///
    /// PutPreferences - PUT /api/v1/preferences
    async fn put_preferences(&self, method: &Method, host: &Host, cookies: &CookieJar, claims: &Self::Claims, body: &models::Preferences) -> Result<PutPreferencesResponse, E>;
}
