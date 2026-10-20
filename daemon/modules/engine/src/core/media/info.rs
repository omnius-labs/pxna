use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

use super::ImageInfo;
use crate::prelude::*;

/// 画像か動画のメディア情報。どちらかは拡張子から決める。
#[derive(Debug, Clone, PartialEq)]
pub enum MediaInfo {
    Video(VideoInfo),
    Image(ImageInfo),
}

impl MediaInfo {
    /// 動画の情報を取り出す。画像のときは `UnsupportedType` にする。
    pub fn video(&self) -> Result<&VideoInfo> {
        match self {
            Self::Video(info) => Ok(info),
            Self::Image(_) => Err(Error::new(ErrorKind::UnsupportedType).with_message("not a video")),
        }
    }
}

/// ffprobe の情報を、再生能力の判定に必要な値へ絞ったもの。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoInfo {
    pub container: String,
    pub video_codec: String,
    pub audio_codec: Option<String>,
    pub duration_secs: f64,
    pub width: u32,
    pub height: u32,
    pub frame_rate: f64,
    pub content_type: String,
}

#[derive(Deserialize)]
struct ProbeOutput {
    streams: Vec<ProbeStream>,
    format: ProbeFormat,
}

#[derive(Deserialize)]
struct ProbeFormat {
    format_name: String,
    duration: Option<String>,
}

#[derive(Deserialize)]
struct ProbeStream {
    codec_type: Option<String>,
    codec_name: Option<String>,
    profile: Option<String>,
    level: Option<u32>,
    width: Option<u32>,
    height: Option<u32>,
    avg_frame_rate: Option<String>,
    duration: Option<String>,
}

impl VideoInfo {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let probe: ProbeOutput = serde_json::from_slice(bytes).map_err(|error| Error::from_error(error, ErrorKind::UnsupportedType).with_message("invalid media information"))?;
        let video = probe
            .streams
            .iter()
            .find(|stream| stream.codec_type.as_deref() == Some("video"))
            .ok_or_else(|| Error::new(ErrorKind::UnsupportedType).with_message("no video stream"))?;
        let duration_secs = probe
            .format
            .duration
            .as_ref()
            .or(video.duration.as_ref())
            .and_then(|value| value.parse::<f64>().ok())
            .filter(|duration| duration.is_finite() && *duration >= 0.0)
            .ok_or_else(|| Error::new(ErrorKind::UnsupportedType).with_message("media duration is unavailable"))?;
        let video_codec = video.codec_name.clone().ok_or_else(|| Error::new(ErrorKind::UnsupportedType))?;
        let audio = probe.streams.iter().find(|stream| stream.codec_type.as_deref() == Some("audio"));
        let audio_codec = audio.and_then(|stream| stream.codec_name.clone());
        let codec = match video_codec.as_str() {
            "h264" => {
                let profile = match video.profile.as_deref() {
                    Some("High") => "6400",
                    Some("Main") => "4D40",
                    Some("Constrained Baseline") => "42E0",
                    Some("Baseline") => "4200",
                    _ => "",
                };
                if profile.is_empty() {
                    "unknown".to_string()
                } else {
                    format!("avc1.{profile}{:02X}", video.level.unwrap_or(31))
                }
            }
            "vp8" => "vp8".to_string(),
            "vp9" => "vp9".to_string(),
            _ => "unknown".to_string(),
        };
        let mime = if probe.format.format_name.split(',').any(|name| name == "mp4" || name == "mov") {
            "video/mp4"
        } else if probe.format.format_name.split(',').any(|name| name == "webm") {
            "video/webm"
        } else {
            "application/octet-stream"
        };
        let audio_type = audio.map(|stream| match (stream.codec_name.as_deref(), stream.profile.as_deref()) {
            (Some("aac"), Some("LC")) => "mp4a.40.2",
            (Some("opus"), _) => "opus",
            (Some("vorbis"), _) => "vorbis",
            _ => "unknown",
        });
        let codecs = match audio_type {
            Some(audio) => format!("{codec},{audio}"),
            None => codec,
        };
        let frame_rate = video
            .avg_frame_rate
            .as_deref()
            .and_then(|value| value.split_once('/'))
            .and_then(|(a, b)| {
                let rate = a.parse::<f64>().ok()? / b.parse::<f64>().ok()?;
                (rate.is_finite() && rate > 0.0).then_some(rate)
            })
            .unwrap_or(0.0);
        Ok(Self {
            container: probe.format.format_name,
            video_codec,
            audio_codec,
            duration_secs,
            width: video.width.unwrap_or(0),
            height: video.height.unwrap_or(0),
            frame_rate,
            content_type: format!("{mime}; codecs=\"{codecs}\""),
        })
    }
}

/// ffprobe が返す、コンテナと stream の `creation_time`。
pub struct CreationTime;

impl CreationTime {
    /// MP4 と QuickTime の時刻の起点（1904-01-01 0 時 UTC）の UNIX 時刻。
    const QUICKTIME_EPOCH_SECS: i64 = -2_082_844_800;

    /// コンテナの値を先に、なければ値を持つ最初の stream の値を使う。
    /// 時刻を書かなかった file に現れる、時刻の起点ちょうどの値は、書かれていないものとみなす。
    pub fn parse(output: &[u8]) -> Option<DateTime<Utc>> {
        #[derive(Deserialize)]
        struct Probe {
            format: Option<Tagged>,
            #[serde(default)]
            streams: Vec<Tagged>,
        }
        #[derive(Deserialize)]
        struct Tagged {
            tags: Option<Tags>,
        }
        #[derive(Deserialize)]
        struct Tags {
            creation_time: Option<String>,
        }

        let probe: Probe = serde_json::from_slice(output).ok()?;
        probe
            .format
            .into_iter()
            .chain(probe.streams)
            .filter_map(|tagged| tagged.tags?.creation_time)
            .find_map(|value| Self::instant(&value))
    }

    fn instant(value: &str) -> Option<DateTime<Utc>> {
        let value = value.trim();
        let time = DateTime::parse_from_rfc3339(value)
            .map(|time| time.with_timezone(&Utc))
            .ok()
            .or_else(|| NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S").ok().map(|time| time.and_utc()))?;
        (time.timestamp() != Self::QUICKTIME_EPOCH_SECS && time.timestamp() != 0).then_some(time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn media_information_selects_video_and_preserves_optional_audio_test() -> testresult::TestResult {
        let bytes = br#"{"format":{"format_name":"mov,mp4,m4a","duration":"7.5"},"streams":[{"codec_type":"video","codec_name":"h264","profile":"High","level":40,"width":640,"height":360,"avg_frame_rate":"30000/1001"},{"codec_type":"audio","codec_name":"aac","profile":"LC"}]}"#;
        let info = VideoInfo::parse(bytes)?;
        assert_eq!(info.content_type, "video/mp4; codecs=\"avc1.640028,mp4a.40.2\"");
        assert_eq!(info.duration_secs, 7.5);
        assert!((info.frame_rate - 29.97).abs() < 0.01);
        assert_eq!(info.audio_codec.as_deref(), Some("aac"));
        assert!(VideoInfo::parse(br#"{"format":{"format_name":"matroska","duration":"NaN"},"streams":[]}"#).is_err());
        Ok(())
    }

    #[test]
    fn creation_time_prefers_the_container_and_ignores_epoch_values_test() {
        let both = br#"{"streams":[{"tags":{"creation_time":"2023-01-02T03:04:05.000000Z"}}],"format":{"tags":{"creation_time":"2024-05-01T12:34:56.000000Z"}}}"#;
        assert_eq!(CreationTime::parse(both).map(|time| time.to_rfc3339()), Some("2024-05-01T12:34:56+00:00".to_string()));
        let stream_only = br#"{"streams":[{},{"tags":{"creation_time":"2023-01-02T03:04:05Z"}}],"format":{"tags":{}}}"#;
        assert_eq!(
            CreationTime::parse(stream_only).map(|time| time.to_rfc3339()),
            Some("2023-01-02T03:04:05+00:00".to_string())
        );
        let epoch = br#"{"streams":[],"format":{"tags":{"creation_time":"1904-01-01T00:00:00.000000Z"}}}"#;
        assert_eq!(CreationTime::parse(epoch), None);
        let unix_epoch = br#"{"streams":[],"format":{"tags":{"creation_time":"1970-01-01T00:00:00Z"}}}"#;
        assert_eq!(CreationTime::parse(unix_epoch), None);
        assert_eq!(CreationTime::parse(br#"{"streams":[],"format":{}}"#), None);
        assert_eq!(CreationTime::parse(b"not json"), None);
    }
}
