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
pub enum CancelOperationResponse {
    /// Cancellation requested.
    Status202_CancellationRequested(models::OperationResponse),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::ErrorResponse),
    /// Resource not found.
    Status404_ResourceNotFound(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum GetOperationResponse {
    /// Operation state.
    Status200_OperationState(models::OperationResponse),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::ErrorResponse),
    /// Resource not found.
    Status404_ResourceNotFound(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum ListOperationsResponse {
    /// Known operations.
    Status200_KnownOperations(models::OperationList),
    /// Missing or invalid authentication.
    Status401_MissingOrInvalidAuthentication(models::ErrorResponse),
    /// Internal server error.
    Status500_InternalServerError(models::ErrorResponse),
}

/// Operations
#[async_trait]
#[allow(clippy::ptr_arg)]
pub trait Operations<E: std::fmt::Debug + Send + Sync + 'static = ()>: super::ErrorHandler<E> {
    type Claims;

    /// Cancel an operation.
    ///
    /// CancelOperation - POST /api/v1/operations/{operationId}/cancel
    async fn cancel_operation(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::CancelOperationPathParams,
    ) -> Result<CancelOperationResponse, E>;

    /// Get an operation.
    ///
    /// GetOperation - GET /api/v1/operations/{operationId}
    async fn get_operation(
        &self,

        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::GetOperationPathParams,
    ) -> Result<GetOperationResponse, E>;

    /// List operations.
    ///
    /// ListOperations - GET /api/v1/operations
    async fn list_operations(&self, method: &Method, host: &Host, cookies: &CookieJar, claims: &Self::Claims) -> Result<ListOperationsResponse, E>;
}
