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
pub enum GetEntryTagsResponse {
    /// The tags.
    Status200_TheTags(models::EntryTagsResponse),
    /// Invalid request. `kind` is `InvalidRequest`, or `UnsupportedType` when the target cannot be opened as an archive, or `LimitExceeded` when an archive exceeds an entry, size, or nesting limit.
    Status400_InvalidRequest(models::ErrorResponse),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::ErrorResponse),
    /// Resource not found.
    Status404_ResourceNotFound(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
    /// Extraction or thumbnail generation timed out.
    Status504_ExtractionOrThumbnailGenerationTimedOut(models::ErrorResponse),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum ListTagsResponse {
    /// The words.
    Status200_TheWords(models::TagsResponse),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum PutEntryTagsResponse {
    /// The saved tags.
    Status200_TheSavedTags(models::EntryTagsResponse),
    /// Invalid request. `kind` is `InvalidRequest`, or `UnsupportedType` when the target cannot be opened as an archive, or `LimitExceeded` when an archive exceeds an entry, size, or nesting limit.
    Status400_InvalidRequest(models::ErrorResponse),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::ErrorResponse),
    /// Resource not found.
    Status404_ResourceNotFound(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
    /// Extraction or thumbnail generation timed out.
    Status504_ExtractionOrThumbnailGenerationTimedOut(models::ErrorResponse),
}

/// Tags
#[async_trait]
#[allow(clippy::ptr_arg)]
pub trait Tags<E: std::fmt::Debug + Send + Sync + 'static = ()>: super::ErrorHandler<E> {
    type Claims;

    /// Get the tags of an image or video.
    ///
    /// GetEntryTags - GET /api/v1/libraries/{libraryId}/tags
    async fn get_entry_tags(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::GetEntryTagsPathParams,
        query_params: &models::GetEntryTagsQueryParams,
    ) -> Result<GetEntryTagsResponse, E>;

    /// List the words the current user has created.
    ///
    /// ListTags - GET /api/v1/tags
    async fn list_tags(&self, method: &Method, host: &Host, cookies: &CookieJar, claims: &Self::Claims) -> Result<ListTagsResponse, E>;

    /// Replace the tags of an image or video.
    ///
    /// PutEntryTags - PUT /api/v1/libraries/{libraryId}/tags
    async fn put_entry_tags(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::PutEntryTagsPathParams,
        body: &models::PutEntryTagsRequest,
    ) -> Result<PutEntryTagsResponse, E>;
}
