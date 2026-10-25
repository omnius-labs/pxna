use async_trait::async_trait;
use axum_extra::extract::CookieJar;
use headers::Host;
use http::Method;

use omnius_pxna_interface::{apis, models};

use crate::interface::{ApiServer, session::Claims};

mod route;

pub use route::*;

#[allow(unused_variables)]
#[async_trait]
impl apis::thumbnails::Thumbnails for ApiServer {
    type Claims = Claims;

    async fn get_thumbnail(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        header_params: &models::GetThumbnailHeaderParams,
        path_params: &models::GetThumbnailPathParams,
        query_params: &models::GetThumbnailQueryParams,
    ) -> std::result::Result<apis::thumbnails::GetThumbnailResponse, ()> {
        // 外側のルータの手書きの route が先に処理するため、この実装は呼ばれない。
        let mut error = models::ErrorResponse::new(models::ErrorKind::InternalServerError);
        error.message = Some("served by a hand-written route".to_string());
        Ok(apis::thumbnails::GetThumbnailResponse::Status500_InternalServerError(error))
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use http::StatusCode;
    use omnius_pxna_engine::service::ArchiveOptions;
    use testresult::TestResult;

    use super::*;
    use crate::interface::server::tests::*;

    fn write_png(path: &Path, width: u32, height: u32) -> TestResult {
        let mut output = Vec::new();
        image::RgbaImage::from_pixel(width, height, image::Rgba([10, 20, 30, 255])).write_to(&mut std::io::Cursor::new(&mut output), image::ImageFormat::Png)?;
        std::fs::write(path, output)?;
        Ok(())
    }

    /// オーナーを登録して、cookie のセッションで要求を送るための `Cookie` の値を返す。
    async fn owner_cookie(router: &axum::Router) -> TestResult<String> {
        let setup = send(router, Method::POST, "/api/v1/setup", &[], credentials("owner", "password1", "cookie")).await?;
        cookie(&setup)
    }

    fn thumbnail_uri(query: &str) -> String {
        format!("/api/v1/libraries/photos/thumbnails?{query}")
    }

    #[tokio::test]
    async fn thumbnail_returns_the_requested_format_and_headers_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        write_png(&fixture._dir.path().join("library/photos/wide.png"), 600, 300)?;
        let router = router(&fixture);
        let cookie = owner_cookie(&router).await?;
        let get = |query: String| {
            let (router, cookie) = (router.clone(), cookie.clone());
            async move { send(&router, Method::GET, &thumbnail_uri(&query), &[("cookie", &cookie)], None).await }
        };

        let default_format = get("path=/photos/wide.png&width=200&height=200&resize=fit".to_string()).await?;
        assert_eq!(default_format.status, StatusCode::OK);
        assert_eq!(default_format.header("content-type"), Some("image/webp"), "既定の形式は WebP");
        assert_eq!(image::guess_format(&default_format.bytes)?, image::ImageFormat::WebP);

        let jpeg = get("path=/photos/wide.png&width=200&height=200&resize=fit&format=jpeg".to_string()).await?;
        assert_eq!(jpeg.status, StatusCode::OK);
        assert_eq!(jpeg.header("content-type"), Some("image/jpeg"));
        assert_eq!(jpeg.header("cache-control"), Some("private, max-age=86400"));
        assert!(jpeg.header("etag").is_some_and(|etag| etag.starts_with('"') && etag.ends_with('"')));
        assert_eq!(jpeg.image_size()?, (256, 128), "200 は 256 に切り上がり、縦横比を保って収まる");
        assert_eq!(jpeg.header("pxna-frame-count"), None);

        let webp = get("path=/photos/wide.png&width=200&height=200&resize=fit&format=webp".to_string()).await?;
        assert_eq!(webp.header("content-type"), Some("image/webp"));
        assert_eq!(webp.image_size()?, (256, 128));
        assert_eq!(webp.header("etag"), default_format.header("etag"), "既定の形式と同じ要求は同じ ETag");

        let png = get("path=/photos/wide.png&width=200&height=200&format=png".to_string()).await?;
        assert_eq!(png.header("content-type"), Some("image/png"));
        assert_eq!(png.image_size()?, (256, 256), "resize の既定は crop");
        assert_ne!(png.header("etag"), jpeg.header("etag"));

        let display = get("path=/photos/wide.png&variant=display".to_string()).await?;
        assert_eq!(display.status, StatusCode::OK);
        assert_eq!(display.image_size()?, (600, 300), "表示用画像は元の画像より大きくしない");

        let default_size = get("path=/photos/wide.png".to_string()).await?;
        assert_eq!(default_size.image_size()?, (256, 256), "width と height の既定値は 256");
        Ok(())
    }

    #[tokio::test]
    async fn thumbnail_request_spans_share_one_request_test() -> TestResult {
        use crate::logging::SpanCapture;

        let fixture = fixture(ArchiveOptions::default()).await?;
        write_png(&fixture._dir.path().join("library/photos/spans.png"), 600, 300)?;
        let router = router(&fixture);
        let cookie = owner_cookie(&router).await?;

        let capture = SpanCapture::start();
        let response = send(&router, Method::GET, &thumbnail_uri("path=/photos/spans.png&variant=display"), &[("cookie", &cookie)], None).await?;
        assert_eq!(response.status, StatusCode::OK);

        // 並行する別のテストの span も入るので、この要求の `request` で絞る。
        let names = ["thumbnail_request", "thumbnail_generate", "thumbnail_production", "thumbnail_store"];
        let spans = capture
            .wait_for(|fields| {
                let request = fields.iter().find(|line| line["operation"] == "thumbnail_request" && line["path"] == "/photos/spans.png")?;
                let mine: Vec<_> = fields.iter().filter(|line| line["request"] == request["request"]).cloned().collect();
                names.iter().all(|name| mine.iter().any(|line| line["operation"] == *name)).then_some(mine)
            })
            .await;
        let span = |name: &str| spans.iter().find(|line| line["operation"] == name).cloned().unwrap_or_default();
        assert_eq!(span("thumbnail_request")["ok"].as_bool(), Some(true));
        for (name, parent) in [
            ("thumbnail_generate", "thumbnail_request"),
            ("thumbnail_production", "thumbnail_generation"),
            ("thumbnail_store", "thumbnail_production"),
        ] {
            assert_eq!(span(name)["parent"].as_str(), Some(parent), "spawn した生成も同じ要求に入る: {name}");
        }
        Ok(())
    }

    #[tokio::test]
    async fn display_priority_header_selects_prefetch_or_rejects_unknown_values_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        write_png(&fixture._dir.path().join("library/photos/wide.png"), 600, 300)?;
        let router = router(&fixture);
        let cookie = owner_cookie(&router).await?;
        let uri = thumbnail_uri("path=/photos/wide.png&variant=display");
        for (priority, status) in [("prefetch", StatusCode::OK), ("interactive", StatusCode::OK), ("urgent", StatusCode::BAD_REQUEST)] {
            let response = send(&router, Method::GET, &uri, &[("cookie", &cookie), ("pxna-priority", priority)], None).await?;
            assert_eq!(response.status, status, "{priority}");
        }
        Ok(())
    }

    #[tokio::test]
    async fn thumbnail_without_format_follows_the_configured_format_test() -> TestResult {
        let thumbnail = omnius_pxna_engine::service::ThumbnailOptions {
            format: omnius_pxna_engine::model::ImageFormatType::Jpeg,
            ..Default::default()
        };
        let fixture = fixture_with_thumbnail(ArchiveOptions::default(), None, std::sync::Arc::new(omnius_core_base::clock::ClockUtc), thumbnail).await?;
        write_png(&fixture._dir.path().join("library/photos/wide.png"), 600, 300)?;
        let router = router(&fixture);
        let cookie = owner_cookie(&router).await?;
        let get = |query: &'static str| {
            let (router, cookie) = (router.clone(), cookie.clone());
            async move { send(&router, Method::GET, &thumbnail_uri(query), &[("cookie", &cookie)], None).await }
        };

        assert_eq!(get("path=/photos/wide.png").await?.header("content-type"), Some("image/jpeg"), "省略した形式は設定に従う");
        assert_eq!(
            get("path=/photos/wide.png&format=webp").await?.header("content-type"),
            Some("image/webp"),
            "要求の形式が設定に優先する"
        );
        Ok(())
    }

    #[tokio::test]
    async fn thumbnail_pads_with_transparency_in_webp_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        write_png(&fixture._dir.path().join("library/photos/wide.png"), 600, 300)?;
        let router = router(&fixture);
        let cookie = owner_cookie(&router).await?;
        let padded = send(
            &router,
            Method::GET,
            &thumbnail_uri("path=/photos/wide.png&width=256&height=256&resize=pad"),
            &[("cookie", &cookie)],
            None,
        )
        .await?;
        assert_eq!(padded.status, StatusCode::OK);
        assert_eq!(padded.header("content-type"), Some("image/webp"));
        assert_eq!(image::load_from_memory(&padded.bytes)?.to_rgba8().get_pixel(128, 0)[3], 0, "上の余白は透明");
        Ok(())
    }

    #[tokio::test]
    async fn thumbnail_rejects_invalid_requests_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        write_png(&fixture._dir.path().join("library/photos/wide.png"), 600, 300)?;
        let router = router(&fixture);
        let cookie = owner_cookie(&router).await?;

        let cases = [
            ("path=/photos/wide.png&resize=pad&format=jpeg", StatusCode::BAD_REQUEST, "InvalidRequest"),
            ("path=/photos/wide.png&width=5000", StatusCode::BAD_REQUEST, "InvalidRequest"),
            ("path=/photos/wide.png&width=0", StatusCode::BAD_REQUEST, "InvalidRequest"),
            ("path=/photos/wide.png&variant=unknown", StatusCode::BAD_REQUEST, "InvalidRequest"),
            ("path=/photos/wide.png&format=gif", StatusCode::BAD_REQUEST, "InvalidRequest"),
            ("width=100", StatusCode::BAD_REQUEST, "InvalidRequest"),
            ("path=/photos/wide.png&variant=sprite", StatusCode::BAD_REQUEST, "InvalidRequest"),
            ("path=/photos/notes.txt", StatusCode::BAD_REQUEST, "UnsupportedType"),
            ("path=/photos/missing.png", StatusCode::NOT_FOUND, "NotFound"),
            ("path=/../etc/passwd", StatusCode::BAD_REQUEST, "InvalidRequest"),
        ];
        for (query, status, kind) in cases {
            let response = send(&router, Method::GET, &thumbnail_uri(query), &[("cookie", &cookie)], None).await?;
            assert_eq!((response.status, response.body["kind"].as_str()), (status, Some(kind)), "{query}");
        }

        let unknown_library = send(&router, Method::GET, "/api/v1/libraries/nowhere/thumbnails?path=/a.png", &[("cookie", &cookie)], None).await?;
        assert_eq!((unknown_library.status, unknown_library.body["kind"].as_str()), (StatusCode::NOT_FOUND, Some("NotFound")));
        Ok(())
    }

    #[tokio::test]
    async fn thumbnail_requires_a_session_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        write_png(&fixture._dir.path().join("library/photos/wide.png"), 600, 300)?;
        let router = router(&fixture);
        let uri = thumbnail_uri("path=/photos/wide.png");

        let anonymous = send(&router, Method::GET, &uri, &[], None).await?;
        assert_eq!(anonymous.status, StatusCode::UNAUTHORIZED);
        assert_eq!(anonymous.body["kind"], "Unauthorized", "手書きの route の 401 は本文を持つ");
        let invalid = send(&router, Method::GET, &uri, &[("authorization", "Bearer invalid")], None).await?;
        assert_eq!(invalid.status, StatusCode::UNAUTHORIZED);

        send(&router, Method::POST, "/api/v1/setup", &[], credentials("owner", "password1", "bearer")).await?;
        let login = send(&router, Method::POST, "/api/v1/auth/login", &[], credentials("owner", "password1", "bearer")).await?;
        let bearer = bearer(&login)?;
        let ok = send(&router, Method::GET, &uri, &[("authorization", &bearer)], None).await?;
        assert_eq!(ok.status, StatusCode::OK);
        Ok(())
    }

    #[tokio::test]
    async fn thumbnail_of_folders_and_archives_returns_a_representative_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let photos = fixture._dir.path().join("library/photos");
        std::fs::create_dir_all(photos.join("album"))?;
        std::fs::create_dir_all(photos.join("empty"))?;
        write_png(&photos.join("album/10.png"), 100, 100)?;
        write_png(&photos.join("album/2.png"), 200, 100)?;
        write_png(&photos.join("album/1.png"), 300, 100)?;
        let png = |width, height| -> TestResult<Vec<u8>> {
            let mut output = Vec::new();
            image::RgbaImage::from_pixel(width, height, image::Rgba([1, 2, 3, 255])).write_to(&mut std::io::Cursor::new(&mut output), image::ImageFormat::Png)?;
            Ok(output)
        };
        write_7z(&photos.join("book.7z"), &[("cover.png", &png(400, 200)?), ("dir/x.png", &png(200, 400)?)])?;
        let router = router(&fixture);
        let cookie = owner_cookie(&router).await?;
        let get = |path: &'static str| {
            let (router, cookie) = (router.clone(), cookie.clone());
            async move {
                send(
                    &router,
                    Method::GET,
                    &thumbnail_uri(&format!("path={path}&width=512&height=512&resize=fit")),
                    &[("cookie", &cookie)],
                    None,
                )
                .await
            }
        };

        assert_eq!(get("/photos/album").await?.image_size()?, (300, 100), "フォルダは自然順で最初の 1.png");
        assert_eq!(get("/photos/book.7z").await?.image_size()?, (400, 200), "archive は直下の cover.png");
        assert_eq!(get("/photos/book.7z/dir").await?.image_size()?, (200, 400), "archive の内側のフォルダ");

        let empty = get("/photos/empty").await?;
        assert_eq!((empty.status, empty.body["kind"].as_str()), (StatusCode::NOT_FOUND, Some("NotFound")));
        Ok(())
    }

    /// ローカルでは ffmpeg のない環境を許すが、CI では動画の検証を必須にする。
    fn skip_without_ffmpeg() -> bool {
        let available = ["ffmpeg", "ffprobe"].iter().all(|program| {
            std::process::Command::new(program)
                .arg("-version")
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
        });
        if !available {
            assert!(std::env::var_os("PXNA_REQUIRE_FFMPEG").is_none(), "ffmpeg and ffprobe are required in CI");
            eprintln!("skipped: ffmpeg or ffprobe is not available on PATH");
        }
        !available
    }

    #[tokio::test]
    async fn thumbnail_of_a_video_returns_a_sprite_with_its_frame_count_test() -> TestResult {
        if skip_without_ffmpeg() {
            return Ok(());
        }
        let fixture = fixture(ArchiveOptions::default()).await?;
        let video = fixture._dir.path().join("library/photos/clip.mp4");
        let status = std::process::Command::new("ffmpeg")
            .args([
                "-v",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc=duration=10:size=320x240:rate=10",
                "-c:v",
                "mpeg4",
                "-pix_fmt",
                "yuv420p",
            ])
            .arg(&video)
            .status()?;
        assert!(status.success());
        let router = router(&fixture);
        let cookie = owner_cookie(&router).await?;
        let get = |query: &'static str| {
            let (router, cookie) = (router.clone(), cookie.clone());
            async move { send(&router, Method::GET, &thumbnail_uri(query), &[("cookie", &cookie)], None).await }
        };

        let sprite = get("path=/photos/clip.mp4&variant=sprite&width=128&height=128&resize=fit").await?;
        assert_eq!(sprite.status, StatusCode::OK);
        assert_eq!(sprite.header("content-type"), Some("image/webp"));
        assert_eq!(sprite.header("pxna-frame-count"), Some("8"));
        assert_eq!(sprite.image_size()?, (8 * 128, 96));

        let thumbnail = get("path=/photos/clip.mp4&width=128&height=128&resize=fit").await?;
        assert_eq!(thumbnail.image_size()?, (128, 96), "動画のサムネイルは 1 枚目のフレーム");
        assert_eq!(thumbnail.header("pxna-frame-count"), None);

        let broken = fixture._dir.path().join("library/photos/fake.mp4");
        std::fs::write(&broken, b"this is not a video")?;
        let unreadable = get("path=/photos/fake.mp4").await?;
        assert_eq!((unreadable.status, unreadable.body["kind"].as_str()), (StatusCode::BAD_REQUEST, Some("UnsupportedType")));
        Ok(())
    }
}
