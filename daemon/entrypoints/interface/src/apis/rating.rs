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
pub enum GetRatingResponse {
    /// The rating.
    Status200_TheRating(models::RatingResponse),
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
pub enum PutRatingResponse {
    /// The saved rating.
    Status200_TheSavedRating(models::RatingResponse),
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

/// Rating
#[async_trait]
#[allow(clippy::ptr_arg)]
pub trait Rating<E: std::fmt::Debug + Send + Sync + 'static = ()>: super::ErrorHandler<E> {
    type Claims;

    /// Get the rating of an image or video.
    ///
    /// GetRating - GET /api/v1/libraries/{libraryId}/rating
    async fn get_rating(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::GetRatingPathParams,
        query_params: &models::GetRatingQueryParams,
    ) -> Result<GetRatingResponse, E>;

    /// Set the rating of an image or video.
    ///
    /// PutRating - PUT /api/v1/libraries/{libraryId}/rating
    async fn put_rating(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::PutRatingPathParams,
        body: &models::PutRatingRequest,
    ) -> Result<PutRatingResponse, E>;
}
