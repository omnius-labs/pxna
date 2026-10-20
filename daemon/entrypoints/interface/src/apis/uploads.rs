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
pub enum CommitUploadResponse {
    /// File created at the target path.
    Status201_FileCreatedAtTheTargetPath(models::FileResponse),
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
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum CreateUploadResponse {
    /// Upload session created.
    Status201_UploadSessionCreated(models::UploadResponse),
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
    /// The daemon does not have enough disk space.
    Status507_TheDaemonDoesNotHaveEnoughDiskSpace(models::ErrorResponse),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum DeleteUploadResponse {
    /// Upload session discarded.
    Status204_UploadSessionDiscarded,
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
pub enum GetUploadResponse {
    /// Upload session state.
    Status200_UploadSessionState(models::UploadResponse),
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
pub enum WriteUploadContentResponse {
    /// Chunk accepted.
    Status204_ChunkAccepted,
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
    /// The daemon does not have enough disk space.
    Status507_TheDaemonDoesNotHaveEnoughDiskSpace(models::ErrorResponse),
}

/// Uploads
#[async_trait]
#[allow(clippy::ptr_arg)]
pub trait Uploads<E: std::fmt::Debug + Send + Sync + 'static = ()>: super::ErrorHandler<E> {
    type Claims;

    /// Commit an upload session.
    ///
    /// CommitUpload - POST /api/v1/libraries/{libraryId}/uploads/{uploadId}/commit
    async fn commit_upload(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::CommitUploadPathParams,
    ) -> Result<CommitUploadResponse, E>;

    /// Create an upload session.
    ///
    /// CreateUpload - POST /api/v1/libraries/{libraryId}/uploads
    async fn create_upload(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::CreateUploadPathParams,
        body: &models::UploadCreateRequest,
    ) -> Result<CreateUploadResponse, E>;

    /// Abort an upload session.
    ///
    /// DeleteUpload - DELETE /api/v1/libraries/{libraryId}/uploads/{uploadId}
    async fn delete_upload(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::DeleteUploadPathParams,
    ) -> Result<DeleteUploadResponse, E>;

    /// Get an upload session.
    ///
    /// GetUpload - GET /api/v1/libraries/{libraryId}/uploads/{uploadId}
    async fn get_upload(&self, method: &Method, host: &Host, cookies: &CookieJar, claims: &Self::Claims, path_params: &models::GetUploadPathParams)
    -> Result<GetUploadResponse, E>;

    /// Write an upload chunk.
    ///
    /// WriteUploadContent - PUT /api/v1/libraries/{libraryId}/uploads/{uploadId}/content
    async fn write_upload_content(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::WriteUploadContentPathParams,
        query_params: &models::WriteUploadContentQueryParams,
        body: &Bytes,
    ) -> Result<WriteUploadContentResponse, E>;
}
