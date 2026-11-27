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
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum DeleteFilesResponse {
    /// Delete operation started.
    Status202_DeleteOperationStarted(models::OperationResponse),
    /// File deleted.
    Status204_FileDeleted,
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
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum MoveFilesResponse {
    /// Moved synchronously.
    Status200_MovedSynchronously(models::EntryList),
    /// Move operation started.
    Status202_MoveOperationStarted(models::OperationResponse),
    /// Invalid request. `code` is `InvalidRequest`, or `UnsupportedType` when the target cannot be opened as an archive, or `LimitExceeded` when an archive exceeds an entry, size, or nesting limit.
    Status400_InvalidRequest(models::Problem),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::Problem),
    /// The requested resource cannot be accessed or written. `code` is `Forbidden` for a `readonly` Library and for a mutation inside an archive, or `PasswordRequired` for a password-protected archive.
    Status403_TheRequestedResourceCannotBeAccessedOrWritten(models::Problem),
    /// Resource not found.
    Status404_ResourceNotFound(models::Problem),
    /// The request conflicts with the current state of the target. This also covers a source archive that changed while the daemon was reading it.
    Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(models::Problem),
    /// Internal server error.
    Status500_InternalServerError(models::Problem),
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
}
