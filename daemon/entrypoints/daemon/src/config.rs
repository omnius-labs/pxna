use std::path::{Path, PathBuf};

use serde::Deserialize;

use omnius_pxna_engine::{
    model::ImageFormatType,
    service::{ArchiveOptions, MediaOptions, ScheduleOptions, ThumbnailOptions, ZipLegacyEncoding},
};

use crate::{
    prelude::*,
    service::{timeline::TimelineConfig, video::VideoOptions},
};

const CONFIG_FILE_NAME: &str = "pxna.toml";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct DaemonConfigToml {
    pub core: CoreConfigToml,
    pub logging: LoggingConfigToml,
    pub libraries: Vec<LibraryConfigToml>,
    #[serde(default)]
    pub storage: StorageConfigToml,
    #[serde(default)]
    pub thumbnail: ThumbnailConfigToml,
    #[serde(default)]
    pub scheduler: SchedulerConfigToml,
    #[serde(default)]
    pub media: MediaConfigToml,
    #[serde(default)]
    pub web: WebConfigToml,
    #[serde(default)]
    pub video: VideoOptions,
    #[serde(default)]
    pub listing: ListingConfig,
    #[serde(default)]
    pub timeline: TimelineConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct CoreConfigToml {
    pub state_dir: String,
    pub listen_addr: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct LoggingConfigToml {
    pub level: String,
    pub json: bool,
    /// 標準出力に加えて追記するログ file。設定 directory が基準で、省略すると file には出さない。
    #[serde(default)]
    pub file: Option<String>,
    /// 処理時間の計測ログ（`pxna::performance`）を debug で出す。`level` が info でも、この計測だけを有効にする。
    #[serde(default)]
    pub performance: bool,
}

/// `dir` を省略すると、daemon は web を配信せず API だけを提供する。
/// `insecure_cookie` を有効にすると、セッション cookie から `Secure` と `__Host-` 接頭辞を外す。
/// HTTPS を介さずに `localhost` 以外から使い、経路を別の手段（Tailscale など）で守るときだけ有効にする。
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct WebConfigToml {
    pub dir: Option<String>,
    #[serde(default)]
    pub insecure_cookie: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct StorageConfigToml {
    #[serde(default)]
    pub archive: ArchiveConfigToml,
}

/// 省略した key は `ArchiveOptions` の既定値を使う。
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArchiveConfigToml {
    pub cache_capacity_bytes: Option<u64>,
    pub index_cache_capacity_bytes: Option<u64>,
    pub max_nested_depth: Option<usize>,
    pub max_entry_count: Option<usize>,
    pub max_materialized_file_size: Option<u64>,
    pub max_block_extract_bytes: Option<u64>,
    pub extract_timeout_secs: Option<u64>,
    pub max_concurrent_extractions: Option<usize>,
    pub zip_legacy_encoding: Option<String>,
}

impl ArchiveConfigToml {
    fn into_options(self) -> Result<ArchiveOptions> {
        let defaults = ArchiveOptions::default();
        let zip_legacy_encoding = match self.zip_legacy_encoding.as_deref() {
            None => defaults.zip_legacy_encoding,
            Some("cp932") => ZipLegacyEncoding::Cp932,
            Some("none") => ZipLegacyEncoding::None,
            Some(other) => {
                return Err(Error::new(ErrorKind::InvalidFormat).with_message(format!("storage.archive.zip_legacy_encoding must be \"cp932\" or \"none\": {other:?}")));
            }
        };

        let options = ArchiveOptions {
            cache_capacity_bytes: self.cache_capacity_bytes.unwrap_or(defaults.cache_capacity_bytes),
            index_cache_capacity_bytes: self.index_cache_capacity_bytes.unwrap_or(defaults.index_cache_capacity_bytes),
            max_nested_depth: self.max_nested_depth.unwrap_or(defaults.max_nested_depth),
            max_entry_count: self.max_entry_count.unwrap_or(defaults.max_entry_count),
            max_materialized_file_size: self.max_materialized_file_size.unwrap_or(defaults.max_materialized_file_size),
            max_block_extract_bytes: self.max_block_extract_bytes.unwrap_or(defaults.max_block_extract_bytes),
            extract_timeout: self.extract_timeout_secs.map(std::time::Duration::from_secs).unwrap_or(defaults.extract_timeout),
            max_concurrent_extractions: self.max_concurrent_extractions.unwrap_or(defaults.max_concurrent_extractions),
            zip_legacy_encoding,
        };

        options.validate().map_err(|error| {
            let message = error.message().unwrap_or_default().to_string();
            Error::from_error(error, ErrorKind::InvalidFormat).with_message(format!("invalid [storage.archive]: {message}"))
        })?;

        Ok(options)
    }
}

/// 同時実行の枠の数。省略した key は `ScheduleOptions` の既定値を使う。
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct SchedulerConfigToml {
    pub image_slots: Option<usize>,
    pub image_interactive_slots: Option<usize>,
    pub process_slots: Option<usize>,
    pub process_interactive_slots: Option<usize>,
    pub video_slots: Option<usize>,
    pub video_interactive_slots: Option<usize>,
}

impl SchedulerConfigToml {
    fn into_options(self) -> Result<ScheduleOptions> {
        let defaults = ScheduleOptions::default();
        let options = ScheduleOptions {
            image_slots: self.image_slots.unwrap_or(defaults.image_slots),
            image_interactive_slots: self.image_interactive_slots.unwrap_or(defaults.image_interactive_slots),
            process_slots: self.process_slots.unwrap_or(defaults.process_slots),
            process_interactive_slots: self.process_interactive_slots.unwrap_or(defaults.process_interactive_slots),
            video_slots: self.video_slots.unwrap_or(defaults.video_slots),
            video_interactive_slots: self.video_interactive_slots.unwrap_or(defaults.video_interactive_slots),
        };
        options.validate().map_err(|error| {
            let message = error.message().unwrap_or_default().to_string();
            Error::from_error(error, ErrorKind::InvalidFormat).with_message(format!("invalid [scheduler]: {message}"))
        })?;
        Ok(options)
    }
}

/// 省略した key は `ThumbnailOptions` の既定値を使う。
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct ThumbnailConfigToml {
    pub sizes: Option<Vec<u32>>,
    pub display_sizes: Option<Vec<u32>>,
    pub format: Option<String>,
    pub quality: Option<u8>,
    pub display_quality: Option<u8>,
    pub cache_capacity_bytes: Option<u64>,
    pub display_cache_capacity_bytes: Option<u64>,
    /// `[scheduler]` へ移した key。残っていたら、移す先を示して起動を止める。
    pub max_concurrent_images: Option<usize>,
    pub max_concurrent_videos: Option<usize>,
    pub video_timeout_secs: Option<u64>,
}

/// ffmpeg、ffprobe、heif-dec の場所。サムネイルと動画の再生が共有する。
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct MediaConfigToml {
    pub ffmpeg_path: Option<String>,
    pub ffprobe_path: Option<String>,
    pub heif_dec_path: Option<String>,
}

impl MediaConfigToml {
    fn into_options(self) -> MediaOptions {
        let defaults = MediaOptions::default();
        MediaOptions {
            ffmpeg_path: self.ffmpeg_path.map(PathBuf::from).unwrap_or(defaults.ffmpeg_path),
            ffprobe_path: self.ffprobe_path.map(PathBuf::from).unwrap_or(defaults.ffprobe_path),
            heif_dec_path: self.heif_dec_path.map(PathBuf::from).unwrap_or(defaults.heif_dec_path),
        }
    }
}

impl ThumbnailConfigToml {
    fn into_options(self) -> Result<ThumbnailOptions> {
        for (key, moved) in [
            ("max_concurrent_images", self.max_concurrent_images.is_some()),
            ("max_concurrent_videos", self.max_concurrent_videos.is_some()),
        ] {
            if moved {
                let replacement = if key == "max_concurrent_videos" { "video_slots" } else { "image_slots" };
                return Err(Error::new(ErrorKind::InvalidFormat).with_message(format!("invalid [thumbnail]: {key} was replaced by {replacement} in [scheduler]")));
            }
        }
        let defaults = ThumbnailOptions::default();
        let format = match self.format.as_deref() {
            None => defaults.format,
            Some("webp") => ImageFormatType::Webp,
            Some("jpeg") => ImageFormatType::Jpeg,
            Some("png") => ImageFormatType::Png,
            Some(other) => {
                return Err(Error::new(ErrorKind::InvalidFormat).with_message(format!("invalid [thumbnail]: format must be \"webp\", \"jpeg\", or \"png\": {other:?}")));
            }
        };
        let options = ThumbnailOptions {
            sizes: self.sizes.unwrap_or(defaults.sizes),
            display_sizes: self.display_sizes.unwrap_or(defaults.display_sizes),
            format,
            quality: self.quality.unwrap_or(defaults.quality),
            display_quality: self.display_quality.unwrap_or(defaults.display_quality),
            cache_capacity_bytes: self.cache_capacity_bytes.unwrap_or(defaults.cache_capacity_bytes),
            display_cache_capacity_bytes: self.display_cache_capacity_bytes.unwrap_or(defaults.display_cache_capacity_bytes),
            video_timeout: self.video_timeout_secs.map(std::time::Duration::from_secs).unwrap_or(defaults.video_timeout),
        };

        options.validate().map_err(|error| {
            let message = error.message().unwrap_or_default().to_string();
            Error::from_error(error, ErrorKind::InvalidFormat).with_message(format!("invalid [thumbnail]: {message}"))
        })?;

        Ok(options)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct LibraryConfigToml {
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub readonly: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonConfig {
    pub core: CoreConfig,
    pub logging: LoggingConfig,
    pub libraries: Vec<LibraryConfig>,
    pub storage: StorageConfig,
    pub thumbnail: ThumbnailOptions,
    pub scheduler: ScheduleOptions,
    pub media: MediaOptions,
    pub web: WebConfig,
    pub video: VideoOptions,
    pub listing: ListingConfig,
    pub timeline: TimelineConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreConfig {
    pub state_dir: PathBuf,
    pub listen_addr: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoggingConfig {
    pub level: String,
    pub json: bool,
    pub file: Option<PathBuf>,
    pub performance: bool,
}

/// daemon が配信する web の成果物の置き場（design.md §10.1）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WebConfig {
    pub dir: Option<PathBuf>,
    pub insecure_cookie: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageConfig {
    pub archive: ArchiveOptions,
}

/// 一覧のスナップショットの期限と、保持・再帰走査の件数上限。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ListingConfig {
    pub snapshot_idle_secs: u64,
    pub snapshot_max_age_secs: u64,
    pub snapshot_capacity_entries: usize,
    pub recursive_max_entries: usize,
    pub summary_index_capacity_bytes: u64,
}

impl Default for ListingConfig {
    fn default() -> Self {
        Self {
            snapshot_idle_secs: 600,
            snapshot_max_age_secs: 300,
            snapshot_capacity_entries: 500_000,
            recursive_max_entries: 200_000,
            summary_index_capacity_bytes: 268_435_456,
        }
    }
}

impl ListingConfig {
    pub fn validate(&self) -> Result<()> {
        if self.summary_index_capacity_bytes == 0
            || self.snapshot_capacity_entries > i64::MAX as usize
            || self.snapshot_idle_secs == 0
            || self.snapshot_max_age_secs == 0
            || self.recursive_max_entries == 0
            || self.recursive_max_entries >= self.snapshot_capacity_entries
        {
            return Err(Error::new(ErrorKind::InvalidFormat).with_message("[listing] values must be positive and recursive_max_entries < snapshot_capacity_entries"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryConfig {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub readonly: bool,
}

impl LibraryConfig {
    // A library id is used as a URL path segment, so a misconfigured one must fail
    // at startup instead of breaking routing once the daemon is serving.
    fn validate_id(id: &str) -> Result<()> {
        let valid = !id.is_empty() && id.len() <= LIBRARY_ID_MAX_LENGTH && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));

        if !valid {
            return Err(Error::new(ErrorKind::InvalidFormat).with_message(format!("library id must be 1-{LIBRARY_ID_MAX_LENGTH} characters of [A-Za-z0-9._-]: {id:?}")));
        }

        // クライアントや中継が URL の dot segment を除去すると、その Library へルーティングできなくなる。
        if matches!(id, "." | "..") {
            return Err(Error::new(ErrorKind::InvalidFormat).with_message(format!("library id must not be a dot segment: {id:?}")));
        }

        Ok(())
    }

    /// 同じ id が 2 つあると、後に書いた Library だけが使われ、先の Library は黙って API から見えなくなる。
    fn validate_unique_ids(libraries: &[LibraryConfig]) -> Result<()> {
        let mut seen = std::collections::HashSet::new();
        for library in libraries {
            if !seen.insert(library.id.as_str()) {
                return Err(Error::new(ErrorKind::InvalidFormat).with_message(format!("library id is duplicated: {:?}", library.id)));
            }
        }

        Ok(())
    }
}

impl DaemonConfig {
    pub async fn load<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let dir = dir.as_ref();
        tokio::fs::create_dir_all(dir).await?;

        let toml = Self::load_toml(dir).await?;

        let libraries = toml
            .libraries
            .into_iter()
            .map(|lib| {
                LibraryConfig::validate_id(&lib.id)?;

                // state_dir と同じく設定 directory を基準にし、engine が要求する絶対 path にそろえる。
                Ok(LibraryConfig {
                    id: lib.id,
                    name: lib.name,
                    path: std::path::absolute(dir.join(lib.path))?,
                    readonly: lib.readonly,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        LibraryConfig::validate_unique_ids(&libraries)?;

        toml.listing.validate()?;
        toml.timeline.validate(&libraries)?;
        toml.video
            .validate()
            .map_err(|error| Error::from_error(error, ErrorKind::InvalidFormat).with_message("invalid [video] configuration"))?;
        Ok(DaemonConfig {
            core: CoreConfig {
                state_dir: dir.join(toml.core.state_dir),
                listen_addr: toml.core.listen_addr,
            },
            logging: LoggingConfig {
                level: toml.logging.level,
                json: toml.logging.json,
                file: toml.logging.file.map(|file| dir.join(file)),
                performance: toml.logging.performance,
            },
            libraries,
            storage: StorageConfig {
                archive: toml.storage.archive.into_options()?,
            },
            thumbnail: toml.thumbnail.into_options()?,
            scheduler: toml.scheduler.into_options()?,
            media: toml.media.into_options(),
            video: toml.video,
            listing: toml.listing,
            timeline: toml.timeline,
            // state_dir と同じく設定 directory を基準にする。
            web: WebConfig {
                dir: toml.web.dir.map(|web_dir| dir.join(web_dir)),
                insecure_cookie: toml.web.insecure_cookie,
            },
        })
    }

    async fn load_toml(dir: &Path) -> Result<DaemonConfigToml> {
        let toml_path = dir.join(CONFIG_FILE_NAME);
        let toml = tokio::fs::read_to_string(toml_path).await?;
        Ok(toml::from_str(&toml)?)
    }
}

const LIBRARY_ID_MAX_LENGTH: usize = 64;

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use testresult::TestResult;

    use super::*;

    const BASE_TOML: &str = r#"
        [core]
        state_dir = "./state"
        listen_addr = "0.0.0.0:6050"

        [logging]
        level = "info"
        json = false

        [[libraries]]
        id = "photos"
        name = "Photos"
        path = "/srv/photos"
    "#;

    async fn load(toml: &str) -> Result<DaemonConfig> {
        let dir = tempfile::tempdir()?;
        std::fs::File::create(dir.path().join(CONFIG_FILE_NAME))?.write_all(toml.as_bytes())?;
        DaemonConfig::load(dir.path()).await
    }

    #[tokio::test]
    async fn listing_defaults_and_overrides_test() -> TestResult {
        assert_eq!(load(BASE_TOML).await?.listing, ListingConfig::default());
        let config = load(&format!(
            "{BASE_TOML}\n[listing]\nsnapshot_idle_secs = 7\nsnapshot_max_age_secs = 3\nsnapshot_capacity_entries = 20\nrecursive_max_entries = 10\n"
        ))
        .await?;
        assert_eq!(
            config.listing,
            ListingConfig {
                snapshot_idle_secs: 7,
                snapshot_max_age_secs: 3,
                snapshot_capacity_entries: 20,
                recursive_max_entries: 10,
                ..Default::default()
            }
        );
        assert_eq!(
            load(&format!("{BASE_TOML}\n[listing]\nsnapshot_idle_secs = 7\n")).await?.listing.snapshot_capacity_entries,
            500_000
        );
        Ok(())
    }

    #[tokio::test]
    async fn listing_rejects_invalid_values_test() -> TestResult {
        for setting in [
            "summary_index_capacity_bytes = 0",
            "snapshot_idle_secs = 0",
            "snapshot_max_age_secs = 0",
            "recursive_max_entries = 0",
            "snapshot_capacity_entries = 200000",
            "recursive_max_entries = 500000",
        ] {
            let error = load(&format!("{BASE_TOML}\n[listing]\n{setting}\n")).await.expect_err(setting);
            assert_eq!(*error.kind(), ErrorKind::InvalidFormat);
        }
        assert_eq!(*load(&format!("{BASE_TOML}\n[listing]\nunknown = 1\n")).await.unwrap_err().kind(), ErrorKind::SerdeError);
        Ok(())
    }

    #[tokio::test]
    async fn storage_archive_defaults_when_omitted_test() -> TestResult {
        let config = load(BASE_TOML).await?;
        assert_eq!(config.storage.archive, ArchiveOptions::default());
        Ok(())
    }

    #[tokio::test]
    async fn storage_archive_overrides_test() -> TestResult {
        let toml = format!("{BASE_TOML}\n[storage.archive]\ncache_capacity_bytes = 1024\nextract_timeout_secs = 10\nzip_legacy_encoding = \"none\"\n");
        let config = load(&toml).await?;
        assert_eq!(config.storage.archive.cache_capacity_bytes, 1024);
        assert_eq!(config.storage.archive.extract_timeout, std::time::Duration::from_secs(10));
        assert_eq!(config.storage.archive.zip_legacy_encoding, ZipLegacyEncoding::None);
        assert_eq!(config.storage.archive.max_nested_depth, ArchiveOptions::default().max_nested_depth);
        Ok(())
    }

    #[tokio::test]
    async fn thumbnail_defaults_when_omitted_test() -> TestResult {
        let config = load(BASE_TOML).await?;
        assert_eq!(config.thumbnail, ThumbnailOptions::default());
        Ok(())
    }

    #[tokio::test]
    async fn thumbnail_overrides_test() -> TestResult {
        let toml = format!("{BASE_TOML}\n[thumbnail]\nsizes = [64, 256]\ncache_capacity_bytes = 1024\n");
        let config = load(&toml).await?;
        assert_eq!(config.thumbnail.sizes, [64, 256]);
        assert_eq!(config.thumbnail.cache_capacity_bytes, 1024);
        assert_eq!(config.thumbnail.display_sizes, ThumbnailOptions::default().display_sizes);
        assert_eq!(config.thumbnail.display_sizes, [2048, 4096]);
        assert_eq!(config.thumbnail.format, ImageFormatType::Webp);
        assert_eq!((config.thumbnail.quality, config.thumbnail.display_quality), (80, 90));
        Ok(())
    }

    #[tokio::test]
    async fn scheduler_defaults_overrides_and_moved_thumbnail_keys_test() -> TestResult {
        assert_eq!(load(BASE_TOML).await?.scheduler, ScheduleOptions::default());
        let config = load(&format!(
            "{BASE_TOML}\n[scheduler]\nimage_slots = 2\nprocess_interactive_slots = 0\nvideo_slots = 3\nvideo_interactive_slots = 2\n"
        ))
        .await?;
        assert_eq!(
            config.scheduler,
            ScheduleOptions {
                image_slots: 2,
                process_interactive_slots: 0,
                video_slots: 3,
                video_interactive_slots: 2,
                ..Default::default()
            }
        );
        load(&format!("{BASE_TOML}\n[scheduler]\nprocess_slots = 0\n"))
            .await
            .expect_err("a pool needs a shared slot");
        load(&format!("{BASE_TOML}\n[scheduler]\nvideo_slots = 0\n"))
            .await
            .expect_err("the video pool needs a shared slot");
        for key in ["max_concurrent_images", "max_concurrent_videos"] {
            let error = load(&format!("{BASE_TOML}\n[thumbnail]\n{key} = 2\n")).await.expect_err("moved key");
            let replacement = if key == "max_concurrent_videos" { "video_slots" } else { "image_slots" };
            assert!(
                error.message().unwrap_or_default().contains(&format!("replaced by {replacement} in [scheduler]")),
                "{key}: {error:?}"
            );
        }
        Ok(())
    }

    #[tokio::test]
    async fn media_defaults_and_overrides_test() -> TestResult {
        assert_eq!(load(BASE_TOML).await?.media, MediaOptions::default());

        let toml = format!("{BASE_TOML}\n[media]\nffmpeg_path = \"/opt/ffmpeg/bin/ffmpeg\"\n[thumbnail]\nvideo_timeout_secs = 5\n");
        let config = load(&toml).await?;
        assert_eq!(config.media.ffmpeg_path, PathBuf::from("/opt/ffmpeg/bin/ffmpeg"));
        assert_eq!(config.media.ffprobe_path, PathBuf::from("ffprobe"));
        assert_eq!(config.media.heif_dec_path, PathBuf::from("heif-dec"));
        assert_eq!(config.thumbnail.video_timeout, std::time::Duration::from_secs(5));

        let toml = format!("{BASE_TOML}\n[media]\nunknown_key = 1\n");
        load(&toml).await.expect_err("unknown key should be rejected");
        Ok(())
    }

    #[tokio::test]
    async fn thumbnail_format_and_quality_override_test() -> TestResult {
        for (value, expected) in [("webp", ImageFormatType::Webp), ("jpeg", ImageFormatType::Jpeg), ("png", ImageFormatType::Png)] {
            let toml = format!("{BASE_TOML}\n[thumbnail]\nformat = \"{value}\"\n");
            assert_eq!(load(&toml).await?.thumbnail.format, expected, "{value}");
        }
        let toml = format!("{BASE_TOML}\n[thumbnail]\nquality = 30\ndisplay_quality = 100\n");
        let config = load(&toml).await?;
        assert_eq!((config.thumbnail.quality, config.thumbnail.display_quality), (30, 100));
        Ok(())
    }

    #[tokio::test]
    async fn thumbnail_rejects_invalid_values_test() -> TestResult {
        for section in [
            "sizes = []",
            "sizes = [0, 128]",
            "cache_capacity_bytes = 0",
            "max_concurrent_images = 0",
            "max_concurrent_videos = 0",
            "video_timeout_secs = 0",
            "format = \"gif\"",
            "format = \"unknown\"",
            "quality = 0",
            "quality = 101",
            "display_quality = 0",
            "display_quality = 101",
        ] {
            let toml = format!("{BASE_TOML}\n[thumbnail]\n{section}\n");
            let error = load(&toml).await.expect_err(section);
            assert_eq!(*error.kind(), ErrorKind::InvalidFormat, "{section}");
        }

        let toml = format!("{BASE_TOML}\n[thumbnail]\nunknown_key = 1\n");
        load(&toml).await.expect_err("unknown key should be rejected");
        Ok(())
    }

    #[tokio::test]
    async fn logging_file_and_performance_default_off_and_override_test() -> TestResult {
        let logging = load(BASE_TOML).await?.logging;
        assert_eq!((logging.file, logging.performance), (None, false));

        let config = load(&BASE_TOML.replace("json = false", "json = false\nfile = \"logs/pxna.log\"\nperformance = true")).await?;
        assert!(config.logging.performance);
        assert!(config.logging.file.is_some_and(|file| file.ends_with("logs/pxna.log")));
        Ok(())
    }

    #[tokio::test]
    async fn web_dir_resolves_against_config_dir_test() -> TestResult {
        assert_eq!(load(BASE_TOML).await?.web.dir, None);

        let dir = tempfile::tempdir()?;
        let toml = format!("{BASE_TOML}\n[web]\ndir = \"../ui-web/dist\"\n");
        std::fs::File::create(dir.path().join(CONFIG_FILE_NAME))?.write_all(toml.as_bytes())?;
        let config = DaemonConfig::load(dir.path()).await?;
        assert_eq!(config.web.dir, Some(dir.path().join("../ui-web/dist")));
        assert!(!config.web.insecure_cookie);

        let toml = format!("{BASE_TOML}\n[web]\ninsecure_cookie = true\n");
        assert!(load(&toml).await?.web.insecure_cookie);

        let toml = format!("{BASE_TOML}\n[web]\nunknown_key = 1\n");
        load(&toml).await.expect_err("unknown key should be rejected");
        Ok(())
    }

    #[tokio::test]
    async fn storage_archive_rejects_invalid_values_test() -> TestResult {
        for section in ["cache_capacity_bytes = 0", "max_nested_depth = 0", "zip_legacy_encoding = \"utf16\""] {
            let toml = format!("{BASE_TOML}\n[storage.archive]\n{section}\n");
            let error = load(&toml).await.expect_err(section);
            assert_eq!(*error.kind(), ErrorKind::InvalidFormat, "{section}");
        }

        let toml = format!("{BASE_TOML}\n[storage.archive]\nunknown_key = 1\n");
        load(&toml).await.expect_err("unknown key should be rejected");
        Ok(())
    }

    #[tokio::test]
    async fn load_when_library_id_is_invalid() -> TestResult {
        let toml = r#"
            [core]
            state_dir = "./state"
            listen_addr = "0.0.0.0:6050"

            [logging]
            level = "info"
            json = false

            [[libraries]]
            id = "photos/2026"
            name = "Photos"
            path = "/srv/photos"
        "#;

        let dir = tempfile::tempdir()?;
        std::fs::File::create(dir.path().join("pxna.toml"))?.write_all(toml.as_bytes())?;

        let error = DaemonConfig::load(dir.path()).await.expect_err("invalid library id should be rejected");
        assert_eq!(*error.kind(), ErrorKind::InvalidFormat);

        Ok(())
    }

    fn library_toml(id: &str) -> String {
        format!("\n[[libraries]]\nid = {id:?}\nname = \"Other\"\npath = \"/srv/other\"\n")
    }

    #[tokio::test]
    async fn rejects_dot_segment_library_id_test() -> TestResult {
        for id in [".", ".."] {
            let toml = format!("{BASE_TOML}{}", library_toml(id));
            let error = load(&toml).await.expect_err(id);
            assert_eq!(*error.kind(), ErrorKind::InvalidFormat, "{id}");
        }

        let toml = format!("{BASE_TOML}{}", library_toml("..photos"));
        assert_eq!(load(&toml).await?.libraries[1].id, "..photos", "dot で始まるだけの id は許可する");
        Ok(())
    }

    #[tokio::test]
    async fn rejects_duplicated_library_id_test() -> TestResult {
        let toml = format!("{BASE_TOML}{}", library_toml("photos"));
        let error = load(&toml).await.expect_err("duplicated library id should be rejected");
        assert_eq!(*error.kind(), ErrorKind::InvalidFormat);
        Ok(())
    }

    #[tokio::test]
    async fn timeline_date_rules_are_parsed_and_validated_test() -> TestResult {
        let toml = format!("{BASE_TOML}\n[[timeline.date_rules]]\npath = \"**\"\nname = 'IMG-(?P<date>\\d{{8}})'\nformat = \"%Y%m%d\"\n");
        let config = load(&toml).await?;
        assert_eq!(config.timeline.date_rules.len(), 1);
        assert_eq!(config.timeline.date_rules[0].path, "**");

        let unknown_library = format!("{BASE_TOML}\n[[timeline.date_rules]]\nlibrary = \"missing\"\npath = \"**\"\nname = 'IMG-(?P<date>\\d{{8}})'\nformat = \"%Y%m%d\"\n");
        let error = load(&unknown_library).await.expect_err("unknown library");
        assert_eq!(*error.kind(), ErrorKind::InvalidFormat);
        assert!(error.message().unwrap_or_default().contains("library"));

        let unnamed_capture = format!("{BASE_TOML}\n[[timeline.date_rules]]\npath = \"**\"\nname = 'IMG-(\\d{{8}})'\nformat = \"%Y%m%d\"\n");
        let error = load(&unnamed_capture).await.expect_err("missing capture");
        assert_eq!(*error.kind(), ErrorKind::InvalidFormat);
        Ok(())
    }
}
