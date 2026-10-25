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

    // 同じ設定ディレクトリを指す 2 つ目の daemon を拒否する。
    async fn acquire_process_lock(config_dir: &Path) -> tokio::io::Result<FileLock> {
        let path = config_dir.join(LOCK_FILE_NAME);

        match FileLock::acquire(&path).await {
            Err(e) if e.kind() == tokio::io::ErrorKind::AlreadyExists => {
                // FileLock がロックを消すのは Drop だけである。SIGKILL や電源断のあとは
                // 残骸が居座り、以後の起動を恒久的に塞ぐ。ロックに書かれた PID の生存を見て、
                // 持ち主がいないものだけ取り除き、1 度だけ取り直す。
                if Self::lock_owner_is_alive(&path).await {
                    return Err(e);
                }
                tokio::fs::remove_file(&path).await?;
                let lock = FileLock::acquire(&path).await?;

                // 同じ残骸を見た 2 つ目の daemon が、こちらの取得直後に remove_file して
                // own のロックを作ると、双方が起動してしまう。取り直した中身が自分の PID か
                // 読み返し、奪われていれば起動しない。窓は残るが、無条件に信じるよりは狭い。
                // 根本的には core-rs の FileLock を advisory lock にする必要がある。
                Self::verify_lock_owner(&path).await?;

                Ok(lock)
            }
            result => result,
        }
    }

    // 読めない、PID として解釈できない、PID が生きている、のいずれも「持ち主がいる」とする。
    // 判定を誤るなら、他人のロックを奪うより起動を拒否する側へ倒す。
    async fn lock_owner_is_alive(path: &Path) -> bool {
        let Ok(text) = tokio::fs::read_to_string(path).await else {
            return true;
        };
        let Ok(pid) = text.trim().parse::<u32>() else {
            return true;
        };

        sys::process_exists(pid)
    }

    async fn verify_lock_owner(path: &Path) -> tokio::io::Result<()> {
        let text = tokio::fs::read_to_string(path).await?;
        if text.trim() != std::process::id().to_string() {
            return Err(tokio::io::Error::new(tokio::io::ErrorKind::AlreadyExists, "the process lock was taken by another daemon"));
        }

        Ok(())
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

#[cfg(unix)]
mod sys {
    use rustix::{
        io::Errno,
        process::{Pid, test_kill_process},
    };

    /// `kill(pid, 0)` は signal を送らず、送れるかどうかだけを見る。
    /// `EPERM` は別ユーザーのプロセスが存在することを意味するため、生存として扱う。
    // omnius-lint:allow(free-fn) Unixのprocess生存確認をcfg moduleのOS境界として公開する
    pub fn process_exists(pid: u32) -> bool {
        let Ok(raw) = i32::try_from(pid) else {
            return true;
        };
        let Some(pid) = Pid::from_raw(raw) else {
            return true;
        };

        !matches!(test_kill_process(pid), Err(Errno::SRCH))
    }
}

#[cfg(windows)]
mod sys {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, FALSE, STILL_ACTIVE},
        System::Threading::{GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
    };

    /// 終了済みでも handle を開ける場合があるため、`OpenProcess` の成否だけでは足りず
    /// 終了コードまで見る。exit code が偶然 `STILL_ACTIVE` と同じ場合は生存と誤るが、
    /// 起動を拒否する側へ倒れるため許容する。
    // omnius-lint:allow(free-fn) Windowsのprocess生存確認をcfg moduleのOS境界として公開する
    pub fn process_exists(pid: u32) -> bool {
        // SAFETY: PID を渡して handle を得るだけで、無効な PID には null が返る。
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid) };
        if handle.is_null() {
            // 存在しない PID は ERROR_INVALID_PARAMETER になる。権限不足なら存在している。
            return std::io::Error::last_os_error().raw_os_error() != Some(87);
        }

        let mut code = 0u32;
        // SAFETY: handle は直前の OpenProcess が返した有効なものである。
        let ok = unsafe { GetExitCodeProcess(handle, &mut code) };
        // SAFETY: 同上。ここで所有権を手放す。
        unsafe { CloseHandle(handle) };

        ok == 0 || code == STILL_ACTIVE as u32
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
        assert_eq!(error.kind(), tokio::io::ErrorKind::AlreadyExists);

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
    async fn acquire_process_lock_when_owner_is_alive() -> TestResult {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join(LOCK_FILE_NAME), std::process::id().to_string())?;

        let error = Executor::acquire_process_lock(dir.path()).await.expect_err("a live owner should keep the lock");
        assert_eq!(error.kind(), tokio::io::ErrorKind::AlreadyExists);

        Ok(())
    }

    // 終了済みプロセスの PID を仕込むため、reap まで面倒を見られる unix に限る。
    #[cfg(unix)]
    #[tokio::test]
    async fn acquire_process_lock_when_owner_is_gone() -> TestResult {
        let dir = tempfile::tempdir()?;

        let child = std::process::Command::new("/usr/bin/true").spawn()?;
        let dead_pid = child.id();
        child.wait_with_output()?;

        std::fs::write(dir.path().join(LOCK_FILE_NAME), dead_pid.to_string())?;

        let _lock = Executor::acquire_process_lock(dir.path()).await?;
        assert_eq!(std::fs::read_to_string(dir.path().join(LOCK_FILE_NAME))?, std::process::id().to_string());

        Ok(())
    }

    #[tokio::test]
    async fn acquire_process_lock_when_lock_is_unreadable() -> TestResult {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join(LOCK_FILE_NAME), "not a pid")?;

        let error = Executor::acquire_process_lock(dir.path()).await.expect_err("an unparsable lock should be kept");
        assert_eq!(error.kind(), tokio::io::ErrorKind::AlreadyExists);

        Ok(())
    }
}
