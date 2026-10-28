use std::sync::Arc;

use axum::{
    Json,
    extract::{State, ws::WebSocketUpgrade, ws::rejection::WebSocketUpgradeRejection},
    response::{IntoResponse as _, Response},
};
use axum_extra::extract::CookieJar;
use http::{HeaderMap, StatusCode};

use omnius_pxna_interface::models;

use super::connection::Connection;
use crate::{
    error::ApiError,
    interface::{middleware::OriginCheck, session::SessionAuth},
    prelude::*,
    state::DaemonState,
};

/// image socket の route。WebSocket の upgrade は生成されたルータを通せないので、手書きの route で受ける（design.md §4.2）。
pub struct ImageSocketRoute;

impl ImageSocketRoute {
    pub const PATH: &'static str = "/api/v1/sockets/image";

    /// 認証と `Origin` と socket の数を確かめてから、接続を upgrade する（Image Socket Design §3）。
    pub async fn get(
        State(state): State<Arc<DaemonState>>,
        cookies: CookieJar,
        headers: HeaderMap,
        upgrade: std::result::Result<WebSocketUpgrade, WebSocketUpgradeRejection>,
    ) -> Response {
        let Some((credential, claims)) = SessionAuth::authenticate_with_token(&state, &cookies, &headers).await else {
            return ApiError::unauthorized().into_response();
        };

        // upgrade は `GET` なので、変更系の要求を守る `Origin` の検証には掛からない。cookie が付いて届く以上、ここで同じ検証をする。
        if !OriginCheck::allows_upgrade(&headers) {
            let mut body = models::ErrorResponse::new(models::ErrorKind::Forbidden);
            body.message = Some("origin does not match host".to_string());
            return (StatusCode::FORBIDDEN, Json(body)).into_response();
        }

        let upgrade = match upgrade {
            Ok(upgrade) => upgrade,
            Err(rejection) => return ApiError::from(Error::new(ErrorKind::InvalidInput).with_message(rejection.body_text())).into_response(),
        };

        let Some(slot) = state.image_sockets.open(&claims.session.id) else {
            let mut body = models::ErrorResponse::new(models::ErrorKind::LimitExceeded);
            body.message = Some("too many open image sockets".to_string());
            return (StatusCode::TOO_MANY_REQUESTS, Json(body)).into_response();
        };

        // 契約を超える大きさの message は、読み込む前に tungstenite が拒む。上限ちょうどの超過は接続側が 4400 で閉じる。
        let max_message = state.image_sockets.limits().max_text_bytes.saturating_mul(4);
        upgrade
            .max_message_size(max_message)
            .max_frame_size(max_message)
            .on_upgrade(move |socket| Connection::new(state, credential, slot).run(socket))
    }
}
