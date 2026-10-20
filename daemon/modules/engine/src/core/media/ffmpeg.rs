use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use tokio::process::Command;

use crate::prelude::*;

/// ffmpeg と ffprobe の実行ファイルの場所。`PATH` から探す名前か、絶対 path を指定する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaOptions {
    pub ffmpeg_path: PathBuf,
    pub ffprobe_path: PathBuf,
}

impl Default for MediaOptions {
    fn default() -> Self {
        Self {
            ffmpeg_path: PathBuf::from("ffmpeg"),
            ffprobe_path: PathBuf::from("ffprobe"),
        }
    }
}

/// ffmpeg と ffprobe を外部プロセスとして呼ぶ。
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

    /// ffprobe で動画の長さ（秒）を得る。長さを読めない file は `UnsupportedType` にする。
    #[cfg(test)]
    pub async fn probe_duration_secs(&self, file: &Path) -> Result<f64> {
        Ok(self.probe_video_info(file).await?.duration_secs)
    }

    /// 再生能力の判定とフレーム生成に必要な、動画の情報を一度に得る。
    pub async fn probe_video_info(&self, file: &Path) -> Result<super::VideoInfo> {
        let mut command = Command::new(&self.options.ffprobe_path);
        command.args(["-v", "error", "-show_format", "-show_streams", "-of", "json", "-i"]).arg(file);

        let output = self.run(command).await?;
        super::VideoInfo::parse(&output)
    }

    /// ffmpeg で読む画像（HEIC、AVIF など）の幅と高さと、回転（反時計回りの 0、90、180、270 度）を得る。幅と高さは符号化されたままで、回転を反映しない。
    pub async fn probe_image_size(&self, file: &Path) -> Result<(u32, u32, u16)> {
        let mut command = Command::new(&self.options.ffprobe_path);
        command.args(["-v", "error", "-show_streams", "-of", "json", "-i"]).arg(file);

        let output = self.run(command).await?;
        super::ImageInfo::parse_probe_size(&output)
    }

    /// ffprobe で、コンテナか stream の `creation_time` を得る。書かれていなければ `None` にする。
    pub async fn probe_creation_time(&self, file: &Path) -> Result<Option<chrono::DateTime<chrono::Utc>>> {
        let mut command = Command::new(&self.options.ffprobe_path);
        command
            .args(["-v", "error", "-show_entries", "format_tags=creation_time:stream_tags=creation_time", "-of", "json", "-i"])
            .arg(file);

        let output = self.run(command).await?;
        Ok(super::CreationTime::parse(&output))
    }

    /// 指定した位置（秒）のフレームを、1 回の ffmpeg の実行で PNG として取り出す。
    ///
    /// 位置ごとに `-ss` を付けた入力を並べ、キーフレーム単位で移る。一覧で眺めるためのフレームに正確な時刻は要らず、デコードする量が少ないほど速い。
    /// 動画の終わりを超えた位置など、取り出せなかったフレームは結果から除く。
    pub async fn extract_frames(&self, file: &Path, positions_secs: &[f64]) -> Result<Vec<Vec<u8>>> {
        let output_dir = tempfile::tempdir()?;
        let frame_path = |index: usize| output_dir.path().join(format!("{index}.png"));

        let mut command = Command::new(&self.options.ffmpeg_path);
        command.args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y"]);
        for position in positions_secs {
            command.arg("-ss").arg(format!("{position:.3}")).arg("-i").arg(file);
        }
        for index in 0..positions_secs.len() {
            command
                .arg("-map")
                .arg(format!("{index}:v:0"))
                .args(["-frames:v", "1", "-an", "-sn"])
                .arg(frame_path(index));
        }
        self.run(command).await?;

        let mut frames = Vec::new();
        for index in 0..positions_secs.len() {
            match tokio::fs::read(frame_path(index)).await {
                Ok(bytes) if !bytes.is_empty() => frames.push(bytes),
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        if frames.is_empty() {
            return Err(Error::new(ErrorKind::UnsupportedType).with_message(format!("no frame can be read: {}", file.display())));
        }
        Ok(frames)
    }

    /// 期限付きで実行し、標準出力を返す。失敗したときは、ffmpeg の標準エラーを message に残す。
    async fn run(&self, mut command: Command) -> Result<Vec<u8>> {
        command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
        let program = command.as_std().get_program().to_string_lossy().into_owned();
        let child = command.spawn().map_err(|error| {
            // 実行ファイルがないのは、利用者のファイルの問題ではなく、daemon の環境の問題である。
            Error::from_error(error, ErrorKind::UnexpectedError).with_message(format!("failed to start {program}"))
        })?;

        let output = match tokio::time::timeout(self.timeout, child.wait_with_output()).await {
            Ok(output) => output?,
            Err(_) => return Err(Error::new(ErrorKind::Timeout).with_message(format!("{program} did not finish in {:?}", self.timeout))),
        };
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let detail = stderr.lines().next().unwrap_or_default();
            return Err(Error::new(ErrorKind::UnsupportedType).with_message(format!("{program} failed: {detail}")));
        }
        Ok(output.stdout)
    }
}

/// 動画から取り出すフレームの数と位置。長さに対する割合で決める。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameSpec {
    pub count: usize,
    pub start_ratio: f64,
    pub end_ratio: f64,
}

impl FrameSpec {
    /// 長さの 10% から 90% までを等間隔に 8 点とる。先頭の 0% を避けるのは、冒頭が黒い画面やロゴであることがあるためである。
    pub const DEFAULT: Self = Self {
        count: 8,
        start_ratio: 0.1,
        end_ratio: 0.9,
    };

    /// 動画の長さ（秒）から、取り出す位置（秒）を決める。
    pub fn positions(&self, duration_secs: f64) -> Vec<f64> {
        let step = match self.count {
            0 | 1 => 0.0,
            count => (self.end_ratio - self.start_ratio) / (count - 1) as f64,
        };
        (0..self.count.max(1)).map(|index| duration_secs * (self.start_ratio + step * index as f64)).collect()
    }

    /// キャッシュのキーに含める、割合の固定小数点表現（千分率）。
    pub fn key_parts(&self) -> [u32; 3] {
        [self.count as u32, (self.start_ratio * 1000.0).round() as u32, (self.end_ratio * 1000.0).round() as u32]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_are_evenly_spaced_between_the_ratios_test() {
        let positions = FrameSpec::DEFAULT.positions(100.0);
        assert_eq!(positions.len(), 8);
        assert!((positions[0] - 10.0).abs() < 1e-9);
        assert!((positions[7] - 90.0).abs() < 1e-9);
        assert!(positions.windows(2).all(|pair| ((pair[1] - pair[0]) - 80.0 / 7.0).abs() < 1e-9));
    }

    #[test]
    fn a_single_frame_uses_the_start_position_test() {
        let spec = FrameSpec {
            count: 1,
            start_ratio: 0.25,
            end_ratio: 0.75,
        };
        assert_eq!(spec.positions(40.0), [10.0]);
        assert_eq!(FrameSpec::DEFAULT.positions(0.0), [0.0; 8]);
    }

    #[test]
    fn key_parts_are_stable_test() {
        assert_eq!(FrameSpec::DEFAULT.key_parts(), [8, 100, 900]);
    }

    #[tokio::test]
    async fn missing_executables_are_reported_as_an_environment_problem_test() {
        let tools = MediaTools::new(
            MediaOptions {
                ffmpeg_path: PathBuf::from("/nonexistent/ffmpeg"),
                ffprobe_path: PathBuf::from("/nonexistent/ffprobe"),
            },
            Duration::from_secs(5),
        );
        let error = tools.probe_duration_secs(Path::new("a.mp4")).await.unwrap_err();
        assert_eq!(*error.kind(), ErrorKind::UnexpectedError);
        let error = tools.extract_frames(Path::new("a.mp4"), &[1.0]).await.unwrap_err();
        assert_eq!(*error.kind(), ErrorKind::UnexpectedError);
    }
}
