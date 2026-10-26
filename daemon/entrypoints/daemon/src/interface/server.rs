use std::sync::Arc;

use async_trait::async_trait;
use axum_extra::extract::CookieJar;
use http::HeaderMap;
use tokio_util::sync::CancellationToken;

use omnius_pxna_interface::{apis, apis::BasicAuthKind};

use crate::{
    interface::{
        features::{files::FileContentRoute, thumbnails::ThumbnailRoute, uploads::UploadContentRoute, video::VideoRoute},
        middleware::{OriginCheck, UnauthorizedBody},
        session::{BearerToken, Claims, SessionAuth, SessionCookie},
        web::WebAssets,
    },
    prelude::*,
    state::DaemonState,
};

pub struct ApiServer {
    pub(super) state: Arc<DaemonState>,
}

#[async_trait]
impl apis::CookieAuthentication for ApiServer {
    type Claims = Claims;

    // 生成されたルータが渡す `key` は固定の名前なので使わず、設定に応じた cookie 名で探す。
    async fn extract_claims_from_cookie(&self, cookies: &CookieJar, _key: &str) -> Option<Self::Claims> {
        self.authenticate(cookies.get(SessionCookie::from_config(&self.state.conf.web).name())?.value()).await
    }
}

#[allow(unused_variables)]
#[async_trait]
impl apis::ApiAuthBasic for ApiServer {
    type Claims = Claims;

    async fn extract_claims_from_auth_header(&self, kind: BasicAuthKind, headers: &HeaderMap, key: &str) -> Option<Self::Claims> {
        match kind {
            BasicAuthKind::Bearer => self.authenticate(BearerToken::parse(headers)?).await,
            _ => None,
        }
    }
}

impl omnius_pxna_interface::apis::ErrorHandler for ApiServer {}

impl ApiServer {
    async fn authenticate(&self, token: &str) -> Option<Claims> {
        SessionAuth::authenticate_token(&self.state, token).await
    }

    /// 手書きの route を持つ外側のルータに、生成されたルータを fallback として渡す。
    /// `[web] dir` があれば、`/api` の外の要求に web の成果物を返す層をさらに外側へ置き、全体を `Origin` の検証で包む。
    /// `serve` とテストは同じルータを使う。
    pub fn router(state: Arc<DaemonState>) -> axum::Router {
        let generated = omnius_pxna_interface::server::new(Arc::new(Self { state: state.clone() }));
        let api = axum::Router::new()
            .route(FileContentRoute::PATH, axum::routing::get(FileContentRoute::get).put(FileContentRoute::put))
            .route(UploadContentRoute::PATH, axum::routing::put(UploadContentRoute::put))
            .route(ThumbnailRoute::PATH, axum::routing::get(ThumbnailRoute::get))
            .route(VideoRoute::PLAYLIST, axum::routing::get(VideoRoute::playlist))
            .route(VideoRoute::SEGMENT, axum::routing::get(VideoRoute::segment))
            .with_state(state.clone())
            .fallback_service(generated);
        let app = match &state.conf.web.dir {
            Some(dir) => WebAssets::new(dir).router(api),
            None => api,
        };
        app.layer(axum::middleware::from_fn(UnauthorizedBody::middleware))
            .layer(axum::middleware::from_fn(OriginCheck::middleware))
    }

    pub async fn serve(state: Arc<DaemonState>, token: CancellationToken) -> Result<()> {
        let app = Self::router(state.clone());

        let listener = tokio::net::TcpListener::bind(state.conf.core.listen_addr.clone()).await?;
        info!(addr = state.conf.core.listen_addr, "listen start");
        axum::serve(listener, app).with_graceful_shutdown(token.cancelled_owned()).await?;

        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::path::Path;

    use axum::body::Body;
    use chrono::Utc;
    use headers::Host;
    use http::{Method, StatusCode};
    use omnius_pxna_engine::{
        model::{Session, SessionTransport, User, UserRole},
        service::ArchiveOptions,
    };
    use omnius_pxna_interface::models;
    use serde_json::{Value, json};
    use testresult::TestResult;
    use tower::ServiceExt as _;

    use super::*;
    use crate::{
        config::{CoreConfig, DaemonConfig, LibraryConfig, LoggingConfig, StorageConfig, WebConfig},
        db::DatabaseBootstrap,
        error::ApiError,
        interface::binary_body::BinaryBody,
        service::{
            listing::ListOptions,
            mutation::{DeleteResult, LibraryMutation, MoveResult},
        },
    };

    /// ハンドラを直接呼ぶテストが渡す、照合済みとみなしたユーザとセッション。
    pub(crate) fn claims() -> Claims {
        let now = Utc::now();
        Claims {
            user: User {
                id: "usr_test".to_string(),
                username: "owner".to_string(),
                role: UserRole::Owner,
                created_at: now,
                updated_at: now,
            },
            session: Session {
                id: "ses_test".to_string(),
                user_id: "usr_test".to_string(),
                transport: SessionTransport::Cookie,
                created_at: now,
                expires_at: now,
                last_seen_at: None,
            },
        }
    }

    pub(crate) struct HttpResponse {
        pub(crate) status: StatusCode,
        pub(crate) headers: HeaderMap,
        pub(crate) body: Value,
        pub(crate) bytes: Vec<u8>,
    }

    impl HttpResponse {
        pub(crate) fn set_cookie(&self) -> Option<&str> {
            self.headers.get(http::header::SET_COOKIE)?.to_str().ok()
        }

        pub(crate) fn header(&self, name: &str) -> Option<&str> {
            self.headers.get(name)?.to_str().ok()
        }

        /// 画像の応答を decode して、大きさを返す。
        pub(crate) fn image_size(&self) -> TestResult<(u32, u32)> {
            let image = image::load_from_memory(&self.bytes)?;
            Ok((image.width(), image.height()))
        }
    }

    pub(crate) fn router(fixture: &Fixture) -> axum::Router {
        ApiServer::router(fixture.server.state.clone())
    }

    pub(crate) async fn send(router: &axum::Router, method: Method, uri: &str, headers: &[(&str, &str)], body: Option<Value>) -> TestResult<HttpResponse> {
        let mut request = http::Request::builder().method(method).uri(uri).header(http::header::HOST, "localhost");
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        let request = match body {
            Some(body) => request
                .header(http::header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&body)?))?,
            None => request.body(Body::empty())?,
        };

        let response = router.clone().oneshot(request).await?;
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
        // 画像の応答の本文は JSON ではないので、読めなければ `Null` にして、`bytes` から読む。
        let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        Ok(HttpResponse {
            status,
            headers,
            body,
            bytes: bytes.to_vec(),
        })
    }

    pub(crate) fn credentials(username: &str, password: &str, transport: &str) -> Option<Value> {
        Some(json!({ "username": username, "password": password, "sessionTransport": transport }))
    }

    /// `Set-Cookie` から cookie の `name=value` の部分だけを取り出し、要求の `Cookie` に使える形にする。
    pub(crate) fn cookie(response: &HttpResponse) -> TestResult<String> {
        let set_cookie = response.set_cookie().ok_or("Set-Cookie is missing")?;
        Ok(set_cookie.split(';').next().ok_or("empty Set-Cookie")?.to_string())
    }

    pub(crate) fn bearer(response: &HttpResponse) -> TestResult<String> {
        let token = response.body["accessToken"].as_str().ok_or("accessToken is missing")?;
        Ok(format!("Bearer {token}"))
    }

    pub(crate) fn write_7z(path: &Path, entries: &[(&str, &[u8])]) -> TestResult {
        let mut writer = sevenz_rust2::ArchiveWriter::create(path)?;
        for (name, content) in entries {
            writer.push_archive_entry(sevenz_rust2::ArchiveEntry::new_file(name), Some(std::io::Cursor::new(content.to_vec())))?;
        }
        writer.finish()?;
        Ok(())
    }

    pub(crate) struct Fixture {
        pub(crate) _dir: tempfile::TempDir,
        pub(crate) server: ApiServer,
        pub(crate) archive: std::path::PathBuf,
    }

    pub(crate) async fn fixture(options: ArchiveOptions) -> TestResult<Fixture> {
        fixture_with_web(options, None).await
    }

    pub(crate) async fn fixture_with_web(options: ArchiveOptions, web_dir: Option<std::path::PathBuf>) -> TestResult<Fixture> {
        fixture_with_clock(options, web_dir, Arc::new(omnius_core_base::clock::ClockUtc)).await
    }

    pub(crate) async fn fixture_with_clock(
        options: ArchiveOptions,
        web_dir: Option<std::path::PathBuf>,
        clock: Arc<dyn omnius_core_base::clock::Clock<chrono::Utc> + Send + Sync>,
    ) -> TestResult<Fixture> {
        fixture_with_thumbnail(options, web_dir, clock, Default::default()).await
    }

    pub(crate) async fn fixture_with_thumbnail(
        options: ArchiveOptions,
        web_dir: Option<std::path::PathBuf>,
        clock: Arc<dyn omnius_core_base::clock::Clock<chrono::Utc> + Send + Sync>,
        thumbnail: omnius_pxna_engine::service::ThumbnailOptions,
    ) -> TestResult<Fixture> {
        let dir = tempfile::tempdir()?;
        let library = dir.path().join("library");
        std::fs::create_dir_all(library.join("photos"))?;
        let archive = library.join("photos/a.7z");
        write_7z(&archive, &[("dir/x.png", b"pixel"), ("y.txt", b"y")])?;
        let inner = dir.path().join("inner.7z");
        write_7z(&inner, &[("x.png", b"inner pixel")])?;
        write_7z(&library.join("photos/outer.7z"), &[("inner.7z", &std::fs::read(&inner)?)])?;
        std::fs::write(library.join("photos/notes.txt"), b"plain")?;
        let videos = dir.path().join("videos");
        std::fs::create_dir_all(&videos)?;

        let conf = DaemonConfig {
            core: CoreConfig {
                state_dir: dir.path().join("state"),
                listen_addr: "127.0.0.1:0".to_string(),
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                json: false,
                file: None,
                performance: false,
            },
            libraries: vec![
                LibraryConfig {
                    id: "photos".to_string(),
                    name: "Photos".to_string(),
                    path: library,
                    readonly: false,
                },
                LibraryConfig {
                    id: "videos".to_string(),
                    name: "Videos".to_string(),
                    path: videos,
                    readonly: true,
                },
            ],
            storage: StorageConfig { archive: options },
            listing: Default::default(),
            timeline: Default::default(),
            thumbnail,
            scheduler: Default::default(),
            media: Default::default(),
            video: Default::default(),
            web: WebConfig {
                dir: web_dir,
                insecure_cookie: false,
            },
        };
        let state = Arc::new(DaemonState::new_with_clock(conf, clock).await?);
        Ok(Fixture {
            _dir: dir,
            server: ApiServer { state },
            archive,
        })
    }

    pub(crate) async fn binary(router: &axum::Router, uri: &str, bearer: &str, bytes: Vec<u8>) -> TestResult<HttpResponse> {
        let request = http::Request::builder()
            .method(Method::PUT)
            .uri(uri)
            .header(http::header::HOST, "localhost")
            .header(http::header::AUTHORIZATION, bearer)
            .header(http::header::CONTENT_TYPE, "application/octet-stream")
            .body(Body::from(bytes))?;
        let response = router.clone().oneshot(request).await?;
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await?.to_vec();
        let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        Ok(HttpResponse { status, headers, bytes, body })
    }

    pub(crate) async fn wait_operation(state: &DaemonState, id: &str) -> TestResult<models::Operation> {
        Ok(tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                let operation = state.operations.get(id)?;
                if !matches!(operation.status, models::OperationStatus::Pending | models::OperationStatus::Running) {
                    return Ok::<_, Error>(operation);
                }
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
        })
        .await??)
    }

    pub(crate) fn host() -> Host {
        Host::from(http::uri::Authority::from_static("localhost"))
    }

    #[tokio::test]
    async fn synchronous_mutations_and_binary_limit_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let router = router(&fixture);
        let setup = send(&router, Method::POST, "/api/v1/setup", &[], credentials("owner", "password1", "bearer")).await?;
        let bearer = bearer(&setup)?;
        let headers = &[("authorization", bearer.as_str())];
        let create = |path: &str, parents: bool| Some(json!({"path": path, "createParents": parents}));
        assert_eq!(
            send(&router, Method::POST, "/api/v1/libraries/photos/folders", headers, create("/new/deep", false))
                .await?
                .status,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            send(&router, Method::POST, "/api/v1/libraries/photos/folders", headers, create("/new/deep", true))
                .await?
                .status,
            StatusCode::CREATED
        );
        assert_eq!(
            send(&router, Method::POST, "/api/v1/libraries/photos/folders", headers, create("/new/deep", true))
                .await?
                .status,
            StatusCode::CONFLICT
        );
        for (id, path) in [("videos", "/forbidden"), ("photos", "/photos/a.7z/forbidden")] {
            assert_eq!(
                send(&router, Method::POST, &format!("/api/v1/libraries/{id}/folders"), headers, create(path, true))
                    .await?
                    .status,
                StatusCode::FORBIDDEN
            );
        }
        let written = binary(&router, "/api/v1/libraries/photos/files/content?path=/new/deep/file.bin", &bearer, vec![7; 3 * 1024 * 1024]).await?;
        assert_eq!(written.status, StatusCode::CREATED, "生成 router の既定 2 MiB 上限を使わない");
        assert_eq!(written.body["file"]["path"], "/new/deep/file.bin");
        let too_large = binary(
            &router,
            "/api/v1/libraries/photos/files/content?path=/new/too-large.bin",
            &bearer,
            vec![0; BinaryBody::MAX_BYTES + 1],
        )
        .await?;
        assert_eq!(too_large.status, StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(too_large.body["kind"], "PayloadTooLarge");
        assert!(!fixture.server.state.conf.libraries[0].path.join("new/too-large.bin").exists());
        let moved = send(
            &router,
            Method::POST,
            "/api/v1/libraries/photos/files/move",
            headers,
            Some(json!({"srcPaths":["/new/deep/file.bin"],"dstPath":"/new/moved.bin"})),
        )
        .await?;
        assert_eq!(moved.status, StatusCode::OK);
        assert_eq!(
            send(&router, Method::GET, "/api/v1/libraries/photos/files/content?path=/new/moved.bin&limit=2", headers, None)
                .await?
                .bytes,
            vec![7, 7]
        );
        assert_eq!(
            send(
                &router,
                Method::POST,
                "/api/v1/libraries/photos/files/delete",
                headers,
                Some(json!({"srcPaths":["/new/moved.bin"]}))
            )
            .await?
            .status,
            StatusCode::NO_CONTENT
        );
        assert_eq!(
            send(
                &router,
                Method::POST,
                "/api/v1/libraries/photos/folders/delete",
                headers,
                Some(json!({"srcPaths":["/photos"]}))
            )
            .await?
            .status,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            send(
                &router,
                Method::POST,
                "/api/v1/libraries/photos/folders/delete",
                headers,
                Some(json!({"srcPaths":["/new/deep"]}))
            )
            .await?
            .status,
            StatusCode::NO_CONTENT
        );
        Ok(())
    }

    async fn cached_listing(state: &DaemonState, path: &str, recursive: bool) -> TestResult<models::FolderListing> {
        let mut query = crate::service::listing::tests::query(None, None);
        query.recursive = Some(recursive);
        let options = ListOptions::from_query(&query)?;
        Ok(state.library.list_folder_entries("photos", path, &options).await?)
    }

    async fn prime_listings(state: &DaemonState, clock: &omnius_core_base::clock::FakeClockUtc) -> TestResult<Vec<(String, bool, models::FolderListing, std::time::SystemTime)>> {
        let latest = ["", "changes", "changes/nested"]
            .iter()
            .map(|path| std::fs::metadata(state.conf.libraries[0].path.join(path)).and_then(|metadata| metadata.modified()))
            .collect::<std::io::Result<Vec<_>>>()?
            .into_iter()
            .max()
            .unwrap();
        clock.set(chrono::DateTime::<chrono::Utc>::from(latest) + chrono::Duration::seconds(3));
        let mut listings = Vec::new();
        for path in ["/", "/changes", "/changes/nested"] {
            let folder = state.conf.libraries[0].path.join(path.trim_start_matches('/'));
            for recursive in [false, true] {
                let first = cached_listing(state, path, recursive).await?;
                let repeated = cached_listing(state, path, recursive).await?;
                assert_eq!(first.snapshot, repeated.snapshot, "先に保持されたことを確かめる: {path}");
                listings.push((path.to_string(), recursive, first, std::fs::metadata(&folder)?.modified()?));
            }
        }
        Ok(listings)
    }

    #[cfg(not(windows))]
    fn open_folder_for_times(folder: &std::path::Path) -> std::io::Result<std::fs::File> {
        std::fs::File::open(folder)
    }

    /// Windows で folder を開くには backup semantics が要り、時刻を書くには属性の書き込み権が要る。
    #[cfg(windows)]
    fn open_folder_for_times(folder: &std::path::Path) -> std::io::Result<std::fs::File> {
        use std::os::windows::fs::OpenOptionsExt;

        const FILE_WRITE_ATTRIBUTES: u32 = 0x0100;
        const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
        std::fs::OpenOptions::new()
            .access_mode(FILE_WRITE_ATTRIBUTES)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(folder)
    }

    async fn changed_listings(state: &DaemonState, before: Vec<(String, bool, models::FolderListing, std::time::SystemTime)>) -> TestResult<Vec<models::FolderListing>> {
        let mut listings = Vec::new();
        for (path, recursive, old, modified) in before {
            let folder = state.conf.libraries[0].path.join(path.trim_start_matches('/'));
            // mtime の版を前に戻し、明示的な破棄だけで id が変わることを確かめる。
            open_folder_for_times(&folder)?.set_modified(modified)?;
            let new = cached_listing(state, &path, recursive).await?;
            assert_ne!(old.snapshot, new.snapshot, "変更した Entry の祖先: {path}, recursive={recursive}");
            listings.push(new);
        }
        Ok(listings)
    }

    #[tokio::test]
    async fn listing_reflects_own_mutations_test() -> TestResult {
        let clock = Arc::new(omnius_core_base::clock::FakeClockUtc::new(chrono::Utc::now() + chrono::Duration::seconds(30)));
        let fixture = fixture_with_clock(ArchiveOptions::default(), None, clock.clone()).await?;
        let state = &fixture.server.state;
        let root = &state.conf.libraries[0].path;
        std::fs::create_dir_all(root.join("changes/nested"))?;

        let before = prime_listings(state, &clock).await?;
        state
            .mutation
            .create_folder("photos", &models::CreateFolderRequest::new("/changes/nested/folder".to_string()))
            .await?;
        let after = changed_listings(state, before).await?;
        assert!(after.last().unwrap().entries.iter().any(|entry| entry.path == "/changes/nested/folder"));

        let before = prime_listings(state, &clock).await?;
        state
            .mutation
            .write("photos", "/changes/nested/written.txt", b"write", LibraryMutation::conflict(None))
            .await?;
        let after = changed_listings(state, before).await?;
        assert!(after.last().unwrap().entries.iter().any(|entry| entry.name == "written.txt"));

        let before = prime_listings(state, &clock).await?;
        state
            .mutation
            .write(
                "photos",
                "/changes/nested/written.txt",
                b"longer write",
                LibraryMutation::conflict(Some(models::ConflictPolicy::Overwrite)),
            )
            .await?;
        let after = changed_listings(state, before).await?;
        assert_eq!(after.last().unwrap().entries.iter().find(|entry| entry.name == "written.txt").unwrap().metadata.size, 12);

        let upload = state
            .uploads
            .create("photos", &models::UploadCreateRequest::new("/changes/nested/upload.txt".to_string(), 6))
            .await?;
        state.uploads.write("photos", &upload.id, 0, b"upload").await?;
        let before = prime_listings(state, &clock).await?;
        state.uploads.commit("photos", &upload.id).await?;
        let after = changed_listings(state, before).await?;
        assert!(after.last().unwrap().entries.iter().any(|entry| entry.name == "upload.txt"));

        let before = prime_listings(state, &clock).await?;
        state.mutation.delete("photos", "/changes/nested/upload.txt", false).await?;
        let after = changed_listings(state, before).await?;
        assert!(!after.last().unwrap().entries.iter().any(|entry| entry.name == "upload.txt"));

        let before = prime_listings(state, &clock).await?;
        let moved: models::EntryTransferRequest = serde_json::from_value(json!({"srcPaths":["/changes/nested/written.txt"],"dstPath":"/changes/nested/moved.txt"}))?;
        assert!(matches!(state.operations.move_entries("photos", &moved, false).await?, MoveResult::Completed(_)));
        let after = changed_listings(state, before).await?;
        let entries = &after.last().unwrap().entries;
        assert!(entries.iter().any(|entry| entry.name == "moved.txt"));
        assert!(!entries.iter().any(|entry| entry.name == "written.txt"));

        let before = prime_listings(state, &clock).await?;
        let copy: models::EntryTransferRequest = serde_json::from_value(json!({"srcPaths":["/changes/nested/moved.txt"],"dstPath":"/changes/nested/copied.txt"}))?;
        let operation = state.operations.copy("photos", &copy, false).await?;
        assert_eq!(wait_operation(state, &operation.id).await?.status, models::OperationStatus::Succeeded);
        let after = changed_listings(state, before).await?;
        assert!(after.last().unwrap().entries.iter().any(|entry| entry.name == "copied.txt"));

        let before = prime_listings(state, &clock).await?;
        assert!(matches!(
            state
                .operations
                .delete_entries("photos", &[models::RelativePath("/changes/nested/copied.txt".to_string())], false, false)
                .await?,
            DeleteResult::Completed
        ));
        let after = changed_listings(state, before).await?;
        assert!(!after.last().unwrap().entries.iter().any(|entry| entry.name == "copied.txt"));

        std::fs::create_dir_all(root.join("source/deep"))?;
        std::fs::create_dir_all(root.join("changes/nested/merge/deep"))?;
        std::fs::write(root.join("source/deep/replaced.txt"), b"new content")?;
        std::fs::write(root.join("changes/nested/merge/deep/replaced.txt"), b"old")?;
        let before = prime_listings(state, &clock).await?;
        let child_path = "/changes/nested/merge/deep";
        let child_before = cached_listing(state, child_path, true).await?;
        let child_dir = root.join(child_path.trim_start_matches('/'));
        let child_modified = std::fs::metadata(&child_dir)?.modified()?;
        let merge: models::EntryTransferRequest = serde_json::from_value(json!({"srcPaths":["/source"],"dstPath":"/changes/nested/merge","conflict":"overwrite"}))?;
        let operation = state.operations.copy("photos", &merge, true).await?;
        assert_eq!(wait_operation(state, &operation.id).await?.status, models::OperationStatus::Succeeded);
        changed_listings(state, before).await?;
        std::fs::File::open(&child_dir)?.set_modified(child_modified)?;
        let child_after = cached_listing(state, child_path, true).await?;
        assert_ne!(child_before.snapshot, child_after.snapshot, "再帰の Operation が変更した子 Entry の祖先も捨てる");
        assert_eq!(child_after.entries.iter().find(|entry| entry.name == "replaced.txt").unwrap().metadata.size, 11);

        let before = prime_listings(state, &clock).await?;
        let operation = state
            .operations
            .delete_entries("photos", &[models::RelativePath("/changes/nested/merge".to_string())], true, true)
            .await?;
        let DeleteResult::Started(operation) = operation else {
            panic!("recursive deletion must start an Operation");
        };
        assert_eq!(wait_operation(state, &operation.id).await?.status, models::OperationStatus::Succeeded);
        let after = changed_listings(state, before).await?;
        assert!(!after.last().unwrap().entries.iter().any(|entry| entry.path.starts_with("/changes/nested/merge")));
        Ok(())
    }

    #[tokio::test]
    async fn authenticated_routes_require_a_live_session_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let router = router(&fixture);
        let setup = send(&router, Method::POST, "/api/v1/setup", &[], credentials("owner", "password1", "cookie")).await?;
        let login = send(&router, Method::POST, "/api/v1/auth/login", &[], credentials("owner", "password1", "bearer")).await?;
        let (cookie, bearer) = (cookie(&setup)?, bearer(&login)?);

        let status = |headers: &'static [(&'static str, &'static str)]| send(&router, Method::GET, "/api/v1/libraries", headers, None);
        for headers in [&[][..], &[("authorization", "Bearer invalid")][..]] {
            let response = status(headers).await?;
            assert_eq!(response.status, StatusCode::UNAUTHORIZED);
            assert_eq!(response.body["kind"], "Unauthorized");
            assert!(response.header("content-type").is_some_and(|value| value.starts_with("application/json")));
            assert!(response.header("content-length").is_none(), "本文を差し替えた際に空応答の長さを残さない");
        }
        assert_eq!(send(&router, Method::GET, "/api/v1/libraries", &[("cookie", &cookie)], None).await?.status, StatusCode::OK);
        assert_eq!(
            send(&router, Method::GET, "/api/v1/libraries", &[("authorization", &bearer)], None).await?.status,
            StatusCode::OK
        );

        // 作成から 30 日を過ぎたセッションを、失効の時刻を過去へずらして再現する。
        let db = DatabaseBootstrap::connect(fixture._dir.path().join("state")).await?;
        sqlx::query("UPDATE sessions SET expires_at = ?1 WHERE transport = 'bearer'")
            .bind(Utc::now() - chrono::Duration::seconds(1))
            .execute(&db)
            .await?;
        assert_eq!(
            send(&router, Method::GET, "/api/v1/libraries", &[("authorization", &bearer)], None).await?.status,
            StatusCode::UNAUTHORIZED
        );
        Ok(())
    }

    #[tokio::test]
    async fn timeline_follows_a_merging_folder_move_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let state = &fixture.server.state;
        let root = state.conf.libraries[0].path.clone();
        std::fs::create_dir_all(root.join("src/sub"))?;
        std::fs::create_dir_all(root.join("dst"))?;
        std::fs::write(root.join("src/x.png"), b"x")?;
        std::fs::write(root.join("src/sub/y.png"), b"y")?;
        std::fs::write(root.join("dst/z.png"), b"z")?;
        state.timeline.refresh("photos").await?;
        for _ in 0..200 {
            let status = state.timeline.status("photos").await?;
            if !status.running && status.completed_at.is_some() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        let paths = || async {
            let page = state.timeline.page(&["photos".to_string()], &crate::service::timeline::TimelineAnchor::Newest, 100).await?;
            Ok::<_, Error>(page.rows.into_iter().map(|listed| listed.row.path).collect::<Vec<_>>())
        };
        assert!(paths().await?.contains(&"/src/sub/y.png".to_string()));

        let request: models::EntryTransferRequest = serde_json::from_value(json!({"srcPaths":["/src"],"dstPath":"/dst","conflict":"overwrite"}))?;
        assert!(matches!(state.operations.move_entries("photos", &request, true).await?, MoveResult::Completed(_)));
        let after = paths().await?;
        for expected in ["/dst/x.png", "/dst/sub/y.png", "/dst/z.png"] {
            assert!(after.contains(&expected.to_string()), "{expected} を応答の前に付け替える: {after:?}");
        }
        assert!(!after.iter().any(|path| path.starts_with("/src/")), "{after:?}");
        Ok(())
    }

    async fn timeline_paths(state: &DaemonState) -> TestResult<Vec<String>> {
        let page = state
            .timeline
            .page(&["photos".to_string()], &crate::service::timeline::TimelineAnchor::Newest, 1000)
            .await?;
        Ok(page.rows.into_iter().map(|listed| listed.row.path).collect())
    }

    async fn timeline_scanned(state: &DaemonState) -> TestResult {
        state.timeline.refresh("photos").await?;
        for _ in 0..400 {
            let status = state.timeline.status("photos").await?;
            if !status.running && status.completed_at.is_some() {
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        Err("timeline scan did not finish".into())
    }

    #[tokio::test]
    async fn timeline_drops_archives_replaced_by_a_folder_copy_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let state = &fixture.server.state;
        let root = state.conf.libraries[0].path.clone();
        std::fs::create_dir_all(root.join("src"))?;
        std::fs::create_dir_all(root.join("dst"))?;
        write_7z(&root.join("src/c.7z"), &[("new.png", b"new")])?;
        write_7z(&root.join("dst/c.7z"), &[("old.png", b"old pixel")])?;
        timeline_scanned(state).await?;
        assert!(timeline_paths(state).await?.contains(&"/dst/c.7z/old.png".to_string()));

        let request: models::EntryTransferRequest = serde_json::from_value(json!({"srcPaths":["/src"],"dstPath":"/dst","conflict":"overwrite"}))?;
        let operation = state.operations.copy("photos", &request, true).await?;
        assert_eq!(wait_operation(state, &operation.id).await?.status, models::OperationStatus::Succeeded);
        let after = timeline_paths(state).await?;
        assert!(!after.contains(&"/dst/c.7z/old.png".to_string()), "操作の完了までに旧い archive の Entry を消す: {after:?}");
        Ok(())
    }

    #[tokio::test]
    async fn timeline_adds_files_published_under_another_name_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let state = &fixture.server.state;
        let root = state.conf.libraries[0].path.clone();
        std::fs::write(root.join("dup.png"), b"first")?;
        timeline_scanned(state).await?;
        assert!(timeline_paths(state).await?.contains(&"/dup.png".to_string()));

        state
            .mutation
            .write("photos", "/dup.png", b"second", LibraryMutation::conflict(Some(models::ConflictPolicy::Rename)))
            .await?;
        let mut found = false;
        for _ in 0..200 {
            if timeline_paths(state).await?.iter().any(|path| path.starts_with("/dup (")) {
                found = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        assert!(found, "別名で公開したファイルは範囲の集計で足される");
        Ok(())
    }

    #[tokio::test]
    async fn timeline_drops_a_renamed_move_to_an_unsupported_name_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let state = &fixture.server.state;
        let root = state.conf.libraries[0].path.clone();
        std::fs::write(root.join("photo.png"), b"p")?;
        std::fs::write(root.join("target.txt"), b"t")?;
        timeline_scanned(state).await?;
        assert!(timeline_paths(state).await?.contains(&"/photo.png".to_string()));

        // 既存の target.txt と衝突するので、`target (1).txt` として公開される。対象外の拡張子なので、タイムラインから消える。
        let request: models::EntryTransferRequest = serde_json::from_value(json!({"srcPaths":["/photo.png"],"dstPath":"/target.txt","conflict":"rename"}))?;
        assert!(matches!(state.operations.move_entries("photos", &request, false).await?, MoveResult::Completed(_)));
        assert!(root.join("target (1).txt").exists());
        let mut gone = false;
        for _ in 0..200 {
            if timeline_paths(state).await?.iter().all(|path| !path.contains("target") && path != "/photo.png") {
                gone = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        assert!(gone, "範囲の集計が別名の宛先を判定し直す");
        Ok(())
    }

    #[tokio::test]
    async fn timeline_routes_list_scanned_entries_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let router = router(&fixture);
        send(&router, Method::POST, "/api/v1/setup", &[], credentials("owner", "password1", "cookie")).await?;
        let login = send(&router, Method::POST, "/api/v1/auth/login", &[], credentials("owner", "password1", "bearer")).await?;
        let bearer = bearer(&login)?;
        let auth = [("authorization", bearer.as_str())];

        assert_eq!(
            send(&router, Method::GET, "/api/v1/timeline/months?libraryIds=photos", &[], None).await?.status,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            send(&router, Method::GET, "/api/v1/timeline/months?libraryIds=missing", &auth, None).await?.status,
            StatusCode::NOT_FOUND
        );

        let refresh = send(&router, Method::POST, "/api/v1/timeline/refresh", &auth, Some(json!({"libraryIds": ["photos"]}))).await?;
        assert_eq!(refresh.status, StatusCode::ACCEPTED);
        for _ in 0..200 {
            let status = send(&router, Method::GET, "/api/v1/timeline/status?libraryIds=photos", &auth, None).await?;
            assert_eq!(status.status, StatusCode::OK);
            let status = &status.body["libraries"][0];
            if status["running"] == false && status.get("completedAt").is_some() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }

        let entries = send(&router, Method::GET, "/api/v1/timeline/entries?libraryIds=photos", &auth, None).await?;
        assert_eq!(entries.status, StatusCode::OK);
        let mut paths = entries.body["entries"]
            .as_array()
            .ok_or("entries is missing")?
            .iter()
            .map(|entry| entry["entry"]["path"].as_str().unwrap_or_default().to_string())
            .collect::<Vec<_>>();
        paths.sort();
        assert_eq!(paths, ["/photos/a.7z/dir/x.png", "/photos/outer.7z/inner.7z/x.png"], "画像だけを並べ、archive の中も含める");
        let first = &entries.body["entries"][0];
        assert!(first["timelineDate"].as_str().is_some_and(|date| date.len() == 19));
        assert!(["modified", "archiveModified"].contains(&first["timelineDateSource"].as_str().unwrap_or_default()));
        assert_eq!(first["entry"]["metadata"]["contentType"], "image/png");
        assert_eq!(first["entry"]["metadata"]["readonly"], true);
        assert_eq!(first["libraryId"], "photos");
        assert!(entries.body.get("previous").is_none() && entries.body.get("next").is_none());

        let months = send(&router, Method::GET, "/api/v1/timeline/months?libraryIds=photos", &auth, None).await?;
        assert_eq!(
            months.body["months"]
                .as_array()
                .ok_or("months is missing")?
                .iter()
                .map(|month| month["count"].as_u64().unwrap_or(0))
                .sum::<u64>(),
            2
        );

        let one = send(&router, Method::GET, "/api/v1/timeline/entries?libraryIds=photos&limit=1", &auth, None).await?;
        let next = one.body["next"].as_str().ok_or("next cursor is missing")?.to_string();
        assert!(one.body.get("previous").is_none());
        let second = send(
            &router,
            Method::GET,
            &format!("/api/v1/timeline/entries?libraryIds=photos&limit=1&after={}", next.replace(':', "%3A").replace('/', "%2F")),
            &auth,
            None,
        )
        .await?;
        assert_eq!(second.status, StatusCode::OK);
        assert_ne!(second.body["entries"][0]["entry"]["path"], one.body["entries"][0]["entry"]["path"]);
        assert!(second.body.get("next").is_none());

        for query in [
            "after=x&month=2024-01",
            "after=invalid",
            "month=2024-13",
            "around=/photos/missing.png&aroundLibraryId=photos",
        ] {
            let response = send(&router, Method::GET, &format!("/api/v1/timeline/entries?libraryIds=photos&{query}"), &auth, None).await?;
            assert!(
                response.status == StatusCode::BAD_REQUEST || response.status == StatusCode::NOT_FOUND,
                "{query}: {}",
                response.status
            );
        }
        Ok(())
    }

    #[tokio::test]
    async fn router_checks_origin_of_cookie_requests_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let router = router(&fixture);
        let setup = send(&router, Method::POST, "/api/v1/setup", &[], credentials("owner", "password1", "cookie")).await?;
        let cookie = cookie(&setup)?;

        let missing = send(&router, Method::DELETE, "/api/v1/auth/session", &[("cookie", &cookie)], None).await?;
        assert_eq!(missing.status, StatusCode::FORBIDDEN);
        assert_eq!(missing.body["kind"], "Forbidden");
        let foreign = send(
            &router,
            Method::DELETE,
            "/api/v1/auth/session",
            &[("cookie", &cookie), ("origin", "http://evil.example")],
            None,
        )
        .await?;
        assert_eq!(foreign.status, StatusCode::FORBIDDEN);

        assert_eq!(
            send(&router, Method::GET, "/api/v1/auth/me", &[("cookie", &cookie)], None).await?.status,
            StatusCode::OK,
            "失効していない"
        );
        Ok(())
    }

    #[test]
    fn timeout_maps_to_gateway_timeout_responses_test() {
        let timeout = || ApiError::from(Error::from(omnius_pxna_engine::Error::new(omnius_pxna_engine::ErrorKind::Timeout)));
        assert!(matches!(
            apis::folders::GetFolderResponse::from(timeout()),
            apis::folders::GetFolderResponse::Status504_ExtractionOrThumbnailGenerationTimedOut(_)
        ));
        assert!(matches!(
            apis::folders::ListFolderEntriesResponse::from(timeout()),
            apis::folders::ListFolderEntriesResponse::Status504_ExtractionOrThumbnailGenerationTimedOut(_)
        ));
        assert!(matches!(
            apis::files::GetFileResponse::from(timeout()),
            apis::files::GetFileResponse::Status504_ExtractionOrThumbnailGenerationTimedOut(_)
        ));
        assert!(matches!(
            apis::files::ReadFileContentResponse::from(timeout()),
            apis::files::ReadFileContentResponse::Status504_ExtractionOrThumbnailGenerationTimedOut(_)
        ));
    }
}
