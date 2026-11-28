use async_trait::async_trait;
use axum::extract::*;
use axum_extra::extract::CookieJar;
use bytes::Bytes;
use headers::Host;
use http::Method;
use serde::{Deserialize, Serialize};

use crate::{models, types::*};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum GetImageSocketResponse {
    /// The connection is upgraded to a WebSocket.
    Status101_TheConnectionIsUpgradedToAWebSocket,
    /// Invalid request. `code` is `InvalidRequest`, or `UnsupportedType` when the target cannot be opened as an archive, or `LimitExceeded` when an archive exceeds an entry, size, or nesting limit.
    Status400_InvalidRequest(models::Problem),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::Problem),
    /// The requested resource cannot be accessed or written. `code` is `Forbidden` for a `readonly` Library and for a mutation inside an archive, or `PasswordRequired` for a password-protected archive.
    Status403_TheRequestedResourceCannotBeAccessedOrWritten(models::Problem),
    /// The session already has the most image sockets it may keep open. `code` is `LimitExceeded`.
    Status429_TheSessionAlreadyHasTheMostImageSocketsItMayKeepOpen(models::Problem),
    /// Internal server error.
    Status500_InternalServerError(models::Problem),
}

/// ImageSocket
#[async_trait]
#[allow(clippy::ptr_arg)]
pub trait ImageSocket<E: std::fmt::Debug + Send + Sync + 'static = ()>: super::ErrorHandler<E> {
    type Claims;

    /// Open the image socket.
    ///
    /// GetImageSocket - GET /api/v1/sockets/image
    async fn get_image_socket(&self, method: &Method, host: &Host, cookies: &CookieJar, claims: &Self::Claims) -> Result<GetImageSocketResponse, E>;
}
