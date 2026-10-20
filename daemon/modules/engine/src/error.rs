use std::backtrace::Backtrace;

use omnius_core_base::error::OmniError;

use crate::prelude::*;

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
    SerdeError,
    IoError,
    TimeError,
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
    Unauthenticated,
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::Unknown => write!(fmt, "unknown"),
            ErrorKind::SerdeError => write!(fmt, "serde error"),
            ErrorKind::IoError => write!(fmt, "io error"),
            ErrorKind::TimeError => write!(fmt, "time conversion error"),
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
            ErrorKind::Unauthenticated => write!(fmt, "unauthenticated"),
        }
    }
}

impl From<std::convert::Infallible> for Error {
    fn from(_: std::convert::Infallible) -> Self {
        Error::new(ErrorKind::Unknown)
    }
}

impl From<crate::model::NestedPathError> for Error {
    fn from(e: crate::model::NestedPathError) -> Self {
        Error::from_error(e, ErrorKind::InvalidInput)
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
        Error::from_error(e, ErrorKind::DatabaseError).with_message("database operation failed")
    }
}

impl From<ed25519_dalek::pkcs8::Error> for Error {
    fn from(e: ed25519_dalek::pkcs8::Error) -> Self {
        Error::from_error(e, ErrorKind::InvalidFormat).with_message("pkcs8 error")
    }
}

impl From<ed25519_dalek::pkcs8::spki::Error> for Error {
    fn from(e: ed25519_dalek::pkcs8::spki::Error) -> Self {
        Error::from_error(e, ErrorKind::InvalidFormat).with_message("pkcs8 spki error")
    }
}

impl<T> From<nom::Err<nom::error::Error<T>>> for Error {
    fn from(e: nom::Err<nom::error::Error<T>>) -> Error {
        match e {
            nom::Err::Incomplete(_) => Error::new(ErrorKind::InvalidFormat).with_message("nom incomplete"),
            nom::Err::Error(e) => Error::new(ErrorKind::InvalidFormat).with_message(format!("nom error: {:?}", e.code)),
            nom::Err::Failure(e) => Error::new(ErrorKind::InvalidFormat).with_message(format!("nom failure: {:?}", e.code)),
        }
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

impl From<RocketPackEncoderError> for Error {
    fn from(e: RocketPackEncoderError) -> Self {
        Error::from_error(e, ErrorKind::SerdeError).with_message("rocket pack encode error")
    }
}

impl From<RocketPackDecoderError> for Error {
    fn from(e: RocketPackDecoderError) -> Self {
        Error::from_error(e, ErrorKind::SerdeError).with_message("rocket pack decode error")
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

impl From<omnius_core_omnikit::Error> for Error {
    fn from(e: omnius_core_omnikit::Error) -> Self {
        match e.kind() {
            omnius_core_omnikit::ErrorKind::Unknown => Error::from_error(e, ErrorKind::Unknown),
            omnius_core_omnikit::ErrorKind::SerdeError => Error::from_error(e, ErrorKind::SerdeError),
            omnius_core_omnikit::ErrorKind::IoError => Error::from_error(e, ErrorKind::IoError),
            omnius_core_omnikit::ErrorKind::UnexpectedError => Error::from_error(e, ErrorKind::UnexpectedError),
            omnius_core_omnikit::ErrorKind::InvalidFormat => Error::from_error(e, ErrorKind::InvalidFormat),
            omnius_core_omnikit::ErrorKind::EndOfStream => Error::from_error(e, ErrorKind::EndOfStream),
            omnius_core_omnikit::ErrorKind::UnsupportedType => Error::from_error(e, ErrorKind::UnsupportedType),
            omnius_core_omnikit::ErrorKind::AlreadyConnected => Error::from_error(e, ErrorKind::AlreadyConnected),
            omnius_core_omnikit::ErrorKind::NotConnected => Error::from_error(e, ErrorKind::NotConnected),
        }
    }
}

impl From<std::num::TryFromIntError> for Error {
    fn from(e: std::num::TryFromIntError) -> Self {
        Error::from_error(e, ErrorKind::InvalidFormat).with_message("Integer conversion error")
    }
}

impl From<rupnp::Error> for Error {
    fn from(e: rupnp::Error) -> Self {
        Error::from_error(e, ErrorKind::UpnpError).with_message("UPnP operation failed")
    }
}

impl From<local_ip_address::Error> for Error {
    fn from(e: local_ip_address::Error) -> Self {
        Error::from_error(e, ErrorKind::NetworkError).with_message("Failed to get local IP address")
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::from_error(e, ErrorKind::HttpClientError).with_message("HTTP request failed")
    }
}

impl From<image::ImageError> for Error {
    fn from(e: image::ImageError) -> Self {
        Error::from_error(e, ErrorKind::ImageError).with_message("image operation failed")
    }
}

impl From<tokio::task::JoinError> for Error {
    fn from(e: tokio::task::JoinError) -> Self {
        Error::from_error(e, ErrorKind::TaskError).with_message("join failed")
    }
}

impl From<async_zip::error::ZipError> for Error {
    fn from(e: async_zip::error::ZipError) -> Self {
        Error::from_error(e, ErrorKind::IoError).with_message("zip error")
    }
}

impl From<sevenz_rust2::Error> for Error {
    fn from(e: sevenz_rust2::Error) -> Self {
        Error::from_error(e, ErrorKind::IoError).with_message("7z error")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_database_error_keeps_kind_and_source_test() {
        let source = omnius_core_migration::Error::new(omnius_core_migration::ErrorKind::DatabaseError);
        let error = Error::from(source);

        assert_eq!(error.kind(), &ErrorKind::DatabaseError);
        assert!(std::error::Error::source(&error).is_some());
    }
}
