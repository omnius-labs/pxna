use std::sync::Arc;

use axum::{
    extract::{Path, Query, Request, State, rejection::QueryRejection},
    response::{IntoResponse as _, Response},
};
use axum_extra::extract::CookieJar;
use http::{HeaderMap, StatusCode};
use omnius_pxna_interface::models;
use validator::Validate as _;

use crate::{
    error::ApiError,
    interface::{binary_body::BinaryBody, session::SessionAuth},
    prelude::*,
    state::DaemonState,
};

/// `uploads/{upload_id}/content` の書き込みを、生成ルータの外で扱う route。
pub struct UploadContentRoute;

impl UploadContentRoute {
    pub const PATH: &'static str = "/api/v1/libraries/{library_id}/uploads/{upload_id}/content";

    /// バイト列を上限付きで読み、サイズ超過も API の JSON 応答へ揃える。
    pub async fn put(
        State(state): State<Arc<DaemonState>>,
        Path((library_id, upload_id)): Path<(String, String)>,
        cookies: CookieJar,
        headers: HeaderMap,
        query: std::result::Result<Query<models::WriteUploadContentQueryParams>, QueryRejection>,
        request: Request,
    ) -> Response {
        if SessionAuth::authenticate(&state, &cookies, &headers).await.is_none() {
            return ApiError::unauthorized().into_response();
        }
        let result = async {
            let Query(params) = query.map_err(|error| Error::new(ErrorKind::InvalidInput).with_message(error.body_text()))?;
            params.validate().map_err(|error| Error::new(ErrorKind::InvalidInput).with_message(error.to_string()))?;
            let bytes = BinaryBody::read(request, ErrorKind::LimitExceeded, "upload chunks must be at most 32 MiB").await?;
            state.uploads.write(&library_id, &upload_id, params.offset, &bytes).await
        }
        .await;
        match result {
            Ok(()) => StatusCode::NO_CONTENT.into_response(),
            Err(error) => ApiError::from(error).into_response(),
        }
    }
}
