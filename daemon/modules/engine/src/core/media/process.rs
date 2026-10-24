use std::{path::Path, process::Stdio, time::Duration};

use tokio::process::Command;

use crate::prelude::*;

/// 期限付きで実行し、標準出力を返す。失敗したときは、外部プロセスの標準エラーを message に残す。
// omnius-lint:allow(free-fn) Wraps the process boundary shared by the ffmpeg and heif-dec tools
pub(super) async fn run(mut command: Command, timeout: Duration) -> Result<Vec<u8>> {
    command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
    let program = command.as_std().get_program().to_string_lossy().into_owned();
    let child = command.spawn().map_err(|error| {
        // 実行ファイルがないのは、利用者のファイルの問題ではなく、daemon の環境の問題である。
        Error::from_error(error, ErrorKind::UnexpectedError).with_message(format!("failed to start {program}"))
    })?;

    let output = match tokio::time::timeout(timeout, child.wait_with_output()).await {
        Ok(output) => output?,
        Err(_) => return Err(Error::new(ErrorKind::Timeout).with_message(format!("{program} did not finish in {timeout:?}"))),
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = stderr.lines().next().unwrap_or_default();
        return Err(Error::new(ErrorKind::UnsupportedType).with_message(format!("{program} failed: {detail}")));
    }
    Ok(output.stdout)
}

/// 外部プロセスが書いた画像を読む。書かれていないか空のときは、読めない file として `UnsupportedType` にする。
// omnius-lint:allow(free-fn) Reads the output file at the process boundary shared by the ffmpeg and heif-dec tools
pub(super) async fn read_output(output_path: &Path, file: &Path) -> Result<Vec<u8>> {
    match tokio::fs::read(output_path).await {
        Ok(bytes) if !bytes.is_empty() => Ok(bytes),
        Ok(_) => Err(Error::new(ErrorKind::UnsupportedType).with_message(format!("no image can be read: {}", file.display()))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Err(Error::new(ErrorKind::UnsupportedType).with_message(format!("no image can be read: {}", file.display()))),
        Err(error) => Err(error.into()),
    }
}
