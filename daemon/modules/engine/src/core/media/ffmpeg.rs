use std::path::Path;

use tokio::process::Command;

use super::{ProcessPriority, process};
use crate::prelude::*;

impl super::MediaTools {
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
    ///
    /// タイルに分けて符号化された画像は、タイルの stream ではなく、それらを束ねる stream group の寸法を使う。
    /// stream group を出力できない ffprobe（確かめた範囲では 5.1 と 6.1）は、そのオプションを知らずに失敗するので、オプションを外してやり直す。
    pub async fn probe_image_size(&self, file: &Path) -> Result<(u32, u32, u16, bool)> {
        let probe = |with_stream_groups: bool| {
            let mut command = Command::new(&self.options.ffprobe_path);
            command.args(["-v", "error", "-show_streams"]);
            if with_stream_groups {
                command.arg("-show_stream_groups");
            }
            command.args(["-of", "json", "-i"]).arg(file);
            self.run(command)
        };

        let output = match probe(true).await {
            Ok(output) => output,
            Err(error) if error.message().is_some_and(|message| message.contains("show_stream_groups")) => probe(false).await?,
            Err(error) => return Err(error),
        };
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

    /// ffmpeg で読む画像（HEIC、AVIF など）を、1 枚の PPM（P6、8-bit RGB）として取り出す。
    ///
    /// PPM は raw 画素に寸法のヘッダを付けるだけなので、PNG の圧縮より速い。stream を指定せず、ffmpeg の既定の選択に任せる。
    /// タイルの組み立てと `irot` の回転は ffmpeg が反映する（確かめた範囲では ffmpeg 9.0）。
    pub async fn extract_image_pixels(&self, file: &Path, priority: ProcessPriority) -> Result<Vec<u8>> {
        let mut command = Command::new(&self.options.ffmpeg_path);
        command.args(["-hide_banner", "-loglevel", "error", "-nostdin", "-i"]).arg(file).args([
            "-frames:v",
            "1",
            "-an",
            "-sn",
            "-pix_fmt",
            "rgb24",
            "-f",
            "image2pipe",
            "-vcodec",
            "ppm",
            "-",
        ]);
        self.run_with(command, priority).await
    }

    /// ffmpeg で読む画像（HEIC、AVIF など）を、1 枚の PNG として取り出す。
    ///
    /// JXL のように、ffmpeg が主画像のアルファをそのまま返す形式で使う。stream を指定せず、ffmpeg の既定の選択に任せる。
    pub async fn extract_image(&self, file: &Path, priority: ProcessPriority) -> Result<Vec<u8>> {
        let output_dir = tempfile::tempdir()?;
        let output_path = output_dir.path().join("image.png");

        let mut command = Command::new(&self.options.ffmpeg_path);
        command
            .args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y", "-i"])
            .arg(file)
            .args(["-frames:v", "1", "-an", "-sn"])
            .arg(&output_path);
        self.run_with(command, priority).await?;
        process::read_output(&output_path, file).await
    }

    /// 指定した位置（秒）のフレームを、1 回の ffmpeg の実行で PNG として取り出す。
    ///
    /// 位置ごとに `-ss` を付けた入力を並べ、キーフレーム単位で移る。一覧で眺めるためのフレームに正確な時刻は要らず、デコードする量が少ないほど速い。
    /// 動画の終わりを超えた位置など、取り出せなかったフレームは結果から除く。
    pub async fn extract_frames(&self, file: &Path, positions_secs: &[f64], priority: ProcessPriority) -> Result<Vec<Vec<u8>>> {
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
        self.run_with(command, priority).await?;

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
    use std::{path::PathBuf, time::Duration};

    use crate::core::media::{MediaOptions, MediaTools};

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
                heif_dec_path: PathBuf::from("/nonexistent/heif-dec"),
            },
            Duration::from_secs(5),
        );
        let error = tools.probe_duration_secs(Path::new("a.mp4")).await.unwrap_err();
        assert_eq!(*error.kind(), ErrorKind::UnexpectedError);
        let error = tools.extract_frames(Path::new("a.mp4"), &[1.0], ProcessPriority::Foreground).await.unwrap_err();
        assert_eq!(*error.kind(), ErrorKind::UnexpectedError);
        let error = tools.extract_image_pixels(Path::new("a.avif"), ProcessPriority::Foreground).await.unwrap_err();
        assert_eq!(*error.kind(), ErrorKind::UnexpectedError);
        let error = tools.decode_heif(Path::new("a.heic"), ProcessPriority::Foreground).await.unwrap_err();
        assert_eq!(*error.kind(), ErrorKind::UnexpectedError);
    }
}
