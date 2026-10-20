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
pub enum CopyFoldersResponse {
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
pub enum CreateFolderResponse {
    /// Folder created.
    Status201_FolderCreated(models::FolderResponse),
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
pub enum DeleteFoldersResponse {
    /// Delete operation started.
    Status202_DeleteOperationStarted(models::OperationResponse),
    /// Folders deleted.
    Status204_FoldersDeleted,
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
pub enum GetEntryDetailsResponse {
    /// Exactly one detail or error kind per requested path.
    Status200_ExactlyOneDetailOrErrorKindPerRequestedPath(models::EntryDetailsResponse),
    /// Invalid request. `kind` is `InvalidRequest`, or `UnsupportedType` when the target cannot be opened as an archive, or `LimitExceeded` when an archive exceeds an entry, size, or nesting limit.
    Status400_InvalidRequest(models::ErrorResponse),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::ErrorResponse),
    /// The requested resource cannot be accessed or written. `kind` is `Forbidden` for a `readonly` Library and for a mutation inside an archive, or `PasswordRequired` for a password-protected archive.
    Status403_TheRequestedResourceCannotBeAccessedOrWritten(models::ErrorResponse),
    /// Resource not found.
    Status404_ResourceNotFound(models::ErrorResponse),
    /// The request body exceeds the one-shot write limit.
    Status413_TheRequestBodyExceedsTheOne(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum GetFolderResponse {
    /// Folder metadata.
    Status200_FolderMetadata(models::FolderResponse),
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
pub enum ListFolderEntriesResponse {
    /// Folder entries.
    Status200_FolderEntries(models::FolderListing),
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
pub enum ListFolderSummariesResponse {
    /// Names and kinds from one summary snapshot.
    Status200_NamesAndKindsFromOneSummarySnapshot(models::FolderSummaryListing),
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
pub enum MoveFoldersResponse {
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
pub enum ProbeFolderChildrenResponse {
    /// True, confirmed false, or unknown (null).
    Status200_True(models::FolderChildrenProbe),
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
    /// Child probes are not provided; childProbes is false. No response body.
    Status501_ChildProbesAreNotProvided,
}

/// Folders
#[async_trait]
#[allow(clippy::ptr_arg)]
pub trait Folders<E: std::fmt::Debug + Send + Sync + 'static = ()>: super::ErrorHandler<E> {
    type Claims;

    /// Copy folders.
    ///
    /// CopyFolders - POST /api/v1/libraries/{libraryId}/folders/copy
    async fn copy_folders(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::CopyFoldersPathParams,
        body: &models::EntryTransferRequest,
    ) -> Result<CopyFoldersResponse, E>;

    /// Create a folder.
    ///
    /// CreateFolder - POST /api/v1/libraries/{libraryId}/folders
    async fn create_folder(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::CreateFolderPathParams,
        body: &models::CreateFolderRequest,
    ) -> Result<CreateFolderResponse, E>;

    /// Delete folders.
    ///
    /// DeleteFolders - POST /api/v1/libraries/{libraryId}/folders/delete
    async fn delete_folders(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::DeleteFoldersPathParams,
        body: &models::DeleteFoldersRequest,
    ) -> Result<DeleteFoldersResponse, E>;

    /// Read current details for up to 200 entries.
    ///
    /// GetEntryDetails - POST /api/v1/libraries/{libraryId}/entries/details
    async fn get_entry_details(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::GetEntryDetailsPathParams,
        body: &models::EntryDetailsRequest,
    ) -> Result<GetEntryDetailsResponse, E>;

    /// Get folder metadata.
    ///
    /// GetFolder - GET /api/v1/libraries/{libraryId}/folders
    async fn get_folder(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::GetFolderPathParams,
        query_params: &models::GetFolderQueryParams,
    ) -> Result<GetFolderResponse, E>;

    /// List folder entries.
    ///
    /// ListFolderEntries - GET /api/v1/libraries/{libraryId}/folders/entries
    async fn list_folder_entries(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::ListFolderEntriesPathParams,
        query_params: &models::ListFolderEntriesQueryParams,
    ) -> Result<ListFolderEntriesResponse, E>;

    /// List names and kinds without detailed metadata.
    ///
    /// ListFolderSummaries - GET /api/v1/libraries/{libraryId}/folders/summaries
    async fn list_folder_summaries(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::ListFolderSummariesPathParams,
        query_params: &models::ListFolderSummariesQueryParams,
    ) -> Result<ListFolderSummariesResponse, E>;

    /// Move or rename folders.
    ///
    /// MoveFolders - POST /api/v1/libraries/{libraryId}/folders/move
    async fn move_folders(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::MoveFoldersPathParams,
        body: &models::EntryTransferRequest,
    ) -> Result<MoveFoldersResponse, E>;

    /// Probe for a folder-like direct child within a small budget.
    ///
    /// ProbeFolderChildren - GET /api/v1/libraries/{libraryId}/folders/children
    async fn probe_folder_children(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::ProbeFolderChildrenPathParams,
        query_params: &models::ProbeFolderChildrenQueryParams,
    ) -> Result<ProbeFolderChildrenResponse, E>;
}
