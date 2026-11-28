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
impl apis::folders::Folders for ApiServer {
    type Claims = Claims;

    async fn copy_folders(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::CopyFoldersPathParams,
        body: &models::EntryTransferRequest,
    ) -> std::result::Result<apis::folders::CopyFoldersResponse, ()> {
        Ok(match self.state.operations.copy(&path_params.library_id, body, true).await {
            Ok(operation) => apis::folders::CopyFoldersResponse::Status202_CopyOperationStarted(models::OperationResponse::new(operation)),
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn delete_folders(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::DeleteFoldersPathParams,
        body: &models::DeleteFoldersRequest,
    ) -> std::result::Result<apis::folders::DeleteFoldersResponse, ()> {
        Ok(
            match self
                .state
                .operations
                .delete_entries(&path_params.library_id, &body.src_paths, true, body.recursive.unwrap_or(false))
                .await
            {
                Ok(DeleteResult::Completed) => apis::folders::DeleteFoldersResponse::Status204_FoldersDeleted,
                Ok(DeleteResult::Started(operation)) => apis::folders::DeleteFoldersResponse::Status202_DeleteOperationStarted(models::OperationResponse::new(operation)),
                Err(error) => ApiError::from(error).into(),
            },
        )
    }

    async fn move_folders(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::MoveFoldersPathParams,
        body: &models::EntryTransferRequest,
    ) -> std::result::Result<apis::folders::MoveFoldersResponse, ()> {
        Ok(match self.state.operations.move_entries(&path_params.library_id, body, true).await {
            Ok(MoveResult::Completed(entries)) => apis::folders::MoveFoldersResponse::Status200_MovedSynchronously(models::EntryList::new(entries)),
            Ok(MoveResult::Started(operation)) => apis::folders::MoveFoldersResponse::Status202_MoveOperationStarted(models::OperationResponse::new(operation)),
            Err(error) => ApiError::from(error).into(),
        })
    }
}

impl From<ApiError> for apis::folders::CopyFoldersResponse {
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

impl From<ApiError> for apis::folders::DeleteFoldersResponse {
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

impl From<ApiError> for apis::folders::MoveFoldersResponse {
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
