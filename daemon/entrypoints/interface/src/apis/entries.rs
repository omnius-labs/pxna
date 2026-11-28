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
pub enum CopyEntriesResponse {
    /// The Operation, terminal when the work finished before the response.
    Status202_TheOperation(models::OperationResponse),
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

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum CreateFileResponse {
    /// File created.
    Status201_FileCreated(models::EntryResponse),
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
    /// The request body exceeds the one-shot write limit.
    Status413_TheRequestBodyExceedsTheOne(models::Problem),
    /// Internal server error.
    Status500_InternalServerError(models::Problem),
    /// The daemon does not have enough disk space.
    Status507_TheDaemonDoesNotHaveEnoughDiskSpace(models::Problem),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum CreateFolderResponse {
    /// Folder created.
    Status201_FolderCreated(models::EntryResponse),
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

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum DeleteEntriesResponse {
    /// The Operation, terminal when the work finished before the response.
    Status202_TheOperation(models::OperationResponse),
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

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum GetEntryMetadataResponse {
    /// The Entry.
    Status200_TheEntry(models::EntryResponse),
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
    /// Extraction or thumbnail generation timed out.
    Status504_ExtractionOrThumbnailGenerationTimedOut(models::Problem),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum ListEntriesResponse {
    /// The Entries of the Scope.
    Status200_TheEntriesOfTheScope(models::EntryPage),
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

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum ListEntryMetadataResponse {
    /// Exactly one Entry or error kind per place.
    Status200_ExactlyOneEntryOrErrorKindPerPlace(models::EntryMetadataList),
    /// Invalid request. `code` is `InvalidRequest`, or `UnsupportedType` when the target cannot be opened as an archive, or `LimitExceeded` when an archive exceeds an entry, size, or nesting limit.
    Status400_InvalidRequest(models::Problem),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::Problem),
    /// The requested resource cannot be accessed or written. `code` is `Forbidden` for a `readonly` Library and for a mutation inside an archive, or `PasswordRequired` for a password-protected archive.
    Status403_TheRequestedResourceCannotBeAccessedOrWritten(models::Problem),
    /// Internal server error.
    Status500_InternalServerError(models::Problem),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum ListEntryMonthsResponse {
    /// Entry counts per month.
    Status200_EntryCountsPerMonth(models::TimelineMonths),
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

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum MoveEntriesResponse {
    /// The Operation, terminal when the work finished before the response.
    Status202_TheOperation(models::OperationResponse),
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

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum ProbeFolderChildrenResponse {
    /// True, confirmed false, or unknown (null).
    Status200_True(models::FolderChildrenProbe),
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
    /// Child probes are not provided; childProbes is false. No response body.
    Status501_ChildProbesAreNotProvided,
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
    /// Requested byte range cannot be satisfied.
    Status416_RequestedByteRangeCannotBeSatisfied(models::Problem),
    /// Internal server error.
    Status500_InternalServerError(models::Problem),
    /// Extraction or thumbnail generation timed out.
    Status504_ExtractionOrThumbnailGenerationTimedOut(models::Problem),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum WriteFileContentResponse {
    /// File written.
    Status201_FileWritten(models::EntryResponse),
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
    /// The request body exceeds the one-shot write limit.
    Status413_TheRequestBodyExceedsTheOne(models::Problem),
    /// Internal server error.
    Status500_InternalServerError(models::Problem),
    /// The daemon does not have enough disk space.
    Status507_TheDaemonDoesNotHaveEnoughDiskSpace(models::Problem),
}

/// Entries
#[async_trait]
#[allow(clippy::ptr_arg)]
pub trait Entries<E: std::fmt::Debug + Send + Sync + 'static = ()>: super::ErrorHandler<E> {
    type Claims;

    /// Copy Entries.
    ///
    /// CopyEntries - POST /api/v1/entries/copy
    async fn copy_entries(&self, method: &Method, host: &Host, cookies: &CookieJar, claims: &Self::Claims, body: &models::EntryTransferRequest) -> Result<CopyEntriesResponse, E>;

    /// Create a file under a name the daemon may choose.
    ///
    /// CreateFile - POST /api/v1/libraries/{libraryId}/entries/files
    async fn create_file(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::CreateFilePathParams,
        query_params: &models::CreateFileQueryParams,
        body: &Bytes,
    ) -> Result<CreateFileResponse, E>;

    /// Create a folder.
    ///
    /// CreateFolder - POST /api/v1/libraries/{libraryId}/entries/folders
    async fn create_folder(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::CreateFolderPathParams,
        body: &models::CreateFolderRequest,
    ) -> Result<CreateFolderResponse, E>;

    /// Delete Entries.
    ///
    /// DeleteEntries - POST /api/v1/entries/delete
    async fn delete_entries(&self, method: &Method, host: &Host, cookies: &CookieJar, claims: &Self::Claims, body: &models::EntryDeleteRequest)
    -> Result<DeleteEntriesResponse, E>;

    /// Get the metadata of an Entry.
    ///
    /// GetEntryMetadata - GET /api/v1/libraries/{libraryId}/entries/metadata
    async fn get_entry_metadata(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::GetEntryMetadataPathParams,
        query_params: &models::GetEntryMetadataQueryParams,
    ) -> Result<GetEntryMetadataResponse, E>;

    /// List the Entries of a Scope.
    ///
    /// ListEntries - POST /api/v1/entries
    async fn list_entries(&self, method: &Method, host: &Host, cookies: &CookieJar, claims: &Self::Claims, body: &models::EntryListRequest) -> Result<ListEntriesResponse, E>;

    /// Read the metadata of up to 200 Entries.
    ///
    /// ListEntryMetadata - POST /api/v1/entries/metadata
    async fn list_entry_metadata(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        body: &models::EntryMetadataRequest,
    ) -> Result<ListEntryMetadataResponse, E>;

    /// List the months that have Entries of a Scope.
    ///
    /// ListEntryMonths - POST /api/v1/entries/months
    async fn list_entry_months(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        body: &models::EntryMonthsRequest,
    ) -> Result<ListEntryMonthsResponse, E>;

    /// Move or rename Entries.
    ///
    /// MoveEntries - POST /api/v1/entries/move
    async fn move_entries(&self, method: &Method, host: &Host, cookies: &CookieJar, claims: &Self::Claims, body: &models::EntryTransferRequest) -> Result<MoveEntriesResponse, E>;

    /// Probe whether a folder has a folder-like direct child.
    ///
    /// ProbeFolderChildren - GET /api/v1/libraries/{libraryId}/entries/child-probe
    async fn probe_folder_children(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::ProbeFolderChildrenPathParams,
        query_params: &models::ProbeFolderChildrenQueryParams,
    ) -> Result<ProbeFolderChildrenResponse, E>;

    /// Read file content.
    ///
    /// ReadFileContent - GET /api/v1/libraries/{libraryId}/entries/content
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
    /// WriteFileContent - PUT /api/v1/libraries/{libraryId}/entries/content
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
