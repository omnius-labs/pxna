mod performance;

use std::{path::Path, sync::Mutex};

use tracing_appender::{non_blocking::WorkerGuard, rolling};
use tracing_subscriber::{
    EnvFilter, Layer as _, Registry,
    fmt::{
        self,
        writer::{BoxMakeWriter, MakeWriterExt as _},
    },
    layer::SubscriberExt as _,
    util::SubscriberInitExt as _,
};

use omnius_pxna_engine::telemetry::PERFORMANCE_TARGET;

use crate::{config::LoggingConfig, prelude::*};

pub use performance::PerformanceLayer;

/// file のログを書く thread の guard。落とすと書き切って thread が止まる。
static FILE_GUARD: Mutex<Option<WorkerGuard>> = Mutex::new(None);

/// daemon のログの出力先と絞り込み。
pub struct Logging;

impl Logging {
    /// 標準出力と、設定されていれば file へ出す。計測を有効にしたときは、計測の span の経過時間も同じ出力先へ出す。
    pub fn init(conf: &LoggingConfig) -> Result<()> {
        let directives = if conf.performance {
            format!("{},{PERFORMANCE_TARGET}=debug,sqlx=off", conf.level)
        } else {
            format!("{},sqlx=off", conf.level)
        };
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(directives));

        let mut layers: Vec<Box<dyn tracing_subscriber::Layer<Registry> + Send + Sync>> = Vec::new();
        layers.push(if conf.json { fmt::layer().json().boxed() } else { fmt::layer().boxed() });

        let mut guard = None;
        let spans = match &conf.file {
            Some(file) => {
                let (writer, worker) = tracing_appender::non_blocking(Self::file_appender(file)?);
                guard = Some(worker);
                layers.push(if conf.json {
                    fmt::layer().json().with_ansi(false).with_writer(writer.clone()).boxed()
                } else {
                    fmt::layer().with_ansi(false).with_writer(writer.clone()).boxed()
                });
                BoxMakeWriter::new(std::io::stdout.and(writer))
            }
            None => BoxMakeWriter::new(std::io::stdout),
        };
        layers.push(PerformanceLayer::new(spans, conf.json).boxed());

        tracing_subscriber::registry().with(layers).with(filter).init();
        *FILE_GUARD.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = guard;
        Ok(())
    }

    /// file への未書き込みのログを書き切る。終了前の最後のログ（`main` が出す失敗）まで残すため、`main` の最後に呼ぶ。
    pub fn shutdown() {
        drop(FILE_GUARD.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).take());
    }

    fn file_appender(file: &Path) -> Result<rolling::RollingFileAppender> {
        let name = file
            .file_name()
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput).with_message(format!("logging.file has no file name: {}", file.display())))?;
        let dir = file.parent().filter(|dir| !dir.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(dir)?;
        Ok(rolling::never(dir, name))
    }
}
