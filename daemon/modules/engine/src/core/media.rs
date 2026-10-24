mod ffmpeg;
mod heif;
mod image;
mod info;
mod kind;
mod process;

pub use ffmpeg::*;
pub use image::*;
pub use info::*;
pub use kind::*;

use std::{path::PathBuf, time::Duration};

use crate::prelude::*;

/// ffmpeg、ffprobe、libheif の heif-dec の実行ファイルの場所。`PATH` から探す名前か、絶対 path を指定する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaOptions {
    pub ffmpeg_path: PathBuf,
    pub ffprobe_path: PathBuf,
    pub heif_dec_path: PathBuf,
}

impl Default for MediaOptions {
    fn default() -> Self {
        Self {
            ffmpeg_path: PathBuf::from("ffmpeg"),
            ffprobe_path: PathBuf::from("ffprobe"),
            heif_dec_path: PathBuf::from("heif-dec"),
        }
    }
}

/// ffmpeg、ffprobe、heif-dec を外部プロセスとして呼ぶ。
///
/// daemon にリンクしないので、壊れた動画で ffmpeg が異常終了しても daemon は巻き込まれず、期限を過ぎたプロセスは kill すれば止まる。
/// 呼び出しが drop されたときも、子プロセスは kill される。
#[derive(Debug, Clone)]
pub struct MediaTools {
    options: MediaOptions,
    timeout: Duration,
}

impl MediaTools {
    pub fn new(options: MediaOptions, timeout: Duration) -> Self {
        Self { options, timeout }
    }

    /// 期限付きで実行し、標準出力を返す。失敗したときは、外部プロセスの標準エラーを message に残す。
    async fn run(&self, command: tokio::process::Command) -> Result<Vec<u8>> {
        process::run(command, self.timeout).await
    }
}
