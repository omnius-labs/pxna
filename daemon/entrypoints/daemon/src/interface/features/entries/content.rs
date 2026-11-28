use omnius_pxna_engine::service::WorkClass;
use std::sync::Arc;

use axum::{
    Json,
    body::Body,
    extract::{Path, Query, Request, State, rejection::QueryRejection},
    response::{IntoResponse as _, Response},
};
use axum_extra::extract::CookieJar;
use http::{
    HeaderMap, HeaderValue, StatusCode,
    header::{ACCEPT_RANGES, CACHE_CONTROL, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, ETAG, RANGE},
};
use tokio::io::{AsyncReadExt as _, AsyncSeekExt as _};
use tokio_util::io::ReaderStream;
use validator::Validate as _;

use omnius_pxna_engine::{
    model::FsEntryDetail,
    service::{ArchiveIdentity, ConflictPolicy, MediaKind},
};
use omnius_pxna_interface::models;

use crate::{
    error::ApiError,
    interface::{binary_body::BinaryBody, session::SessionAuth},
    prelude::*,
    service::{library::ReadRange, mutation::LibraryMutation},
    state::DaemonState,
};

/// `entries/content` の読み出しと書き込み、`entries/files` の作成を、生成ルータの外で扱う route。
pub struct FileContentRoute;

impl FileContentRoute {
    pub const PATH: &'static str = "/api/v1/libraries/{library_id}/entries/content";
    pub const CREATE_PATH: &'static str = "/api/v1/libraries/{library_id}/entries/files";

    /// file handle と archive lease を応答が読み終わるまで保持する content の配信。
    pub async fn get(
        State(state): State<Arc<DaemonState>>,
        Path(library_id): Path<String>,
        cookies: CookieJar,
        headers: HeaderMap,
        query: std::result::Result<Query<models::ReadFileContentQueryParams>, QueryRejection>,
    ) -> Response {
        if SessionAuth::authenticate(&state, &cookies, &headers).await.is_none() {
            return ApiError::unauthorized().into_response();
        }
        let result = async {
            let Query(query) = query.map_err(|error| Error::new(ErrorKind::InvalidInput).with_message(error.body_text()))?;
            query.validate().map_err(|error| Error::new(ErrorKind::InvalidInput).with_message(error.to_string()))?;
            let range = ReadRange::parse(headers.get(RANGE).and_then(|value| value.to_str().ok()), query.offset, query.limit)?;
            let source = state.library.thumbnail_source(&library_id, &query.path).await?;
            if !matches!(source.get_entry(WorkClass::Foreground).await?.detail, FsEntryDetail::File { .. }) {
                return Err(Error::new(ErrorKind::InvalidInput).with_message("not a file"));
            }
            let mut file = source.open_file(WorkClass::Foreground).await?;
            let fingerprint = file.fingerprint().await?;
            let len = fingerprint.size;
            let (start, end) = match range.resolve(len) {
                Ok(range) => range,
                Err(error) => {
                    let mut response = ApiError::from(error).into_response();
                    response.headers_mut().insert(
                        CONTENT_RANGE,
                        HeaderValue::from_str(&format!("bytes */{len}")).map_err(|_| Error::new(ErrorKind::UnexpectedError))?,
                    );
                    return Ok(response);
                }
            };
            let key = source
                .entry_cache_key()
                .map(|key| key.to_hex())
                .unwrap_or_else(|| ArchiveIdentity::from_local(source.path().as_str(), &fingerprint).to_hex());
            file.seek(std::io::SeekFrom::Start(start)).await?;
            let mut response = Response::new(Body::from_stream(ReaderStream::with_capacity(file.take(end - start), 64 * 1024)));
            *response.status_mut() = if range.is_partial() { StatusCode::PARTIAL_CONTENT } else { StatusCode::OK };
            let headers = response.headers_mut();
            headers.insert(ACCEPT_RANGES, HeaderValue::from_static("bytes"));
            headers.insert(CONTENT_LENGTH, HeaderValue::from(end - start));
            headers.insert(CACHE_CONTROL, HeaderValue::from_static("private, no-cache"));
            headers.insert(
                CONTENT_TYPE,
                HeaderValue::from_static(MediaKind::content_type(&query.path).unwrap_or("application/octet-stream")),
            );
            headers.insert(ETAG, HeaderValue::from_str(&format!("\"{key}\"")).map_err(|_| Error::new(ErrorKind::UnexpectedError))?);
            if range.is_partial() {
                headers.insert(
                    CONTENT_RANGE,
                    HeaderValue::from_str(&format!("bytes {start}-{}/{len}", end - 1)).map_err(|_| Error::new(ErrorKind::UnexpectedError))?,
                );
            }
            Ok::<_, Error>(response)
        }
        .await;
        match result {
            Ok(response) => response,
            Err(error) => ApiError::from(error).into_response(),
        }
    }

    /// バイト列を上限付きで読み、サイズ超過も API の JSON 応答へ揃える。
    pub async fn put(
        State(state): State<Arc<DaemonState>>,
        Path(library_id): Path<String>,
        cookies: CookieJar,
        headers: HeaderMap,
        query: std::result::Result<Query<models::WriteFileContentQueryParams>, QueryRejection>,
        request: Request,
    ) -> Response {
        if SessionAuth::authenticate(&state, &cookies, &headers).await.is_none() {
            return ApiError::unauthorized().into_response();
        }
        let result = async {
            let Query(params) = query.map_err(|error| Error::new(ErrorKind::InvalidInput).with_message(error.body_text()))?;
            params.validate().map_err(|error| Error::new(ErrorKind::InvalidInput).with_message(error.to_string()))?;
            let bytes = BinaryBody::read(request, ErrorKind::PayloadTooLarge, "use an upload session for files larger than 32 MiB").await?;
            state
                .mutation
                .write(&library_id, &params.path, &bytes, LibraryMutation::write_conflict(params.conflict))
                .await
        }
        .await;
        Self::written(result)
    }

    /// `put` と同じく書き込むが、`path` に entry があれば daemon が選ぶ別の名前で公開する。
    pub async fn post(
        State(state): State<Arc<DaemonState>>,
        Path(library_id): Path<String>,
        cookies: CookieJar,
        headers: HeaderMap,
        query: std::result::Result<Query<models::CreateFileQueryParams>, QueryRejection>,
        request: Request,
    ) -> Response {
        if SessionAuth::authenticate(&state, &cookies, &headers).await.is_none() {
            return ApiError::unauthorized().into_response();
        }
        let result = async {
            let Query(params) = query.map_err(|error| Error::new(ErrorKind::InvalidInput).with_message(error.body_text()))?;
            params.validate().map_err(|error| Error::new(ErrorKind::InvalidInput).with_message(error.to_string()))?;
            let bytes = BinaryBody::read(request, ErrorKind::PayloadTooLarge, "use an upload session for files larger than 32 MiB").await?;
            state.mutation.write(&library_id, &params.path, &bytes, ConflictPolicy::Rename).await
        }
        .await;
        Self::written(result)
    }

    fn written(result: Result<models::FileSystemEntry>) -> Response {
        match result {
            Ok(entry) => (StatusCode::CREATED, Json(models::EntryResponse::new(entry))).into_response(),
            Err(error) => ApiError::from(error).into_response(),
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

    use http::Method;
    use omnius_pxna_interface::apis;

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
            .uri("/api/v1/libraries/photos/entries/content?path=/large.bin")
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
            .uri("/api/v1/libraries/photos/entries/content?path=/large.bin")
            .header(http::header::HOST, "localhost")
            .header(http::header::AUTHORIZATION, &bearer)
            .header(http::header::RANGE, "bytes=0-")
            .body(Body::empty())?;
        let range = router.clone().oneshot(range).await?;
        assert_eq!(range.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(range.headers()[http::header::CONTENT_LENGTH], large_len.to_string(), "Range を旧 128 MiB 上限で切らない");
        drop(range);
        std::fs::write(root.join("empty.bin"), b"")?;
        let empty = send(&router, Method::GET, "/api/v1/libraries/photos/entries/content?path=/empty.bin", headers, None).await?;
        assert_eq!(empty.status, StatusCode::OK);
        assert_eq!(empty.header("content-length"), Some("0"));
        let unsatisfied = send(
            &router,
            Method::GET,
            "/api/v1/libraries/photos/entries/content?path=/empty.bin",
            &[("authorization", bearer.as_str()), ("range", "bytes=0-")],
            None,
        )
        .await?;
        assert_eq!(unsatisfied.status, StatusCode::RANGE_NOT_SATISFIABLE);
        assert_eq!(unsatisfied.header("content-range"), Some("bytes */0"));
        let uri = "/api/v1/libraries/photos/entries/content?path=/photos/notes.txt";
        let original = send(&router, Method::GET, uri, headers, None).await?;
        let repeated = send(&router, Method::GET, uri, headers, None).await?;
        assert!(original.header("etag").is_some());
        assert_eq!(original.header("etag"), repeated.header("etag"));
        std::fs::write(root.join("photos/notes.txt"), b"changed content")?;
        let changed = send(&router, Method::GET, uri, headers, None).await?;
        assert_ne!(original.header("etag"), changed.header("etag"));
        Ok(())
    }

    async fn read(fixture: &Fixture, path: &str, range: Option<&str>) -> apis::entries::ReadFileContentResponse {
        let header = models::ReadFileContentHeaderParams { range: range.map(str::to_string) };
        let params = models::ReadFileContentPathParams { library_id: "photos".to_string() };
        let query = models::ReadFileContentQueryParams {
            path: path.to_string(),
            offset: None,
            limit: None,
        };
        apis::entries::Entries::read_file_content(&fixture.server, &Method::GET, &host(), &CookieJar::default(), &claims(), &header, &params, &query)
            .await
            .unwrap()
    }

    async fn get_entry(fixture: &Fixture, path: &str, view: Option<models::EntryView>) -> apis::entries::GetEntryMetadataResponse {
        let params = models::GetEntryMetadataPathParams { library_id: "photos".to_string() };
        let query = models::GetEntryMetadataQueryParams { path: path.to_string(), view };
        apis::entries::Entries::get_entry_metadata(&fixture.server, &Method::GET, &host(), &CookieJar::default(), &claims(), &params, &query)
            .await
            .unwrap()
    }

    fn error_kind(response: apis::entries::ReadFileContentResponse) -> Option<(u16, models::ErrorKind)> {
        use apis::entries::ReadFileContentResponse as R;
        match response {
            R::Status400_InvalidRequest(body) => Some((400, body.code)),
            R::Status403_TheRequestedResourceCannotBeAccessedOrWritten(body) => Some((403, body.code)),
            R::Status404_ResourceNotFound(body) => Some((404, body.code)),
            R::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(body) => Some((409, body.code)),
            R::Status416_RequestedByteRangeCannotBeSatisfied(body) => Some((416, body.code)),
            R::Status500_InternalServerError(body) => Some((500, body.code)),
            _ => None,
        }
    }

    #[tokio::test]
    async fn archive_file_itself_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let apis::entries::ReadFileContentResponse::Status200_FullOrRequestedFileBytes { body, .. } = read(&fixture, "/photos/a.7z", None).await else {
            panic!("file endpoint must return the archive bytes");
        };
        assert_eq!(body.0, std::fs::read(&fixture.archive)?);
        Ok(())
    }

    #[tokio::test]
    async fn archive_inner_content_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let apis::entries::ReadFileContentResponse::Status200_FullOrRequestedFileBytes { body, .. } = read(&fixture, "/photos/a.7z/dir/x.png", None).await else {
            panic!("inner file must be readable");
        };
        assert_eq!(body.0, b"pixel");

        let apis::entries::ReadFileContentResponse::Status200_FullOrRequestedFileBytes { body, .. } = read(&fixture, "/photos/outer.7z/inner.7z/x.png", None).await else {
            panic!("nested inner file must be readable");
        };
        assert_eq!(body.0, b"inner pixel");
        Ok(())
    }

    #[tokio::test]
    async fn archive_inner_range_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let apis::entries::ReadFileContentResponse::Status206_PartialFileBytes { body, content_range, .. } = read(&fixture, "/photos/a.7z/dir/x.png", Some("bytes=1-3")).await
        else {
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
    async fn create_publishes_under_another_name_and_a_write_refuses_rename_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let router = router(&fixture);
        let setup = send(&router, Method::POST, "/api/v1/setup", &[], credentials("owner", "password1", "bearer")).await?;
        let bearer = bearer(&setup)?;
        let uri = "/api/v1/libraries/photos/entries/files?path=/photos/notes.txt";

        let created = binary_with(&router, Method::POST, uri, &bearer, b"new".to_vec()).await?;
        assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
        let published = created.body["entry"]["path"].as_str().ok_or("path is missing")?;
        assert_ne!(published, "/photos/notes.txt", "an existing entry is kept and the file gets another name");
        let root = &fixture.server.state.conf.libraries[0].path;
        assert_eq!(std::fs::read(root.join(published.trim_start_matches('/')))?, b"new");

        let fresh = binary_with(
            &router,
            Method::POST,
            "/api/v1/libraries/photos/entries/files?path=/photos/fresh.txt",
            &bearer,
            b"f".to_vec(),
        )
        .await?;
        assert_eq!(fresh.body["entry"]["path"], "/photos/fresh.txt", "a free path is used as it is");

        let rename = binary(
            &router,
            "/api/v1/libraries/photos/entries/content?path=/photos/notes.txt&conflict=rename",
            &bearer,
            b"x".to_vec(),
        )
        .await?;
        assert_eq!(rename.status, StatusCode::BAD_REQUEST, "an exact write does not take rename");
        assert_eq!(rename.body["code"], "InvalidRequest");
        Ok(())
    }

    #[tokio::test]
    async fn metadata_reads_the_face_the_view_names_test() -> TestResult {
        use models::EntryView::{File, Folder};
        let fixture = fixture(ArchiveOptions::default()).await?;
        for (path, view, archive) in [
            ("/photos/notes.txt", None, false),
            ("/photos/notes.txt", Some(File), false),
            ("/photos/a.7z", None, true),
            ("/photos/a.7z", Some(File), true),
            ("/photos/a.7z/dir/x.png", None, false),
        ] {
            let apis::entries::GetEntryMetadataResponse::Status200_TheEntry(response) = get_entry(&fixture, path, view).await else {
                panic!("file must be found: {path} {view:?}");
            };
            assert_eq!(response.entry.path, path);
            assert_eq!(response.entry.kind, models::FileSystemEntryKind::File, "{path} {view:?}");
            let Some(Nullable::Present(detail)) = response.entry.file else {
                panic!("file detail is missing: {path}");
            };
            assert_eq!(detail.archive, archive, "an omitted view keeps an archive its file: {path}");
        }
        for (path, view) in [("/photos", None), ("/photos", Some(Folder)), ("/photos/a.7z", Some(Folder))] {
            let apis::entries::GetEntryMetadataResponse::Status200_TheEntry(response) = get_entry(&fixture, path, view).await else {
                panic!("folder must be found: {path} {view:?}");
            };
            assert_eq!(response.entry.path, path);
            assert_eq!(response.entry.kind, models::FileSystemEntryKind::Folder, "{path} {view:?}");
        }

        for (path, view) in [("/photos", Some(File)), ("/photos/notes.txt", Some(Folder))] {
            let apis::entries::GetEntryMetadataResponse::Status400_InvalidRequest(body) = get_entry(&fixture, path, view).await else {
                panic!("a face the path does not have must be rejected: {path} {view:?}");
            };
            assert_eq!(body.code, models::ErrorKind::InvalidRequest);
        }

        let apis::entries::GetEntryMetadataResponse::Status404_ResourceNotFound(body) = get_entry(&fixture, "/photos/missing.txt", None).await else {
            panic!("missing file must be 404");
        };
        assert_eq!(body.code, models::ErrorKind::NotFound);
        Ok(())
    }
}
