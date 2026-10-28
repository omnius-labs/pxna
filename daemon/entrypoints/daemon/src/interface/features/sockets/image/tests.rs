//! The image socket through a real WebSocket: the upgrade, the images it carries, the order they come in, and the ways it closes.
use std::{
    collections::HashMap,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use futures::{SinkExt as _, StreamExt as _};
use http::{Method, StatusCode};
use serde_json::{Value, json};
use testresult::TestResult;
use tokio_tungstenite::tungstenite::{self, client::IntoClientRequest as _};

use omnius_core_base::clock::ClockUtc;
use omnius_pxna_engine::{
    model::ImageFormatType,
    service::{ArchiveOptions, SchedulePermit, ThumbnailOptions, WorkClass},
};

use super::message::{FrameCodec, FrameHeader, ServerMessage};
use crate::{
    interface::server::tests::*,
    service::sockets::image::{ImageSocketLimits, ImageSockets},
};

type Socket = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// What the client reads from the socket.
#[derive(Debug)]
enum Event {
    Text(ServerMessage),
    Frame(FrameHeader, Vec<u8>),
    Ping,
    /// The close code the server sent, if it sent one.
    Closed(Option<u16>),
}

/// A daemon listening on a port, with its owner logged in.
struct Harness {
    fixture: Fixture,
    router: axum::Router,
    addr: SocketAddr,
    cookie: String,
}

impl Harness {
    async fn start(limits: ImageSocketLimits) -> TestResult<Self> {
        Self::start_with(limits, ThumbnailOptions::default()).await
    }

    async fn start_with(limits: ImageSocketLimits, thumbnail: ThumbnailOptions) -> TestResult<Self> {
        let mut fixture = fixture_with_thumbnail(ArchiveOptions::default(), None, Arc::new(ClockUtc), thumbnail).await?;
        Arc::get_mut(&mut fixture.server.state).expect("nothing shares the state yet").image_sockets = ImageSockets::new(limits);
        let router = router(&fixture);
        let setup = send(&router, Method::POST, "/api/v1/setup", &[], credentials("owner", "password1", "cookie")).await?;
        let cookie = cookie(&setup)?;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let app = router.clone();
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        Ok(Self { fixture, router, addr, cookie })
    }

    /// Local runs may lack ffmpeg, but CI requires video verification.
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

    fn library(&self) -> PathBuf {
        self.fixture._dir.path().join("library/photos")
    }

    fn state(&self) -> &Arc<crate::state::DaemonState> {
        &self.fixture.server.state
    }

    /// Opens a socket with the owner's cookie and an `Origin` that matches the host.
    async fn open(&self) -> TestResult<Socket> {
        Ok(self
            .try_open(Some(&self.cookie), Some(&format!("http://{}", self.addr)), None)
            .await
            .map_err(|status| format!("refused with {status}"))?)
    }

    /// The upgrade request with the given credentials. `Err` is the status the daemon refused with.
    async fn try_open(&self, cookie: Option<&str>, origin: Option<&str>, bearer: Option<&str>) -> std::result::Result<Socket, StatusCode> {
        let mut request = format!("ws://{}/api/v1/sockets/image", self.addr).into_client_request().expect("a valid request");
        let headers = request.headers_mut();
        if let Some(cookie) = cookie {
            headers.insert("cookie", cookie.parse().expect("a header value"));
        }
        if let Some(origin) = origin {
            headers.insert("origin", origin.parse().expect("a header value"));
        }
        if let Some(bearer) = bearer {
            headers.insert("authorization", bearer.parse().expect("a header value"));
        }
        match tokio_tungstenite::connect_async(request).await {
            Ok((socket, _)) => Ok(socket),
            Err(tungstenite::Error::Http(response)) => Err(response.status()),
            Err(error) => panic!("the connection failed: {error}"),
        }
    }

    fn pool(&self) -> &omnius_pxna_engine::service::PriorityPool {
        &self.state().pools.cpu
    }

    fn write_png(&self, name: &str, width: u32, height: u32) -> TestResult {
        let mut output = Vec::new();
        image::RgbaImage::from_pixel(width, height, image::Rgba([10, 20, 30, 255])).write_to(&mut std::io::Cursor::new(&mut output), image::ImageFormat::Png)?;
        std::fs::write(self.library().join(name), output)?;
        Ok(())
    }

    fn write_noisy_png(&self, name: &str, width: u32, height: u32) -> TestResult {
        let mut output = Vec::new();
        image::RgbaImage::from_fn(width, height, |x, y| {
            let v = x.wrapping_mul(2654435761).wrapping_add(y.wrapping_mul(40503)) >> 8;
            image::Rgba([v as u8, (v >> 8) as u8, (v >> 16) as u8, 255])
        })
        .write_to(&mut std::io::Cursor::new(&mut output), image::ImageFormat::Png)?;
        std::fs::write(self.library().join(name), output)?;
        Ok(())
    }

    /// How many files the thumbnail cache holds, which tells how many images were made.
    fn cached_files(&self) -> usize {
        fn count(dir: &Path) -> usize {
            std::fs::read_dir(dir)
                .into_iter()
                .flatten()
                .flatten()
                .map(|entry| if entry.path().is_dir() { count(&entry.path()) } else { 1 })
                .sum()
        }
        count(&self.fixture._dir.path().join("state/thumbnail/files"))
    }

    /// Takes every slot of the image pool, so that the jobs that ask for it wait. The shared slots go first, because Foreground
    /// does not start while Interactive runs.
    async fn hold_all(&self) -> Vec<SchedulePermit> {
        let limits = self.pool().limits();
        let mut held = Vec::new();
        for _ in 0..limits.shared {
            held.push(self.pool().acquire(WorkClass::Foreground).await);
        }
        for _ in 0..limits.interactive {
            held.push(self.pool().acquire(WorkClass::Interactive).await);
        }
        held
    }
}

/// An image the client has put together from its chunks.
struct Received {
    header: FrameHeader,
    bytes: Vec<u8>,
    frames: usize,
}

/// The client side of the socket: reading what the server sends and sending what it wants.
struct Wire;

impl Wire {
    async fn until(condition: impl Fn() -> bool) {
        tokio::time::timeout(Duration::from_secs(30), async {
            while !condition() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("the condition was not reached in time");
    }

    fn wanted(id: u64, path: &str, kind: &str, class: &str, order: u32) -> Value {
        json!({ "id": id, "libraryId": "photos", "path": path, "kind": kind, "width": 256, "height": 256, "class": class, "order": order })
    }

    async fn want(socket: &mut Socket, items: Vec<Value>) -> TestResult {
        socket.send(tungstenite::Message::text(json!({ "type": "want", "items": items }).to_string())).await?;
        Ok(())
    }

    async fn drop_items(socket: &mut Socket, ids: &[u64]) -> TestResult {
        socket.send(tungstenite::Message::text(json!({ "type": "drop", "ids": ids }).to_string())).await?;
        Ok(())
    }

    /// The next thing the server sends. The client answers pings by reading, as a browser does.
    async fn next_event(socket: &mut Socket) -> TestResult<Event> {
        loop {
            let message = tokio::time::timeout(Duration::from_secs(30), socket.next()).await?;
            match message {
                None => return Ok(Event::Closed(None)),
                Some(Err(_)) => return Ok(Event::Closed(None)),
                Some(Ok(tungstenite::Message::Text(text))) => return Ok(Event::Text(serde_json::from_str(text.as_str())?)),
                Some(Ok(tungstenite::Message::Binary(frame))) => {
                    let (header, chunk) = FrameCodec::decode(&frame)?;
                    return Ok(Event::Frame(header, chunk.to_vec()));
                }
                Some(Ok(tungstenite::Message::Ping(_))) => return Ok(Event::Ping),
                Some(Ok(tungstenite::Message::Close(frame))) => return Ok(Event::Closed(frame.map(|frame| u16::from(frame.code)))),
                Some(Ok(_)) => {}
            }
        }
    }

    /// Reads until `count` images are complete, and returns them in the order they completed, with the text messages that came between.
    async fn read_images(socket: &mut Socket, count: usize) -> TestResult<(Vec<Received>, Vec<ServerMessage>)> {
        let mut partial: HashMap<u64, Received> = HashMap::new();
        let mut done = Vec::new();
        let mut texts = Vec::new();
        while done.len() < count {
            match Wire::next_event(socket).await? {
                Event::Text(text) => texts.push(text),
                Event::Frame(header, chunk) => {
                    let received = partial.entry(header.id).or_insert_with(|| Received {
                        header: header.clone(),
                        bytes: Vec::new(),
                        frames: 0,
                    });
                    assert_eq!(header.offset as usize, received.bytes.len(), "the chunks of an image come in order");
                    received.bytes.extend_from_slice(&chunk);
                    received.frames += 1;
                    if received.bytes.len() as u64 == header.total {
                        done.push(partial.remove(&header.id).expect("the image is being received"));
                    }
                }
                Event::Ping => {}
                Event::Closed(code) => return Err(format!("the socket closed with {code:?}").into()),
            }
        }
        Ok((done, texts))
    }

    /// Expects nothing to arrive for a while.
    async fn expect_quiet(socket: &mut Socket) -> TestResult {
        match tokio::time::timeout(Duration::from_millis(300), Wire::next_event(socket)).await {
            Err(_) => Ok(()),
            Ok(event) => Err(format!("something arrived: {:?}", event?).into()),
        }
    }

    async fn expect_close(socket: &mut Socket) -> TestResult<Option<u16>> {
        loop {
            match Wire::next_event(socket).await? {
                Event::Closed(code) => return Ok(code),
                Event::Ping | Event::Text(_) | Event::Frame(..) => {}
            }
        }
    }
}

#[tokio::test]
async fn the_upgrade_needs_a_session_and_a_matching_origin_test() -> TestResult {
    let harness = Harness::start(ImageSocketLimits::default()).await?;
    let matching = format!("http://{}", harness.addr);
    let cookie = harness.cookie.as_str();

    assert_eq!(harness.try_open(None, Some(&matching), None).await.err(), Some(StatusCode::UNAUTHORIZED), "no session");
    assert_eq!(
        harness.try_open(Some("pxna_session=nonsense"), Some(&matching), None).await.err(),
        Some(StatusCode::UNAUTHORIZED)
    );
    assert_eq!(
        harness.try_open(Some(cookie), None, None).await.err(),
        Some(StatusCode::FORBIDDEN),
        "a cookie and no Origin"
    );
    assert_eq!(harness.try_open(Some(cookie), Some("http://evil.example"), None).await.err(), Some(StatusCode::FORBIDDEN));
    assert!(harness.try_open(Some(cookie), Some(&matching), None).await.is_ok(), "a cookie and a matching Origin");

    // A client that sets a header sends no cookie, and needs no Origin.
    let login = send(&harness.router, Method::POST, "/api/v1/auth/login", &[], credentials("owner", "password1", "bearer")).await?;
    let bearer = bearer(&login)?;
    assert!(harness.try_open(None, None, Some(&bearer)).await.is_ok());
    Ok(())
}

#[tokio::test]
async fn a_session_may_keep_a_limited_number_of_sockets_open_test() -> TestResult {
    let harness = Harness::start(ImageSocketLimits {
        max_sockets_per_session: 2,
        ..ImageSocketLimits::default()
    })
    .await?;
    let first = harness.open().await?;
    let _second = harness.open().await?;
    assert_eq!(
        harness.try_open(Some(&harness.cookie), Some(&format!("http://{}", harness.addr)), None).await.err(),
        Some(StatusCode::TOO_MANY_REQUESTS)
    );

    drop(first);
    // The place frees when the daemon notices the connection ended.
    let origin = format!("http://{}", harness.addr);
    tokio::time::timeout(Duration::from_secs(10), async {
        while harness.try_open(Some(&harness.cookie), Some(&origin), None).await.is_err() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await?;
    Ok(())
}

#[tokio::test]
async fn an_image_arrives_in_chunks_in_the_default_format_with_an_etag_per_key_test() -> TestResult {
    let harness = Harness::start(ImageSocketLimits {
        chunk_bytes: 1024,
        ..ImageSocketLimits::default()
    })
    .await?;
    harness.write_noisy_png("wide.png", 600, 300)?;

    let mut socket = harness.open().await?;
    let fit = |id| {
        let mut item = Wire::wanted(id, "/photos/wide.png", "thumbnail", "visible", 0);
        item["width"] = json!(200);
        item["height"] = json!(200);
        item["shape"] = json!("fit");
        item
    };
    Wire::want(&mut socket, vec![fit(1)]).await?;
    let (images, _) = Wire::read_images(&mut socket, 1).await?;

    let image = &images[0];
    let etag = image.header.etag.clone().expect("a file has an etag");
    assert_eq!(etag.len(), 64, "the etag is the key, in hex");
    assert_eq!(image.header.content_type, "image/webp", "the default format is WebP");
    assert_eq!(image::guess_format(&image.bytes)?, image::ImageFormat::WebP);
    assert_eq!(image.header.frame_count, None);
    assert!(image.frames > 1, "an image over the chunk size is sent in several frames: {}", image.frames);

    // Another width and shape is another key, and the same key keeps its etag.
    Wire::want(&mut socket, vec![Wire::wanted(2, "/photos/wide.png", "thumbnail", "visible", 1)]).await?;
    let (images, _) = Wire::read_images(&mut socket, 1).await?;
    assert_ne!(images[0].header.etag.as_deref(), Some(etag.as_str()));
    Ok(())
}

#[tokio::test]
async fn the_default_format_follows_the_configuration_test() -> TestResult {
    let harness = Harness::start_with(
        ImageSocketLimits::default(),
        ThumbnailOptions {
            format: ImageFormatType::Jpeg,
            ..ThumbnailOptions::default()
        },
    )
    .await?;
    harness.write_png("wide.png", 600, 300)?;
    let mut socket = harness.open().await?;
    Wire::want(&mut socket, vec![Wire::wanted(1, "/photos/wide.png", "thumbnail", "visible", 0)]).await?;
    let (images, _) = Wire::read_images(&mut socket, 1).await?;
    assert_eq!(images[0].header.content_type, "image/jpeg", "the configured format is the default");
    assert_eq!(image::guess_format(&images[0].bytes)?, image::ImageFormat::Jpeg);
    Ok(())
}

#[tokio::test]
async fn an_item_for_a_folder_or_an_archive_uses_its_representative_test() -> TestResult {
    let harness = Harness::start(ImageSocketLimits::default()).await?;
    let photos = harness.library();
    std::fs::create_dir_all(photos.join("album"))?;
    std::fs::create_dir_all(photos.join("empty"))?;
    let png = |width, height| -> TestResult<Vec<u8>> {
        let mut output = Vec::new();
        image::RgbaImage::from_pixel(width, height, image::Rgba([1, 2, 3, 255])).write_to(&mut std::io::Cursor::new(&mut output), image::ImageFormat::Png)?;
        Ok(output)
    };
    harness.write_png("album/10.png", 100, 100)?;
    harness.write_png("album/2.png", 200, 100)?;
    harness.write_png("album/1.png", 300, 100)?;
    write_7z(&photos.join("book.7z"), &[("cover.png", &png(400, 200)?), ("dir/x.png", &png(200, 400)?)])?;

    let mut socket = harness.open().await?;
    let wanted = |id, path| {
        let mut item = Wire::wanted(id, path, "thumbnail", "visible", id as u32);
        item["width"] = json!(512);
        item["height"] = json!(512);
        item["shape"] = json!("fit");
        item
    };
    Wire::want(
        &mut socket,
        vec![wanted(1, "/photos/album"), wanted(2, "/photos/book.7z"), wanted(3, "/photos/book.7z/dir")],
    )
    .await?;
    let (images, _) = Wire::read_images(&mut socket, 3).await?;
    let size = |id: u64| -> TestResult<(u32, u32)> {
        let image = images.iter().find(|image| image.header.id == id).ok_or("the item was not answered")?;
        let image = image::load_from_memory(&image.bytes)?;
        Ok((image.width(), image.height()))
    };
    assert_eq!(size(1)?, (300, 100), "a folder uses its natural-first image");
    assert_eq!(size(2)?, (400, 200), "an archive uses its cover image");
    assert_eq!(size(3)?, (200, 400), "an inner folder uses its first image");

    Wire::want(&mut socket, vec![wanted(4, "/photos/empty")]).await?;
    let mut failure = None;
    while failure.is_none() {
        match Wire::next_event(&mut socket).await? {
            Event::Text(ServerMessage::Failed { id: 4, kind, .. }) => failure = Some(serde_json::to_value(kind)?),
            other => panic!("expected failed, got {other:?}"),
        }
    }
    assert_eq!(failure, Some(json!("NotFound")), "an empty folder has no representative");
    Ok(())
}

#[tokio::test]
async fn an_item_with_the_current_etag_is_answered_unchanged_without_a_slot_test() -> TestResult {
    let harness = Harness::start(ImageSocketLimits::default()).await?;
    harness.write_png("a.png", 600, 300)?;
    let mut socket = harness.open().await?;
    Wire::want(&mut socket, vec![Wire::wanted(1, "/photos/a.png", "thumbnail", "visible", 0)]).await?;
    let (images, _) = Wire::read_images(&mut socket, 1).await?;
    let etag = images[0].header.etag.clone().expect("a file has an etag");

    // Every slot is taken, so a check that generated would wait for a slot. The image is in the cache, so an item that needs it is not held up.
    let _held = harness.hold_all().await;
    let mut again = Wire::wanted(2, "/photos/a.png", "thumbnail", "visible", 0);
    again["etag"] = json!(etag);
    let mut stale = Wire::wanted(3, "/photos/a.png", "thumbnail", "visible", 1);
    stale["etag"] = json!("0".repeat(64));
    Wire::want(&mut socket, vec![again, stale]).await?;

    // The item with the current etag is answered by a text message and no image, and the one with another etag gets its image.
    let (images, texts) = Wire::read_images(&mut socket, 1).await?;
    assert_eq!(images[0].header.id, 3);
    assert_eq!(images[0].header.etag.as_deref(), Some(etag.as_str()));
    let mut texts = texts;
    while texts.is_empty() {
        match Wire::next_event(&mut socket).await? {
            Event::Text(text) => texts.push(text),
            Event::Ping => {}
            other => panic!("expected unchanged, got {other:?}"),
        }
    }
    assert_eq!(texts, [ServerMessage::Unchanged { id: 2 }]);
    Wire::expect_quiet(&mut socket).await?;
    assert_eq!(harness.pool().waiting(WorkClass::Foreground), 0, "neither item waited for a slot");
    Ok(())
}

#[tokio::test]
async fn an_item_that_cannot_be_made_fails_with_its_kind_test() -> TestResult {
    let harness = Harness::start(ImageSocketLimits::default()).await?;
    std::fs::write(harness.library().join("notes.txt"), b"text")?;
    let mut socket = harness.open().await?;
    Wire::want(
        &mut socket,
        vec![
            Wire::wanted(1, "/photos/missing.png", "thumbnail", "visible", 0),
            Wire::wanted(2, "/photos/notes.txt", "thumbnail", "visible", 1),
            Wire::wanted(3, "/photos/missing.png", "thumbnail", "visible", 2),
        ],
    )
    .await?;
    let mut failures = HashMap::new();
    while failures.len() < 3 {
        match Wire::next_event(&mut socket).await? {
            Event::Text(ServerMessage::Failed { id, kind, .. }) => {
                failures.insert(id, serde_json::to_value(kind)?);
            }
            other => panic!("expected failed, got {other:?}"),
        }
    }
    assert_eq!(failures[&1], "NotFound");
    assert_eq!(failures[&2], "UnsupportedType");
    assert_eq!(failures[&3], "NotFound");
    Ok(())
}

#[tokio::test]
async fn items_go_to_generation_in_limited_number_and_the_interactive_one_goes_first_test() -> TestResult {
    let harness = Harness::start(ImageSocketLimits {
        max_generating: 3,
        ..ImageSocketLimits::default()
    })
    .await?;
    for index in 0..11 {
        harness.write_png(&format!("{index}.png"), 400, 300)?;
    }
    let mut held = harness.hold_all().await;
    let mut socket = harness.open().await?;

    // Ten files are wanted at once, but only three are passed to generation, and they are the first by order.
    let items = (0..10)
        .map(|index| Wire::wanted(index, &format!("/photos/{index}.png"), "thumbnail", "visible", index as u32))
        .collect();
    Wire::want(&mut socket, items).await?;
    Wire::until(|| harness.pool().waiting(WorkClass::Foreground) == 3).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(harness.pool().waiting(WorkClass::Foreground), 3, "no more than three at a time");

    // A display image wanted after them is passed at once, and gets the first slot that frees.
    Wire::want(&mut socket, vec![Wire::wanted(100, "/photos/10.png", "display", "interactive", 0)]).await?;
    Wire::until(|| harness.pool().waiting(WorkClass::Interactive) == 1).await;
    drop(held.remove(0));
    let (first, _) = Wire::read_images(&mut socket, 1).await?;
    assert_eq!(first[0].header.id, 100, "the interactive image comes before the ten that were wanted earlier");

    drop(held);
    let (rest, _) = Wire::read_images(&mut socket, 10).await?;
    let mut ids: Vec<u64> = rest.iter().map(|image| image.header.id).collect();
    ids.sort_unstable();
    assert_eq!(ids, (0..10).collect::<Vec<u64>>(), "every item is answered");
    Ok(())
}

#[tokio::test]
async fn a_sprite_carries_its_frame_count_and_a_thumbnail_shows_the_first_frame_test() -> TestResult {
    if Harness::skip_without_ffmpeg() {
        return Ok(());
    }
    let harness = Harness::start(ImageSocketLimits::default()).await?;
    let video = harness.library().join("clip.mp4");
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

    let mut socket = harness.open().await?;
    let item = |id, kind| {
        let mut item = Wire::wanted(id, "/photos/clip.mp4", kind, "visible", id as u32);
        item["width"] = json!(128);
        item["height"] = json!(128);
        item["shape"] = json!("fit");
        item
    };
    Wire::want(&mut socket, vec![item(1, "sprite"), item(2, "thumbnail")]).await?;
    let (images, _) = Wire::read_images(&mut socket, 2).await?;
    let find = |id| images.iter().find(|image| image.header.id == id).ok_or("the item was not answered");

    let sprite = find(1)?;
    assert_eq!(sprite.header.content_type, "image/webp");
    assert_eq!(sprite.header.frame_count, Some(8));
    let image = image::load_from_memory(&sprite.bytes)?;
    assert_eq!((image.width(), image.height()), (8 * 128, 96));

    let thumbnail = find(2)?;
    assert_eq!(thumbnail.header.frame_count, None);
    let image = image::load_from_memory(&thumbnail.bytes)?;
    assert_eq!((image.width(), image.height()), (128, 96), "a video thumbnail is its first frame");
    Ok(())
}

#[tokio::test]
async fn an_item_dropped_before_its_job_starts_is_never_made_test() -> TestResult {
    let harness = Harness::start(ImageSocketLimits {
        max_generating: 2,
        ..ImageSocketLimits::default()
    })
    .await?;
    for index in 0..6 {
        harness.write_png(&format!("{index}.png"), 400, 300)?;
    }
    // One job's worth of files in the cache, to count against.
    let mut warm = harness.open().await?;
    Wire::want(&mut warm, vec![Wire::wanted(0, "/photos/5.png", "thumbnail", "visible", 0)]).await?;
    Wire::read_images(&mut warm, 1).await?;
    drop(warm);
    let one_job = harness.cached_files();
    assert!(one_job > 0);

    let held = harness.hold_all().await;
    let mut socket = harness.open().await?;
    let items = (0..5)
        .map(|index| Wire::wanted(index, &format!("/photos/{index}.png"), "thumbnail", "visible", index as u32))
        .collect();
    Wire::want(&mut socket, items).await?;
    Wire::until(|| harness.pool().waiting(WorkClass::Foreground) == 2).await;

    // Items 2 to 4 were never passed to generation. Item 1 was, and waits for a slot; with nothing else in its job, it is discarded.
    Wire::drop_items(&mut socket, &[1, 2, 3, 4]).await?;
    Wire::until(|| harness.pool().waiting(WorkClass::Foreground) == 1).await;
    drop(held);

    let (images, _) = Wire::read_images(&mut socket, 1).await?;
    assert_eq!(images[0].header.id, 0);
    Wire::expect_quiet(&mut socket).await?;
    Wire::until(|| harness.pool().running(WorkClass::Foreground) == 0).await;
    assert_eq!(harness.cached_files(), 2 * one_job, "only one more job ran than the warm-up");
    Ok(())
}

#[tokio::test]
async fn the_socket_is_closed_with_4400_when_the_client_breaks_the_contract_or_a_limit_test() -> TestResult {
    let harness = Harness::start(ImageSocketLimits {
        max_items: 3,
        max_text_bytes: 2048,
        ..ImageSocketLimits::default()
    })
    .await?;
    // The file exists and every slot is taken, so a wanted item stays in the wanted set instead of ending at once.
    harness.write_png("a.png", 400, 300)?;
    harness.write_png("other.png", 400, 300)?;
    let _held = harness.hold_all().await;
    let item = |id: u64| Wire::wanted(id, "/photos/a.png", "thumbnail", "visible", 0);

    // Each case needs a socket of its own, because the daemon closes the one that breaks the contract.
    let cases: Vec<(&str, tungstenite::Message)> = vec![
        ("not JSON", tungstenite::Message::text("nonsense")),
        ("an unknown type", tungstenite::Message::text(json!({ "type": "reload" }).to_string())),
        (
            "an unknown field",
            tungstenite::Message::text(
                json!({ "type": "want", "items": [{ "id": 1, "libraryId": "photos", "path": "/a", "kind": "thumbnail", "class": "visible", "order": 0, "format": "jpeg" }] })
                    .to_string(),
            ),
        ),
        (
            "a width out of range",
            tungstenite::Message::text(
                json!({ "type": "want", "items": [{ "id": 1, "libraryId": "photos", "path": "/a", "kind": "thumbnail", "class": "visible", "order": 0, "width": 5000 }] })
                    .to_string(),
            ),
        ),
        ("a binary message", tungstenite::Message::binary(vec![1, 2, 3])),
        (
            "too many items",
            tungstenite::Message::text(json!({ "type": "want", "items": [item(1), item(2), item(3), item(4)] }).to_string()),
        ),
        (
            "a message over the text limit",
            tungstenite::Message::text(format!("{{\"type\":\"drop\",\"ids\":[{}]}}", "1,".repeat(2000) + "1")),
        ),
    ];
    for (name, message) in cases {
        let mut socket = harness.open().await?;
        socket.send(message).await?;
        let closed = tokio::time::timeout(Duration::from_secs(5), Wire::expect_close(&mut socket)).await;
        assert_eq!(closed.map_err(|_| format!("{name}: the socket was not closed"))??, Some(4400), "{name}");
    }

    // An id reused for another image breaks the contract too, and a socket that obeys it is not closed.
    let mut socket = harness.open().await?;
    Wire::want(&mut socket, vec![item(1)]).await?;
    Wire::want(&mut socket, vec![Wire::wanted(1, "/photos/other.png", "thumbnail", "visible", 0)]).await?;
    assert_eq!(Wire::expect_close(&mut socket).await?, Some(4400), "an id reused for another image");

    let mut socket = harness.open().await?;
    Wire::want(&mut socket, vec![item(1)]).await?;
    Wire::want(
        &mut socket,
        vec![{
            let mut moved = item(1);
            moved["class"] = json!("prefetch");
            moved
        }],
    )
    .await?;
    Wire::drop_items(&mut socket, &[1, 99]).await?;
    Wire::expect_quiet(&mut socket).await?;
    Ok(())
}

#[tokio::test]
async fn the_socket_is_closed_with_4401_after_logout_test() -> TestResult {
    let harness = Harness::start(ImageSocketLimits {
        ping_interval: Duration::from_millis(100),
        ..ImageSocketLimits::default()
    })
    .await?;
    let mut socket = harness.open().await?;
    let logout = send(
        &harness.router,
        Method::DELETE,
        "/api/v1/auth/session",
        &[("cookie", &harness.cookie), ("origin", "http://localhost")],
        None,
    )
    .await?;
    assert_eq!(logout.status, StatusCode::NO_CONTENT);

    assert_eq!(Wire::expect_close(&mut socket).await?, Some(4401), "closed at the next ping");
    Ok(())
}

#[tokio::test]
async fn the_socket_is_closed_with_1001_when_the_daemon_shuts_down_test() -> TestResult {
    let harness = Harness::start(ImageSocketLimits::default()).await?;
    let mut socket = harness.open().await?;
    harness.state().image_sockets.closing().cancel();
    assert_eq!(Wire::expect_close(&mut socket).await?, Some(1001));
    Ok(())
}

#[tokio::test]
async fn a_client_that_does_not_answer_pings_is_closed_test() -> TestResult {
    let harness = Harness::start(ImageSocketLimits {
        ping_interval: Duration::from_millis(100),
        ..ImageSocketLimits::default()
    })
    .await?;
    let mut socket = harness.open().await?;

    // The client does not read, so it answers nothing. Two pings go unanswered, and the next tick closes the socket.
    tokio::time::sleep(Duration::from_millis(600)).await;
    let mut pings = 0;
    let code = loop {
        match Wire::next_event(&mut socket).await? {
            Event::Ping => pings += 1,
            Event::Closed(code) => break code,
            other => panic!("unexpected {other:?}"),
        }
    };
    assert_eq!(pings, 2);
    assert_eq!(code, Some(1011));

    // A client that reads is not closed, however long it is idle.
    let mut alive = harness.open().await?;
    for _ in 0..5 {
        assert!(matches!(Wire::next_event(&mut alive).await?, Event::Ping));
    }
    Ok(())
}
