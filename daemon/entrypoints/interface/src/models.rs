#![allow(unused_qualifications)]

use http::HeaderValue;
use validator::Validate;

#[cfg(feature = "server")]
use crate::header;
use crate::{models, types::*};

#[allow(dead_code)]
fn from_validation_error(e: validator::ValidationError) -> validator::ValidationErrors {
    let mut errs = validator::ValidationErrors::new();
    errs.add("na", e);
    errs
}

#[allow(dead_code)]
pub fn check_xss_string(v: &str) -> std::result::Result<(), validator::ValidationError> {
    if ammonia::is_html(v) {
        std::result::Result::Err(validator::ValidationError::new("xss detected"))
    } else {
        std::result::Result::Ok(())
    }
}

#[allow(dead_code)]
pub fn check_xss_vec_string(v: &[String]) -> std::result::Result<(), validator::ValidationError> {
    if v.iter().any(|i| ammonia::is_html(i)) {
        std::result::Result::Err(validator::ValidationError::new("xss detected"))
    } else {
        std::result::Result::Ok(())
    }
}

#[allow(dead_code)]
pub fn check_xss_map_string(v: &std::collections::HashMap<String, String>) -> std::result::Result<(), validator::ValidationError> {
    if v.keys().any(|k| ammonia::is_html(k)) || v.values().any(|v| ammonia::is_html(v)) {
        std::result::Result::Err(validator::ValidationError::new("xss detected"))
    } else {
        std::result::Result::Ok(())
    }
}

#[allow(dead_code)]
pub fn check_xss_map_nested<T>(v: &std::collections::HashMap<String, T>) -> std::result::Result<(), validator::ValidationError>
where
    T: validator::Validate,
{
    if v.keys().any(|k| ammonia::is_html(k)) || v.values().any(|v| v.validate().is_err()) {
        std::result::Result::Err(validator::ValidationError::new("xss detected"))
    } else {
        std::result::Result::Ok(())
    }
}

#[allow(dead_code)]
pub fn check_xss_map<T>(v: &std::collections::HashMap<String, T>) -> std::result::Result<(), validator::ValidationError> {
    if v.keys().any(|k| ammonia::is_html(k)) {
        std::result::Result::Err(validator::ValidationError::new("xss detected"))
    } else {
        std::result::Result::Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct CopyFilesPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_COPYFILESPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
}

lazy_static::lazy_static! {
    static ref RE_COPYFILESPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct DeleteFilesPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_DELETEFILESPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
}

lazy_static::lazy_static! {
    static ref RE_DELETEFILESPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct GetFilePathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_GETFILEPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
}

lazy_static::lazy_static! {
    static ref RE_GETFILEPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct GetFileQueryParams {
    /// Path relative to the selected Library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096))]
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct MoveFilesPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_MOVEFILESPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
}

lazy_static::lazy_static! {
    static ref RE_MOVEFILESPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ReadFileContentHeaderParams {
    pub range: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ReadFileContentPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_READFILECONTENTPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
}

lazy_static::lazy_static! {
    static ref RE_READFILECONTENTPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ReadFileContentQueryParams {
    /// Path relative to the selected Library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096))]
    pub path: String,
    /// Byte offset used when `Range` is not supplied.
    #[serde(rename = "offset")]
    #[validate(range(min = 0u64))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
    /// Maximum bytes in an explicit read window. Defaults to 1 MiB only when offset is supplied.
    #[serde(rename = "limit")]
    #[validate(range(min = 1u64, max = 134217728u64))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct WriteFileContentPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_WRITEFILECONTENTPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
}

lazy_static::lazy_static! {
    static ref RE_WRITEFILECONTENTPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct WriteFileContentQueryParams {
    /// Path relative to the selected Library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096))]
    pub path: String,
    /// How to resolve an existing entry at the target path.
    #[serde(rename = "conflict")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict: Option<models::ConflictPolicy>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct CopyFoldersPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_COPYFOLDERSPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
}

lazy_static::lazy_static! {
    static ref RE_COPYFOLDERSPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct CreateFolderPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_CREATEFOLDERPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
}

lazy_static::lazy_static! {
    static ref RE_CREATEFOLDERPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct DeleteFoldersPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_DELETEFOLDERSPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
}

lazy_static::lazy_static! {
    static ref RE_DELETEFOLDERSPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct GetEntryDetailsPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_GETENTRYDETAILSPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
}

lazy_static::lazy_static! {
    static ref RE_GETENTRYDETAILSPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct GetFolderPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_GETFOLDERPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
}

lazy_static::lazy_static! {
    static ref RE_GETFOLDERPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct GetFolderQueryParams {
    /// Path relative to the selected Library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096))]
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ListFolderEntriesPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_LISTFOLDERENTRIESPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
}

lazy_static::lazy_static! {
    static ref RE_LISTFOLDERENTRIESPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ListFolderEntriesQueryParams {
    /// Path relative to the selected Library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096))]
    pub path: String,
    /// Whether to include descendants below the requested folder.
    #[serde(rename = "recursive")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recursive: Option<bool>,
    /// Offset in the sorted and filtered listing.
    #[serde(rename = "offset")]
    #[validate(range(min = 0u64))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
    /// Whitespace-separated terms that must all occur in an entry name. Names and terms are normalized with NFKC and converted to lowercase.
    #[serde(rename = "query")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    /// Maximum number of entries to return.
    #[serde(rename = "limit")]
    #[validate(range(min = 1u32, max = 1000u32))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Sort key for folder entries.
    /// Note: inline enums are not fully supported by openapi-generator
    #[serde(rename = "sort")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
    /// Sort direction.
    /// Note: inline enums are not fully supported by openapi-generator
    #[serde(rename = "order")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<String>,
    /// Whether hidden entries should be returned.
    #[serde(rename = "includeHidden")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_hidden: Option<bool>,
    /// Place folders, and files whose `file.archive` hint is true, before the other entries. `sort` and `order` apply within each of the two groups.
    #[serde(rename = "foldersFirst")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folders_first: Option<bool>,
    /// Return only folders and files whose `file.archive` hint is true. `totalCount` counts the entries after this filter.
    #[serde(rename = "foldersOnly")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folders_only: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ListFolderSummariesPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_LISTFOLDERSUMMARIESPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
}

lazy_static::lazy_static! {
    static ref RE_LISTFOLDERSUMMARIESPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ListFolderSummariesQueryParams {
    /// Path relative to the selected Library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096))]
    pub path: String,
    /// Offset in the sorted and filtered listing.
    #[serde(rename = "offset")]
    #[validate(range(min = 0u64))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
    /// Whitespace-separated terms that must all occur in an entry name. Names and terms are normalized with NFKC and converted to lowercase.
    #[serde(rename = "query")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    /// Maximum number of entries to return.
    #[serde(rename = "limit")]
    #[validate(range(min = 1u32, max = 1000u32))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Note: inline enums are not fully supported by openapi-generator
    #[serde(rename = "sort")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
    /// Sort direction.
    /// Note: inline enums are not fully supported by openapi-generator
    #[serde(rename = "order")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<String>,
    /// Whether hidden entries should be returned.
    #[serde(rename = "includeHidden")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_hidden: Option<bool>,
    /// Place folders, and files whose `file.archive` hint is true, before the other entries. `sort` and `order` apply within each of the two groups.
    #[serde(rename = "foldersFirst")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folders_first: Option<bool>,
    /// Return only folders and files whose `file.archive` hint is true. `totalCount` counts the entries after this filter.
    #[serde(rename = "foldersOnly")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folders_only: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct MoveFoldersPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_MOVEFOLDERSPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
}

lazy_static::lazy_static! {
    static ref RE_MOVEFOLDERSPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ProbeFolderChildrenPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_PROBEFOLDERCHILDRENPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
}

lazy_static::lazy_static! {
    static ref RE_PROBEFOLDERCHILDRENPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ProbeFolderChildrenQueryParams {
    /// Path relative to the selected Library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096))]
    pub path: String,
    /// Whether hidden entries should be returned.
    #[serde(rename = "includeHidden")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_hidden: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct GetLibraryPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_GETLIBRARYPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
}

lazy_static::lazy_static! {
    static ref RE_GETLIBRARYPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct CancelOperationPathParams {
    /// Opaque identifier of a long-running operation.
    #[validate(length(min = 1))]
    pub operation_id: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct GetOperationPathParams {
    /// Opaque identifier of a long-running operation.
    #[validate(length(min = 1))]
    pub operation_id: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct GetRatingPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_GETRATINGPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
}

lazy_static::lazy_static! {
    static ref RE_GETRATINGPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct GetRatingQueryParams {
    /// Path relative to the selected Library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096))]
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct PutRatingPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_PUTRATINGPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
}

lazy_static::lazy_static! {
    static ref RE_PUTRATINGPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ListSearchEntriesQueryParams {
    /// Libraries to search, repeated as `libraryIds=a&libraryIds=b`. Every id must name a Library; an unknown id is 404.
    #[serde(rename = "libraryIds")]
    #[validate(length(min = 1, max = 64))]
    pub library_ids: Vec<models::LibraryId>,
    /// Start relative to the Library. Only with one Library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Whether to include descendants below the requested folder.
    #[serde(rename = "recursive")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recursive: Option<bool>,
    /// Whitespace-separated terms that must all occur in an entry name. Names and terms are normalized with NFKC and converted to lowercase.
    #[serde(rename = "query")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    /// Return only images and videos with this rating of the current user. `none` selects the images and videos that have not been rated. Folders and other files never match. `totalCount` counts the entries after this filter.
    /// Note: inline enums are not fully supported by openapi-generator
    #[serde(rename = "rating")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<String>,
    /// Return only images and videos that have all of these tags of the current user, repeated as `tag=a&tag=b`. Each word is trimmed like a tag being set; a word the user has never used selects nothing. `totalCount` counts the entries after this filter.
    #[serde(rename = "tag")]
    #[validate(length(max = 32))]
    #[serde(default)]
    pub tag: Vec<models::TagName>,
    /// Sort key for folder entries.
    /// Note: inline enums are not fully supported by openapi-generator
    #[serde(rename = "sort")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
    /// Sort direction.
    /// Note: inline enums are not fully supported by openapi-generator
    #[serde(rename = "order")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<String>,
    /// Place folders, and files whose `file.archive` hint is true, before the other entries. `sort` and `order` apply within each of the two groups.
    #[serde(rename = "foldersFirst")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folders_first: Option<bool>,
    /// Offset in the sorted and filtered listing.
    #[serde(rename = "offset")]
    #[validate(range(min = 0u64))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
    /// Maximum number of entries to return.
    #[serde(rename = "limit")]
    #[validate(range(min = 1u32, max = 1000u32))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct GetEntryTagsPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_GETENTRYTAGSPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
}

lazy_static::lazy_static! {
    static ref RE_GETENTRYTAGSPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct GetEntryTagsQueryParams {
    /// Path relative to the selected Library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096))]
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct PutEntryTagsPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_PUTENTRYTAGSPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
}

lazy_static::lazy_static! {
    static ref RE_PUTENTRYTAGSPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct GetThumbnailPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_GETTHUMBNAILPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
}

lazy_static::lazy_static! {
    static ref RE_GETTHUMBNAILPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct GetThumbnailQueryParams {
    /// Path relative to the selected Library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096))]
    pub path: String,
    /// What to generate. See the operation description.
    #[serde(rename = "variant")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<models::ThumbnailVariant>,
    /// Requested thumbnail width in pixels.
    #[serde(rename = "width")]
    #[validate(range(min = 1u32, max = 4096u32))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// Requested thumbnail height in pixels.
    #[serde(rename = "height")]
    #[validate(range(min = 1u32, max = 4096u32))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    /// Thumbnail image format. When omitted, the server uses the format set by `[thumbnail] format` (`webp` by default).
    #[serde(rename = "format")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<models::ImageFormat>,
    /// Resize behavior. `fit` keeps the aspect ratio, fits the image inside the box, and never enlarges it. `pad` fills the box with transparent margins, so it cannot be combined with `format=jpeg`.
    #[serde(rename = "resize")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resize: Option<models::ImageResize>,
    /// Image resampler.
    #[serde(rename = "resampler")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resampler: Option<models::ImageResampler>,
    /// Anchor position used when resizing and cropping.
    #[serde(rename = "anchor")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<models::ImageAnchor>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct GetTimelineStatusQueryParams {
    /// Libraries to list, repeated as `libraryIds=a&libraryIds=b`. Every id must name a Library; an unknown id is 404.
    #[serde(rename = "libraryIds")]
    #[validate(length(min = 1, max = 64))]
    pub library_ids: Vec<models::LibraryId>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ListTimelineEntriesQueryParams {
    /// Libraries to list, repeated as `libraryIds=a&libraryIds=b`. Every id must name a Library; an unknown id is 404.
    #[serde(rename = "libraryIds")]
    #[validate(length(min = 1, max = 64))]
    pub library_ids: Vec<models::LibraryId>,
    /// Cursor from `next`. Returns the entries older than the cursor.
    #[serde(rename = "after")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    /// Cursor from `previous`. Returns the entries newer than the cursor.
    #[serde(rename = "before")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
    /// Starts at the newest entry of this month (`YYYY-MM`), or of the next older month that has entries.
    #[serde(rename = "month")]
    #[validate(
                          regex(path = *RE_LISTTIMELINEENTRIESQUERYPARAMS_MONTH),
              )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub month: Option<String>,
    /// Returns a page that has this entry of `aroundLibraryId` near its middle.
    #[serde(rename = "around")]
    #[validate(length(min = 1, max = 4096))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub around: Option<String>,
    /// The Library of `around`. Given exactly when `around` is given.
    #[serde(rename = "aroundLibraryId")]
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_LISTTIMELINEENTRIESQUERYPARAMS_AROUND_LIBRARY_ID),
              )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub around_library_id: Option<String>,
    /// Maximum number of entries to return.
    #[serde(rename = "limit")]
    #[validate(range(min = 1u32, max = 1000u32))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

lazy_static::lazy_static! {
    static ref RE_LISTTIMELINEENTRIESQUERYPARAMS_MONTH: regex::Regex = regex::Regex::new("^[0-9]{4}-[0-9]{2}$").unwrap();
}
lazy_static::lazy_static! {
    static ref RE_LISTTIMELINEENTRIESQUERYPARAMS_AROUND_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ListTimelineMonthsQueryParams {
    /// Libraries to list, repeated as `libraryIds=a&libraryIds=b`. Every id must name a Library; an unknown id is 404.
    #[serde(rename = "libraryIds")]
    #[validate(length(min = 1, max = 64))]
    pub library_ids: Vec<models::LibraryId>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct CommitUploadPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_COMMITUPLOADPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
    /// Opaque identifier of an upload session.
    #[validate(length(min = 1))]
    pub upload_id: String,
}

lazy_static::lazy_static! {
    static ref RE_COMMITUPLOADPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct CreateUploadPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_CREATEUPLOADPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
}

lazy_static::lazy_static! {
    static ref RE_CREATEUPLOADPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct DeleteUploadPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_DELETEUPLOADPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
    /// Opaque identifier of an upload session.
    #[validate(length(min = 1))]
    pub upload_id: String,
}

lazy_static::lazy_static! {
    static ref RE_DELETEUPLOADPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct GetUploadPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_GETUPLOADPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
    /// Opaque identifier of an upload session.
    #[validate(length(min = 1))]
    pub upload_id: String,
}

lazy_static::lazy_static! {
    static ref RE_GETUPLOADPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct WriteUploadContentPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_WRITEUPLOADCONTENTPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
    /// Opaque identifier of an upload session.
    #[validate(length(min = 1))]
    pub upload_id: String,
}

lazy_static::lazy_static! {
    static ref RE_WRITEUPLOADCONTENTPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct WriteUploadContentQueryParams {
    /// Byte offset of this chunk. Must equal the session's current `committedSize`.
    #[serde(rename = "offset")]
    #[validate(range(min = 0u64))]
    pub offset: u64,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct CreateVideoSessionPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_CREATEVIDEOSESSIONPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
}

lazy_static::lazy_static! {
    static ref RE_CREATEVIDEOSESSIONPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct DeleteVideoSessionPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_DELETEVIDEOSESSIONPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
    #[validate(length(min = 1))]
    pub session_id: String,
}

lazy_static::lazy_static! {
    static ref RE_DELETEVIDEOSESSIONPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct GetMediaInfoPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_GETMEDIAINFOPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
}

lazy_static::lazy_static! {
    static ref RE_GETMEDIAINFOPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct GetMediaInfoQueryParams {
    /// Path relative to the selected Library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096))]
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct GetVideoPlaylistPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_GETVIDEOPLAYLISTPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
    #[validate(length(min = 1))]
    pub session_id: String,
}

lazy_static::lazy_static! {
    static ref RE_GETVIDEOPLAYLISTPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct GetVideoSegmentPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_GETVIDEOSEGMENTPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
    #[validate(length(min = 1))]
    pub session_id: String,
    #[validate(
                          regex(path = *RE_GETVIDEOSEGMENTPATHPARAMS_SEGMENT_NAME),
            )]
    pub segment_name: String,
}

lazy_static::lazy_static! {
    static ref RE_GETVIDEOSEGMENTPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}
lazy_static::lazy_static! {
    static ref RE_GETVIDEOSEGMENTPATHPARAMS_SEGMENT_NAME: regex::Regex = regex::Regex::new("^[0-9]+\\.ts$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct AuthResult {
    #[serde(rename = "user")]
    #[validate(nested)]
    pub user: models::User,

    #[serde(rename = "session")]
    #[validate(nested)]
    pub session: models::AuthSession,

    /// Opaque token returned only for `sessionTransport=bearer`. Browser cookie sessions omit this property.
    #[serde(rename = "accessToken")]
    #[validate(length(min = 1), custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
}

impl AuthResult {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(user: models::User, session: models::AuthSession) -> AuthResult {
        AuthResult {
            user,
            session,
            access_token: None,
        }
    }
}

/// Converts the AuthResult value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for AuthResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping user in query parameter serialization

            // Skipping session in query parameter serialization
            self.access_token
                .as_ref()
                .map(|access_token| ["accessToken".to_string(), access_token.to_string()].join(",")),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a AuthResult value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for AuthResult {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub user: Vec<models::User>,
            pub session: Vec<models::AuthSession>,
            pub access_token: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing AuthResult".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "user" => intermediate_rep.user.push(<models::User as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "session" => intermediate_rep
                        .session
                        .push(<models::AuthSession as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "accessToken" => intermediate_rep.access_token.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing AuthResult".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(AuthResult {
            user: intermediate_rep.user.into_iter().next().ok_or_else(|| "user missing in AuthResult".to_string())?,
            session: intermediate_rep.session.into_iter().next().ok_or_else(|| "session missing in AuthResult".to_string())?,
            access_token: intermediate_rep.access_token.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<AuthResult> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<AuthResult>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<AuthResult>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for AuthResult - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<AuthResult> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <AuthResult as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into AuthResult - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct AuthSession {
    /// Opaque daemon-issued session identifier.
    #[serde(rename = "id")]
    #[validate(length(min = 1), custom(function = "check_xss_string"))]
    pub id: String,

    /// Opaque daemon-issued user identifier.
    #[serde(rename = "userId")]
    #[validate(length(min = 1), custom(function = "check_xss_string"))]
    pub user_id: String,

    #[serde(rename = "status")]
    #[validate(nested)]
    pub status: models::SessionStatus,

    #[serde(rename = "transport")]
    #[validate(nested)]
    pub transport: models::SessionTransport,

    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,

    #[serde(rename = "expiresAt")]
    pub expires_at: chrono::DateTime<chrono::Utc>,

    #[serde(rename = "lastSeenAt")]
    #[serde(deserialize_with = "deserialize_optional_nullable")]
    #[serde(default = "default_optional_nullable")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_seen_at: Option<Nullable<chrono::DateTime<chrono::Utc>>>,
}

impl AuthSession {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(
        id: String,
        user_id: String,
        status: models::SessionStatus,
        transport: models::SessionTransport,
        created_at: chrono::DateTime<chrono::Utc>,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> AuthSession {
        AuthSession {
            id,
            user_id,
            status,
            transport,
            created_at,
            expires_at,
            last_seen_at: None,
        }
    }
}

/// Converts the AuthSession value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for AuthSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("id".to_string()),
            Some(self.id.to_string()),
            Some("userId".to_string()),
            Some(self.user_id.to_string()),
            // Skipping status in query parameter serialization

            // Skipping transport in query parameter serialization

            // Skipping createdAt in query parameter serialization

            // Skipping expiresAt in query parameter serialization

            // Skipping lastSeenAt in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a AuthSession value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for AuthSession {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub id: Vec<String>,
            pub user_id: Vec<String>,
            pub status: Vec<models::SessionStatus>,
            pub transport: Vec<models::SessionTransport>,
            pub created_at: Vec<chrono::DateTime<chrono::Utc>>,
            pub expires_at: Vec<chrono::DateTime<chrono::Utc>>,
            pub last_seen_at: Vec<chrono::DateTime<chrono::Utc>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing AuthSession".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "id" => intermediate_rep.id.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "userId" => intermediate_rep.user_id.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "status" => intermediate_rep
                        .status
                        .push(<models::SessionStatus as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "transport" => intermediate_rep
                        .transport
                        .push(<models::SessionTransport as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "createdAt" => intermediate_rep
                        .created_at
                        .push(<chrono::DateTime<chrono::Utc> as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "expiresAt" => intermediate_rep
                        .expires_at
                        .push(<chrono::DateTime<chrono::Utc> as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    "lastSeenAt" => return std::result::Result::Err("Parsing a nullable type in this style is not supported in AuthSession".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing AuthSession".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(AuthSession {
            id: intermediate_rep.id.into_iter().next().ok_or_else(|| "id missing in AuthSession".to_string())?,
            user_id: intermediate_rep.user_id.into_iter().next().ok_or_else(|| "userId missing in AuthSession".to_string())?,
            status: intermediate_rep.status.into_iter().next().ok_or_else(|| "status missing in AuthSession".to_string())?,
            transport: intermediate_rep
                .transport
                .into_iter()
                .next()
                .ok_or_else(|| "transport missing in AuthSession".to_string())?,
            created_at: intermediate_rep
                .created_at
                .into_iter()
                .next()
                .ok_or_else(|| "createdAt missing in AuthSession".to_string())?,
            expires_at: intermediate_rep
                .expires_at
                .into_iter()
                .next()
                .ok_or_else(|| "expiresAt missing in AuthSession".to_string())?,
            last_seen_at: std::result::Result::Err("Nullable types not supported in AuthSession".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<AuthSession> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<AuthSession>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<AuthSession>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for AuthSession - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<AuthSession> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <AuthSession as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into AuthSession - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct Capabilities {
    #[serde(rename = "summaryListing")]
    pub summary_listing: bool,

    #[serde(rename = "entryDetails")]
    pub entry_details: bool,

    #[serde(rename = "childProbes")]
    pub child_probes: bool,
}

impl Capabilities {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(summary_listing: bool, entry_details: bool, child_probes: bool) -> Capabilities {
        Capabilities {
            summary_listing,
            entry_details,
            child_probes,
        }
    }
}

/// Converts the Capabilities value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for Capabilities {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("summaryListing".to_string()),
            Some(self.summary_listing.to_string()),
            Some("entryDetails".to_string()),
            Some(self.entry_details.to_string()),
            Some("childProbes".to_string()),
            Some(self.child_probes.to_string()),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a Capabilities value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for Capabilities {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub summary_listing: Vec<bool>,
            pub entry_details: Vec<bool>,
            pub child_probes: Vec<bool>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing Capabilities".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "summaryListing" => intermediate_rep
                        .summary_listing
                        .push(<bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "entryDetails" => intermediate_rep.entry_details.push(<bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "childProbes" => intermediate_rep.child_probes.push(<bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing Capabilities".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(Capabilities {
            summary_listing: intermediate_rep
                .summary_listing
                .into_iter()
                .next()
                .ok_or_else(|| "summaryListing missing in Capabilities".to_string())?,
            entry_details: intermediate_rep
                .entry_details
                .into_iter()
                .next()
                .ok_or_else(|| "entryDetails missing in Capabilities".to_string())?,
            child_probes: intermediate_rep
                .child_probes
                .into_iter()
                .next()
                .ok_or_else(|| "childProbes missing in Capabilities".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<Capabilities> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<Capabilities>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<Capabilities>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for Capabilities - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<Capabilities> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <Capabilities as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into Capabilities - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// How to resolve an entry that already exists at the target path. `fail` rejects with `409`. `skip` leaves the existing entry untouched and reports the source as skipped. `overwrite` replaces the existing entry. `rename` writes under an alternative name, which is reported back in the result.
/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types, clippy::large_enum_variant)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk_enum_derive::LabelledGenericEnum))]
pub enum ConflictPolicy {
    #[serde(rename = "fail")]
    Fail,
    #[serde(rename = "skip")]
    Skip,
    #[serde(rename = "overwrite")]
    Overwrite,
    #[serde(rename = "rename")]
    Rename,
}

impl validator::Validate for ConflictPolicy {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for ConflictPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            ConflictPolicy::Fail => write!(f, "fail"),
            ConflictPolicy::Skip => write!(f, "skip"),
            ConflictPolicy::Overwrite => write!(f, "overwrite"),
            ConflictPolicy::Rename => write!(f, "rename"),
        }
    }
}

impl std::str::FromStr for ConflictPolicy {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "fail" => std::result::Result::Ok(ConflictPolicy::Fail),
            "skip" => std::result::Result::Ok(ConflictPolicy::Skip),
            "overwrite" => std::result::Result::Ok(ConflictPolicy::Overwrite),
            "rename" => std::result::Result::Ok(ConflictPolicy::Rename),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct CreateFolderRequest {
    /// Absolute-looking path within the selected Library. The resolved path must never escape that library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096), custom(function = "check_xss_string"))]
    pub path: String,

    /// Create missing intermediate folders as well.
    #[serde(rename = "createParents")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_parents: Option<bool>,
}

impl CreateFolderRequest {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(path: String) -> CreateFolderRequest {
        CreateFolderRequest {
            path,
            create_parents: Some(false),
        }
    }
}

/// Converts the CreateFolderRequest value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for CreateFolderRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("path".to_string()),
            Some(self.path.to_string()),
            self.create_parents
                .as_ref()
                .map(|create_parents| ["createParents".to_string(), create_parents.to_string()].join(",")),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a CreateFolderRequest value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for CreateFolderRequest {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub path: Vec<String>,
            pub create_parents: Vec<bool>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing CreateFolderRequest".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "path" => intermediate_rep.path.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "createParents" => intermediate_rep.create_parents.push(<bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing CreateFolderRequest".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(CreateFolderRequest {
            path: intermediate_rep.path.into_iter().next().ok_or_else(|| "path missing in CreateFolderRequest".to_string())?,
            create_parents: intermediate_rep.create_parents.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<CreateFolderRequest> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<CreateFolderRequest>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<CreateFolderRequest>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for CreateFolderRequest - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<CreateFolderRequest> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <CreateFolderRequest as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into CreateFolderRequest - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct CurrentAuth {
    #[serde(rename = "user")]
    #[validate(nested)]
    pub user: models::User,

    #[serde(rename = "session")]
    #[validate(nested)]
    pub session: models::AuthSession,
}

impl CurrentAuth {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(user: models::User, session: models::AuthSession) -> CurrentAuth {
        CurrentAuth { user, session }
    }
}

/// Converts the CurrentAuth value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for CurrentAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping user in query parameter serialization

            // Skipping session in query parameter serialization

        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a CurrentAuth value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for CurrentAuth {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub user: Vec<models::User>,
            pub session: Vec<models::AuthSession>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing CurrentAuth".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "user" => intermediate_rep.user.push(<models::User as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "session" => intermediate_rep
                        .session
                        .push(<models::AuthSession as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing CurrentAuth".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(CurrentAuth {
            user: intermediate_rep.user.into_iter().next().ok_or_else(|| "user missing in CurrentAuth".to_string())?,
            session: intermediate_rep.session.into_iter().next().ok_or_else(|| "session missing in CurrentAuth".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<CurrentAuth> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<CurrentAuth>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<CurrentAuth>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for CurrentAuth - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<CurrentAuth> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <CurrentAuth as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into CurrentAuth - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct DeleteFilesRequest {
    #[serde(rename = "srcPaths")]
    #[validate(length(min = 1, max = 1000), nested)]
    pub src_paths: Vec<models::RelativePath>,
}

impl DeleteFilesRequest {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(src_paths: Vec<models::RelativePath>) -> DeleteFilesRequest {
        DeleteFilesRequest { src_paths }
    }
}

/// Converts the DeleteFilesRequest value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for DeleteFilesRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("srcPaths".to_string()),
            Some(self.src_paths.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a DeleteFilesRequest value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for DeleteFilesRequest {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub src_paths: Vec<Vec<models::RelativePath>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing DeleteFilesRequest".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "srcPaths" => return std::result::Result::Err("Parsing a container in this style is not supported in DeleteFilesRequest".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing DeleteFilesRequest".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(DeleteFilesRequest {
            src_paths: intermediate_rep
                .src_paths
                .into_iter()
                .next()
                .ok_or_else(|| "srcPaths missing in DeleteFilesRequest".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<DeleteFilesRequest> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<DeleteFilesRequest>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<DeleteFilesRequest>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for DeleteFilesRequest - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<DeleteFilesRequest> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <DeleteFilesRequest as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into DeleteFilesRequest - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct DeleteFoldersRequest {
    #[serde(rename = "srcPaths")]
    #[validate(length(min = 1, max = 1000), nested)]
    pub src_paths: Vec<models::RelativePath>,

    /// Delete non-empty folders and everything below them.
    #[serde(rename = "recursive")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recursive: Option<bool>,
}

impl DeleteFoldersRequest {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(src_paths: Vec<models::RelativePath>) -> DeleteFoldersRequest {
        DeleteFoldersRequest {
            src_paths,
            recursive: Some(false),
        }
    }
}

/// Converts the DeleteFoldersRequest value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for DeleteFoldersRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("srcPaths".to_string()),
            Some(self.src_paths.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")),
            self.recursive.as_ref().map(|recursive| ["recursive".to_string(), recursive.to_string()].join(",")),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a DeleteFoldersRequest value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for DeleteFoldersRequest {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub src_paths: Vec<Vec<models::RelativePath>>,
            pub recursive: Vec<bool>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing DeleteFoldersRequest".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "srcPaths" => return std::result::Result::Err("Parsing a container in this style is not supported in DeleteFoldersRequest".to_string()),
                    #[allow(clippy::redundant_clone)]
                    "recursive" => intermediate_rep.recursive.push(<bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing DeleteFoldersRequest".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(DeleteFoldersRequest {
            src_paths: intermediate_rep
                .src_paths
                .into_iter()
                .next()
                .ok_or_else(|| "srcPaths missing in DeleteFoldersRequest".to_string())?,
            recursive: intermediate_rep.recursive.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<DeleteFoldersRequest> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<DeleteFoldersRequest>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<DeleteFoldersRequest>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for DeleteFoldersRequest - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<DeleteFoldersRequest> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <DeleteFoldersRequest as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into DeleteFoldersRequest - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct DisplaySet {
    /// Tile size step from 1 (smallest) to 5 (largest).
    #[serde(rename = "tileSize")]
    #[validate(range(min = 1u32, max = 5u32))]
    pub tile_size: u32,

    #[serde(rename = "thumbnailShape")]
    #[validate(nested)]
    pub thumbnail_shape: models::ThumbnailShape,

    #[serde(rename = "view")]
    #[validate(nested)]
    pub view: models::ListView,

    #[serde(rename = "originalImage")]
    #[validate(nested)]
    pub original_image: models::OriginalImageMode,
}

impl DisplaySet {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(tile_size: u32, thumbnail_shape: models::ThumbnailShape, view: models::ListView, original_image: models::OriginalImageMode) -> DisplaySet {
        DisplaySet {
            tile_size,
            thumbnail_shape,
            view,
            original_image,
        }
    }
}

/// Converts the DisplaySet value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for DisplaySet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("tileSize".to_string()),
            Some(self.tile_size.to_string()),
            // Skipping thumbnailShape in query parameter serialization

            // Skipping view in query parameter serialization

            // Skipping originalImage in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a DisplaySet value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for DisplaySet {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub tile_size: Vec<u32>,
            pub thumbnail_shape: Vec<models::ThumbnailShape>,
            pub view: Vec<models::ListView>,
            pub original_image: Vec<models::OriginalImageMode>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing DisplaySet".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "tileSize" => intermediate_rep.tile_size.push(<u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "thumbnailShape" => intermediate_rep
                        .thumbnail_shape
                        .push(<models::ThumbnailShape as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "view" => intermediate_rep
                        .view
                        .push(<models::ListView as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "originalImage" => intermediate_rep
                        .original_image
                        .push(<models::OriginalImageMode as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing DisplaySet".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(DisplaySet {
            tile_size: intermediate_rep.tile_size.into_iter().next().ok_or_else(|| "tileSize missing in DisplaySet".to_string())?,
            thumbnail_shape: intermediate_rep
                .thumbnail_shape
                .into_iter()
                .next()
                .ok_or_else(|| "thumbnailShape missing in DisplaySet".to_string())?,
            view: intermediate_rep.view.into_iter().next().ok_or_else(|| "view missing in DisplaySet".to_string())?,
            original_image: intermediate_rep
                .original_image
                .into_iter()
                .next()
                .ok_or_else(|| "originalImage missing in DisplaySet".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<DisplaySet> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<DisplaySet>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<DisplaySet>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for DisplaySet - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<DisplaySet> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <DisplaySet as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into DisplaySet - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// Exactly one of detail and error is present; no host paths or error messages.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct EntryDetailResult {
    /// Absolute-looking path within the selected Library. The resolved path must never escape that library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096), custom(function = "check_xss_string"))]
    pub path: String,

    #[serde(rename = "detail")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<models::FileSystemEntry>,

    #[serde(rename = "error")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<models::ErrorKind>,
}

impl EntryDetailResult {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(path: String) -> EntryDetailResult {
        EntryDetailResult { path, detail: None, error: None }
    }
}

/// Converts the EntryDetailResult value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for EntryDetailResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("path".to_string()),
            Some(self.path.to_string()),
            // Skipping detail in query parameter serialization

            // Skipping error in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a EntryDetailResult value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for EntryDetailResult {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub path: Vec<String>,
            pub detail: Vec<models::FileSystemEntry>,
            pub error: Vec<models::ErrorKind>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing EntryDetailResult".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "path" => intermediate_rep.path.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "detail" => intermediate_rep
                        .detail
                        .push(<models::FileSystemEntry as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "error" => intermediate_rep
                        .error
                        .push(<models::ErrorKind as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing EntryDetailResult".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(EntryDetailResult {
            path: intermediate_rep.path.into_iter().next().ok_or_else(|| "path missing in EntryDetailResult".to_string())?,
            detail: intermediate_rep.detail.into_iter().next(),
            error: intermediate_rep.error.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<EntryDetailResult> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<EntryDetailResult>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<EntryDetailResult>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for EntryDetailResult - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<EntryDetailResult> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <EntryDetailResult as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into EntryDetailResult - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct EntryDetailsRequest {
    #[serde(rename = "paths")]
    #[validate(length(min = 1, max = 200), nested)]
    pub paths: Vec<models::RelativePath>,
}

impl EntryDetailsRequest {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(paths: Vec<models::RelativePath>) -> EntryDetailsRequest {
        EntryDetailsRequest { paths }
    }
}

/// Converts the EntryDetailsRequest value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for EntryDetailsRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![Some("paths".to_string()), Some(self.paths.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a EntryDetailsRequest value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for EntryDetailsRequest {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub paths: Vec<Vec<models::RelativePath>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing EntryDetailsRequest".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "paths" => return std::result::Result::Err("Parsing a container in this style is not supported in EntryDetailsRequest".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing EntryDetailsRequest".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(EntryDetailsRequest {
            paths: intermediate_rep
                .paths
                .into_iter()
                .next()
                .ok_or_else(|| "paths missing in EntryDetailsRequest".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<EntryDetailsRequest> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<EntryDetailsRequest>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<EntryDetailsRequest>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for EntryDetailsRequest - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<EntryDetailsRequest> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <EntryDetailsRequest as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into EntryDetailsRequest - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct EntryDetailsResponse {
    #[serde(rename = "results")]
    #[validate(nested)]
    pub results: Vec<models::EntryDetailResult>,
}

impl EntryDetailsResponse {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(results: Vec<models::EntryDetailResult>) -> EntryDetailsResponse {
        EntryDetailsResponse { results }
    }
}

/// Converts the EntryDetailsResponse value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for EntryDetailsResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping results in query parameter serialization

        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a EntryDetailsResponse value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for EntryDetailsResponse {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub results: Vec<Vec<models::EntryDetailResult>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing EntryDetailsResponse".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "results" => return std::result::Result::Err("Parsing a container in this style is not supported in EntryDetailsResponse".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing EntryDetailsResponse".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(EntryDetailsResponse {
            results: intermediate_rep
                .results
                .into_iter()
                .next()
                .ok_or_else(|| "results missing in EntryDetailsResponse".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<EntryDetailsResponse> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<EntryDetailsResponse>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<EntryDetailsResponse>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for EntryDetailsResponse - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<EntryDetailsResponse> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <EntryDetailsResponse as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into EntryDetailsResponse - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct EntryList {
    #[serde(rename = "entries")]
    #[validate(nested)]
    pub entries: Vec<models::FileSystemEntry>,
}

impl EntryList {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(entries: Vec<models::FileSystemEntry>) -> EntryList {
        EntryList { entries }
    }
}

/// Converts the EntryList value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for EntryList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping entries in query parameter serialization

        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a EntryList value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for EntryList {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub entries: Vec<Vec<models::FileSystemEntry>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing EntryList".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "entries" => return std::result::Result::Err("Parsing a container in this style is not supported in EntryList".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing EntryList".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(EntryList {
            entries: intermediate_rep.entries.into_iter().next().ok_or_else(|| "entries missing in EntryList".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<EntryList> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<EntryList>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<EntryList>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for EntryList - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<EntryList> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <EntryList as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into EntryList - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct EntryTagsResponse {
    #[serde(rename = "tags")]
    #[validate(nested)]
    pub tags: Vec<models::TagName>,
}

impl EntryTagsResponse {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(tags: Vec<models::TagName>) -> EntryTagsResponse {
        EntryTagsResponse { tags }
    }
}

/// Converts the EntryTagsResponse value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for EntryTagsResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![Some("tags".to_string()), Some(self.tags.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a EntryTagsResponse value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for EntryTagsResponse {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub tags: Vec<Vec<models::TagName>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing EntryTagsResponse".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "tags" => return std::result::Result::Err("Parsing a container in this style is not supported in EntryTagsResponse".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing EntryTagsResponse".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(EntryTagsResponse {
            tags: intermediate_rep.tags.into_iter().next().ok_or_else(|| "tags missing in EntryTagsResponse".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<EntryTagsResponse> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<EntryTagsResponse>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<EntryTagsResponse>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for EntryTagsResponse - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<EntryTagsResponse> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <EntryTagsResponse as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into EntryTagsResponse - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// Copy or move request. Exactly one of `dstPath` and `dstFolderPath` must be provided. `dstPath` names the exact target and is only valid with a single source; this is also how a rename is expressed. `dstFolderPath` names a target folder and keeps each source's name. `dstLibraryId` defaults to the source Library.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct EntryTransferRequest {
    #[serde(rename = "srcPaths")]
    #[validate(length(min = 1, max = 1000), nested)]
    pub src_paths: Vec<models::RelativePath>,

    /// Library identifier chosen by the operator in the daemon configuration file. Unlike the daemon-issued identifiers in this API it is a readable name, not a generated token. It is what the API uses in place of a host path, so the physical location of a Library is never exposed.  Because the value is used as a URL path segment, the character set is restricted. The daemon rejects a configuration that violates it at startup.
    #[serde(rename = "dstLibraryId")]
    #[validate(
            length(min = 1, max = 64),
            regex(path = *RE_ENTRYTRANSFERREQUEST_DST_LIBRARY_ID),
          custom(function = "check_xss_string"),
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dst_library_id: Option<String>,

    /// Absolute-looking path within the selected Library. The resolved path must never escape that library.
    #[serde(rename = "dstPath")]
    #[validate(length(min = 1, max = 4096), custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dst_path: Option<String>,

    /// Absolute-looking path within the selected Library. The resolved path must never escape that library.
    #[serde(rename = "dstFolderPath")]
    #[validate(length(min = 1, max = 4096), custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dst_folder_path: Option<String>,

    #[serde(rename = "conflict")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict: Option<models::ConflictPolicy>,
}

lazy_static::lazy_static! {
    static ref RE_ENTRYTRANSFERREQUEST_DST_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

impl EntryTransferRequest {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(src_paths: Vec<models::RelativePath>) -> EntryTransferRequest {
        EntryTransferRequest {
            src_paths,
            dst_library_id: None,
            dst_path: None,
            dst_folder_path: None,
            conflict: None,
        }
    }
}

/// Converts the EntryTransferRequest value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for EntryTransferRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("srcPaths".to_string()),
            Some(self.src_paths.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")),
            self.dst_library_id
                .as_ref()
                .map(|dst_library_id| ["dstLibraryId".to_string(), dst_library_id.to_string()].join(",")),
            self.dst_path.as_ref().map(|dst_path| ["dstPath".to_string(), dst_path.to_string()].join(",")),
            self.dst_folder_path
                .as_ref()
                .map(|dst_folder_path| ["dstFolderPath".to_string(), dst_folder_path.to_string()].join(",")),
            // Skipping conflict in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a EntryTransferRequest value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for EntryTransferRequest {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub src_paths: Vec<Vec<models::RelativePath>>,
            pub dst_library_id: Vec<String>,
            pub dst_path: Vec<String>,
            pub dst_folder_path: Vec<String>,
            pub conflict: Vec<models::ConflictPolicy>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing EntryTransferRequest".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "srcPaths" => return std::result::Result::Err("Parsing a container in this style is not supported in EntryTransferRequest".to_string()),
                    #[allow(clippy::redundant_clone)]
                    "dstLibraryId" => intermediate_rep
                        .dst_library_id
                        .push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "dstPath" => intermediate_rep.dst_path.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "dstFolderPath" => intermediate_rep
                        .dst_folder_path
                        .push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "conflict" => intermediate_rep
                        .conflict
                        .push(<models::ConflictPolicy as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing EntryTransferRequest".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(EntryTransferRequest {
            src_paths: intermediate_rep
                .src_paths
                .into_iter()
                .next()
                .ok_or_else(|| "srcPaths missing in EntryTransferRequest".to_string())?,
            dst_library_id: intermediate_rep.dst_library_id.into_iter().next(),
            dst_path: intermediate_rep.dst_path.into_iter().next(),
            dst_folder_path: intermediate_rep.dst_folder_path.into_iter().next(),
            conflict: intermediate_rep.conflict.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<EntryTransferRequest> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<EntryTransferRequest>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<EntryTransferRequest>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for EntryTransferRequest - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<EntryTransferRequest> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <EntryTransferRequest as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into EntryTransferRequest - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// `UnsupportedType`, `LimitExceeded`, and `PasswordRequired` describe the target file rather than the request, so a client can tell \"cannot open this format\", \"too large to handle\", and \"password protected\" apart without parsing `message`. `Timeout` means that extracting an archive or generating a thumbnail did not finish within the daemon's time limit. The result is not cached, so repeating the same request can hit the same limit again; no `Retry-After` is returned.
/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types, clippy::large_enum_variant)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk_enum_derive::LabelledGenericEnum))]
pub enum ErrorKind {
    #[serde(rename = "InternalServerError")]
    InternalServerError,
    #[serde(rename = "InvalidRequest")]
    InvalidRequest,
    #[serde(rename = "UnsupportedType")]
    UnsupportedType,
    #[serde(rename = "LimitExceeded")]
    LimitExceeded,
    #[serde(rename = "NotFound")]
    NotFound,
    #[serde(rename = "Unauthorized")]
    Unauthorized,
    #[serde(rename = "Forbidden")]
    Forbidden,
    #[serde(rename = "PasswordRequired")]
    PasswordRequired,
    #[serde(rename = "AlreadyInitialized")]
    AlreadyInitialized,
    #[serde(rename = "RangeNotSatisfiable")]
    RangeNotSatisfiable,
    #[serde(rename = "Conflict")]
    Conflict,
    #[serde(rename = "PayloadTooLarge")]
    PayloadTooLarge,
    #[serde(rename = "InsufficientStorage")]
    InsufficientStorage,
    #[serde(rename = "Timeout")]
    Timeout,
}

impl validator::Validate for ErrorKind {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            ErrorKind::InternalServerError => write!(f, "InternalServerError"),
            ErrorKind::InvalidRequest => write!(f, "InvalidRequest"),
            ErrorKind::UnsupportedType => write!(f, "UnsupportedType"),
            ErrorKind::LimitExceeded => write!(f, "LimitExceeded"),
            ErrorKind::NotFound => write!(f, "NotFound"),
            ErrorKind::Unauthorized => write!(f, "Unauthorized"),
            ErrorKind::Forbidden => write!(f, "Forbidden"),
            ErrorKind::PasswordRequired => write!(f, "PasswordRequired"),
            ErrorKind::AlreadyInitialized => write!(f, "AlreadyInitialized"),
            ErrorKind::RangeNotSatisfiable => write!(f, "RangeNotSatisfiable"),
            ErrorKind::Conflict => write!(f, "Conflict"),
            ErrorKind::PayloadTooLarge => write!(f, "PayloadTooLarge"),
            ErrorKind::InsufficientStorage => write!(f, "InsufficientStorage"),
            ErrorKind::Timeout => write!(f, "Timeout"),
        }
    }
}

impl std::str::FromStr for ErrorKind {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "InternalServerError" => std::result::Result::Ok(ErrorKind::InternalServerError),
            "InvalidRequest" => std::result::Result::Ok(ErrorKind::InvalidRequest),
            "UnsupportedType" => std::result::Result::Ok(ErrorKind::UnsupportedType),
            "LimitExceeded" => std::result::Result::Ok(ErrorKind::LimitExceeded),
            "NotFound" => std::result::Result::Ok(ErrorKind::NotFound),
            "Unauthorized" => std::result::Result::Ok(ErrorKind::Unauthorized),
            "Forbidden" => std::result::Result::Ok(ErrorKind::Forbidden),
            "PasswordRequired" => std::result::Result::Ok(ErrorKind::PasswordRequired),
            "AlreadyInitialized" => std::result::Result::Ok(ErrorKind::AlreadyInitialized),
            "RangeNotSatisfiable" => std::result::Result::Ok(ErrorKind::RangeNotSatisfiable),
            "Conflict" => std::result::Result::Ok(ErrorKind::Conflict),
            "PayloadTooLarge" => std::result::Result::Ok(ErrorKind::PayloadTooLarge),
            "InsufficientStorage" => std::result::Result::Ok(ErrorKind::InsufficientStorage),
            "Timeout" => std::result::Result::Ok(ErrorKind::Timeout),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ErrorResponse {
    #[serde(rename = "kind")]
    #[validate(nested)]
    pub kind: models::ErrorKind,

    #[serde(rename = "message")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// Optional request identifier for server logs.
    #[serde(rename = "requestId")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl ErrorResponse {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(kind: models::ErrorKind) -> ErrorResponse {
        ErrorResponse {
            kind,
            message: None,
            request_id: None,
        }
    }
}

/// Converts the ErrorResponse value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for ErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping kind in query parameter serialization
            self.message.as_ref().map(|message| ["message".to_string(), message.to_string()].join(",")),
            self.request_id.as_ref().map(|request_id| ["requestId".to_string(), request_id.to_string()].join(",")),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a ErrorResponse value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for ErrorResponse {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub kind: Vec<models::ErrorKind>,
            pub message: Vec<String>,
            pub request_id: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing ErrorResponse".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "kind" => intermediate_rep
                        .kind
                        .push(<models::ErrorKind as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "message" => intermediate_rep.message.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "requestId" => intermediate_rep.request_id.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing ErrorResponse".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(ErrorResponse {
            kind: intermediate_rep.kind.into_iter().next().ok_or_else(|| "kind missing in ErrorResponse".to_string())?,
            message: intermediate_rep.message.into_iter().next(),
            request_id: intermediate_rep.request_id.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<ErrorResponse> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<ErrorResponse>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<ErrorResponse>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for ErrorResponse - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<ErrorResponse> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <ErrorResponse as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into ErrorResponse - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct FileResponse {
    #[serde(rename = "file")]
    #[validate(nested)]
    pub file: models::FileSystemEntry,
}

impl FileResponse {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(file: models::FileSystemEntry) -> FileResponse {
        FileResponse { file }
    }
}

/// Converts the FileResponse value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for FileResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping file in query parameter serialization

        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a FileResponse value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for FileResponse {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub file: Vec<models::FileSystemEntry>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing FileResponse".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "file" => intermediate_rep
                        .file
                        .push(<models::FileSystemEntry as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing FileResponse".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(FileResponse {
            file: intermediate_rep.file.into_iter().next().ok_or_else(|| "file missing in FileResponse".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<FileResponse> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<FileResponse>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<FileResponse>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for FileResponse - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<FileResponse> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <FileResponse as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into FileResponse - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct FileSystemEntry {
    /// Absolute-looking path within the selected Library. The resolved path must never escape that library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096), custom(function = "check_xss_string"))]
    pub path: String,

    /// Last path segment.
    #[serde(rename = "name")]
    #[validate(custom(function = "check_xss_string"))]
    pub name: String,

    #[serde(rename = "metadata")]
    #[validate(nested)]
    pub metadata: models::FileSystemMetadata,

    #[serde(rename = "kind")]
    #[validate(nested)]
    pub kind: models::FileSystemEntryKind,

    #[serde(rename = "file")]
    #[serde(deserialize_with = "deserialize_optional_nullable")]
    #[serde(default = "default_optional_nullable")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<Nullable<models::FileSystemEntryFileDetail>>,

    #[serde(rename = "folder")]
    #[serde(deserialize_with = "deserialize_optional_nullable")]
    #[serde(default = "default_optional_nullable")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder: Option<Nullable<models::FileSystemEntryFolderDetail>>,

    #[serde(rename = "link")]
    #[serde(deserialize_with = "deserialize_optional_nullable")]
    #[serde(default = "default_optional_nullable")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<Nullable<models::FileSystemEntryLinkDetail>>,
}

impl FileSystemEntry {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(path: String, name: String, metadata: models::FileSystemMetadata, kind: models::FileSystemEntryKind) -> FileSystemEntry {
        FileSystemEntry {
            path,
            name,
            metadata,
            kind,
            file: None,
            folder: None,
            link: None,
        }
    }
}

/// Converts the FileSystemEntry value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for FileSystemEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("path".to_string()),
            Some(self.path.to_string()),
            Some("name".to_string()),
            Some(self.name.to_string()),
            // Skipping metadata in query parameter serialization

            // Skipping kind in query parameter serialization

            // Skipping file in query parameter serialization

            // Skipping folder in query parameter serialization

            // Skipping link in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a FileSystemEntry value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for FileSystemEntry {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub path: Vec<String>,
            pub name: Vec<String>,
            pub metadata: Vec<models::FileSystemMetadata>,
            pub kind: Vec<models::FileSystemEntryKind>,
            pub file: Vec<models::FileSystemEntryFileDetail>,
            pub folder: Vec<models::FileSystemEntryFolderDetail>,
            pub link: Vec<models::FileSystemEntryLinkDetail>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing FileSystemEntry".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "path" => intermediate_rep.path.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "name" => intermediate_rep.name.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "metadata" => intermediate_rep
                        .metadata
                        .push(<models::FileSystemMetadata as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "kind" => intermediate_rep
                        .kind
                        .push(<models::FileSystemEntryKind as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    "file" => return std::result::Result::Err("Parsing a nullable type in this style is not supported in FileSystemEntry".to_string()),
                    "folder" => return std::result::Result::Err("Parsing a nullable type in this style is not supported in FileSystemEntry".to_string()),
                    "link" => return std::result::Result::Err("Parsing a nullable type in this style is not supported in FileSystemEntry".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing FileSystemEntry".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(FileSystemEntry {
            path: intermediate_rep.path.into_iter().next().ok_or_else(|| "path missing in FileSystemEntry".to_string())?,
            name: intermediate_rep.name.into_iter().next().ok_or_else(|| "name missing in FileSystemEntry".to_string())?,
            metadata: intermediate_rep
                .metadata
                .into_iter()
                .next()
                .ok_or_else(|| "metadata missing in FileSystemEntry".to_string())?,
            kind: intermediate_rep.kind.into_iter().next().ok_or_else(|| "kind missing in FileSystemEntry".to_string())?,
            file: std::result::Result::Err("Nullable types not supported in FileSystemEntry".to_string())?,
            folder: std::result::Result::Err("Nullable types not supported in FileSystemEntry".to_string())?,
            link: std::result::Result::Err("Nullable types not supported in FileSystemEntry".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<FileSystemEntry> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<FileSystemEntry>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<FileSystemEntry>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for FileSystemEntry - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<FileSystemEntry> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <FileSystemEntry as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into FileSystemEntry - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// Present when `kind` is `file`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct FileSystemEntryFileDetail {
    /// Whether the daemon expects to be able to list this file's contents as a folder. This is a hint derived from the extension alone; the daemon does not read the file to answer a listing. Opening the archive may still fail with `UnsupportedType`.  The set of recognised extensions is a server property, so clients must not maintain their own list.
    #[serde(rename = "archive")]
    pub archive: bool,
}

impl FileSystemEntryFileDetail {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(archive: bool) -> FileSystemEntryFileDetail {
        FileSystemEntryFileDetail { archive }
    }
}

/// Converts the FileSystemEntryFileDetail value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for FileSystemEntryFileDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![Some("archive".to_string()), Some(self.archive.to_string())];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a FileSystemEntryFileDetail value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for FileSystemEntryFileDetail {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub archive: Vec<bool>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing FileSystemEntryFileDetail".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "archive" => intermediate_rep.archive.push(<bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing FileSystemEntryFileDetail".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(FileSystemEntryFileDetail {
            archive: intermediate_rep
                .archive
                .into_iter()
                .next()
                .ok_or_else(|| "archive missing in FileSystemEntryFileDetail".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<FileSystemEntryFileDetail> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<FileSystemEntryFileDetail>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<FileSystemEntryFileDetail>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for FileSystemEntryFileDetail - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<FileSystemEntryFileDetail> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <FileSystemEntryFileDetail as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into FileSystemEntryFileDetail - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// Present when `kind` is `folder`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct FileSystemEntryFolderDetail {
    #[serde(rename = "hasChildren")]
    #[serde(deserialize_with = "deserialize_optional_nullable")]
    #[serde(default = "default_optional_nullable")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_children: Option<Nullable<bool>>,
}

impl FileSystemEntryFolderDetail {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new() -> FileSystemEntryFolderDetail {
        FileSystemEntryFolderDetail { has_children: None }
    }
}

/// Converts the FileSystemEntryFolderDetail value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for FileSystemEntryFolderDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            self.has_children
                .as_ref()
                .map(|has_children| ["hasChildren".to_string(), has_children.as_ref().map_or("null".to_string(), |x| x.to_string())].join(",")),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a FileSystemEntryFolderDetail value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for FileSystemEntryFolderDetail {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub has_children: Vec<bool>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing FileSystemEntryFolderDetail".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "hasChildren" => return std::result::Result::Err("Parsing a nullable type in this style is not supported in FileSystemEntryFolderDetail".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing FileSystemEntryFolderDetail".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(FileSystemEntryFolderDetail {
            has_children: std::result::Result::Err("Nullable types not supported in FileSystemEntryFolderDetail".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<FileSystemEntryFolderDetail> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<FileSystemEntryFolderDetail>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<FileSystemEntryFolderDetail>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for FileSystemEntryFolderDetail - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<FileSystemEntryFolderDetail> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <FileSystemEntryFolderDetail as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into FileSystemEntryFolderDetail - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types, clippy::large_enum_variant)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk_enum_derive::LabelledGenericEnum))]
pub enum FileSystemEntryKind {
    #[serde(rename = "file")]
    File,
    #[serde(rename = "folder")]
    Folder,
    #[serde(rename = "link")]
    Link,
}

impl validator::Validate for FileSystemEntryKind {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for FileSystemEntryKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            FileSystemEntryKind::File => write!(f, "file"),
            FileSystemEntryKind::Folder => write!(f, "folder"),
            FileSystemEntryKind::Link => write!(f, "link"),
        }
    }
}

impl std::str::FromStr for FileSystemEntryKind {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "file" => std::result::Result::Ok(FileSystemEntryKind::File),
            "folder" => std::result::Result::Ok(FileSystemEntryKind::Folder),
            "link" => std::result::Result::Ok(FileSystemEntryKind::Link),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
        }
    }
}

/// Present when `kind` is `link`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct FileSystemEntryLinkDetail {
    /// Link target, present only when it resolves to a path inside the same Library. It is null for a broken link and for a link pointing outside the Library, because host absolute paths are never exposed.
    #[serde(rename = "targetPath")]
    #[validate(length(min = 1, max = 4096))]
    #[serde(deserialize_with = "deserialize_optional_nullable")]
    #[serde(default = "default_optional_nullable")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_path: Option<Nullable<String>>,

    /// Kind of the resolved target, so a client can choose an icon without a second request. Null whenever `targetPath` is null.
    #[serde(rename = "targetKind")]
    #[serde(deserialize_with = "deserialize_optional_nullable")]
    #[serde(default = "default_optional_nullable")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_kind: Option<Nullable<models::FileSystemEntryKind>>,
}

impl FileSystemEntryLinkDetail {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new() -> FileSystemEntryLinkDetail {
        FileSystemEntryLinkDetail {
            target_path: None,
            target_kind: None,
        }
    }
}

/// Converts the FileSystemEntryLinkDetail value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for FileSystemEntryLinkDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            self.target_path
                .as_ref()
                .map(|target_path| ["targetPath".to_string(), target_path.as_ref().map_or("null".to_string(), |x| x.to_string())].join(",")),
            // Skipping targetKind in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a FileSystemEntryLinkDetail value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for FileSystemEntryLinkDetail {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub target_path: Vec<String>,
            pub target_kind: Vec<models::FileSystemEntryKind>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing FileSystemEntryLinkDetail".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "targetPath" => return std::result::Result::Err("Parsing a nullable type in this style is not supported in FileSystemEntryLinkDetail".to_string()),
                    "targetKind" => return std::result::Result::Err("Parsing a nullable type in this style is not supported in FileSystemEntryLinkDetail".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing FileSystemEntryLinkDetail".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(FileSystemEntryLinkDetail {
            target_path: std::result::Result::Err("Nullable types not supported in FileSystemEntryLinkDetail".to_string())?,
            target_kind: std::result::Result::Err("Nullable types not supported in FileSystemEntryLinkDetail".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<FileSystemEntryLinkDetail> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<FileSystemEntryLinkDetail>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<FileSystemEntryLinkDetail>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for FileSystemEntryLinkDetail - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<FileSystemEntryLinkDetail> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <FileSystemEntryLinkDetail as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into FileSystemEntryLinkDetail - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct FileSystemEntrySummary {
    /// Absolute-looking path within the selected Library. The resolved path must never escape that library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096), custom(function = "check_xss_string"))]
    pub path: String,

    /// Last path segment.
    #[serde(rename = "name")]
    #[validate(custom(function = "check_xss_string"))]
    pub name: String,

    #[serde(rename = "kind")]
    #[validate(nested)]
    pub kind: models::FileSystemEntryKind,

    /// Extension-derived archive hint; false unless kind is file.
    #[serde(rename = "archive")]
    pub archive: bool,
}

impl FileSystemEntrySummary {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(path: String, name: String, kind: models::FileSystemEntryKind, archive: bool) -> FileSystemEntrySummary {
        FileSystemEntrySummary { path, name, kind, archive }
    }
}

/// Converts the FileSystemEntrySummary value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for FileSystemEntrySummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("path".to_string()),
            Some(self.path.to_string()),
            Some("name".to_string()),
            Some(self.name.to_string()),
            // Skipping kind in query parameter serialization
            Some("archive".to_string()),
            Some(self.archive.to_string()),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a FileSystemEntrySummary value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for FileSystemEntrySummary {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub path: Vec<String>,
            pub name: Vec<String>,
            pub kind: Vec<models::FileSystemEntryKind>,
            pub archive: Vec<bool>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing FileSystemEntrySummary".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "path" => intermediate_rep.path.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "name" => intermediate_rep.name.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "kind" => intermediate_rep
                        .kind
                        .push(<models::FileSystemEntryKind as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "archive" => intermediate_rep.archive.push(<bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing FileSystemEntrySummary".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(FileSystemEntrySummary {
            path: intermediate_rep
                .path
                .into_iter()
                .next()
                .ok_or_else(|| "path missing in FileSystemEntrySummary".to_string())?,
            name: intermediate_rep
                .name
                .into_iter()
                .next()
                .ok_or_else(|| "name missing in FileSystemEntrySummary".to_string())?,
            kind: intermediate_rep
                .kind
                .into_iter()
                .next()
                .ok_or_else(|| "kind missing in FileSystemEntrySummary".to_string())?,
            archive: intermediate_rep
                .archive
                .into_iter()
                .next()
                .ok_or_else(|| "archive missing in FileSystemEntrySummary".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<FileSystemEntrySummary> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<FileSystemEntrySummary>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<FileSystemEntrySummary>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for FileSystemEntrySummary - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<FileSystemEntrySummary> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <FileSystemEntrySummary as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into FileSystemEntrySummary - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct FileSystemMetadata {
    /// File size in bytes. Folders return 0 unless known.
    #[serde(rename = "size")]
    #[validate(range(min = 0u64))]
    pub size: u64,

    #[serde(rename = "readonly")]
    pub readonly: bool,

    #[serde(rename = "createdAt")]
    #[serde(deserialize_with = "deserialize_optional_nullable")]
    #[serde(default = "default_optional_nullable")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<Nullable<chrono::DateTime<chrono::Utc>>>,

    #[serde(rename = "modifiedAt")]
    #[serde(deserialize_with = "deserialize_optional_nullable")]
    #[serde(default = "default_optional_nullable")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<Nullable<chrono::DateTime<chrono::Utc>>>,

    #[serde(rename = "accessedAt")]
    #[serde(deserialize_with = "deserialize_optional_nullable")]
    #[serde(default = "default_optional_nullable")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessed_at: Option<Nullable<chrono::DateTime<chrono::Utc>>>,

    #[serde(rename = "contentType")]
    #[serde(deserialize_with = "deserialize_optional_nullable")]
    #[serde(default = "default_optional_nullable")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<Nullable<String>>,
}

impl FileSystemMetadata {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(size: u64, readonly: bool) -> FileSystemMetadata {
        FileSystemMetadata {
            size,
            readonly,
            created_at: None,
            modified_at: None,
            accessed_at: None,
            content_type: None,
        }
    }
}

/// Converts the FileSystemMetadata value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for FileSystemMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("size".to_string()),
            Some(self.size.to_string()),
            Some("readonly".to_string()),
            Some(self.readonly.to_string()),
            // Skipping createdAt in query parameter serialization

            // Skipping modifiedAt in query parameter serialization

            // Skipping accessedAt in query parameter serialization
            self.content_type
                .as_ref()
                .map(|content_type| ["contentType".to_string(), content_type.as_ref().map_or("null".to_string(), |x| x.to_string())].join(",")),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a FileSystemMetadata value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for FileSystemMetadata {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub size: Vec<u64>,
            pub readonly: Vec<bool>,
            pub created_at: Vec<chrono::DateTime<chrono::Utc>>,
            pub modified_at: Vec<chrono::DateTime<chrono::Utc>>,
            pub accessed_at: Vec<chrono::DateTime<chrono::Utc>>,
            pub content_type: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing FileSystemMetadata".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "size" => intermediate_rep.size.push(<u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "readonly" => intermediate_rep.readonly.push(<bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    "createdAt" => return std::result::Result::Err("Parsing a nullable type in this style is not supported in FileSystemMetadata".to_string()),
                    "modifiedAt" => return std::result::Result::Err("Parsing a nullable type in this style is not supported in FileSystemMetadata".to_string()),
                    "accessedAt" => return std::result::Result::Err("Parsing a nullable type in this style is not supported in FileSystemMetadata".to_string()),
                    "contentType" => return std::result::Result::Err("Parsing a nullable type in this style is not supported in FileSystemMetadata".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing FileSystemMetadata".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(FileSystemMetadata {
            size: intermediate_rep.size.into_iter().next().ok_or_else(|| "size missing in FileSystemMetadata".to_string())?,
            readonly: intermediate_rep
                .readonly
                .into_iter()
                .next()
                .ok_or_else(|| "readonly missing in FileSystemMetadata".to_string())?,
            created_at: std::result::Result::Err("Nullable types not supported in FileSystemMetadata".to_string())?,
            modified_at: std::result::Result::Err("Nullable types not supported in FileSystemMetadata".to_string())?,
            accessed_at: std::result::Result::Err("Nullable types not supported in FileSystemMetadata".to_string())?,
            content_type: std::result::Result::Err("Nullable types not supported in FileSystemMetadata".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<FileSystemMetadata> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<FileSystemMetadata>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<FileSystemMetadata>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for FileSystemMetadata - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<FileSystemMetadata> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <FileSystemMetadata as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into FileSystemMetadata - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct FolderChildrenProbe {
    /// Remaining validity; zero for unknown. Reused cache results do not receive a new lifetime.
    #[serde(rename = "maxAgeMs")]
    #[validate(range(min = 0u32, max = 30000u32))]
    pub max_age_ms: u32,

    #[serde(rename = "hasChildren")]
    pub has_children: Nullable<bool>,

    /// Opaque source version when it could be observed; absent if not known.
    #[serde(rename = "sourceVersion")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_version: Option<String>,
}

impl FolderChildrenProbe {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(max_age_ms: u32, has_children: Nullable<bool>) -> FolderChildrenProbe {
        FolderChildrenProbe {
            max_age_ms,
            has_children,
            source_version: None,
        }
    }
}

/// Converts the FolderChildrenProbe value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for FolderChildrenProbe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("maxAgeMs".to_string()),
            Some(self.max_age_ms.to_string()),
            Some("hasChildren".to_string()),
            Some(self.has_children.as_ref().map_or("null".to_string(), |x| x.to_string())),
            self.source_version
                .as_ref()
                .map(|source_version| ["sourceVersion".to_string(), source_version.to_string()].join(",")),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a FolderChildrenProbe value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for FolderChildrenProbe {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub max_age_ms: Vec<u32>,
            pub has_children: Vec<bool>,
            pub source_version: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing FolderChildrenProbe".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "maxAgeMs" => intermediate_rep.max_age_ms.push(<u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    "hasChildren" => return std::result::Result::Err("Parsing a nullable type in this style is not supported in FolderChildrenProbe".to_string()),
                    #[allow(clippy::redundant_clone)]
                    "sourceVersion" => intermediate_rep
                        .source_version
                        .push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing FolderChildrenProbe".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(FolderChildrenProbe {
            max_age_ms: intermediate_rep
                .max_age_ms
                .into_iter()
                .next()
                .ok_or_else(|| "maxAgeMs missing in FolderChildrenProbe".to_string())?,
            has_children: std::result::Result::Err("Nullable types not supported in FolderChildrenProbe".to_string())?,
            source_version: intermediate_rep.source_version.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<FolderChildrenProbe> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<FolderChildrenProbe>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<FolderChildrenProbe>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for FolderChildrenProbe - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<FolderChildrenProbe> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <FolderChildrenProbe as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into FolderChildrenProbe - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct FolderListing {
    #[serde(rename = "entries")]
    #[validate(nested)]
    pub entries: Vec<models::FileSystemEntry>,

    /// Total count after filtering the snapshot.
    #[serde(rename = "totalCount")]
    #[validate(range(min = 0u64))]
    pub total_count: u64,

    /// Opaque identifier of the listing snapshot.
    #[serde(rename = "snapshot")]
    #[validate(custom(function = "check_xss_string"))]
    pub snapshot: String,

    /// True when recursive traversal reached its entry limit.
    #[serde(rename = "truncated")]
    pub truncated: bool,
}

impl FolderListing {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(entries: Vec<models::FileSystemEntry>, total_count: u64, snapshot: String, truncated: bool) -> FolderListing {
        FolderListing {
            entries,
            total_count,
            snapshot,
            truncated,
        }
    }
}

/// Converts the FolderListing value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for FolderListing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping entries in query parameter serialization
            Some("totalCount".to_string()),
            Some(self.total_count.to_string()),
            Some("snapshot".to_string()),
            Some(self.snapshot.to_string()),
            Some("truncated".to_string()),
            Some(self.truncated.to_string()),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a FolderListing value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for FolderListing {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub entries: Vec<Vec<models::FileSystemEntry>>,
            pub total_count: Vec<u64>,
            pub snapshot: Vec<String>,
            pub truncated: Vec<bool>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing FolderListing".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "entries" => return std::result::Result::Err("Parsing a container in this style is not supported in FolderListing".to_string()),
                    #[allow(clippy::redundant_clone)]
                    "totalCount" => intermediate_rep.total_count.push(<u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "snapshot" => intermediate_rep.snapshot.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "truncated" => intermediate_rep.truncated.push(<bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing FolderListing".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(FolderListing {
            entries: intermediate_rep.entries.into_iter().next().ok_or_else(|| "entries missing in FolderListing".to_string())?,
            total_count: intermediate_rep
                .total_count
                .into_iter()
                .next()
                .ok_or_else(|| "totalCount missing in FolderListing".to_string())?,
            snapshot: intermediate_rep
                .snapshot
                .into_iter()
                .next()
                .ok_or_else(|| "snapshot missing in FolderListing".to_string())?,
            truncated: intermediate_rep
                .truncated
                .into_iter()
                .next()
                .ok_or_else(|| "truncated missing in FolderListing".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<FolderListing> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<FolderListing>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<FolderListing>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for FolderListing - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<FolderListing> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <FolderListing as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into FolderListing - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct FolderResponse {
    #[serde(rename = "folder")]
    #[validate(nested)]
    pub folder: models::FileSystemEntry,
}

impl FolderResponse {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(folder: models::FileSystemEntry) -> FolderResponse {
        FolderResponse { folder }
    }
}

/// Converts the FolderResponse value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for FolderResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping folder in query parameter serialization

        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a FolderResponse value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for FolderResponse {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub folder: Vec<models::FileSystemEntry>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing FolderResponse".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "folder" => intermediate_rep
                        .folder
                        .push(<models::FileSystemEntry as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing FolderResponse".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(FolderResponse {
            folder: intermediate_rep.folder.into_iter().next().ok_or_else(|| "folder missing in FolderResponse".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<FolderResponse> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<FolderResponse>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<FolderResponse>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for FolderResponse - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<FolderResponse> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <FolderResponse as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into FolderResponse - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct FolderSummaryListing {
    #[serde(rename = "entries")]
    #[validate(nested)]
    pub entries: Vec<models::FileSystemEntrySummary>,

    #[serde(rename = "totalCount")]
    #[validate(range(min = 0u64))]
    pub total_count: u64,

    /// Identifier of the names and kinds, not of later detail metadata.
    #[serde(rename = "snapshot")]
    #[validate(custom(function = "check_xss_string"))]
    pub snapshot: String,

    /// Always false for a complete direct-child summary listing.
    #[serde(rename = "truncated")]
    pub truncated: bool,
}

impl FolderSummaryListing {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(entries: Vec<models::FileSystemEntrySummary>, total_count: u64, snapshot: String, truncated: bool) -> FolderSummaryListing {
        FolderSummaryListing {
            entries,
            total_count,
            snapshot,
            truncated,
        }
    }
}

/// Converts the FolderSummaryListing value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for FolderSummaryListing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping entries in query parameter serialization
            Some("totalCount".to_string()),
            Some(self.total_count.to_string()),
            Some("snapshot".to_string()),
            Some(self.snapshot.to_string()),
            Some("truncated".to_string()),
            Some(self.truncated.to_string()),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a FolderSummaryListing value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for FolderSummaryListing {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub entries: Vec<Vec<models::FileSystemEntrySummary>>,
            pub total_count: Vec<u64>,
            pub snapshot: Vec<String>,
            pub truncated: Vec<bool>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing FolderSummaryListing".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "entries" => return std::result::Result::Err("Parsing a container in this style is not supported in FolderSummaryListing".to_string()),
                    #[allow(clippy::redundant_clone)]
                    "totalCount" => intermediate_rep.total_count.push(<u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "snapshot" => intermediate_rep.snapshot.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "truncated" => intermediate_rep.truncated.push(<bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing FolderSummaryListing".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(FolderSummaryListing {
            entries: intermediate_rep
                .entries
                .into_iter()
                .next()
                .ok_or_else(|| "entries missing in FolderSummaryListing".to_string())?,
            total_count: intermediate_rep
                .total_count
                .into_iter()
                .next()
                .ok_or_else(|| "totalCount missing in FolderSummaryListing".to_string())?,
            snapshot: intermediate_rep
                .snapshot
                .into_iter()
                .next()
                .ok_or_else(|| "snapshot missing in FolderSummaryListing".to_string())?,
            truncated: intermediate_rep
                .truncated
                .into_iter()
                .next()
                .ok_or_else(|| "truncated missing in FolderSummaryListing".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<FolderSummaryListing> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<FolderSummaryListing>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<FolderSummaryListing>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for FolderSummaryListing - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<FolderSummaryListing> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <FolderSummaryListing as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into FolderSummaryListing - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct HealthResponse {
    #[serde(rename = "result")]
    pub result: bool,
}

impl HealthResponse {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(result: bool) -> HealthResponse {
        HealthResponse { result }
    }
}

/// Converts the HealthResponse value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for HealthResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![Some("result".to_string()), Some(self.result.to_string())];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a HealthResponse value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for HealthResponse {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub result: Vec<bool>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing HealthResponse".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "result" => intermediate_rep.result.push(<bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing HealthResponse".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(HealthResponse {
            result: intermediate_rep.result.into_iter().next().ok_or_else(|| "result missing in HealthResponse".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<HealthResponse> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<HealthResponse>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<HealthResponse>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for HealthResponse - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<HealthResponse> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <HealthResponse as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into HealthResponse - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types, clippy::large_enum_variant)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk_enum_derive::LabelledGenericEnum))]
pub enum ImageAnchor {
    #[serde(rename = "bottom")]
    Bottom,
    #[serde(rename = "bottom_left")]
    BottomLeft,
    #[serde(rename = "bottom_right")]
    BottomRight,
    #[serde(rename = "center")]
    Center,
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "right")]
    Right,
    #[serde(rename = "top")]
    Top,
    #[serde(rename = "top_left")]
    TopLeft,
    #[serde(rename = "top_right")]
    TopRight,
}

impl validator::Validate for ImageAnchor {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for ImageAnchor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            ImageAnchor::Bottom => write!(f, "bottom"),
            ImageAnchor::BottomLeft => write!(f, "bottom_left"),
            ImageAnchor::BottomRight => write!(f, "bottom_right"),
            ImageAnchor::Center => write!(f, "center"),
            ImageAnchor::Left => write!(f, "left"),
            ImageAnchor::Right => write!(f, "right"),
            ImageAnchor::Top => write!(f, "top"),
            ImageAnchor::TopLeft => write!(f, "top_left"),
            ImageAnchor::TopRight => write!(f, "top_right"),
        }
    }
}

impl std::str::FromStr for ImageAnchor {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "bottom" => std::result::Result::Ok(ImageAnchor::Bottom),
            "bottom_left" => std::result::Result::Ok(ImageAnchor::BottomLeft),
            "bottom_right" => std::result::Result::Ok(ImageAnchor::BottomRight),
            "center" => std::result::Result::Ok(ImageAnchor::Center),
            "left" => std::result::Result::Ok(ImageAnchor::Left),
            "right" => std::result::Result::Ok(ImageAnchor::Right),
            "top" => std::result::Result::Ok(ImageAnchor::Top),
            "top_left" => std::result::Result::Ok(ImageAnchor::TopLeft),
            "top_right" => std::result::Result::Ok(ImageAnchor::TopRight),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
        }
    }
}

/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types, clippy::large_enum_variant)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk_enum_derive::LabelledGenericEnum))]
pub enum ImageFormat {
    #[serde(rename = "webp")]
    Webp,
    #[serde(rename = "jpeg")]
    Jpeg,
    #[serde(rename = "png")]
    Png,
}

impl validator::Validate for ImageFormat {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for ImageFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            ImageFormat::Webp => write!(f, "webp"),
            ImageFormat::Jpeg => write!(f, "jpeg"),
            ImageFormat::Png => write!(f, "png"),
        }
    }
}

impl std::str::FromStr for ImageFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "webp" => std::result::Result::Ok(ImageFormat::Webp),
            "jpeg" => std::result::Result::Ok(ImageFormat::Jpeg),
            "png" => std::result::Result::Ok(ImageFormat::Png),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
        }
    }
}

/// Format, size and shooting information of an image.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ImageMediaInfo {
    /// MIME type decided from the file extension.
    #[serde(rename = "contentType")]
    #[validate(custom(function = "check_xss_string"))]
    pub content_type: String,

    /// Width after applying the EXIF orientation. For formats whose orientation is given by a rotation (HEIC, AVIF and so on), the width as stored; apply `rotation` to get the displayed one.
    #[serde(rename = "width")]
    #[validate(range(min = 1u32))]
    pub width: u32,

    /// Height after applying the EXIF orientation. For formats whose orientation is given by a rotation (HEIC, AVIF and so on), the height as stored; apply `rotation` to get the displayed one.
    #[serde(rename = "height")]
    #[validate(range(min = 1u32))]
    pub height: u32,

    /// Counterclockwise rotation in degrees (90, 180 or 270) that a viewer applies to the stored image to display it upright. It is not applied to `width` and `height`. Present only for formats whose orientation is given by a rotation, when the image has one.
    #[serde(rename = "rotation")]
    #[validate(range(min = 90u32, max = 270u32))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotation: Option<u32>,

    #[serde(rename = "shooting")]
    #[validate(nested)]
    pub shooting: models::ShootingInfo,
}

impl ImageMediaInfo {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(content_type: String, width: u32, height: u32, shooting: models::ShootingInfo) -> ImageMediaInfo {
        ImageMediaInfo {
            content_type,
            width,
            height,
            rotation: None,
            shooting,
        }
    }
}

/// Converts the ImageMediaInfo value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for ImageMediaInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("contentType".to_string()),
            Some(self.content_type.to_string()),
            Some("width".to_string()),
            Some(self.width.to_string()),
            Some("height".to_string()),
            Some(self.height.to_string()),
            self.rotation.as_ref().map(|rotation| ["rotation".to_string(), rotation.to_string()].join(",")),
            // Skipping shooting in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a ImageMediaInfo value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for ImageMediaInfo {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub content_type: Vec<String>,
            pub width: Vec<u32>,
            pub height: Vec<u32>,
            pub rotation: Vec<u32>,
            pub shooting: Vec<models::ShootingInfo>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing ImageMediaInfo".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "contentType" => intermediate_rep.content_type.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "width" => intermediate_rep.width.push(<u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "height" => intermediate_rep.height.push(<u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "rotation" => intermediate_rep.rotation.push(<u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "shooting" => intermediate_rep
                        .shooting
                        .push(<models::ShootingInfo as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing ImageMediaInfo".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(ImageMediaInfo {
            content_type: intermediate_rep
                .content_type
                .into_iter()
                .next()
                .ok_or_else(|| "contentType missing in ImageMediaInfo".to_string())?,
            width: intermediate_rep.width.into_iter().next().ok_or_else(|| "width missing in ImageMediaInfo".to_string())?,
            height: intermediate_rep.height.into_iter().next().ok_or_else(|| "height missing in ImageMediaInfo".to_string())?,
            rotation: intermediate_rep.rotation.into_iter().next(),
            shooting: intermediate_rep
                .shooting
                .into_iter()
                .next()
                .ok_or_else(|| "shooting missing in ImageMediaInfo".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<ImageMediaInfo> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<ImageMediaInfo>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<ImageMediaInfo>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for ImageMediaInfo - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<ImageMediaInfo> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <ImageMediaInfo as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into ImageMediaInfo - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types, clippy::large_enum_variant)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk_enum_derive::LabelledGenericEnum))]
pub enum ImageResampler {
    #[serde(rename = "lanczos3")]
    Lanczos3,
}

impl validator::Validate for ImageResampler {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for ImageResampler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            ImageResampler::Lanczos3 => write!(f, "lanczos3"),
        }
    }
}

impl std::str::FromStr for ImageResampler {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "lanczos3" => std::result::Result::Ok(ImageResampler::Lanczos3),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
        }
    }
}

/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types, clippy::large_enum_variant)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk_enum_derive::LabelledGenericEnum))]
pub enum ImageResize {
    #[serde(rename = "crop")]
    Crop,
    #[serde(rename = "pad")]
    Pad,
    #[serde(rename = "manual")]
    Manual,
    #[serde(rename = "fit")]
    Fit,
}

impl validator::Validate for ImageResize {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for ImageResize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            ImageResize::Crop => write!(f, "crop"),
            ImageResize::Pad => write!(f, "pad"),
            ImageResize::Manual => write!(f, "manual"),
            ImageResize::Fit => write!(f, "fit"),
        }
    }
}

impl std::str::FromStr for ImageResize {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "crop" => std::result::Result::Ok(ImageResize::Crop),
            "pad" => std::result::Result::Ok(ImageResize::Pad),
            "manual" => std::result::Result::Ok(ImageResize::Manual),
            "fit" => std::result::Result::Ok(ImageResize::Fit),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct InitialSetupRequest {
    #[serde(rename = "username")]
    #[validate(
            length(min = 1, max = 128),
            regex(path = *RE_INITIALSETUPREQUEST_USERNAME),
          custom(function = "check_xss_string"),
    )]
    pub username: String,

    #[serde(rename = "password")]
    #[validate(length(min = 8, max = 256), custom(function = "check_xss_string"))]
    pub password: String,

    #[serde(rename = "sessionTransport")]
    #[validate(nested)]
    pub session_transport: models::SessionTransport,
}

lazy_static::lazy_static! {
    static ref RE_INITIALSETUPREQUEST_USERNAME: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

impl InitialSetupRequest {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(username: String, password: String, session_transport: models::SessionTransport) -> InitialSetupRequest {
        InitialSetupRequest {
            username,
            password,
            session_transport,
        }
    }
}

/// Converts the InitialSetupRequest value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for InitialSetupRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("username".to_string()),
            Some(self.username.to_string()),
            Some("password".to_string()),
            Some(self.password.to_string()),
            // Skipping sessionTransport in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a InitialSetupRequest value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for InitialSetupRequest {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub username: Vec<String>,
            pub password: Vec<String>,
            pub session_transport: Vec<models::SessionTransport>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing InitialSetupRequest".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "username" => intermediate_rep.username.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "password" => intermediate_rep.password.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "sessionTransport" => intermediate_rep
                        .session_transport
                        .push(<models::SessionTransport as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing InitialSetupRequest".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(InitialSetupRequest {
            username: intermediate_rep
                .username
                .into_iter()
                .next()
                .ok_or_else(|| "username missing in InitialSetupRequest".to_string())?,
            password: intermediate_rep
                .password
                .into_iter()
                .next()
                .ok_or_else(|| "password missing in InitialSetupRequest".to_string())?,
            session_transport: intermediate_rep
                .session_transport
                .into_iter()
                .next()
                .ok_or_else(|| "sessionTransport missing in InitialSetupRequest".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<InitialSetupRequest> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<InitialSetupRequest>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<InitialSetupRequest>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for InitialSetupRequest - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<InitialSetupRequest> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <InitialSetupRequest as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into InitialSetupRequest - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct Library {
    /// Library identifier chosen by the operator in the daemon configuration file. Unlike the daemon-issued identifiers in this API it is a readable name, not a generated token. It is what the API uses in place of a host path, so the physical location of a Library is never exposed.  Because the value is used as a URL path segment, the character set is restricted. The daemon rejects a configuration that violates it at startup.
    #[serde(rename = "id")]
    #[validate(
            length(min = 1, max = 64),
            regex(path = *RE_LIBRARY_ID),
          custom(function = "check_xss_string"),
    )]
    pub id: String,

    #[serde(rename = "name")]
    #[validate(length(min = 1, max = 128), custom(function = "check_xss_string"))]
    pub name: String,

    #[serde(rename = "readonly")]
    pub readonly: bool,
}

lazy_static::lazy_static! {
    static ref RE_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

impl Library {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(id: String, name: String, readonly: bool) -> Library {
        Library { id, name, readonly }
    }
}

/// Converts the Library value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for Library {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("id".to_string()),
            Some(self.id.to_string()),
            Some("name".to_string()),
            Some(self.name.to_string()),
            Some("readonly".to_string()),
            Some(self.readonly.to_string()),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a Library value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for Library {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub id: Vec<String>,
            pub name: Vec<String>,
            pub readonly: Vec<bool>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing Library".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "id" => intermediate_rep.id.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "name" => intermediate_rep.name.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "readonly" => intermediate_rep.readonly.push(<bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing Library".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(Library {
            id: intermediate_rep.id.into_iter().next().ok_or_else(|| "id missing in Library".to_string())?,
            name: intermediate_rep.name.into_iter().next().ok_or_else(|| "name missing in Library".to_string())?,
            readonly: intermediate_rep.readonly.into_iter().next().ok_or_else(|| "readonly missing in Library".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<Library> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<Library>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<Library>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for Library - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<Library> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <Library as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into Library - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// Library identifier chosen by the operator in the daemon configuration file. Unlike the daemon-issued identifiers in this API it is a readable name, not a generated token. It is what the API uses in place of a host path, so the physical location of a Library is never exposed.  Because the value is used as a URL path segment, the character set is restricted. The daemon rejects a configuration that violates it at startup.
#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct LibraryId(pub String);

impl validator::Validate for LibraryId {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::convert::From<String> for LibraryId {
    fn from(x: String) -> Self {
        LibraryId(x)
    }
}

impl std::fmt::Display for LibraryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for LibraryId {
    type Err = std::string::ParseError;
    fn from_str(x: &str) -> std::result::Result<Self, Self::Err> {
        std::result::Result::Ok(LibraryId(x.to_string()))
    }
}

impl std::convert::From<LibraryId> for String {
    fn from(x: LibraryId) -> Self {
        x.0
    }
}

impl std::ops::Deref for LibraryId {
    type Target = String;
    fn deref(&self) -> &String {
        &self.0
    }
}

impl std::ops::DerefMut for LibraryId {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct LibraryList {
    #[serde(rename = "libraries")]
    #[validate(nested)]
    pub libraries: Vec<models::Library>,
}

impl LibraryList {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(libraries: Vec<models::Library>) -> LibraryList {
        LibraryList { libraries }
    }
}

/// Converts the LibraryList value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for LibraryList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping libraries in query parameter serialization

        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a LibraryList value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for LibraryList {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub libraries: Vec<Vec<models::Library>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing LibraryList".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "libraries" => return std::result::Result::Err("Parsing a container in this style is not supported in LibraryList".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing LibraryList".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(LibraryList {
            libraries: intermediate_rep
                .libraries
                .into_iter()
                .next()
                .ok_or_else(|| "libraries missing in LibraryList".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<LibraryList> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<LibraryList>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<LibraryList>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for LibraryList - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<LibraryList> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <LibraryList as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into LibraryList - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct LibraryResponse {
    #[serde(rename = "library")]
    #[validate(nested)]
    pub library: models::Library,
}

impl LibraryResponse {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(library: models::Library) -> LibraryResponse {
        LibraryResponse { library }
    }
}

/// Converts the LibraryResponse value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for LibraryResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping library in query parameter serialization

        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a LibraryResponse value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for LibraryResponse {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub library: Vec<models::Library>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing LibraryResponse".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "library" => intermediate_rep
                        .library
                        .push(<models::Library as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing LibraryResponse".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(LibraryResponse {
            library: intermediate_rep
                .library
                .into_iter()
                .next()
                .ok_or_else(|| "library missing in LibraryResponse".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<LibraryResponse> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<LibraryResponse>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<LibraryResponse>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for LibraryResponse - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<LibraryResponse> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <LibraryResponse as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into LibraryResponse - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types, clippy::large_enum_variant)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk_enum_derive::LabelledGenericEnum))]
pub enum ListView {
    #[serde(rename = "grid")]
    Grid,
    #[serde(rename = "list")]
    List,
}

impl validator::Validate for ListView {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for ListView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            ListView::Grid => write!(f, "grid"),
            ListView::List => write!(f, "list"),
        }
    }
}

impl std::str::FromStr for ListView {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "grid" => std::result::Result::Ok(ListView::Grid),
            "list" => std::result::Result::Ok(ListView::List),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct LoginRequest {
    #[serde(rename = "username")]
    #[validate(
            length(min = 1, max = 128),
            regex(path = *RE_LOGINREQUEST_USERNAME),
          custom(function = "check_xss_string"),
    )]
    pub username: String,

    #[serde(rename = "password")]
    #[validate(length(min = 8, max = 256), custom(function = "check_xss_string"))]
    pub password: String,

    #[serde(rename = "sessionTransport")]
    #[validate(nested)]
    pub session_transport: models::SessionTransport,
}

lazy_static::lazy_static! {
    static ref RE_LOGINREQUEST_USERNAME: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

impl LoginRequest {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(username: String, password: String, session_transport: models::SessionTransport) -> LoginRequest {
        LoginRequest {
            username,
            password,
            session_transport,
        }
    }
}

/// Converts the LoginRequest value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for LoginRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("username".to_string()),
            Some(self.username.to_string()),
            Some("password".to_string()),
            Some(self.password.to_string()),
            // Skipping sessionTransport in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a LoginRequest value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for LoginRequest {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub username: Vec<String>,
            pub password: Vec<String>,
            pub session_transport: Vec<models::SessionTransport>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing LoginRequest".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "username" => intermediate_rep.username.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "password" => intermediate_rep.password.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "sessionTransport" => intermediate_rep
                        .session_transport
                        .push(<models::SessionTransport as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing LoginRequest".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(LoginRequest {
            username: intermediate_rep.username.into_iter().next().ok_or_else(|| "username missing in LoginRequest".to_string())?,
            password: intermediate_rep.password.into_iter().next().ok_or_else(|| "password missing in LoginRequest".to_string())?,
            session_transport: intermediate_rep
                .session_transport
                .into_iter()
                .next()
                .ok_or_else(|| "sessionTransport missing in LoginRequest".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<LoginRequest> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<LoginRequest>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<LoginRequest>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for LoginRequest - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<LoginRequest> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <LoginRequest as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into LoginRequest - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// Media information of an image or a video. `kind` tells which one it is, and only the matching property (`image` or `video`) is present.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct MediaInfo {
    #[serde(rename = "kind")]
    #[validate(nested)]
    pub kind: models::MediaInfoKind,

    #[serde(rename = "video")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video: Option<models::VideoMediaInfo>,

    #[serde(rename = "image")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<models::ImageMediaInfo>,
}

impl MediaInfo {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(kind: models::MediaInfoKind) -> MediaInfo {
        MediaInfo { kind, video: None, image: None }
    }
}

/// Converts the MediaInfo value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for MediaInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping kind in query parameter serialization

            // Skipping video in query parameter serialization

            // Skipping image in query parameter serialization

        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a MediaInfo value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for MediaInfo {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub kind: Vec<models::MediaInfoKind>,
            pub video: Vec<models::VideoMediaInfo>,
            pub image: Vec<models::ImageMediaInfo>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing MediaInfo".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "kind" => intermediate_rep
                        .kind
                        .push(<models::MediaInfoKind as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "video" => intermediate_rep
                        .video
                        .push(<models::VideoMediaInfo as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "image" => intermediate_rep
                        .image
                        .push(<models::ImageMediaInfo as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing MediaInfo".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(MediaInfo {
            kind: intermediate_rep.kind.into_iter().next().ok_or_else(|| "kind missing in MediaInfo".to_string())?,
            video: intermediate_rep.video.into_iter().next(),
            image: intermediate_rep.image.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<MediaInfo> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<MediaInfo>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<MediaInfo>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for MediaInfo - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<MediaInfo> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <MediaInfo as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into MediaInfo - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types, clippy::large_enum_variant)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk_enum_derive::LabelledGenericEnum))]
pub enum MediaInfoKind {
    #[serde(rename = "image")]
    Image,
    #[serde(rename = "video")]
    Video,
}

impl validator::Validate for MediaInfoKind {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for MediaInfoKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            MediaInfoKind::Image => write!(f, "image"),
            MediaInfoKind::Video => write!(f, "video"),
        }
    }
}

impl std::str::FromStr for MediaInfoKind {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "image" => std::result::Result::Ok(MediaInfoKind::Image),
            "video" => std::result::Result::Ok(MediaInfoKind::Video),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct MediaInfoResponse {
    #[serde(rename = "mediaInfo")]
    #[validate(nested)]
    pub media_info: models::MediaInfo,
}

impl MediaInfoResponse {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(media_info: models::MediaInfo) -> MediaInfoResponse {
        MediaInfoResponse { media_info }
    }
}

/// Converts the MediaInfoResponse value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for MediaInfoResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping mediaInfo in query parameter serialization

        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a MediaInfoResponse value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for MediaInfoResponse {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub media_info: Vec<models::MediaInfo>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing MediaInfoResponse".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "mediaInfo" => intermediate_rep
                        .media_info
                        .push(<models::MediaInfo as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing MediaInfoResponse".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(MediaInfoResponse {
            media_info: intermediate_rep
                .media_info
                .into_iter()
                .next()
                .ok_or_else(|| "mediaInfo missing in MediaInfoResponse".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<MediaInfoResponse> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<MediaInfoResponse>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<MediaInfoResponse>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for MediaInfoResponse - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<MediaInfoResponse> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <MediaInfoResponse as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into MediaInfoResponse - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct Operation {
    /// Opaque daemon-issued operation identifier.
    #[serde(rename = "id")]
    #[validate(length(min = 1), custom(function = "check_xss_string"))]
    pub id: String,

    #[serde(rename = "kind")]
    #[validate(nested)]
    pub kind: models::OperationKind,

    #[serde(rename = "status")]
    #[validate(nested)]
    pub status: models::OperationStatus,

    /// Library identifier chosen by the operator in the daemon configuration file. Unlike the daemon-issued identifiers in this API it is a readable name, not a generated token. It is what the API uses in place of a host path, so the physical location of a Library is never exposed.  Because the value is used as a URL path segment, the character set is restricted. The daemon rejects a configuration that violates it at startup.
    #[serde(rename = "libraryId")]
    #[validate(
            length(min = 1, max = 64),
            regex(path = *RE_OPERATION_LIBRARY_ID),
          custom(function = "check_xss_string"),
    )]
    pub library_id: String,

    /// Library identifier chosen by the operator in the daemon configuration file. Unlike the daemon-issued identifiers in this API it is a readable name, not a generated token. It is what the API uses in place of a host path, so the physical location of a Library is never exposed.  Because the value is used as a URL path segment, the character set is restricted. The daemon rejects a configuration that violates it at startup.
    #[serde(rename = "destinationLibraryId")]
    #[validate(
            length(min = 1, max = 64),
            regex(path = *RE_OPERATION_DESTINATION_LIBRARY_ID),
          custom(function = "check_xss_string"),
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_library_id: Option<String>,

    /// Successfully published top-level destinations, including names chosen by conflict=rename.
    #[serde(rename = "resultEntries")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_entries: Option<Vec<models::FileSystemEntry>>,

    #[serde(rename = "processedCount")]
    #[validate(range(min = 0u64))]
    pub processed_count: u64,

    /// Total entries to process, when known up front.
    #[serde(rename = "totalCount")]
    #[validate(range(min = 0u64))]
    #[serde(deserialize_with = "deserialize_optional_nullable")]
    #[serde(default = "default_optional_nullable")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_count: Option<Nullable<u64>>,

    /// Entries left untouched by `conflict=skip`.
    #[serde(rename = "skippedCount")]
    #[validate(range(min = 0u64))]
    pub skipped_count: u64,

    #[serde(rename = "failedEntries")]
    #[validate(nested)]
    pub failed_entries: Vec<models::OperationFailure>,

    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,

    #[serde(rename = "finishedAt")]
    #[serde(deserialize_with = "deserialize_optional_nullable")]
    #[serde(default = "default_optional_nullable")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<Nullable<chrono::DateTime<chrono::Utc>>>,
}

lazy_static::lazy_static! {
    static ref RE_OPERATION_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}
lazy_static::lazy_static! {
    static ref RE_OPERATION_DESTINATION_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

impl Operation {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(
        id: String,
        kind: models::OperationKind,
        status: models::OperationStatus,
        library_id: String,
        processed_count: u64,
        skipped_count: u64,
        failed_entries: Vec<models::OperationFailure>,
        created_at: chrono::DateTime<chrono::Utc>,
    ) -> Operation {
        Operation {
            id,
            kind,
            status,
            library_id,
            destination_library_id: None,
            result_entries: None,
            processed_count,
            total_count: None,
            skipped_count,
            failed_entries,
            created_at,
            finished_at: None,
        }
    }
}

/// Converts the Operation value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("id".to_string()),
            Some(self.id.to_string()),
            // Skipping kind in query parameter serialization

            // Skipping status in query parameter serialization
            Some("libraryId".to_string()),
            Some(self.library_id.to_string()),
            self.destination_library_id
                .as_ref()
                .map(|destination_library_id| ["destinationLibraryId".to_string(), destination_library_id.to_string()].join(",")),
            // Skipping resultEntries in query parameter serialization
            Some("processedCount".to_string()),
            Some(self.processed_count.to_string()),
            self.total_count
                .as_ref()
                .map(|total_count| ["totalCount".to_string(), total_count.as_ref().map_or("null".to_string(), |x| x.to_string())].join(",")),
            Some("skippedCount".to_string()),
            Some(self.skipped_count.to_string()),
            // Skipping failedEntries in query parameter serialization

            // Skipping createdAt in query parameter serialization

            // Skipping finishedAt in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a Operation value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for Operation {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub id: Vec<String>,
            pub kind: Vec<models::OperationKind>,
            pub status: Vec<models::OperationStatus>,
            pub library_id: Vec<String>,
            pub destination_library_id: Vec<String>,
            pub result_entries: Vec<Vec<models::FileSystemEntry>>,
            pub processed_count: Vec<u64>,
            pub total_count: Vec<u64>,
            pub skipped_count: Vec<u64>,
            pub failed_entries: Vec<Vec<models::OperationFailure>>,
            pub created_at: Vec<chrono::DateTime<chrono::Utc>>,
            pub finished_at: Vec<chrono::DateTime<chrono::Utc>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing Operation".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "id" => intermediate_rep.id.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "kind" => intermediate_rep
                        .kind
                        .push(<models::OperationKind as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "status" => intermediate_rep
                        .status
                        .push(<models::OperationStatus as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "libraryId" => intermediate_rep.library_id.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "destinationLibraryId" => intermediate_rep
                        .destination_library_id
                        .push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    "resultEntries" => return std::result::Result::Err("Parsing a container in this style is not supported in Operation".to_string()),
                    #[allow(clippy::redundant_clone)]
                    "processedCount" => intermediate_rep.processed_count.push(<u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    "totalCount" => return std::result::Result::Err("Parsing a nullable type in this style is not supported in Operation".to_string()),
                    #[allow(clippy::redundant_clone)]
                    "skippedCount" => intermediate_rep.skipped_count.push(<u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    "failedEntries" => return std::result::Result::Err("Parsing a container in this style is not supported in Operation".to_string()),
                    #[allow(clippy::redundant_clone)]
                    "createdAt" => intermediate_rep
                        .created_at
                        .push(<chrono::DateTime<chrono::Utc> as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    "finishedAt" => return std::result::Result::Err("Parsing a nullable type in this style is not supported in Operation".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing Operation".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(Operation {
            id: intermediate_rep.id.into_iter().next().ok_or_else(|| "id missing in Operation".to_string())?,
            kind: intermediate_rep.kind.into_iter().next().ok_or_else(|| "kind missing in Operation".to_string())?,
            status: intermediate_rep.status.into_iter().next().ok_or_else(|| "status missing in Operation".to_string())?,
            library_id: intermediate_rep.library_id.into_iter().next().ok_or_else(|| "libraryId missing in Operation".to_string())?,
            destination_library_id: intermediate_rep.destination_library_id.into_iter().next(),
            result_entries: intermediate_rep.result_entries.into_iter().next(),
            processed_count: intermediate_rep
                .processed_count
                .into_iter()
                .next()
                .ok_or_else(|| "processedCount missing in Operation".to_string())?,
            total_count: std::result::Result::Err("Nullable types not supported in Operation".to_string())?,
            skipped_count: intermediate_rep
                .skipped_count
                .into_iter()
                .next()
                .ok_or_else(|| "skippedCount missing in Operation".to_string())?,
            failed_entries: intermediate_rep
                .failed_entries
                .into_iter()
                .next()
                .ok_or_else(|| "failedEntries missing in Operation".to_string())?,
            created_at: intermediate_rep.created_at.into_iter().next().ok_or_else(|| "createdAt missing in Operation".to_string())?,
            finished_at: std::result::Result::Err("Nullable types not supported in Operation".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<Operation> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<Operation>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<Operation>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for Operation - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<Operation> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <Operation as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into Operation - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct OperationFailure {
    /// Absolute-looking path within the selected Library. The resolved path must never escape that library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096), custom(function = "check_xss_string"))]
    pub path: String,

    #[serde(rename = "kind")]
    #[validate(nested)]
    pub kind: models::ErrorKind,

    #[serde(rename = "message")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl OperationFailure {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(path: String, kind: models::ErrorKind) -> OperationFailure {
        OperationFailure { path, kind, message: None }
    }
}

/// Converts the OperationFailure value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for OperationFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("path".to_string()),
            Some(self.path.to_string()),
            // Skipping kind in query parameter serialization
            self.message.as_ref().map(|message| ["message".to_string(), message.to_string()].join(",")),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a OperationFailure value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for OperationFailure {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub path: Vec<String>,
            pub kind: Vec<models::ErrorKind>,
            pub message: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing OperationFailure".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "path" => intermediate_rep.path.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "kind" => intermediate_rep
                        .kind
                        .push(<models::ErrorKind as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "message" => intermediate_rep.message.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing OperationFailure".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(OperationFailure {
            path: intermediate_rep.path.into_iter().next().ok_or_else(|| "path missing in OperationFailure".to_string())?,
            kind: intermediate_rep.kind.into_iter().next().ok_or_else(|| "kind missing in OperationFailure".to_string())?,
            message: intermediate_rep.message.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<OperationFailure> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<OperationFailure>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<OperationFailure>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for OperationFailure - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<OperationFailure> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <OperationFailure as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into OperationFailure - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// Opaque daemon-issued operation identifier.
#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct OperationId(pub String);

impl validator::Validate for OperationId {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::convert::From<String> for OperationId {
    fn from(x: String) -> Self {
        OperationId(x)
    }
}

impl std::fmt::Display for OperationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for OperationId {
    type Err = std::string::ParseError;
    fn from_str(x: &str) -> std::result::Result<Self, Self::Err> {
        std::result::Result::Ok(OperationId(x.to_string()))
    }
}

impl std::convert::From<OperationId> for String {
    fn from(x: OperationId) -> Self {
        x.0
    }
}

impl std::ops::Deref for OperationId {
    type Target = String;
    fn deref(&self) -> &String {
        &self.0
    }
}

impl std::ops::DerefMut for OperationId {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types, clippy::large_enum_variant)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk_enum_derive::LabelledGenericEnum))]
pub enum OperationKind {
    #[serde(rename = "copy_files")]
    CopyFiles,
    #[serde(rename = "move_files")]
    MoveFiles,
    #[serde(rename = "delete_files")]
    DeleteFiles,
    #[serde(rename = "copy_folders")]
    CopyFolders,
    #[serde(rename = "move_folders")]
    MoveFolders,
    #[serde(rename = "delete_folders")]
    DeleteFolders,
}

impl validator::Validate for OperationKind {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for OperationKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            OperationKind::CopyFiles => write!(f, "copy_files"),
            OperationKind::MoveFiles => write!(f, "move_files"),
            OperationKind::DeleteFiles => write!(f, "delete_files"),
            OperationKind::CopyFolders => write!(f, "copy_folders"),
            OperationKind::MoveFolders => write!(f, "move_folders"),
            OperationKind::DeleteFolders => write!(f, "delete_folders"),
        }
    }
}

impl std::str::FromStr for OperationKind {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "copy_files" => std::result::Result::Ok(OperationKind::CopyFiles),
            "move_files" => std::result::Result::Ok(OperationKind::MoveFiles),
            "delete_files" => std::result::Result::Ok(OperationKind::DeleteFiles),
            "copy_folders" => std::result::Result::Ok(OperationKind::CopyFolders),
            "move_folders" => std::result::Result::Ok(OperationKind::MoveFolders),
            "delete_folders" => std::result::Result::Ok(OperationKind::DeleteFolders),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct OperationList {
    #[serde(rename = "operations")]
    #[validate(nested)]
    pub operations: Vec<models::Operation>,
}

impl OperationList {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(operations: Vec<models::Operation>) -> OperationList {
        OperationList { operations }
    }
}

/// Converts the OperationList value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for OperationList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping operations in query parameter serialization

        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a OperationList value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for OperationList {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub operations: Vec<Vec<models::Operation>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing OperationList".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "operations" => return std::result::Result::Err("Parsing a container in this style is not supported in OperationList".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing OperationList".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(OperationList {
            operations: intermediate_rep
                .operations
                .into_iter()
                .next()
                .ok_or_else(|| "operations missing in OperationList".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<OperationList> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<OperationList>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<OperationList>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for OperationList - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<OperationList> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <OperationList as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into OperationList - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct OperationResponse {
    #[serde(rename = "operation")]
    #[validate(nested)]
    pub operation: models::Operation,
}

impl OperationResponse {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(operation: models::Operation) -> OperationResponse {
        OperationResponse { operation }
    }
}

/// Converts the OperationResponse value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for OperationResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping operation in query parameter serialization

        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a OperationResponse value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for OperationResponse {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub operation: Vec<models::Operation>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing OperationResponse".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "operation" => intermediate_rep
                        .operation
                        .push(<models::Operation as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing OperationResponse".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(OperationResponse {
            operation: intermediate_rep
                .operation
                .into_iter()
                .next()
                .ok_or_else(|| "operation missing in OperationResponse".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<OperationResponse> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<OperationResponse>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<OperationResponse>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for OperationResponse - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<OperationResponse> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <OperationResponse as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into OperationResponse - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types, clippy::large_enum_variant)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk_enum_derive::LabelledGenericEnum))]
pub enum OperationStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "succeeded")]
    Succeeded,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
}

impl validator::Validate for OperationStatus {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for OperationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            OperationStatus::Pending => write!(f, "pending"),
            OperationStatus::Running => write!(f, "running"),
            OperationStatus::Succeeded => write!(f, "succeeded"),
            OperationStatus::Failed => write!(f, "failed"),
            OperationStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl std::str::FromStr for OperationStatus {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "pending" => std::result::Result::Ok(OperationStatus::Pending),
            "running" => std::result::Result::Ok(OperationStatus::Running),
            "succeeded" => std::result::Result::Ok(OperationStatus::Succeeded),
            "failed" => std::result::Result::Ok(OperationStatus::Failed),
            "cancelled" => std::result::Result::Ok(OperationStatus::Cancelled),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
        }
    }
}

/// When a viewer swaps the display image for the original image. `auto` swaps automatically; `onZoom` swaps only when the user zooms in.
/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types, clippy::large_enum_variant)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk_enum_derive::LabelledGenericEnum))]
pub enum OriginalImageMode {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "onZoom")]
    OnZoom,
}

impl validator::Validate for OriginalImageMode {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for OriginalImageMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            OriginalImageMode::Auto => write!(f, "auto"),
            OriginalImageMode::OnZoom => write!(f, "onZoom"),
        }
    }
}

impl std::str::FromStr for OriginalImageMode {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "auto" => std::result::Result::Ok(OriginalImageMode::Auto),
            "onZoom" => std::result::Result::Ok(OriginalImageMode::OnZoom),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct Password(pub String);

impl validator::Validate for Password {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::convert::From<String> for Password {
    fn from(x: String) -> Self {
        Password(x)
    }
}

impl std::fmt::Display for Password {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for Password {
    type Err = std::string::ParseError;
    fn from_str(x: &str) -> std::result::Result<Self, Self::Err> {
        std::result::Result::Ok(Password(x.to_string()))
    }
}

impl std::convert::From<Password> for String {
    fn from(x: Password) -> Self {
        x.0
    }
}

impl std::ops::Deref for Password {
    type Target = String;
    fn deref(&self) -> &String {
        &self.0
    }
}

impl std::ops::DerefMut for Password {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

/// Display preferences. `wide` and `narrow` are chosen by the width of the screen (768 CSS px); `sort` and `timelineLibraries` do not depend on it.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct Preferences {
    #[serde(rename = "wide")]
    #[validate(nested)]
    pub wide: models::DisplaySet,

    #[serde(rename = "narrow")]
    #[validate(nested)]
    pub narrow: models::DisplaySet,

    #[serde(rename = "sort")]
    #[validate(nested)]
    pub sort: models::SortPreference,

    /// Libraries the timeline shows. Absent means every Library. Ids of Libraries that no longer exist are kept as saved, and clients ignore them.
    #[serde(rename = "timelineLibraries")]
    #[validate(length(min = 1, max = 64), nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeline_libraries: Option<Vec<models::LibraryId>>,
}

impl Preferences {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(wide: models::DisplaySet, narrow: models::DisplaySet, sort: models::SortPreference) -> Preferences {
        Preferences {
            wide,
            narrow,
            sort,
            timeline_libraries: None,
        }
    }
}

/// Converts the Preferences value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for Preferences {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping wide in query parameter serialization

            // Skipping narrow in query parameter serialization

            // Skipping sort in query parameter serialization
            self.timeline_libraries.as_ref().map(|timeline_libraries| {
                [
                    "timelineLibraries".to_string(),
                    timeline_libraries.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","),
                ]
                .join(",")
            }),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a Preferences value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for Preferences {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub wide: Vec<models::DisplaySet>,
            pub narrow: Vec<models::DisplaySet>,
            pub sort: Vec<models::SortPreference>,
            pub timeline_libraries: Vec<Vec<models::LibraryId>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing Preferences".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "wide" => intermediate_rep
                        .wide
                        .push(<models::DisplaySet as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "narrow" => intermediate_rep
                        .narrow
                        .push(<models::DisplaySet as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "sort" => intermediate_rep
                        .sort
                        .push(<models::SortPreference as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    "timelineLibraries" => return std::result::Result::Err("Parsing a container in this style is not supported in Preferences".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing Preferences".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(Preferences {
            wide: intermediate_rep.wide.into_iter().next().ok_or_else(|| "wide missing in Preferences".to_string())?,
            narrow: intermediate_rep.narrow.into_iter().next().ok_or_else(|| "narrow missing in Preferences".to_string())?,
            sort: intermediate_rep.sort.into_iter().next().ok_or_else(|| "sort missing in Preferences".to_string())?,
            timeline_libraries: intermediate_rep.timeline_libraries.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<Preferences> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<Preferences>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<Preferences>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for Preferences - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<Preferences> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <Preferences as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into Preferences - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct PutEntryTagsRequest {
    /// Absolute-looking path within the selected Library. The resolved path must never escape that library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096), custom(function = "check_xss_string"))]
    pub path: String,

    #[serde(rename = "tags")]
    #[validate(length(max = 32), nested)]
    pub tags: Vec<models::TagName>,
}

impl PutEntryTagsRequest {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(path: String, tags: Vec<models::TagName>) -> PutEntryTagsRequest {
        PutEntryTagsRequest { path, tags }
    }
}

/// Converts the PutEntryTagsRequest value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for PutEntryTagsRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("path".to_string()),
            Some(self.path.to_string()),
            Some("tags".to_string()),
            Some(self.tags.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a PutEntryTagsRequest value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for PutEntryTagsRequest {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub path: Vec<String>,
            pub tags: Vec<Vec<models::TagName>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing PutEntryTagsRequest".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "path" => intermediate_rep.path.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    "tags" => return std::result::Result::Err("Parsing a container in this style is not supported in PutEntryTagsRequest".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing PutEntryTagsRequest".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(PutEntryTagsRequest {
            path: intermediate_rep.path.into_iter().next().ok_or_else(|| "path missing in PutEntryTagsRequest".to_string())?,
            tags: intermediate_rep.tags.into_iter().next().ok_or_else(|| "tags missing in PutEntryTagsRequest".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<PutEntryTagsRequest> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<PutEntryTagsRequest>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<PutEntryTagsRequest>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for PutEntryTagsRequest - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<PutEntryTagsRequest> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <PutEntryTagsRequest as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into PutEntryTagsRequest - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct PutRatingRequest {
    /// Absolute-looking path within the selected Library. The resolved path must never escape that library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096), custom(function = "check_xss_string"))]
    pub path: String,

    #[serde(rename = "rating")]
    #[validate(nested)]
    pub rating: models::RatingValue,
}

impl PutRatingRequest {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(path: String, rating: models::RatingValue) -> PutRatingRequest {
        PutRatingRequest { path, rating }
    }
}

/// Converts the PutRatingRequest value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for PutRatingRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("path".to_string()),
            Some(self.path.to_string()),
            // Skipping rating in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a PutRatingRequest value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for PutRatingRequest {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub path: Vec<String>,
            pub rating: Vec<models::RatingValue>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing PutRatingRequest".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "path" => intermediate_rep.path.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "rating" => intermediate_rep
                        .rating
                        .push(<models::RatingValue as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing PutRatingRequest".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(PutRatingRequest {
            path: intermediate_rep.path.into_iter().next().ok_or_else(|| "path missing in PutRatingRequest".to_string())?,
            rating: intermediate_rep.rating.into_iter().next().ok_or_else(|| "rating missing in PutRatingRequest".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<PutRatingRequest> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<PutRatingRequest>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<PutRatingRequest>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for PutRatingRequest - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<PutRatingRequest> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <PutRatingRequest as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into PutRatingRequest - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct RatingResponse {
    #[serde(rename = "rating")]
    #[validate(nested)]
    pub rating: models::RatingValue,
}

impl RatingResponse {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(rating: models::RatingValue) -> RatingResponse {
        RatingResponse { rating }
    }
}

/// Converts the RatingResponse value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for RatingResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping rating in query parameter serialization

        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a RatingResponse value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for RatingResponse {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub rating: Vec<models::RatingValue>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing RatingResponse".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "rating" => intermediate_rep
                        .rating
                        .push(<models::RatingValue as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing RatingResponse".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(RatingResponse {
            rating: intermediate_rep.rating.into_iter().next().ok_or_else(|| "rating missing in RatingResponse".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<RatingResponse> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<RatingResponse>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<RatingResponse>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for RatingResponse - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<RatingResponse> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <RatingResponse as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into RatingResponse - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// A rating. `none` means the entry has not been rated.
/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types, clippy::large_enum_variant)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk_enum_derive::LabelledGenericEnum))]
pub enum RatingValue {
    #[serde(rename = "good")]
    Good,
    #[serde(rename = "bad")]
    Bad,
    #[serde(rename = "none")]
    None,
}

impl validator::Validate for RatingValue {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for RatingValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            RatingValue::Good => write!(f, "good"),
            RatingValue::Bad => write!(f, "bad"),
            RatingValue::None => write!(f, "none"),
        }
    }
}

impl std::str::FromStr for RatingValue {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "good" => std::result::Result::Ok(RatingValue::Good),
            "bad" => std::result::Result::Ok(RatingValue::Bad),
            "none" => std::result::Result::Ok(RatingValue::None),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
        }
    }
}

/// Absolute-looking path within the selected Library. The resolved path must never escape that library.
#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct RelativePath(pub String);

impl validator::Validate for RelativePath {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::convert::From<String> for RelativePath {
    fn from(x: String) -> Self {
        RelativePath(x)
    }
}

impl std::fmt::Display for RelativePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for RelativePath {
    type Err = std::string::ParseError;
    fn from_str(x: &str) -> std::result::Result<Self, Self::Err> {
        std::result::Result::Ok(RelativePath(x.to_string()))
    }
}

impl std::convert::From<RelativePath> for String {
    fn from(x: RelativePath) -> Self {
        x.0
    }
}

impl std::ops::Deref for RelativePath {
    type Target = String;
    fn deref(&self) -> &String {
        &self.0
    }
}

impl std::ops::DerefMut for RelativePath {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SearchEntry {
    /// Library identifier chosen by the operator in the daemon configuration file. Unlike the daemon-issued identifiers in this API it is a readable name, not a generated token. It is what the API uses in place of a host path, so the physical location of a Library is never exposed.  Because the value is used as a URL path segment, the character set is restricted. The daemon rejects a configuration that violates it at startup.
    #[serde(rename = "libraryId")]
    #[validate(
            length(min = 1, max = 64),
            regex(path = *RE_SEARCHENTRY_LIBRARY_ID),
          custom(function = "check_xss_string"),
    )]
    pub library_id: String,

    #[serde(rename = "entry")]
    #[validate(nested)]
    pub entry: models::FileSystemEntry,
}

lazy_static::lazy_static! {
    static ref RE_SEARCHENTRY_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

impl SearchEntry {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(library_id: String, entry: models::FileSystemEntry) -> SearchEntry {
        SearchEntry { library_id, entry }
    }
}

/// Converts the SearchEntry value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for SearchEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("libraryId".to_string()),
            Some(self.library_id.to_string()),
            // Skipping entry in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a SearchEntry value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for SearchEntry {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub library_id: Vec<String>,
            pub entry: Vec<models::FileSystemEntry>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing SearchEntry".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "libraryId" => intermediate_rep.library_id.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "entry" => intermediate_rep
                        .entry
                        .push(<models::FileSystemEntry as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing SearchEntry".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(SearchEntry {
            library_id: intermediate_rep
                .library_id
                .into_iter()
                .next()
                .ok_or_else(|| "libraryId missing in SearchEntry".to_string())?,
            entry: intermediate_rep.entry.into_iter().next().ok_or_else(|| "entry missing in SearchEntry".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<SearchEntry> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<SearchEntry>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<SearchEntry>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for SearchEntry - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<SearchEntry> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <SearchEntry as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into SearchEntry - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SearchListing {
    #[serde(rename = "entries")]
    #[validate(nested)]
    pub entries: Vec<models::SearchEntry>,

    /// Sum of the per-Library counts after filtering.
    #[serde(rename = "totalCount")]
    #[validate(range(min = 0u64))]
    pub total_count: u64,

    /// Opaque identifier of the merged listing. It changes when the listing of any searched Library is rebuilt.
    #[serde(rename = "snapshot")]
    #[validate(custom(function = "check_xss_string"))]
    pub snapshot: String,

    /// Libraries whose recursive traversal reached the entry limit.
    #[serde(rename = "truncatedLibraryIds")]
    #[validate(nested)]
    pub truncated_library_ids: Vec<models::LibraryId>,
}

impl SearchListing {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(entries: Vec<models::SearchEntry>, total_count: u64, snapshot: String, truncated_library_ids: Vec<models::LibraryId>) -> SearchListing {
        SearchListing {
            entries,
            total_count,
            snapshot,
            truncated_library_ids,
        }
    }
}

/// Converts the SearchListing value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for SearchListing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping entries in query parameter serialization
            Some("totalCount".to_string()),
            Some(self.total_count.to_string()),
            Some("snapshot".to_string()),
            Some(self.snapshot.to_string()),
            Some("truncatedLibraryIds".to_string()),
            Some(self.truncated_library_ids.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a SearchListing value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for SearchListing {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub entries: Vec<Vec<models::SearchEntry>>,
            pub total_count: Vec<u64>,
            pub snapshot: Vec<String>,
            pub truncated_library_ids: Vec<Vec<models::LibraryId>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing SearchListing".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "entries" => return std::result::Result::Err("Parsing a container in this style is not supported in SearchListing".to_string()),
                    #[allow(clippy::redundant_clone)]
                    "totalCount" => intermediate_rep.total_count.push(<u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "snapshot" => intermediate_rep.snapshot.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    "truncatedLibraryIds" => return std::result::Result::Err("Parsing a container in this style is not supported in SearchListing".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing SearchListing".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(SearchListing {
            entries: intermediate_rep.entries.into_iter().next().ok_or_else(|| "entries missing in SearchListing".to_string())?,
            total_count: intermediate_rep
                .total_count
                .into_iter()
                .next()
                .ok_or_else(|| "totalCount missing in SearchListing".to_string())?,
            snapshot: intermediate_rep
                .snapshot
                .into_iter()
                .next()
                .ok_or_else(|| "snapshot missing in SearchListing".to_string())?,
            truncated_library_ids: intermediate_rep
                .truncated_library_ids
                .into_iter()
                .next()
                .ok_or_else(|| "truncatedLibraryIds missing in SearchListing".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<SearchListing> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<SearchListing>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<SearchListing>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for SearchListing - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<SearchListing> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <SearchListing as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into SearchListing - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// Opaque daemon-issued session identifier.
#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SessionId(pub String);

impl validator::Validate for SessionId {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::convert::From<String> for SessionId {
    fn from(x: String) -> Self {
        SessionId(x)
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for SessionId {
    type Err = std::string::ParseError;
    fn from_str(x: &str) -> std::result::Result<Self, Self::Err> {
        std::result::Result::Ok(SessionId(x.to_string()))
    }
}

impl std::convert::From<SessionId> for String {
    fn from(x: SessionId) -> Self {
        x.0
    }
}

impl std::ops::Deref for SessionId {
    type Target = String;
    fn deref(&self) -> &String {
        &self.0
    }
}

impl std::ops::DerefMut for SessionId {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types, clippy::large_enum_variant)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk_enum_derive::LabelledGenericEnum))]
pub enum SessionStatus {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "expired")]
    Expired,
    #[serde(rename = "revoked")]
    Revoked,
}

impl validator::Validate for SessionStatus {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            SessionStatus::Active => write!(f, "active"),
            SessionStatus::Expired => write!(f, "expired"),
            SessionStatus::Revoked => write!(f, "revoked"),
        }
    }
}

impl std::str::FromStr for SessionStatus {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "active" => std::result::Result::Ok(SessionStatus::Active),
            "expired" => std::result::Result::Ok(SessionStatus::Expired),
            "revoked" => std::result::Result::Ok(SessionStatus::Revoked),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
        }
    }
}

/// How the client receives and sends the session credential.
/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types, clippy::large_enum_variant)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk_enum_derive::LabelledGenericEnum))]
pub enum SessionTransport {
    #[serde(rename = "cookie")]
    Cookie,
    #[serde(rename = "bearer")]
    Bearer,
}

impl validator::Validate for SessionTransport {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for SessionTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            SessionTransport::Cookie => write!(f, "cookie"),
            SessionTransport::Bearer => write!(f, "bearer"),
        }
    }
}

impl std::str::FromStr for SessionTransport {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "cookie" => std::result::Result::Ok(SessionTransport::Cookie),
            "bearer" => std::result::Result::Ok(SessionTransport::Bearer),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SetupStatus {
    /// Whether the initial owner exists.
    #[serde(rename = "initialized")]
    pub initialized: bool,
}

impl SetupStatus {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(initialized: bool) -> SetupStatus {
        SetupStatus { initialized }
    }
}

/// Converts the SetupStatus value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for SetupStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![Some("initialized".to_string()), Some(self.initialized.to_string())];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a SetupStatus value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for SetupStatus {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub initialized: Vec<bool>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing SetupStatus".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "initialized" => intermediate_rep.initialized.push(<bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing SetupStatus".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(SetupStatus {
            initialized: intermediate_rep
                .initialized
                .into_iter()
                .next()
                .ok_or_else(|| "initialized missing in SetupStatus".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<SetupStatus> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<SetupStatus>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<SetupStatus>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for SetupStatus - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<SetupStatus> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <SetupStatus as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into SetupStatus - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// Shooting information read from Exif. Every property is absent when the image has no Exif or the Exif does not carry it. Location is never returned.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ShootingInfo {
    /// Shooting time as `YYYY-MM-DDTHH:MM:SS` without a time zone.
    #[serde(rename = "takenAt")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub taken_at: Option<String>,

    #[serde(rename = "cameraMake")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera_make: Option<String>,

    #[serde(rename = "cameraModel")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera_model: Option<String>,

    #[serde(rename = "lensModel")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lens_model: Option<String>,

    #[serde(rename = "focalLengthMm")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focal_length_mm: Option<f64>,

    #[serde(rename = "fNumber")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub f_number: Option<f64>,

    #[serde(rename = "exposureTimeSecs")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exposure_time_secs: Option<f64>,

    #[serde(rename = "iso")]
    #[validate(range(min = 0u32))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iso: Option<u32>,
}

impl ShootingInfo {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new() -> ShootingInfo {
        ShootingInfo {
            taken_at: None,
            camera_make: None,
            camera_model: None,
            lens_model: None,
            focal_length_mm: None,
            f_number: None,
            exposure_time_secs: None,
            iso: None,
        }
    }
}

/// Converts the ShootingInfo value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for ShootingInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            self.taken_at.as_ref().map(|taken_at| ["takenAt".to_string(), taken_at.to_string()].join(",")),
            self.camera_make.as_ref().map(|camera_make| ["cameraMake".to_string(), camera_make.to_string()].join(",")),
            self.camera_model
                .as_ref()
                .map(|camera_model| ["cameraModel".to_string(), camera_model.to_string()].join(",")),
            self.lens_model.as_ref().map(|lens_model| ["lensModel".to_string(), lens_model.to_string()].join(",")),
            self.focal_length_mm
                .as_ref()
                .map(|focal_length_mm| ["focalLengthMm".to_string(), focal_length_mm.to_string()].join(",")),
            self.f_number.as_ref().map(|f_number| ["fNumber".to_string(), f_number.to_string()].join(",")),
            self.exposure_time_secs
                .as_ref()
                .map(|exposure_time_secs| ["exposureTimeSecs".to_string(), exposure_time_secs.to_string()].join(",")),
            self.iso.as_ref().map(|iso| ["iso".to_string(), iso.to_string()].join(",")),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a ShootingInfo value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for ShootingInfo {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub taken_at: Vec<String>,
            pub camera_make: Vec<String>,
            pub camera_model: Vec<String>,
            pub lens_model: Vec<String>,
            pub focal_length_mm: Vec<f64>,
            pub f_number: Vec<f64>,
            pub exposure_time_secs: Vec<f64>,
            pub iso: Vec<u32>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing ShootingInfo".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "takenAt" => intermediate_rep.taken_at.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "cameraMake" => intermediate_rep.camera_make.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "cameraModel" => intermediate_rep.camera_model.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "lensModel" => intermediate_rep.lens_model.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "focalLengthMm" => intermediate_rep.focal_length_mm.push(<f64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "fNumber" => intermediate_rep.f_number.push(<f64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "exposureTimeSecs" => intermediate_rep
                        .exposure_time_secs
                        .push(<f64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "iso" => intermediate_rep.iso.push(<u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing ShootingInfo".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(ShootingInfo {
            taken_at: intermediate_rep.taken_at.into_iter().next(),
            camera_make: intermediate_rep.camera_make.into_iter().next(),
            camera_model: intermediate_rep.camera_model.into_iter().next(),
            lens_model: intermediate_rep.lens_model.into_iter().next(),
            focal_length_mm: intermediate_rep.focal_length_mm.into_iter().next(),
            f_number: intermediate_rep.f_number.into_iter().next(),
            exposure_time_secs: intermediate_rep.exposure_time_secs.into_iter().next(),
            iso: intermediate_rep.iso.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<ShootingInfo> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<ShootingInfo>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<ShootingInfo>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for ShootingInfo - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<ShootingInfo> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <ShootingInfo as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into ShootingInfo - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// Same values as the `sort` query parameter.
/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types, clippy::large_enum_variant)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk_enum_derive::LabelledGenericEnum))]
pub enum SortKey {
    #[serde(rename = "name")]
    Name,
    #[serde(rename = "kind")]
    Kind,
    #[serde(rename = "size")]
    Size,
    #[serde(rename = "modified_at")]
    ModifiedAt,
}

impl validator::Validate for SortKey {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for SortKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            SortKey::Name => write!(f, "name"),
            SortKey::Kind => write!(f, "kind"),
            SortKey::Size => write!(f, "size"),
            SortKey::ModifiedAt => write!(f, "modified_at"),
        }
    }
}

impl std::str::FromStr for SortKey {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "name" => std::result::Result::Ok(SortKey::Name),
            "kind" => std::result::Result::Ok(SortKey::Kind),
            "size" => std::result::Result::Ok(SortKey::Size),
            "modified_at" => std::result::Result::Ok(SortKey::ModifiedAt),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
        }
    }
}

/// Same values as the `order` query parameter.
/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types, clippy::large_enum_variant)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk_enum_derive::LabelledGenericEnum))]
pub enum SortOrder {
    #[serde(rename = "asc")]
    Asc,
    #[serde(rename = "desc")]
    Desc,
}

impl validator::Validate for SortOrder {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for SortOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            SortOrder::Asc => write!(f, "asc"),
            SortOrder::Desc => write!(f, "desc"),
        }
    }
}

impl std::str::FromStr for SortOrder {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "asc" => std::result::Result::Ok(SortOrder::Asc),
            "desc" => std::result::Result::Ok(SortOrder::Desc),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SortPreference {
    #[serde(rename = "key")]
    #[validate(nested)]
    pub key: models::SortKey,

    #[serde(rename = "order")]
    #[validate(nested)]
    pub order: models::SortOrder,
}

impl SortPreference {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(key: models::SortKey, order: models::SortOrder) -> SortPreference {
        SortPreference { key, order }
    }
}

/// Converts the SortPreference value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for SortPreference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping key in query parameter serialization

            // Skipping order in query parameter serialization

        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a SortPreference value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for SortPreference {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub key: Vec<models::SortKey>,
            pub order: Vec<models::SortOrder>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing SortPreference".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "key" => intermediate_rep.key.push(<models::SortKey as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "order" => intermediate_rep
                        .order
                        .push(<models::SortOrder as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing SortPreference".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(SortPreference {
            key: intermediate_rep.key.into_iter().next().ok_or_else(|| "key missing in SortPreference".to_string())?,
            order: intermediate_rep.order.into_iter().next().ok_or_else(|| "order missing in SortPreference".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<SortPreference> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<SortPreference>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<SortPreference>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for SortPreference - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<SortPreference> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <SortPreference as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into SortPreference - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// A tag word of 1 to 64 characters after trimming, without control characters. Upper and lower case are different words.
#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct TagName(pub String);

impl validator::Validate for TagName {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::convert::From<String> for TagName {
    fn from(x: String) -> Self {
        TagName(x)
    }
}

impl std::fmt::Display for TagName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for TagName {
    type Err = std::string::ParseError;
    fn from_str(x: &str) -> std::result::Result<Self, Self::Err> {
        std::result::Result::Ok(TagName(x.to_string()))
    }
}

impl std::convert::From<TagName> for String {
    fn from(x: TagName) -> Self {
        x.0
    }
}

impl std::ops::Deref for TagName {
    type Target = String;
    fn deref(&self) -> &String {
        &self.0
    }
}

impl std::ops::DerefMut for TagName {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct TagsResponse {
    #[serde(rename = "tags")]
    #[validate(nested)]
    pub tags: Vec<models::TagName>,
}

impl TagsResponse {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(tags: Vec<models::TagName>) -> TagsResponse {
        TagsResponse { tags }
    }
}

/// Converts the TagsResponse value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for TagsResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![Some("tags".to_string()), Some(self.tags.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a TagsResponse value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for TagsResponse {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub tags: Vec<Vec<models::TagName>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing TagsResponse".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "tags" => return std::result::Result::Err("Parsing a container in this style is not supported in TagsResponse".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing TagsResponse".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(TagsResponse {
            tags: intermediate_rep.tags.into_iter().next().ok_or_else(|| "tags missing in TagsResponse".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<TagsResponse> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<TagsResponse>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<TagsResponse>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for TagsResponse - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<TagsResponse> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <TagsResponse as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into TagsResponse - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types, clippy::large_enum_variant)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk_enum_derive::LabelledGenericEnum))]
pub enum ThumbnailShape {
    #[serde(rename = "crop")]
    Crop,
    #[serde(rename = "fit")]
    Fit,
}

impl validator::Validate for ThumbnailShape {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for ThumbnailShape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            ThumbnailShape::Crop => write!(f, "crop"),
            ThumbnailShape::Fit => write!(f, "fit"),
        }
    }
}

impl std::str::FromStr for ThumbnailShape {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "crop" => std::result::Result::Ok(ThumbnailShape::Crop),
            "fit" => std::result::Result::Ok(ThumbnailShape::Fit),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
        }
    }
}

/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types, clippy::large_enum_variant)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk_enum_derive::LabelledGenericEnum))]
pub enum ThumbnailVariant {
    #[serde(rename = "thumbnail")]
    Thumbnail,
    #[serde(rename = "display")]
    Display,
    #[serde(rename = "sprite")]
    Sprite,
}

impl validator::Validate for ThumbnailVariant {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for ThumbnailVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            ThumbnailVariant::Thumbnail => write!(f, "thumbnail"),
            ThumbnailVariant::Display => write!(f, "display"),
            ThumbnailVariant::Sprite => write!(f, "sprite"),
        }
    }
}

impl std::str::FromStr for ThumbnailVariant {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "thumbnail" => std::result::Result::Ok(ThumbnailVariant::Thumbnail),
            "display" => std::result::Result::Ok(ThumbnailVariant::Display),
            "sprite" => std::result::Result::Ok(ThumbnailVariant::Sprite),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct TimelineEntry {
    /// Library identifier chosen by the operator in the daemon configuration file. Unlike the daemon-issued identifiers in this API it is a readable name, not a generated token. It is what the API uses in place of a host path, so the physical location of a Library is never exposed.  Because the value is used as a URL path segment, the character set is restricted. The daemon rejects a configuration that violates it at startup.
    #[serde(rename = "libraryId")]
    #[validate(
            length(min = 1, max = 64),
            regex(path = *RE_TIMELINEENTRY_LIBRARY_ID),
          custom(function = "check_xss_string"),
    )]
    pub library_id: String,

    #[serde(rename = "entry")]
    #[validate(nested)]
    pub entry: models::FileSystemEntry,

    /// Date the entry is ordered by, as `YYYY-MM-DDTHH:MM:SS` without a time zone, in the daemon's configured time zone.
    #[serde(rename = "timelineDate")]
    #[validate(custom(function = "check_xss_string"))]
    pub timeline_date: String,

    /// Where `timelineDate` came from.
    /// Note: inline enums are not fully supported by openapi-generator
    #[serde(rename = "timelineDateSource")]
    #[validate(custom(function = "check_xss_string"))]
    pub timeline_date_source: String,
}

lazy_static::lazy_static! {
    static ref RE_TIMELINEENTRY_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

impl TimelineEntry {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(library_id: String, entry: models::FileSystemEntry, timeline_date: String, timeline_date_source: String) -> TimelineEntry {
        TimelineEntry {
            library_id,
            entry,
            timeline_date,
            timeline_date_source,
        }
    }
}

/// Converts the TimelineEntry value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for TimelineEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("libraryId".to_string()),
            Some(self.library_id.to_string()),
            // Skipping entry in query parameter serialization
            Some("timelineDate".to_string()),
            Some(self.timeline_date.to_string()),
            Some("timelineDateSource".to_string()),
            Some(self.timeline_date_source.to_string()),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a TimelineEntry value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for TimelineEntry {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub library_id: Vec<String>,
            pub entry: Vec<models::FileSystemEntry>,
            pub timeline_date: Vec<String>,
            pub timeline_date_source: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing TimelineEntry".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "libraryId" => intermediate_rep.library_id.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "entry" => intermediate_rep
                        .entry
                        .push(<models::FileSystemEntry as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "timelineDate" => intermediate_rep
                        .timeline_date
                        .push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "timelineDateSource" => intermediate_rep
                        .timeline_date_source
                        .push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing TimelineEntry".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(TimelineEntry {
            library_id: intermediate_rep
                .library_id
                .into_iter()
                .next()
                .ok_or_else(|| "libraryId missing in TimelineEntry".to_string())?,
            entry: intermediate_rep.entry.into_iter().next().ok_or_else(|| "entry missing in TimelineEntry".to_string())?,
            timeline_date: intermediate_rep
                .timeline_date
                .into_iter()
                .next()
                .ok_or_else(|| "timelineDate missing in TimelineEntry".to_string())?,
            timeline_date_source: intermediate_rep
                .timeline_date_source
                .into_iter()
                .next()
                .ok_or_else(|| "timelineDateSource missing in TimelineEntry".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<TimelineEntry> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<TimelineEntry>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<TimelineEntry>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for TimelineEntry - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<TimelineEntry> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <TimelineEntry as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into TimelineEntry - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct TimelineListing {
    #[serde(rename = "entries")]
    #[validate(nested)]
    pub entries: Vec<models::TimelineEntry>,

    /// Cursor for the page of newer entries. Absent when there are none.
    #[serde(rename = "previous")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous: Option<String>,

    /// Cursor for the page of older entries. Absent when there are none.
    #[serde(rename = "next")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
}

impl TimelineListing {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(entries: Vec<models::TimelineEntry>) -> TimelineListing {
        TimelineListing {
            entries,
            previous: None,
            next: None,
        }
    }
}

/// Converts the TimelineListing value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for TimelineListing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping entries in query parameter serialization
            self.previous.as_ref().map(|previous| ["previous".to_string(), previous.to_string()].join(",")),
            self.next.as_ref().map(|next| ["next".to_string(), next.to_string()].join(",")),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a TimelineListing value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for TimelineListing {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub entries: Vec<Vec<models::TimelineEntry>>,
            pub previous: Vec<String>,
            pub next: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing TimelineListing".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "entries" => return std::result::Result::Err("Parsing a container in this style is not supported in TimelineListing".to_string()),
                    #[allow(clippy::redundant_clone)]
                    "previous" => intermediate_rep.previous.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "next" => intermediate_rep.next.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing TimelineListing".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(TimelineListing {
            entries: intermediate_rep
                .entries
                .into_iter()
                .next()
                .ok_or_else(|| "entries missing in TimelineListing".to_string())?,
            previous: intermediate_rep.previous.into_iter().next(),
            next: intermediate_rep.next.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<TimelineListing> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<TimelineListing>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<TimelineListing>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for TimelineListing - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<TimelineListing> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <TimelineListing as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into TimelineListing - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct TimelineMonth {
    #[serde(rename = "month")]
    #[validate(
            regex(path = *RE_TIMELINEMONTH_MONTH),
          custom(function = "check_xss_string"),
    )]
    pub month: String,

    #[serde(rename = "count")]
    #[validate(range(min = 1u64))]
    pub count: u64,
}

lazy_static::lazy_static! {
    static ref RE_TIMELINEMONTH_MONTH: regex::Regex = regex::Regex::new("^[0-9]{4}-[0-9]{2}$").unwrap();
}

impl TimelineMonth {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(month: String, count: u64) -> TimelineMonth {
        TimelineMonth { month, count }
    }
}

/// Converts the TimelineMonth value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for TimelineMonth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("month".to_string()),
            Some(self.month.to_string()),
            Some("count".to_string()),
            Some(self.count.to_string()),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a TimelineMonth value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for TimelineMonth {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub month: Vec<String>,
            pub count: Vec<u64>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing TimelineMonth".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "month" => intermediate_rep.month.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "count" => intermediate_rep.count.push(<u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing TimelineMonth".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(TimelineMonth {
            month: intermediate_rep.month.into_iter().next().ok_or_else(|| "month missing in TimelineMonth".to_string())?,
            count: intermediate_rep.count.into_iter().next().ok_or_else(|| "count missing in TimelineMonth".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<TimelineMonth> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<TimelineMonth>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<TimelineMonth>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for TimelineMonth - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<TimelineMonth> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <TimelineMonth as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into TimelineMonth - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct TimelineMonths {
    #[serde(rename = "months")]
    #[validate(nested)]
    pub months: Vec<models::TimelineMonth>,
}

impl TimelineMonths {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(months: Vec<models::TimelineMonth>) -> TimelineMonths {
        TimelineMonths { months }
    }
}

/// Converts the TimelineMonths value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for TimelineMonths {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping months in query parameter serialization

        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a TimelineMonths value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for TimelineMonths {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub months: Vec<Vec<models::TimelineMonth>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing TimelineMonths".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "months" => return std::result::Result::Err("Parsing a container in this style is not supported in TimelineMonths".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing TimelineMonths".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(TimelineMonths {
            months: intermediate_rep.months.into_iter().next().ok_or_else(|| "months missing in TimelineMonths".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<TimelineMonths> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<TimelineMonths>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<TimelineMonths>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for TimelineMonths - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<TimelineMonths> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <TimelineMonths as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into TimelineMonths - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct TimelineRefreshRequest {
    #[serde(rename = "libraryIds")]
    #[validate(length(min = 1, max = 64), nested)]
    pub library_ids: Vec<models::LibraryId>,
}

impl TimelineRefreshRequest {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(library_ids: Vec<models::LibraryId>) -> TimelineRefreshRequest {
        TimelineRefreshRequest { library_ids }
    }
}

/// Converts the TimelineRefreshRequest value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for TimelineRefreshRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("libraryIds".to_string()),
            Some(self.library_ids.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a TimelineRefreshRequest value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for TimelineRefreshRequest {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub library_ids: Vec<Vec<models::LibraryId>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing TimelineRefreshRequest".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "libraryIds" => return std::result::Result::Err("Parsing a container in this style is not supported in TimelineRefreshRequest".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing TimelineRefreshRequest".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(TimelineRefreshRequest {
            library_ids: intermediate_rep
                .library_ids
                .into_iter()
                .next()
                .ok_or_else(|| "libraryIds missing in TimelineRefreshRequest".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<TimelineRefreshRequest> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<TimelineRefreshRequest>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<TimelineRefreshRequest>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for TimelineRefreshRequest - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<TimelineRefreshRequest> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <TimelineRefreshRequest as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into TimelineRefreshRequest - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct TimelineStatus {
    /// Library identifier chosen by the operator in the daemon configuration file. Unlike the daemon-issued identifiers in this API it is a readable name, not a generated token. It is what the API uses in place of a host path, so the physical location of a Library is never exposed.  Because the value is used as a URL path segment, the character set is restricted. The daemon rejects a configuration that violates it at startup.
    #[serde(rename = "libraryId")]
    #[validate(
            length(min = 1, max = 64),
            regex(path = *RE_TIMELINESTATUS_LIBRARY_ID),
          custom(function = "check_xss_string"),
    )]
    pub library_id: String,

    /// True while a full scan of the Library is running.
    #[serde(rename = "running")]
    pub running: bool,

    /// Files and archive entries processed by the running scan.
    #[serde(rename = "processedFiles")]
    #[validate(range(min = 0u64))]
    pub processed_files: u64,

    /// When the last full scan finished. Absent before the first one.
    #[serde(rename = "completedAt")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

lazy_static::lazy_static! {
    static ref RE_TIMELINESTATUS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

impl TimelineStatus {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(library_id: String, running: bool, processed_files: u64) -> TimelineStatus {
        TimelineStatus {
            library_id,
            running,
            processed_files,
            completed_at: None,
        }
    }
}

/// Converts the TimelineStatus value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for TimelineStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("libraryId".to_string()),
            Some(self.library_id.to_string()),
            Some("running".to_string()),
            Some(self.running.to_string()),
            Some("processedFiles".to_string()),
            Some(self.processed_files.to_string()),
            // Skipping completedAt in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a TimelineStatus value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for TimelineStatus {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub library_id: Vec<String>,
            pub running: Vec<bool>,
            pub processed_files: Vec<u64>,
            pub completed_at: Vec<chrono::DateTime<chrono::Utc>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing TimelineStatus".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "libraryId" => intermediate_rep.library_id.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "running" => intermediate_rep.running.push(<bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "processedFiles" => intermediate_rep.processed_files.push(<u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "completedAt" => intermediate_rep
                        .completed_at
                        .push(<chrono::DateTime<chrono::Utc> as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing TimelineStatus".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(TimelineStatus {
            library_id: intermediate_rep
                .library_id
                .into_iter()
                .next()
                .ok_or_else(|| "libraryId missing in TimelineStatus".to_string())?,
            running: intermediate_rep.running.into_iter().next().ok_or_else(|| "running missing in TimelineStatus".to_string())?,
            processed_files: intermediate_rep
                .processed_files
                .into_iter()
                .next()
                .ok_or_else(|| "processedFiles missing in TimelineStatus".to_string())?,
            completed_at: intermediate_rep.completed_at.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<TimelineStatus> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<TimelineStatus>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<TimelineStatus>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for TimelineStatus - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<TimelineStatus> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <TimelineStatus as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into TimelineStatus - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct TimelineStatuses {
    #[serde(rename = "libraries")]
    #[validate(nested)]
    pub libraries: Vec<models::TimelineStatus>,
}

impl TimelineStatuses {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(libraries: Vec<models::TimelineStatus>) -> TimelineStatuses {
        TimelineStatuses { libraries }
    }
}

/// Converts the TimelineStatuses value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for TimelineStatuses {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping libraries in query parameter serialization

        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a TimelineStatuses value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for TimelineStatuses {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub libraries: Vec<Vec<models::TimelineStatus>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing TimelineStatuses".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "libraries" => return std::result::Result::Err("Parsing a container in this style is not supported in TimelineStatuses".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing TimelineStatuses".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(TimelineStatuses {
            libraries: intermediate_rep
                .libraries
                .into_iter()
                .next()
                .ok_or_else(|| "libraries missing in TimelineStatuses".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<TimelineStatuses> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<TimelineStatuses>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<TimelineStatuses>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for TimelineStatuses - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<TimelineStatuses> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <TimelineStatuses as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into TimelineStatuses - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct Upload {
    /// Opaque daemon-issued upload session identifier.
    #[serde(rename = "id")]
    #[validate(length(min = 1), custom(function = "check_xss_string"))]
    pub id: String,

    /// Absolute-looking path within the selected Library. The resolved path must never escape that library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096), custom(function = "check_xss_string"))]
    pub path: String,

    #[serde(rename = "size")]
    #[validate(range(min = 0u64))]
    pub size: u64,

    /// Bytes accepted so far; the next chunk's `offset`.
    #[serde(rename = "committedSize")]
    #[validate(range(min = 0u64))]
    pub committed_size: u64,

    #[serde(rename = "status")]
    #[validate(nested)]
    pub status: models::UploadStatus,

    #[serde(rename = "expiresAt")]
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

impl Upload {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(id: String, path: String, size: u64, committed_size: u64, status: models::UploadStatus, expires_at: chrono::DateTime<chrono::Utc>) -> Upload {
        Upload {
            id,
            path,
            size,
            committed_size,
            status,
            expires_at,
        }
    }
}

/// Converts the Upload value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for Upload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("id".to_string()),
            Some(self.id.to_string()),
            Some("path".to_string()),
            Some(self.path.to_string()),
            Some("size".to_string()),
            Some(self.size.to_string()),
            Some("committedSize".to_string()),
            Some(self.committed_size.to_string()),
            // Skipping status in query parameter serialization

            // Skipping expiresAt in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a Upload value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for Upload {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub id: Vec<String>,
            pub path: Vec<String>,
            pub size: Vec<u64>,
            pub committed_size: Vec<u64>,
            pub status: Vec<models::UploadStatus>,
            pub expires_at: Vec<chrono::DateTime<chrono::Utc>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing Upload".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "id" => intermediate_rep.id.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "path" => intermediate_rep.path.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "size" => intermediate_rep.size.push(<u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "committedSize" => intermediate_rep.committed_size.push(<u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "status" => intermediate_rep
                        .status
                        .push(<models::UploadStatus as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "expiresAt" => intermediate_rep
                        .expires_at
                        .push(<chrono::DateTime<chrono::Utc> as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing Upload".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(Upload {
            id: intermediate_rep.id.into_iter().next().ok_or_else(|| "id missing in Upload".to_string())?,
            path: intermediate_rep.path.into_iter().next().ok_or_else(|| "path missing in Upload".to_string())?,
            size: intermediate_rep.size.into_iter().next().ok_or_else(|| "size missing in Upload".to_string())?,
            committed_size: intermediate_rep
                .committed_size
                .into_iter()
                .next()
                .ok_or_else(|| "committedSize missing in Upload".to_string())?,
            status: intermediate_rep.status.into_iter().next().ok_or_else(|| "status missing in Upload".to_string())?,
            expires_at: intermediate_rep.expires_at.into_iter().next().ok_or_else(|| "expiresAt missing in Upload".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<Upload> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<Upload>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<Upload>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for Upload - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<Upload> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <Upload as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into Upload - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct UploadCreateRequest {
    /// Absolute-looking path within the selected Library. The resolved path must never escape that library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096), custom(function = "check_xss_string"))]
    pub path: String,

    /// Final file size in bytes, verified at commit.
    #[serde(rename = "size")]
    #[validate(range(min = 0u64))]
    pub size: u64,

    #[serde(rename = "conflict")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict: Option<models::ConflictPolicy>,
}

impl UploadCreateRequest {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(path: String, size: u64) -> UploadCreateRequest {
        UploadCreateRequest { path, size, conflict: None }
    }
}

/// Converts the UploadCreateRequest value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for UploadCreateRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("path".to_string()),
            Some(self.path.to_string()),
            Some("size".to_string()),
            Some(self.size.to_string()),
            // Skipping conflict in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a UploadCreateRequest value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for UploadCreateRequest {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub path: Vec<String>,
            pub size: Vec<u64>,
            pub conflict: Vec<models::ConflictPolicy>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing UploadCreateRequest".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "path" => intermediate_rep.path.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "size" => intermediate_rep.size.push(<u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "conflict" => intermediate_rep
                        .conflict
                        .push(<models::ConflictPolicy as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing UploadCreateRequest".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(UploadCreateRequest {
            path: intermediate_rep.path.into_iter().next().ok_or_else(|| "path missing in UploadCreateRequest".to_string())?,
            size: intermediate_rep.size.into_iter().next().ok_or_else(|| "size missing in UploadCreateRequest".to_string())?,
            conflict: intermediate_rep.conflict.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<UploadCreateRequest> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<UploadCreateRequest>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<UploadCreateRequest>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for UploadCreateRequest - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<UploadCreateRequest> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <UploadCreateRequest as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into UploadCreateRequest - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// Opaque daemon-issued upload session identifier.
#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct UploadId(pub String);

impl validator::Validate for UploadId {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::convert::From<String> for UploadId {
    fn from(x: String) -> Self {
        UploadId(x)
    }
}

impl std::fmt::Display for UploadId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for UploadId {
    type Err = std::string::ParseError;
    fn from_str(x: &str) -> std::result::Result<Self, Self::Err> {
        std::result::Result::Ok(UploadId(x.to_string()))
    }
}

impl std::convert::From<UploadId> for String {
    fn from(x: UploadId) -> Self {
        x.0
    }
}

impl std::ops::Deref for UploadId {
    type Target = String;
    fn deref(&self) -> &String {
        &self.0
    }
}

impl std::ops::DerefMut for UploadId {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct UploadResponse {
    #[serde(rename = "upload")]
    #[validate(nested)]
    pub upload: models::Upload,
}

impl UploadResponse {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(upload: models::Upload) -> UploadResponse {
        UploadResponse { upload }
    }
}

/// Converts the UploadResponse value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for UploadResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping upload in query parameter serialization

        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a UploadResponse value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for UploadResponse {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub upload: Vec<models::Upload>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing UploadResponse".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "upload" => intermediate_rep
                        .upload
                        .push(<models::Upload as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing UploadResponse".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(UploadResponse {
            upload: intermediate_rep.upload.into_iter().next().ok_or_else(|| "upload missing in UploadResponse".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<UploadResponse> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<UploadResponse>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<UploadResponse>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for UploadResponse - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<UploadResponse> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <UploadResponse as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into UploadResponse - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types, clippy::large_enum_variant)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk_enum_derive::LabelledGenericEnum))]
pub enum UploadStatus {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "committed")]
    Committed,
    #[serde(rename = "aborted")]
    Aborted,
    #[serde(rename = "expired")]
    Expired,
}

impl validator::Validate for UploadStatus {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for UploadStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            UploadStatus::Active => write!(f, "active"),
            UploadStatus::Committed => write!(f, "committed"),
            UploadStatus::Aborted => write!(f, "aborted"),
            UploadStatus::Expired => write!(f, "expired"),
        }
    }
}

impl std::str::FromStr for UploadStatus {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "active" => std::result::Result::Ok(UploadStatus::Active),
            "committed" => std::result::Result::Ok(UploadStatus::Committed),
            "aborted" => std::result::Result::Ok(UploadStatus::Aborted),
            "expired" => std::result::Result::Ok(UploadStatus::Expired),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct User {
    /// Opaque daemon-issued user identifier.
    #[serde(rename = "id")]
    #[validate(length(min = 1), custom(function = "check_xss_string"))]
    pub id: String,

    #[serde(rename = "username")]
    #[validate(
            length(min = 1, max = 128),
            regex(path = *RE_USER_USERNAME),
          custom(function = "check_xss_string"),
    )]
    pub username: String,

    #[serde(rename = "role")]
    #[validate(nested)]
    pub role: models::UserRole,

    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,

    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

lazy_static::lazy_static! {
    static ref RE_USER_USERNAME: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

impl User {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(id: String, username: String, role: models::UserRole, created_at: chrono::DateTime<chrono::Utc>, updated_at: chrono::DateTime<chrono::Utc>) -> User {
        User {
            id,
            username,
            role,
            created_at,
            updated_at,
        }
    }
}

/// Converts the User value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("id".to_string()),
            Some(self.id.to_string()),
            Some("username".to_string()),
            Some(self.username.to_string()),
            // Skipping role in query parameter serialization

            // Skipping createdAt in query parameter serialization

            // Skipping updatedAt in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a User value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for User {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub id: Vec<String>,
            pub username: Vec<String>,
            pub role: Vec<models::UserRole>,
            pub created_at: Vec<chrono::DateTime<chrono::Utc>>,
            pub updated_at: Vec<chrono::DateTime<chrono::Utc>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing User".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "id" => intermediate_rep.id.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "username" => intermediate_rep.username.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "role" => intermediate_rep
                        .role
                        .push(<models::UserRole as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "createdAt" => intermediate_rep
                        .created_at
                        .push(<chrono::DateTime<chrono::Utc> as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "updatedAt" => intermediate_rep
                        .updated_at
                        .push(<chrono::DateTime<chrono::Utc> as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing User".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(User {
            id: intermediate_rep.id.into_iter().next().ok_or_else(|| "id missing in User".to_string())?,
            username: intermediate_rep.username.into_iter().next().ok_or_else(|| "username missing in User".to_string())?,
            role: intermediate_rep.role.into_iter().next().ok_or_else(|| "role missing in User".to_string())?,
            created_at: intermediate_rep.created_at.into_iter().next().ok_or_else(|| "createdAt missing in User".to_string())?,
            updated_at: intermediate_rep.updated_at.into_iter().next().ok_or_else(|| "updatedAt missing in User".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<User> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<User>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<User>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for User - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<User> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <User as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into User - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// Opaque daemon-issued user identifier.
#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct UserId(pub String);

impl validator::Validate for UserId {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::convert::From<String> for UserId {
    fn from(x: String) -> Self {
        UserId(x)
    }
}

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for UserId {
    type Err = std::string::ParseError;
    fn from_str(x: &str) -> std::result::Result<Self, Self::Err> {
        std::result::Result::Ok(UserId(x.to_string()))
    }
}

impl std::convert::From<UserId> for String {
    fn from(x: UserId) -> Self {
        x.0
    }
}

impl std::ops::Deref for UserId {
    type Target = String;
    fn deref(&self) -> &String {
        &self.0
    }
}

impl std::ops::DerefMut for UserId {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types, clippy::large_enum_variant)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk_enum_derive::LabelledGenericEnum))]
pub enum UserRole {
    #[serde(rename = "owner")]
    Owner,
}

impl validator::Validate for UserRole {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            UserRole::Owner => write!(f, "owner"),
        }
    }
}

impl std::str::FromStr for UserRole {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "owner" => std::result::Result::Ok(UserRole::Owner),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct Username(pub String);

impl validator::Validate for Username {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::convert::From<String> for Username {
    fn from(x: String) -> Self {
        Username(x)
    }
}

impl std::fmt::Display for Username {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for Username {
    type Err = std::string::ParseError;
    fn from_str(x: &str) -> std::result::Result<Self, Self::Err> {
        std::result::Result::Ok(Username(x.to_string()))
    }
}

impl std::convert::From<Username> for String {
    fn from(x: Username) -> Self {
        x.0
    }
}

impl std::ops::Deref for Username {
    type Target = String;
    fn deref(&self) -> &String {
        &self.0
    }
}

impl std::ops::DerefMut for Username {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

/// Container, codecs, duration and resolution of a video.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct VideoMediaInfo {
    #[serde(rename = "container")]
    #[validate(custom(function = "check_xss_string"))]
    pub container: String,

    #[serde(rename = "videoCodec")]
    #[validate(custom(function = "check_xss_string"))]
    pub video_codec: String,

    #[serde(rename = "audioCodec")]
    #[serde(deserialize_with = "deserialize_optional_nullable")]
    #[serde(default = "default_optional_nullable")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_codec: Option<Nullable<String>>,

    #[serde(rename = "durationSecs")]
    #[validate(range(min = 0f64))]
    pub duration_secs: f64,

    #[serde(rename = "width")]
    #[validate(range(min = 0u32))]
    pub width: u32,

    #[serde(rename = "height")]
    #[validate(range(min = 0u32))]
    pub height: u32,

    #[serde(rename = "frameRate")]
    #[validate(range(min = 0f64))]
    pub frame_rate: f64,

    /// MIME type with codecs for HTMLMediaElement.canPlayType.
    #[serde(rename = "contentType")]
    #[validate(custom(function = "check_xss_string"))]
    pub content_type: String,
}

impl VideoMediaInfo {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(container: String, video_codec: String, duration_secs: f64, width: u32, height: u32, frame_rate: f64, content_type: String) -> VideoMediaInfo {
        VideoMediaInfo {
            container,
            video_codec,
            audio_codec: None,
            duration_secs,
            width,
            height,
            frame_rate,
            content_type,
        }
    }
}

/// Converts the VideoMediaInfo value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for VideoMediaInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("container".to_string()),
            Some(self.container.to_string()),
            Some("videoCodec".to_string()),
            Some(self.video_codec.to_string()),
            self.audio_codec
                .as_ref()
                .map(|audio_codec| ["audioCodec".to_string(), audio_codec.as_ref().map_or("null".to_string(), |x| x.to_string())].join(",")),
            Some("durationSecs".to_string()),
            Some(self.duration_secs.to_string()),
            Some("width".to_string()),
            Some(self.width.to_string()),
            Some("height".to_string()),
            Some(self.height.to_string()),
            Some("frameRate".to_string()),
            Some(self.frame_rate.to_string()),
            Some("contentType".to_string()),
            Some(self.content_type.to_string()),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a VideoMediaInfo value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for VideoMediaInfo {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub container: Vec<String>,
            pub video_codec: Vec<String>,
            pub audio_codec: Vec<String>,
            pub duration_secs: Vec<f64>,
            pub width: Vec<u32>,
            pub height: Vec<u32>,
            pub frame_rate: Vec<f64>,
            pub content_type: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing VideoMediaInfo".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "container" => intermediate_rep.container.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "videoCodec" => intermediate_rep.video_codec.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    "audioCodec" => return std::result::Result::Err("Parsing a nullable type in this style is not supported in VideoMediaInfo".to_string()),
                    #[allow(clippy::redundant_clone)]
                    "durationSecs" => intermediate_rep.duration_secs.push(<f64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "width" => intermediate_rep.width.push(<u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "height" => intermediate_rep.height.push(<u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "frameRate" => intermediate_rep.frame_rate.push(<f64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "contentType" => intermediate_rep.content_type.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing VideoMediaInfo".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(VideoMediaInfo {
            container: intermediate_rep
                .container
                .into_iter()
                .next()
                .ok_or_else(|| "container missing in VideoMediaInfo".to_string())?,
            video_codec: intermediate_rep
                .video_codec
                .into_iter()
                .next()
                .ok_or_else(|| "videoCodec missing in VideoMediaInfo".to_string())?,
            audio_codec: std::result::Result::Err("Nullable types not supported in VideoMediaInfo".to_string())?,
            duration_secs: intermediate_rep
                .duration_secs
                .into_iter()
                .next()
                .ok_or_else(|| "durationSecs missing in VideoMediaInfo".to_string())?,
            width: intermediate_rep.width.into_iter().next().ok_or_else(|| "width missing in VideoMediaInfo".to_string())?,
            height: intermediate_rep.height.into_iter().next().ok_or_else(|| "height missing in VideoMediaInfo".to_string())?,
            frame_rate: intermediate_rep
                .frame_rate
                .into_iter()
                .next()
                .ok_or_else(|| "frameRate missing in VideoMediaInfo".to_string())?,
            content_type: intermediate_rep
                .content_type
                .into_iter()
                .next()
                .ok_or_else(|| "contentType missing in VideoMediaInfo".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<VideoMediaInfo> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<VideoMediaInfo>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<VideoMediaInfo>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for VideoMediaInfo - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<VideoMediaInfo> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <VideoMediaInfo as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into VideoMediaInfo - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct VideoSession {
    #[serde(rename = "id")]
    #[validate(custom(function = "check_xss_string"))]
    pub id: String,

    /// Library identifier chosen by the operator in the daemon configuration file. Unlike the daemon-issued identifiers in this API it is a readable name, not a generated token. It is what the API uses in place of a host path, so the physical location of a Library is never exposed.  Because the value is used as a URL path segment, the character set is restricted. The daemon rejects a configuration that violates it at startup.
    #[serde(rename = "libraryId")]
    #[validate(
            length(min = 1, max = 64),
            regex(path = *RE_VIDEOSESSION_LIBRARY_ID),
          custom(function = "check_xss_string"),
    )]
    pub library_id: String,

    /// Absolute-looking path within the selected Library. The resolved path must never escape that library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096), custom(function = "check_xss_string"))]
    pub path: String,

    #[serde(rename = "playlistUrl")]
    #[validate(custom(function = "check_xss_string"))]
    pub playlist_url: String,

    #[serde(rename = "durationSecs")]
    #[validate(range(min = 0f64))]
    pub duration_secs: f64,
}

lazy_static::lazy_static! {
    static ref RE_VIDEOSESSION_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

impl VideoSession {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(id: String, library_id: String, path: String, playlist_url: String, duration_secs: f64) -> VideoSession {
        VideoSession {
            id,
            library_id,
            path,
            playlist_url,
            duration_secs,
        }
    }
}

/// Converts the VideoSession value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for VideoSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("id".to_string()),
            Some(self.id.to_string()),
            Some("libraryId".to_string()),
            Some(self.library_id.to_string()),
            Some("path".to_string()),
            Some(self.path.to_string()),
            Some("playlistUrl".to_string()),
            Some(self.playlist_url.to_string()),
            Some("durationSecs".to_string()),
            Some(self.duration_secs.to_string()),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a VideoSession value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for VideoSession {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub id: Vec<String>,
            pub library_id: Vec<String>,
            pub path: Vec<String>,
            pub playlist_url: Vec<String>,
            pub duration_secs: Vec<f64>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing VideoSession".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "id" => intermediate_rep.id.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "libraryId" => intermediate_rep.library_id.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "path" => intermediate_rep.path.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "playlistUrl" => intermediate_rep.playlist_url.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "durationSecs" => intermediate_rep.duration_secs.push(<f64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing VideoSession".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(VideoSession {
            id: intermediate_rep.id.into_iter().next().ok_or_else(|| "id missing in VideoSession".to_string())?,
            library_id: intermediate_rep
                .library_id
                .into_iter()
                .next()
                .ok_or_else(|| "libraryId missing in VideoSession".to_string())?,
            path: intermediate_rep.path.into_iter().next().ok_or_else(|| "path missing in VideoSession".to_string())?,
            playlist_url: intermediate_rep
                .playlist_url
                .into_iter()
                .next()
                .ok_or_else(|| "playlistUrl missing in VideoSession".to_string())?,
            duration_secs: intermediate_rep
                .duration_secs
                .into_iter()
                .next()
                .ok_or_else(|| "durationSecs missing in VideoSession".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<VideoSession> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<VideoSession>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<VideoSession>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for VideoSession - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<VideoSession> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <VideoSession as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into VideoSession - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct VideoSessionCreateRequest {
    /// Absolute-looking path within the selected Library. The resolved path must never escape that library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096), custom(function = "check_xss_string"))]
    pub path: String,
}

impl VideoSessionCreateRequest {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(path: String) -> VideoSessionCreateRequest {
        VideoSessionCreateRequest { path }
    }
}

/// Converts the VideoSessionCreateRequest value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for VideoSessionCreateRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![Some("path".to_string()), Some(self.path.to_string())];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a VideoSessionCreateRequest value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for VideoSessionCreateRequest {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub path: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing VideoSessionCreateRequest".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "path" => intermediate_rep.path.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing VideoSessionCreateRequest".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(VideoSessionCreateRequest {
            path: intermediate_rep
                .path
                .into_iter()
                .next()
                .ok_or_else(|| "path missing in VideoSessionCreateRequest".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<VideoSessionCreateRequest> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<VideoSessionCreateRequest>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<VideoSessionCreateRequest>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for VideoSessionCreateRequest - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<VideoSessionCreateRequest> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <VideoSessionCreateRequest as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into VideoSessionCreateRequest - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct VideoSessionResponse {
    #[serde(rename = "session")]
    #[validate(nested)]
    pub session: models::VideoSession,
}

impl VideoSessionResponse {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(session: models::VideoSession) -> VideoSessionResponse {
        VideoSessionResponse { session }
    }
}

/// Converts the VideoSessionResponse value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for VideoSessionResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping session in query parameter serialization

        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a VideoSessionResponse value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for VideoSessionResponse {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub session: Vec<models::VideoSession>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing VideoSessionResponse".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "session" => intermediate_rep
                        .session
                        .push(<models::VideoSession as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing VideoSessionResponse".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(VideoSessionResponse {
            session: intermediate_rep
                .session
                .into_iter()
                .next()
                .ok_or_else(|| "session missing in VideoSessionResponse".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<VideoSessionResponse> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<VideoSessionResponse>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<VideoSessionResponse>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for VideoSessionResponse - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<VideoSessionResponse> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <VideoSessionResponse as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into VideoSessionResponse - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}
