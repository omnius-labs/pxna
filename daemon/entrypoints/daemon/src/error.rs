use std::backtrace::Backtrace;

use omnius_core_base::error::OmniError;
use omnius_core_rocketpack::{RocketPackDecoderError, RocketPackEncoderError};

pub struct Error {
    kind: ErrorKind,
    message: Option<String>,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
    backtrace: Backtrace,
}

impl OmniError for Error {
    type ErrorKind = ErrorKind;

    fn new(kind: Self::ErrorKind) -> Self {
        Self {
            kind,
            message: None,
            source: None,
            backtrace: Backtrace::capture(),
        }
    }

    fn from_error<E: Into<Box<dyn std::error::Error + Send + Sync>>>(source: E, kind: Self::ErrorKind) -> Self {
        Self {
            kind,
            message: None,
            source: Some(source.into()),
            backtrace: Backtrace::capture(),
        }
    }

    fn kind(&self) -> &Self::ErrorKind {
        &self.kind
    }

    fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    fn backtrace(&self) -> &Backtrace {
        &self.backtrace
    }

    fn with_message<S: Into<String>>(mut self, message: S) -> Self {
        self.message = Some(message.into());
        self
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|s| &**s as &(dyn std::error::Error + 'static))
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        OmniError::fmt(self, f)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        OmniError::fmt(self, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    Unknown,
    IoError,
    TimeError,
    SerdeError,
    DatabaseError,
    HttpClientError,
    CryptoError,
    UpnpError,
    NetworkError,
    ImageError,
    TaskError,
    UnexpectedError,

    InvalidInput,
    InvalidFormat,
    EndOfStream,
    UnsupportedVersion,
    UnsupportedType,
    NotFound,
    AlreadyExists,
    AlreadyConnected,
    NotConnected,

    ReadOnly,
    PasswordRequired,
    LimitExceeded,
    Timeout,
    Cancelled,
    SourceChanged,
    RangeNotSatisfiable,
    PayloadTooLarge,
    Unauthenticated,
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::Unknown => write!(fmt, "unknown"),
            ErrorKind::IoError => write!(fmt, "io error"),
            ErrorKind::TimeError => write!(fmt, "time conversion error"),
            ErrorKind::SerdeError => write!(fmt, "serde error"),
            ErrorKind::DatabaseError => write!(fmt, "database error"),
            ErrorKind::HttpClientError => write!(fmt, "http client error"),
            ErrorKind::CryptoError => write!(fmt, "crypto error"),
            ErrorKind::UpnpError => write!(fmt, "upnp error"),
            ErrorKind::NetworkError => write!(fmt, "network error"),
            ErrorKind::ImageError => write!(fmt, "image error"),
            ErrorKind::TaskError => write!(fmt, "task error"),
            ErrorKind::UnexpectedError => write!(fmt, "unexpected error"),

            ErrorKind::InvalidInput => write!(fmt, "invalid input"),
            ErrorKind::InvalidFormat => write!(fmt, "invalid format"),
            ErrorKind::EndOfStream => write!(fmt, "end of stream"),
            ErrorKind::UnsupportedVersion => write!(fmt, "unsupported version"),
            ErrorKind::UnsupportedType => write!(fmt, "unsupported type"),
            ErrorKind::NotFound => write!(fmt, "not found"),
            ErrorKind::AlreadyExists => write!(fmt, "already exists"),
            ErrorKind::AlreadyConnected => write!(fmt, "already_connected"),
            ErrorKind::NotConnected => write!(fmt, "not_connected"),

            ErrorKind::ReadOnly => write!(fmt, "read only"),
            ErrorKind::PasswordRequired => write!(fmt, "password required"),
            ErrorKind::LimitExceeded => write!(fmt, "limit exceeded"),
            ErrorKind::Timeout => write!(fmt, "timeout"),
            ErrorKind::Cancelled => write!(fmt, "cancelled"),
            ErrorKind::SourceChanged => write!(fmt, "source changed"),
            ErrorKind::RangeNotSatisfiable => write!(fmt, "range not satisfiable"),
            ErrorKind::PayloadTooLarge => write!(fmt, "payload too large"),
            ErrorKind::Unauthenticated => write!(fmt, "unauthenticated"),
        }
    }
}

impl From<std::convert::Infallible> for Error {
    fn from(_: std::convert::Infallible) -> Self {
        Error::new(ErrorKind::Unknown)
    }
}

impl From<std::array::TryFromSliceError> for Error {
    fn from(e: std::array::TryFromSliceError) -> Self {
        Error::from_error(e, ErrorKind::InvalidFormat).with_message("failed to convert slice to array")
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::from_error(e, ErrorKind::IoError).with_message("io error")
    }
}

impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        Error::from_error(e, ErrorKind::DatabaseError).with_message("Database operation failed")
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(e: std::num::ParseIntError) -> Error {
        Error::from_error(e, ErrorKind::InvalidFormat).with_message("int parse error")
    }
}

impl From<std::net::AddrParseError> for Error {
    fn from(e: std::net::AddrParseError) -> Error {
        Error::from_error(e, ErrorKind::InvalidFormat).with_message("addr parse error")
    }
}

impl From<hex::FromHexError> for Error {
    fn from(e: hex::FromHexError) -> Self {
        Error::from_error(e, ErrorKind::InvalidFormat).with_message("hex decode error")
    }
}

impl From<base64::DecodeError> for Error {
    fn from(e: base64::DecodeError) -> Self {
        Error::from_error(e, ErrorKind::InvalidFormat).with_message("base64 decode error")
    }
}

impl From<std::num::TryFromIntError> for Error {
    fn from(e: std::num::TryFromIntError) -> Self {
        Error::from_error(e, ErrorKind::InvalidFormat).with_message("Integer conversion error")
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::from_error(e, ErrorKind::HttpClientError).with_message("HTTP request failed")
    }
}

impl From<toml::de::Error> for Error {
    fn from(e: toml::de::Error) -> Error {
        Error::from_error(e, ErrorKind::SerdeError).with_message("failed to parse toml file")
    }
}

impl From<RocketPackEncoderError> for Error {
    fn from(e: RocketPackEncoderError) -> Self {
        Error::from_error(e, ErrorKind::SerdeError).with_message("failed to parse toml file")
    }
}

impl From<RocketPackDecoderError> for Error {
    fn from(e: RocketPackDecoderError) -> Self {
        Error::from_error(e, ErrorKind::SerdeError).with_message("failed to parse toml file")
    }
}

impl From<omnius_core_migration::Error> for Error {
    fn from(e: omnius_core_migration::Error) -> Self {
        match e.kind() {
            omnius_core_migration::ErrorKind::Unknown => Error::from_error(e, ErrorKind::Unknown),
            omnius_core_migration::ErrorKind::IoError => Error::from_error(e, ErrorKind::IoError),
            omnius_core_migration::ErrorKind::DatabaseError => Error::from_error(e, ErrorKind::DatabaseError),
            omnius_core_migration::ErrorKind::InvalidFormat => Error::from_error(e, ErrorKind::InvalidFormat),
        }
    }
}

impl From<omnius_pxna_engine::Error> for Error {
    fn from(e: omnius_pxna_engine::Error) -> Self {
        match e.kind() {
            omnius_pxna_engine::ErrorKind::Unknown => Error::from_error(e, ErrorKind::Unknown),
            omnius_pxna_engine::ErrorKind::SerdeError => Error::from_error(e, ErrorKind::SerdeError),
            omnius_pxna_engine::ErrorKind::IoError => Error::from_error(e, ErrorKind::IoError),
            omnius_pxna_engine::ErrorKind::TimeError => Error::from_error(e, ErrorKind::TimeError),
            omnius_pxna_engine::ErrorKind::DatabaseError => Error::from_error(e, ErrorKind::DatabaseError),
            omnius_pxna_engine::ErrorKind::HttpClientError => Error::from_error(e, ErrorKind::HttpClientError),
            omnius_pxna_engine::ErrorKind::CryptoError => Error::from_error(e, ErrorKind::CryptoError),
            omnius_pxna_engine::ErrorKind::UpnpError => Error::from_error(e, ErrorKind::UpnpError),
            omnius_pxna_engine::ErrorKind::NetworkError => Error::from_error(e, ErrorKind::NetworkError),
            omnius_pxna_engine::ErrorKind::ImageError => Error::from_error(e, ErrorKind::ImageError),
            omnius_pxna_engine::ErrorKind::TaskError => Error::from_error(e, ErrorKind::TaskError),
            omnius_pxna_engine::ErrorKind::UnexpectedError => Error::from_error(e, ErrorKind::UnexpectedError),

            omnius_pxna_engine::ErrorKind::InvalidInput => Error::from_error(e, ErrorKind::InvalidInput),
            omnius_pxna_engine::ErrorKind::InvalidFormat => Error::from_error(e, ErrorKind::InvalidFormat),
            omnius_pxna_engine::ErrorKind::EndOfStream => Error::from_error(e, ErrorKind::EndOfStream),
            omnius_pxna_engine::ErrorKind::UnsupportedType => Error::from_error(e, ErrorKind::UnsupportedType),
            omnius_pxna_engine::ErrorKind::NotFound => Error::from_error(e, ErrorKind::NotFound),
            omnius_pxna_engine::ErrorKind::AlreadyExists => Error::from_error(e, ErrorKind::AlreadyExists),
            omnius_pxna_engine::ErrorKind::AlreadyConnected => Error::from_error(e, ErrorKind::AlreadyConnected),
            omnius_pxna_engine::ErrorKind::NotConnected => Error::from_error(e, ErrorKind::NotConnected),

            omnius_pxna_engine::ErrorKind::ReadOnly => Error::from_error(e, ErrorKind::ReadOnly),
            omnius_pxna_engine::ErrorKind::PasswordRequired => Error::from_error(e, ErrorKind::PasswordRequired),
            omnius_pxna_engine::ErrorKind::LimitExceeded => Error::from_error(e, ErrorKind::LimitExceeded),
            omnius_pxna_engine::ErrorKind::Timeout => Error::from_error(e, ErrorKind::Timeout),
            omnius_pxna_engine::ErrorKind::Cancelled => Error::from_error(e, ErrorKind::Cancelled),
            omnius_pxna_engine::ErrorKind::SourceChanged => Error::from_error(e, ErrorKind::SourceChanged),
            omnius_pxna_engine::ErrorKind::Unauthenticated => Error::from_error(e, ErrorKind::Unauthenticated),
        }
    }
}

/// API 応答の HTTP ステータスの区分。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiStatus {
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    RangeNotSatisfiable,
    GatewayTimeout,
    InternalServerError,
    PayloadTooLarge,
    InsufficientStorage,
}

/// daemon の `Error` を、API の `ErrorKind` と HTTP ステータスへ写した値。
///
/// 内部の分類をそのまま外へ出さず、design.md §4.4 の対応表に沿って利用者可視の種別へまとめる。
#[derive(Debug, Clone, PartialEq)]
pub struct ApiError {
    pub status: ApiStatus,
    pub body: omnius_pxna_interface::models::ErrorResponse,
}

impl ApiError {
    pub fn from_error(error: &Error) -> Self {
        use omnius_pxna_interface::models::ErrorKind as ApiErrorKind;

        let (status, kind) = match error.kind() {
            ErrorKind::PayloadTooLarge => (ApiStatus::PayloadTooLarge, ApiErrorKind::PayloadTooLarge),
            ErrorKind::IoError if Self::disk_full(error) => (ApiStatus::InsufficientStorage, ApiErrorKind::InsufficientStorage),
            ErrorKind::InvalidInput => (ApiStatus::BadRequest, ApiErrorKind::InvalidRequest),
            // 壊れた archive は、開けない archive と同じく対象 file についての事実として返す。
            ErrorKind::UnsupportedType | ErrorKind::InvalidFormat => (ApiStatus::BadRequest, ApiErrorKind::UnsupportedType),
            ErrorKind::LimitExceeded => (ApiStatus::BadRequest, ApiErrorKind::LimitExceeded),
            ErrorKind::PasswordRequired => (ApiStatus::Forbidden, ApiErrorKind::PasswordRequired),
            ErrorKind::ReadOnly => (ApiStatus::Forbidden, ApiErrorKind::Forbidden),
            ErrorKind::Unauthenticated => (ApiStatus::Unauthorized, ApiErrorKind::Unauthorized),
            ErrorKind::NotFound => (ApiStatus::NotFound, ApiErrorKind::NotFound),
            ErrorKind::SourceChanged | ErrorKind::AlreadyExists => (ApiStatus::Conflict, ApiErrorKind::Conflict),
            ErrorKind::RangeNotSatisfiable => (ApiStatus::RangeNotSatisfiable, ApiErrorKind::RangeNotSatisfiable),
            // archive の展開か ffmpeg による生成が期限を過ぎた。Cancelled は応答を受け取る相手がいないので 500 のままにする。
            ErrorKind::Timeout => (ApiStatus::GatewayTimeout, ApiErrorKind::Timeout),
            _ => (ApiStatus::InternalServerError, ApiErrorKind::InternalServerError),
        };

        let mut body = omnius_pxna_interface::models::ErrorResponse::new(kind);
        if status == ApiStatus::InternalServerError {
            // 内部の詳細は log にだけ残し、応答へは出さない。
            tracing::error!(error = ?error, "request failed");
        } else {
            body.message = error.message().map(str::to_string);
        }

        Self { status, body }
    }
}

impl ApiError {
    fn disk_full(error: &Error) -> bool {
        let mut source = std::error::Error::source(error);
        while let Some(cause) = source {
            if let Some(io) = cause.downcast_ref::<std::io::Error>()
                && io.kind() == std::io::ErrorKind::StorageFull
            {
                return true;
            }
            source = cause.source();
        }
        false
    }

    pub fn status_code(&self) -> http::StatusCode {
        match self.status {
            ApiStatus::BadRequest => http::StatusCode::BAD_REQUEST,
            ApiStatus::Unauthorized => http::StatusCode::UNAUTHORIZED,
            ApiStatus::Forbidden => http::StatusCode::FORBIDDEN,
            ApiStatus::NotFound => http::StatusCode::NOT_FOUND,
            ApiStatus::Conflict => http::StatusCode::CONFLICT,
            ApiStatus::RangeNotSatisfiable => http::StatusCode::RANGE_NOT_SATISFIABLE,
            ApiStatus::GatewayTimeout => http::StatusCode::GATEWAY_TIMEOUT,
            ApiStatus::InternalServerError => http::StatusCode::INTERNAL_SERVER_ERROR,
            ApiStatus::PayloadTooLarge => http::StatusCode::PAYLOAD_TOO_LARGE,
            ApiStatus::InsufficientStorage => http::StatusCode::INSUFFICIENT_STORAGE,
        }
    }

    /// 契約の `ErrorResponse` を本文に持つ応答にする。生成されたルータの外の route が使う。
    pub fn into_response(self) -> axum::response::Response {
        use axum::response::IntoResponse as _;
        (self.status_code(), axum::Json(self.body)).into_response()
    }

    /// 認証が要る route で、セッションを取り出せなかったときの応答。
    pub fn unauthorized() -> Self {
        let mut body = omnius_pxna_interface::models::ErrorResponse::new(omnius_pxna_interface::models::ErrorKind::Unauthorized);
        body.message = Some("Authentication is required.".to_string());
        Self {
            status: ApiStatus::Unauthorized,
            body,
        }
    }
}

impl From<Error> for ApiError {
    fn from(error: Error) -> Self {
        Self::from_error(&error)
    }
}

#[cfg(test)]
mod tests {
    use omnius_pxna_interface::models::ErrorKind as ApiErrorKind;

    use super::*;

    #[test]
    fn engine_errors_map_to_api_status_test() {
        let cases = [
            (omnius_pxna_engine::ErrorKind::UnsupportedType, ApiStatus::BadRequest, ApiErrorKind::UnsupportedType),
            (omnius_pxna_engine::ErrorKind::LimitExceeded, ApiStatus::BadRequest, ApiErrorKind::LimitExceeded),
            (omnius_pxna_engine::ErrorKind::PasswordRequired, ApiStatus::Forbidden, ApiErrorKind::PasswordRequired),
            (omnius_pxna_engine::ErrorKind::ReadOnly, ApiStatus::Forbidden, ApiErrorKind::Forbidden),
            (omnius_pxna_engine::ErrorKind::SourceChanged, ApiStatus::Conflict, ApiErrorKind::Conflict),
            (omnius_pxna_engine::ErrorKind::Unauthenticated, ApiStatus::Unauthorized, ApiErrorKind::Unauthorized),
            (omnius_pxna_engine::ErrorKind::Timeout, ApiStatus::GatewayTimeout, ApiErrorKind::Timeout),
            (omnius_pxna_engine::ErrorKind::Cancelled, ApiStatus::InternalServerError, ApiErrorKind::InternalServerError),
        ];
        for (engine_kind, status, api_kind) in cases {
            let error: Error = omnius_pxna_engine::Error::new(engine_kind.clone()).with_message("detail").into();
            let api = ApiError::from(error);
            assert_eq!(api.status, status, "{engine_kind:?}");
            assert_eq!(api.body.kind, api_kind, "{engine_kind:?}");
        }
    }

    #[tokio::test]
    async fn timeout_response_is_gateway_timeout_test() -> testresult::TestResult {
        let error: Error = omnius_pxna_engine::Error::new(omnius_pxna_engine::ErrorKind::Timeout)
            .with_message("extract timed out")
            .into();
        let response = ApiError::from(error).into_response();
        assert_eq!(response.status(), http::StatusCode::GATEWAY_TIMEOUT);

        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
        let body: serde_json::Value = serde_json::from_slice(&bytes)?;
        assert_eq!(body["kind"], "Timeout");
        Ok(())
    }

    #[test]
    fn internal_error_does_not_leak_message_test() {
        let error: Error = omnius_pxna_engine::Error::new(omnius_pxna_engine::ErrorKind::IoError)
            .with_message("/host/secret/path")
            .into();
        let api = ApiError::from(error);
        assert_eq!(api.status, ApiStatus::InternalServerError);
        assert_eq!(api.body.message, None);
    }
}
