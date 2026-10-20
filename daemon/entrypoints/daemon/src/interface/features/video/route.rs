use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, State},
    response::Response,
};
use axum_extra::extract::CookieJar;
use http::{
    HeaderMap, HeaderValue,
    header::{CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE},
};
use tokio_util::io::ReaderStream;

use crate::{error::ApiError, interface::session::SessionAuth, prelude::*, state::DaemonState};

/// HLS の MIME type と、file の寿命を保持した stream を生成ルータの外で扱う。
pub struct VideoRoute;

impl VideoRoute {
    pub const PLAYLIST: &str = "/api/v1/libraries/{libraryId}/video/sessions/{sessionId}/playlist.m3u8";
    pub const SEGMENT: &str = "/api/v1/libraries/{libraryId}/video/sessions/{sessionId}/segments/{segmentName}";

    pub fn index(name: &str) -> Result<u32> {
        let digits = name
            .strip_suffix(".ts")
            .filter(|digits| !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()))
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput).with_message("invalid segment name"))?;
        digits.parse().map_err(|_| Error::new(ErrorKind::InvalidInput).with_message("invalid segment number"))
    }

    pub async fn playlist(State(state): State<Arc<DaemonState>>, Path((library, id)): Path<(String, String)>, cookies: CookieJar, headers: HeaderMap) -> Response {
        if SessionAuth::authenticate(&state, &cookies, &headers).await.is_none() {
            return ApiError::unauthorized().into_response();
        }
        match state.videos.playlist(&library, &id).await {
            Ok(playlist) => Self::response(Body::from(playlist), "application/vnd.apple.mpegurl", None),
            Err(error) => ApiError::from(error).into_response(),
        }
    }

    pub async fn segment(State(state): State<Arc<DaemonState>>, Path((library, id, name)): Path<(String, String, String)>, cookies: CookieJar, headers: HeaderMap) -> Response {
        if SessionAuth::authenticate(&state, &cookies, &headers).await.is_none() {
            return ApiError::unauthorized().into_response();
        }
        let result = async {
            let index = Self::index(&name)?;
            state.videos.segment(&library, &id, index).await
        }
        .await;
        match result {
            Ok(reader) => {
                let len = reader.len;
                Self::response(Body::from_stream(ReaderStream::with_capacity(reader, 64 * 1024)), "video/mp2t", Some(len))
            }
            Err(error) => ApiError::from(error).into_response(),
        }
    }

    fn response(body: Body, content_type: &'static str, len: Option<u64>) -> Response {
        let mut response = Response::new(body);
        response.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
        response.headers_mut().insert(CACHE_CONTROL, HeaderValue::from_static("private, no-store"));
        if let Some(len) = len {
            response.headers_mut().insert(CONTENT_LENGTH, HeaderValue::from(len));
        }
        response
    }
}
