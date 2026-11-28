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
pub struct CreateFilePathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_CREATEFILEPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
}

lazy_static::lazy_static! {
    static ref RE_CREATEFILEPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct CreateFileQueryParams {
    /// Path relative to the selected Library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096))]
    pub path: String,
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
pub struct GetEntryMetadataPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_GETENTRYMETADATAPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
}

lazy_static::lazy_static! {
    static ref RE_GETENTRYMETADATAPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct GetEntryMetadataQueryParams {
    /// Path relative to the selected Library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096))]
    pub path: String,
    /// Which face of the path is meant: `file` or `folder`. Only an archive has both; without it, an archive is its file.
    #[serde(rename = "view")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view: Option<models::EntryView>,
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
    /// How to resolve an existing entry at the exact path. `rename` is not accepted; use `POST .../entries/files`.
    #[serde(rename = "conflict")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict: Option<models::WriteConflictPolicy>,
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
pub struct PutEntryTagsQueryParams {
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
pub struct PutRatingQueryParams {
    /// Path relative to the selected Library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096))]
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct DeleteFolderSettingPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_DELETEFOLDERSETTINGPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
}

lazy_static::lazy_static! {
    static ref RE_DELETEFOLDERSETTINGPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct DeleteFolderSettingQueryParams {
    /// Path relative to the selected Library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096))]
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct DeleteShelfPathParams {
    /// Identifier the daemon gave the Shelf.
    #[validate(length(min = 1))]
    pub shelf_id: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct PutFolderSettingPathParams {
    /// Opaque identifier of a Library owned by the owner.
    #[validate(
                        length(min = 1, max = 64),
                          regex(path = *RE_PUTFOLDERSETTINGPATHPARAMS_LIBRARY_ID),
            )]
    pub library_id: String,
}

lazy_static::lazy_static! {
    static ref RE_PUTFOLDERSETTINGPATHPARAMS_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct PutFolderSettingQueryParams {
    /// Path relative to the selected Library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096))]
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ReplaceShelfPathParams {
    /// Identifier the daemon gave the Shelf.
    #[validate(length(min = 1))]
    pub shelf_id: String,
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

/// How an archive opens: `folder` browses its contents like a folder, and `viewer` opens them in the viewer. An archive opens as a folder when the value is absent.
/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types, clippy::large_enum_variant)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk_enum_derive::LabelledGenericEnum))]
pub enum ArchiveOpen {
    #[serde(rename = "folder")]
    Folder,
    #[serde(rename = "viewer")]
    Viewer,
}

impl validator::Validate for ArchiveOpen {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for ArchiveOpen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            ArchiveOpen::Folder => write!(f, "folder"),
            ArchiveOpen::Viewer => write!(f, "viewer"),
        }
    }
}

impl std::str::FromStr for ArchiveOpen {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "folder" => std::result::Result::Ok(ArchiveOpen::Folder),
            "viewer" => std::result::Result::Ok(ArchiveOpen::Viewer),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
        }
    }
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

    /// Whether the daemon writes the performance log, so a client should send its timings to `/performance/client-timings`.
    #[serde(rename = "performanceLogging")]
    pub performance_logging: bool,
}

impl Capabilities {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(summary_listing: bool, entry_details: bool, child_probes: bool, performance_logging: bool) -> Capabilities {
        Capabilities {
            summary_listing,
            entry_details,
            child_probes,
            performance_logging,
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
            Some("performanceLogging".to_string()),
            Some(self.performance_logging.to_string()),
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
            pub performance_logging: Vec<bool>,
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
                    #[allow(clippy::redundant_clone)]
                    "performanceLogging" => intermediate_rep
                        .performance_logging
                        .push(<bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
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
            performance_logging: intermediate_rep
                .performance_logging
                .into_iter()
                .next()
                .ok_or_else(|| "performanceLogging missing in Capabilities".to_string())?,
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

/// One timing in milliseconds. The phases come from the browser's Resource Timing and are omitted when the browser does not report them.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ClientTiming {
    #[serde(rename = "operation")]
    #[validate(nested)]
    pub operation: models::ClientTimingOperation,

    /// Library identifier chosen by the operator in the daemon configuration file. Unlike the daemon-issued identifiers in this API it is a readable name, not a generated token. It is what the API uses in place of a host path, so the physical location of a Library is never exposed.  Because the value is used as a URL path segment, the character set is restricted. The daemon rejects a configuration that violates it at startup.
    #[serde(rename = "libraryId")]
    #[validate(
            length(min = 1, max = 64),
            regex(path = *RE_CLIENTTIMING_LIBRARY_ID),
          custom(function = "check_xss_string"),
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library_id: Option<String>,

    /// Absolute-looking path within the selected Library. The resolved path must never escape that library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096), custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    /// The whole time, from the start of the request or the move until the load.
    #[serde(rename = "durationMs")]
    #[validate(range(min = 0f64))]
    pub duration_ms: f64,

    /// From the start of the request until the browser sent it.
    #[serde(rename = "queuedMs")]
    #[validate(range(min = 0f64))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queued_ms: Option<f64>,

    /// From sending the request until the first byte of the response.
    #[serde(rename = "waitingMs")]
    #[validate(range(min = 0f64))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waiting_ms: Option<f64>,

    /// From the first byte until the last byte of the response.
    #[serde(rename = "downloadMs")]
    #[validate(range(min = 0f64))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_ms: Option<f64>,

    /// From the last byte until the load event, including decoding.
    #[serde(rename = "afterResponseMs")]
    #[validate(range(min = 0f64))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_response_ms: Option<f64>,

    /// The encoded body size.
    #[serde(rename = "bytes")]
    #[validate(range(min = 0u64))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,

    /// Whether the browser served the response from its cache.
    #[serde(rename = "cached")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached: Option<bool>,

    #[serde(rename = "mode")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<models::ClientTimingMode>,
}

lazy_static::lazy_static! {
    static ref RE_CLIENTTIMING_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

impl ClientTiming {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(operation: models::ClientTimingOperation, duration_ms: f64) -> ClientTiming {
        ClientTiming {
            operation,
            library_id: None,
            path: None,
            duration_ms,
            queued_ms: None,
            waiting_ms: None,
            download_ms: None,
            after_response_ms: None,
            bytes: None,
            cached: None,
            mode: None,
        }
    }
}

/// Converts the ClientTiming value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for ClientTiming {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping operation in query parameter serialization
            self.library_id.as_ref().map(|library_id| ["libraryId".to_string(), library_id.to_string()].join(",")),
            self.path.as_ref().map(|path| ["path".to_string(), path.to_string()].join(",")),
            Some("durationMs".to_string()),
            Some(self.duration_ms.to_string()),
            self.queued_ms.as_ref().map(|queued_ms| ["queuedMs".to_string(), queued_ms.to_string()].join(",")),
            self.waiting_ms.as_ref().map(|waiting_ms| ["waitingMs".to_string(), waiting_ms.to_string()].join(",")),
            self.download_ms.as_ref().map(|download_ms| ["downloadMs".to_string(), download_ms.to_string()].join(",")),
            self.after_response_ms
                .as_ref()
                .map(|after_response_ms| ["afterResponseMs".to_string(), after_response_ms.to_string()].join(",")),
            self.bytes.as_ref().map(|bytes| ["bytes".to_string(), bytes.to_string()].join(",")),
            self.cached.as_ref().map(|cached| ["cached".to_string(), cached.to_string()].join(",")),
            // Skipping mode in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a ClientTiming value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for ClientTiming {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub operation: Vec<models::ClientTimingOperation>,
            pub library_id: Vec<String>,
            pub path: Vec<String>,
            pub duration_ms: Vec<f64>,
            pub queued_ms: Vec<f64>,
            pub waiting_ms: Vec<f64>,
            pub download_ms: Vec<f64>,
            pub after_response_ms: Vec<f64>,
            pub bytes: Vec<u64>,
            pub cached: Vec<bool>,
            pub mode: Vec<models::ClientTimingMode>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing ClientTiming".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "operation" => intermediate_rep
                        .operation
                        .push(<models::ClientTimingOperation as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "libraryId" => intermediate_rep.library_id.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "path" => intermediate_rep.path.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "durationMs" => intermediate_rep.duration_ms.push(<f64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "queuedMs" => intermediate_rep.queued_ms.push(<f64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "waitingMs" => intermediate_rep.waiting_ms.push(<f64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "downloadMs" => intermediate_rep.download_ms.push(<f64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "afterResponseMs" => intermediate_rep
                        .after_response_ms
                        .push(<f64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "bytes" => intermediate_rep.bytes.push(<u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "cached" => intermediate_rep.cached.push(<bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "mode" => intermediate_rep
                        .mode
                        .push(<models::ClientTimingMode as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing ClientTiming".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(ClientTiming {
            operation: intermediate_rep
                .operation
                .into_iter()
                .next()
                .ok_or_else(|| "operation missing in ClientTiming".to_string())?,
            library_id: intermediate_rep.library_id.into_iter().next(),
            path: intermediate_rep.path.into_iter().next(),
            duration_ms: intermediate_rep
                .duration_ms
                .into_iter()
                .next()
                .ok_or_else(|| "durationMs missing in ClientTiming".to_string())?,
            queued_ms: intermediate_rep.queued_ms.into_iter().next(),
            waiting_ms: intermediate_rep.waiting_ms.into_iter().next(),
            download_ms: intermediate_rep.download_ms.into_iter().next(),
            after_response_ms: intermediate_rep.after_response_ms.into_iter().next(),
            bytes: intermediate_rep.bytes.into_iter().next(),
            cached: intermediate_rep.cached.into_iter().next(),
            mode: intermediate_rep.mode.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<ClientTiming> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<ClientTiming>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<ClientTiming>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for ClientTiming - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<ClientTiming> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <ClientTiming as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into ClientTiming - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// How a `video_start` plays the video. `direct` plays the original file, and the HLS modes play a converted session.
/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types, clippy::large_enum_variant)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk_enum_derive::LabelledGenericEnum))]
pub enum ClientTimingMode {
    #[serde(rename = "direct")]
    Direct,
    #[serde(rename = "native-hls")]
    NativeHls,
    #[serde(rename = "hls-js")]
    HlsJs,
}

impl validator::Validate for ClientTimingMode {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for ClientTimingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            ClientTimingMode::Direct => write!(f, "direct"),
            ClientTimingMode::NativeHls => write!(f, "native-hls"),
            ClientTimingMode::HlsJs => write!(f, "hls-js"),
        }
    }
}

impl std::str::FromStr for ClientTimingMode {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "direct" => std::result::Result::Ok(ClientTimingMode::Direct),
            "native-hls" => std::result::Result::Ok(ClientTimingMode::NativeHls),
            "hls-js" => std::result::Result::Ok(ClientTimingMode::HlsJs),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
        }
    }
}

/// `display_image`, `original_image`, and `thumbnail_image` are the loads of those images. `viewer_display` is the time from moving the viewer to an entry until its display image is loaded. `video_start` is the time from opening a video player until the first frame is playing.
/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types, clippy::large_enum_variant)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk_enum_derive::LabelledGenericEnum))]
pub enum ClientTimingOperation {
    #[serde(rename = "display_image")]
    DisplayImage,
    #[serde(rename = "original_image")]
    OriginalImage,
    #[serde(rename = "thumbnail_image")]
    ThumbnailImage,
    #[serde(rename = "video_start")]
    VideoStart,
    #[serde(rename = "viewer_display")]
    ViewerDisplay,
}

impl validator::Validate for ClientTimingOperation {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for ClientTimingOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            ClientTimingOperation::DisplayImage => write!(f, "display_image"),
            ClientTimingOperation::OriginalImage => write!(f, "original_image"),
            ClientTimingOperation::ThumbnailImage => write!(f, "thumbnail_image"),
            ClientTimingOperation::VideoStart => write!(f, "video_start"),
            ClientTimingOperation::ViewerDisplay => write!(f, "viewer_display"),
        }
    }
}

impl std::str::FromStr for ClientTimingOperation {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "display_image" => std::result::Result::Ok(ClientTimingOperation::DisplayImage),
            "original_image" => std::result::Result::Ok(ClientTimingOperation::OriginalImage),
            "thumbnail_image" => std::result::Result::Ok(ClientTimingOperation::ThumbnailImage),
            "video_start" => std::result::Result::Ok(ClientTimingOperation::VideoStart),
            "viewer_display" => std::result::Result::Ok(ClientTimingOperation::ViewerDisplay),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ClientTimingReport {
    #[serde(rename = "timings")]
    #[validate(length(min = 1, max = 200), nested)]
    pub timings: Vec<models::ClientTiming>,
}

impl ClientTimingReport {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(timings: Vec<models::ClientTiming>) -> ClientTimingReport {
        ClientTimingReport { timings }
    }
}

/// Converts the ClientTimingReport value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for ClientTimingReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping timings in query parameter serialization

        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a ClientTimingReport value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for ClientTimingReport {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub timings: Vec<Vec<models::ClientTiming>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing ClientTimingReport".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "timings" => return std::result::Result::Err("Parsing a container in this style is not supported in ClientTimingReport".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing ClientTimingReport".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(ClientTimingReport {
            timings: intermediate_rep
                .timings
                .into_iter()
                .next()
                .ok_or_else(|| "timings missing in ClientTimingReport".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<ClientTimingReport> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<ClientTimingReport>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<ClientTimingReport>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for ClientTimingReport - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<ClientTimingReport> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <ClientTimingReport as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into ClientTimingReport - {err}"#)),
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

/// How a listing is laid out.
/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types, clippy::large_enum_variant)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk_enum_derive::LabelledGenericEnum))]
pub enum DisplayLayout {
    #[serde(rename = "grid")]
    Grid,
    #[serde(rename = "list")]
    List,
    #[serde(rename = "timeline")]
    Timeline,
}

impl validator::Validate for DisplayLayout {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for DisplayLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            DisplayLayout::Grid => write!(f, "grid"),
            DisplayLayout::List => write!(f, "list"),
            DisplayLayout::Timeline => write!(f, "timeline"),
        }
    }
}

impl std::str::FromStr for DisplayLayout {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "grid" => std::result::Result::Ok(DisplayLayout::Grid),
            "list" => std::result::Result::Ok(DisplayLayout::List),
            "timeline" => std::result::Result::Ok(DisplayLayout::Timeline),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
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

    /// Filmstrip size step of the viewer from 1 (smallest) to 5 (largest). Only the wide set uses it; the narrow set keeps the value without showing a filmstrip.
    #[serde(rename = "filmstripSize")]
    #[validate(range(min = 1u32, max = 5u32))]
    pub filmstrip_size: u32,
}

impl DisplaySet {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(tile_size: u32, thumbnail_shape: models::ThumbnailShape, view: models::ListView, original_image: models::OriginalImageMode, filmstrip_size: u32) -> DisplaySet {
        DisplaySet {
            tile_size,
            thumbnail_shape,
            view,
            original_image,
            filmstrip_size,
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
            Some("filmstripSize".to_string()),
            Some(self.filmstrip_size.to_string()),
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
            pub filmstrip_size: Vec<u32>,
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
                    #[allow(clippy::redundant_clone)]
                    "filmstripSize" => intermediate_rep.filmstrip_size.push(<u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
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
            filmstrip_size: intermediate_rep
                .filmstrip_size
                .into_iter()
                .next()
                .ok_or_else(|| "filmstripSize missing in DisplaySet".to_string())?,
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

/// The values chosen by the width of the screen. An absent value is taken from the layer below.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct DisplayValueSet {
    #[serde(rename = "view")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view: Option<models::DisplayLayout>,

    /// Tile size step from 1 (smallest) to 5 (largest).
    #[serde(rename = "tileSize")]
    #[validate(range(min = 1u32, max = 5u32))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tile_size: Option<u32>,

    #[serde(rename = "thumbnailShape")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_shape: Option<models::ThumbnailShape>,
}

impl DisplayValueSet {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new() -> DisplayValueSet {
        DisplayValueSet {
            view: None,
            tile_size: None,
            thumbnail_shape: None,
        }
    }
}

/// Converts the DisplayValueSet value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for DisplayValueSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping view in query parameter serialization
            self.tile_size.as_ref().map(|tile_size| ["tileSize".to_string(), tile_size.to_string()].join(",")),
            // Skipping thumbnailShape in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a DisplayValueSet value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for DisplayValueSet {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub view: Vec<models::DisplayLayout>,
            pub tile_size: Vec<u32>,
            pub thumbnail_shape: Vec<models::ThumbnailShape>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing DisplayValueSet".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "view" => intermediate_rep
                        .view
                        .push(<models::DisplayLayout as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "tileSize" => intermediate_rep.tile_size.push(<u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "thumbnailShape" => intermediate_rep
                        .thumbnail_shape
                        .push(<models::ThumbnailShape as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing DisplayValueSet".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(DisplayValueSet {
            view: intermediate_rep.view.into_iter().next(),
            tile_size: intermediate_rep.tile_size.into_iter().next(),
            thumbnail_shape: intermediate_rep.thumbnail_shape.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<DisplayValueSet> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<DisplayValueSet>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<DisplayValueSet>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for DisplayValueSet - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<DisplayValueSet> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <DisplayValueSet as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into DisplayValueSet - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// The display values a Shelf or the settings of a folder hold, in the shape of the display preferences. Every value is optional.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct DisplayValues {
    #[serde(rename = "wide")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wide: Option<models::DisplayValueSet>,

    #[serde(rename = "narrow")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub narrow: Option<models::DisplayValueSet>,

    #[serde(rename = "sort")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<models::SortPreference>,

    #[serde(rename = "archiveOpen")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive_open: Option<models::ArchiveOpen>,
}

impl DisplayValues {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new() -> DisplayValues {
        DisplayValues {
            wide: None,
            narrow: None,
            sort: None,
            archive_open: None,
        }
    }
}

/// Converts the DisplayValues value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for DisplayValues {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping wide in query parameter serialization

            // Skipping narrow in query parameter serialization

            // Skipping sort in query parameter serialization

            // Skipping archiveOpen in query parameter serialization

        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a DisplayValues value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for DisplayValues {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub wide: Vec<models::DisplayValueSet>,
            pub narrow: Vec<models::DisplayValueSet>,
            pub sort: Vec<models::SortPreference>,
            pub archive_open: Vec<models::ArchiveOpen>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing DisplayValues".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "wide" => intermediate_rep
                        .wide
                        .push(<models::DisplayValueSet as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "narrow" => intermediate_rep
                        .narrow
                        .push(<models::DisplayValueSet as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "sort" => intermediate_rep
                        .sort
                        .push(<models::SortPreference as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "archiveOpen" => intermediate_rep
                        .archive_open
                        .push(<models::ArchiveOpen as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing DisplayValues".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(DisplayValues {
            wide: intermediate_rep.wide.into_iter().next(),
            narrow: intermediate_rep.narrow.into_iter().next(),
            sort: intermediate_rep.sort.into_iter().next(),
            archive_open: intermediate_rep.archive_open.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<DisplayValues> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<DisplayValues>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<DisplayValues>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for DisplayValues - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<DisplayValues> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <DisplayValues as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into DisplayValues - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// Delete request over Entries of one Library. `recursive`, false when omitted, deletes non-empty folders and everything below them.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct EntryDeleteRequest {
    #[serde(rename = "targets")]
    #[validate(length(min = 1, max = 1000), nested)]
    pub targets: Vec<models::EntryRef>,

    #[serde(rename = "recursive")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recursive: Option<bool>,
}

impl EntryDeleteRequest {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(targets: Vec<models::EntryRef>) -> EntryDeleteRequest {
        EntryDeleteRequest { targets, recursive: None }
    }
}

/// Converts the EntryDeleteRequest value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for EntryDeleteRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping targets in query parameter serialization
            self.recursive.as_ref().map(|recursive| ["recursive".to_string(), recursive.to_string()].join(",")),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a EntryDeleteRequest value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for EntryDeleteRequest {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub targets: Vec<Vec<models::EntryRef>>,
            pub recursive: Vec<bool>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing EntryDeleteRequest".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "targets" => return std::result::Result::Err("Parsing a container in this style is not supported in EntryDeleteRequest".to_string()),
                    #[allow(clippy::redundant_clone)]
                    "recursive" => intermediate_rep.recursive.push(<bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing EntryDeleteRequest".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(EntryDeleteRequest {
            targets: intermediate_rep
                .targets
                .into_iter()
                .next()
                .ok_or_else(|| "targets missing in EntryDeleteRequest".to_string())?,
            recursive: intermediate_rep.recursive.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<EntryDeleteRequest> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<EntryDeleteRequest>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<EntryDeleteRequest>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for EntryDeleteRequest - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<EntryDeleteRequest> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <EntryDeleteRequest as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into EntryDeleteRequest - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// Where a copy or move puts its sources: a Library and exactly one of `folderPath`, into which each source keeps its name, and `path`, the exact target of a single source, which is also how a rename is expressed. A destination is never inside an archive, so it has no `view`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct EntryDestination {
    /// Library identifier chosen by the operator in the daemon configuration file. Unlike the daemon-issued identifiers in this API it is a readable name, not a generated token. It is what the API uses in place of a host path, so the physical location of a Library is never exposed.  Because the value is used as a URL path segment, the character set is restricted. The daemon rejects a configuration that violates it at startup.
    #[serde(rename = "libraryId")]
    #[validate(
            length(min = 1, max = 64),
            regex(path = *RE_ENTRYDESTINATION_LIBRARY_ID),
          custom(function = "check_xss_string"),
    )]
    pub library_id: String,

    /// Absolute-looking path within the selected Library. The resolved path must never escape that library.
    #[serde(rename = "folderPath")]
    #[validate(length(min = 1, max = 4096), custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_path: Option<String>,

    /// Absolute-looking path within the selected Library. The resolved path must never escape that library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096), custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

lazy_static::lazy_static! {
    static ref RE_ENTRYDESTINATION_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

impl EntryDestination {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(library_id: String) -> EntryDestination {
        EntryDestination {
            library_id,
            folder_path: None,
            path: None,
        }
    }
}

/// Converts the EntryDestination value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for EntryDestination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("libraryId".to_string()),
            Some(self.library_id.to_string()),
            self.folder_path.as_ref().map(|folder_path| ["folderPath".to_string(), folder_path.to_string()].join(",")),
            self.path.as_ref().map(|path| ["path".to_string(), path.to_string()].join(",")),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a EntryDestination value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for EntryDestination {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub library_id: Vec<String>,
            pub folder_path: Vec<String>,
            pub path: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing EntryDestination".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "libraryId" => intermediate_rep.library_id.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "folderPath" => intermediate_rep.folder_path.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "path" => intermediate_rep.path.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing EntryDestination".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(EntryDestination {
            library_id: intermediate_rep
                .library_id
                .into_iter()
                .next()
                .ok_or_else(|| "libraryId missing in EntryDestination".to_string())?,
            folder_path: intermediate_rep.folder_path.into_iter().next(),
            path: intermediate_rep.path.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<EntryDestination> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<EntryDestination>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<EntryDestination>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for EntryDestination - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<EntryDestination> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <EntryDestination as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into EntryDestination - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// The conditions the Entries of a listing meet.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct EntryFilter {
    /// Whitespace-separated terms that must all occur in an entry name. Names and terms are normalized with NFKC and converted to lowercase.
    #[serde(rename = "query")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,

    /// Return only images and videos with this rating of the current user. `none` selects the images and videos that have not been rated. Folders and other files never match.
    /// Note: inline enums are not fully supported by openapi-generator
    #[serde(rename = "rating")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<String>,

    /// Return only images and videos that have all of these tags of the current user. Each word is trimmed like a tag being set; a word the user has never used selects nothing.
    #[serde(rename = "tags")]
    #[validate(length(max = 32), nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<models::TagName>>,

    /// Return only Entries of these kinds. A symbolic link is `other`.
    /// Note: inline enums are not fully supported by openapi-generator
    #[serde(rename = "kinds")]
    #[validate(length(max = 5), custom(function = "check_xss_vec_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kinds: Option<Vec<String>>,
}

impl EntryFilter {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new() -> EntryFilter {
        EntryFilter {
            query: None,
            rating: None,
            tags: None,
            kinds: None,
        }
    }
}

/// Converts the EntryFilter value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for EntryFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            self.query.as_ref().map(|query| ["query".to_string(), query.to_string()].join(",")),
            self.rating.as_ref().map(|rating| ["rating".to_string(), rating.to_string()].join(",")),
            self.tags
                .as_ref()
                .map(|tags| ["tags".to_string(), tags.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")].join(",")),
            self.kinds
                .as_ref()
                .map(|kinds| ["kinds".to_string(), kinds.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")].join(",")),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a EntryFilter value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for EntryFilter {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub query: Vec<String>,
            pub rating: Vec<String>,
            pub tags: Vec<Vec<models::TagName>>,
            pub kinds: Vec<Vec<String>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing EntryFilter".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "query" => intermediate_rep.query.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "rating" => intermediate_rep.rating.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    "tags" => return std::result::Result::Err("Parsing a container in this style is not supported in EntryFilter".to_string()),
                    "kinds" => return std::result::Result::Err("Parsing a container in this style is not supported in EntryFilter".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing EntryFilter".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(EntryFilter {
            query: intermediate_rep.query.into_iter().next(),
            rating: intermediate_rep.rating.into_iter().next(),
            tags: intermediate_rep.tags.into_iter().next(),
            kinds: intermediate_rep.kinds.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<EntryFilter> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<EntryFilter>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<EntryFilter>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for EntryFilter - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<EntryFilter> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <EntryFilter as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into EntryFilter - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct EntryListOptions {
    /// Place folders, and files whose `file.archive` hint is true, before the other entries. `sort` and `order` apply within each of the two groups. Defaults to `false`.
    #[serde(rename = "foldersFirst")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folders_first: Option<bool>,

    /// Read the entries whose names start with `.` and the entries inside them. A saved Shelf holds no options, so it never reads them. Defaults to `false`.
    #[serde(rename = "includeHidden")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_hidden: Option<bool>,

    /// `full` returns `items` with their Entries; `summary` returns `summaries` for the direct children of one start. Defaults to `full`.
    /// Note: inline enums are not fully supported by openapi-generator
    #[serde(rename = "fields")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<String>,
}

impl EntryListOptions {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new() -> EntryListOptions {
        EntryListOptions {
            folders_first: None,
            include_hidden: None,
            fields: None,
        }
    }
}

/// Converts the EntryListOptions value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for EntryListOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            self.folders_first
                .as_ref()
                .map(|folders_first| ["foldersFirst".to_string(), folders_first.to_string()].join(",")),
            self.include_hidden
                .as_ref()
                .map(|include_hidden| ["includeHidden".to_string(), include_hidden.to_string()].join(",")),
            self.fields.as_ref().map(|fields| ["fields".to_string(), fields.to_string()].join(",")),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a EntryListOptions value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for EntryListOptions {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub folders_first: Vec<bool>,
            pub include_hidden: Vec<bool>,
            pub fields: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing EntryListOptions".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "foldersFirst" => intermediate_rep.folders_first.push(<bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "includeHidden" => intermediate_rep.include_hidden.push(<bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "fields" => intermediate_rep.fields.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing EntryListOptions".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(EntryListOptions {
            folders_first: intermediate_rep.folders_first.into_iter().next(),
            include_hidden: intermediate_rep.include_hidden.into_iter().next(),
            fields: intermediate_rep.fields.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<EntryListOptions> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<EntryListOptions>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<EntryListOptions>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for EntryListOptions - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<EntryListOptions> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <EntryListOptions as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into EntryListOptions - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct EntryListRequest {
    #[serde(rename = "scope")]
    #[validate(nested)]
    pub scope: models::EntryScope,

    #[serde(rename = "filter")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<models::EntryFilter>,

    #[serde(rename = "options")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<models::EntryListOptions>,

    /// Sort key. `timeline_date` orders the images and videos by their timeline dates, newest first only. Defaults to `name`.
    /// Note: inline enums are not fully supported by openapi-generator
    #[serde(rename = "sort")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,

    /// Sort direction. Defaults to `asc`.
    /// Note: inline enums are not fully supported by openapi-generator
    #[serde(rename = "order")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<String>,

    #[serde(rename = "page")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<models::EntryPageRequest>,
}

impl EntryListRequest {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(scope: models::EntryScope) -> EntryListRequest {
        EntryListRequest {
            scope,
            filter: None,
            options: None,
            sort: None,
            order: None,
            page: None,
        }
    }
}

/// Converts the EntryListRequest value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for EntryListRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping scope in query parameter serialization

            // Skipping filter in query parameter serialization

            // Skipping options in query parameter serialization
            self.sort.as_ref().map(|sort| ["sort".to_string(), sort.to_string()].join(",")),
            self.order.as_ref().map(|order| ["order".to_string(), order.to_string()].join(",")),
            // Skipping page in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a EntryListRequest value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for EntryListRequest {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub scope: Vec<models::EntryScope>,
            pub filter: Vec<models::EntryFilter>,
            pub options: Vec<models::EntryListOptions>,
            pub sort: Vec<String>,
            pub order: Vec<String>,
            pub page: Vec<models::EntryPageRequest>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing EntryListRequest".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "scope" => intermediate_rep
                        .scope
                        .push(<models::EntryScope as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "filter" => intermediate_rep
                        .filter
                        .push(<models::EntryFilter as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "options" => intermediate_rep
                        .options
                        .push(<models::EntryListOptions as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "sort" => intermediate_rep.sort.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "order" => intermediate_rep.order.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "page" => intermediate_rep
                        .page
                        .push(<models::EntryPageRequest as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing EntryListRequest".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(EntryListRequest {
            scope: intermediate_rep.scope.into_iter().next().ok_or_else(|| "scope missing in EntryListRequest".to_string())?,
            filter: intermediate_rep.filter.into_iter().next(),
            options: intermediate_rep.options.into_iter().next(),
            sort: intermediate_rep.sort.into_iter().next(),
            order: intermediate_rep.order.into_iter().next(),
            page: intermediate_rep.page.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<EntryListRequest> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<EntryListRequest>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<EntryListRequest>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for EntryListRequest - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<EntryListRequest> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <EntryListRequest as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into EntryListRequest - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct EntryMetadataList {
    #[serde(rename = "items")]
    #[validate(nested)]
    pub items: Vec<models::EntryMetadataResult>,
}

impl EntryMetadataList {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(items: Vec<models::EntryMetadataResult>) -> EntryMetadataList {
        EntryMetadataList { items }
    }
}

/// Converts the EntryMetadataList value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for EntryMetadataList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping items in query parameter serialization

        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a EntryMetadataList value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for EntryMetadataList {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub items: Vec<Vec<models::EntryMetadataResult>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing EntryMetadataList".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "items" => return std::result::Result::Err("Parsing a container in this style is not supported in EntryMetadataList".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing EntryMetadataList".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(EntryMetadataList {
            items: intermediate_rep.items.into_iter().next().ok_or_else(|| "items missing in EntryMetadataList".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<EntryMetadataList> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<EntryMetadataList>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<EntryMetadataList>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for EntryMetadataList - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<EntryMetadataList> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <EntryMetadataList as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into EntryMetadataList - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct EntryMetadataRequest {
    #[serde(rename = "places")]
    #[validate(length(min = 1, max = 200), nested)]
    pub places: Vec<models::LibraryPath>,
}

impl EntryMetadataRequest {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(places: Vec<models::LibraryPath>) -> EntryMetadataRequest {
        EntryMetadataRequest { places }
    }
}

/// Converts the EntryMetadataRequest value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for EntryMetadataRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping places in query parameter serialization

        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a EntryMetadataRequest value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for EntryMetadataRequest {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub places: Vec<Vec<models::LibraryPath>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing EntryMetadataRequest".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "places" => return std::result::Result::Err("Parsing a container in this style is not supported in EntryMetadataRequest".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing EntryMetadataRequest".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(EntryMetadataRequest {
            places: intermediate_rep
                .places
                .into_iter()
                .next()
                .ok_or_else(|| "places missing in EntryMetadataRequest".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<EntryMetadataRequest> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<EntryMetadataRequest>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<EntryMetadataRequest>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for EntryMetadataRequest - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<EntryMetadataRequest> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <EntryMetadataRequest as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into EntryMetadataRequest - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// One place of the request, with either its Entry or its error kind.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct EntryMetadataResult {
    /// Library identifier chosen by the operator in the daemon configuration file. Unlike the daemon-issued identifiers in this API it is a readable name, not a generated token. It is what the API uses in place of a host path, so the physical location of a Library is never exposed.  Because the value is used as a URL path segment, the character set is restricted. The daemon rejects a configuration that violates it at startup.
    #[serde(rename = "libraryId")]
    #[validate(
            length(min = 1, max = 64),
            regex(path = *RE_ENTRYMETADATARESULT_LIBRARY_ID),
          custom(function = "check_xss_string"),
    )]
    pub library_id: String,

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

lazy_static::lazy_static! {
    static ref RE_ENTRYMETADATARESULT_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

impl EntryMetadataResult {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(library_id: String, path: String) -> EntryMetadataResult {
        EntryMetadataResult {
            library_id,
            path,
            detail: None,
            error: None,
        }
    }
}

/// Converts the EntryMetadataResult value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for EntryMetadataResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("libraryId".to_string()),
            Some(self.library_id.to_string()),
            Some("path".to_string()),
            Some(self.path.to_string()),
            // Skipping detail in query parameter serialization

            // Skipping error in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a EntryMetadataResult value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for EntryMetadataResult {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub library_id: Vec<String>,
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
                None => return std::result::Result::Err("Missing value while parsing EntryMetadataResult".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "libraryId" => intermediate_rep.library_id.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
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
                    _ => return std::result::Result::Err("Unexpected key while parsing EntryMetadataResult".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(EntryMetadataResult {
            library_id: intermediate_rep
                .library_id
                .into_iter()
                .next()
                .ok_or_else(|| "libraryId missing in EntryMetadataResult".to_string())?,
            path: intermediate_rep.path.into_iter().next().ok_or_else(|| "path missing in EntryMetadataResult".to_string())?,
            detail: intermediate_rep.detail.into_iter().next(),
            error: intermediate_rep.error.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<EntryMetadataResult> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<EntryMetadataResult>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<EntryMetadataResult>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for EntryMetadataResult - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<EntryMetadataResult> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <EntryMetadataResult as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into EntryMetadataResult - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct EntryMonthsRequest {
    #[serde(rename = "scope")]
    #[validate(nested)]
    pub scope: models::EntryScope,

    #[serde(rename = "filter")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<models::EntryFilter>,
}

impl EntryMonthsRequest {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(scope: models::EntryScope) -> EntryMonthsRequest {
        EntryMonthsRequest { scope, filter: None }
    }
}

/// Converts the EntryMonthsRequest value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for EntryMonthsRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping scope in query parameter serialization

            // Skipping filter in query parameter serialization

        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a EntryMonthsRequest value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for EntryMonthsRequest {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub scope: Vec<models::EntryScope>,
            pub filter: Vec<models::EntryFilter>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing EntryMonthsRequest".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "scope" => intermediate_rep
                        .scope
                        .push(<models::EntryScope as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "filter" => intermediate_rep
                        .filter
                        .push(<models::EntryFilter as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing EntryMonthsRequest".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(EntryMonthsRequest {
            scope: intermediate_rep.scope.into_iter().next().ok_or_else(|| "scope missing in EntryMonthsRequest".to_string())?,
            filter: intermediate_rep.filter.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<EntryMonthsRequest> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<EntryMonthsRequest>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<EntryMonthsRequest>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for EntryMonthsRequest - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<EntryMonthsRequest> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <EntryMonthsRequest as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into EntryMonthsRequest - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// One page of a listing. It carries `items` with `options.fields` of `full`, and `summaries` with `summary`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct EntryPage {
    #[serde(rename = "items")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<models::ScopedEntry>>,

    #[serde(rename = "summaries")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summaries: Option<Vec<models::ScopedEntrySummary>>,

    #[serde(rename = "page")]
    #[validate(nested)]
    pub page: models::EntryPageInfo,

    #[serde(rename = "warnings")]
    #[validate(nested)]
    pub warnings: models::EntryWarnings,
}

impl EntryPage {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(page: models::EntryPageInfo, warnings: models::EntryWarnings) -> EntryPage {
        EntryPage {
            items: None,
            summaries: None,
            page,
            warnings,
        }
    }
}

/// Converts the EntryPage value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for EntryPage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping items in query parameter serialization

            // Skipping summaries in query parameter serialization

            // Skipping page in query parameter serialization

            // Skipping warnings in query parameter serialization

        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a EntryPage value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for EntryPage {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub items: Vec<Vec<models::ScopedEntry>>,
            pub summaries: Vec<Vec<models::ScopedEntrySummary>>,
            pub page: Vec<models::EntryPageInfo>,
            pub warnings: Vec<models::EntryWarnings>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing EntryPage".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "items" => return std::result::Result::Err("Parsing a container in this style is not supported in EntryPage".to_string()),
                    "summaries" => return std::result::Result::Err("Parsing a container in this style is not supported in EntryPage".to_string()),
                    #[allow(clippy::redundant_clone)]
                    "page" => intermediate_rep
                        .page
                        .push(<models::EntryPageInfo as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "warnings" => intermediate_rep
                        .warnings
                        .push(<models::EntryWarnings as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing EntryPage".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(EntryPage {
            items: intermediate_rep.items.into_iter().next(),
            summaries: intermediate_rep.summaries.into_iter().next(),
            page: intermediate_rep.page.into_iter().next().ok_or_else(|| "page missing in EntryPage".to_string())?,
            warnings: intermediate_rep.warnings.into_iter().next().ok_or_else(|| "warnings missing in EntryPage".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<EntryPage> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<EntryPage>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<EntryPage>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for EntryPage - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<EntryPage> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <EntryPage as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into EntryPage - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// The effective page. An `offset` page carries `offset`, `totalCount`, and `snapshot`; a `cursor` page carries `previous` and `next`, each absent at an end.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct EntryPageInfo {
    #[serde(rename = "kind")]
    #[validate(nested)]
    pub kind: models::EntryPageKind,

    /// Offset of this page in the merged sequence.
    #[serde(rename = "offset")]
    #[validate(range(min = 0u64))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,

    /// Number of Entries in the merged sequence after the Filter.
    #[serde(rename = "totalCount")]
    #[validate(range(min = 0u64))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_count: Option<u64>,

    /// Opaque identifier of the merged listing. It changes when the listing of any start is rebuilt.
    #[serde(rename = "snapshot")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<String>,

    /// Cursor for the page of newer Entries.
    #[serde(rename = "previous")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous: Option<String>,

    /// Cursor for the page of older Entries.
    #[serde(rename = "next")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
}

impl EntryPageInfo {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(kind: models::EntryPageKind) -> EntryPageInfo {
        EntryPageInfo {
            kind,
            offset: None,
            total_count: None,
            snapshot: None,
            previous: None,
            next: None,
        }
    }
}

/// Converts the EntryPageInfo value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for EntryPageInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping kind in query parameter serialization
            self.offset.as_ref().map(|offset| ["offset".to_string(), offset.to_string()].join(",")),
            self.total_count.as_ref().map(|total_count| ["totalCount".to_string(), total_count.to_string()].join(",")),
            self.snapshot.as_ref().map(|snapshot| ["snapshot".to_string(), snapshot.to_string()].join(",")),
            self.previous.as_ref().map(|previous| ["previous".to_string(), previous.to_string()].join(",")),
            self.next.as_ref().map(|next| ["next".to_string(), next.to_string()].join(",")),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a EntryPageInfo value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for EntryPageInfo {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub kind: Vec<models::EntryPageKind>,
            pub offset: Vec<u64>,
            pub total_count: Vec<u64>,
            pub snapshot: Vec<String>,
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
                None => return std::result::Result::Err("Missing value while parsing EntryPageInfo".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "kind" => intermediate_rep
                        .kind
                        .push(<models::EntryPageKind as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "offset" => intermediate_rep.offset.push(<u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "totalCount" => intermediate_rep.total_count.push(<u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "snapshot" => intermediate_rep.snapshot.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "previous" => intermediate_rep.previous.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "next" => intermediate_rep.next.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing EntryPageInfo".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(EntryPageInfo {
            kind: intermediate_rep.kind.into_iter().next().ok_or_else(|| "kind missing in EntryPageInfo".to_string())?,
            offset: intermediate_rep.offset.into_iter().next(),
            total_count: intermediate_rep.total_count.into_iter().next(),
            snapshot: intermediate_rep.snapshot.into_iter().next(),
            previous: intermediate_rep.previous.into_iter().next(),
            next: intermediate_rep.next.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<EntryPageInfo> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<EntryPageInfo>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<EntryPageInfo>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for EntryPageInfo - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<EntryPageInfo> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <EntryPageInfo as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into EntryPageInfo - {err}"#)),
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
pub enum EntryPageKind {
    #[serde(rename = "offset")]
    Offset,
    #[serde(rename = "cursor")]
    Cursor,
}

impl validator::Validate for EntryPageKind {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for EntryPageKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            EntryPageKind::Offset => write!(f, "offset"),
            EntryPageKind::Cursor => write!(f, "cursor"),
        }
    }
}

impl std::str::FromStr for EntryPageKind {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "offset" => std::result::Result::Ok(EntryPageKind::Offset),
            "cursor" => std::result::Result::Ok(EntryPageKind::Cursor),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
        }
    }
}

/// Where the page starts. `kind` is `offset` for every sort except `timeline_date`, and `cursor` with it. An `offset` page takes `offset` or `locate`; a `cursor` page takes at most one of `after`, `before`, `month`, and `around`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct EntryPageRequest {
    #[serde(rename = "kind")]
    #[validate(nested)]
    pub kind: models::EntryPageKind,

    /// Defaults to `200`.
    #[serde(rename = "limit")]
    #[validate(range(min = 1u32, max = 1000u32))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    /// Defaults to `0`.
    #[serde(rename = "offset")]
    #[validate(range(min = 0u64))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,

    /// The Entry to find, named as a start is named. Not with `offset`.
    #[serde(rename = "locate")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locate: Option<models::LibraryPath>,

    /// A cursor from `next`; returns the Entries older than it.
    #[serde(rename = "after")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    /// A cursor from `previous`; returns the Entries newer than it.
    #[serde(rename = "before")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,

    /// Starts at the newest Entry of this month (`YYYY-MM`), or of the next older month that has Entries.
    #[serde(rename = "month")]
    #[validate(
            regex(path = *RE_ENTRYPAGEREQUEST_MONTH),
          custom(function = "check_xss_string"),
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub month: Option<String>,

    /// The Entry to start around, named as a start is named. The page has it near its middle, and an Entry that is not in the sequence is 404.
    #[serde(rename = "around")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub around: Option<models::LibraryPath>,
}

lazy_static::lazy_static! {
    static ref RE_ENTRYPAGEREQUEST_MONTH: regex::Regex = regex::Regex::new("^[0-9]{4}-[0-9]{2}$").unwrap();
}

impl EntryPageRequest {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(kind: models::EntryPageKind) -> EntryPageRequest {
        EntryPageRequest {
            kind,
            limit: None,
            offset: None,
            locate: None,
            after: None,
            before: None,
            month: None,
            around: None,
        }
    }
}

/// Converts the EntryPageRequest value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for EntryPageRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping kind in query parameter serialization
            self.limit.as_ref().map(|limit| ["limit".to_string(), limit.to_string()].join(",")),
            self.offset.as_ref().map(|offset| ["offset".to_string(), offset.to_string()].join(",")),
            // Skipping locate in query parameter serialization
            self.after.as_ref().map(|after| ["after".to_string(), after.to_string()].join(",")),
            self.before.as_ref().map(|before| ["before".to_string(), before.to_string()].join(",")),
            self.month.as_ref().map(|month| ["month".to_string(), month.to_string()].join(",")),
            // Skipping around in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a EntryPageRequest value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for EntryPageRequest {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub kind: Vec<models::EntryPageKind>,
            pub limit: Vec<u32>,
            pub offset: Vec<u64>,
            pub locate: Vec<models::LibraryPath>,
            pub after: Vec<String>,
            pub before: Vec<String>,
            pub month: Vec<String>,
            pub around: Vec<models::LibraryPath>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing EntryPageRequest".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "kind" => intermediate_rep
                        .kind
                        .push(<models::EntryPageKind as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "limit" => intermediate_rep.limit.push(<u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "offset" => intermediate_rep.offset.push(<u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "locate" => intermediate_rep
                        .locate
                        .push(<models::LibraryPath as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "after" => intermediate_rep.after.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "before" => intermediate_rep.before.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "month" => intermediate_rep.month.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "around" => intermediate_rep
                        .around
                        .push(<models::LibraryPath as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing EntryPageRequest".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(EntryPageRequest {
            kind: intermediate_rep.kind.into_iter().next().ok_or_else(|| "kind missing in EntryPageRequest".to_string())?,
            limit: intermediate_rep.limit.into_iter().next(),
            offset: intermediate_rep.offset.into_iter().next(),
            locate: intermediate_rep.locate.into_iter().next(),
            after: intermediate_rep.after.into_iter().next(),
            before: intermediate_rep.before.into_iter().next(),
            month: intermediate_rep.month.into_iter().next(),
            around: intermediate_rep.around.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<EntryPageRequest> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<EntryPageRequest>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<EntryPageRequest>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for EntryPageRequest - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<EntryPageRequest> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <EntryPageRequest as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into EntryPageRequest - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// One Entry: its Library, its Library-relative path, and the face of the path. `view=folder` is a folder or an archive's root; any other value, or none, is the file, an archive file included.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct EntryRef {
    /// Library identifier chosen by the operator in the daemon configuration file. Unlike the daemon-issued identifiers in this API it is a readable name, not a generated token. It is what the API uses in place of a host path, so the physical location of a Library is never exposed.  Because the value is used as a URL path segment, the character set is restricted. The daemon rejects a configuration that violates it at startup.
    #[serde(rename = "libraryId")]
    #[validate(
            length(min = 1, max = 64),
            regex(path = *RE_ENTRYREF_LIBRARY_ID),
          custom(function = "check_xss_string"),
    )]
    pub library_id: String,

    /// Absolute-looking path within the selected Library. The resolved path must never escape that library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096), custom(function = "check_xss_string"))]
    pub path: String,

    #[serde(rename = "view")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view: Option<models::EntryView>,
}

lazy_static::lazy_static! {
    static ref RE_ENTRYREF_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

impl EntryRef {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(library_id: String, path: String) -> EntryRef {
        EntryRef { library_id, path, view: None }
    }
}

/// Converts the EntryRef value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for EntryRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("libraryId".to_string()),
            Some(self.library_id.to_string()),
            Some("path".to_string()),
            Some(self.path.to_string()),
            // Skipping view in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a EntryRef value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for EntryRef {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub library_id: Vec<String>,
            pub path: Vec<String>,
            pub view: Vec<models::EntryView>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing EntryRef".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "libraryId" => intermediate_rep.library_id.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "path" => intermediate_rep.path.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "view" => intermediate_rep
                        .view
                        .push(<models::EntryView as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing EntryRef".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(EntryRef {
            library_id: intermediate_rep.library_id.into_iter().next().ok_or_else(|| "libraryId missing in EntryRef".to_string())?,
            path: intermediate_rep.path.into_iter().next().ok_or_else(|| "path missing in EntryRef".to_string())?,
            view: intermediate_rep.view.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<EntryRef> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<EntryRef>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<EntryRef>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for EntryRef - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<EntryRef> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <EntryRef as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into EntryRef - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct EntryResponse {
    #[serde(rename = "entry")]
    #[validate(nested)]
    pub entry: models::FileSystemEntry,
}

impl EntryResponse {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(entry: models::FileSystemEntry) -> EntryResponse {
        EntryResponse { entry }
    }
}

/// Converts the EntryResponse value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for EntryResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping entry in query parameter serialization

        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a EntryResponse value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for EntryResponse {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub entry: Vec<models::FileSystemEntry>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing EntryResponse".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "entry" => intermediate_rep
                        .entry
                        .push(<models::FileSystemEntry as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing EntryResponse".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(EntryResponse {
            entry: intermediate_rep.entry.into_iter().next().ok_or_else(|| "entry missing in EntryResponse".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<EntryResponse> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<EntryResponse>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<EntryResponse>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for EntryResponse - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<EntryResponse> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <EntryResponse as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into EntryResponse - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// The starts of a listing and how far it reads from them.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct EntryScope {
    #[serde(rename = "starts")]
    #[validate(length(min = 1, max = 64), nested)]
    pub starts: Vec<models::LibraryPath>,

    /// Whether to include descendants below each start. Defaults to `false`.
    #[serde(rename = "recursive")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recursive: Option<bool>,

    /// Whether the Scope reads the archives found under its starts like folders, down to the nesting limit. It changes nothing without `recursive`. Defaults to `false`.
    #[serde(rename = "insideArchives")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inside_archives: Option<bool>,
}

impl EntryScope {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(starts: Vec<models::LibraryPath>) -> EntryScope {
        EntryScope {
            starts,
            recursive: None,
            inside_archives: None,
        }
    }
}

/// Converts the EntryScope value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for EntryScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping starts in query parameter serialization
            self.recursive.as_ref().map(|recursive| ["recursive".to_string(), recursive.to_string()].join(",")),
            self.inside_archives
                .as_ref()
                .map(|inside_archives| ["insideArchives".to_string(), inside_archives.to_string()].join(",")),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a EntryScope value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for EntryScope {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub starts: Vec<Vec<models::LibraryPath>>,
            pub recursive: Vec<bool>,
            pub inside_archives: Vec<bool>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing EntryScope".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "starts" => return std::result::Result::Err("Parsing a container in this style is not supported in EntryScope".to_string()),
                    #[allow(clippy::redundant_clone)]
                    "recursive" => intermediate_rep.recursive.push(<bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "insideArchives" => intermediate_rep
                        .inside_archives
                        .push(<bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing EntryScope".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(EntryScope {
            starts: intermediate_rep.starts.into_iter().next().ok_or_else(|| "starts missing in EntryScope".to_string())?,
            recursive: intermediate_rep.recursive.into_iter().next(),
            inside_archives: intermediate_rep.inside_archives.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<EntryScope> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<EntryScope>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<EntryScope>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for EntryScope - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<EntryScope> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <EntryScope as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into EntryScope - {err}"#)),
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

/// Copy or move request. The sources are in one Library; the destination may be in another.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct EntryTransferRequest {
    #[serde(rename = "sources")]
    #[validate(length(min = 1, max = 1000), nested)]
    pub sources: Vec<models::EntryRef>,

    #[serde(rename = "destination")]
    #[validate(nested)]
    pub destination: models::EntryDestination,

    #[serde(rename = "conflict")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict: Option<models::ConflictPolicy>,
}

impl EntryTransferRequest {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(sources: Vec<models::EntryRef>, destination: models::EntryDestination) -> EntryTransferRequest {
        EntryTransferRequest {
            sources,
            destination,
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
            // Skipping sources in query parameter serialization

            // Skipping destination in query parameter serialization

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
            pub sources: Vec<Vec<models::EntryRef>>,
            pub destination: Vec<models::EntryDestination>,
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
                    "sources" => return std::result::Result::Err("Parsing a container in this style is not supported in EntryTransferRequest".to_string()),
                    #[allow(clippy::redundant_clone)]
                    "destination" => intermediate_rep
                        .destination
                        .push(<models::EntryDestination as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
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
            sources: intermediate_rep
                .sources
                .into_iter()
                .next()
                .ok_or_else(|| "sources missing in EntryTransferRequest".to_string())?,
            destination: intermediate_rep
                .destination
                .into_iter()
                .next()
                .ok_or_else(|| "destination missing in EntryTransferRequest".to_string())?,
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

/// The face of a path. `file` is the file, an archive file included; `folder` is a folder or an archive's root.
/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types, clippy::large_enum_variant)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk_enum_derive::LabelledGenericEnum))]
pub enum EntryView {
    #[serde(rename = "file")]
    File,
    #[serde(rename = "folder")]
    Folder,
}

impl validator::Validate for EntryView {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for EntryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            EntryView::File => write!(f, "file"),
            EntryView::Folder => write!(f, "folder"),
        }
    }
}

impl std::str::FromStr for EntryView {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "file" => std::result::Result::Ok(EntryView::File),
            "folder" => std::result::Result::Ok(EntryView::Folder),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct EntryWarnings {
    /// Starts, as they were given, whose recursive traversal was cut off at the entry limit.
    #[serde(rename = "cutOffStarts")]
    #[validate(nested)]
    pub cut_off_starts: Vec<models::LibraryPath>,

    /// Starts, as they were given, that the name index would answer from a local folder but that were walked because the index of their Library is not usable.
    #[serde(rename = "walkedStarts")]
    #[validate(nested)]
    pub walked_starts: Vec<models::LibraryPath>,

    /// Starts, as they were given, that were skipped because their folder or Library does not exist or they name a file that is neither a folder nor an archive.
    #[serde(rename = "missingStarts")]
    #[validate(nested)]
    pub missing_starts: Vec<models::LibraryPath>,
}

impl EntryWarnings {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(cut_off_starts: Vec<models::LibraryPath>, walked_starts: Vec<models::LibraryPath>, missing_starts: Vec<models::LibraryPath>) -> EntryWarnings {
        EntryWarnings {
            cut_off_starts,
            walked_starts,
            missing_starts,
        }
    }
}

/// Converts the EntryWarnings value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for EntryWarnings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping cutOffStarts in query parameter serialization

            // Skipping walkedStarts in query parameter serialization

            // Skipping missingStarts in query parameter serialization

        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a EntryWarnings value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for EntryWarnings {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub cut_off_starts: Vec<Vec<models::LibraryPath>>,
            pub walked_starts: Vec<Vec<models::LibraryPath>>,
            pub missing_starts: Vec<Vec<models::LibraryPath>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing EntryWarnings".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "cutOffStarts" => return std::result::Result::Err("Parsing a container in this style is not supported in EntryWarnings".to_string()),
                    "walkedStarts" => return std::result::Result::Err("Parsing a container in this style is not supported in EntryWarnings".to_string()),
                    "missingStarts" => return std::result::Result::Err("Parsing a container in this style is not supported in EntryWarnings".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing EntryWarnings".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(EntryWarnings {
            cut_off_starts: intermediate_rep
                .cut_off_starts
                .into_iter()
                .next()
                .ok_or_else(|| "cutOffStarts missing in EntryWarnings".to_string())?,
            walked_starts: intermediate_rep
                .walked_starts
                .into_iter()
                .next()
                .ok_or_else(|| "walkedStarts missing in EntryWarnings".to_string())?,
            missing_starts: intermediate_rep
                .missing_starts
                .into_iter()
                .next()
                .ok_or_else(|| "missingStarts missing in EntryWarnings".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<EntryWarnings> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<EntryWarnings>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<EntryWarnings>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for EntryWarnings - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<EntryWarnings> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <EntryWarnings as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into EntryWarnings - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// `UnsupportedType`, `LimitExceeded`, and `PasswordRequired` describe the target file rather than the request, so a client can tell \"cannot open this format\", \"too large to handle\", and \"password protected\" apart without parsing `detail`. `MethodNotAllowed` means that the path exists but does not accept the method. `Timeout` means that extracting an archive or generating a thumbnail did not finish within the daemon's time limit. The result is not cached, so repeating the same request can hit the same limit again; no `Retry-After` is returned. `SessionEvicted` means that the daemon ended a video session to make way for a newer one.
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
    #[serde(rename = "MethodNotAllowed")]
    MethodNotAllowed,
    #[serde(rename = "SessionEvicted")]
    SessionEvicted,
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
            ErrorKind::MethodNotAllowed => write!(f, "MethodNotAllowed"),
            ErrorKind::SessionEvicted => write!(f, "SessionEvicted"),
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
            "MethodNotAllowed" => std::result::Result::Ok(ErrorKind::MethodNotAllowed),
            "SessionEvicted" => std::result::Result::Ok(ErrorKind::SessionEvicted),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
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

/// The display values saved for one folder. The path is returned in normal form, with `.` and `..` resolved.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct FolderSetting {
    /// Library identifier chosen by the operator in the daemon configuration file. Unlike the daemon-issued identifiers in this API it is a readable name, not a generated token. It is what the API uses in place of a host path, so the physical location of a Library is never exposed.  Because the value is used as a URL path segment, the character set is restricted. The daemon rejects a configuration that violates it at startup.
    #[serde(rename = "libraryId")]
    #[validate(
            length(min = 1, max = 64),
            regex(path = *RE_FOLDERSETTING_LIBRARY_ID),
          custom(function = "check_xss_string"),
    )]
    pub library_id: String,

    /// Absolute-looking path within the selected Library. The resolved path must never escape that library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096), custom(function = "check_xss_string"))]
    pub path: String,

    #[serde(rename = "display")]
    #[validate(nested)]
    pub display: models::DisplayValues,
}

lazy_static::lazy_static! {
    static ref RE_FOLDERSETTING_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

impl FolderSetting {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(library_id: String, path: String, display: models::DisplayValues) -> FolderSetting {
        FolderSetting { library_id, path, display }
    }
}

/// Converts the FolderSetting value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for FolderSetting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("libraryId".to_string()),
            Some(self.library_id.to_string()),
            Some("path".to_string()),
            Some(self.path.to_string()),
            // Skipping display in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a FolderSetting value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for FolderSetting {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub library_id: Vec<String>,
            pub path: Vec<String>,
            pub display: Vec<models::DisplayValues>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing FolderSetting".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "libraryId" => intermediate_rep.library_id.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "path" => intermediate_rep.path.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "display" => intermediate_rep
                        .display
                        .push(<models::DisplayValues as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing FolderSetting".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(FolderSetting {
            library_id: intermediate_rep
                .library_id
                .into_iter()
                .next()
                .ok_or_else(|| "libraryId missing in FolderSetting".to_string())?,
            path: intermediate_rep.path.into_iter().next().ok_or_else(|| "path missing in FolderSetting".to_string())?,
            display: intermediate_rep.display.into_iter().next().ok_or_else(|| "display missing in FolderSetting".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<FolderSetting> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<FolderSetting>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<FolderSetting>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for FolderSetting - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<FolderSetting> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <FolderSetting as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into FolderSetting - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct FolderSettingList {
    #[serde(rename = "settings")]
    #[validate(nested)]
    pub settings: Vec<models::FolderSetting>,
}

impl FolderSettingList {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(settings: Vec<models::FolderSetting>) -> FolderSettingList {
        FolderSettingList { settings }
    }
}

/// Converts the FolderSettingList value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for FolderSettingList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping settings in query parameter serialization

        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a FolderSettingList value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for FolderSettingList {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub settings: Vec<Vec<models::FolderSetting>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing FolderSettingList".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "settings" => return std::result::Result::Err("Parsing a container in this style is not supported in FolderSettingList".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing FolderSettingList".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(FolderSettingList {
            settings: intermediate_rep
                .settings
                .into_iter()
                .next()
                .ok_or_else(|| "settings missing in FolderSettingList".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<FolderSettingList> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<FolderSettingList>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<FolderSettingList>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for FolderSettingList - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<FolderSettingList> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <FolderSettingList as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into FolderSettingList - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

/// The display values to save for the folder named in the query.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct FolderSettingRequest {
    #[serde(rename = "display")]
    #[validate(nested)]
    pub display: models::DisplayValues,
}

impl FolderSettingRequest {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(display: models::DisplayValues) -> FolderSettingRequest {
        FolderSettingRequest { display }
    }
}

/// Converts the FolderSettingRequest value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for FolderSettingRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping display in query parameter serialization

        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a FolderSettingRequest value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for FolderSettingRequest {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub display: Vec<models::DisplayValues>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing FolderSettingRequest".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "display" => intermediate_rep
                        .display
                        .push(<models::DisplayValues as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing FolderSettingRequest".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(FolderSettingRequest {
            display: intermediate_rep
                .display
                .into_iter()
                .next()
                .ok_or_else(|| "display missing in FolderSettingRequest".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<FolderSettingRequest> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<FolderSettingRequest>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<FolderSettingRequest>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for FolderSettingRequest - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<FolderSettingRequest> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <FolderSettingRequest as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into FolderSettingRequest - {err}"#)),
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

/// A place in a Library: a start of a Scope, an Entry, or a folder. A listing keeps the path of a start as it was given, so its response names a start by the path it was sent with. A saved Shelf and the folder settings return their paths in normal form.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct LibraryPath {
    /// Library identifier chosen by the operator in the daemon configuration file. Unlike the daemon-issued identifiers in this API it is a readable name, not a generated token. It is what the API uses in place of a host path, so the physical location of a Library is never exposed.  Because the value is used as a URL path segment, the character set is restricted. The daemon rejects a configuration that violates it at startup.
    #[serde(rename = "libraryId")]
    #[validate(
            length(min = 1, max = 64),
            regex(path = *RE_LIBRARYPATH_LIBRARY_ID),
          custom(function = "check_xss_string"),
    )]
    pub library_id: String,

    /// Absolute-looking path within the selected Library. The resolved path must never escape that library.
    #[serde(rename = "path")]
    #[validate(length(min = 1, max = 4096), custom(function = "check_xss_string"))]
    pub path: String,
}

lazy_static::lazy_static! {
    static ref RE_LIBRARYPATH_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

impl LibraryPath {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(library_id: String, path: String) -> LibraryPath {
        LibraryPath { library_id, path }
    }
}

/// Converts the LibraryPath value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for LibraryPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("libraryId".to_string()),
            Some(self.library_id.to_string()),
            Some("path".to_string()),
            Some(self.path.to_string()),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a LibraryPath value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for LibraryPath {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub library_id: Vec<String>,
            pub path: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing LibraryPath".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "libraryId" => intermediate_rep.library_id.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "path" => intermediate_rep.path.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing LibraryPath".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(LibraryPath {
            library_id: intermediate_rep
                .library_id
                .into_iter()
                .next()
                .ok_or_else(|| "libraryId missing in LibraryPath".to_string())?,
            path: intermediate_rep.path.into_iter().next().ok_or_else(|| "path missing in LibraryPath".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<LibraryPath> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<LibraryPath>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<LibraryPath>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for LibraryPath - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<LibraryPath> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <LibraryPath as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into LibraryPath - {err}"#)),
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
    #[serde(rename = "copy")]
    Copy,
    #[serde(rename = "move")]
    Move,
    #[serde(rename = "delete")]
    Delete,
    #[serde(rename = "timelineScan")]
    TimelineScan,
}

impl validator::Validate for OperationKind {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for OperationKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            OperationKind::Copy => write!(f, "copy"),
            OperationKind::Move => write!(f, "move"),
            OperationKind::Delete => write!(f, "delete"),
            OperationKind::TimelineScan => write!(f, "timelineScan"),
        }
    }
}

impl std::str::FromStr for OperationKind {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "copy" => std::result::Result::Ok(OperationKind::Copy),
            "move" => std::result::Result::Ok(OperationKind::Move),
            "delete" => std::result::Result::Ok(OperationKind::Delete),
            "timelineScan" => std::result::Result::Ok(OperationKind::TimelineScan),
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

/// Display preferences. `wide` and `narrow` are chosen by the width of the screen (768 CSS px); `sort`, `archiveOpen`, and `timelineLibraries` do not depend on it.
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

    #[serde(rename = "archiveOpen")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive_open: Option<models::ArchiveOpen>,

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
            archive_open: None,
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

            // Skipping archiveOpen in query parameter serialization
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
            pub archive_open: Vec<models::ArchiveOpen>,
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
                    #[allow(clippy::redundant_clone)]
                    "archiveOpen" => intermediate_rep
                        .archive_open
                        .push(<models::ArchiveOpen as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
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
            archive_open: intermediate_rep.archive_open.into_iter().next(),
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

/// An RFC 9457 problem document. Every error response is one, served as `application/problem+json`, including a missing route, a method the route does not allow, a body that does not fit the contract, and a missing session. A body that does not fit the types of the contract is answered with status 422.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct Problem {
    /// Always `about:blank`; a client branches on `code`.
    #[serde(rename = "type")]
    #[validate(custom(function = "check_xss_string"))]
    pub r_type: String,

    /// The reason phrase of `status`.
    #[serde(rename = "title")]
    #[validate(custom(function = "check_xss_string"))]
    pub title: String,

    #[serde(rename = "status")]
    pub status: i32,

    #[serde(rename = "code")]
    #[validate(nested)]
    pub code: models::ErrorKind,

    /// A message for a person. It never contains a host path, and it is absent for an internal error.
    #[serde(rename = "detail")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,

    /// Identifies this error. An internal error is logged under it.
    #[serde(rename = "requestId")]
    #[validate(custom(function = "check_xss_string"))]
    pub request_id: String,
}

impl Problem {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(r_type: String, title: String, status: i32, code: models::ErrorKind, request_id: String) -> Problem {
        Problem {
            r_type,
            title,
            status,
            code,
            detail: None,
            request_id,
        }
    }
}

/// Converts the Problem value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("type".to_string()),
            Some(self.r_type.to_string()),
            Some("title".to_string()),
            Some(self.title.to_string()),
            Some("status".to_string()),
            Some(self.status.to_string()),
            // Skipping code in query parameter serialization
            self.detail.as_ref().map(|detail| ["detail".to_string(), detail.to_string()].join(",")),
            Some("requestId".to_string()),
            Some(self.request_id.to_string()),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a Problem value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for Problem {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub r_type: Vec<String>,
            pub title: Vec<String>,
            pub status: Vec<i32>,
            pub code: Vec<models::ErrorKind>,
            pub detail: Vec<String>,
            pub request_id: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing Problem".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "type" => intermediate_rep.r_type.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "title" => intermediate_rep.title.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "status" => intermediate_rep.status.push(<i32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "code" => intermediate_rep
                        .code
                        .push(<models::ErrorKind as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "detail" => intermediate_rep.detail.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "requestId" => intermediate_rep.request_id.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing Problem".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(Problem {
            r_type: intermediate_rep.r_type.into_iter().next().ok_or_else(|| "type missing in Problem".to_string())?,
            title: intermediate_rep.title.into_iter().next().ok_or_else(|| "title missing in Problem".to_string())?,
            status: intermediate_rep.status.into_iter().next().ok_or_else(|| "status missing in Problem".to_string())?,
            code: intermediate_rep.code.into_iter().next().ok_or_else(|| "code missing in Problem".to_string())?,
            detail: intermediate_rep.detail.into_iter().next(),
            request_id: intermediate_rep.request_id.into_iter().next().ok_or_else(|| "requestId missing in Problem".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<Problem> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<Problem>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<Problem>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for Problem - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<Problem> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <Problem as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into Problem - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct PutEntryTagsRequest {
    #[serde(rename = "tags")]
    #[validate(length(max = 32), nested)]
    pub tags: Vec<models::TagName>,
}

impl PutEntryTagsRequest {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(tags: Vec<models::TagName>) -> PutEntryTagsRequest {
        PutEntryTagsRequest { tags }
    }
}

/// Converts the PutEntryTagsRequest value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for PutEntryTagsRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![Some("tags".to_string()), Some(self.tags.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))];

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
                    "tags" => return std::result::Result::Err("Parsing a container in this style is not supported in PutEntryTagsRequest".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing PutEntryTagsRequest".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(PutEntryTagsRequest {
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
    #[serde(rename = "rating")]
    #[validate(nested)]
    pub rating: models::RatingValue,
}

impl PutRatingRequest {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(rating: models::RatingValue) -> PutRatingRequest {
        PutRatingRequest { rating }
    }
}

/// Converts the PutRatingRequest value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for PutRatingRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
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
pub struct ScopedEntry {
    /// Library identifier chosen by the operator in the daemon configuration file. Unlike the daemon-issued identifiers in this API it is a readable name, not a generated token. It is what the API uses in place of a host path, so the physical location of a Library is never exposed.  Because the value is used as a URL path segment, the character set is restricted. The daemon rejects a configuration that violates it at startup.
    #[serde(rename = "libraryId")]
    #[validate(
            length(min = 1, max = 64),
            regex(path = *RE_SCOPEDENTRY_LIBRARY_ID),
          custom(function = "check_xss_string"),
    )]
    pub library_id: String,

    #[serde(rename = "entry")]
    #[validate(nested)]
    pub entry: models::FileSystemEntry,

    /// Present with `sort=timeline_date`. Date the Entry is ordered by, as `YYYY-MM-DDTHH:MM:SS` without a time zone, in the daemon's configured time zone.
    #[serde(rename = "timelineDate")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeline_date: Option<String>,

    /// Present with `timelineDate`. Where it came from.
    /// Note: inline enums are not fully supported by openapi-generator
    #[serde(rename = "timelineDateSource")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeline_date_source: Option<String>,
}

lazy_static::lazy_static! {
    static ref RE_SCOPEDENTRY_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

impl ScopedEntry {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(library_id: String, entry: models::FileSystemEntry) -> ScopedEntry {
        ScopedEntry {
            library_id,
            entry,
            timeline_date: None,
            timeline_date_source: None,
        }
    }
}

/// Converts the ScopedEntry value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for ScopedEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("libraryId".to_string()),
            Some(self.library_id.to_string()),
            // Skipping entry in query parameter serialization
            self.timeline_date
                .as_ref()
                .map(|timeline_date| ["timelineDate".to_string(), timeline_date.to_string()].join(",")),
            self.timeline_date_source
                .as_ref()
                .map(|timeline_date_source| ["timelineDateSource".to_string(), timeline_date_source.to_string()].join(",")),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a ScopedEntry value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for ScopedEntry {
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
                None => return std::result::Result::Err("Missing value while parsing ScopedEntry".to_string()),
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
                    _ => return std::result::Result::Err("Unexpected key while parsing ScopedEntry".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(ScopedEntry {
            library_id: intermediate_rep
                .library_id
                .into_iter()
                .next()
                .ok_or_else(|| "libraryId missing in ScopedEntry".to_string())?,
            entry: intermediate_rep.entry.into_iter().next().ok_or_else(|| "entry missing in ScopedEntry".to_string())?,
            timeline_date: intermediate_rep.timeline_date.into_iter().next(),
            timeline_date_source: intermediate_rep.timeline_date_source.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<ScopedEntry> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<ScopedEntry>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<ScopedEntry>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for ScopedEntry - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<ScopedEntry> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <ScopedEntry as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into ScopedEntry - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ScopedEntrySummary {
    /// Library identifier chosen by the operator in the daemon configuration file. Unlike the daemon-issued identifiers in this API it is a readable name, not a generated token. It is what the API uses in place of a host path, so the physical location of a Library is never exposed.  Because the value is used as a URL path segment, the character set is restricted. The daemon rejects a configuration that violates it at startup.
    #[serde(rename = "libraryId")]
    #[validate(
            length(min = 1, max = 64),
            regex(path = *RE_SCOPEDENTRYSUMMARY_LIBRARY_ID),
          custom(function = "check_xss_string"),
    )]
    pub library_id: String,

    #[serde(rename = "summary")]
    #[validate(nested)]
    pub summary: models::FileSystemEntrySummary,
}

lazy_static::lazy_static! {
    static ref RE_SCOPEDENTRYSUMMARY_LIBRARY_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9._-]+$").unwrap();
}

impl ScopedEntrySummary {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(library_id: String, summary: models::FileSystemEntrySummary) -> ScopedEntrySummary {
        ScopedEntrySummary { library_id, summary }
    }
}

/// Converts the ScopedEntrySummary value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for ScopedEntrySummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("libraryId".to_string()),
            Some(self.library_id.to_string()),
            // Skipping summary in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a ScopedEntrySummary value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for ScopedEntrySummary {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub library_id: Vec<String>,
            pub summary: Vec<models::FileSystemEntrySummary>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing ScopedEntrySummary".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "libraryId" => intermediate_rep.library_id.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "summary" => intermediate_rep
                        .summary
                        .push(<models::FileSystemEntrySummary as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing ScopedEntrySummary".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(ScopedEntrySummary {
            library_id: intermediate_rep
                .library_id
                .into_iter()
                .next()
                .ok_or_else(|| "libraryId missing in ScopedEntrySummary".to_string())?,
            summary: intermediate_rep
                .summary
                .into_iter()
                .next()
                .ok_or_else(|| "summary missing in ScopedEntrySummary".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<ScopedEntrySummary> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<ScopedEntrySummary>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<ScopedEntrySummary>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for ScopedEntrySummary - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<ScopedEntrySummary> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <ScopedEntrySummary as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into ScopedEntrySummary - {err}"#)),
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

/// A saved Shelf. Its Scope and Filter can be sent to `listEntries` as they are. The path of each start is returned in normal form, with `.` and `..` resolved.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct Shelf {
    #[serde(rename = "id")]
    #[validate(custom(function = "check_xss_string"))]
    pub id: String,

    #[serde(rename = "name")]
    #[validate(custom(function = "check_xss_string"))]
    pub name: String,

    #[serde(rename = "scope")]
    #[validate(nested)]
    pub scope: models::EntryScope,

    #[serde(rename = "filter")]
    #[validate(nested)]
    pub filter: models::EntryFilter,

    #[serde(rename = "display")]
    #[validate(nested)]
    pub display: models::DisplayValues,
}

impl Shelf {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(id: String, name: String, scope: models::EntryScope, filter: models::EntryFilter, display: models::DisplayValues) -> Shelf {
        Shelf { id, name, scope, filter, display }
    }
}

/// Converts the Shelf value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for Shelf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("id".to_string()),
            Some(self.id.to_string()),
            Some("name".to_string()),
            Some(self.name.to_string()),
            // Skipping scope in query parameter serialization

            // Skipping filter in query parameter serialization

            // Skipping display in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a Shelf value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for Shelf {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub id: Vec<String>,
            pub name: Vec<String>,
            pub scope: Vec<models::EntryScope>,
            pub filter: Vec<models::EntryFilter>,
            pub display: Vec<models::DisplayValues>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing Shelf".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "id" => intermediate_rep.id.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "name" => intermediate_rep.name.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "scope" => intermediate_rep
                        .scope
                        .push(<models::EntryScope as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "filter" => intermediate_rep
                        .filter
                        .push(<models::EntryFilter as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "display" => intermediate_rep
                        .display
                        .push(<models::DisplayValues as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing Shelf".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(Shelf {
            id: intermediate_rep.id.into_iter().next().ok_or_else(|| "id missing in Shelf".to_string())?,
            name: intermediate_rep.name.into_iter().next().ok_or_else(|| "name missing in Shelf".to_string())?,
            scope: intermediate_rep.scope.into_iter().next().ok_or_else(|| "scope missing in Shelf".to_string())?,
            filter: intermediate_rep.filter.into_iter().next().ok_or_else(|| "filter missing in Shelf".to_string())?,
            display: intermediate_rep.display.into_iter().next().ok_or_else(|| "display missing in Shelf".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<Shelf> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<Shelf>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<Shelf>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for Shelf - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<Shelf> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <Shelf as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into Shelf - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ShelfInput {
    /// 1 to 100 characters after surrounding whitespace is trimmed.
    #[serde(rename = "name")]
    #[validate(custom(function = "check_xss_string"))]
    pub name: String,

    #[serde(rename = "scope")]
    #[validate(nested)]
    pub scope: models::EntryScope,

    #[serde(rename = "filter")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<models::EntryFilter>,

    #[serde(rename = "display")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<models::DisplayValues>,
}

impl ShelfInput {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(name: String, scope: models::EntryScope) -> ShelfInput {
        ShelfInput {
            name,
            scope,
            filter: None,
            display: None,
        }
    }
}

/// Converts the ShelfInput value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for ShelfInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("name".to_string()),
            Some(self.name.to_string()),
            // Skipping scope in query parameter serialization

            // Skipping filter in query parameter serialization

            // Skipping display in query parameter serialization
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a ShelfInput value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for ShelfInput {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub name: Vec<String>,
            pub scope: Vec<models::EntryScope>,
            pub filter: Vec<models::EntryFilter>,
            pub display: Vec<models::DisplayValues>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing ShelfInput".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "name" => intermediate_rep.name.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "scope" => intermediate_rep
                        .scope
                        .push(<models::EntryScope as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "filter" => intermediate_rep
                        .filter
                        .push(<models::EntryFilter as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "display" => intermediate_rep
                        .display
                        .push(<models::DisplayValues as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing ShelfInput".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(ShelfInput {
            name: intermediate_rep.name.into_iter().next().ok_or_else(|| "name missing in ShelfInput".to_string())?,
            scope: intermediate_rep.scope.into_iter().next().ok_or_else(|| "scope missing in ShelfInput".to_string())?,
            filter: intermediate_rep.filter.into_iter().next(),
            display: intermediate_rep.display.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<ShelfInput> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<ShelfInput>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<ShelfInput>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for ShelfInput - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<ShelfInput> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <ShelfInput as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into ShelfInput - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ShelfList {
    #[serde(rename = "shelves")]
    #[validate(nested)]
    pub shelves: Vec<models::Shelf>,
}

impl ShelfList {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(shelves: Vec<models::Shelf>) -> ShelfList {
        ShelfList { shelves }
    }
}

/// Converts the ShelfList value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for ShelfList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping shelves in query parameter serialization

        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a ShelfList value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for ShelfList {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub shelves: Vec<Vec<models::Shelf>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing ShelfList".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "shelves" => return std::result::Result::Err("Parsing a container in this style is not supported in ShelfList".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing ShelfList".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(ShelfList {
            shelves: intermediate_rep.shelves.into_iter().next().ok_or_else(|| "shelves missing in ShelfList".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<ShelfList> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<ShelfList>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<ShelfList>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for ShelfList - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<ShelfList> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <ShelfList as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into ShelfList - {err}"#)),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Unable to convert header: {hdr_value:?} to string: {e}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ShelfOrder {
    #[serde(rename = "ids")]
    #[validate(length(max = 200), custom(function = "check_xss_vec_string"))]
    pub ids: Vec<String>,
}

impl ShelfOrder {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(ids: Vec<String>) -> ShelfOrder {
        ShelfOrder { ids }
    }
}

/// Converts the ShelfOrder value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for ShelfOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![Some("ids".to_string()), Some(self.ids.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a ShelfOrder value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for ShelfOrder {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub ids: Vec<Vec<String>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => return std::result::Result::Err("Missing value while parsing ShelfOrder".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "ids" => return std::result::Result::Err("Parsing a container in this style is not supported in ShelfOrder".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing ShelfOrder".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(ShelfOrder {
            ids: intermediate_rep.ids.into_iter().next().ok_or_else(|| "ids missing in ShelfOrder".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<ShelfOrder> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<ShelfOrder>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<ShelfOrder>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for ShelfOrder - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<ShelfOrder> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <ShelfOrder as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into ShelfOrder - {err}"#)),
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
pub struct TimelineScanRequest {
    #[serde(rename = "libraryIds")]
    #[validate(length(min = 1, max = 64), nested)]
    pub library_ids: Vec<models::LibraryId>,
}

impl TimelineScanRequest {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(library_ids: Vec<models::LibraryId>) -> TimelineScanRequest {
        TimelineScanRequest { library_ids }
    }
}

/// Converts the TimelineScanRequest value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for TimelineScanRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("libraryIds".to_string()),
            Some(self.library_ids.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")),
        ];

        write!(f, "{}", params.into_iter().flatten().collect::<Vec<_>>().join(","))
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a TimelineScanRequest value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for TimelineScanRequest {
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
                None => return std::result::Result::Err("Missing value while parsing TimelineScanRequest".to_string()),
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "libraryIds" => return std::result::Result::Err("Parsing a container in this style is not supported in TimelineScanRequest".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing TimelineScanRequest".to_string()),
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(TimelineScanRequest {
            library_ids: intermediate_rep
                .library_ids
                .into_iter()
                .next()
                .ok_or_else(|| "libraryIds missing in TimelineScanRequest".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<TimelineScanRequest> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<TimelineScanRequest>> for HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<TimelineScanRequest>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(r#"Invalid header value for TimelineScanRequest - value: {hdr_value} is invalid {e}"#)),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<TimelineScanRequest> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <TimelineScanRequest as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                std::result::Result::Err(err) => std::result::Result::Err(format!(r#"Unable to convert header value '{value}' into TimelineScanRequest - {err}"#)),
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

/// How a write to an exact path resolves an existing entry there: `fail` rejects with `409`, `skip` leaves it in place, and `overwrite` replaces it.
/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types, clippy::large_enum_variant)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk_enum_derive::LabelledGenericEnum))]
pub enum WriteConflictPolicy {
    #[serde(rename = "fail")]
    Fail,
    #[serde(rename = "skip")]
    Skip,
    #[serde(rename = "overwrite")]
    Overwrite,
}

impl validator::Validate for WriteConflictPolicy {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for WriteConflictPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            WriteConflictPolicy::Fail => write!(f, "fail"),
            WriteConflictPolicy::Skip => write!(f, "skip"),
            WriteConflictPolicy::Overwrite => write!(f, "overwrite"),
        }
    }
}

impl std::str::FromStr for WriteConflictPolicy {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "fail" => std::result::Result::Ok(WriteConflictPolicy::Fail),
            "skip" => std::result::Result::Ok(WriteConflictPolicy::Skip),
            "overwrite" => std::result::Result::Ok(WriteConflictPolicy::Overwrite),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
        }
    }
}
