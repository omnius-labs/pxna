use std::{
    collections::{HashMap, VecDeque},
    path::Path,
    sync::{Arc, Weak},
    time::Duration,
};

use tokio::sync::Semaphore;

use super::{ArchiveIdentity, ResolvedStoragePath, SourceFingerprint};
use crate::{
    core::{
        media::{MediaKind, MediaOptions},
        work::{WorkBudget, WorkClass, WorkKind},
    },
    prelude::*,
};

pub use crate::core::media::{CreationTime, ImageExif, ImageInfo, ImageInspector, MediaInfo, MediaTools, VideoInfo};

/// メディア情報とフレーム列の長さ取得が共有する process 内 LRU。
pub struct MediaInfoService {
    tools: MediaTools,
    cache: Mutex<VecDeque<(String, MediaInfo)>>,
    permits: Arc<Semaphore>,
    guards: Mutex<HashMap<String, Weak<TokioMutex<()>>>>,
    budget: Arc<WorkBudget>,
    #[cfg(test)]
    probes: std::sync::atomic::AtomicUsize,
}

impl MediaInfoService {
    pub fn new(options: MediaOptions, timeout: Duration) -> Self {
        Self {
            tools: MediaTools::new(options, timeout),
            cache: Mutex::new(VecDeque::new()),
            permits: Arc::new(Semaphore::new(2)),
            guards: Mutex::new(HashMap::new()),
            budget: Arc::new(WorkBudget::default()),
            #[cfg(test)]
            probes: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Runs the blocking image metadata reads under `budget`, so that they wait with the class the caller passes.
    pub fn with_budget(mut self, budget: Arc<WorkBudget>) -> Self {
        self.budget = budget;
        self
    }

    /// 動画の情報を得る。画像のときは `UnsupportedType` にする。
    pub async fn probe_video(&self, source: &ResolvedStoragePath, class: WorkClass) -> Result<VideoInfo> {
        Ok(self.probe(source, class).await?.video()?.clone())
    }

    /// 拡張子で画像か動画かを決めて、メディア情報を得る。拡張子が表にない file は、動画として ffprobe に渡す。
    pub async fn probe(&self, source: &ResolvedStoragePath, class: WorkClass) -> Result<MediaInfo> {
        let (key, fingerprint) = match source.entry_cache_key() {
            Some(key) => (key.to_hex(), None),
            None => {
                let fingerprint = SourceFingerprint::read(Path::new(source.path().as_str())).await?;
                (ArchiveIdentity::from_local(source.path().as_str(), &fingerprint).to_hex(), Some(fingerprint))
            }
        };
        if let Some(info) = self.cached(&key) {
            return Ok(info);
        }
        let guard = {
            let mut guards = self.guards.lock();
            guards.retain(|_, guard| guard.strong_count() > 0);
            let guard = guards.get(&key).and_then(Weak::upgrade).unwrap_or_else(|| Arc::new(TokioMutex::new(())));
            guards.insert(key.clone(), Arc::downgrade(&guard));
            guard
        };
        let _guard = guard.lock().await;
        if let Some(info) = self.cached(&key) {
            return Ok(info);
        }
        let _permit = self.permits.acquire().await.map_err(|error| Error::from_error(error, ErrorKind::TaskError))?;
        if let Some(info) = self.cached(&key) {
            return Ok(info);
        }
        let file = source.materialize_file(class).await?;
        let name = Self::file_name(source);
        let info = self.inspect(file.path(), name, class).await.map_err(|error| {
            let kind = error.kind().clone();
            Error::from_error(error, kind).with_message("cannot inspect media")
        })?;
        if let Some(fingerprint) = fingerprint {
            fingerprint.ensure_unchanged(file.path()).await?;
        }
        #[cfg(test)]
        self.probes.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let mut cache = self.cache.lock();
        cache.retain(|(existing, _)| existing != &key);
        cache.push_front((key, info.clone()));
        cache.truncate(256);
        Ok(info)
    }

    async fn inspect(&self, file: &Path, name: &str, class: WorkClass) -> Result<MediaInfo> {
        let image = |width, height| {
            let content_type = MediaKind::content_type(name).unwrap_or("application/octet-stream").to_string();
            (content_type, width, height)
        };
        let to_info = |(content_type, width, height): (String, u32, u32), rotation: u16, has_alpha: bool, exif| {
            MediaInfo::Image(ImageInfo {
                content_type,
                width,
                height,
                rotation,
                has_alpha,
                exif,
            })
        };
        match MediaKind::of_name(name) {
            Some(MediaKind::Image) => {
                let path = file.to_path_buf();
                let (width, height, exif) = self
                    .budget
                    .io()
                    .ticket(class)
                    .counted(WorkKind::Inspect)
                    .acquire()
                    .await?
                    .run_blocking(move || -> Result<_> {
                        let (width, height) =
                            ImageInspector::read_size(&path).map_err(|error| Error::from_error(error, ErrorKind::UnsupportedType).with_message("cannot read the image size"))?;
                        Ok((width, height, ImageInspector::read_exif(&path)))
                    })
                    .await??;
                Ok(to_info(image(width, height), 0, false, exif))
            }
            Some(MediaKind::FfmpegImage | MediaKind::HeifImage) => {
                let (width, height, rotation, has_alpha) = self.tools.probe_image_size(file, class).await?;
                let path = file.to_path_buf();
                let exif = self
                    .budget
                    .io()
                    .ticket(class)
                    .counted(WorkKind::Inspect)
                    .acquire()
                    .await?
                    .run_blocking(move || ImageInspector::read_exif(&path))
                    .await?;
                Ok(to_info(image(width, height), rotation, has_alpha, exif))
            }
            Some(MediaKind::Video) | None => Ok(MediaInfo::Video(self.tools.probe_video_info(file, class).await?)),
        }
    }

    /// 最後の segment の、親を除いた名前。拡張子の判定に使う。
    fn file_name(source: &ResolvedStoragePath) -> &str {
        source.path().iter().last().and_then(|segment| segment.rsplit(['/', '\\']).next()).unwrap_or_default()
    }

    fn cached(&self, key: &str) -> Option<MediaInfo> {
        let mut cache = self.cache.lock();
        let index = cache.iter().position(|(existing, _)| existing == key)?;
        let entry = cache.remove(index)?;
        let info = entry.1.clone();
        cache.push_front(entry);
        Some(info)
    }

    #[cfg(test)]
    pub fn probe_count(&self) -> usize {
        self.probes.load(std::sync::atomic::Ordering::SeqCst)
    }
}
