use axum::{body::Body, extract::Request, middleware::Next, response::Response};
use http::{
    StatusCode,
    header::{CONTENT_LENGTH, CONTENT_TYPE},
};

use crate::error::ApiError;

/// 生成ルータが本文なしで返す 401 を API 契約の応答に揃える。
pub struct UnauthorizedBody;

impl UnauthorizedBody {
    pub async fn middleware(request: Request, next: Next) -> Response {
        let response = next.run(request).await;
        if response.status() != StatusCode::UNAUTHORIZED || response.headers().contains_key(CONTENT_TYPE) {
            return response;
        }
        let (mut parts, _) = response.into_parts();
        let (body_parts, body) = ApiError::unauthorized().into_response().into_parts();
        parts.headers.remove(CONTENT_LENGTH);
        parts.headers.extend(body_parts.headers);
        Response::from_parts(parts, Body::new(body))
    }
}
