use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use chrono::{DateTime, Utc};
use omnius_core_base::clock::Clock;
use serde::Deserialize;

use omnius_pxna_engine::service::{ArchiveIdentity, MediaTools, SourceFingerprint, StorageService};

use crate::{config::LibraryConfig, prelude::*};

mod date;
mod index;
mod rules;
mod scanner;

pub use date::*;
pub use index::*;
pub use rules::*;
pub use scanner::*;

/// 定期の集計の時機を確かめる間隔。
const SCHEDULE_TICK: Duration = Duration::from_secs(60);

/// タイムラインのタイムゾーン、集計の間隔と同時実行数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TimelineConfig {
    /// 更新日時と `creation_time` を直す先の IANA の名前。省略時は `TZ` か OS の設定、どちらもなければ UTC にする。
    pub time_zone: Option<String>,
    /// 最後に全体の集計を終えてから次の集計までの秒数。0 なら定期の集計を持たない。
    pub scan_interval_secs: u64,
    pub max_concurrent_extractions: usize,
    pub max_concurrent_probes: usize,
    /// Rules that read a timeline date from the file name. They are tried in order.
    #[serde(default)]
    pub date_rules: Vec<DateRuleConfig>,
}

impl Default for TimelineConfig {
    fn default() -> Self {
        Self {
            time_zone: None,
            scan_interval_secs: 86_400,
            max_concurrent_extractions: 1,
            max_concurrent_probes: 1,
            date_rules: Vec::new(),
        }
    }
}

impl TimelineConfig {
    pub fn validate(&self, libraries: &[LibraryConfig]) -> Result<()> {
        if self.max_concurrent_extractions == 0 || self.max_concurrent_probes == 0 {
            return Err(Error::new(ErrorKind::InvalidFormat).with_message("[timeline] concurrency must be positive"));
        }
        TimelineZone::new(self.time_zone.as_deref()).map_err(|error| Error::from_error(error, ErrorKind::InvalidFormat).with_message("invalid [timeline] time_zone"))?;
        DateRules::new(&self.date_rules, libraries)?;
        Ok(())
    }
}

/// Library の集計の状態。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineStatus {
    pub library_id: String,
    pub running: bool,
    pub processed_files: u64,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Library ごとのタイムラインの索引を集計し、Library を横断したページと月ごとの件数、Library ごとの状態を返す。
pub struct TimelineService {
    libraries: Vec<LibraryConfig>,
    index: Arc<TimelineIndex>,
    scanner: Arc<TimelineScanner>,
    interval: Option<Duration>,
    clock: Arc<dyn Clock<Utc> + Send + Sync>,
    /// 全体の集計を実行中の Library と、その処理したファイルの数。
    running: Mutex<HashMap<String, Arc<AtomicU64>>>,
}

impl TimelineService {
    pub async fn open(
        config: &TimelineConfig,
        libraries: &[LibraryConfig],
        storage: Arc<StorageService>,
        tools: MediaTools,
        dir: &Path,
        clock: Arc<dyn Clock<Utc> + Send + Sync>,
    ) -> Result<Arc<Self>> {
        let zone = TimelineZone::new(config.time_zone.as_deref())?;
        let rules = DateRules::new(&config.date_rules, libraries)?;
        let index = Arc::new(TimelineIndex::open(dir).await?);
        index.reset_for_date_rules(rules.fingerprint(), !config.date_rules.is_empty()).await?;
        index.rezone(&zone).await?;
        let scanner = Arc::new(TimelineScanner::new(
            storage,
            index.clone(),
            tools,
            zone,
            rules,
            config.max_concurrent_extractions,
            config.max_concurrent_probes,
        ));
        Ok(Arc::new(Self {
            libraries: libraries.to_vec(),
            index,
            scanner,
            interval: (config.scan_interval_secs > 0).then(|| Duration::from_secs(config.scan_interval_secs)),
            clock,
            running: Mutex::new(HashMap::new()),
        }))
    }

    /// 起動時の集計と、定期の集計を始める task を作る。
    ///
    /// 起動時は、索引にファイルの行を持たない Library を集計する。以後は、最後に集計を終えてから間隔が過ぎた Library を集計する。
    pub fn start(self: &Arc<Self>) {
        let weak = Arc::downgrade(self);
        tokio::spawn(async move {
            if let Some(service) = weak.upgrade() {
                service
                    .start_where(|service, library| async move { Ok(!service.index.has_files(&library.id).await?) })
                    .await;
            }
            let mut ticks = tokio::time::interval(SCHEDULE_TICK);
            loop {
                ticks.tick().await;
                let Some(service) = weak.upgrade() else {
                    break;
                };
                service.start_where(|service, library| async move { service.is_due(&library).await }).await;
            }
        });
    }

    async fn start_where<F, Fut>(self: &Arc<Self>, condition: F)
    where
        F: Fn(Arc<Self>, LibraryConfig) -> Fut,
        Fut: Future<Output = Result<bool>>,
    {
        for library in self.libraries.clone() {
            match condition(self.clone(), library.clone()).await {
                Ok(true) => {
                    self.start_scan(&library);
                }
                Ok(false) => {}
                Err(error) => warn!(library = %library.id, ?error, "timeline: cannot check the scan schedule"),
            }
        }
    }

    async fn is_due(&self, library: &LibraryConfig) -> Result<bool> {
        let Some(interval) = self.interval else {
            return Ok(false);
        };
        Ok(match self.index.completed_at(&library.id).await? {
            None => true,
            Some(completed) => (self.clock.now() - completed).to_std().is_ok_and(|elapsed| elapsed >= interval),
        })
    }

    /// 全体の集計を始める。同じ Library の全体の集計が実行中なら何もしない。
    fn start_scan(self: &Arc<Self>, library: &LibraryConfig) {
        let progress = {
            let mut running = self.running.lock();
            if running.contains_key(&library.id) {
                return;
            }
            let progress = Arc::new(AtomicU64::new(0));
            running.insert(library.id.clone(), progress.clone());
            progress
        };
        let service = self.clone();
        let library = library.clone();
        tokio::spawn(async move {
            let result = service.full_scan(&library, &progress).await;
            if let Err(error) = result {
                warn!(library = %library.id, ?error, "timeline: scan failed");
            }
            service.running.lock().remove(&library.id);
        });
    }

    async fn full_scan(&self, library: &LibraryConfig, progress: &AtomicU64) -> Result<()> {
        let generation = self.index.begin_scan(&library.id).await?;
        info!(library = %library.id, generation, "timeline: scan started");
        let target = TimelineScan {
            library,
            scope: "/",
            generation,
            scoped: false,
            progress,
        };
        self.scanner.scan(&target).await?;
        self.index.complete_scan(&library.id, self.clock.now()).await?;
        info!(library = %library.id, files = progress.load(Ordering::Relaxed), "timeline: scan completed");
        Ok(())
    }

    /// web から全体の集計を始め、今の状態を返す。
    pub async fn refresh(self: &Arc<Self>, library_id: &str) -> Result<TimelineStatus> {
        let library = self.library(library_id)?.clone();
        self.start_scan(&library);
        self.status(library_id).await
    }

    pub async fn status(&self, library_id: &str) -> Result<TimelineStatus> {
        let library = self.library(library_id)?;
        let progress = self.running.lock().get(&library.id).cloned();
        Ok(TimelineStatus {
            library_id: library.id.clone(),
            running: progress.is_some(),
            processed_files: progress.map(|progress| progress.load(Ordering::Relaxed)).unwrap_or(0),
            completed_at: self.index.completed_at(&library.id).await?,
        })
    }

    /// `library_ids` の Entry を 1 本の並びにしたページ。どれかの Library がなければ `NotFound` にする。
    pub async fn page(&self, library_ids: &[String], anchor: &TimelineAnchor, limit: usize) -> Result<TimelinePage> {
        self.libraries(library_ids)?;
        self.index.page(library_ids, anchor, limit).await
    }

    /// `library_ids` の月ごとの件数の和。どれかの Library がなければ `NotFound` にする。
    pub async fn months(&self, library_ids: &[String]) -> Result<Vec<(String, u64)>> {
        self.libraries(library_ids)?;
        self.index.months(library_ids).await
    }

    /// `library_ids` の Library をすべて返す。1 つでもなければ `NotFound` にする。
    pub fn libraries(&self, library_ids: &[String]) -> Result<Vec<&LibraryConfig>> {
        library_ids.iter().map(|library_id| self.library(library_id)).collect()
    }

    pub fn library(&self, library_id: &str) -> Result<&LibraryConfig> {
        self.libraries
            .iter()
            .find(|library| library.id == library_id)
            .ok_or_else(|| Error::new(ErrorKind::NotFound).with_message("library not found"))
    }

    // ---- daemon 自身の変更 ----

    /// 変更した Entry の Library 相対パスを受け取る。もう存在しない path の行を消し、存在する path の範囲の集計を背景で始める。
    ///
    /// 行を消し終えてから戻るので、呼び出し側が応答を返した時点で、消えた Entry はページに現れない。
    pub async fn changed(self: &Arc<Self>, changes: &HashMap<String, Vec<String>>) {
        for (library_id, paths) in changes {
            let Ok(library) = self.library(library_id).cloned() else {
                continue;
            };
            // 消えた path と、上書きされたファイルの行は、応答の前に消す。親フォルダの陰になる子も、1 つずつ調べる。
            let mut existing = Vec::new();
            let mut stale = Vec::new();
            let mut sorted = paths.clone();
            sorted.sort();
            sorted.dedup();
            for path in sorted {
                let present = tokio::fs::symlink_metadata(Self::local(&library, &path)).await.is_ok();
                // 上書きされて中身が変わったファイルの行は消す。新しい中身の行は、範囲の集計が足す。
                if !present || self.scanner.is_replaced(&library, &path).await.unwrap_or(false) {
                    stale.push(path.clone());
                }
                if present {
                    existing.push(path);
                }
            }
            if !stale.is_empty() {
                let lock = self.scanner.lock(&library.id);
                let _guard = lock.lock().await;
                for path in &stale {
                    if let Err(error) = self.index.remove(&library.id, path).await {
                        warn!(library = %library.id, path, ?error, "timeline: cannot remove changed entries");
                    }
                }
            }
            // 範囲の集計は、ほかの path の配下にある path をまとめて、外側の path だけで行う。
            let existing = Self::outermost(&existing);
            if existing.is_empty() {
                continue;
            }
            let service = self.clone();
            tokio::spawn(async move {
                for path in existing {
                    if let Err(error) = service.scoped_scan(&library, &path).await {
                        warn!(library = %library.id, path, ?error, "timeline: cannot scan changed entries");
                    }
                }
            });
        }
    }

    /// 同じ Library の中の名前の変更と移動を、行の付け替えで反映する。並びの日時は読み直さない。
    pub async fn renamed(&self, library_id: &str, src: &str, dst: &str) -> Result<()> {
        let library = self.library(library_id)?;
        let lock = self.scanner.lock(&library.id);
        let _guard = lock.lock().await;
        for archive in self.index.rename(&library.id, src, dst).await? {
            let local = Self::local(library, &archive);
            let Ok(fingerprint) = SourceFingerprint::read(&local).await else {
                continue;
            };
            let identity = ArchiveIdentity::from_local(&local.to_string_lossy(), &fingerprint).to_hex();
            self.index.set_archive_identity(&library.id, &archive, &identity).await?;
        }
        Ok(())
    }

    async fn scoped_scan(&self, library: &LibraryConfig, path: &str) -> Result<()> {
        let progress = AtomicU64::new(0);
        let target = TimelineScan {
            library,
            scope: path,
            generation: self.index.generation(&library.id).await?,
            scoped: true,
            progress: &progress,
        };
        self.scanner.scan(&target).await
    }

    /// ほかの path の配下にある path を除く。
    fn outermost(paths: &[String]) -> Vec<String> {
        let mut sorted = paths.to_vec();
        sorted.sort();
        sorted.dedup();
        let mut result: Vec<String> = Vec::new();
        for path in sorted {
            if result.iter().any(|parent| parent == "/" || path.starts_with(&format!("{parent}/"))) {
                continue;
            }
            result.push(path);
        }
        result
    }

    fn local(library: &LibraryConfig, relative: &str) -> PathBuf {
        let mut path = library.path.clone();
        for segment in relative.split('/').filter(|segment| !segment.is_empty()) {
            path.push(segment);
        }
        path
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::Cursor,
        path::{Path, PathBuf},
        time::{Duration as StdDuration, SystemTime},
    };

    use omnius_core_base::{clock::ClockUtc, tsid::TsidProviderImpl};
    use omnius_pxna_engine::service::{ArchiveRuntime, LocalStorageDriver, MediaOptions};
    use rand::{SeedableRng as _, rngs::ChaCha20Rng};
    use sevenz_rust2::{ArchiveEntry as SevenZipEntry, ArchiveWriter, SourceReader};
    use testresult::TestResult;

    use super::*;

    const LIBRARY: &str = "lib";

    struct Fixture {
        _dir: tempfile::TempDir,
        root: PathBuf,
        state: PathBuf,
    }

    impl Fixture {
        fn new() -> TestResult<Self> {
            let dir = tempfile::tempdir()?;
            let root = dir.path().join("library");
            let state = dir.path().join("state");
            std::fs::create_dir_all(&root)?;
            Ok(Self { _dir: dir, root, state })
        }

        fn library(&self) -> LibraryConfig {
            LibraryConfig {
                id: LIBRARY.to_string(),
                name: "Lib".to_string(),
                path: self.root.clone(),
                readonly: false,
            }
        }

        async fn service(&self, zone: &str, interval_secs: u64) -> TestResult<Arc<TimelineService>> {
            self.service_with_rules(zone, interval_secs, Vec::new()).await
        }

        async fn service_with_rules(&self, zone: &str, interval_secs: u64, date_rules: Vec<DateRuleConfig>) -> TestResult<Arc<TimelineService>> {
            let rng = ChaCha20Rng::try_from_rng(&mut rand::rngs::SysRng)?;
            let tsid = Arc::new(Mutex::new(TsidProviderImpl::new(omnius_core_base::clock::ClockUtc, rng, 16)));
            let storage = Arc::new(StorageService::new(Arc::new(LocalStorageDriver::new(tsid)), Arc::new(ArchiveRuntime::default())));
            let config = TimelineConfig {
                time_zone: Some(zone.to_string()),
                scan_interval_secs: interval_secs,
                date_rules,
                ..TimelineConfig::default()
            };
            Ok(TimelineService::open(
                &config,
                &[self.library()],
                storage,
                MediaTools::new(MediaOptions::default(), StdDuration::from_secs(10)),
                &self.state,
                Arc::new(ClockUtc),
            )
            .await?)
        }

        fn path(&self, relative: &str) -> PathBuf {
            self.root.join(relative)
        }

        /// Simulate an index written before date rules existed.
        async fn forget_date_rules(&self) -> TestResult {
            let options = sqlx::sqlite::SqliteConnectOptions::new().filename(self.state.join("index.db"));
            let pool = sqlx::SqlitePool::connect_with(options).await?;
            sqlx::query("DELETE FROM timeline_settings WHERE key = 'date_rules'").execute(&pool).await?;
            pool.close().await;
            Ok(())
        }
    }

    /// Exif の撮影日時だけを持つ JPEG。
    fn exif_jpeg(taken: &str) -> Vec<u8> {
        let mut jpeg = Vec::new();
        image::RgbImage::new(8, 8).write_to(&mut Cursor::new(&mut jpeg), image::ImageFormat::Jpeg).unwrap();
        let mut date = taken.as_bytes().to_vec();
        date.push(0);
        let mut tiff = vec![0x49, 0x49, 0x2a, 0x00, 8, 0, 0, 0];
        // IFD0: Exif IFD への pointer が 1 つ。Exif IFD は 26 byte 目から始まる。
        tiff.extend_from_slice(&1u16.to_le_bytes());
        tiff.extend_from_slice(&0x8769u16.to_le_bytes());
        tiff.extend_from_slice(&4u16.to_le_bytes());
        tiff.extend_from_slice(&1u32.to_le_bytes());
        tiff.extend_from_slice(&26u32.to_le_bytes());
        tiff.extend_from_slice(&[0; 4]);
        // Exif IFD: DateTimeOriginal が 1 つ。値は IFD の直後（44 byte 目）に置く。
        tiff.extend_from_slice(&1u16.to_le_bytes());
        tiff.extend_from_slice(&0x9003u16.to_le_bytes());
        tiff.extend_from_slice(&2u16.to_le_bytes());
        tiff.extend_from_slice(&(date.len() as u32).to_le_bytes());
        tiff.extend_from_slice(&44u32.to_le_bytes());
        tiff.extend_from_slice(&[0; 4]);
        tiff.extend_from_slice(&date);

        let mut app1 = b"Exif\0\0".to_vec();
        app1.extend_from_slice(&tiff);
        let mut output = jpeg[..2].to_vec();
        output.extend_from_slice(&[0xff, 0xe1]);
        output.extend_from_slice(&((app1.len() + 2) as u16).to_be_bytes());
        output.extend_from_slice(&app1);
        output.extend_from_slice(&jpeg[2..]);
        output
    }

    fn plain_png() -> Vec<u8> {
        let mut png = Vec::new();
        image::RgbaImage::new(4, 4).write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
        png
    }

    fn write_7z(path: &Path, entries: &[(&str, Vec<u8>)]) -> TestResult {
        let mut writer = ArchiveWriter::create(path)?;
        writer.push_archive_entries(
            entries.iter().map(|(name, _)| SevenZipEntry::new_file(name)).collect(),
            entries.iter().map(|(_, bytes)| SourceReader::new(Cursor::new(bytes.clone()))).collect(),
        )?;
        writer.finish()?;
        Ok(())
    }

    /// 更新時刻を UNIX 時刻の秒で設定する。
    fn set_modified(path: &Path, secs: u64) -> TestResult {
        let file = std::fs::OpenOptions::new().write(true).open(path)?;
        file.set_modified(SystemTime::UNIX_EPOCH + StdDuration::from_secs(secs))?;
        Ok(())
    }

    /// 2024-01-02T03:04:05Z。
    const T_2024_01_02: u64 = 1_704_164_645;
    /// 2025-01-02T03:04:05Z。
    const T_2025_01_02: u64 = 1_735_787_045;
    /// 2023-07-07T00:00:00Z。
    const T_2023_07_07: u64 = 1_688_688_000;

    async fn wait_scan(service: &Arc<TimelineService>) -> TestResult {
        service.refresh(LIBRARY).await?;
        wait_idle(service).await
    }

    async fn wait_idle(service: &Arc<TimelineService>) -> TestResult {
        for _ in 0..600 {
            let status = service.status(LIBRARY).await?;
            if !status.running && status.completed_at.is_some() {
                return Ok(());
            }
            tokio::time::sleep(StdDuration::from_millis(50)).await;
        }
        Err("scan did not finish".into())
    }

    async fn wait_until<F, Fut>(mut condition: F) -> TestResult
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = TestResult<bool>>,
    {
        for _ in 0..200 {
            if condition().await? {
                return Ok(());
            }
            tokio::time::sleep(StdDuration::from_millis(50)).await;
        }
        Err("condition was not met".into())
    }

    async fn listed(service: &Arc<TimelineService>) -> TestResult<Vec<(String, String, TimelineDateSource)>> {
        let page = service.page(&[LIBRARY.to_string()], &TimelineAnchor::Newest, 100).await?;
        Ok(page
            .rows
            .iter()
            .map(|listed| (listed.row.path.clone(), listed.row.date.text(), listed.row.date.source))
            .collect())
    }

    async fn paths(service: &Arc<TimelineService>) -> TestResult<Vec<String>> {
        Ok(listed(service).await?.into_iter().map(|(path, _, _)| path).collect())
    }

    fn changes(paths: &[&str]) -> HashMap<String, Vec<String>> {
        HashMap::from([(LIBRARY.to_string(), paths.iter().map(|path| path.to_string()).collect())])
    }

    #[tokio::test]
    async fn scan_orders_entries_by_date_and_counts_months_test() -> TestResult {
        let fixture = Fixture::new()?;
        std::fs::write(fixture.path("a.jpg"), exif_jpeg("2024:05:01 12:34:56"))?;
        std::fs::write(fixture.path("b.png"), plain_png())?;
        set_modified(&fixture.path("b.png"), T_2025_01_02)?;
        write_7z(&fixture.path("c.7z"), &[("x.jpg", exif_jpeg("2024:06:01 00:00:00")), ("y.png", plain_png())])?;
        set_modified(&fixture.path("c.7z"), T_2023_07_07)?;
        std::fs::create_dir(fixture.path(".hidden"))?;
        std::fs::write(fixture.path(".hidden/h.jpg"), plain_png())?;
        std::fs::write(fixture.path("notes.txt"), b"text")?;

        let service = fixture.service("UTC", 0).await?;
        wait_scan(&service).await?;

        assert_eq!(
            listed(&service).await?,
            [
                ("/b.png".to_string(), "2025-01-02T03:04:05".to_string(), TimelineDateSource::Modified),
                ("/c.7z/x.jpg".to_string(), "2024-06-01T00:00:00".to_string(), TimelineDateSource::Exif),
                ("/a.jpg".to_string(), "2024-05-01T12:34:56".to_string(), TimelineDateSource::Exif),
                ("/c.7z/y.png".to_string(), "2023-07-07T00:00:00".to_string(), TimelineDateSource::ArchiveModified),
            ]
        );
        let months = service.months(&[LIBRARY.to_string()]).await?;
        assert_eq!(
            months,
            [
                ("2025-01".to_string(), 1),
                ("2024-06".to_string(), 1),
                ("2024-05".to_string(), 1),
                ("2023-07".to_string(), 1)
            ]
        );
        let total = service.page(&[LIBRARY.to_string()], &TimelineAnchor::Newest, 100).await?.rows.len() as u64;
        assert_eq!(months.iter().map(|(_, count)| count).sum::<u64>(), total);
        Ok(())
    }

    #[tokio::test]
    async fn date_rules_read_names_before_exif_and_archives_test() -> TestResult {
        let fixture = Fixture::new()?;
        std::fs::write(fixture.path("IMG-20240102_030405.jpg"), exif_jpeg("2024:05:01 12:34:56"))?;
        write_7z(
            &fixture.path("c.7z"),
            &[("IMG-20200304_050607.jpg", exif_jpeg("2024:06:01 00:00:00")), ("x.jpg", exif_jpeg("2024:07:01 00:00:00"))],
        )?;
        let service = fixture
            .service_with_rules(
                "UTC",
                0,
                vec![DateRuleConfig {
                    library: None,
                    path: "**/IMG-*".to_string(),
                    name: r"IMG-(?P<date>\d{8}_\d{6})".to_string(),
                    format: "%Y%m%d_%H%M%S".to_string(),
                    time_zone: None,
                }],
            )
            .await?;
        wait_scan(&service).await?;

        assert_eq!(
            listed(&service).await?,
            [
                ("/c.7z/x.jpg".to_string(), "2024-07-01T00:00:00".to_string(), TimelineDateSource::Exif),
                ("/IMG-20240102_030405.jpg".to_string(), "2024-01-02T03:04:05".to_string(), TimelineDateSource::FileName),
                ("/c.7z/IMG-20200304_050607.jpg".to_string(), "2020-03-04T05:06:07".to_string(), TimelineDateSource::FileName),
            ]
        );
        Ok(())
    }

    #[tokio::test]
    async fn date_rule_time_zone_is_shifted_into_the_timeline_zone_test() -> TestResult {
        let fixture = Fixture::new()?;
        std::fs::write(fixture.path("IMG-20240501120000.png"), plain_png())?;
        let service = fixture
            .service_with_rules(
                "UTC",
                0,
                vec![DateRuleConfig {
                    library: None,
                    path: "**".to_string(),
                    name: r"IMG-(?P<date>\d{14})".to_string(),
                    format: "%Y%m%d%H%M%S".to_string(),
                    time_zone: Some("Asia/Tokyo".to_string()),
                }],
            )
            .await?;
        wait_scan(&service).await?;

        assert_eq!(
            listed(&service).await?,
            [("/IMG-20240501120000.png".to_string(), "2024-05-01T03:00:00".to_string(), TimelineDateSource::FileName)]
        );
        Ok(())
    }

    #[tokio::test]
    async fn changing_date_rules_rereads_dates_test() -> TestResult {
        let fixture = Fixture::new()?;
        std::fs::write(fixture.path("IMG-20240102_030405.jpg"), exif_jpeg("2024:05:01 12:34:56"))?;
        let rule = DateRuleConfig {
            library: None,
            path: "**".to_string(),
            name: r"IMG-(?P<date>\d{8}_\d{6})".to_string(),
            format: "%Y%m%d_%H%M%S".to_string(),
            time_zone: None,
        };
        let service = fixture.service_with_rules("UTC", 0, vec![rule]).await?;
        wait_scan(&service).await?;
        assert_eq!(listed(&service).await?[0].2, TimelineDateSource::FileName);

        let service = fixture.service_with_rules("UTC", 0, Vec::new()).await?;
        wait_scan(&service).await?;
        assert_eq!(
            listed(&service).await?,
            [("/IMG-20240102_030405.jpg".to_string(), "2024-05-01T12:34:56".to_string(), TimelineDateSource::Exif)],
            "a rule change rereads even when the file version is the same"
        );
        Ok(())
    }

    #[tokio::test]
    async fn configuring_date_rules_for_an_old_index_rereads_dates_test() -> TestResult {
        let fixture = Fixture::new()?;
        std::fs::write(fixture.path("IMG-20240102_030405.jpg"), exif_jpeg("2024:05:01 12:34:56"))?;

        let service = fixture.service("UTC", 0).await?;
        wait_scan(&service).await?;
        assert_eq!(listed(&service).await?[0].2, TimelineDateSource::Exif);

        fixture.forget_date_rules().await?;
        let service = fixture
            .service_with_rules(
                "UTC",
                0,
                vec![DateRuleConfig {
                    library: None,
                    path: "**".to_string(),
                    name: r"IMG-(?P<date>\d{8}_\d{6})".to_string(),
                    format: "%Y%m%d_%H%M%S".to_string(),
                    time_zone: None,
                }],
            )
            .await?;
        wait_scan(&service).await?;
        assert_eq!(
            listed(&service).await?,
            [("/IMG-20240102_030405.jpg".to_string(), "2024-01-02T03:04:05".to_string(), TimelineDateSource::FileName)],
            "an old index without a recorded rule list is reread once rules are configured"
        );
        Ok(())
    }

    #[tokio::test]
    async fn unchanged_files_are_not_read_again_test() -> TestResult {
        let fixture = Fixture::new()?;
        let path = fixture.path("a.jpg");
        std::fs::write(&path, exif_jpeg("2024:05:01 12:34:56"))?;
        set_modified(&path, T_2024_01_02)?;
        let service = fixture.service("UTC", 0).await?;
        wait_scan(&service).await?;
        assert_eq!(listed(&service).await?[0].1, "2024-05-01T12:34:56");

        // 同じ大きさと更新時刻のまま中身を変える。読み直さなければ、古い日時が残る。
        std::fs::write(&path, exif_jpeg("2020:01:01 00:00:00"))?;
        set_modified(&path, T_2024_01_02)?;
        wait_scan(&service).await?;
        assert_eq!(listed(&service).await?[0].1, "2024-05-01T12:34:56", "変わっていないファイルは読み直さない");

        set_modified(&path, T_2025_01_02)?;
        wait_scan(&service).await?;
        assert_eq!(listed(&service).await?[0].1, "2020-01-01T00:00:00", "更新時刻が変われば読み直す");
        Ok(())
    }

    #[tokio::test]
    async fn removing_an_image_from_an_archive_drops_its_entry_test() -> TestResult {
        let fixture = Fixture::new()?;
        let archive = fixture.path("c.7z");
        write_7z(&archive, &[("a.jpg", exif_jpeg("2024:01:01 00:00:00")), ("b.jpg", exif_jpeg("2024:02:01 00:00:00"))])?;
        let service = fixture.service("UTC", 0).await?;
        wait_scan(&service).await?;
        assert_eq!(paths(&service).await?, ["/c.7z/b.jpg", "/c.7z/a.jpg"]);

        write_7z(&archive, &[("a.jpg", exif_jpeg("2024:01:01 00:00:00"))])?;
        wait_scan(&service).await?;
        assert_eq!(paths(&service).await?, ["/c.7z/a.jpg"]);
        assert_eq!(service.months(&[LIBRARY.to_string()]).await?, [("2024-01".to_string(), 1)]);
        Ok(())
    }

    #[tokio::test]
    async fn a_large_archive_is_published_in_parts_and_removed_with_its_file_test() -> TestResult {
        let fixture = Fixture::new()?;
        let names = (0..450).map(|i| format!("p{i:03}.jpg")).collect::<Vec<_>>();
        let entries = names.iter().map(|name| (name.as_str(), exif_jpeg("2024:03:03 03:03:03"))).collect::<Vec<_>>();
        write_7z(&fixture.path("big.7z"), &entries)?;
        let service = fixture.service("UTC", 0).await?;
        wait_scan(&service).await?;
        assert_eq!(service.months(&[LIBRARY.to_string()]).await?, [("2024-03".to_string(), 450)]);
        let page = service.page(&[LIBRARY.to_string()], &TimelineAnchor::Newest, 1000).await?;
        assert_eq!(page.rows.len(), 450);

        std::fs::remove_file(fixture.path("big.7z"))?;
        wait_scan(&service).await?;
        assert!(paths(&service).await?.is_empty());
        assert!(service.months(&[LIBRARY.to_string()]).await?.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn nested_archives_are_read_test() -> TestResult {
        let fixture = Fixture::new()?;
        let inner = fixture.path("inner.7z");
        write_7z(&inner, &[("deep.jpg", exif_jpeg("2022:02:02 02:02:02"))])?;
        let inner_bytes = std::fs::read(&inner)?;
        std::fs::remove_file(&inner)?;
        write_7z(&fixture.path("outer.7z"), &[("inner.7z", inner_bytes)])?;
        let service = fixture.service("UTC", 0).await?;
        wait_scan(&service).await?;
        assert_eq!(
            listed(&service).await?,
            [("/outer.7z/inner.7z/deep.jpg".to_string(), "2022-02-02T02:02:02".to_string(), TimelineDateSource::Exif)]
        );
        Ok(())
    }

    #[tokio::test]
    async fn changing_the_time_zone_rewrites_dates_without_reading_files_test() -> TestResult {
        let fixture = Fixture::new()?;
        let path = fixture.path("a.png");
        std::fs::write(&path, plain_png())?;
        set_modified(&path, T_2024_01_02)?;
        {
            let service = fixture.service("UTC", 0).await?;
            wait_scan(&service).await?;
            assert_eq!(listed(&service).await?[0].1, "2024-01-02T03:04:05");
        }
        let service = fixture.service("Asia/Tokyo", 0).await?;
        assert_eq!(listed(&service).await?[0].1, "2024-01-02T12:04:05", "UTC の更新日時を 9 時間進める");
        assert_eq!(service.months(&[LIBRARY.to_string()]).await?, [("2024-01".to_string(), 1)]);
        Ok(())
    }

    #[tokio::test]
    async fn deleted_entries_disappear_before_the_response_test() -> TestResult {
        let fixture = Fixture::new()?;
        std::fs::write(fixture.path("a.png"), plain_png())?;
        std::fs::write(fixture.path("b.png"), plain_png())?;
        set_modified(&fixture.path("a.png"), T_2024_01_02)?;
        set_modified(&fixture.path("b.png"), T_2025_01_02)?;
        let service = fixture.service("UTC", 0).await?;
        wait_scan(&service).await?;

        std::fs::remove_file(fixture.path("b.png"))?;
        service.changed(&changes(&["/b.png"])).await;
        assert_eq!(paths(&service).await?, ["/a.png"]);
        assert_eq!(service.months(&[LIBRARY.to_string()]).await?, [("2024-01".to_string(), 1)]);
        Ok(())
    }

    #[tokio::test]
    async fn overwritten_files_lose_their_old_entries_before_the_response_test() -> TestResult {
        let fixture = Fixture::new()?;
        write_7z(&fixture.path("c.7z"), &[("old.jpg", exif_jpeg("2024:01:01 00:00:00"))])?;
        std::fs::write(fixture.path("p.png"), plain_png())?;
        set_modified(&fixture.path("p.png"), T_2024_01_02)?;
        let service = fixture.service("UTC", 0).await?;
        wait_scan(&service).await?;
        assert_eq!(paths(&service).await?.len(), 2);

        write_7z(&fixture.path("c.7z"), &[("new.jpg", exif_jpeg("2025:01:01 00:00:00"))])?;
        std::fs::write(fixture.path("p.png"), [plain_png(), vec![0]].concat())?;
        service.changed(&changes(&["/c.7z", "/p.png"])).await;
        let right_away = paths(&service).await?;
        assert!(!right_away.contains(&"/c.7z/old.jpg".to_string()), "上書きされた archive の旧 Entry は応答の前に消える");
        assert!(!right_away.contains(&"/p.png".to_string()), "上書きされた画像の旧い行も消える");
        wait_until(|| async { Ok(paths(&service).await? == ["/c.7z/new.jpg", "/p.png"] || paths(&service).await? == ["/p.png", "/c.7z/new.jpg"]) }).await?;
        assert_eq!(service.months(&[LIBRARY.to_string()]).await?.iter().map(|(_, count)| count).sum::<u64>(), 2);
        Ok(())
    }

    #[tokio::test]
    async fn renamed_entries_keep_their_dates_and_are_reevaluated_test() -> TestResult {
        let fixture = Fixture::new()?;
        std::fs::create_dir(fixture.path("dir"))?;
        std::fs::write(fixture.path("dir/photo.png"), plain_png())?;
        std::fs::write(fixture.path("keep.png"), plain_png())?;
        set_modified(&fixture.path("dir/photo.png"), T_2024_01_02)?;
        set_modified(&fixture.path("keep.png"), T_2025_01_02)?;
        let service = fixture.service("UTC", 0).await?;
        wait_scan(&service).await?;

        // 同じ種類の名前への変更は、日時を保ったまま新しい path で返る。
        std::fs::rename(fixture.path("dir/photo.png"), fixture.path("dir/renamed.png"))?;
        service.renamed(LIBRARY, "/dir/photo.png", "/dir/renamed.png").await?;
        service.changed(&changes(&["/dir/photo.png", "/dir/renamed.png"])).await;
        assert_eq!(
            listed(&service).await?[1],
            ("/dir/renamed.png".to_string(), "2024-01-02T03:04:05".to_string(), TimelineDateSource::Modified)
        );

        // 対象外の拡張子、隠し項目、隠しフォルダの中へ変えると、範囲の集計の後に消える。
        for (to, relative) in [("dir/renamed.txt", "/dir/renamed.txt"), ("dir/.renamed.png", "/dir/.renamed.png")] {
            std::fs::rename(fixture.path("dir/renamed.png"), fixture.path(to))?;
            service.renamed(LIBRARY, "/dir/renamed.png", relative).await?;
            service.changed(&changes(&["/dir/renamed.png", relative])).await;
            wait_until(|| async { Ok(paths(&service).await? == ["/keep.png"]) }).await?;
            assert_eq!(service.months(&[LIBRARY.to_string()]).await?, [("2025-01".to_string(), 1)]);
            std::fs::rename(fixture.path(to), fixture.path("dir/renamed.png"))?;
            service.renamed(LIBRARY, relative, "/dir/renamed.png").await?;
            service.changed(&changes(&[relative, "/dir/renamed.png"])).await;
            wait_until(|| async { Ok(paths(&service).await?.len() == 2) }).await?;
        }

        std::fs::create_dir(fixture.path(".secret"))?;
        std::fs::rename(fixture.path("dir"), fixture.path(".secret/dir"))?;
        service.renamed(LIBRARY, "/dir", "/.secret/dir").await?;
        service.changed(&changes(&["/dir", "/.secret/dir"])).await;
        wait_until(|| async { Ok(paths(&service).await? == ["/keep.png"]) }).await?;
        assert_eq!(service.months(&[LIBRARY.to_string()]).await?, [("2025-01".to_string(), 1)]);
        Ok(())
    }

    #[tokio::test]
    async fn overwriting_an_archive_replaces_its_entries_test() -> TestResult {
        let fixture = Fixture::new()?;
        write_7z(&fixture.path("src.7z"), &[("a.jpg", exif_jpeg("2024:01:01 00:00:00"))])?;
        write_7z(&fixture.path("dst.7z"), &[("b.jpg", exif_jpeg("2024:02:01 00:00:00"))])?;
        let service = fixture.service("UTC", 0).await?;
        wait_scan(&service).await?;
        assert_eq!(paths(&service).await?.len(), 2);

        std::fs::rename(fixture.path("src.7z"), fixture.path("dst.7z"))?;
        service.renamed(LIBRARY, "/src.7z", "/dst.7z").await?;
        assert_eq!(paths(&service).await?, ["/dst.7z/a.jpg"], "置き換えられた宛先の Entry は応答の前に消える");
        assert_eq!(service.months(&[LIBRARY.to_string()]).await?, [("2024-01".to_string(), 1)]);

        // 付け替えた archive の identity は新しい path から決まるので、範囲の集計は読み直さずに世代だけを記録する。
        service.changed(&changes(&["/src.7z", "/dst.7z"])).await;
        tokio::time::sleep(StdDuration::from_millis(500)).await;
        assert_eq!(paths(&service).await?, ["/dst.7z/a.jpg"]);
        Ok(())
    }

    #[tokio::test]
    async fn a_scan_does_not_write_results_read_before_a_deletion_test() -> TestResult {
        let fixture = Fixture::new()?;
        std::fs::write(fixture.path("keep.png"), plain_png())?;
        let service = fixture.service("UTC", 0).await?;
        wait_scan(&service).await?;

        std::fs::write(fixture.path("new.png"), plain_png())?;
        // 変更の反映を模して排他を持つ間に、集計に新しいファイルを読ませ、その後で消す。
        let lock = service.scanner.lock(LIBRARY);
        let guard = lock.lock().await;
        service.refresh(LIBRARY).await?;
        tokio::time::sleep(StdDuration::from_millis(500)).await;
        std::fs::remove_file(fixture.path("new.png"))?;
        drop(guard);
        wait_until(|| async { Ok(!service.status(LIBRARY).await?.running) }).await?;
        assert_eq!(paths(&service).await?, ["/keep.png"], "消したファイルの行は書き込まれない");
        Ok(())
    }

    #[tokio::test]
    async fn startup_scans_only_empty_libraries_when_the_interval_is_zero_test() -> TestResult {
        let fixture = Fixture::new()?;
        std::fs::write(fixture.path("a.png"), plain_png())?;
        let service = fixture.service("UTC", 0).await?;
        service.start();
        wait_idle(&service).await?;
        assert_eq!(paths(&service).await?, ["/a.png"], "索引が空の Library は間隔が 0 でも集計する");
        drop(service);

        std::fs::write(fixture.path("b.png"), plain_png())?;
        let service = fixture.service("UTC", 0).await?;
        service.start();
        tokio::time::sleep(StdDuration::from_millis(500)).await;
        assert!(!service.status(LIBRARY).await?.running);
        assert_eq!(paths(&service).await?, ["/a.png"], "ファイルの行がある Library は間隔が 0 なら集計しない");
        Ok(())
    }

    fn skip_without_ffmpeg() -> bool {
        let available = ["ffmpeg", "ffprobe"].iter().all(|program| {
            std::process::Command::new(program)
                .arg("-version")
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
        });
        if !available {
            assert!(std::env::var_os("PXNA_REQUIRE_FFMPEG").is_none(), "ffmpeg and ffprobe are required in CI");
            eprintln!("skipped: ffmpeg or ffprobe is not available on PATH");
        }
        !available
    }

    /// コンテナの `creation_time` を指定した、短い動画を作る。
    fn write_video(path: &Path, creation_time: Option<&str>) -> TestResult {
        let mut command = std::process::Command::new("ffmpeg");
        command.args(["-hide_banner", "-loglevel", "error", "-y", "-f", "lavfi", "-i", "color=c=black:s=64x64:d=0.2"]);
        if let Some(creation_time) = creation_time {
            command.arg("-metadata").arg(format!("creation_time={creation_time}"));
        }
        let status = command.arg(path).status()?;
        assert!(status.success());
        Ok(())
    }

    #[tokio::test]
    async fn videos_use_creation_time_and_fall_back_to_the_modified_time_test() -> TestResult {
        if skip_without_ffmpeg() {
            return Ok(());
        }
        let fixture = Fixture::new()?;
        write_video(&fixture.path("tagged.mp4"), Some("2022-03-04T05:06:07Z"))?;
        set_modified(&fixture.path("tagged.mp4"), T_2025_01_02)?;
        write_video(&fixture.path("untagged.mkv"), None)?;
        set_modified(&fixture.path("untagged.mkv"), T_2024_01_02)?;

        let service = fixture.service("Asia/Tokyo", 0).await?;
        wait_scan(&service).await?;
        let entries = listed(&service).await?;
        assert_eq!(
            entries.iter().map(|(path, date, source)| (path.as_str(), date.as_str(), *source)).collect::<Vec<_>>(),
            [
                ("/untagged.mkv", "2024-01-02T12:04:05", TimelineDateSource::Modified),
                ("/tagged.mp4", "2022-03-04T14:06:07", TimelineDateSource::CreationTime),
            ],
            "creation_time も更新日時も、設定のタイムゾーンの日時に直す"
        );
        Ok(())
    }
}
