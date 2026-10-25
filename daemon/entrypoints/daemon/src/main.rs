use std::{process::ExitCode, time::Duration};

use crate::{executor::Executor, prelude::*};

mod config;
mod db;
mod error;
mod executor;
mod interface;
mod media_check;
mod prelude;
mod result;
mod service;
mod state;

pub use error::*;
pub use result::*;

// RUST_LOG=trace RUST_BACKTRACE=full cargo run

fn main() -> ExitCode {
    let runtime = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("failed to create daemon runtime: {error}");
            return ExitCode::FAILURE;
        }
    };
    let result = runtime.block_on(Executor::run());
    // spawn_blocking は中断できないため、runtime の Drop による無期限の終了待ちを避ける。
    runtime.shutdown_timeout(Duration::from_secs(1));

    let code = match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            error!(error = ?e, "unexpected error");
            ExitCode::FAILURE
        }
    };
    Executor::shutdown_logging();
    code
}
