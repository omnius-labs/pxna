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
    Status401_MissingOrInvalidAuthentication(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum GetLibraryResponse {
    /// Library metadata.
    Status200_LibraryMetadata(models::LibraryResponse),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::ErrorResponse),
    /// The requested resource cannot be accessed or written. `kind` is `Forbidden` for a `readonly` Library and for a mutation inside an archive, or `PasswordRequired` for a password-protected archive.
    Status403_TheRequestedResourceCannotBeAccessedOrWritten(models::ErrorResponse),
    /// Resource not found.
    Status404_ResourceNotFound(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum ListLibrariesResponse {
    /// Available Libraries.
    Status200_AvailableLibraries(models::LibraryList),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
}

/// Libraries
#[async_trait]
#[allow(clippy::ptr_arg)]
pub trait Libraries<E: std::fmt::Debug + Send + Sync + 'static = ()>: super::ErrorHandler<E> {
    type Claims;

    /// Get optional browsing capabilities.
    ///
    /// GetCapabilities - GET /api/v1/capabilities
    async fn get_capabilities(&self, method: &Method, host: &Host, cookies: &CookieJar, claims: &Self::Claims) -> Result<GetCapabilitiesResponse, E>;

    /// Get a Library.
    ///
    /// GetLibrary - GET /api/v1/libraries/{libraryId}
    async fn get_library(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::GetLibraryPathParams,
    ) -> Result<GetLibraryResponse, E>;

    /// List available Libraries.
    ///
    /// ListLibraries - GET /api/v1/libraries
    async fn list_libraries(&self, method: &Method, host: &Host, cookies: &CookieJar, claims: &Self::Claims) -> Result<ListLibrariesResponse, E>;
}
