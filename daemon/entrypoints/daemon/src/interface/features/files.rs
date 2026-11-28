use async_trait::async_trait;
use axum_extra::extract::CookieJar;
use headers::Host;
use http::Method;

use omnius_pxna_interface::{apis, models};

use crate::{
    error::{ApiError, ApiStatus},
    interface::{ApiServer, session::Claims},
    service::mutation::{DeleteResult, MoveResult},
};

#[allow(unused_variables)]
#[async_trait]
impl apis::files::Files for ApiServer {
    type Claims = Claims;

    async fn copy_files(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::CopyFilesPathParams,
        body: &models::EntryTransferRequest,
    ) -> std::result::Result<apis::files::CopyFilesResponse, ()> {
        Ok(match self.state.operations.copy(&path_params.library_id, body, false).await {
            Ok(operation) => apis::files::CopyFilesResponse::Status202_CopyOperationStarted(models::OperationResponse::new(operation)),
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn delete_files(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::DeleteFilesPathParams,
        body: &models::DeleteFilesRequest,
    ) -> std::result::Result<apis::files::DeleteFilesResponse, ()> {
        Ok(match self.state.operations.delete_entries(&path_params.library_id, &body.src_paths, false, false).await {
            Ok(DeleteResult::Completed) => apis::files::DeleteFilesResponse::Status204_FileDeleted,
            Ok(DeleteResult::Started(operation)) => apis::files::DeleteFilesResponse::Status202_DeleteOperationStarted(models::OperationResponse::new(operation)),
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn move_files(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::MoveFilesPathParams,
        body: &models::EntryTransferRequest,
    ) -> std::result::Result<apis::files::MoveFilesResponse, ()> {
        Ok(match self.state.operations.move_entries(&path_params.library_id, body, false).await {
            Ok(MoveResult::Completed(entries)) => apis::files::MoveFilesResponse::Status200_MovedSynchronously(models::EntryList::new(entries)),
            Ok(MoveResult::Started(operation)) => apis::files::MoveFilesResponse::Status202_MoveOperationStarted(models::OperationResponse::new(operation)),
            Err(error) => ApiError::from(error).into(),
        })
    }
}

impl From<ApiError> for apis::files::CopyFilesResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::files::DeleteFilesResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::files::MoveFilesResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            ApiStatus::Conflict => Self::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}
