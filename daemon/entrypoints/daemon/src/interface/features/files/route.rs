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
    service::{ArchiveIdentity, MediaKind},
};
use omnius_pxna_interface::models;

use crate::{
    error::ApiError,
    interface::{binary_body::BinaryBody, session::SessionAuth},
    prelude::*,
    service::{library::ReadRange, mutation::LibraryMutation},
    state::DaemonState,
};

/// `files/content` の読み出しと書き込みを、生成ルータの外で扱う route。
pub struct FileContentRoute;

impl FileContentRoute {
    pub const PATH: &'static str = "/api/v1/libraries/{library_id}/files/content";

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
            state.mutation.write(&library_id, &params.path, &bytes, LibraryMutation::conflict(params.conflict)).await
        }
        .await;
        match result {
            Ok(file) => (StatusCode::CREATED, Json(models::FileResponse::new(file))).into_response(),
            Err(error) => ApiError::from(error).into_response(),
        }
    }
}
