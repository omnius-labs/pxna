use async_trait::async_trait;
use axum::body::Bytes;
use axum_extra::extract::CookieJar;
use headers::Host;
use http::Method;

use omnius_pxna_interface::{apis, models, types::ByteArray};

use crate::{
    error::{ApiError, ApiStatus},
    interface::{ApiServer, binary_body::BinaryBody, session::Claims},
    prelude::*,
    service::{
        library::ReadRange,
        mutation::{DeleteResult, LibraryMutation, MoveResult},
    },
};

mod route;

pub use route::*;

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

    async fn get_file(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::GetFilePathParams,
        query_params: &models::GetFileQueryParams,
    ) -> std::result::Result<apis::files::GetFileResponse, ()> {
        Ok(match self.state.library.get_file(&path_params.library_id, &query_params.path).await {
            Ok(file) => apis::files::GetFileResponse::Status200_FileMetadata(models::FileResponse::new(file)),
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

    async fn read_file_content(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        header_params: &models::ReadFileContentHeaderParams,
        path_params: &models::ReadFileContentPathParams,
        query_params: &models::ReadFileContentQueryParams,
    ) -> std::result::Result<apis::files::ReadFileContentResponse, ()> {
        let content = async {
            let range = ReadRange::parse(header_params.range.as_deref(), query_params.offset, query_params.limit)?;
            self.state.library.read_file_content(&path_params.library_id, &query_params.path, &range).await
        }
        .await;
        Ok(match content {
            Ok(content) => {
                let content_length = Some(content.bytes.len() as u64);
                let accept_ranges = Some("bytes".to_string());
                match content.content_range() {
                    Some(content_range) => apis::files::ReadFileContentResponse::Status206_PartialFileBytes {
                        body: ByteArray(content.bytes),
                        accept_ranges,
                        content_range: Some(content_range),
                        content_length,
                        e_tag: None,
                    },
                    None => apis::files::ReadFileContentResponse::Status200_FullOrRequestedFileBytes {
                        body: ByteArray(content.bytes),
                        accept_ranges,
                        content_length,
                        e_tag: None,
                    },
                }
            }
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn write_file_content(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::WriteFileContentPathParams,
        query_params: &models::WriteFileContentQueryParams,
        body: &Bytes,
    ) -> std::result::Result<apis::files::WriteFileContentResponse, ()> {
        if body.len() > BinaryBody::MAX_BYTES {
            return Ok(ApiError::from(Error::new(ErrorKind::PayloadTooLarge)).into());
        }
        Ok(
            match self
                .state
                .mutation
                .write(&path_params.library_id, &query_params.path, body, LibraryMutation::conflict(query_params.conflict))
                .await
            {
                Ok(file) => apis::files::WriteFileContentResponse::Status201_FileWritten(models::FileResponse::new(file)),
                Err(error) => ApiError::from(error).into(),
            },
        )
    }
}

impl From<ApiError> for apis::files::GetFileResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            ApiStatus::Conflict => Self::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(error.body),
            ApiStatus::GatewayTimeout => Self::Status504_ExtractionOrThumbnailGenerationTimedOut(error.body),
            ApiStatus::RangeNotSatisfiable | ApiStatus::PayloadTooLarge | ApiStatus::InsufficientStorage | ApiStatus::InternalServerError => {
                Self::Status500_InternalServerError(error.body)
            }
        }
    }
}

impl From<ApiError> for apis::files::ReadFileContentResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            ApiStatus::Conflict => Self::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(error.body),
            ApiStatus::RangeNotSatisfiable => Self::Status416_RequestedByteRangeCannotBeSatisfied(error.body),
            ApiStatus::GatewayTimeout => Self::Status504_ExtractionOrThumbnailGenerationTimedOut(error.body),
            ApiStatus::PayloadTooLarge | ApiStatus::InsufficientStorage | ApiStatus::InternalServerError => Self::Status500_InternalServerError(error.body),
        }
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

impl From<ApiError> for apis::files::WriteFileContentResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            ApiStatus::Conflict => Self::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(error.body),
            ApiStatus::PayloadTooLarge => Self::Status413_TheRequestBodyExceedsTheOne(error.body),
            ApiStatus::InsufficientStorage => Self::Status507_TheDaemonDoesNotHaveEnoughDiskSpace(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use http::StatusCode;
    use omnius_pxna_engine::service::ArchiveOptions;
    use omnius_pxna_interface::types::Nullable;
    use testresult::TestResult;
    use tower::ServiceExt as _;

    use super::*;
    use crate::interface::server::tests::*;

    #[tokio::test]
    async fn content_stream_ranges_empty_file_and_etag_test() -> TestResult {
        use futures::StreamExt as _;
        let fixture = fixture(ArchiveOptions::default()).await?;
        let router = router(&fixture);
        let setup = send(&router, Method::POST, "/api/v1/setup", &[], credentials("owner", "password1", "bearer")).await?;
        let bearer = bearer(&setup)?;
        let headers = &[("authorization", bearer.as_str())];
        let root = &fixture.server.state.conf.libraries[0].path;
        let large_len = 256 * 1024 * 1024;
        std::fs::File::create(root.join("large.bin"))?.set_len(large_len)?;
        let request = http::Request::builder()
            .method(Method::GET)
            .uri("/api/v1/libraries/photos/files/content?path=/large.bin")
            .header(http::header::HOST, "localhost")
            .header(http::header::AUTHORIZATION, &bearer)
            .body(Body::empty())?;
        let response = router.clone().oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[http::header::CONTENT_LENGTH], large_len.to_string());
        let mut stream = response.into_body().into_data_stream();
        assert!(stream.next().await.ok_or("no frame")??.len() <= 64 * 1024);
        drop(stream);
        let range = http::Request::builder()
            .method(Method::GET)
            .uri("/api/v1/libraries/photos/files/content?path=/large.bin")
            .header(http::header::HOST, "localhost")
            .header(http::header::AUTHORIZATION, &bearer)
            .header(http::header::RANGE, "bytes=0-")
            .body(Body::empty())?;
        let range = router.clone().oneshot(range).await?;
        assert_eq!(range.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(range.headers()[http::header::CONTENT_LENGTH], large_len.to_string(), "Range を旧 128 MiB 上限で切らない");
        drop(range);
        std::fs::write(root.join("empty.bin"), b"")?;
        let empty = send(&router, Method::GET, "/api/v1/libraries/photos/files/content?path=/empty.bin", headers, None).await?;
        assert_eq!(empty.status, StatusCode::OK);
        assert_eq!(empty.header("content-length"), Some("0"));
        let unsatisfied = send(
            &router,
            Method::GET,
            "/api/v1/libraries/photos/files/content?path=/empty.bin",
            &[("authorization", bearer.as_str()), ("range", "bytes=0-")],
            None,
        )
        .await?;
        assert_eq!(unsatisfied.status, StatusCode::RANGE_NOT_SATISFIABLE);
        assert_eq!(unsatisfied.header("content-range"), Some("bytes */0"));
        let uri = "/api/v1/libraries/photos/files/content?path=/photos/notes.txt";
        let original = send(&router, Method::GET, uri, headers, None).await?;
        let repeated = send(&router, Method::GET, uri, headers, None).await?;
        assert!(original.header("etag").is_some());
        assert_eq!(original.header("etag"), repeated.header("etag"));
        std::fs::write(root.join("photos/notes.txt"), b"changed content")?;
        let changed = send(&router, Method::GET, uri, headers, None).await?;
        assert_ne!(original.header("etag"), changed.header("etag"));
        Ok(())
    }

    async fn read(fixture: &Fixture, path: &str, range: Option<&str>) -> apis::files::ReadFileContentResponse {
        let header = models::ReadFileContentHeaderParams { range: range.map(str::to_string) };
        let params = models::ReadFileContentPathParams { library_id: "photos".to_string() };
        let query = models::ReadFileContentQueryParams {
            path: path.to_string(),
            offset: None,
            limit: None,
        };
        apis::files::Files::read_file_content(&fixture.server, &Method::GET, &host(), &CookieJar::default(), &claims(), &header, &params, &query)
            .await
            .unwrap()
    }

    async fn get_file(fixture: &Fixture, path: &str) -> apis::files::GetFileResponse {
        let params = models::GetFilePathParams { library_id: "photos".to_string() };
        let query = models::GetFileQueryParams { path: path.to_string() };
        apis::files::Files::get_file(&fixture.server, &Method::GET, &host(), &CookieJar::default(), &claims(), &params, &query)
            .await
            .unwrap()
    }

    fn error_kind(response: apis::files::ReadFileContentResponse) -> Option<(u16, models::ErrorKind)> {
        use apis::files::ReadFileContentResponse as R;
        match response {
            R::Status400_InvalidRequest(body) => Some((400, body.kind)),
            R::Status403_TheRequestedResourceCannotBeAccessedOrWritten(body) => Some((403, body.kind)),
            R::Status404_ResourceNotFound(body) => Some((404, body.kind)),
            R::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(body) => Some((409, body.kind)),
            R::Status416_RequestedByteRangeCannotBeSatisfied(body) => Some((416, body.kind)),
            R::Status500_InternalServerError(body) => Some((500, body.kind)),
            _ => None,
        }
    }

    #[tokio::test]
    async fn archive_file_itself_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let apis::files::ReadFileContentResponse::Status200_FullOrRequestedFileBytes { body, .. } = read(&fixture, "/photos/a.7z", None).await else {
            panic!("file endpoint must return the archive bytes");
        };
        assert_eq!(body.0, std::fs::read(&fixture.archive)?);
        Ok(())
    }

    #[tokio::test]
    async fn archive_inner_content_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let apis::files::ReadFileContentResponse::Status200_FullOrRequestedFileBytes { body, .. } = read(&fixture, "/photos/a.7z/dir/x.png", None).await else {
            panic!("inner file must be readable");
        };
        assert_eq!(body.0, b"pixel");

        let apis::files::ReadFileContentResponse::Status200_FullOrRequestedFileBytes { body, .. } = read(&fixture, "/photos/outer.7z/inner.7z/x.png", None).await else {
            panic!("nested inner file must be readable");
        };
        assert_eq!(body.0, b"inner pixel");
        Ok(())
    }

    #[tokio::test]
    async fn archive_inner_range_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let apis::files::ReadFileContentResponse::Status206_PartialFileBytes { body, content_range, .. } = read(&fixture, "/photos/a.7z/dir/x.png", Some("bytes=1-3")).await else {
            panic!("range request must return 206");
        };
        assert_eq!(body.0, b"ixe");
        assert_eq!(content_range.as_deref(), Some("bytes 1-3/5"));

        assert_eq!(
            error_kind(read(&fixture, "/photos/a.7z/dir/x.png", Some("bytes=10-")).await),
            Some((416, models::ErrorKind::RangeNotSatisfiable))
        );
        Ok(())
    }

    #[tokio::test]
    async fn library_boundary_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        for path in ["/../state/storage/archive/cache.sqlite3", "/photos/../../x", "/C:/Windows"] {
            assert_eq!(error_kind(read(&fixture, path, None).await), Some((400, models::ErrorKind::InvalidRequest)), "{path}");
        }
        Ok(())
    }

    #[tokio::test]
    async fn not_an_archive_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        assert_eq!(
            error_kind(read(&fixture, "/photos/notes.txt/x", None).await),
            Some((400, models::ErrorKind::UnsupportedType))
        );
        Ok(())
    }

    #[tokio::test]
    async fn max_depth_test() -> TestResult {
        let fixture = fixture(ArchiveOptions {
            max_nested_depth: 1,
            ..ArchiveOptions::default()
        })
        .await?;
        assert_eq!(
            error_kind(read(&fixture, "/photos/outer.7z/inner.7z/x.png", None).await),
            Some((400, models::ErrorKind::LimitExceeded))
        );
        Ok(())
    }

    #[tokio::test]
    async fn get_file_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        for (path, archive) in [("/photos/notes.txt", false), ("/photos/a.7z", true), ("/photos/a.7z/dir/x.png", false)] {
            let apis::files::GetFileResponse::Status200_FileMetadata(response) = get_file(&fixture, path).await else {
                panic!("file must be found: {path}");
            };
            assert_eq!(response.file.path, path);
            assert_eq!(response.file.kind, models::FileSystemEntryKind::File, "{path}");
            let Some(Nullable::Present(detail)) = response.file.file else {
                panic!("file detail is missing: {path}");
            };
            assert_eq!(detail.archive, archive, "archive の path は root へ入らず file 自体を返す: {path}");
        }

        let apis::files::GetFileResponse::Status400_InvalidRequest(body) = get_file(&fixture, "/photos").await else {
            panic!("folder must be rejected");
        };
        assert_eq!(body.kind, models::ErrorKind::InvalidRequest);

        let apis::files::GetFileResponse::Status404_ResourceNotFound(body) = get_file(&fixture, "/photos/missing.txt").await else {
            panic!("missing file must be 404");
        };
        assert_eq!(body.kind, models::ErrorKind::NotFound);
        Ok(())
    }
}
