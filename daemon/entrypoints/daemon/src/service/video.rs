use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    pin::Pin,
    process::Stdio,
    sync::Arc,
    task::{Context, Poll},
    time::{Duration, Instant},
};

use omnius_core_base::tsid::TsidProvider;
use omnius_pxna_engine::{
    perf_span,
    service::{MaterializedFile, MediaInfoService, MediaOptions, WorkClass, WorkPriorityPool},
};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncRead, ReadBuf},
    process::{Child, Command},
};
use tokio_util::sync::CancellationToken;
use tracing::{Instrument as _, field::Empty};

use crate::{
    prelude::*,
    service::{library::LibraryRelativePath, mutation::LibraryMutation},
};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct VideoOptions {
    pub max_concurrent_transcodes: usize,
    pub segment_secs: u32,
    pub restart_threshold_segments: u32,
    pub max_height: u32,
    pub encoder: String,
    pub encoder_args: Vec<String>,
    pub session_idle_timeout_secs: u64,
}

impl Default for VideoOptions {
    fn default() -> Self {
        Self {
            max_concurrent_transcodes: 1,
            segment_secs: 6,
            restart_threshold_segments: 3,
            max_height: 1080,
            encoder: "libx264".to_string(),
            encoder_args: vec!["-preset".to_string(), "veryfast".to_string(), "-crf".to_string(), "23".to_string()],
            session_idle_timeout_secs: 300,
        }
    }
}

impl VideoOptions {
    pub fn validate(&self) -> Result<()> {
        if self.max_concurrent_transcodes == 0 || self.segment_secs == 0 || self.max_height < 2 || self.encoder.is_empty() || self.session_idle_timeout_secs == 0 {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("video limits and encoder must be positive"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoSessionInfo {
    pub id: String,
    pub library_id: String,
    pub path: String,
    pub playlist_url: String,
    pub duration_secs: f64,
}

struct Transcode {
    child: Option<Child>,
    start: u32,
    frontier: u32,
    generation: u64,
}

struct VideoSession {
    info: VideoSessionInfo,
    touched: Mutex<Instant>,
    cancel: CancellationToken,
    process: TokioMutex<Transcode>,
    file: Option<MaterializedFile>,
    directory: Option<tempfile::TempDir>,
    count: u32,
}

impl VideoSession {
    fn directory(&self) -> &Path {
        self.directory.as_ref().expect("live session has an output directory").path()
    }
    fn file(&self) -> &Path {
        self.file.as_ref().expect("live session has a materialized file").path()
    }
    fn segment(&self, index: u32) -> PathBuf {
        self.directory().join(format!("{index}.ts"))
    }
    fn touch(&self) {
        *self.touched.lock() = Instant::now();
    }
    fn reclaim_directory(directory: tempfile::TempDir) {
        let path = directory.path().to_path_buf();
        if let Err(error) = directory.close() {
            debug!(path = %path.display(), %error, "retrying video directory cleanup");
            tokio::spawn(async move {
                let mut last_error = error;
                let result = tokio::time::timeout(Duration::from_secs(5), async {
                    loop {
                        tokio::time::sleep(Duration::from_millis(50)).await;
                        match tokio::fs::remove_dir_all(&path).await {
                            Ok(()) => return,
                            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
                            Err(error) => last_error = error,
                        }
                    }
                })
                .await;
                if result.is_err() {
                    warn!(path = %path.display(), error = %last_error, "video directory cleanup failed; it will be retried at startup");
                }
            });
        }
    }
    async fn close(&self) {
        self.cancel.cancel();
        let mut process = self.process.lock().await;
        if let Some(mut child) = process.child.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
    }
}

impl Drop for VideoSession {
    fn drop(&mut self) {
        // ffmpeg の終了まで元 file の lease と出力 directory を保持する。
        let child = self.process.get_mut().child.take();
        let file = self.file.take();
        let directory = self.directory.take();
        if let Some(mut child) = child {
            tokio::spawn(async move {
                let _ = child.kill().await;
                let _ = child.wait().await;
                drop(file);
                if let Some(directory) = directory {
                    Self::reclaim_directory(directory);
                }
            });
        } else {
            drop(file);
            if let Some(directory) = directory {
                Self::reclaim_directory(directory);
            }
        }
    }
}

/// 出力セグメントを読んでいる間、セッションと directory を回収しない。
pub struct SegmentReader {
    file: Option<tokio::fs::File>,
    session: Arc<VideoSession>,
    pub len: u64,
}

impl AsyncRead for SegmentReader {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        self.session.touch();
        Pin::new(self.file.as_mut().expect("live reader has a segment file")).poll_read(cx, buf)
    }
}

impl Drop for SegmentReader {
    fn drop(&mut self) {
        if let Some(file) = self.file.take() {
            match file.try_into_std() {
                Ok(file) => drop(file),
                Err(file) => {
                    // pending I/O の handle が閉じるまで directory の lease を保持する。
                    let session = self.session.clone();
                    tokio::spawn(async move {
                        drop(file.into_std().await);
                        drop(session);
                    });
                }
            }
        }
    }
}

/// 1 本の再生につき ffmpeg を 1 つ保持し、シークと終了で資源を回収する。
pub struct VideoService {
    mutation: Arc<LibraryMutation>,
    media: Arc<MediaInfoService>,
    ids: Arc<Mutex<dyn TsidProvider + Send + Sync>>,
    options: VideoOptions,
    ffmpeg: PathBuf,
    root: PathBuf,
    sessions: TokioMutex<VecDeque<Arc<VideoSession>>>,
    /// セグメントを待たせている間、Interactive が動いているものとして数える外部プロセスの資源。
    videos: WorkPriorityPool,
}

impl VideoService {
    pub async fn open(
        mutation: Arc<LibraryMutation>,
        media: Arc<MediaInfoService>,
        ids: Arc<Mutex<dyn TsidProvider + Send + Sync>>,
        root: &Path,
        options: VideoOptions,
        tools: &MediaOptions,
        videos: WorkPriorityPool,
    ) -> Result<Arc<Self>> {
        options.validate()?;
        tokio::fs::create_dir_all(root).await?;
        let mut entries = tokio::fs::read_dir(root).await?;
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                tokio::fs::remove_dir_all(entry.path()).await?;
            } else {
                tokio::fs::remove_file(entry.path()).await?;
            }
        }
        let service = Arc::new(Self {
            mutation,
            media,
            ids,
            options,
            ffmpeg: tools.ffmpeg_path.clone(),
            root: root.to_path_buf(),
            sessions: TokioMutex::new(VecDeque::new()),
            videos,
        });
        let weak = Arc::downgrade(&service);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                let Some(service) = weak.upgrade() else {
                    break;
                };
                service.reap().await;
            }
        });
        Ok(service)
    }

    pub async fn create(&self, library: &str, path: &str) -> Result<VideoSessionInfo> {
        let span = perf_span!("video_session_create", library_id = library, path, ok = Empty);
        let result = self.create_traced(library, path).instrument(span.clone()).await;
        span.record("ok", result.is_ok());
        result
    }

    async fn create_traced(&self, library: &str, path: &str) -> Result<VideoSessionInfo> {
        let source = self.mutation.source(library, path, false, false).await?;
        let media = self.media.probe_video(&source).instrument(perf_span!("video_probe")).await?;
        let count = (media.duration_secs / f64::from(self.options.segment_secs)).ceil();
        if !(1.0..=100_000.0).contains(&count) || media.width < 2 || media.height < 2 {
            return Err(Error::new(ErrorKind::LimitExceeded).with_message("video has no playable segments or exceeds 100000 segments"));
        }
        let file = source.materialize_file().instrument(perf_span!("video_materialize")).await?;
        let id = format!("vid_{}", self.ids.lock().create());
        let info = VideoSessionInfo {
            playlist_url: format!("/api/v1/libraries/{library}/video/sessions/{id}/playlist.m3u8"),
            library_id: library.to_string(),
            path: format!("/{}", LibraryRelativePath::parse(path)?.segments().join("/")),
            id,
            duration_secs: media.duration_secs,
        };
        let directory = tempfile::Builder::new().prefix(&info.id).tempdir_in(&self.root)?;
        let session = Arc::new(VideoSession {
            info: info.clone(),
            touched: Mutex::new(Instant::now()),
            cancel: CancellationToken::new(),
            process: TokioMutex::new(Transcode {
                child: None,
                start: 0,
                frontier: 0,
                generation: 0,
            }),
            file: Some(file),
            directory: Some(directory),
            count: count as u32,
        });
        // 作成の受付を直列化し、上限を超える子プロセスが一瞬でも同時に動かないようにする。
        let mut sessions = self.sessions.lock().await;
        while sessions.len() >= self.options.max_concurrent_transcodes {
            if let Some(oldest) = sessions.pop_front() {
                oldest.close().await;
            }
        }
        {
            let mut process = session.process.lock().await;
            self.restart(&session, &mut process, 0).instrument(perf_span!("video_transcode_start")).await?;
        }
        sessions.push_back(session);
        Ok(info)
    }

    pub async fn playlist(&self, library: &str, id: &str) -> Result<String> {
        let session = self.session(library, id).await?;
        session.touch();
        let mut playlist = format!(
            "#EXTM3U\n#EXT-X-VERSION:3\n#EXT-X-TARGETDURATION:{}\n#EXT-X-MEDIA-SEQUENCE:0\n#EXT-X-PLAYLIST-TYPE:VOD\n#EXT-X-INDEPENDENT-SEGMENTS\n",
            self.options.segment_secs.saturating_add(1)
        );
        for index in 0..session.count {
            let duration = (session.info.duration_secs - f64::from(index) * f64::from(self.options.segment_secs)).min(f64::from(self.options.segment_secs));
            playlist.push_str(&format!("#EXTINF:{duration:.6},\nsegments/{index}.ts\n"));
        }
        playlist.push_str("#EXT-X-ENDLIST\n");
        Ok(playlist)
    }

    pub async fn segment(&self, library: &str, id: &str, index: u32) -> Result<SegmentReader> {
        let span = perf_span!("video_segment_wait", library_id = library, path = Empty, index, restarted = false, outcome = Empty);
        let result = self.segment_traced(library, id, index, &span).instrument(span.clone()).await;
        span.record(
            "outcome",
            match &result {
                Ok(_) => "ready",
                Err(error) => match error.kind() {
                    ErrorKind::Timeout => "timeout",
                    ErrorKind::AlreadyExists => "superseded",
                    ErrorKind::NotFound => "ended",
                    _ => "failed",
                },
            },
        );
        result
    }

    async fn segment_traced(&self, library: &str, id: &str, index: u32, span: &tracing::Span) -> Result<SegmentReader> {
        let session = self.session(library, id).await?;
        span.record("path", session.info.path.as_str());
        if index >= session.count {
            return Err(Error::new(ErrorKind::NotFound).with_message("segment not found"));
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
        let mut generation = None;
        // 再生が止まっている間だけ、サムネイルの生成と背景処理を保留させる。
        let mut interactive_wait = None;
        loop {
            if session.cancel.is_cancelled() {
                return Err(Error::new(ErrorKind::NotFound).with_message("video session ended"));
            }
            session.touch();
            match tokio::fs::File::open(session.segment(index)).await {
                Ok(file) => {
                    let len = file.metadata().await?.len();
                    return Ok(SegmentReader { file: Some(file), session, len });
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            interactive_wait.get_or_insert_with(|| self.videos.occupy(WorkClass::Interactive));
            {
                let mut process = session.process.lock().await;
                if session.cancel.is_cancelled() {
                    return Err(Error::new(ErrorKind::NotFound));
                }
                if generation.is_some_and(|generation| generation != process.generation) {
                    return Err(Error::new(ErrorKind::AlreadyExists).with_message("segment request was superseded by a seek"));
                }
                while process.frontier < session.count && tokio::fs::metadata(session.segment(process.frontier)).await.is_ok() {
                    process.frontier += 1;
                }
                let far = index < process.start || index > process.frontier.saturating_add(self.options.restart_threshold_segments);
                if far {
                    span.record("restarted", true);
                    self.restart(&session, &mut process, index).await?;
                }
                generation = Some(process.generation);
                if let Some(child) = process.child.as_mut()
                    && let Some(status) = child.try_wait()?
                    && (!status.success() || tokio::fs::metadata(session.segment(index)).await.is_err())
                {
                    return Err(Error::new(ErrorKind::UnsupportedType).with_message("video conversion did not produce the requested segment"));
                }
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(Error::new(ErrorKind::Timeout).with_message("video segment generation timed out"));
            }
            tokio::select! { _ = tokio::time::sleep(Duration::from_millis(50)) => {}, _ = session.cancel.cancelled() => return Err(Error::new(ErrorKind::NotFound)) }
        }
    }

    pub async fn close(&self, library: &str, id: &str) -> Result<()> {
        let mut sessions = self.sessions.lock().await;
        let index = sessions
            .iter()
            .position(|session| session.info.id == id && session.info.library_id == library)
            .ok_or_else(|| Error::new(ErrorKind::NotFound).with_message("video session not found"))?;
        let session = sessions.remove(index).ok_or_else(|| Error::new(ErrorKind::NotFound))?;
        session.close().await;
        Ok(())
    }

    async fn session(&self, library: &str, id: &str) -> Result<Arc<VideoSession>> {
        self.sessions
            .lock()
            .await
            .iter()
            .find(|session| session.info.id == id && session.info.library_id == library)
            .cloned()
            .ok_or_else(|| Error::new(ErrorKind::NotFound).with_message("video session not found"))
    }

    async fn restart(&self, session: &VideoSession, process: &mut Transcode, index: u32) -> Result<()> {
        if let Some(mut child) = process.child.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        let start_secs = f64::from(index) * f64::from(self.options.segment_secs);
        let mut command = Command::new(&self.ffmpeg);
        command
            .args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y", "-ss"])
            .arg(format!("{start_secs:.6}"))
            .arg("-i")
            .arg(session.file())
            .args(["-map", "0:v:0", "-map", "0:a:0?", "-sn", "-dn", "-c:v"])
            .arg(&self.options.encoder)
            .args(&self.options.encoder_args)
            .args(["-pix_fmt", "yuv420p", "-c:a", "aac", "-flags", "+cgop", "-sc_threshold", "0", "-vf"])
            .arg(format!("scale=-2:trunc(min(ih\\,{})/2)*2", self.options.max_height))
            .arg("-force_key_frames")
            .arg(format!("expr:gte(t,n_forced*{})", self.options.segment_secs))
            .arg("-output_ts_offset")
            .arg(format!("{start_secs:.6}"))
            .args(["-f", "hls", "-hls_list_size", "0", "-hls_flags", "temp_file+independent_segments", "-hls_time"])
            .arg(self.options.segment_secs.to_string())
            .arg("-start_number")
            .arg(index.to_string())
            .arg("-hls_segment_filename")
            .arg(session.directory().join("%d.ts"))
            .arg(session.directory().join("generated.m3u8"))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        process.child = Some(
            command
                .spawn()
                .map_err(|error| Error::from_error(error, ErrorKind::UnexpectedError).with_message("cannot start video conversion"))?,
        );
        process.start = index;
        process.frontier = index;
        process.generation += 1;
        Ok(())
    }

    async fn reap(&self) {
        let mut sessions = self.sessions.lock().await;
        let timeout = Duration::from_secs(self.options.session_idle_timeout_secs);
        let mut index = 0;
        while index < sessions.len() {
            if sessions[index].touched.lock().elapsed() >= timeout {
                if let Some(session) = sessions.remove(index) {
                    session.close().await;
                }
            } else {
                index += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testresult::TestResult;
    use tokio::io::AsyncReadExt as _;

    fn available() -> bool {
        let available = ["ffmpeg", "ffprobe"]
            .iter()
            .all(|program| std::process::Command::new(program).arg("-version").output().is_ok_and(|output| output.status.success()));
        if !available {
            assert!(std::env::var_os("PXNA_REQUIRE_FFMPEG").is_none(), "ffmpeg is required in CI");
        }
        available
    }

    async fn fixture() -> TestResult<(tempfile::TempDir, crate::config::DaemonConfig)> {
        let (dir, state) = crate::service::mutation::tests::state().await?;
        let mut conf = state.conf.clone();
        drop(state);
        conf.video.segment_secs = 2;
        conf.video.restart_threshold_segments = 1;
        conf.video.session_idle_timeout_secs = 1;
        let status = std::process::Command::new("ffmpeg")
            .args([
                "-v",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc=duration=100:size=160x90:rate=10",
                "-c:v",
                "mpeg4",
                "-pix_fmt",
                "yuv420p",
            ])
            .arg(conf.libraries[0].path.join("clip.mp4"))
            .status()?;
        assert!(status.success());
        Ok((dir, conf))
    }

    async fn wait_for_cleanup(path: &Path) -> TestResult {
        let result = tokio::time::timeout(Duration::from_secs(5), async {
            while path.exists() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await;
        assert!(result.is_ok(), "output directory was not reclaimed: {}", path.display());
        Ok(())
    }

    #[test]
    fn pending_segment_read_keeps_directory_until_handle_closes_test() -> TestResult {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().max_blocking_threads(1).build()?;
        runtime.block_on(async {
            let root = tempfile::tempdir()?;
            let directory = tempfile::tempdir_in(root.path())?;
            let path = directory.path().to_path_buf();
            let segment = path.join("0.ts");
            std::fs::write(&segment, b"segment")?;
            let session = Arc::new(VideoSession {
                info: VideoSessionInfo {
                    id: "test".into(),
                    library_id: "test".into(),
                    path: "/clip.mp4".into(),
                    playlist_url: "playlist.m3u8".into(),
                    duration_secs: 1.0,
                },
                touched: Mutex::new(Instant::now()),
                cancel: CancellationToken::new(),
                process: TokioMutex::new(Transcode {
                    child: None,
                    start: 0,
                    frontier: 0,
                    generation: 0,
                }),
                file: None,
                directory: Some(directory),
                count: 1,
            });
            let mut reader = SegmentReader {
                file: Some(tokio::fs::File::from_std(std::fs::File::open(segment)?)),
                session: session.clone(),
                len: 7,
            };
            let (started_tx, started_rx) = tokio::sync::oneshot::channel();
            let (release_tx, release_rx) = std::sync::mpsc::channel();
            let worker = tokio::task::spawn_blocking(move || {
                let _ = started_tx.send(());
                let _ = release_rx.recv();
            });
            started_rx.await?;
            let mut bytes = [0; 1];
            let mut buf = ReadBuf::new(&mut bytes);
            let mut cx = Context::from_waker(std::task::Waker::noop());
            assert!(Pin::new(&mut reader).poll_read(&mut cx, &mut buf).is_pending());
            session.close().await;
            drop(session);
            drop(reader);
            assert!(path.exists(), "pending read retains the directory lease");
            release_tx.send(())?;
            worker.await?;
            wait_for_cleanup(&path).await?;
            Ok(())
        })
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn locked_segment_directory_is_reclaimed_after_handle_closes_test() -> TestResult {
        use std::os::windows::fs::OpenOptionsExt as _;

        let root = tempfile::tempdir()?;
        let directory = tempfile::tempdir_in(root.path())?;
        let path = directory.path().to_path_buf();
        let segment = path.join("0.ts");
        std::fs::write(&segment, b"segment")?;
        let file = std::fs::OpenOptions::new().read(true).share_mode(0).open(segment)?;
        VideoSession::reclaim_directory(directory);
        assert!(path.exists(), "exclusive handle prevents the first cleanup attempt");
        drop(file);
        wait_for_cleanup(&path).await?;
        Ok(())
    }

    #[tokio::test]
    async fn video_session_seek_scope_limit_and_cleanup_test() -> TestResult {
        if !available() {
            return Ok(());
        }
        let (dir, conf) = fixture().await?;
        let leftover = conf.core.state_dir.join("video/leftover");
        std::fs::create_dir_all(&leftover)?;
        std::fs::write(leftover.join("partial"), b"old")?;
        let state = crate::state::DaemonState::new(conf).await?;
        assert!(!leftover.exists());
        let info = state.videos.create("test", "/clip.mp4").await?;
        let playlist = state.videos.playlist("test", &info.id).await?;
        assert!(playlist.contains("segments/49.ts"));
        assert!(playlist.ends_with("#EXT-X-ENDLIST\n"));
        assert_eq!(*state.videos.playlist("other", &info.id).await.unwrap_err().kind(), ErrorKind::NotFound);
        let mut reader = state.videos.segment("test", &info.id, 40).await?;
        let session = state.videos.session("test", &info.id).await?;
        let output_dir = session.directory().to_path_buf();
        assert_eq!(session.process.lock().await.start, 40, "far seek starts at the requested segment");
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        assert!(!bytes.is_empty());
        let probe = std::process::Command::new("ffprobe")
            .args(["-v", "error", "-show_entries", "format=start_time,duration", "-of", "json"])
            .arg(session.segment(40))
            .output()?;
        assert!(probe.status.success());
        let probe: serde_json::Value = serde_json::from_slice(&probe.stdout)?;
        let pts: f64 = probe["format"]["start_time"].as_str().ok_or("start time missing")?.parse()?;
        assert!((pts - 81.4).abs() < 0.3, "segment timestamp must retain the seek position: {pts}");
        if session.segment(0).exists() {
            tokio::fs::remove_file(session.segment(0)).await?;
        }
        let back = state.videos.segment("test", &info.id, 0).await?;
        assert_eq!(session.process.lock().await.start, 0, "uncached backward seek restarts conversion");
        drop(back);
        drop(reader);
        drop(session);
        let next = state.videos.create("test", "/clip.mp4").await?;
        assert_eq!(*state.videos.playlist("test", &info.id).await.unwrap_err().kind(), ErrorKind::NotFound);
        wait_for_cleanup(&output_dir).await?;
        let reader = state.videos.segment("test", &next.id, 0).await?;
        let path = state.videos.session("test", &next.id).await?.directory().to_path_buf();
        state.videos.close("test", &next.id).await?;
        assert!(path.exists(), "active body retains directory lease after close");
        drop(reader);
        wait_for_cleanup(&path).await?;
        assert_eq!(std::fs::read_dir(dir.path().join("state/video"))?.count(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn video_session_idle_expiry_and_failed_process_test() -> TestResult {
        if !available() {
            return Ok(());
        }
        let (_dir, mut conf) = fixture().await?;
        conf.video.encoder = "no_such_encoder".to_string();
        let state = crate::state::DaemonState::new(conf).await?;
        let info = state.videos.create("test", "/clip.mp4").await?;
        assert_eq!(
            *state.videos.segment("test", &info.id, 0).await.err().ok_or("expected failure")?.kind(),
            ErrorKind::UnsupportedType
        );
        tokio::time::sleep(Duration::from_millis(1100)).await;
        state.videos.reap().await;
        assert_eq!(*state.videos.playlist("test", &info.id).await.unwrap_err().kind(), ErrorKind::NotFound);
        assert_eq!(std::fs::read_dir(state.conf.core.state_dir.join("video"))?.count(), 0);
        Ok(())
    }
}
