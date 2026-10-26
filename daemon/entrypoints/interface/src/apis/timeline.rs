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
pub enum GetTimelineStatusResponse {
    /// State of the scans.
    Status200_StateOfTheScans(models::TimelineStatuses),
    /// Invalid request. `kind` is `InvalidRequest`, or `UnsupportedType` when the target cannot be opened as an archive, or `LimitExceeded` when an archive exceeds an entry, size, or nesting limit.
    Status400_InvalidRequest(models::ErrorResponse),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::ErrorResponse),
    /// Resource not found.
    Status404_ResourceNotFound(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum ListTimelineEntriesResponse {
    /// Timeline entries.
    Status200_TimelineEntries(models::TimelineListing),
    /// Invalid request. `kind` is `InvalidRequest`, or `UnsupportedType` when the target cannot be opened as an archive, or `LimitExceeded` when an archive exceeds an entry, size, or nesting limit.
    Status400_InvalidRequest(models::ErrorResponse),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::ErrorResponse),
    /// Resource not found.
    Status404_ResourceNotFound(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum ListTimelineMonthsResponse {
    /// Entry counts per month.
    Status200_EntryCountsPerMonth(models::TimelineMonths),
    /// Invalid request. `kind` is `InvalidRequest`, or `UnsupportedType` when the target cannot be opened as an archive, or `LimitExceeded` when an archive exceeds an entry, size, or nesting limit.
    Status400_InvalidRequest(models::ErrorResponse),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::ErrorResponse),
    /// Resource not found.
    Status404_ResourceNotFound(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum RefreshTimelineResponse {
    /// The scans are running.
    Status202_TheScansAreRunning(models::TimelineStatuses),
    /// Invalid request. `kind` is `InvalidRequest`, or `UnsupportedType` when the target cannot be opened as an archive, or `LimitExceeded` when an archive exceeds an entry, size, or nesting limit.
    Status400_InvalidRequest(models::ErrorResponse),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::ErrorResponse),
    /// Resource not found.
    Status404_ResourceNotFound(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
}

/// Timeline
#[async_trait]
#[allow(clippy::ptr_arg)]
pub trait Timeline<E: std::fmt::Debug + Send + Sync + 'static = ()>: super::ErrorHandler<E> {
    type Claims;

    /// Get the state of the timeline scans.
    ///
    /// GetTimelineStatus - GET /api/v1/timeline/status
    async fn get_timeline_status(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        query_params: &models::GetTimelineStatusQueryParams,
    ) -> Result<GetTimelineStatusResponse, E>;

    /// List timeline entries.
    ///
    /// ListTimelineEntries - GET /api/v1/timeline/entries
    async fn list_timeline_entries(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        query_params: &models::ListTimelineEntriesQueryParams,
    ) -> Result<ListTimelineEntriesResponse, E>;

    /// List the months that have timeline entries.
    ///
    /// ListTimelineMonths - GET /api/v1/timeline/months
    async fn list_timeline_months(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        query_params: &models::ListTimelineMonthsQueryParams,
    ) -> Result<ListTimelineMonthsResponse, E>;

    /// Start full timeline scans.
    ///
    /// RefreshTimeline - POST /api/v1/timeline/refresh
    async fn refresh_timeline(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        body: &models::TimelineRefreshRequest,
    ) -> Result<RefreshTimelineResponse, E>;
}
