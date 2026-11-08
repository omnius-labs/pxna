use std::{
    future::Future,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use clap::Parser as _;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use omnius_core_base::file_lock::FileLock;

use crate::{config::DaemonConfig, interface::ApiServer, logging::Logging, prelude::*, state::DaemonState};

const LOCK_FILE_NAME: &str = "pxna.lock";
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(clap::Parser)]
#[command(name = "pxna-daemon", about = "xxx", version = env!("GIT_TAG"))]
struct Args {
    #[command(subcommand)]
    command: SubCommand,
}

#[derive(Debug, clap::Subcommand)]
enum SubCommand {
    Start {
        #[arg(long, default_value_os_t = Executor::default_config_dir())]
        config_dir: PathBuf,
    },
}

pub struct Executor;

impl Executor {
    fn default_config_dir() -> PathBuf {
        std::env::var_os("PXNA_DAEMON_CONFIG_DIR")
            .map(PathBuf::from)
            .or_else(|| Self::home_dir().map(|path| path.join(".config/pxna")))
            .unwrap_or_else(|| PathBuf::from(".config/pxna"))
    }

    fn home_dir() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        return std::env::var_os("USERPROFILE").map(PathBuf::from);

        #[cfg(any(target_os = "linux", target_os = "macos"))]
        return std::env::var_os("HOME").map(PathBuf::from);
    }

    pub async fn run() -> Result<()> {
        let args = Args::parse();
        Self::execute(args).await
    }

    async fn execute(args: Args) -> Result<()> {
        match args.command {
            SubCommand::Start { config_dir } => Self::handle_start(config_dir).await?,
        }

        Ok(())
    }

    // Reject a second daemon that points at the same configuration directory.
    // `FileLock` holds an OS advisory lock. The OS releases it however the process exits,
    // so a file left behind can be acquired again.
    async fn acquire_process_lock(config_dir: &Path) -> tokio::io::Result<FileLock> {
        FileLock::acquire(config_dir.join(LOCK_FILE_NAME)).await
    }

    async fn handle_start(config_dir: impl AsRef<Path>) -> Result<()> {
        let config_dir = config_dir.as_ref();

        // tracing はまだ初期化していないため、多重起動の拒否は stderr へ直接出す。
        let _lock = Self::acquire_process_lock(config_dir).await.inspect_err(|error| {
            eprintln!("failed to acquire the process lock in {}: {error}", config_dir.display());
        })?;

        let token = CancellationToken::new();
        _ = Self::spawn_signal_handler(&token)?;

        let operation = async {
            let conf = DaemonConfig::load(&config_dir).await.inspect_err(|error| {
                eprintln!("failed to load daemon config from {}: {error:?}", config_dir.display());
            })?;

            Logging::init(&conf.logging)?;

            crate::media_check::MediaEnvironment::verify(&conf.media).await?;

            let state = Arc::new(DaemonState::new(conf).await?);
            state.timeline.start();
            ApiServer::serve(state, token.clone()).await
        };

        Self::run_until_shutdown(operation, &token, SHUTDOWN_TIMEOUT).await
    }

    // 稼働時間は制限せず、終了要求を受けた時点から起動処理と HTTP の終了待ちに期限を設ける。
    async fn run_until_shutdown(operation: impl Future<Output = Result<()>>, token: &CancellationToken, timeout: Duration) -> Result<()> {
        tokio::pin!(operation);
        tokio::select! {
            result = &mut operation => return result,
            _ = token.cancelled() => {}
        }

        info!(?timeout, "shutting down daemon");
        tokio::time::timeout(timeout, operation)
            .await
            .map_err(|error| Error::from_error(error, ErrorKind::Timeout).with_message("daemon shutdown timed out"))?
    }

    fn spawn_signal_handler(token: &CancellationToken) -> Result<JoinHandle<()>> {
        let token = token.clone();

        #[cfg(unix)]
        {
            use tokio::signal::unix::{SignalKind, signal};

            let mut terminate = signal(SignalKind::terminate())?;
            let mut interrupt = signal(SignalKind::interrupt())?;

            Ok(tokio::spawn(async move {
                tokio::select! {
                    result = tokio::signal::ctrl_c() => {
                        if result.is_ok() {
                            token.cancel();
                        }
                    }
                    _ = terminate.recv() => {
                        token.cancel();
                    }
                    _ = interrupt.recv() => {
                        token.cancel();
                    }
                    _ = token.cancelled() => {}
                }
            }))
        }

        #[cfg(not(unix))]
        {
            Ok(tokio::spawn(async move {
                tokio::select! {
                    result = tokio::signal::ctrl_c() => {
                        if result.is_ok() {
                            token.cancel();
                        }
                    }
                    _ = token.cancelled() => {}
                }
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::Notify;

    use testresult::TestResult;

    use super::*;

    #[tokio::test]
    async fn shutdown_deadline_starts_only_after_cancellation() -> TestResult {
        let token = CancellationToken::new();
        let operation = async {
            tokio::time::sleep(Duration::from_millis(50)).await;
            Ok(())
        };

        Executor::run_until_shutdown(operation, &token, Duration::from_millis(10)).await?;
        Ok(())
    }

    #[tokio::test]
    async fn shutdown_waits_for_in_flight_http_request_with_deadline() -> TestResult {
        for complete_request in [true, false] {
            let token = CancellationToken::new();
            let started = Arc::new(Notify::new());
            let release = Arc::new(Notify::new());
            let app = axum::Router::new().route(
                "/",
                axum::routing::get({
                    let started = started.clone();
                    let release = release.clone();
                    move || {
                        let started = started.clone();
                        let release = release.clone();
                        async move {
                            started.notify_one();
                            release.notified().await;
                            "finished"
                        }
                    }
                }),
            );
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
            let addr = listener.local_addr()?;
            let shutdown = token.clone();
            let server = tokio::spawn(async move {
                let operation = async {
                    axum::serve(listener, app).with_graceful_shutdown(shutdown.clone().cancelled_owned()).await?;
                    Ok(())
                };
                Executor::run_until_shutdown(operation, &shutdown, Duration::from_millis(100)).await
            });
            let client = tokio::spawn(async move { reqwest::get(format!("http://{addr}/")).await });

            tokio::time::timeout(Duration::from_secs(5), started.notified()).await?;
            token.cancel();
            if complete_request {
                release.notify_one();
                assert_eq!(client.await??.text().await?, "finished");
                tokio::time::timeout(Duration::from_secs(5), server).await???;
            } else {
                let error = tokio::time::timeout(Duration::from_secs(5), server)
                    .await??
                    .expect_err("a stalled request must not keep shutdown waiting");
                assert_eq!(error.kind(), &ErrorKind::Timeout);
                // Axum の接続 task は main の runtime shutdown で中断される。テストでは明示的に完了させる。
                release.notify_one();
                client.await??;
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn acquire_process_lock_when_same_config_dir() -> TestResult {
        let dir = tempfile::tempdir()?;

        let _lock = Executor::acquire_process_lock(dir.path()).await?;
        assert!(dir.path().join(LOCK_FILE_NAME).exists());

        let error = Executor::acquire_process_lock(dir.path()).await.expect_err("second daemon should be rejected");
        assert_eq!(error.kind(), tokio::io::ErrorKind::WouldBlock);

        Ok(())
    }

    #[tokio::test]
    async fn acquire_process_lock_when_other_config_dir() -> TestResult {
        let dir1 = tempfile::tempdir()?;
        let dir2 = tempfile::tempdir()?;

        let _lock1 = Executor::acquire_process_lock(dir1.path()).await?;
        let _lock2 = Executor::acquire_process_lock(dir2.path()).await?;

        Ok(())
    }

    #[tokio::test]
    async fn acquire_process_lock_over_a_leftover_file() -> TestResult {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join(LOCK_FILE_NAME), "999999")?;

        // Windows does not let another handle read a file while it is locked.
        {
            let _lock = Executor::acquire_process_lock(dir.path()).await?;
        }
        assert_eq!(std::fs::read_to_string(dir.path().join(LOCK_FILE_NAME))?, std::process::id().to_string());

        Ok(())
    }
}
