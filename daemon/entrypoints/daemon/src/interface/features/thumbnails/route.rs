use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, Query, State, rejection::QueryRejection},
    response::Response,
};
use axum_extra::extract::CookieJar;
use http::{
    HeaderMap, HeaderValue, StatusCode,
    header::{CACHE_CONTROL, CONTENT_TYPE, ETAG},
};
use tracing::{Instrument as _, Span, field::Empty};
use validator::Validate as _;

use omnius_pxna_engine::{
    model::{ImageAnchorPositionType, ImageFormatType, ImageResizeType},
    perf_span,
    service::{Thumbnail, ThumbnailRequest, ThumbnailVariant},
};
use omnius_pxna_interface::models;

use crate::{error::ApiError, interface::session::SessionAuth, prelude::*, state::DaemonState};

/// thumbnail API の route。
///
/// 応答の `Content-Type` を要求した形式に合わせるため、生成されたルータの外で返す（design.md §11.1）。
pub struct ThumbnailRoute;

impl ThumbnailRoute {
    pub const PATH: &'static str = "/api/v1/libraries/{library_id}/thumbnails";

    /// `width` と `height` の契約の既定値。
    const DEFAULT_SIZE: u32 = 256;

    pub async fn get(
        State(state): State<Arc<DaemonState>>,
        Path(library_id): Path<String>,
        cookies: CookieJar,
        headers: HeaderMap,
        query: std::result::Result<Query<models::GetThumbnailQueryParams>, QueryRejection>,
    ) -> Response {
        if SessionAuth::authenticate(&state, &cookies, &headers).await.is_none() {
            return ApiError::unauthorized().into_response();
        }

        let span = perf_span!("thumbnail_request", library_id, path = Empty, variant = Empty, ok = Empty, bytes = Empty);
        let result = async {
            let Query(params) = query.map_err(|rejection| Error::new(ErrorKind::InvalidInput).with_message(rejection.body_text()))?;
            let request = Self::request(&params, state.conf.thumbnail.format)?;
            Span::current().record("path", &params.path).record("variant", tracing::field::debug(request.variant));
            let source = state
                .library
                .thumbnail_source(&library_id, &params.path)
                .instrument(perf_span!("thumbnail_resolve"))
                .await?;
            Ok::<_, Error>(state.thumbnails.generate_resolved(source, &request).await?)
        }
        .instrument(span.clone())
        .await;
        span.record("ok", result.is_ok())
            .record("bytes", result.as_ref().map_or(0, |thumbnail| thumbnail.bytes.len()));
        drop(span);

        match result {
            Ok(thumbnail) => Self::response(thumbnail),
            Err(error) => ApiError::from(error).into_response(),
        }
    }

    /// クエリを、契約の既定値を補った指定へ写す。`format` を省略した要求には `default_format` を使う。
    fn request(params: &models::GetThumbnailQueryParams, default_format: ImageFormatType) -> Result<ThumbnailRequest> {
        params.validate().map_err(|error| Error::new(ErrorKind::InvalidInput).with_message(error.to_string()))?;

        let variant = match params.variant.unwrap_or(models::ThumbnailVariant::Thumbnail) {
            models::ThumbnailVariant::Thumbnail => ThumbnailVariant::Thumbnail,
            models::ThumbnailVariant::Display => ThumbnailVariant::Display,
            models::ThumbnailVariant::Sprite => ThumbnailVariant::Sprite,
        };
        // 表示用画像は、幅と高さを指定しなければ、表示用の段階の最大へ収める。
        let default_size = (variant != ThumbnailVariant::Display).then_some(Self::DEFAULT_SIZE);

        Ok(ThumbnailRequest {
            variant,
            width: params.width.or(default_size),
            height: params.height.or(default_size),
            format: match params.format {
                Some(models::ImageFormat::Webp) => ImageFormatType::Webp,
                Some(models::ImageFormat::Jpeg) => ImageFormatType::Jpeg,
                Some(models::ImageFormat::Png) => ImageFormatType::Png,
                None => default_format,
            },
            resize: match params.resize.unwrap_or(models::ImageResize::Crop) {
                models::ImageResize::Crop => ImageResizeType::Crop,
                models::ImageResize::Pad => ImageResizeType::Pad,
                models::ImageResize::Manual => ImageResizeType::Manual,
                models::ImageResize::Fit => ImageResizeType::Fit,
            },
            anchor: match params.anchor.unwrap_or(models::ImageAnchor::Center) {
                models::ImageAnchor::Bottom => ImageAnchorPositionType::Bottom,
                models::ImageAnchor::BottomLeft => ImageAnchorPositionType::BottomLeft,
                models::ImageAnchor::BottomRight => ImageAnchorPositionType::BottomRight,
                models::ImageAnchor::Center => ImageAnchorPositionType::Center,
                models::ImageAnchor::Left => ImageAnchorPositionType::Left,
                models::ImageAnchor::Right => ImageAnchorPositionType::Right,
                models::ImageAnchor::Top => ImageAnchorPositionType::Top,
                models::ImageAnchor::TopLeft => ImageAnchorPositionType::TopLeft,
                models::ImageAnchor::TopRight => ImageAnchorPositionType::TopRight,
            },
        })
    }

    fn response(thumbnail: Thumbnail) -> Response {
        let content_type = match thumbnail.format {
            ImageFormatType::Webp => "image/webp",
            ImageFormatType::Png => "image/png",
            _ => "image/jpeg",
        };

        let mut response = Response::new(Body::from(thumbnail.bytes));
        *response.status_mut() = StatusCode::OK;
        let headers = response.headers_mut();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
        // key は file の更新を含むため、差し替えられたファイルは新しい ETag になる。URL は変わらないので、期限は 1 日に留める。
        headers.insert(CACHE_CONTROL, HeaderValue::from_static("private, max-age=86400"));
        if let Some(etag) = thumbnail.key.and_then(|key| HeaderValue::from_str(&format!("\"{key}\"")).ok()) {
            headers.insert(ETAG, etag);
        }
        if let Some(count) = thumbnail.frame_count {
            headers.insert("pxna-frame-count", HeaderValue::from(count));
        }
        response
    }
}
