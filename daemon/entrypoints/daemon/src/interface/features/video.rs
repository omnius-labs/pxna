use async_trait::async_trait;
use axum_extra::extract::CookieJar;
use headers::Host;
use http::Method;

use omnius_pxna_engine::service::{MediaKind, WorkClass, WorkPools, WorkPriorityPool};
use omnius_pxna_interface::{apis, models, types::ByteArray};

use crate::{
    error::{ApiError, ApiStatus},
    interface::{ApiServer, session::Claims},
    prelude::*,
};

mod media_info;
mod route;

pub use media_info::*;
pub use route::*;

#[allow(unused_variables)]
#[async_trait]
impl apis::video::Video for ApiServer {
    type Claims = Claims;
    async fn create_video_session(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::CreateVideoSessionPathParams,
        body: &models::VideoSessionCreateRequest,
    ) -> std::result::Result<apis::video::CreateVideoSessionResponse, ()> {
        Ok(match self.state.videos.create(&path_params.library_id, &body.path).await {
            Ok(info) => apis::video::CreateVideoSessionResponse::Status201_VideoSessionCreated(models::VideoSessionResponse::new(models::VideoSession::new(
                info.id,
                info.library_id,
                info.path,
                info.playlist_url,
                info.duration_secs,
            ))),
            Err(error) => ApiError::from(error).into(),
        })
    }
    async fn delete_video_session(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::DeleteVideoSessionPathParams,
    ) -> std::result::Result<apis::video::DeleteVideoSessionResponse, ()> {
        Ok(match self.state.videos.close(&path_params.library_id, &path_params.session_id).await {
            Ok(()) => apis::video::DeleteVideoSessionResponse::Status204_VideoSessionEnded,
            Err(error) => ApiError::from(error).into(),
        })
    }
    async fn get_media_info(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::GetMediaInfoPathParams,
        query_params: &models::GetMediaInfoQueryParams,
    ) -> std::result::Result<apis::video::GetMediaInfoResponse, ()> {
        // 再生と情報パネルが開いた 1 件の情報なので、ユーザーが待っているものとして数える。
        // occupy は枠を使わずに下位の段を保留するだけで、probe 自体は従来どおり Foreground の枠で走る。
        let _waiting = media_info_pool(&self.state.pools, &query_params.path).occupy(WorkClass::Interactive);
        let result = async {
            let source = self.state.mutation.source(&path_params.library_id, &query_params.path, false, false).await?;
            Ok::<_, Error>(MediaInfoModel::from(self.state.media_info.probe(&source, WorkClass::Foreground).await?))
        }
        .await;
        Ok(match result {
            Ok(info) => apis::video::GetMediaInfoResponse::Status200_MediaInformationOfAnImageOrAVideo(models::MediaInfoResponse::new(info.0)),
            Err(error) => ApiError::from(error).into(),
        })
    }
    async fn get_video_playlist(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::GetVideoPlaylistPathParams,
    ) -> std::result::Result<apis::video::GetVideoPlaylistResponse, ()> {
        Ok(match self.state.videos.playlist(&path_params.library_id, &path_params.session_id).await {
            Ok(playlist) => apis::video::GetVideoPlaylistResponse::Status200_VODPlaylist(playlist),
            Err(error) => ApiError::from(error).into(),
        })
    }
    async fn get_video_segment(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::GetVideoSegmentPathParams,
    ) -> std::result::Result<apis::video::GetVideoSegmentResponse, ()> {
        let result = async {
            let index = VideoSessionRoute::index(&path_params.segment_name)?;
            let mut reader = self.state.videos.segment(&path_params.library_id, &path_params.session_id, index).await?;
            let mut bytes = Vec::new();
            tokio::io::AsyncReadExt::read_to_end(&mut reader, &mut bytes).await?;
            Ok::<_, Error>(bytes)
        }
        .await;
        Ok(match result {
            Ok(bytes) => apis::video::GetVideoSegmentResponse::Status200_MPEGTransportStreamSegment(ByteArray(bytes)),
            Err(error) => ApiError::from(error).into(),
        })
    }
}

impl From<ApiError> for apis::video::CreateVideoSessionResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            ApiStatus::Conflict => Self::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(error.body),
            ApiStatus::GatewayTimeout => Self::Status504_ExtractionOrThumbnailGenerationTimedOut(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::video::DeleteVideoSessionResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            ApiStatus::Conflict => Self::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(error.body),
            ApiStatus::GatewayTimeout => Self::Status504_ExtractionOrThumbnailGenerationTimedOut(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::video::GetMediaInfoResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            ApiStatus::Conflict => Self::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(error.body),
            ApiStatus::GatewayTimeout => Self::Status504_ExtractionOrThumbnailGenerationTimedOut(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::video::GetVideoPlaylistResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            ApiStatus::Conflict => Self::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(error.body),
            ApiStatus::GatewayTimeout => Self::Status504_ExtractionOrThumbnailGenerationTimedOut(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::video::GetVideoSegmentResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            ApiStatus::Conflict => Self::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(error.body),
            ApiStatus::GatewayTimeout => Self::Status504_ExtractionOrThumbnailGenerationTimedOut(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

/// A video, and a file with an unknown extension that ffprobe reads as a video, count in the video pool; an image counts in the image process pool.
// omnius-lint:allow(free-fn) Pure pool choice for the media info handler, with no owning type
fn media_info_pool<'a>(pools: &'a WorkPools, path: &str) -> &'a WorkPriorityPool {
    if uses_video_pool(MediaKind::of_name(path)) {
        &pools.process_video
    } else {
        &pools.process_image
    }
}

// omnius-lint:allow(free-fn) Pure kind check for media_info_pool, with no owning type
fn uses_video_pool(kind: Option<MediaKind>) -> bool {
    matches!(kind, Some(MediaKind::Video) | None)
}

#[cfg(test)]
mod tests {
    use http::StatusCode;
    use omnius_pxna_engine::service::ArchiveOptions;
    use serde_json::json;
    use testresult::TestResult;

    use super::*;
    use crate::interface::server::tests::*;

    #[test]
    fn media_info_counts_videos_and_unknown_files_in_the_video_pool_test() {
        assert!(uses_video_pool(MediaKind::of_name("a/clip.MP4")));
        assert!(uses_video_pool(MediaKind::of_name("a/clip.unknown")));
        assert!(!uses_video_pool(MediaKind::of_name("a/photo.heic")));
        assert!(!uses_video_pool(MediaKind::of_name("a/photo.png")));
    }

    #[tokio::test]
    async fn media_info_of_an_image_has_the_image_kind_and_no_video_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let root = &fixture.server.state.conf.libraries[0].path;
        image::RgbaImage::new(40, 20).save(root.join("pic.png"))?;
        let router = router(&fixture);
        let uri = "/api/v1/libraries/photos/media-info?path=/pic.png";
        assert_eq!(send(&router, Method::GET, uri, &[], None).await?.status, StatusCode::UNAUTHORIZED);
        let setup = send(&router, Method::POST, "/api/v1/setup", &[], credentials("owner", "password1", "bearer")).await?;
        let bearer = bearer(&setup)?;
        let info = send(&router, Method::GET, uri, &[("authorization", bearer.as_str())], None).await?;
        assert_eq!(info.status, StatusCode::OK);
        let media = &info.body["mediaInfo"];
        assert_eq!(media["kind"], "image");
        assert_eq!(media["image"]["contentType"], "image/png");
        assert_eq!((media["image"]["width"].as_u64(), media["image"]["height"].as_u64()), (Some(40), Some(20)));
        assert_eq!(media["image"]["shooting"], json!({}), "Exif を持たない画像の撮影情報は空");
        assert!(media.get("video").is_none(), "画像の応答は video を持たない");
        assert!(media["image"]["shooting"].get("latitude").is_none());

        std::fs::write(root.join("broken.png"), b"not an image")?;
        let broken = send(
            &router,
            Method::GET,
            "/api/v1/libraries/photos/media-info?path=/broken.png",
            &[("authorization", bearer.as_str())],
            None,
        )
        .await?;
        assert_eq!((broken.status, broken.body["kind"].as_str()), (StatusCode::BAD_REQUEST, Some("UnsupportedType")));
        Ok(())
    }

    #[tokio::test]
    async fn video_http_auth_media_playlist_segment_and_close_test() -> TestResult {
        let available = ["ffmpeg", "ffprobe"]
            .iter()
            .all(|program| std::process::Command::new(program).arg("-version").output().is_ok_and(|output| output.status.success()));
        if !available {
            assert!(std::env::var_os("PXNA_REQUIRE_FFMPEG").is_none());
            return Ok(());
        }
        let fixture = fixture(ArchiveOptions::default()).await?;
        let root = &fixture.server.state.conf.libraries[0].path;
        let status = std::process::Command::new("ffmpeg")
            .args([
                "-v",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc=duration=2:size=160x90:rate=10",
                "-c:v",
                "mpeg4",
                "-pix_fmt",
                "yuv420p",
            ])
            .arg(root.join("clip.mp4"))
            .status()?;
        assert!(status.success());
        let router = router(&fixture);
        let no_auth = send(&router, Method::GET, "/api/v1/libraries/photos/media-info?path=/clip.mp4", &[], None).await?;
        assert_eq!(no_auth.status, StatusCode::UNAUTHORIZED);
        let setup = send(&router, Method::POST, "/api/v1/setup", &[], credentials("owner", "password1", "bearer")).await?;
        let bearer = bearer(&setup)?;
        let headers = &[("authorization", bearer.as_str())];
        let info = send(&router, Method::GET, "/api/v1/libraries/photos/media-info?path=/clip.mp4", headers, None).await?;
        assert_eq!(info.status, StatusCode::OK);
        assert_eq!(info.body["mediaInfo"]["kind"], "video");
        assert_eq!(info.body["mediaInfo"]["video"]["videoCodec"], "mpeg4");
        assert!(info.body["mediaInfo"].get("image").is_none(), "動画の応答は image を持たない");
        let created = send(&router, Method::POST, "/api/v1/libraries/photos/video/sessions", headers, Some(json!({"path":"/clip.mp4"}))).await?;
        assert_eq!(created.status, StatusCode::CREATED);
        let id = created.body["session"]["id"].as_str().ok_or("no id")?;
        let url = created.body["session"]["playlistUrl"].as_str().ok_or("no playlist")?;
        assert_eq!(send(&router, Method::GET, url, &[], None).await?.status, StatusCode::UNAUTHORIZED);
        let playlist = send(&router, Method::GET, url, headers, None).await?;
        assert_eq!(playlist.status, StatusCode::OK);
        assert_eq!(playlist.header("content-type"), Some("application/vnd.apple.mpegurl"));
        assert!(playlist.bytes.starts_with(b"#EXTM3U"));
        let segment_url = format!("/api/v1/libraries/photos/video/sessions/{id}/segments/0.ts");
        let segment = send(&router, Method::GET, &segment_url, headers, None).await?;
        assert_eq!(segment.status, StatusCode::OK);
        assert_eq!(segment.header("content-type"), Some("video/mp2t"));
        assert_eq!(segment.bytes[0], 0x47);
        assert_eq!(segment.header("content-length"), Some(segment.bytes.len().to_string().as_str()));
        let wrong_scope = segment_url.replace("/photos/", "/other/");
        assert_eq!(send(&router, Method::GET, &wrong_scope, headers, None).await?.status, StatusCode::NOT_FOUND);
        let invalid = segment_url.replace("/0.ts", "/0.png");
        assert_eq!(send(&router, Method::GET, &invalid, headers, None).await?.status, StatusCode::BAD_REQUEST);
        let close_url = format!("/api/v1/libraries/photos/video/sessions/{id}");
        assert_eq!(send(&router, Method::DELETE, &close_url, headers, None).await?.status, StatusCode::NO_CONTENT);
        assert_eq!(send(&router, Method::GET, url, headers, None).await?.status, StatusCode::NOT_FOUND);
        Ok(())
    }
}
