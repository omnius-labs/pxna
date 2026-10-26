use std::collections::HashMap;

use axum::{body::Body, extract::*, response::Response, routing::*};
use axum_extra::{
    TypedHeader,
    extract::{CookieJar, Query as QueryExtra},
};
use bytes::Bytes;
use headers::Host;
use http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode, header::CONTENT_TYPE};
use tracing::error;
use validator::{Validate, ValidationErrors};

#[allow(unused_imports)]
use crate::{apis, models};
use crate::{header, types::*};
#[allow(unused_imports)]
use crate::{models::check_xss_map, models::check_xss_map_nested, models::check_xss_map_string, models::check_xss_string, models::check_xss_vec_string};

/// Setup API Server.
pub fn new<I, A, E, C>(api_impl: I) -> Router
where
    I: AsRef<A> + Clone + Send + Sync + 'static,
    A: apis::auth::Auth<E, Claims = C>
        + apis::files::Files<E, Claims = C>
        + apis::folders::Folders<E, Claims = C>
        + apis::health::Health<E>
        + apis::libraries::Libraries<E, Claims = C>
        + apis::operations::Operations<E, Claims = C>
        + apis::performance::Performance<E, Claims = C>
        + apis::preferences::Preferences<E, Claims = C>
        + apis::rating::Rating<E, Claims = C>
        + apis::search::Search<E, Claims = C>
        + apis::setup::Setup<E>
        + apis::tags::Tags<E, Claims = C>
        + apis::thumbnails::Thumbnails<E, Claims = C>
        + apis::timeline::Timeline<E, Claims = C>
        + apis::uploads::Uploads<E, Claims = C>
        + apis::video::Video<E, Claims = C>
        + apis::CookieAuthentication<Claims = C>
        + apis::ApiAuthBasic<Claims = C>
        + Send
        + Sync
        + 'static,
    E: std::fmt::Debug + Send + Sync + 'static,
    C: Send + Sync + 'static,
{
    // build our application with a route
    Router::new()
        .route("/api/v1/auth/login", post(login::<I, A, E, C>))
        .route("/api/v1/auth/me", get(get_current_user::<I, A, E, C>))
        .route("/api/v1/auth/session", delete(delete_current_session::<I, A, E, C>))
        .route("/api/v1/capabilities", get(get_capabilities::<I, A, E, C>))
        .route("/api/v1/health", get(get_health::<I, A, E>))
        .route("/api/v1/libraries", get(list_libraries::<I, A, E, C>))
        .route("/api/v1/libraries/{library_id}", get(get_library::<I, A, E, C>))
        .route("/api/v1/libraries/{library_id}/entries/details", post(get_entry_details::<I, A, E, C>))
        .route("/api/v1/libraries/{library_id}/files", get(get_file::<I, A, E, C>))
        .route(
            "/api/v1/libraries/{library_id}/files/content",
            get(read_file_content::<I, A, E, C>).put(write_file_content::<I, A, E, C>),
        )
        .route("/api/v1/libraries/{library_id}/files/copy", post(copy_files::<I, A, E, C>))
        .route("/api/v1/libraries/{library_id}/files/delete", post(delete_files::<I, A, E, C>))
        .route("/api/v1/libraries/{library_id}/files/move", post(move_files::<I, A, E, C>))
        .route("/api/v1/libraries/{library_id}/folders", get(get_folder::<I, A, E, C>).post(create_folder::<I, A, E, C>))
        .route("/api/v1/libraries/{library_id}/folders/children", get(probe_folder_children::<I, A, E, C>))
        .route("/api/v1/libraries/{library_id}/folders/copy", post(copy_folders::<I, A, E, C>))
        .route("/api/v1/libraries/{library_id}/folders/delete", post(delete_folders::<I, A, E, C>))
        .route("/api/v1/libraries/{library_id}/folders/entries", get(list_folder_entries::<I, A, E, C>))
        .route("/api/v1/libraries/{library_id}/folders/move", post(move_folders::<I, A, E, C>))
        .route("/api/v1/libraries/{library_id}/folders/summaries", get(list_folder_summaries::<I, A, E, C>))
        .route("/api/v1/libraries/{library_id}/media-info", get(get_media_info::<I, A, E, C>))
        .route("/api/v1/libraries/{library_id}/rating", get(get_rating::<I, A, E, C>).put(put_rating::<I, A, E, C>))
        .route("/api/v1/libraries/{library_id}/tags", get(get_entry_tags::<I, A, E, C>).put(put_entry_tags::<I, A, E, C>))
        .route("/api/v1/libraries/{library_id}/thumbnails", get(get_thumbnail::<I, A, E, C>))
        .route("/api/v1/libraries/{library_id}/uploads", post(create_upload::<I, A, E, C>))
        .route(
            "/api/v1/libraries/{library_id}/uploads/{upload_id}",
            delete(delete_upload::<I, A, E, C>).get(get_upload::<I, A, E, C>),
        )
        .route("/api/v1/libraries/{library_id}/uploads/{upload_id}/commit", post(commit_upload::<I, A, E, C>))
        .route("/api/v1/libraries/{library_id}/uploads/{upload_id}/content", put(write_upload_content::<I, A, E, C>))
        .route("/api/v1/libraries/{library_id}/video/sessions", post(create_video_session::<I, A, E, C>))
        .route("/api/v1/libraries/{library_id}/video/sessions/{session_id}", delete(delete_video_session::<I, A, E, C>))
        .route(
            "/api/v1/libraries/{library_id}/video/sessions/{session_id}/playlist.m3u8",
            get(get_video_playlist::<I, A, E, C>),
        )
        .route(
            "/api/v1/libraries/{library_id}/video/sessions/{session_id}/segments/{segment_name}",
            get(get_video_segment::<I, A, E, C>),
        )
        .route("/api/v1/operations", get(list_operations::<I, A, E, C>))
        .route("/api/v1/operations/{operation_id}", get(get_operation::<I, A, E, C>))
        .route("/api/v1/operations/{operation_id}/cancel", post(cancel_operation::<I, A, E, C>))
        .route("/api/v1/performance/client-timings", post(report_client_timings::<I, A, E, C>))
        .route("/api/v1/preferences", get(get_preferences::<I, A, E, C>).put(put_preferences::<I, A, E, C>))
        .route("/api/v1/search/entries", get(list_search_entries::<I, A, E, C>))
        .route("/api/v1/setup", post(create_initial_setup::<I, A, E>))
        .route("/api/v1/setup/status", get(get_setup_status::<I, A, E>))
        .route("/api/v1/tags", get(list_tags::<I, A, E, C>))
        .route("/api/v1/timeline/entries", get(list_timeline_entries::<I, A, E, C>))
        .route("/api/v1/timeline/months", get(list_timeline_months::<I, A, E, C>))
        .route("/api/v1/timeline/refresh", post(refresh_timeline::<I, A, E, C>))
        .route("/api/v1/timeline/status", get(get_timeline_status::<I, A, E, C>))
        .with_state(api_impl)
}

#[tracing::instrument(skip_all)]
fn delete_current_session_validation() -> std::result::Result<(), ValidationErrors> {
    Ok(())
}
/// DeleteCurrentSession - DELETE /api/v1/auth/session
#[tracing::instrument(skip_all)]
async fn delete_current_session<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::auth::Auth<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || delete_current_session_validation()).await.unwrap();

    let Ok(()) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().delete_current_session(&method, &host, &cookies, &claims).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::auth::DeleteCurrentSessionResponse::Status204_SessionRevoked { set_cookie } => {
                if let Some(set_cookie) = set_cookie {
                    let set_cookie = match header::IntoHeaderValue(set_cookie).try_into() {
                        Ok(val) => val,
                        Err(e) => {
                            return Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .body(Body::from(format!("An internal server error occurred handling set_cookie header - {e}")))
                                .map_err(|e| {
                                    error!(error = ?e);
                                    StatusCode::INTERNAL_SERVER_ERROR
                                });
                        }
                    };

                    {
                        let mut response_headers = response.headers_mut().unwrap();
                        response_headers.insert(HeaderName::from_static("set-cookie"), set_cookie);
                    }
                }
                let mut response = response.status(204);
                response.body(Body::empty())
            }
            apis::auth::DeleteCurrentSessionResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::auth::DeleteCurrentSessionResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn get_current_user_validation() -> std::result::Result<(), ValidationErrors> {
    Ok(())
}
/// GetCurrentUser - GET /api/v1/auth/me
#[tracing::instrument(skip_all)]
async fn get_current_user<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::auth::Auth<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || get_current_user_validation()).await.unwrap();

    let Ok(()) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().get_current_user(&method, &host, &cookies, &claims).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::auth::GetCurrentUserResponse::Status200_CurrentAuthenticatedUserAndSession(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::auth::GetCurrentUserResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::auth::GetCurrentUserResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[derive(validator::Validate)]
#[allow(dead_code)]
struct LoginBodyValidator<'a> {
    #[validate(nested)]
    body: &'a models::LoginRequest,
}

#[tracing::instrument(skip_all)]
fn login_validation(body: models::LoginRequest) -> std::result::Result<(models::LoginRequest,), ValidationErrors> {
    let b = LoginBodyValidator { body: &body };
    b.validate()?;

    Ok((body,))
}
/// Login - POST /api/v1/auth/login
#[tracing::instrument(skip_all)]
async fn login<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    State(api_impl): State<I>,
    Json(body): Json<models::LoginRequest>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::auth::Auth<E, Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || login_validation(body)).await.unwrap();

    let Ok((body,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().login(&method, &host, &cookies, &body).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::auth::LoginResponse::Status200_LoginSucceeded { body, set_cookie } => {
                if let Some(set_cookie) = set_cookie {
                    let set_cookie = match header::IntoHeaderValue(set_cookie).try_into() {
                        Ok(val) => val,
                        Err(e) => {
                            return Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .body(Body::from(format!("An internal server error occurred handling set_cookie header - {e}")))
                                .map_err(|e| {
                                    error!(error = ?e);
                                    StatusCode::INTERNAL_SERVER_ERROR
                                });
                        }
                    };

                    {
                        let mut response_headers = response.headers_mut().unwrap();
                        response_headers.insert(HeaderName::from_static("set-cookie"), set_cookie);
                    }
                }
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::auth::LoginResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::auth::LoginResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::auth::LoginResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[derive(validator::Validate)]
#[allow(dead_code)]
struct CopyFilesBodyValidator<'a> {
    #[validate(nested)]
    body: &'a models::EntryTransferRequest,
}

#[tracing::instrument(skip_all)]
fn copy_files_validation(
    path_params: models::CopyFilesPathParams,
    body: models::EntryTransferRequest,
) -> std::result::Result<(models::CopyFilesPathParams, models::EntryTransferRequest), ValidationErrors> {
    path_params.validate()?;
    let b = CopyFilesBodyValidator { body: &body };
    b.validate()?;

    Ok((path_params, body))
}
/// CopyFiles - POST /api/v1/libraries/{libraryId}/files/copy
#[tracing::instrument(skip_all)]
async fn copy_files<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::CopyFilesPathParams>,
    State(api_impl): State<I>,
    Json(body): Json<models::EntryTransferRequest>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::files::Files<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || copy_files_validation(path_params, body)).await.unwrap();

    let Ok((path_params, body)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().copy_files(&method, &host, &cookies, &claims, &path_params, &body).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::files::CopyFilesResponse::Status202_CopyOperationStarted(body) => {
                let mut response = response.status(202);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::CopyFilesResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::CopyFilesResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::CopyFilesResponse::Status403_TheRequestedResourceCannotBeAccessedOrWritten(body) => {
                let mut response = response.status(403);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::CopyFilesResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::CopyFilesResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[derive(validator::Validate)]
#[allow(dead_code)]
struct DeleteFilesBodyValidator<'a> {
    #[validate(nested)]
    body: &'a models::DeleteFilesRequest,
}

#[tracing::instrument(skip_all)]
fn delete_files_validation(
    path_params: models::DeleteFilesPathParams,
    body: models::DeleteFilesRequest,
) -> std::result::Result<(models::DeleteFilesPathParams, models::DeleteFilesRequest), ValidationErrors> {
    path_params.validate()?;
    let b = DeleteFilesBodyValidator { body: &body };
    b.validate()?;

    Ok((path_params, body))
}
/// DeleteFiles - POST /api/v1/libraries/{libraryId}/files/delete
#[tracing::instrument(skip_all)]
async fn delete_files<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::DeleteFilesPathParams>,
    State(api_impl): State<I>,
    Json(body): Json<models::DeleteFilesRequest>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::files::Files<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || delete_files_validation(path_params, body)).await.unwrap();

    let Ok((path_params, body)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().delete_files(&method, &host, &cookies, &claims, &path_params, &body).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::files::DeleteFilesResponse::Status202_DeleteOperationStarted(body) => {
                let mut response = response.status(202);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::DeleteFilesResponse::Status204_FileDeleted => {
                let mut response = response.status(204);
                response.body(Body::empty())
            }
            apis::files::DeleteFilesResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::DeleteFilesResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::DeleteFilesResponse::Status403_TheRequestedResourceCannotBeAccessedOrWritten(body) => {
                let mut response = response.status(403);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::DeleteFilesResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::DeleteFilesResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn get_file_validation(
    path_params: models::GetFilePathParams,
    query_params: models::GetFileQueryParams,
) -> std::result::Result<(models::GetFilePathParams, models::GetFileQueryParams), ValidationErrors> {
    path_params.validate()?;
    query_params.validate()?;

    Ok((path_params, query_params))
}
/// GetFile - GET /api/v1/libraries/{libraryId}/files
#[tracing::instrument(skip_all)]
async fn get_file<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::GetFilePathParams>,
    QueryExtra(query_params): QueryExtra<models::GetFileQueryParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::files::Files<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || get_file_validation(path_params, query_params)).await.unwrap();

    let Ok((path_params, query_params)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().get_file(&method, &host, &cookies, &claims, &path_params, &query_params).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::files::GetFileResponse::Status200_FileMetadata(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::GetFileResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::GetFileResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::GetFileResponse::Status403_TheRequestedResourceCannotBeAccessedOrWritten(body) => {
                let mut response = response.status(403);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::GetFileResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::GetFileResponse::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(body) => {
                let mut response = response.status(409);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::GetFileResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::GetFileResponse::Status504_ExtractionOrThumbnailGenerationTimedOut(body) => {
                let mut response = response.status(504);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[derive(validator::Validate)]
#[allow(dead_code)]
struct MoveFilesBodyValidator<'a> {
    #[validate(nested)]
    body: &'a models::EntryTransferRequest,
}

#[tracing::instrument(skip_all)]
fn move_files_validation(
    path_params: models::MoveFilesPathParams,
    body: models::EntryTransferRequest,
) -> std::result::Result<(models::MoveFilesPathParams, models::EntryTransferRequest), ValidationErrors> {
    path_params.validate()?;
    let b = MoveFilesBodyValidator { body: &body };
    b.validate()?;

    Ok((path_params, body))
}
/// MoveFiles - POST /api/v1/libraries/{libraryId}/files/move
#[tracing::instrument(skip_all)]
async fn move_files<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::MoveFilesPathParams>,
    State(api_impl): State<I>,
    Json(body): Json<models::EntryTransferRequest>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::files::Files<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || move_files_validation(path_params, body)).await.unwrap();

    let Ok((path_params, body)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().move_files(&method, &host, &cookies, &claims, &path_params, &body).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::files::MoveFilesResponse::Status200_MovedSynchronously(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::MoveFilesResponse::Status202_MoveOperationStarted(body) => {
                let mut response = response.status(202);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::MoveFilesResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::MoveFilesResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::MoveFilesResponse::Status403_TheRequestedResourceCannotBeAccessedOrWritten(body) => {
                let mut response = response.status(403);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::MoveFilesResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::MoveFilesResponse::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(body) => {
                let mut response = response.status(409);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::MoveFilesResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn read_file_content_validation(
    header_params: models::ReadFileContentHeaderParams,
    path_params: models::ReadFileContentPathParams,
    query_params: models::ReadFileContentQueryParams,
) -> std::result::Result<(models::ReadFileContentHeaderParams, models::ReadFileContentPathParams, models::ReadFileContentQueryParams), ValidationErrors> {
    header_params.validate()?;
    path_params.validate()?;
    query_params.validate()?;

    Ok((header_params, path_params, query_params))
}
/// ReadFileContent - GET /api/v1/libraries/{libraryId}/files/content
#[tracing::instrument(skip_all)]
async fn read_file_content<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::ReadFileContentPathParams>,
    QueryExtra(query_params): QueryExtra<models::ReadFileContentQueryParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::files::Files<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    // Header parameters
    let header_params = {
        let header_range = headers.get(HeaderName::from_static("range"));

        let header_range = match header_range {
            Some(v) => match header::IntoHeaderValue::<String>::try_from((*v).clone()) {
                Ok(result) => Some(result.0),
                Err(err) => {
                    return Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Body::from(format!("Invalid header Range - {err}")))
                        .map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        });
                }
            },
            None => None,
        };

        models::ReadFileContentHeaderParams { range: header_range }
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || read_file_content_validation(header_params, path_params, query_params))
        .await
        .unwrap();

    let Ok((header_params, path_params, query_params)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl
        .as_ref()
        .read_file_content(&method, &host, &cookies, &claims, &header_params, &path_params, &query_params)
        .await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::files::ReadFileContentResponse::Status200_FullOrRequestedFileBytes {
                body,
                accept_ranges,
                content_length,
                e_tag,
            } => {
                if let Some(accept_ranges) = accept_ranges {
                    let accept_ranges = match header::IntoHeaderValue(accept_ranges).try_into() {
                        Ok(val) => val,
                        Err(e) => {
                            return Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .body(Body::from(format!("An internal server error occurred handling accept_ranges header - {e}")))
                                .map_err(|e| {
                                    error!(error = ?e);
                                    StatusCode::INTERNAL_SERVER_ERROR
                                });
                        }
                    };

                    {
                        let mut response_headers = response.headers_mut().unwrap();
                        response_headers.insert(HeaderName::from_static("accept-ranges"), accept_ranges);
                    }
                }
                if let Some(content_length) = content_length {
                    let content_length = match header::IntoHeaderValue(content_length).try_into() {
                        Ok(val) => val,
                        Err(e) => {
                            return Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .body(Body::from(format!("An internal server error occurred handling content_length header - {e}")))
                                .map_err(|e| {
                                    error!(error = ?e);
                                    StatusCode::INTERNAL_SERVER_ERROR
                                });
                        }
                    };

                    {
                        let mut response_headers = response.headers_mut().unwrap();
                        response_headers.insert(HeaderName::from_static("content-length"), content_length);
                    }
                }
                if let Some(e_tag) = e_tag {
                    let e_tag = match header::IntoHeaderValue(e_tag).try_into() {
                        Ok(val) => val,
                        Err(e) => {
                            return Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .body(Body::from(format!("An internal server error occurred handling e_tag header - {e}")))
                                .map_err(|e| {
                                    error!(error = ?e);
                                    StatusCode::INTERNAL_SERVER_ERROR
                                });
                        }
                    };

                    {
                        let mut response_headers = response.headers_mut().unwrap();
                        response_headers.insert(HeaderName::from_static("etag"), e_tag);
                    }
                }
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/octet-stream"));
                }

                let body_content = body.0;
                response.body(Body::from(body_content))
            }
            apis::files::ReadFileContentResponse::Status206_PartialFileBytes {
                body,
                accept_ranges,
                content_range,
                content_length,
                e_tag,
            } => {
                if let Some(accept_ranges) = accept_ranges {
                    let accept_ranges = match header::IntoHeaderValue(accept_ranges).try_into() {
                        Ok(val) => val,
                        Err(e) => {
                            return Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .body(Body::from(format!("An internal server error occurred handling accept_ranges header - {e}")))
                                .map_err(|e| {
                                    error!(error = ?e);
                                    StatusCode::INTERNAL_SERVER_ERROR
                                });
                        }
                    };

                    {
                        let mut response_headers = response.headers_mut().unwrap();
                        response_headers.insert(HeaderName::from_static("accept-ranges"), accept_ranges);
                    }
                }
                if let Some(content_range) = content_range {
                    let content_range = match header::IntoHeaderValue(content_range).try_into() {
                        Ok(val) => val,
                        Err(e) => {
                            return Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .body(Body::from(format!("An internal server error occurred handling content_range header - {e}")))
                                .map_err(|e| {
                                    error!(error = ?e);
                                    StatusCode::INTERNAL_SERVER_ERROR
                                });
                        }
                    };

                    {
                        let mut response_headers = response.headers_mut().unwrap();
                        response_headers.insert(HeaderName::from_static("content-range"), content_range);
                    }
                }
                if let Some(content_length) = content_length {
                    let content_length = match header::IntoHeaderValue(content_length).try_into() {
                        Ok(val) => val,
                        Err(e) => {
                            return Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .body(Body::from(format!("An internal server error occurred handling content_length header - {e}")))
                                .map_err(|e| {
                                    error!(error = ?e);
                                    StatusCode::INTERNAL_SERVER_ERROR
                                });
                        }
                    };

                    {
                        let mut response_headers = response.headers_mut().unwrap();
                        response_headers.insert(HeaderName::from_static("content-length"), content_length);
                    }
                }
                if let Some(e_tag) = e_tag {
                    let e_tag = match header::IntoHeaderValue(e_tag).try_into() {
                        Ok(val) => val,
                        Err(e) => {
                            return Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .body(Body::from(format!("An internal server error occurred handling e_tag header - {e}")))
                                .map_err(|e| {
                                    error!(error = ?e);
                                    StatusCode::INTERNAL_SERVER_ERROR
                                });
                        }
                    };

                    {
                        let mut response_headers = response.headers_mut().unwrap();
                        response_headers.insert(HeaderName::from_static("etag"), e_tag);
                    }
                }
                let mut response = response.status(206);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/octet-stream"));
                }

                let body_content = body.0;
                response.body(Body::from(body_content))
            }
            apis::files::ReadFileContentResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::ReadFileContentResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::ReadFileContentResponse::Status403_TheRequestedResourceCannotBeAccessedOrWritten(body) => {
                let mut response = response.status(403);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::ReadFileContentResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::ReadFileContentResponse::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(body) => {
                let mut response = response.status(409);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::ReadFileContentResponse::Status416_RequestedByteRangeCannotBeSatisfied(body) => {
                let mut response = response.status(416);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::ReadFileContentResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::ReadFileContentResponse::Status504_ExtractionOrThumbnailGenerationTimedOut(body) => {
                let mut response = response.status(504);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[derive(validator::Validate)]
#[allow(dead_code)]
struct WriteFileContentBodyValidator<'a> {
    body: &'a [u8],
}

#[tracing::instrument(skip_all)]
fn write_file_content_validation(
    path_params: models::WriteFileContentPathParams,
    query_params: models::WriteFileContentQueryParams,
    body: Bytes,
) -> std::result::Result<(models::WriteFileContentPathParams, models::WriteFileContentQueryParams, Bytes), ValidationErrors> {
    path_params.validate()?;
    query_params.validate()?;

    Ok((path_params, query_params, body))
}
/// WriteFileContent - PUT /api/v1/libraries/{libraryId}/files/content
#[tracing::instrument(skip_all)]
async fn write_file_content<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::WriteFileContentPathParams>,
    QueryExtra(query_params): QueryExtra<models::WriteFileContentQueryParams>,
    State(api_impl): State<I>,
    body: Bytes,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::files::Files<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || write_file_content_validation(path_params, query_params, body))
        .await
        .unwrap();

    let Ok((path_params, query_params, body)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl
        .as_ref()
        .write_file_content(&method, &host, &cookies, &claims, &path_params, &query_params, &body)
        .await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::files::WriteFileContentResponse::Status201_FileWritten(body) => {
                let mut response = response.status(201);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::WriteFileContentResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::WriteFileContentResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::WriteFileContentResponse::Status403_TheRequestedResourceCannotBeAccessedOrWritten(body) => {
                let mut response = response.status(403);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::WriteFileContentResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::WriteFileContentResponse::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(body) => {
                let mut response = response.status(409);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::WriteFileContentResponse::Status413_TheRequestBodyExceedsTheOne(body) => {
                let mut response = response.status(413);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::WriteFileContentResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::files::WriteFileContentResponse::Status507_TheDaemonDoesNotHaveEnoughDiskSpace(body) => {
                let mut response = response.status(507);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[derive(validator::Validate)]
#[allow(dead_code)]
struct CopyFoldersBodyValidator<'a> {
    #[validate(nested)]
    body: &'a models::EntryTransferRequest,
}

#[tracing::instrument(skip_all)]
fn copy_folders_validation(
    path_params: models::CopyFoldersPathParams,
    body: models::EntryTransferRequest,
) -> std::result::Result<(models::CopyFoldersPathParams, models::EntryTransferRequest), ValidationErrors> {
    path_params.validate()?;
    let b = CopyFoldersBodyValidator { body: &body };
    b.validate()?;

    Ok((path_params, body))
}
/// CopyFolders - POST /api/v1/libraries/{libraryId}/folders/copy
#[tracing::instrument(skip_all)]
async fn copy_folders<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::CopyFoldersPathParams>,
    State(api_impl): State<I>,
    Json(body): Json<models::EntryTransferRequest>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::folders::Folders<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || copy_folders_validation(path_params, body)).await.unwrap();

    let Ok((path_params, body)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().copy_folders(&method, &host, &cookies, &claims, &path_params, &body).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::folders::CopyFoldersResponse::Status202_CopyOperationStarted(body) => {
                let mut response = response.status(202);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::CopyFoldersResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::CopyFoldersResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::CopyFoldersResponse::Status403_TheRequestedResourceCannotBeAccessedOrWritten(body) => {
                let mut response = response.status(403);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::CopyFoldersResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::CopyFoldersResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[derive(validator::Validate)]
#[allow(dead_code)]
struct CreateFolderBodyValidator<'a> {
    #[validate(nested)]
    body: &'a models::CreateFolderRequest,
}

#[tracing::instrument(skip_all)]
fn create_folder_validation(
    path_params: models::CreateFolderPathParams,
    body: models::CreateFolderRequest,
) -> std::result::Result<(models::CreateFolderPathParams, models::CreateFolderRequest), ValidationErrors> {
    path_params.validate()?;
    let b = CreateFolderBodyValidator { body: &body };
    b.validate()?;

    Ok((path_params, body))
}
/// CreateFolder - POST /api/v1/libraries/{libraryId}/folders
#[tracing::instrument(skip_all)]
async fn create_folder<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::CreateFolderPathParams>,
    State(api_impl): State<I>,
    Json(body): Json<models::CreateFolderRequest>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::folders::Folders<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || create_folder_validation(path_params, body)).await.unwrap();

    let Ok((path_params, body)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().create_folder(&method, &host, &cookies, &claims, &path_params, &body).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::folders::CreateFolderResponse::Status201_FolderCreated(body) => {
                let mut response = response.status(201);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::CreateFolderResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::CreateFolderResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::CreateFolderResponse::Status403_TheRequestedResourceCannotBeAccessedOrWritten(body) => {
                let mut response = response.status(403);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::CreateFolderResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::CreateFolderResponse::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(body) => {
                let mut response = response.status(409);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::CreateFolderResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[derive(validator::Validate)]
#[allow(dead_code)]
struct DeleteFoldersBodyValidator<'a> {
    #[validate(nested)]
    body: &'a models::DeleteFoldersRequest,
}

#[tracing::instrument(skip_all)]
fn delete_folders_validation(
    path_params: models::DeleteFoldersPathParams,
    body: models::DeleteFoldersRequest,
) -> std::result::Result<(models::DeleteFoldersPathParams, models::DeleteFoldersRequest), ValidationErrors> {
    path_params.validate()?;
    let b = DeleteFoldersBodyValidator { body: &body };
    b.validate()?;

    Ok((path_params, body))
}
/// DeleteFolders - POST /api/v1/libraries/{libraryId}/folders/delete
#[tracing::instrument(skip_all)]
async fn delete_folders<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::DeleteFoldersPathParams>,
    State(api_impl): State<I>,
    Json(body): Json<models::DeleteFoldersRequest>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::folders::Folders<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || delete_folders_validation(path_params, body)).await.unwrap();

    let Ok((path_params, body)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().delete_folders(&method, &host, &cookies, &claims, &path_params, &body).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::folders::DeleteFoldersResponse::Status202_DeleteOperationStarted(body) => {
                let mut response = response.status(202);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::DeleteFoldersResponse::Status204_FoldersDeleted => {
                let mut response = response.status(204);
                response.body(Body::empty())
            }
            apis::folders::DeleteFoldersResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::DeleteFoldersResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::DeleteFoldersResponse::Status403_TheRequestedResourceCannotBeAccessedOrWritten(body) => {
                let mut response = response.status(403);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::DeleteFoldersResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::DeleteFoldersResponse::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(body) => {
                let mut response = response.status(409);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::DeleteFoldersResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[derive(validator::Validate)]
#[allow(dead_code)]
struct GetEntryDetailsBodyValidator<'a> {
    #[validate(nested)]
    body: &'a models::EntryDetailsRequest,
}

#[tracing::instrument(skip_all)]
fn get_entry_details_validation(
    path_params: models::GetEntryDetailsPathParams,
    body: models::EntryDetailsRequest,
) -> std::result::Result<(models::GetEntryDetailsPathParams, models::EntryDetailsRequest), ValidationErrors> {
    path_params.validate()?;
    let b = GetEntryDetailsBodyValidator { body: &body };
    b.validate()?;

    Ok((path_params, body))
}
/// GetEntryDetails - POST /api/v1/libraries/{libraryId}/entries/details
#[tracing::instrument(skip_all)]
async fn get_entry_details<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::GetEntryDetailsPathParams>,
    State(api_impl): State<I>,
    Json(body): Json<models::EntryDetailsRequest>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::folders::Folders<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || get_entry_details_validation(path_params, body)).await.unwrap();

    let Ok((path_params, body)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().get_entry_details(&method, &host, &cookies, &claims, &path_params, &body).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::folders::GetEntryDetailsResponse::Status200_ExactlyOneDetailOrErrorKindPerRequestedPath(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::GetEntryDetailsResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::GetEntryDetailsResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::GetEntryDetailsResponse::Status403_TheRequestedResourceCannotBeAccessedOrWritten(body) => {
                let mut response = response.status(403);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::GetEntryDetailsResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::GetEntryDetailsResponse::Status413_TheRequestBodyExceedsTheOne(body) => {
                let mut response = response.status(413);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::GetEntryDetailsResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn get_folder_validation(
    path_params: models::GetFolderPathParams,
    query_params: models::GetFolderQueryParams,
) -> std::result::Result<(models::GetFolderPathParams, models::GetFolderQueryParams), ValidationErrors> {
    path_params.validate()?;
    query_params.validate()?;

    Ok((path_params, query_params))
}
/// GetFolder - GET /api/v1/libraries/{libraryId}/folders
#[tracing::instrument(skip_all)]
async fn get_folder<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::GetFolderPathParams>,
    QueryExtra(query_params): QueryExtra<models::GetFolderQueryParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::folders::Folders<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || get_folder_validation(path_params, query_params)).await.unwrap();

    let Ok((path_params, query_params)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().get_folder(&method, &host, &cookies, &claims, &path_params, &query_params).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::folders::GetFolderResponse::Status200_FolderMetadata(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::GetFolderResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::GetFolderResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::GetFolderResponse::Status403_TheRequestedResourceCannotBeAccessedOrWritten(body) => {
                let mut response = response.status(403);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::GetFolderResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::GetFolderResponse::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(body) => {
                let mut response = response.status(409);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::GetFolderResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::GetFolderResponse::Status504_ExtractionOrThumbnailGenerationTimedOut(body) => {
                let mut response = response.status(504);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn list_folder_entries_validation(
    path_params: models::ListFolderEntriesPathParams,
    query_params: models::ListFolderEntriesQueryParams,
) -> std::result::Result<(models::ListFolderEntriesPathParams, models::ListFolderEntriesQueryParams), ValidationErrors> {
    path_params.validate()?;
    query_params.validate()?;

    Ok((path_params, query_params))
}
/// ListFolderEntries - GET /api/v1/libraries/{libraryId}/folders/entries
#[tracing::instrument(skip_all)]
async fn list_folder_entries<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::ListFolderEntriesPathParams>,
    QueryExtra(query_params): QueryExtra<models::ListFolderEntriesQueryParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::folders::Folders<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || list_folder_entries_validation(path_params, query_params))
        .await
        .unwrap();

    let Ok((path_params, query_params)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().list_folder_entries(&method, &host, &cookies, &claims, &path_params, &query_params).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::folders::ListFolderEntriesResponse::Status200_FolderEntries(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::ListFolderEntriesResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::ListFolderEntriesResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::ListFolderEntriesResponse::Status403_TheRequestedResourceCannotBeAccessedOrWritten(body) => {
                let mut response = response.status(403);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::ListFolderEntriesResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::ListFolderEntriesResponse::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(body) => {
                let mut response = response.status(409);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::ListFolderEntriesResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::ListFolderEntriesResponse::Status504_ExtractionOrThumbnailGenerationTimedOut(body) => {
                let mut response = response.status(504);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn list_folder_summaries_validation(
    path_params: models::ListFolderSummariesPathParams,
    query_params: models::ListFolderSummariesQueryParams,
) -> std::result::Result<(models::ListFolderSummariesPathParams, models::ListFolderSummariesQueryParams), ValidationErrors> {
    path_params.validate()?;
    query_params.validate()?;

    Ok((path_params, query_params))
}
/// ListFolderSummaries - GET /api/v1/libraries/{libraryId}/folders/summaries
#[tracing::instrument(skip_all)]
async fn list_folder_summaries<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::ListFolderSummariesPathParams>,
    QueryExtra(query_params): QueryExtra<models::ListFolderSummariesQueryParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::folders::Folders<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || list_folder_summaries_validation(path_params, query_params))
        .await
        .unwrap();

    let Ok((path_params, query_params)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl
        .as_ref()
        .list_folder_summaries(&method, &host, &cookies, &claims, &path_params, &query_params)
        .await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::folders::ListFolderSummariesResponse::Status200_NamesAndKindsFromOneSummarySnapshot(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::ListFolderSummariesResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::ListFolderSummariesResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::ListFolderSummariesResponse::Status403_TheRequestedResourceCannotBeAccessedOrWritten(body) => {
                let mut response = response.status(403);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::ListFolderSummariesResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::ListFolderSummariesResponse::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(body) => {
                let mut response = response.status(409);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::ListFolderSummariesResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::ListFolderSummariesResponse::Status504_ExtractionOrThumbnailGenerationTimedOut(body) => {
                let mut response = response.status(504);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[derive(validator::Validate)]
#[allow(dead_code)]
struct MoveFoldersBodyValidator<'a> {
    #[validate(nested)]
    body: &'a models::EntryTransferRequest,
}

#[tracing::instrument(skip_all)]
fn move_folders_validation(
    path_params: models::MoveFoldersPathParams,
    body: models::EntryTransferRequest,
) -> std::result::Result<(models::MoveFoldersPathParams, models::EntryTransferRequest), ValidationErrors> {
    path_params.validate()?;
    let b = MoveFoldersBodyValidator { body: &body };
    b.validate()?;

    Ok((path_params, body))
}
/// MoveFolders - POST /api/v1/libraries/{libraryId}/folders/move
#[tracing::instrument(skip_all)]
async fn move_folders<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::MoveFoldersPathParams>,
    State(api_impl): State<I>,
    Json(body): Json<models::EntryTransferRequest>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::folders::Folders<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || move_folders_validation(path_params, body)).await.unwrap();

    let Ok((path_params, body)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().move_folders(&method, &host, &cookies, &claims, &path_params, &body).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::folders::MoveFoldersResponse::Status200_MovedSynchronously(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::MoveFoldersResponse::Status202_MoveOperationStarted(body) => {
                let mut response = response.status(202);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::MoveFoldersResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::MoveFoldersResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::MoveFoldersResponse::Status403_TheRequestedResourceCannotBeAccessedOrWritten(body) => {
                let mut response = response.status(403);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::MoveFoldersResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::MoveFoldersResponse::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(body) => {
                let mut response = response.status(409);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::MoveFoldersResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn probe_folder_children_validation(
    path_params: models::ProbeFolderChildrenPathParams,
    query_params: models::ProbeFolderChildrenQueryParams,
) -> std::result::Result<(models::ProbeFolderChildrenPathParams, models::ProbeFolderChildrenQueryParams), ValidationErrors> {
    path_params.validate()?;
    query_params.validate()?;

    Ok((path_params, query_params))
}
/// ProbeFolderChildren - GET /api/v1/libraries/{libraryId}/folders/children
#[tracing::instrument(skip_all)]
async fn probe_folder_children<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::ProbeFolderChildrenPathParams>,
    QueryExtra(query_params): QueryExtra<models::ProbeFolderChildrenQueryParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::folders::Folders<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || probe_folder_children_validation(path_params, query_params))
        .await
        .unwrap();

    let Ok((path_params, query_params)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl
        .as_ref()
        .probe_folder_children(&method, &host, &cookies, &claims, &path_params, &query_params)
        .await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::folders::ProbeFolderChildrenResponse::Status200_True(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::ProbeFolderChildrenResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::ProbeFolderChildrenResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::ProbeFolderChildrenResponse::Status403_TheRequestedResourceCannotBeAccessedOrWritten(body) => {
                let mut response = response.status(403);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::ProbeFolderChildrenResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::ProbeFolderChildrenResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::folders::ProbeFolderChildrenResponse::Status501_ChildProbesAreNotProvided => {
                let mut response = response.status(501);
                response.body(Body::empty())
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn get_health_validation() -> std::result::Result<(), ValidationErrors> {
    Ok(())
}
/// GetHealth - GET /api/v1/health
#[tracing::instrument(skip_all)]
async fn get_health<I, A, E>(method: Method, TypedHeader(host): TypedHeader<Host>, cookies: CookieJar, State(api_impl): State<I>) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::health::Health<E> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || get_health_validation()).await.unwrap();

    let Ok(()) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().get_health(&method, &host, &cookies).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::health::GetHealthResponse::Status200_TheDaemonIsRunning(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::health::GetHealthResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn get_capabilities_validation() -> std::result::Result<(), ValidationErrors> {
    Ok(())
}
/// GetCapabilities - GET /api/v1/capabilities
#[tracing::instrument(skip_all)]
async fn get_capabilities<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::libraries::Libraries<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || get_capabilities_validation()).await.unwrap();

    let Ok(()) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().get_capabilities(&method, &host, &cookies, &claims).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::libraries::GetCapabilitiesResponse::Status200_OptionalBrowsingFeaturesImplementedByThisDaemon(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::libraries::GetCapabilitiesResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::libraries::GetCapabilitiesResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn get_library_validation(path_params: models::GetLibraryPathParams) -> std::result::Result<(models::GetLibraryPathParams,), ValidationErrors> {
    path_params.validate()?;

    Ok((path_params,))
}
/// GetLibrary - GET /api/v1/libraries/{libraryId}
#[tracing::instrument(skip_all)]
async fn get_library<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::GetLibraryPathParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::libraries::Libraries<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || get_library_validation(path_params)).await.unwrap();

    let Ok((path_params,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().get_library(&method, &host, &cookies, &claims, &path_params).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::libraries::GetLibraryResponse::Status200_LibraryMetadata(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::libraries::GetLibraryResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::libraries::GetLibraryResponse::Status403_TheRequestedResourceCannotBeAccessedOrWritten(body) => {
                let mut response = response.status(403);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::libraries::GetLibraryResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::libraries::GetLibraryResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn list_libraries_validation() -> std::result::Result<(), ValidationErrors> {
    Ok(())
}
/// ListLibraries - GET /api/v1/libraries
#[tracing::instrument(skip_all)]
async fn list_libraries<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::libraries::Libraries<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || list_libraries_validation()).await.unwrap();

    let Ok(()) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().list_libraries(&method, &host, &cookies, &claims).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::libraries::ListLibrariesResponse::Status200_AvailableLibraries(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::libraries::ListLibrariesResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::libraries::ListLibrariesResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn cancel_operation_validation(path_params: models::CancelOperationPathParams) -> std::result::Result<(models::CancelOperationPathParams,), ValidationErrors> {
    path_params.validate()?;

    Ok((path_params,))
}
/// CancelOperation - POST /api/v1/operations/{operationId}/cancel
#[tracing::instrument(skip_all)]
async fn cancel_operation<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::CancelOperationPathParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::operations::Operations<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || cancel_operation_validation(path_params)).await.unwrap();

    let Ok((path_params,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().cancel_operation(&method, &host, &cookies, &claims, &path_params).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::operations::CancelOperationResponse::Status202_CancellationRequested(body) => {
                let mut response = response.status(202);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::operations::CancelOperationResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::operations::CancelOperationResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::operations::CancelOperationResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn get_operation_validation(path_params: models::GetOperationPathParams) -> std::result::Result<(models::GetOperationPathParams,), ValidationErrors> {
    path_params.validate()?;

    Ok((path_params,))
}
/// GetOperation - GET /api/v1/operations/{operationId}
#[tracing::instrument(skip_all)]
async fn get_operation<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::GetOperationPathParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::operations::Operations<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || get_operation_validation(path_params)).await.unwrap();

    let Ok((path_params,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().get_operation(&method, &host, &cookies, &claims, &path_params).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::operations::GetOperationResponse::Status200_OperationState(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::operations::GetOperationResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::operations::GetOperationResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::operations::GetOperationResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn list_operations_validation() -> std::result::Result<(), ValidationErrors> {
    Ok(())
}
/// ListOperations - GET /api/v1/operations
#[tracing::instrument(skip_all)]
async fn list_operations<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::operations::Operations<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || list_operations_validation()).await.unwrap();

    let Ok(()) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().list_operations(&method, &host, &cookies, &claims).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::operations::ListOperationsResponse::Status200_KnownOperations(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::operations::ListOperationsResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::operations::ListOperationsResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[derive(validator::Validate)]
#[allow(dead_code)]
struct ReportClientTimingsBodyValidator<'a> {
    #[validate(nested)]
    body: &'a models::ClientTimingReport,
}

#[tracing::instrument(skip_all)]
fn report_client_timings_validation(body: models::ClientTimingReport) -> std::result::Result<(models::ClientTimingReport,), ValidationErrors> {
    let b = ReportClientTimingsBodyValidator { body: &body };
    b.validate()?;

    Ok((body,))
}
/// ReportClientTimings - POST /api/v1/performance/client-timings
#[tracing::instrument(skip_all)]
async fn report_client_timings<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    State(api_impl): State<I>,
    Json(body): Json<models::ClientTimingReport>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::performance::Performance<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || report_client_timings_validation(body)).await.unwrap();

    let Ok((body,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().report_client_timings(&method, &host, &cookies, &claims, &body).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::performance::ReportClientTimingsResponse::Status204_TheTimingsWereAccepted => {
                let mut response = response.status(204);
                response.body(Body::empty())
            }
            apis::performance::ReportClientTimingsResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::performance::ReportClientTimingsResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::performance::ReportClientTimingsResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn get_preferences_validation() -> std::result::Result<(), ValidationErrors> {
    Ok(())
}
/// GetPreferences - GET /api/v1/preferences
#[tracing::instrument(skip_all)]
async fn get_preferences<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::preferences::Preferences<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || get_preferences_validation()).await.unwrap();

    let Ok(()) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().get_preferences(&method, &host, &cookies, &claims).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::preferences::GetPreferencesResponse::Status200_DisplayPreferences(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::preferences::GetPreferencesResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::preferences::GetPreferencesResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[derive(validator::Validate)]
#[allow(dead_code)]
struct PutPreferencesBodyValidator<'a> {
    #[validate(nested)]
    body: &'a models::Preferences,
}

#[tracing::instrument(skip_all)]
fn put_preferences_validation(body: models::Preferences) -> std::result::Result<(models::Preferences,), ValidationErrors> {
    let b = PutPreferencesBodyValidator { body: &body };
    b.validate()?;

    Ok((body,))
}
/// PutPreferences - PUT /api/v1/preferences
#[tracing::instrument(skip_all)]
async fn put_preferences<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    State(api_impl): State<I>,
    Json(body): Json<models::Preferences>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::preferences::Preferences<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || put_preferences_validation(body)).await.unwrap();

    let Ok((body,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().put_preferences(&method, &host, &cookies, &claims, &body).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::preferences::PutPreferencesResponse::Status200_TheSavedDisplayPreferences(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::preferences::PutPreferencesResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::preferences::PutPreferencesResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::preferences::PutPreferencesResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn get_rating_validation(
    path_params: models::GetRatingPathParams,
    query_params: models::GetRatingQueryParams,
) -> std::result::Result<(models::GetRatingPathParams, models::GetRatingQueryParams), ValidationErrors> {
    path_params.validate()?;
    query_params.validate()?;

    Ok((path_params, query_params))
}
/// GetRating - GET /api/v1/libraries/{libraryId}/rating
#[tracing::instrument(skip_all)]
async fn get_rating<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::GetRatingPathParams>,
    QueryExtra(query_params): QueryExtra<models::GetRatingQueryParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::rating::Rating<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || get_rating_validation(path_params, query_params)).await.unwrap();

    let Ok((path_params, query_params)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().get_rating(&method, &host, &cookies, &claims, &path_params, &query_params).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::rating::GetRatingResponse::Status200_TheRating(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::rating::GetRatingResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::rating::GetRatingResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::rating::GetRatingResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::rating::GetRatingResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::rating::GetRatingResponse::Status504_ExtractionOrThumbnailGenerationTimedOut(body) => {
                let mut response = response.status(504);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[derive(validator::Validate)]
#[allow(dead_code)]
struct PutRatingBodyValidator<'a> {
    #[validate(nested)]
    body: &'a models::PutRatingRequest,
}

#[tracing::instrument(skip_all)]
fn put_rating_validation(
    path_params: models::PutRatingPathParams,
    body: models::PutRatingRequest,
) -> std::result::Result<(models::PutRatingPathParams, models::PutRatingRequest), ValidationErrors> {
    path_params.validate()?;
    let b = PutRatingBodyValidator { body: &body };
    b.validate()?;

    Ok((path_params, body))
}
/// PutRating - PUT /api/v1/libraries/{libraryId}/rating
#[tracing::instrument(skip_all)]
async fn put_rating<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::PutRatingPathParams>,
    State(api_impl): State<I>,
    Json(body): Json<models::PutRatingRequest>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::rating::Rating<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || put_rating_validation(path_params, body)).await.unwrap();

    let Ok((path_params, body)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().put_rating(&method, &host, &cookies, &claims, &path_params, &body).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::rating::PutRatingResponse::Status200_TheSavedRating(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::rating::PutRatingResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::rating::PutRatingResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::rating::PutRatingResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::rating::PutRatingResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::rating::PutRatingResponse::Status504_ExtractionOrThumbnailGenerationTimedOut(body) => {
                let mut response = response.status(504);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn list_search_entries_validation(query_params: models::ListSearchEntriesQueryParams) -> std::result::Result<(models::ListSearchEntriesQueryParams,), ValidationErrors> {
    query_params.validate()?;

    Ok((query_params,))
}
/// ListSearchEntries - GET /api/v1/search/entries
#[tracing::instrument(skip_all)]
async fn list_search_entries<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    QueryExtra(query_params): QueryExtra<models::ListSearchEntriesQueryParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::search::Search<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || list_search_entries_validation(query_params)).await.unwrap();

    let Ok((query_params,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().list_search_entries(&method, &host, &cookies, &claims, &query_params).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::search::ListSearchEntriesResponse::Status200_MatchingEntriesOfTheLibraries(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::search::ListSearchEntriesResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::search::ListSearchEntriesResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::search::ListSearchEntriesResponse::Status403_TheRequestedResourceCannotBeAccessedOrWritten(body) => {
                let mut response = response.status(403);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::search::ListSearchEntriesResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::search::ListSearchEntriesResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::search::ListSearchEntriesResponse::Status504_ExtractionOrThumbnailGenerationTimedOut(body) => {
                let mut response = response.status(504);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[derive(validator::Validate)]
#[allow(dead_code)]
struct CreateInitialSetupBodyValidator<'a> {
    #[validate(nested)]
    body: &'a models::InitialSetupRequest,
}

#[tracing::instrument(skip_all)]
fn create_initial_setup_validation(body: models::InitialSetupRequest) -> std::result::Result<(models::InitialSetupRequest,), ValidationErrors> {
    let b = CreateInitialSetupBodyValidator { body: &body };
    b.validate()?;

    Ok((body,))
}
/// CreateInitialSetup - POST /api/v1/setup
#[tracing::instrument(skip_all)]
async fn create_initial_setup<I, A, E>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    State(api_impl): State<I>,
    Json(body): Json<models::InitialSetupRequest>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::setup::Setup<E> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || create_initial_setup_validation(body)).await.unwrap();

    let Ok((body,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().create_initial_setup(&method, &host, &cookies, &body).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::setup::CreateInitialSetupResponse::Status201_OwnerAndAuthenticatedSessionCreated { body, set_cookie } => {
                if let Some(set_cookie) = set_cookie {
                    let set_cookie = match header::IntoHeaderValue(set_cookie).try_into() {
                        Ok(val) => val,
                        Err(e) => {
                            return Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .body(Body::from(format!("An internal server error occurred handling set_cookie header - {e}")))
                                .map_err(|e| {
                                    error!(error = ?e);
                                    StatusCode::INTERNAL_SERVER_ERROR
                                });
                        }
                    };

                    {
                        let mut response_headers = response.headers_mut().unwrap();
                        response_headers.insert(HeaderName::from_static("set-cookie"), set_cookie);
                    }
                }
                let mut response = response.status(201);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::setup::CreateInitialSetupResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::setup::CreateInitialSetupResponse::Status409_TheInitialOwnerHasAlreadyBeenRegistered(body) => {
                let mut response = response.status(409);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::setup::CreateInitialSetupResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn get_setup_status_validation() -> std::result::Result<(), ValidationErrors> {
    Ok(())
}
/// GetSetupStatus - GET /api/v1/setup/status
#[tracing::instrument(skip_all)]
async fn get_setup_status<I, A, E>(method: Method, TypedHeader(host): TypedHeader<Host>, cookies: CookieJar, State(api_impl): State<I>) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::setup::Setup<E> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || get_setup_status_validation()).await.unwrap();

    let Ok(()) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().get_setup_status(&method, &host, &cookies).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::setup::GetSetupStatusResponse::Status200_CurrentSetupStatus(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::setup::GetSetupStatusResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn get_entry_tags_validation(
    path_params: models::GetEntryTagsPathParams,
    query_params: models::GetEntryTagsQueryParams,
) -> std::result::Result<(models::GetEntryTagsPathParams, models::GetEntryTagsQueryParams), ValidationErrors> {
    path_params.validate()?;
    query_params.validate()?;

    Ok((path_params, query_params))
}
/// GetEntryTags - GET /api/v1/libraries/{libraryId}/tags
#[tracing::instrument(skip_all)]
async fn get_entry_tags<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::GetEntryTagsPathParams>,
    QueryExtra(query_params): QueryExtra<models::GetEntryTagsQueryParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::tags::Tags<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || get_entry_tags_validation(path_params, query_params)).await.unwrap();

    let Ok((path_params, query_params)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().get_entry_tags(&method, &host, &cookies, &claims, &path_params, &query_params).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::tags::GetEntryTagsResponse::Status200_TheTags(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::tags::GetEntryTagsResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::tags::GetEntryTagsResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::tags::GetEntryTagsResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::tags::GetEntryTagsResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::tags::GetEntryTagsResponse::Status504_ExtractionOrThumbnailGenerationTimedOut(body) => {
                let mut response = response.status(504);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn list_tags_validation() -> std::result::Result<(), ValidationErrors> {
    Ok(())
}
/// ListTags - GET /api/v1/tags
#[tracing::instrument(skip_all)]
async fn list_tags<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::tags::Tags<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || list_tags_validation()).await.unwrap();

    let Ok(()) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().list_tags(&method, &host, &cookies, &claims).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::tags::ListTagsResponse::Status200_TheWords(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::tags::ListTagsResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::tags::ListTagsResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[derive(validator::Validate)]
#[allow(dead_code)]
struct PutEntryTagsBodyValidator<'a> {
    #[validate(nested)]
    body: &'a models::PutEntryTagsRequest,
}

#[tracing::instrument(skip_all)]
fn put_entry_tags_validation(
    path_params: models::PutEntryTagsPathParams,
    body: models::PutEntryTagsRequest,
) -> std::result::Result<(models::PutEntryTagsPathParams, models::PutEntryTagsRequest), ValidationErrors> {
    path_params.validate()?;
    let b = PutEntryTagsBodyValidator { body: &body };
    b.validate()?;

    Ok((path_params, body))
}
/// PutEntryTags - PUT /api/v1/libraries/{libraryId}/tags
#[tracing::instrument(skip_all)]
async fn put_entry_tags<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::PutEntryTagsPathParams>,
    State(api_impl): State<I>,
    Json(body): Json<models::PutEntryTagsRequest>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::tags::Tags<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || put_entry_tags_validation(path_params, body)).await.unwrap();

    let Ok((path_params, body)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().put_entry_tags(&method, &host, &cookies, &claims, &path_params, &body).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::tags::PutEntryTagsResponse::Status200_TheSavedTags(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::tags::PutEntryTagsResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::tags::PutEntryTagsResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::tags::PutEntryTagsResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::tags::PutEntryTagsResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::tags::PutEntryTagsResponse::Status504_ExtractionOrThumbnailGenerationTimedOut(body) => {
                let mut response = response.status(504);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn get_thumbnail_validation(
    path_params: models::GetThumbnailPathParams,
    query_params: models::GetThumbnailQueryParams,
) -> std::result::Result<(models::GetThumbnailPathParams, models::GetThumbnailQueryParams), ValidationErrors> {
    path_params.validate()?;
    query_params.validate()?;

    Ok((path_params, query_params))
}
/// GetThumbnail - GET /api/v1/libraries/{libraryId}/thumbnails
#[tracing::instrument(skip_all)]
async fn get_thumbnail<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::GetThumbnailPathParams>,
    QueryExtra(query_params): QueryExtra<models::GetThumbnailQueryParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::thumbnails::Thumbnails<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || get_thumbnail_validation(path_params, query_params)).await.unwrap();

    let Ok((path_params, query_params)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().get_thumbnail(&method, &host, &cookies, &claims, &path_params, &query_params).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::thumbnails::GetThumbnailResponse::Status200_ThumbnailImage {
                body,
                cache_control,
                e_tag,
                pxna_frame_count,
            } => {
                if let Some(cache_control) = cache_control {
                    let cache_control = match header::IntoHeaderValue(cache_control).try_into() {
                        Ok(val) => val,
                        Err(e) => {
                            return Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .body(Body::from(format!("An internal server error occurred handling cache_control header - {e}")))
                                .map_err(|e| {
                                    error!(error = ?e);
                                    StatusCode::INTERNAL_SERVER_ERROR
                                });
                        }
                    };

                    {
                        let mut response_headers = response.headers_mut().unwrap();
                        response_headers.insert(HeaderName::from_static("cache-control"), cache_control);
                    }
                }
                if let Some(e_tag) = e_tag {
                    let e_tag = match header::IntoHeaderValue(e_tag).try_into() {
                        Ok(val) => val,
                        Err(e) => {
                            return Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .body(Body::from(format!("An internal server error occurred handling e_tag header - {e}")))
                                .map_err(|e| {
                                    error!(error = ?e);
                                    StatusCode::INTERNAL_SERVER_ERROR
                                });
                        }
                    };

                    {
                        let mut response_headers = response.headers_mut().unwrap();
                        response_headers.insert(HeaderName::from_static("etag"), e_tag);
                    }
                }
                if let Some(pxna_frame_count) = pxna_frame_count {
                    let pxna_frame_count = match header::IntoHeaderValue(pxna_frame_count).try_into() {
                        Ok(val) => val,
                        Err(e) => {
                            return Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .body(Body::from(format!("An internal server error occurred handling pxna_frame_count header - {e}")))
                                .map_err(|e| {
                                    error!(error = ?e);
                                    StatusCode::INTERNAL_SERVER_ERROR
                                });
                        }
                    };

                    {
                        let mut response_headers = response.headers_mut().unwrap();
                        response_headers.insert(HeaderName::from_static("pxna-frame-count"), pxna_frame_count);
                    }
                }
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("image/webp"));
                }

                let body_content = body.0;
                response.body(Body::from(body_content))
            }
            apis::thumbnails::GetThumbnailResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::thumbnails::GetThumbnailResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::thumbnails::GetThumbnailResponse::Status403_TheRequestedResourceCannotBeAccessedOrWritten(body) => {
                let mut response = response.status(403);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::thumbnails::GetThumbnailResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::thumbnails::GetThumbnailResponse::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(body) => {
                let mut response = response.status(409);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::thumbnails::GetThumbnailResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::thumbnails::GetThumbnailResponse::Status504_ExtractionOrThumbnailGenerationTimedOut(body) => {
                let mut response = response.status(504);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn get_timeline_status_validation(query_params: models::GetTimelineStatusQueryParams) -> std::result::Result<(models::GetTimelineStatusQueryParams,), ValidationErrors> {
    query_params.validate()?;

    Ok((query_params,))
}
/// GetTimelineStatus - GET /api/v1/timeline/status
#[tracing::instrument(skip_all)]
async fn get_timeline_status<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    QueryExtra(query_params): QueryExtra<models::GetTimelineStatusQueryParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::timeline::Timeline<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || get_timeline_status_validation(query_params)).await.unwrap();

    let Ok((query_params,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().get_timeline_status(&method, &host, &cookies, &claims, &query_params).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::timeline::GetTimelineStatusResponse::Status200_StateOfTheScans(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::timeline::GetTimelineStatusResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::timeline::GetTimelineStatusResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::timeline::GetTimelineStatusResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::timeline::GetTimelineStatusResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn list_timeline_entries_validation(query_params: models::ListTimelineEntriesQueryParams) -> std::result::Result<(models::ListTimelineEntriesQueryParams,), ValidationErrors> {
    query_params.validate()?;

    Ok((query_params,))
}
/// ListTimelineEntries - GET /api/v1/timeline/entries
#[tracing::instrument(skip_all)]
async fn list_timeline_entries<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    QueryExtra(query_params): QueryExtra<models::ListTimelineEntriesQueryParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::timeline::Timeline<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || list_timeline_entries_validation(query_params)).await.unwrap();

    let Ok((query_params,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().list_timeline_entries(&method, &host, &cookies, &claims, &query_params).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::timeline::ListTimelineEntriesResponse::Status200_TimelineEntries(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::timeline::ListTimelineEntriesResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::timeline::ListTimelineEntriesResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::timeline::ListTimelineEntriesResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::timeline::ListTimelineEntriesResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn list_timeline_months_validation(query_params: models::ListTimelineMonthsQueryParams) -> std::result::Result<(models::ListTimelineMonthsQueryParams,), ValidationErrors> {
    query_params.validate()?;

    Ok((query_params,))
}
/// ListTimelineMonths - GET /api/v1/timeline/months
#[tracing::instrument(skip_all)]
async fn list_timeline_months<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    QueryExtra(query_params): QueryExtra<models::ListTimelineMonthsQueryParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::timeline::Timeline<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || list_timeline_months_validation(query_params)).await.unwrap();

    let Ok((query_params,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().list_timeline_months(&method, &host, &cookies, &claims, &query_params).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::timeline::ListTimelineMonthsResponse::Status200_EntryCountsPerMonth(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::timeline::ListTimelineMonthsResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::timeline::ListTimelineMonthsResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::timeline::ListTimelineMonthsResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::timeline::ListTimelineMonthsResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[derive(validator::Validate)]
#[allow(dead_code)]
struct RefreshTimelineBodyValidator<'a> {
    #[validate(nested)]
    body: &'a models::TimelineRefreshRequest,
}

#[tracing::instrument(skip_all)]
fn refresh_timeline_validation(body: models::TimelineRefreshRequest) -> std::result::Result<(models::TimelineRefreshRequest,), ValidationErrors> {
    let b = RefreshTimelineBodyValidator { body: &body };
    b.validate()?;

    Ok((body,))
}
/// RefreshTimeline - POST /api/v1/timeline/refresh
#[tracing::instrument(skip_all)]
async fn refresh_timeline<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    State(api_impl): State<I>,
    Json(body): Json<models::TimelineRefreshRequest>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::timeline::Timeline<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || refresh_timeline_validation(body)).await.unwrap();

    let Ok((body,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().refresh_timeline(&method, &host, &cookies, &claims, &body).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::timeline::RefreshTimelineResponse::Status202_TheScansAreRunning(body) => {
                let mut response = response.status(202);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::timeline::RefreshTimelineResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::timeline::RefreshTimelineResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::timeline::RefreshTimelineResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::timeline::RefreshTimelineResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn commit_upload_validation(path_params: models::CommitUploadPathParams) -> std::result::Result<(models::CommitUploadPathParams,), ValidationErrors> {
    path_params.validate()?;

    Ok((path_params,))
}
/// CommitUpload - POST /api/v1/libraries/{libraryId}/uploads/{uploadId}/commit
#[tracing::instrument(skip_all)]
async fn commit_upload<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::CommitUploadPathParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::uploads::Uploads<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || commit_upload_validation(path_params)).await.unwrap();

    let Ok((path_params,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().commit_upload(&method, &host, &cookies, &claims, &path_params).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::uploads::CommitUploadResponse::Status201_FileCreatedAtTheTargetPath(body) => {
                let mut response = response.status(201);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::uploads::CommitUploadResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::uploads::CommitUploadResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::uploads::CommitUploadResponse::Status403_TheRequestedResourceCannotBeAccessedOrWritten(body) => {
                let mut response = response.status(403);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::uploads::CommitUploadResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::uploads::CommitUploadResponse::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(body) => {
                let mut response = response.status(409);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::uploads::CommitUploadResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[derive(validator::Validate)]
#[allow(dead_code)]
struct CreateUploadBodyValidator<'a> {
    #[validate(nested)]
    body: &'a models::UploadCreateRequest,
}

#[tracing::instrument(skip_all)]
fn create_upload_validation(
    path_params: models::CreateUploadPathParams,
    body: models::UploadCreateRequest,
) -> std::result::Result<(models::CreateUploadPathParams, models::UploadCreateRequest), ValidationErrors> {
    path_params.validate()?;
    let b = CreateUploadBodyValidator { body: &body };
    b.validate()?;

    Ok((path_params, body))
}
/// CreateUpload - POST /api/v1/libraries/{libraryId}/uploads
#[tracing::instrument(skip_all)]
async fn create_upload<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::CreateUploadPathParams>,
    State(api_impl): State<I>,
    Json(body): Json<models::UploadCreateRequest>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::uploads::Uploads<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || create_upload_validation(path_params, body)).await.unwrap();

    let Ok((path_params, body)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().create_upload(&method, &host, &cookies, &claims, &path_params, &body).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::uploads::CreateUploadResponse::Status201_UploadSessionCreated(body) => {
                let mut response = response.status(201);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::uploads::CreateUploadResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::uploads::CreateUploadResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::uploads::CreateUploadResponse::Status403_TheRequestedResourceCannotBeAccessedOrWritten(body) => {
                let mut response = response.status(403);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::uploads::CreateUploadResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::uploads::CreateUploadResponse::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(body) => {
                let mut response = response.status(409);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::uploads::CreateUploadResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::uploads::CreateUploadResponse::Status507_TheDaemonDoesNotHaveEnoughDiskSpace(body) => {
                let mut response = response.status(507);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn delete_upload_validation(path_params: models::DeleteUploadPathParams) -> std::result::Result<(models::DeleteUploadPathParams,), ValidationErrors> {
    path_params.validate()?;

    Ok((path_params,))
}
/// DeleteUpload - DELETE /api/v1/libraries/{libraryId}/uploads/{uploadId}
#[tracing::instrument(skip_all)]
async fn delete_upload<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::DeleteUploadPathParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::uploads::Uploads<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || delete_upload_validation(path_params)).await.unwrap();

    let Ok((path_params,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().delete_upload(&method, &host, &cookies, &claims, &path_params).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::uploads::DeleteUploadResponse::Status204_UploadSessionDiscarded => {
                let mut response = response.status(204);
                response.body(Body::empty())
            }
            apis::uploads::DeleteUploadResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::uploads::DeleteUploadResponse::Status403_TheRequestedResourceCannotBeAccessedOrWritten(body) => {
                let mut response = response.status(403);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::uploads::DeleteUploadResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::uploads::DeleteUploadResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn get_upload_validation(path_params: models::GetUploadPathParams) -> std::result::Result<(models::GetUploadPathParams,), ValidationErrors> {
    path_params.validate()?;

    Ok((path_params,))
}
/// GetUpload - GET /api/v1/libraries/{libraryId}/uploads/{uploadId}
#[tracing::instrument(skip_all)]
async fn get_upload<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::GetUploadPathParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::uploads::Uploads<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || get_upload_validation(path_params)).await.unwrap();

    let Ok((path_params,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().get_upload(&method, &host, &cookies, &claims, &path_params).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::uploads::GetUploadResponse::Status200_UploadSessionState(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::uploads::GetUploadResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::uploads::GetUploadResponse::Status403_TheRequestedResourceCannotBeAccessedOrWritten(body) => {
                let mut response = response.status(403);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::uploads::GetUploadResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::uploads::GetUploadResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[derive(validator::Validate)]
#[allow(dead_code)]
struct WriteUploadContentBodyValidator<'a> {
    body: &'a [u8],
}

#[tracing::instrument(skip_all)]
fn write_upload_content_validation(
    path_params: models::WriteUploadContentPathParams,
    query_params: models::WriteUploadContentQueryParams,
    body: Bytes,
) -> std::result::Result<(models::WriteUploadContentPathParams, models::WriteUploadContentQueryParams, Bytes), ValidationErrors> {
    path_params.validate()?;
    query_params.validate()?;

    Ok((path_params, query_params, body))
}
/// WriteUploadContent - PUT /api/v1/libraries/{libraryId}/uploads/{uploadId}/content
#[tracing::instrument(skip_all)]
async fn write_upload_content<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::WriteUploadContentPathParams>,
    QueryExtra(query_params): QueryExtra<models::WriteUploadContentQueryParams>,
    State(api_impl): State<I>,
    body: Bytes,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::uploads::Uploads<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || write_upload_content_validation(path_params, query_params, body))
        .await
        .unwrap();

    let Ok((path_params, query_params, body)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl
        .as_ref()
        .write_upload_content(&method, &host, &cookies, &claims, &path_params, &query_params, &body)
        .await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::uploads::WriteUploadContentResponse::Status204_ChunkAccepted => {
                let mut response = response.status(204);
                response.body(Body::empty())
            }
            apis::uploads::WriteUploadContentResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::uploads::WriteUploadContentResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::uploads::WriteUploadContentResponse::Status403_TheRequestedResourceCannotBeAccessedOrWritten(body) => {
                let mut response = response.status(403);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::uploads::WriteUploadContentResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::uploads::WriteUploadContentResponse::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(body) => {
                let mut response = response.status(409);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::uploads::WriteUploadContentResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::uploads::WriteUploadContentResponse::Status507_TheDaemonDoesNotHaveEnoughDiskSpace(body) => {
                let mut response = response.status(507);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[derive(validator::Validate)]
#[allow(dead_code)]
struct CreateVideoSessionBodyValidator<'a> {
    #[validate(nested)]
    body: &'a models::VideoSessionCreateRequest,
}

#[tracing::instrument(skip_all)]
fn create_video_session_validation(
    path_params: models::CreateVideoSessionPathParams,
    body: models::VideoSessionCreateRequest,
) -> std::result::Result<(models::CreateVideoSessionPathParams, models::VideoSessionCreateRequest), ValidationErrors> {
    path_params.validate()?;
    let b = CreateVideoSessionBodyValidator { body: &body };
    b.validate()?;

    Ok((path_params, body))
}
/// CreateVideoSession - POST /api/v1/libraries/{libraryId}/video/sessions
#[tracing::instrument(skip_all)]
async fn create_video_session<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::CreateVideoSessionPathParams>,
    State(api_impl): State<I>,
    Json(body): Json<models::VideoSessionCreateRequest>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::video::Video<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || create_video_session_validation(path_params, body)).await.unwrap();

    let Ok((path_params, body)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().create_video_session(&method, &host, &cookies, &claims, &path_params, &body).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::video::CreateVideoSessionResponse::Status201_VideoSessionCreated(body) => {
                let mut response = response.status(201);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::video::CreateVideoSessionResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::video::CreateVideoSessionResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::video::CreateVideoSessionResponse::Status403_TheRequestedResourceCannotBeAccessedOrWritten(body) => {
                let mut response = response.status(403);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::video::CreateVideoSessionResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::video::CreateVideoSessionResponse::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(body) => {
                let mut response = response.status(409);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::video::CreateVideoSessionResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::video::CreateVideoSessionResponse::Status504_ExtractionOrThumbnailGenerationTimedOut(body) => {
                let mut response = response.status(504);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn delete_video_session_validation(path_params: models::DeleteVideoSessionPathParams) -> std::result::Result<(models::DeleteVideoSessionPathParams,), ValidationErrors> {
    path_params.validate()?;

    Ok((path_params,))
}
/// DeleteVideoSession - DELETE /api/v1/libraries/{libraryId}/video/sessions/{sessionId}
#[tracing::instrument(skip_all)]
async fn delete_video_session<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::DeleteVideoSessionPathParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::video::Video<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || delete_video_session_validation(path_params)).await.unwrap();

    let Ok((path_params,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().delete_video_session(&method, &host, &cookies, &claims, &path_params).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::video::DeleteVideoSessionResponse::Status204_VideoSessionEnded => {
                let mut response = response.status(204);
                response.body(Body::empty())
            }
            apis::video::DeleteVideoSessionResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::video::DeleteVideoSessionResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::video::DeleteVideoSessionResponse::Status403_TheRequestedResourceCannotBeAccessedOrWritten(body) => {
                let mut response = response.status(403);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::video::DeleteVideoSessionResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::video::DeleteVideoSessionResponse::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(body) => {
                let mut response = response.status(409);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::video::DeleteVideoSessionResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::video::DeleteVideoSessionResponse::Status504_ExtractionOrThumbnailGenerationTimedOut(body) => {
                let mut response = response.status(504);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn get_media_info_validation(
    path_params: models::GetMediaInfoPathParams,
    query_params: models::GetMediaInfoQueryParams,
) -> std::result::Result<(models::GetMediaInfoPathParams, models::GetMediaInfoQueryParams), ValidationErrors> {
    path_params.validate()?;
    query_params.validate()?;

    Ok((path_params, query_params))
}
/// GetMediaInfo - GET /api/v1/libraries/{libraryId}/media-info
#[tracing::instrument(skip_all)]
async fn get_media_info<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::GetMediaInfoPathParams>,
    QueryExtra(query_params): QueryExtra<models::GetMediaInfoQueryParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::video::Video<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || get_media_info_validation(path_params, query_params)).await.unwrap();

    let Ok((path_params, query_params)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().get_media_info(&method, &host, &cookies, &claims, &path_params, &query_params).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::video::GetMediaInfoResponse::Status200_MediaInformationOfAnImageOrAVideo(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::video::GetMediaInfoResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::video::GetMediaInfoResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::video::GetMediaInfoResponse::Status403_TheRequestedResourceCannotBeAccessedOrWritten(body) => {
                let mut response = response.status(403);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::video::GetMediaInfoResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::video::GetMediaInfoResponse::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(body) => {
                let mut response = response.status(409);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::video::GetMediaInfoResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::video::GetMediaInfoResponse::Status504_ExtractionOrThumbnailGenerationTimedOut(body) => {
                let mut response = response.status(504);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn get_video_playlist_validation(path_params: models::GetVideoPlaylistPathParams) -> std::result::Result<(models::GetVideoPlaylistPathParams,), ValidationErrors> {
    path_params.validate()?;

    Ok((path_params,))
}
/// GetVideoPlaylist - GET /api/v1/libraries/{libraryId}/video/sessions/{sessionId}/playlist.m3u8
#[tracing::instrument(skip_all)]
async fn get_video_playlist<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::GetVideoPlaylistPathParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::video::Video<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || get_video_playlist_validation(path_params)).await.unwrap();

    let Ok((path_params,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().get_video_playlist(&method, &host, &cookies, &claims, &path_params).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::video::GetVideoPlaylistResponse::Status200_VODPlaylist(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/vnd.apple.mpegurl"));
                }

                let body_content = body;
                response.body(Body::from(body_content))
            }
            apis::video::GetVideoPlaylistResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::video::GetVideoPlaylistResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::video::GetVideoPlaylistResponse::Status403_TheRequestedResourceCannotBeAccessedOrWritten(body) => {
                let mut response = response.status(403);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::video::GetVideoPlaylistResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::video::GetVideoPlaylistResponse::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(body) => {
                let mut response = response.status(409);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::video::GetVideoPlaylistResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::video::GetVideoPlaylistResponse::Status504_ExtractionOrThumbnailGenerationTimedOut(body) => {
                let mut response = response.status(504);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn get_video_segment_validation(path_params: models::GetVideoSegmentPathParams) -> std::result::Result<(models::GetVideoSegmentPathParams,), ValidationErrors> {
    path_params.validate()?;

    Ok((path_params,))
}
/// GetVideoSegment - GET /api/v1/libraries/{libraryId}/video/sessions/{sessionId}/segments/{segmentName}
#[tracing::instrument(skip_all)]
async fn get_video_segment<I, A, E, C>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::GetVideoSegmentPathParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::video::Video<E, Claims = C> + apis::CookieAuthentication<Claims = C> + apis::ApiAuthBasic<Claims = C> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // Authentication
    let claims_in_cookie = api_impl.as_ref().extract_claims_from_cookie(&cookies, "__Host-pxna_session").await;
    let claims_in_auth_header = api_impl
        .as_ref()
        .extract_claims_from_auth_header(apis::BasicAuthKind::Bearer, &headers, "authorization")
        .await;
    let claims = None.or(claims_in_cookie).or(claims_in_auth_header);
    let Some(claims) = claims else {
        return response_with_status_code_only(StatusCode::UNAUTHORIZED);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || get_video_segment_validation(path_params)).await.unwrap();

    let Ok((path_params,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().get_video_segment(&method, &host, &cookies, &claims, &path_params).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::video::GetVideoSegmentResponse::Status200_MPEGTransportStreamSegment(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("video/mp2t"));
                }

                let body_content = body.0;
                response.body(Body::from(body_content))
            }
            apis::video::GetVideoSegmentResponse::Status400_InvalidRequest(body) => {
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::video::GetVideoSegmentResponse::Status401_MissingOrInvalidAuthentication(body) => {
                let mut response = response.status(401);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::video::GetVideoSegmentResponse::Status403_TheRequestedResourceCannotBeAccessedOrWritten(body) => {
                let mut response = response.status(403);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::video::GetVideoSegmentResponse::Status404_ResourceNotFound(body) => {
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::video::GetVideoSegmentResponse::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(body) => {
                let mut response = response.status(409);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::video::GetVideoSegmentResponse::Status500_InternalServerError(body) => {
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::video::GetVideoSegmentResponse::Status504_ExtractionOrThumbnailGenerationTimedOut(body) => {
                let mut response = response.status(504);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl.as_ref().handle_error(&method, &host, &cookies, why).await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[allow(dead_code)]
#[inline]
fn response_with_status_code_only(code: StatusCode) -> Result<Response, StatusCode> {
    Response::builder().status(code).body(Body::empty()).map_err(|_| code)
}
