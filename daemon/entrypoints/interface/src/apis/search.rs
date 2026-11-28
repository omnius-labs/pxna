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
pub enum ListSearchEntriesResponse {
    /// Matching entries of the Libraries.
    Status200_MatchingEntriesOfTheLibraries(models::SearchListing),
    /// Invalid request. `code` is `InvalidRequest`, or `UnsupportedType` when the target cannot be opened as an archive, or `LimitExceeded` when an archive exceeds an entry, size, or nesting limit.
    Status400_InvalidRequest(models::Problem),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::Problem),
    /// The requested resource cannot be accessed or written. `code` is `Forbidden` for a `readonly` Library and for a mutation inside an archive, or `PasswordRequired` for a password-protected archive.
    Status403_TheRequestedResourceCannotBeAccessedOrWritten(models::Problem),
    /// Resource not found.
    Status404_ResourceNotFound(models::Problem),
    /// Internal server error.
    Status500_InternalServerError(models::Problem),
    /// Extraction or thumbnail generation timed out.
    Status504_ExtractionOrThumbnailGenerationTimedOut(models::Problem),
}

/// Search
#[async_trait]
#[allow(clippy::ptr_arg)]
pub trait Search<E: std::fmt::Debug + Send + Sync + 'static = ()>: super::ErrorHandler<E> {
    type Claims;

    /// Search entries across Libraries.
    ///
    /// ListSearchEntries - GET /api/v1/search/entries
    async fn list_search_entries(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        query_params: &models::ListSearchEntriesQueryParams,
    ) -> Result<ListSearchEntriesResponse, E>;
}
