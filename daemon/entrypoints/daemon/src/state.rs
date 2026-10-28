use std::{path::PathBuf, sync::Arc};

use chrono::Utc;
use omnius_core_base::{
    clock::{Clock, ClockUtc},
    tsid::TsidProviderImpl,
};
use omnius_pxna_engine::service::{
    ArchiveRuntime, AuthService, LocalStorageDriver, MediaInfoService, MediaPools, MediaTools, PreferencesService, RatingTagService, Scheduler, StorageService, ThumbnailGenerator,
    WorkBudget,
};
use rand::{SeedableRng as _, rngs::ChaCha20Rng};

use crate::{
    config::DaemonConfig,
    db::DatabaseBootstrap,
    prelude::*,
    service::{
        library::LibraryBrowser,
        listing::ListingSnapshots,
        mutation::{LibraryMutation, OperationService, UploadService},
        sockets::image::{ImageSocketLimits, ImageSockets},
        timeline::TimelineService,
        video::VideoService,
    },
};

const TSID_RANDOM_BYTE_COUNT: usize = 16;

pub struct DaemonState {
    pub conf: DaemonConfig,
    pub library: LibraryBrowser,
    pub auth: AuthService,
    pub preferences: PreferencesService,
    pub rating_tags: Arc<RatingTagService>,
    pub thumbnails: ThumbnailGenerator,
    pub mutation: Arc<LibraryMutation>,
    pub uploads: Arc<UploadService>,
    pub operations: Arc<OperationService>,
    pub media_info: Arc<MediaInfoService>,
    /// 画像の変換と外部プロセスの枠。一覧の枠と同じ scheduler に属する。
    pub pools: MediaPools,
    pub videos: Arc<VideoService>,
    pub timeline: Arc<TimelineService>,
    /// The open image sockets, their limits, and the signal that closes them at shutdown.
    pub image_sockets: Arc<ImageSockets>,
}

impl DaemonState {
    pub async fn new(conf: DaemonConfig) -> Result<Self> {
        Self::new_with_clock(conf, Arc::new(ClockUtc)).await
    }

    pub async fn new_with_clock(conf: DaemonConfig, clock: Arc<dyn Clock<Utc> + Send + Sync>) -> Result<Self> {
        conf.listing.validate()?;
        conf.timeline.validate(&conf.libraries)?;
        let state_dir = PathBuf::from(&conf.core.state_dir);
        tokio::fs::create_dir_all(&state_dir).await?;

        // 書き込みの staging は公開先と同じディレクトリに取るため、プロセス共通の一時ディレクトリは持たない。

        // 一覧、背景処理、サムネイルの生成の枠は、1 つの scheduler で優先度をそろえる。
        let scheduler = Arc::new(Scheduler::default());
        let budget = Arc::new(WorkBudget::new(&scheduler));
        let pools = MediaPools::new(&scheduler, &conf.scheduler);

        // HTTP サーバを開く前に extraction cache を開き、前回の残骸を回収しておく。
        let archive = Arc::new(
            ArchiveRuntime::open(conf.storage.archive.clone(), &Self::archive_cache_dir(&state_dir))
                .await?
                .with_budget(budget.clone()),
        );

        let rng = ChaCha20Rng::try_from_rng(&mut rand::rngs::SysRng).map_err(|error| Error::from_error(error, ErrorKind::UnexpectedError))?;
        let tsid_provider = Arc::new(Mutex::new(TsidProviderImpl::new(ClockUtc, rng, TSID_RANDOM_BYTE_COUNT)));
        let storage = Arc::new(StorageService::new(
            Arc::new(LocalStorageDriver::with_budget(tsid_provider.clone(), budget.clone())),
            archive,
        ));
        let snapshots = ListingSnapshots::open(conf.listing.clone(), clock.clone(), &state_dir.join("listing"), budget).await;
        let library = LibraryBrowser::new(&conf.libraries, storage.clone(), conf.storage.archive.max_nested_depth, snapshots.clone());
        let db = DatabaseBootstrap::connect(&state_dir).await?;
        let rating_tags = Arc::new(RatingTagService::new(db.clone()).await?);
        let timeline = TimelineService::open(
            &conf.timeline,
            &conf.libraries,
            storage.clone(),
            MediaTools::new(conf.media.clone(), conf.thumbnail.video_timeout),
            &Self::timeline_dir(&state_dir),
            clock.clone(),
        )
        .await?;
        let mutation = Arc::new(LibraryMutation::new(
            &conf.libraries,
            storage.clone(),
            conf.storage.archive.max_nested_depth,
            snapshots,
            timeline.clone(),
            rating_tags.clone(),
        ));
        let operations = OperationService::new(mutation.clone(), tsid_provider.clone());
        let uploads = UploadService::new(mutation.clone(), tsid_provider.clone());
        let media_info = Arc::new(MediaInfoService::new(conf.media.clone(), conf.thumbnail.video_timeout).with_budget(storage.work_budget().clone()));
        let thumbnails = ThumbnailGenerator::open_with_media(
            storage,
            &Self::thumbnail_cache_dir(&state_dir),
            conf.thumbnail.clone(),
            conf.media.clone(),
            media_info.clone(),
            pools.clone(),
        )
        .await?;
        let videos = VideoService::open(
            mutation.clone(),
            media_info.clone(),
            tsid_provider.clone(),
            &state_dir.join("video"),
            conf.video.clone(),
            &conf.media,
            pools.process_video.clone(),
        )
        .await?;

        let preferences = PreferencesService::new(db.clone()).await?;
        let auth = AuthService::new(db, clock, tsid_provider).await?;

        Ok(Self {
            conf,
            library,
            auth,
            preferences,
            rating_tags,
            thumbnails,
            mutation,
            uploads,
            operations,
            media_info,
            pools,
            videos,
            timeline,
            image_sockets: ImageSockets::new(ImageSocketLimits::default()),
        })
    }

    fn archive_cache_dir(state_dir: &std::path::Path) -> PathBuf {
        state_dir.join("storage").join("archive")
    }

    fn timeline_dir(state_dir: &std::path::Path) -> PathBuf {
        state_dir.join("timeline")
    }

    fn thumbnail_cache_dir(state_dir: &std::path::Path) -> PathBuf {
        state_dir.join("thumbnail")
    }
}

#[cfg(test)]
mod tests {
    use testresult::TestResult;

    use super::*;
    use crate::config::{CoreConfig, LoggingConfig, StorageConfig};

    fn config(state_dir: PathBuf) -> DaemonConfig {
        DaemonConfig {
            core: CoreConfig {
                state_dir,
                listen_addr: "127.0.0.1:0".to_string(),
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                json: false,
                file: None,
                performance: false,
            },
            libraries: Vec::new(),
            storage: StorageConfig { archive: Default::default() },
            listing: Default::default(),
            timeline: Default::default(),
            thumbnail: Default::default(),
            scheduler: Default::default(),
            media: Default::default(),
            video: Default::default(),
            web: Default::default(),
        }
    }

    #[tokio::test]
    async fn opens_archive_cache_under_state_dir_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let staging = DaemonState::archive_cache_dir(dir.path()).join("staging");
        std::fs::create_dir_all(&staging)?;
        std::fs::write(staging.join("leftover"), b"partial")?;

        let state = DaemonState::new(config(dir.path().to_path_buf())).await?;
        assert!(DaemonState::archive_cache_dir(dir.path()).join("cache.sqlite3").exists());
        assert_eq!(std::fs::read_dir(&staging)?.count(), 0, "起動時に staging を空にする");
        drop(state);
        Ok(())
    }

    #[tokio::test]
    async fn opens_thumbnail_cache_under_state_dir_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let _state = DaemonState::new(config(dir.path().to_path_buf())).await?;
        assert!(DaemonState::thumbnail_cache_dir(dir.path()).join("cache.sqlite3").exists());
        Ok(())
    }

    #[tokio::test]
    async fn connects_sqlite_under_state_dir_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let _state = DaemonState::new(config(dir.path().to_path_buf())).await?;
        assert!(dir.path().join("sqlite.db").exists());
        Ok(())
    }
}
