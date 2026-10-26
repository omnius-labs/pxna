use std::{path::Path, time::Duration};

use omnius_pxna_engine::service::MediaOptions;
use tokio::process::Command;

use crate::prelude::*;

/// HEIC/AVIF のタイル組み立てと回転を扱える ffmpeg の最低のメジャー版。
const MINIMUM_FFMPEG_MAJOR: u32 = 9;

/// 画像のデコードに使う外部ツールが、この daemon の前提を満たすかを起動時に確かめる。
pub struct MediaEnvironment;

impl MediaEnvironment {
    /// ffmpeg と ffprobe は 9 以上を必須にし、heif-dec は Linux と macOS で必須にする。
    /// Windows の heif-dec は、アルファ付きの画像を要求したときにだけ必要になる。
    pub async fn verify(media: &MediaOptions) -> Result<()> {
        Self::require_ffmpeg("ffmpeg", &media.ffmpeg_path).await?;
        Self::require_ffmpeg("ffprobe", &media.ffprobe_path).await?;
        #[cfg(not(target_os = "windows"))]
        Self::require_heif_dec(&media.heif_dec_path).await?;
        Ok(())
    }

    async fn require_ffmpeg(name: &str, path: &Path) -> Result<()> {
        let output = Self::run(name, path, &["-version"]).await?;
        let text = String::from_utf8_lossy(&output);
        let major = Self::major_version(name, &text);
        match major {
            Some(major) if major >= MINIMUM_FFMPEG_MAJOR => Ok(()),
            Some(major) => {
                Err(Error::new(ErrorKind::UnexpectedError).with_message(format!("{name} {MINIMUM_FFMPEG_MAJOR} or newer is required, but {} is version {major}", path.display())))
            }
            None => Err(Error::new(ErrorKind::UnexpectedError).with_message(format!("cannot read the version of {name} at {}", path.display()))),
        }
    }

    #[cfg(not(target_os = "windows"))]
    async fn require_heif_dec(path: &Path) -> Result<()> {
        // 古い libheif の heif-convert は --version を受け付けないことがあるため、起動できることだけを確かめる。
        let mut command = Command::new(path);
        command.arg("--version").kill_on_drop(true);
        tokio::time::timeout(Duration::from_secs(5), command.output())
            .await
            .map_err(|_| Error::new(ErrorKind::Timeout).with_message(format!("heif-dec did not answer within 5 seconds: {}", path.display())))?
            .map_err(|error| Error::from_error(error, ErrorKind::UnexpectedError).with_message(format!("failed to start heif-dec at {}", path.display())))?;
        Ok(())
    }

    async fn run(name: &str, path: &Path, args: &[&str]) -> Result<Vec<u8>> {
        let mut command = Command::new(path);
        command.args(args).kill_on_drop(true);
        let output = tokio::time::timeout(Duration::from_secs(5), command.output())
            .await
            .map_err(|_| Error::new(ErrorKind::Timeout).with_message(format!("{name} did not answer within 5 seconds: {}", path.display())))?
            .map_err(|error| Error::from_error(error, ErrorKind::UnexpectedError).with_message(format!("failed to start {name} at {}", path.display())))?;
        if !output.status.success() {
            return Err(Error::new(ErrorKind::UnexpectedError).with_message(format!("{name} failed its version check: {}", path.display())));
        }
        Ok(output.stdout)
    }

    /// `ffmpeg version n9.0.2...` と `ffprobe version 9.0.2...` からメジャー版を読む。読めなければ `None`。
    fn major_version(name: &str, text: &str) -> Option<u32> {
        let prefix = format!("{name} version ");
        let rest = text.lines().next()?.strip_prefix(&prefix)?;
        let rest = rest.strip_prefix('n').unwrap_or(rest);
        rest.chars().take_while(char::is_ascii_digit).collect::<String>().parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_major_version_from_common_banners_test() {
        assert_eq!(MediaEnvironment::major_version("ffmpeg", "ffmpeg version n9.0.2-17-g2a571b6068 Copyright"), Some(9));
        assert_eq!(MediaEnvironment::major_version("ffprobe", "ffprobe version 9.0.2 Copyright"), Some(9));
        assert_eq!(MediaEnvironment::major_version("ffmpeg", "ffmpeg version 6.1.1-3ubuntu5"), Some(6));
        assert_eq!(MediaEnvironment::major_version("ffmpeg", "ffmpeg version N-117000-gabc"), None);
        assert_eq!(MediaEnvironment::major_version("ffmpeg", ""), None);
    }
}
