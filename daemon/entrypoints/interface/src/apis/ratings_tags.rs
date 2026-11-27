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
    /// Invalid request. `code` is `InvalidRequest`, or `UnsupportedType` when the target cannot be opened as an archive, or `LimitExceeded` when an archive exceeds an entry, size, or nesting limit.
    Status400_InvalidRequest(models::Problem),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::Problem),
    /// Resource not found.
    Status404_ResourceNotFound(models::Problem),
    /// Internal server error.
    Status500_InternalServerError(models::Problem),
    /// Extraction or thumbnail generation timed out.
    Status504_ExtractionOrThumbnailGenerationTimedOut(models::Problem),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum GetRatingResponse {
    /// The rating.
    Status200_TheRating(models::RatingResponse),
    /// Invalid request. `code` is `InvalidRequest`, or `UnsupportedType` when the target cannot be opened as an archive, or `LimitExceeded` when an archive exceeds an entry, size, or nesting limit.
    Status400_InvalidRequest(models::Problem),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::Problem),
    /// Resource not found.
    Status404_ResourceNotFound(models::Problem),
    /// Internal server error.
    Status500_InternalServerError(models::Problem),
    /// Extraction or thumbnail generation timed out.
    Status504_ExtractionOrThumbnailGenerationTimedOut(models::Problem),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum ListTagWordsResponse {
    /// The words.
    Status200_TheWords(models::TagsResponse),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::Problem),
    /// Internal server error.
    Status500_InternalServerError(models::Problem),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum PutEntryTagsResponse {
    /// The saved tags.
    Status200_TheSavedTags(models::EntryTagsResponse),
    /// Invalid request. `code` is `InvalidRequest`, or `UnsupportedType` when the target cannot be opened as an archive, or `LimitExceeded` when an archive exceeds an entry, size, or nesting limit.
    Status400_InvalidRequest(models::Problem),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::Problem),
    /// Resource not found.
    Status404_ResourceNotFound(models::Problem),
    /// Internal server error.
    Status500_InternalServerError(models::Problem),
    /// Extraction or thumbnail generation timed out.
    Status504_ExtractionOrThumbnailGenerationTimedOut(models::Problem),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum PutRatingResponse {
    /// The saved rating.
    Status200_TheSavedRating(models::RatingResponse),
    /// Invalid request. `code` is `InvalidRequest`, or `UnsupportedType` when the target cannot be opened as an archive, or `LimitExceeded` when an archive exceeds an entry, size, or nesting limit.
    Status400_InvalidRequest(models::Problem),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::Problem),
    /// Resource not found.
    Status404_ResourceNotFound(models::Problem),
    /// Internal server error.
    Status500_InternalServerError(models::Problem),
    /// Extraction or thumbnail generation timed out.
    Status504_ExtractionOrThumbnailGenerationTimedOut(models::Problem),
}

/// RatingsTags
#[async_trait]
#[allow(clippy::ptr_arg)]
pub trait RatingsTags<E: std::fmt::Debug + Send + Sync + 'static = ()>: super::ErrorHandler<E> {
    type Claims;

    /// Get the tags of an image or video.
    ///
    /// GetEntryTags - GET /api/v1/libraries/{libraryId}/entries/tags
    async fn get_entry_tags(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::GetEntryTagsPathParams,
        query_params: &models::GetEntryTagsQueryParams,
    ) -> Result<GetEntryTagsResponse, E>;

    /// Get the rating of an image or video.
    ///
    /// GetRating - GET /api/v1/libraries/{libraryId}/entries/rating
    async fn get_rating(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::GetRatingPathParams,
        query_params: &models::GetRatingQueryParams,
    ) -> Result<GetRatingResponse, E>;

    /// List the words the current user has created.
    ///
    /// ListTagWords - GET /api/v1/tags/words
    async fn list_tag_words(&self, method: &Method, host: &Host, cookies: &CookieJar, claims: &Self::Claims) -> Result<ListTagWordsResponse, E>;

    /// Replace the tags of an image or video.
    ///
    /// PutEntryTags - PUT /api/v1/libraries/{libraryId}/entries/tags
    async fn put_entry_tags(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::PutEntryTagsPathParams,
        query_params: &models::PutEntryTagsQueryParams,
        body: &models::PutEntryTagsRequest,
    ) -> Result<PutEntryTagsResponse, E>;

    /// Set the rating of an image or video.
    ///
    /// PutRating - PUT /api/v1/libraries/{libraryId}/entries/rating
    async fn put_rating(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::PutRatingPathParams,
        query_params: &models::PutRatingQueryParams,
        body: &models::PutRatingRequest,
    ) -> Result<PutRatingResponse, E>;
}
