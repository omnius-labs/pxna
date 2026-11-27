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
pub enum CreateShelfResponse {
    /// The created Shelf.
    Status201_TheCreatedShelf(models::Shelf),
    /// Invalid request. `kind` is `InvalidRequest`, or `UnsupportedType` when the target cannot be opened as an archive, or `LimitExceeded` when an archive exceeds an entry, size, or nesting limit.
    Status400_InvalidRequest(models::ErrorResponse),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum DeleteFolderSettingResponse {
    /// The settings were removed.
    Status204_TheSettingsWereRemoved,
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
pub enum DeleteShelfResponse {
    /// The Shelf was deleted.
    Status204_TheShelfWasDeleted,
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
pub enum ListFolderSettingsResponse {
    /// The folder settings.
    Status200_TheFolderSettings(models::FolderSettingList),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum ListShelvesResponse {
    /// The Shelves.
    Status200_TheShelves(models::ShelfList),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum PutFolderSettingResponse {
    /// The saved settings.
    Status200_TheSavedSettings(models::FolderSetting),
    /// Invalid request. `kind` is `InvalidRequest`, or `UnsupportedType` when the target cannot be opened as an archive, or `LimitExceeded` when an archive exceeds an entry, size, or nesting limit.
    Status400_InvalidRequest(models::ErrorResponse),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum ReorderShelvesResponse {
    /// The Shelves in the new order.
    Status200_TheShelvesInTheNewOrder(models::ShelfList),
    /// Invalid request. `kind` is `InvalidRequest`, or `UnsupportedType` when the target cannot be opened as an archive, or `LimitExceeded` when an archive exceeds an entry, size, or nesting limit.
    Status400_InvalidRequest(models::ErrorResponse),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum ReplaceShelfResponse {
    /// The saved Shelf.
    Status200_TheSavedShelf(models::Shelf),
    /// Invalid request. `kind` is `InvalidRequest`, or `UnsupportedType` when the target cannot be opened as an archive, or `LimitExceeded` when an archive exceeds an entry, size, or nesting limit.
    Status400_InvalidRequest(models::ErrorResponse),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::ErrorResponse),
    /// Resource not found.
    Status404_ResourceNotFound(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
}

/// Shelves
#[async_trait]
#[allow(clippy::ptr_arg)]
pub trait Shelves<E: std::fmt::Debug + Send + Sync + 'static = ()>: super::ErrorHandler<E> {
    type Claims;

    /// Create a Shelf.
    ///
    /// CreateShelf - POST /api/v1/shelves
    async fn create_shelf(&self, method: &Method, host: &Host, cookies: &CookieJar, claims: &Self::Claims, body: &models::ShelfInput) -> Result<CreateShelfResponse, E>;

    /// Remove the settings of a folder.
    ///
    /// DeleteFolderSetting - POST /api/v1/settings/folder/delete
    async fn delete_folder_setting(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        body: &models::LibraryPath,
    ) -> Result<DeleteFolderSettingResponse, E>;

    /// Delete a Shelf.
    ///
    /// DeleteShelf - DELETE /api/v1/shelves/{shelfId}
    async fn delete_shelf(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::DeleteShelfPathParams,
    ) -> Result<DeleteShelfResponse, E>;

    /// List the folder settings.
    ///
    /// ListFolderSettings - GET /api/v1/settings/folder
    async fn list_folder_settings(&self, method: &Method, host: &Host, cookies: &CookieJar, claims: &Self::Claims) -> Result<ListFolderSettingsResponse, E>;

    /// List the Shelves.
    ///
    /// ListShelves - GET /api/v1/shelves
    async fn list_shelves(&self, method: &Method, host: &Host, cookies: &CookieJar, claims: &Self::Claims) -> Result<ListShelvesResponse, E>;

    /// Save the settings of a folder.
    ///
    /// PutFolderSetting - PUT /api/v1/settings/folder
    async fn put_folder_setting(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        body: &models::FolderSetting,
    ) -> Result<PutFolderSettingResponse, E>;

    /// Reorder the Shelves.
    ///
    /// ReorderShelves - PUT /api/v1/shelves/order
    async fn reorder_shelves(&self, method: &Method, host: &Host, cookies: &CookieJar, claims: &Self::Claims, body: &models::ShelfOrder) -> Result<ReorderShelvesResponse, E>;

    /// Replace a Shelf.
    ///
    /// ReplaceShelf - PUT /api/v1/shelves/{shelfId}
    async fn replace_shelf(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::ReplaceShelfPathParams,
        body: &models::ShelfInput,
    ) -> Result<ReplaceShelfResponse, E>;
}
