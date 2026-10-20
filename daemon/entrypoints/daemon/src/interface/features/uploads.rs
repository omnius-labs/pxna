use async_trait::async_trait;
use axum::body::Bytes;
use axum_extra::extract::CookieJar;
use headers::Host;
use http::Method;

use omnius_pxna_interface::{apis, models};

use crate::{
    error::{ApiError, ApiStatus},
    interface::{ApiServer, session::Claims},
};

mod route;

pub use route::*;

#[allow(unused_variables)]
#[async_trait]
impl apis::uploads::Uploads for ApiServer {
    type Claims = Claims;

    async fn commit_upload(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::CommitUploadPathParams,
    ) -> std::result::Result<apis::uploads::CommitUploadResponse, ()> {
        Ok(match self.state.uploads.commit(&path_params.library_id, &path_params.upload_id).await {
            Ok(file) => apis::uploads::CommitUploadResponse::Status201_FileCreatedAtTheTargetPath(models::FileResponse::new(file)),
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn create_upload(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::CreateUploadPathParams,
        body: &models::UploadCreateRequest,
    ) -> std::result::Result<apis::uploads::CreateUploadResponse, ()> {
        Ok(match self.state.uploads.create(&path_params.library_id, body).await {
            Ok(file) => apis::uploads::CreateUploadResponse::Status201_UploadSessionCreated(models::UploadResponse::new(file)),
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn delete_upload(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::DeleteUploadPathParams,
    ) -> std::result::Result<apis::uploads::DeleteUploadResponse, ()> {
        Ok(match self.state.uploads.abort(&path_params.library_id, &path_params.upload_id).await {
            Ok(()) => apis::uploads::DeleteUploadResponse::Status204_UploadSessionDiscarded,
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn get_upload(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::GetUploadPathParams,
    ) -> std::result::Result<apis::uploads::GetUploadResponse, ()> {
        Ok(match self.state.uploads.get(&path_params.library_id, &path_params.upload_id).await {
            Ok(file) => apis::uploads::GetUploadResponse::Status200_UploadSessionState(models::UploadResponse::new(file)),
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn write_upload_content(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::WriteUploadContentPathParams,
        query_params: &models::WriteUploadContentQueryParams,
        body: &Bytes,
    ) -> std::result::Result<apis::uploads::WriteUploadContentResponse, ()> {
        Ok(
            match self.state.uploads.write(&path_params.library_id, &path_params.upload_id, query_params.offset, body).await {
                Ok(()) => apis::uploads::WriteUploadContentResponse::Status204_ChunkAccepted,
                Err(error) => ApiError::from(error).into(),
            },
        )
    }
}

impl From<ApiError> for apis::uploads::CommitUploadResponse {
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

impl From<ApiError> for apis::uploads::CreateUploadResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            ApiStatus::Conflict => Self::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(error.body),
            ApiStatus::InsufficientStorage => Self::Status507_TheDaemonDoesNotHaveEnoughDiskSpace(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::uploads::DeleteUploadResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::uploads::GetUploadResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::uploads::WriteUploadContentResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            ApiStatus::Conflict => Self::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(error.body),
            ApiStatus::InsufficientStorage => Self::Status507_TheDaemonDoesNotHaveEnoughDiskSpace(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

#[cfg(test)]
mod tests {
    use http::StatusCode;
    use omnius_pxna_engine::service::ArchiveOptions;
    use serde_json::json;
    use testresult::TestResult;

    use super::*;
    use crate::interface::server::tests::*;

    #[tokio::test]
    async fn uploads_resume_recheck_conflict_and_abort_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let router = router(&fixture);
        let setup = send(&router, Method::POST, "/api/v1/setup", &[], credentials("owner", "password1", "bearer")).await?;
        let bearer = bearer(&setup)?;
        let headers = &[("authorization", bearer.as_str())];
        let created = send(
            &router,
            Method::POST,
            "/api/v1/libraries/photos/uploads",
            headers,
            Some(json!({"path":"/upload.bin","size":4})),
        )
        .await?;
        assert_eq!(created.status, StatusCode::CREATED);
        let id = created.body["upload"]["id"].as_str().ok_or("missing upload id")?;
        let base = format!("/api/v1/libraries/photos/uploads/{id}");
        assert_eq!(
            binary(&router, &format!("{base}/content?offset=1"), &bearer, b"ab".to_vec()).await?.status,
            StatusCode::CONFLICT
        );
        assert_eq!(
            binary(&router, &format!("{base}/content?offset=0"), &bearer, b"ab".to_vec()).await?.status,
            StatusCode::NO_CONTENT
        );
        let resumed = send(&router, Method::GET, &base, headers, None).await?;
        assert_eq!(resumed.body["upload"]["committedSize"], 2);
        assert_eq!(send(&router, Method::POST, &format!("{base}/commit"), headers, None).await?.status, StatusCode::BAD_REQUEST);
        assert_eq!(
            binary(&router, &format!("{base}/content?offset=2"), &bearer, b"cde".to_vec()).await?.status,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            binary(&router, &format!("{base}/content?offset=2"), &bearer, b"cd".to_vec()).await?.status,
            StatusCode::NO_CONTENT
        );
        assert_eq!(send(&router, Method::POST, &format!("{base}/commit"), headers, None).await?.status, StatusCode::CREATED);
        assert_eq!(std::fs::read(fixture.server.state.conf.libraries[0].path.join("upload.bin"))?, b"abcd");
        let second = send(
            &router,
            Method::POST,
            "/api/v1/libraries/photos/uploads",
            headers,
            Some(json!({"path":"/collision.bin","size":0})),
        )
        .await?;
        let id = second.body["upload"]["id"].as_str().ok_or("missing id")?;
        std::fs::write(fixture.server.state.conf.libraries[0].path.join("collision.bin"), b"existing")?;
        assert_eq!(
            send(&router, Method::POST, &format!("/api/v1/libraries/photos/uploads/{id}/commit"), headers, None)
                .await?
                .status,
            StatusCode::CONFLICT
        );
        assert_eq!(std::fs::read(fixture.server.state.conf.libraries[0].path.join("collision.bin"))?, b"existing");
        let third = send(
            &router,
            Method::POST,
            "/api/v1/libraries/photos/uploads",
            headers,
            Some(json!({"path":"/aborted.bin","size":10})),
        )
        .await?;
        let id = third.body["upload"]["id"].as_str().ok_or("missing id")?;
        assert_eq!(
            send(&router, Method::GET, &format!("/api/v1/libraries/videos/uploads/{id}"), headers, None).await?.status,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            send(&router, Method::DELETE, &format!("/api/v1/libraries/photos/uploads/{id}"), headers, None)
                .await?
                .status,
            StatusCode::NO_CONTENT
        );
        assert!(!fixture.server.state.conf.libraries[0].path.join("aborted.bin").exists());
        assert_eq!(
            std::fs::read_dir(&fixture.server.state.conf.libraries[0].path)?
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.file_name().to_string_lossy().starts_with(".pxna-tmp-"))
                .count(),
            0
        );
        Ok(())
    }
}
