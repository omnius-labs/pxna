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
pub enum CreateVideoSessionResponse {
    /// Video session created.
    Status201_VideoSessionCreated(models::VideoSessionResponse),
    /// Invalid request. `kind` is `InvalidRequest`, or `UnsupportedType` when the target cannot be opened as an archive, or `LimitExceeded` when an archive exceeds an entry, size, or nesting limit.
    Status400_InvalidRequest(models::ErrorResponse),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::ErrorResponse),
    /// The requested resource cannot be accessed or written. `kind` is `Forbidden` for a `readonly` Library and for a mutation inside an archive, or `PasswordRequired` for a password-protected archive.
    Status403_TheRequestedResourceCannotBeAccessedOrWritten(models::ErrorResponse),
    /// Resource not found.
    Status404_ResourceNotFound(models::ErrorResponse),
    /// The request conflicts with the current state of the target. This also covers a source archive that changed while the daemon was reading it.
    Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
    /// Extraction or thumbnail generation timed out.
    Status504_ExtractionOrThumbnailGenerationTimedOut(models::ErrorResponse),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum DeleteVideoSessionResponse {
    /// Video session ended.
    Status204_VideoSessionEnded,
    /// Invalid request. `kind` is `InvalidRequest`, or `UnsupportedType` when the target cannot be opened as an archive, or `LimitExceeded` when an archive exceeds an entry, size, or nesting limit.
    Status400_InvalidRequest(models::ErrorResponse),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::ErrorResponse),
    /// The requested resource cannot be accessed or written. `kind` is `Forbidden` for a `readonly` Library and for a mutation inside an archive, or `PasswordRequired` for a password-protected archive.
    Status403_TheRequestedResourceCannotBeAccessedOrWritten(models::ErrorResponse),
    /// Resource not found.
    Status404_ResourceNotFound(models::ErrorResponse),
    /// The request conflicts with the current state of the target. This also covers a source archive that changed while the daemon was reading it.
    Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
    /// Extraction or thumbnail generation timed out.
    Status504_ExtractionOrThumbnailGenerationTimedOut(models::ErrorResponse),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum GetMediaInfoResponse {
    /// Media information of an image or a video.
    Status200_MediaInformationOfAnImageOrAVideo(models::MediaInfoResponse),
    /// Invalid request. `kind` is `InvalidRequest`, or `UnsupportedType` when the target cannot be opened as an archive, or `LimitExceeded` when an archive exceeds an entry, size, or nesting limit.
    Status400_InvalidRequest(models::ErrorResponse),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::ErrorResponse),
    /// The requested resource cannot be accessed or written. `kind` is `Forbidden` for a `readonly` Library and for a mutation inside an archive, or `PasswordRequired` for a password-protected archive.
    Status403_TheRequestedResourceCannotBeAccessedOrWritten(models::ErrorResponse),
    /// Resource not found.
    Status404_ResourceNotFound(models::ErrorResponse),
    /// The request conflicts with the current state of the target. This also covers a source archive that changed while the daemon was reading it.
    Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
    /// Extraction or thumbnail generation timed out.
    Status504_ExtractionOrThumbnailGenerationTimedOut(models::ErrorResponse),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum GetVideoPlaylistResponse {
    /// VOD playlist.
    Status200_VODPlaylist(String),
    /// Invalid request. `kind` is `InvalidRequest`, or `UnsupportedType` when the target cannot be opened as an archive, or `LimitExceeded` when an archive exceeds an entry, size, or nesting limit.
    Status400_InvalidRequest(models::ErrorResponse),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::ErrorResponse),
    /// The requested resource cannot be accessed or written. `kind` is `Forbidden` for a `readonly` Library and for a mutation inside an archive, or `PasswordRequired` for a password-protected archive.
    Status403_TheRequestedResourceCannotBeAccessedOrWritten(models::ErrorResponse),
    /// Resource not found.
    Status404_ResourceNotFound(models::ErrorResponse),
    /// The request conflicts with the current state of the target. This also covers a source archive that changed while the daemon was reading it.
    Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
    /// Extraction or thumbnail generation timed out.
    Status504_ExtractionOrThumbnailGenerationTimedOut(models::ErrorResponse),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum GetVideoSegmentResponse {
    /// MPEG transport stream segment.
    Status200_MPEGTransportStreamSegment(ByteArray),
    /// Invalid request. `kind` is `InvalidRequest`, or `UnsupportedType` when the target cannot be opened as an archive, or `LimitExceeded` when an archive exceeds an entry, size, or nesting limit.
    Status400_InvalidRequest(models::ErrorResponse),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::ErrorResponse),
    /// The requested resource cannot be accessed or written. `kind` is `Forbidden` for a `readonly` Library and for a mutation inside an archive, or `PasswordRequired` for a password-protected archive.
    Status403_TheRequestedResourceCannotBeAccessedOrWritten(models::ErrorResponse),
    /// Resource not found.
    Status404_ResourceNotFound(models::ErrorResponse),
    /// The request conflicts with the current state of the target. This also covers a source archive that changed while the daemon was reading it.
    Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
    /// Extraction or thumbnail generation timed out.
    Status504_ExtractionOrThumbnailGenerationTimedOut(models::ErrorResponse),
}

/// Video
#[async_trait]
#[allow(clippy::ptr_arg)]
pub trait Video<E: std::fmt::Debug + Send + Sync + 'static = ()>: super::ErrorHandler<E> {
    type Claims;

    /// Start an HLS conversion session.
    ///
    /// CreateVideoSession - POST /api/v1/libraries/{libraryId}/video/sessions
    async fn create_video_session(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::CreateVideoSessionPathParams,
        body: &models::VideoSessionCreateRequest,
    ) -> Result<CreateVideoSessionResponse, E>;

    /// End an HLS conversion session.
    ///
    /// DeleteVideoSession - DELETE /api/v1/libraries/{libraryId}/video/sessions/{sessionId}
    async fn delete_video_session(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::DeleteVideoSessionPathParams,
    ) -> Result<DeleteVideoSessionResponse, E>;

    /// Inspect media information.
    ///
    /// GetMediaInfo - GET /api/v1/libraries/{libraryId}/media-info
    async fn get_media_info(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::GetMediaInfoPathParams,
        query_params: &models::GetMediaInfoQueryParams,
    ) -> Result<GetMediaInfoResponse, E>;

    /// Read the full VOD playlist.
    ///
    /// GetVideoPlaylist - GET /api/v1/libraries/{libraryId}/video/sessions/{sessionId}/playlist.m3u8
    async fn get_video_playlist(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::GetVideoPlaylistPathParams,
    ) -> Result<GetVideoPlaylistResponse, E>;

    /// Read a completed transport stream segment.
    ///
    /// GetVideoSegment - GET /api/v1/libraries/{libraryId}/video/sessions/{sessionId}/segments/{segmentName}
    async fn get_video_segment(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::GetVideoSegmentPathParams,
    ) -> Result<GetVideoSegmentResponse, E>;
}
