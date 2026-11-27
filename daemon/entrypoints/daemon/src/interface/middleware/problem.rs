use axum::{body::Body, extract::Request, middleware::Next, response::Response};
use http::{
    HeaderValue, Method, StatusCode,
    header::{CONTENT_LENGTH, CONTENT_TYPE},
};
use omnius_pxna_interface::models::ErrorKind;

use crate::error::{ApiError, PROBLEM_CONTENT_TYPE};

/// 生成されたルータと axum が本文なしか平文で返す失敗を、契約の `Problem` に揃える（api.md §11.1）。
pub struct ProblemBody;

impl ProblemBody {
    /// 平文の本文をこの大きさまで `detail` に移す。生成されたルータの拒否の理由は短い。
    const MAX_DETAIL_BYTES: usize = 4096;

    pub async fn middleware(request: Request, next: Next) -> Response {
        // web の成果物の 404 は API の契約の外なので、`/api` の下だけを揃える。HEAD の応答は本文を持たない。
        let applies = request.uri().path().starts_with("/api/") && request.method() != Method::HEAD;
        let response = next.run(request).await;
        if !applies || !Self::needs_problem(&response) {
            return response;
        }

        let status = response.status();
        let (mut parts, body) = response.into_parts();
        let detail = match axum::body::to_bytes(body, Self::MAX_DETAIL_BYTES).await {
            // 5xx の平文は内部の事情を含みうるので、応答へは出さない。
            Ok(bytes) if !status.is_server_error() => Some(String::from_utf8_lossy(&bytes).trim().to_string()).filter(|text| !text.is_empty()),
            _ => None,
        };
        let detail = match (status, detail) {
            (StatusCode::UNAUTHORIZED, None) => Some("Authentication is required.".to_string()),
            (_, detail) => detail,
        };
        let problem = ApiError::problem(status, Self::code(status), detail);
        let (body_parts, body) = ApiError::problem_response(status, &problem).into_parts();
        parts.headers.remove(CONTENT_LENGTH);
        parts.headers.insert(CONTENT_TYPE, HeaderValue::from_static(PROBLEM_CONTENT_TYPE));
        parts.headers.extend(body_parts.headers);
        Response::from_parts(parts, Body::new(body))
    }

    fn needs_problem(response: &Response) -> bool {
        let status = response.status();
        if !(status.is_client_error() || status.is_server_error()) {
            return false;
        }
        response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_none_or(|value| !value.starts_with(PROBLEM_CONTENT_TYPE))
    }

    /// ステータスだけが分かる失敗の `code`。handler が返す失敗は自分の `code` を持つので、ここへは来ない。
    fn code(status: StatusCode) -> ErrorKind {
        match status {
            StatusCode::UNAUTHORIZED => ErrorKind::Unauthorized,
            StatusCode::FORBIDDEN => ErrorKind::Forbidden,
            StatusCode::NOT_FOUND => ErrorKind::NotFound,
            StatusCode::METHOD_NOT_ALLOWED => ErrorKind::MethodNotAllowed,
            StatusCode::CONFLICT => ErrorKind::Conflict,
            StatusCode::PAYLOAD_TOO_LARGE => ErrorKind::PayloadTooLarge,
            StatusCode::RANGE_NOT_SATISFIABLE => ErrorKind::RangeNotSatisfiable,
            StatusCode::GATEWAY_TIMEOUT => ErrorKind::Timeout,
            StatusCode::INSUFFICIENT_STORAGE => ErrorKind::InsufficientStorage,
            status if status.is_server_error() => ErrorKind::InternalServerError,
            _ => ErrorKind::InvalidRequest,
        }
    }
}
