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
pub enum CopyFilesResponse {
    /// Copy operation started.
    Status202_CopyOperationStarted(models::OperationResponse),
    /// Invalid request. `kind` is `InvalidRequest`, or `UnsupportedType` when the target cannot be opened as an archive, or `LimitExceeded` when an archive exceeds an entry, size, or nesting limit.
    Status400_InvalidRequest(models::ErrorResponse),
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
pub enum DeleteFilesResponse {
    /// Delete operation started.
    Status202_DeleteOperationStarted(models::OperationResponse),
    /// File deleted.
    Status204_FileDeleted,
    /// Invalid request. `kind` is `InvalidRequest`, or `UnsupportedType` when the target cannot be opened as an archive, or `LimitExceeded` when an archive exceeds an entry, size, or nesting limit.
    Status400_InvalidRequest(models::ErrorResponse),
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
pub enum GetFileResponse {
    /// File metadata.
    Status200_FileMetadata(models::FileResponse),
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
pub enum MoveFilesResponse {
    /// Moved synchronously.
    Status200_MovedSynchronously(models::EntryList),
    /// Move operation started.
    Status202_MoveOperationStarted(models::OperationResponse),
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
pub enum ReadFileContentResponse {
    /// Full or requested file bytes.
    Status200_FullOrRequestedFileBytes {
        body: ByteArray,
        accept_ranges: Option<String>,
        content_length: Option<u64>,
        e_tag: Option<String>,
    },
    /// Partial file bytes.
    Status206_PartialFileBytes {
        body: ByteArray,
        accept_ranges: Option<String>,
        content_range: Option<String>,
        content_length: Option<u64>,
        e_tag: Option<String>,
    },
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
    /// Requested byte range cannot be satisfied.
    Status416_RequestedByteRangeCannotBeSatisfied(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
    /// Extraction or thumbnail generation timed out.
    Status504_ExtractionOrThumbnailGenerationTimedOut(models::ErrorResponse),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum WriteFileContentResponse {
    /// File written.
    Status201_FileWritten(models::FileResponse),
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
    /// The request body exceeds the one-shot write limit.
    Status413_TheRequestBodyExceedsTheOne(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
    /// The daemon does not have enough disk space.
    Status507_TheDaemonDoesNotHaveEnoughDiskSpace(models::ErrorResponse),
}

/// Files
#[async_trait]
#[allow(clippy::ptr_arg)]
pub trait Files<E: std::fmt::Debug + Send + Sync + 'static = ()>: super::ErrorHandler<E> {
    type Claims;

    /// Copy files.
    ///
    /// CopyFiles - POST /api/v1/libraries/{libraryId}/files/copy
    async fn copy_files(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::CopyFilesPathParams,
        body: &models::EntryTransferRequest,
    ) -> Result<CopyFilesResponse, E>;

    /// Delete files.
    ///
    /// DeleteFiles - POST /api/v1/libraries/{libraryId}/files/delete
    async fn delete_files(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::DeleteFilesPathParams,
        body: &models::DeleteFilesRequest,
    ) -> Result<DeleteFilesResponse, E>;

    /// Get file metadata.
    ///
    /// GetFile - GET /api/v1/libraries/{libraryId}/files
    async fn get_file(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::GetFilePathParams,
        query_params: &models::GetFileQueryParams,
    ) -> Result<GetFileResponse, E>;

    /// Move or rename files.
    ///
    /// MoveFiles - POST /api/v1/libraries/{libraryId}/files/move
    async fn move_files(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::MoveFilesPathParams,
        body: &models::EntryTransferRequest,
    ) -> Result<MoveFilesResponse, E>;

    /// Read file content.
    ///
    /// ReadFileContent - GET /api/v1/libraries/{libraryId}/files/content
    async fn read_file_content(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        header_params: &models::ReadFileContentHeaderParams,
        path_params: &models::ReadFileContentPathParams,
        query_params: &models::ReadFileContentQueryParams,
    ) -> Result<ReadFileContentResponse, E>;

    /// Write file content in one request.
    ///
    /// WriteFileContent - PUT /api/v1/libraries/{libraryId}/files/content
    async fn write_file_content(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::WriteFileContentPathParams,
        query_params: &models::WriteFileContentQueryParams,
        body: &Bytes,
    ) -> Result<WriteFileContentResponse, E>;
}
