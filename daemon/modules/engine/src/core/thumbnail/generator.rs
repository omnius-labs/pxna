use std::{
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_util::bytes::Bytes;

use crate::{
    base::util::SingleFlight,
    core::{
        image::{ImageConvertOptions, ImageConverter, SizeSteps},
        media::{FrameSpec, MediaKind, MediaOptions, MediaTools},
        thumbnail::{
            cache::{CacheBucket, CachedImage, ThumbnailCache, ThumbnailCacheOptions},
            key::{ThumbnailKey, ThumbnailKeyHasher},
            request::{Thumbnail, ThumbnailRequest, ThumbnailVariant},
        },
    },
    prelude::*,
    service::{EntryCacheKey, MediaInfo, MediaInfoService, NestedFsEntry, ResolvedStoragePath, StorageService},
};

const GIB: u64 = 1024 * 1024 * 1024;

/// 動画から取り出すフレームの数と位置。設定ファイルからは指定できず、ここから渡す。
const FRAME_SPEC: FrameSpec = FrameSpec::DEFAULT;

/// サムネイルの生成とキャッシュの設定。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThumbnailOptions {
    /// サムネイルとスプライトシートの幅と高さを切り上げる先のサイズ段階。
    pub sizes: Vec<u32>,
    /// 表示用画像の長辺を収める先のサイズ段階。
    pub display_sizes: Vec<u32>,
    /// 要求で形式を指定しないときの出力形式。
    pub format: ImageFormatType,
    /// サムネイルとスプライトシートの非可逆の圧縮の品質（1 から 100）。
    pub quality: u8,
    /// 表示用画像の非可逆の圧縮の品質（1 から 100）。
    pub display_quality: u8,
    pub cache_capacity_bytes: u64,
    pub display_cache_capacity_bytes: u64,
    pub max_concurrent_images: usize,
    /// 同時に走る ffmpeg の数。
    pub max_concurrent_videos: usize,
    /// 1 回の ffmpeg の実行の期限。
    pub video_timeout: Duration,
}

impl Default for ThumbnailOptions {
    fn default() -> Self {
        Self {
            sizes: vec![128, 256, 512, 1024],
            display_sizes: vec![2048, 4096],
            format: ImageFormatType::Webp,
            quality: 80,
            display_quality: 90,
            cache_capacity_bytes: 2 * GIB,
            display_cache_capacity_bytes: 2 * GIB,
            max_concurrent_images: 4,
            max_concurrent_videos: 2,
            video_timeout: Duration::from_secs(60),
        }
    }
}

impl ThumbnailOptions {
    /// 設定として正しいかを確かめる。
    pub fn validate(&self) -> Result<()> {
        self.steps().map(|_| ())
    }

    fn steps(&self) -> Result<(SizeSteps, SizeSteps)> {
        let positive = self.cache_capacity_bytes > 0
            && self.display_cache_capacity_bytes > 0
            && self.max_concurrent_images > 0
            && self.max_concurrent_videos > 0
            && !self.video_timeout.is_zero();
        if !positive {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("thumbnail capacities, concurrency, and the timeout must be greater than zero"));
        }
        if self.format == ImageFormatType::Unknown {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("thumbnail format must be webp, jpeg, or png"));
        }
        if !(1..=100).contains(&self.quality) || !(1..=100).contains(&self.display_quality) {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("thumbnail quality and display_quality must be between 1 and 100"));
        }
        Ok((SizeSteps::new(self.sizes.clone())?, SizeSteps::new(self.display_sizes.clone())?))
    }
}

/// 画像、動画、フォルダ、archive から、サムネイルと表示用画像を生成してキャッシュする。
///
/// 生成は要求を受けたときにだけ行う。どの要求を先に生成するかはクライアントが決め、ここでは到着順に処理する。
#[derive(Clone)]
pub struct ThumbnailGenerator {
    inner: Arc<Inner>,
}

struct Inner {
    fs: Arc<StorageService>,
    cache: ThumbnailCache,
    selection: super::selection::RepresentativeSelection,
    tools: MediaTools,
    media_info: Arc<MediaInfoService>,
    sizes: SizeSteps,
    display_sizes: SizeSteps,
    quality: u8,
    display_quality: u8,
    image_permits: Arc<Semaphore>,
    video_permits: Arc<Semaphore>,
    /// 表示用画像の専用枠。一覧のサムネイル生成の待ち行列に、プレビューを並ばせない。
    display_image_permits: Arc<Semaphore>,
    display_video_permits: Arc<Semaphore>,
    flights: SingleFlight<ThumbnailKey, Produced>,
    #[cfg(test)]
    conversions: std::sync::atomic::AtomicUsize,
    #[cfg(test)]
    ffmpeg_runs: std::sync::atomic::AtomicUsize,
    #[cfg(test)]
    heif_dec_runs: std::sync::atomic::AtomicUsize,
}

/// 要求を、サイズ段階へ切り上げた、キャッシュのキーに含める条件にそろえたもの。
#[derive(Debug, Clone)]
struct Plan {
    variant: ThumbnailVariant,
    bucket: CacheBucket,
    convert: ImageConvertOptions,
}

/// 1 回の生成で得た画像。動画は、サムネイルとスプライトシートを一緒に作る。
#[derive(Debug, Clone)]
struct Produced {
    /// サムネイルか、表示用画像。
    image: CachedImage,
    sprite: Option<CachedImage>,
}

/// 生成の 1 件分の仕事。独立した task へ渡すため、借用を持たない。
struct Job {
    source: ResolvedStoragePath,
    plan: Plan,
    kind: MediaKind,
    /// `image` のキャッシュのキー。元の file の更新を取れない場合は、キャッシュしないので `None` になる。
    primary_key: Option<ThumbnailKey>,
    sprite_key: Option<ThumbnailKey>,
}

/// キャッシュの key に含める、元の file を指す値。
enum SourceId {
    Local { path: String, len: u64, modified_secs: i64, modified_nanos: u32 },
    Archive(EntryCacheKey),
}

impl ThumbnailGenerator {
    pub async fn open(fs: Arc<StorageService>, cache_dir: &Path, options: ThumbnailOptions, media: MediaOptions) -> Result<Self> {
        let media_info = Arc::new(MediaInfoService::new(media.clone(), options.video_timeout));
        Self::open_with_media(fs, cache_dir, options, media, media_info).await
    }

    pub async fn open_with_media(fs: Arc<StorageService>, cache_dir: &Path, options: ThumbnailOptions, media: MediaOptions, media_info: Arc<MediaInfoService>) -> Result<Self> {
        let (sizes, display_sizes) = options.steps()?;
        let cache = ThumbnailCache::open(
            cache_dir,
            ThumbnailCacheOptions {
                thumbnail_capacity_bytes: options.cache_capacity_bytes,
                display_capacity_bytes: options.display_cache_capacity_bytes,
            },
        )
        .await?;

        Ok(Self {
            inner: Arc::new(Inner {
                selection: super::selection::RepresentativeSelection::new(fs.clone()),
                fs,
                cache,
                tools: MediaTools::new(media, options.video_timeout),
                media_info,
                sizes,
                display_sizes,
                quality: options.quality,
                display_quality: options.display_quality,
                image_permits: Arc::new(Semaphore::new(options.max_concurrent_images)),
                video_permits: Arc::new(Semaphore::new(options.max_concurrent_videos)),
                display_image_permits: Arc::new(Semaphore::new(options.max_concurrent_images)),
                display_video_permits: Arc::new(Semaphore::new(options.max_concurrent_videos)),
                flights: SingleFlight::new(),
                #[cfg(test)]
                conversions: std::sync::atomic::AtomicUsize::new(0),
                #[cfg(test)]
                ffmpeg_runs: std::sync::atomic::AtomicUsize::new(0),
                #[cfg(test)]
                heif_dec_runs: std::sync::atomic::AtomicUsize::new(0),
            }),
        })
    }

    /// file にはその file のサムネイルを返す。フォルダと archive には、直下の代表の file のサムネイルを返す。
    pub async fn generate(&self, path: &NestedPath, request: &ThumbnailRequest) -> Result<Thumbnail> {
        self.generate_resolved(self.inner.fs.resolve_entry(path).await?, request).await
    }

    /// daemon の境界探索で得た driver を、そのまま生成に使う。
    pub async fn generate_resolved(&self, resolved: ResolvedStoragePath, request: &ThumbnailRequest) -> Result<Thumbnail> {
        let inner = &self.inner;
        let path = resolved.path().to_nested_path_buf();
        let entry = resolved.get_entry().await?;
        let (target, entry) = match entry.detail.clone() {
            FsEntryDetail::File { archive: false } => (resolved, entry),
            FsEntryDetail::Dir { .. } | FsEntryDetail::File { archive: true } => {
                let mut folder = path.to_nested_path_buf();
                if matches!(entry.detail, FsEntryDetail::File { .. }) {
                    folder.push_archive_root()?;
                }
                let folder = if folder == path { resolved } else { inner.fs.resolve_entry(&folder).await? };
                let mut target = inner.selection.select(&folder).await?;
                let metadata_started = Instant::now();
                let entry = match target.get_listing_entry().await {
                    Ok(entry) if matches!(entry.detail, FsEntryDetail::File { .. }) => entry,
                    Ok(_) => {
                        inner.selection.invalidate();
                        target = inner.selection.select(&folder).await?;
                        target.get_listing_entry().await?
                    }
                    Err(error) if error.kind() == &ErrorKind::NotFound => {
                        inner.selection.invalidate();
                        target = inner.selection.select(&folder).await?;
                        target.get_listing_entry().await?
                    }
                    Err(error) => return Err(error),
                };
                if !matches!(entry.detail, FsEntryDetail::File { .. }) {
                    return Err(Error::new(ErrorKind::UnsupportedType).with_message("representative is no longer a file"));
                }
                tracing::debug!(target: "pxna::performance", operation = "representative_metadata", elapsed_sec = metadata_started.elapsed().as_secs_f64(), "representative metadata read completed");
                (target, entry)
            }
            FsEntryDetail::Link { .. } => return Err(Error::new(ErrorKind::UnsupportedType).with_message(format!("not a file: {path}"))),
        };

        let name = Inner::file_name(&entry.path);
        let kind = MediaKind::of_name(name).ok_or_else(|| Error::new(ErrorKind::UnsupportedType).with_message(format!("not a supported image or video: {name}")))?;
        if request.variant == ThumbnailVariant::Sprite && kind != MediaKind::Video {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("the sprite variant requires a video"));
        }

        let plan = inner.plan(request)?;
        let source = inner.source_id(&target, &entry);
        let key_of = |variant| source.as_ref().map(|source| plan.key(source, variant, kind));
        let requested_key = key_of(request.variant);

        if let Some(key) = &requested_key
            && let Some(hit) = inner.cache.get(key).await?
        {
            tracing::debug!(target: "pxna::performance", operation = "thumbnail_image_cache", hit = true, variant = ?request.variant, "thumbnail image cache checked");
            return Ok(plan.thumbnail(hit, Some(key)));
        }
        tracing::debug!(target: "pxna::performance", operation = "thumbnail_image_cache", hit = false, variant = ?request.variant, name, eligible = requested_key.is_some(), "thumbnail image cache checked");

        // 動画のサムネイルとスプライトシートは 1 回の ffmpeg の実行で両方を作るため、同じ flight にまとめる。
        let shares_a_run = kind == MediaKind::Video && request.variant != ThumbnailVariant::Display;
        let job = Job {
            source: target,
            plan: plan.clone(),
            kind,
            primary_key: if shares_a_run { key_of(ThumbnailVariant::Thumbnail) } else { requested_key.clone() },
            sprite_key: if shares_a_run { key_of(ThumbnailVariant::Sprite) } else { None },
        };
        let flight_key = job.primary_key.clone();
        let semaphore = inner.semaphore_for(kind, request.variant);
        let file_name = name.to_string();
        let gate = {
            let inner = inner.clone();
            let file_name = file_name.clone();
            let variant = request.variant;
            async move {
                let available = semaphore.available_permits();
                let waiting = Instant::now();
                let permit = inner.acquire(semaphore).await?;
                tracing::debug!(target: "pxna::performance", operation = "thumbnail_permit_wait", elapsed_sec = waiting.elapsed().as_secs_f64(), available_when_requested = available, variant = ?variant, kind = ?kind, name = file_name, "thumbnail permit acquired");
                Ok(permit)
            }
        };

        // 生成枠と同じ key の生成を待つ時間も含む。生成そのものは `thumbnail_production` が測る。
        let generating = Instant::now();
        let produced = match flight_key {
            Some(flight_key) => {
                let work = inner.clone();
                let produced = inner
                    .flights
                    .run_gated(flight_key, gate, move |permit| async move { work.produce(job, permit).await })
                    .await?;
                (*produced).clone()
            }
            None => {
                let permit = gate.await?;
                inner.clone().produce(job, permit).await?
            }
        };

        tracing::debug!(target: "pxna::performance", operation = "thumbnail_generation", elapsed_sec = generating.elapsed().as_secs_f64(), variant = ?request.variant, kind = ?kind, name = file_name, "thumbnail generation completed");
        let image = match request.variant {
            ThumbnailVariant::Sprite => produced
                .sprite
                .ok_or_else(|| Error::new(ErrorKind::UnexpectedError).with_message("the sprite was not produced"))?,
            _ => produced.image,
        };
        Ok(plan.thumbnail(image, requested_key.as_ref()))
    }

    /// 実際に変換した回数。キャッシュと single-flight が効いているかを、テストが確かめるために使う。
    #[cfg(test)]
    fn conversions(&self) -> usize {
        self.inner.conversions.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// ffmpeg を起動した回数。
    #[cfg(test)]
    fn ffmpeg_runs(&self) -> usize {
        self.inner.ffmpeg_runs.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// heif-dec を起動した回数。
    #[cfg(test)]
    fn heif_dec_runs(&self) -> usize {
        self.inner.heif_dec_runs.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl Inner {
    /// path の最後の segment の、親を除いた名前。local は host の path、archive 内は `/` 区切りの path である。
    fn file_name(path: &NestedPathBuf) -> &str {
        path.iter().last().and_then(|segment| segment.rsplit(['/', '\\']).next()).unwrap_or_default()
    }

    fn plan(&self, request: &ThumbnailRequest) -> Result<Plan> {
        let (bucket, resize, width, height, quality) = match request.variant {
            ThumbnailVariant::Thumbnail | ThumbnailVariant::Sprite => (
                CacheBucket::Thumbnail,
                request.resize,
                request.width.map(|width| self.sizes.round_up(width)),
                request.height.map(|height| self.sizes.round_up(height)),
                self.quality,
            ),
            // 表示用画像は縦横比を保って長辺を収める。幅と高さのどちらを指定しても、長辺の上限として扱う。
            ThumbnailVariant::Display => {
                let requested = request.width.into_iter().chain(request.height).max();
                let side = requested.map_or(self.display_sizes.max(), |side| self.display_sizes.round_up(side));
                (CacheBucket::Display, ImageResizeType::Fit, Some(side), Some(side), self.display_quality)
            }
        };

        let convert = ImageConvertOptions {
            format_type: request.format,
            quality,
            resize_type: resize,
            resampler_type: ImageResamplerType::Lanczos3,
            anchor_position_type: request.anchor,
            width,
            height,
        };
        ImageConverter::validate(&convert)?;

        Ok(Plan {
            variant: request.variant,
            bucket,
            convert,
        })
    }

    /// `mtime` を取れない local file は、更新を検知できないため、キャッシュしない。
    fn source_id(&self, path: &ResolvedStoragePath, entry: &NestedFsEntry) -> Option<SourceId> {
        if let Some(key) = path.entry_cache_key() {
            return Some(SourceId::Archive(key));
        }
        entry.meta.modified.map(|modified| SourceId::Local {
            path: entry.path.as_str().to_string(),
            len: entry.meta.len,
            modified_secs: modified.timestamp(),
            modified_nanos: modified.timestamp_subsec_nanos(),
        })
    }

    /// 画像の変換と、外部プロセスの ffmpeg は、別の同時実行数の上限で数える。
    /// 表示用画像は、ユーザーが待っているため、サムネイルとは別の枠で数える。
    fn semaphore_for(&self, kind: MediaKind, variant: ThumbnailVariant) -> Arc<Semaphore> {
        match (kind, variant == ThumbnailVariant::Display) {
            (MediaKind::Image, false) => self.image_permits.clone(),
            (MediaKind::Image, true) => self.display_image_permits.clone(),
            (_, false) => self.video_permits.clone(),
            (_, true) => self.display_video_permits.clone(),
        }
    }

    /// 同時実行数の上限を待つ。待つ間に要求が drop されたら、何も始まらない。
    async fn acquire(&self, semaphore: Arc<Semaphore>) -> Result<OwnedSemaphorePermit> {
        semaphore.acquire_owned().await.map_err(|error| Error::from_error(error, ErrorKind::TaskError))
    }

    /// 生成してキャッシュへ入れる。独立した task で走るため、始まった後に要求が切れても最後まで走る。
    async fn produce(self: Arc<Self>, job: Job, _permit: OwnedSemaphorePermit) -> Result<Produced> {
        // 直前に終わった同じ key の処理が入れた結果を、重ねて作らない。
        if let Some(cached) = self.cached(&job).await? {
            return Ok(cached);
        }

        let started = Instant::now();
        // 変換が終わるまで、archive 内の file の lease を保つ。
        let file = job.source.materialize_file().await?;
        tracing::debug!(target: "pxna::performance", operation = "thumbnail_materialize", elapsed_sec = started.elapsed().as_secs_f64(), "thumbnail source materialized");
        let converting = Instant::now();
        let produced = match job.kind {
            MediaKind::Image => {
                let reading = Instant::now();
                let input = tokio::fs::read(file.path()).await?;
                tracing::debug!(target: "pxna::performance", operation = "thumbnail_source_read", elapsed_sec = reading.elapsed().as_secs_f64(), bytes = input.len(), "source file read");
                Produced {
                    image: self.convert_one(input, &job.plan).await?,
                    sprite: None,
                }
            }
            MediaKind::FfmpegImage | MediaKind::HeifImage => {
                let decoding = Instant::now();
                let input = self.decode_ffmpeg_image(&job.source, file.path(), job.kind).await?;
                tracing::debug!(target: "pxna::performance", operation = "thumbnail_external_decode", elapsed_sec = decoding.elapsed().as_secs_f64(), kind = ?job.kind, bytes = input.len(), "external decoder finished");
                Produced {
                    image: self.convert_one(input, &job.plan).await?,
                    sprite: None,
                }
            }
            MediaKind::Video => self.produce_video(file.path(), &job.source, &job.plan).await?,
        };
        #[cfg(test)]
        self.conversions.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        tracing::debug!(target: "pxna::performance", operation = "thumbnail_convert", elapsed_sec = converting.elapsed().as_secs_f64(), "thumbnail source read and converted");
        let storing = Instant::now();
        self.store(&job, &produced).await;
        tracing::debug!(target: "pxna::performance", operation = "thumbnail_store", elapsed_sec = storing.elapsed().as_secs_f64(), "thumbnail stored");
        tracing::debug!(target: "pxna::performance", operation = "thumbnail_production", elapsed_sec = started.elapsed().as_secs_f64(), "thumbnail produced and stored");
        Ok(produced)
    }

    async fn cached(&self, job: &Job) -> Result<Option<Produced>> {
        let Some(primary_key) = &job.primary_key else {
            return Ok(None);
        };
        let Some(image) = self.cache.get(primary_key).await? else {
            return Ok(None);
        };
        let sprite = match &job.sprite_key {
            Some(key) => match self.cache.get(key).await? {
                Some(sprite) => Some(sprite),
                None => return Ok(None),
            },
            None => None,
        };
        Ok(Some(Produced { image, sprite }))
    }

    /// キャッシュへ入れられなくても、生成した画像は返す。
    async fn store(&self, job: &Job, produced: &Produced) {
        let targets = [
            (&job.primary_key, Some(&produced.image), job.plan.bucket),
            (&job.sprite_key, produced.sprite.as_ref(), CacheBucket::Thumbnail),
        ];
        for (key, image, bucket) in targets {
            if let (Some(key), Some(image)) = (key, image)
                && let Err(error) = self.cache.put(key, bucket, image).await
            {
                warn!(error = ?error, "failed to store a thumbnail in the cache");
            }
        }
    }

    async fn convert_one(&self, input: Vec<u8>, plan: &Plan) -> Result<CachedImage> {
        let options = plan.convert.clone();
        let bytes = tokio::task::spawn_blocking(move || ImageConverter::convert(&input, &options))
            .await
            .map_err(|error| Error::from_error(error, ErrorKind::TaskError))?
            .map_err(Self::decode_error)?;
        Ok(CachedImage {
            bytes: Bytes::from(bytes),
            frame_count: None,
        })
    }

    /// 動画のフレームを取り出し、サムネイルと、フレームを横に並べたスプライトシートを作る。
    /// 表示用画像は、最初のフレームだけを使う。
    async fn produce_video(&self, file: &Path, source: &ResolvedStoragePath, plan: &Plan) -> Result<Produced> {
        let duration = self.media_info.probe_video(source).await?.duration_secs;
        let mut positions = FRAME_SPEC.positions(duration);
        if plan.variant == ThumbnailVariant::Display {
            positions.truncate(1);
        }
        let frames = self.extract_frames(file, &positions).await?;

        let plan = plan.clone();
        tokio::task::spawn_blocking(move || -> Result<Produced> {
            let resized = frames
                .iter()
                .map(|frame| ImageConverter::decode(frame).and_then(|image| ImageConverter::resize(image, &plan.convert)))
                .collect::<Result<Vec<_>>>()
                .map_err(Self::decode_error)?;

            let image = CachedImage {
                bytes: Bytes::from(ImageConverter::encode(&resized[0], plan.convert.format_type, plan.convert.quality)?),
                frame_count: None,
            };
            let sprite = if plan.variant == ThumbnailVariant::Display {
                None
            } else {
                Some(CachedImage {
                    bytes: Bytes::from(ImageConverter::encode(
                        &ImageConverter::tile_horizontally(&resized),
                        plan.convert.format_type,
                        plan.convert.quality,
                    )?),
                    frame_count: Some(resized.len() as u32),
                })
            };
            Ok(Produced { image, sprite })
        })
        .await
        .map_err(|error| Error::from_error(error, ErrorKind::TaskError))?
    }

    async fn extract_frames(&self, file: &Path, positions_secs: &[f64]) -> Result<Vec<Vec<u8>>> {
        #[cfg(test)]
        self.ffmpeg_runs.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.tools.extract_frames(file, positions_secs).await
    }

    /// ffmpeg で読む画像をデコードする。HEIC/HEIF/AVIF は PPM、それ以外（JXL、JP2）は PNG を使う。
    /// アルファ付きの HEIC/HEIF/AVIF は heif-dec に回し、ffmpeg が失敗したときも heif-dec で読み直す。
    async fn decode_ffmpeg_image(&self, source: &ResolvedStoragePath, file: &Path, kind: MediaKind) -> Result<Vec<u8>> {
        let heif_decodable = kind == MediaKind::HeifImage || Self::is_avif(source);
        if !heif_decodable {
            return self.extract_image(file).await;
        }
        if self.has_alpha(source).await? {
            return self.decode_heif(file).await;
        }
        match self.extract_image_pixels(file).await {
            Ok(image) => Ok(image),
            Err(error) => {
                warn!(error = ?error, "ffmpeg failed to decode a HEIF or AVIF image, so heif-dec reads it instead");
                self.decode_heif(file).await
            }
        }
    }

    /// 拡張子が avif の file か。アルファを保持できる heif-dec に回せるかの判定に使う。
    fn is_avif(source: &ResolvedStoragePath) -> bool {
        Self::file_name(&source.path().to_nested_path_buf())
            .rsplit_once('.')
            .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("avif"))
    }

    /// 画像がアルファの補助画像を持つか。probe に失敗したときは、アルファを見落とさないよう heif-dec に回す。
    async fn has_alpha(&self, source: &ResolvedStoragePath) -> Result<bool> {
        match self.media_info.probe(source).await {
            Ok(MediaInfo::Image(info)) => Ok(info.has_alpha),
            Ok(_) => Ok(false),
            Err(error) => {
                warn!(error = ?error, "failed to probe an image for alpha, so heif-dec reads it");
                Ok(true)
            }
        }
    }

    async fn extract_image(&self, file: &Path) -> Result<Vec<u8>> {
        #[cfg(test)]
        self.ffmpeg_runs.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.tools.extract_image(file).await
    }

    async fn extract_image_pixels(&self, file: &Path) -> Result<Vec<u8>> {
        #[cfg(test)]
        self.ffmpeg_runs.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.tools.extract_image_pixels(file).await
    }

    /// heif-dec で読む。アルファ付きの HEIC/HEIF/AVIF と、ffmpeg が読めなかったときのフォールバックに使う。
    async fn decode_heif(&self, file: &Path) -> Result<Vec<u8>> {
        #[cfg(test)]
        self.heif_dec_runs.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.tools.decode_heif(file).await
    }

    /// 拡張子が合っていても中身が読めない file は、`UnsupportedType` にする。
    fn decode_error(error: Error) -> Error {
        match error.kind() {
            ErrorKind::ImageError => Error::from_error(error, ErrorKind::UnsupportedType).with_message("the file cannot be decoded as an image"),
            _ => error,
        }
    }
}

impl Plan {
    /// 動画のキーは、フレームの数と位置を含める。
    fn key(&self, source: &SourceId, variant: ThumbnailVariant, kind: MediaKind) -> ThumbnailKey {
        let mut hasher = ThumbnailKeyHasher::new();
        match source {
            SourceId::Local {
                path,
                len,
                modified_secs,
                modified_nanos,
            } => {
                hasher.field(b"local").field(path.as_bytes()).field(&len.to_be_bytes());
                hasher.field(&modified_secs.to_be_bytes()).field(&modified_nanos.to_be_bytes());
            }
            SourceId::Archive(key) => {
                hasher.field(b"archive").field(key.to_hex().as_bytes());
            }
        }

        let tag = match variant {
            ThumbnailVariant::Thumbnail => "thumbnail",
            ThumbnailVariant::Display => "display",
            ThumbnailVariant::Sprite => "sprite",
        };
        let dimension = |value: Option<u32>| value.map_or(0, |value| value).to_be_bytes();
        hasher
            .field(tag.as_bytes())
            .field(self.convert.resize_type.as_ref().as_bytes())
            .field(self.convert.anchor_position_type.as_ref().as_bytes())
            .field(self.convert.resampler_type.as_ref().as_bytes())
            .field(self.convert.format_type.as_ref().as_bytes())
            .field(&dimension(self.convert.width))
            .field(&dimension(self.convert.height));
        // 品質は非可逆の形式の出力だけを変える。PNG は品質を使わないので、品質を変えてもキーを変えない。
        if self.convert.format_type != ImageFormatType::Png {
            hasher.field(&[self.convert.quality]);
        }
        // 動画は、表示用画像も、取り出す位置が FRAME_SPEC の始点に依存する。
        if kind == MediaKind::Video {
            for part in FRAME_SPEC.key_parts() {
                hasher.field(&part.to_be_bytes());
            }
        }
        hasher.finish()
    }

    fn thumbnail(&self, image: CachedImage, key: Option<&ThumbnailKey>) -> Thumbnail {
        Thumbnail {
            bytes: image.bytes,
            format: self.convert.format_type,
            frame_count: image.frame_count,
            key: key.map(|key| key.hex().to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{io::Cursor, time::Duration};

    use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
    use omnius_core_base::{clock::ClockUtc, tsid::TsidProviderImpl};
    use rand::{SeedableRng as _, rngs::ChaCha20Rng};
    use testresult::TestResult;

    use crate::{
        core::nfs::FileStorageLocal,
        service::{ArchiveOptions, ArchiveRuntime, ImageExif, MediaInfo},
    };

    use super::*;

    struct Fixture {
        dir: tempfile::TempDir,
        generator: ThumbnailGenerator,
    }

    async fn fixture(options: ThumbnailOptions) -> TestResult<Fixture> {
        fixture_with_media(options, MediaOptions::default()).await
    }

    async fn fixture_with_media(options: ThumbnailOptions, media: MediaOptions) -> TestResult<Fixture> {
        let dir = tempfile::tempdir()?;
        let tsid_provider: Arc<Mutex<dyn omnius_core_base::tsid::TsidProvider + Send + Sync>> =
            Arc::new(Mutex::new(TsidProviderImpl::new(ClockUtc, ChaCha20Rng::seed_from_u64(0), 16)));
        let runtime = Arc::new(ArchiveRuntime::open(ArchiveOptions::default(), &dir.path().join("archive-cache")).await?);
        let fs = Arc::new(StorageService::new(Arc::new(FileStorageLocal::new(tsid_provider)), runtime));
        let generator = ThumbnailGenerator::open(fs, &dir.path().join("thumbnail"), options, media).await?;
        Ok(Fixture { dir, generator })
    }

    fn write_png(path: &Path, width: u32, height: u32) -> TestResult {
        let mut output = Vec::new();
        RgbaImage::from_pixel(width, height, Rgba([10, 20, 30, 255])).write_to(&mut Cursor::new(&mut output), ImageFormat::Png)?;
        std::fs::write(path, output)?;
        Ok(())
    }

    fn request(variant: ThumbnailVariant, width: Option<u32>, height: Option<u32>, format: ImageFormatType, resize: ImageResizeType) -> ThumbnailRequest {
        ThumbnailRequest {
            variant,
            width,
            height,
            format,
            resize,
            anchor: ImageAnchorPositionType::Center,
        }
    }

    fn thumbnail_request(width: u32, height: u32) -> ThumbnailRequest {
        request(ThumbnailVariant::Thumbnail, Some(width), Some(height), ImageFormatType::Jpeg, ImageResizeType::Fit)
    }

    fn nested(path: &Path) -> NestedPathBuf {
        NestedPathBuf::try_from(path).unwrap()
    }

    fn size(thumbnail: &Thumbnail) -> TestResult<(u32, u32)> {
        let image: DynamicImage = ImageConverter::decode(&thumbnail.bytes)?;
        Ok((image.width(), image.height()))
    }

    #[tokio::test]
    async fn rounds_the_box_up_to_the_size_steps_test() -> TestResult {
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let file = fixture.dir.path().join("wide.png");
        write_png(&file, 1000, 500)?;

        let fit = fixture.generator.generate(&nested(&file), &thumbnail_request(200, 200)).await?;
        assert_eq!(size(&fit)?, (256, 128), "200 は 256 に切り上がり、縦横比を保って収まる");
        assert_eq!(image::guess_format(&fit.bytes)?, ImageFormat::Jpeg);

        let crop = request(ThumbnailVariant::Thumbnail, Some(300), Some(300), ImageFormatType::Png, ImageResizeType::Crop);
        let cropped = fixture.generator.generate(&nested(&file), &crop).await?;
        assert_eq!(size(&cropped)?, (512, 512), "300 は 512 に切り上がる");
        assert_eq!(image::guess_format(&cropped.bytes)?, ImageFormat::Png);
        assert_eq!(cropped.format, ImageFormatType::Png);
        Ok(())
    }

    #[tokio::test]
    async fn display_fits_the_long_edge_into_the_display_step_test() -> TestResult {
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let large = fixture.dir.path().join("large.png");
        write_png(&large, 4000, 2000)?;
        let small = fixture.dir.path().join("small.png");
        write_png(&small, 800, 400)?;

        let display = |side| request(ThumbnailVariant::Display, side, side, ImageFormatType::Jpeg, ImageResizeType::Crop);
        assert_eq!(
            size(&fixture.generator.generate(&nested(&large), &display(Some(2048))).await?)?,
            (2048, 1024),
            "長辺が 2048 に収まり、縦横比を保つ"
        );
        assert_eq!(
            size(&fixture.generator.generate(&nested(&large), &display(Some(3000))).await?)?,
            (4000, 2000),
            "3000 は 4096 に切り上がり、4000 の長辺はそのまま収まる"
        );
        assert_eq!(
            size(&fixture.generator.generate(&nested(&large), &display(None)).await?)?,
            (4000, 2000),
            "幅と高さを指定しなければ、最大の段階へ収める"
        );
        assert_eq!(
            size(&fixture.generator.generate(&nested(&small), &display(Some(2048))).await?)?,
            (800, 400),
            "元の画像より大きくしない"
        );
        Ok(())
    }

    #[tokio::test]
    async fn serves_the_second_request_from_the_cache_test() -> TestResult {
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let file = fixture.dir.path().join("a.png");
        write_png(&file, 600, 300)?;

        let first = fixture.generator.generate(&nested(&file), &thumbnail_request(256, 256)).await?;
        let second = fixture.generator.generate(&nested(&file), &thumbnail_request(256, 256)).await?;
        assert_eq!(first, second);
        assert_eq!(fixture.generator.conversions(), 1, "2 回目は変換しない");
        assert!(first.key.is_some());

        // 段階へ切り上がる前の値が違っても、同じ段階なら同じキャッシュに当たる。
        fixture.generator.generate(&nested(&file), &thumbnail_request(250, 250)).await?;
        assert_eq!(fixture.generator.conversions(), 1);

        // 条件が違えば別のキャッシュになる。
        fixture.generator.generate(&nested(&file), &thumbnail_request(512, 512)).await?;
        assert_eq!(fixture.generator.conversions(), 2);
        Ok(())
    }

    #[tokio::test]
    async fn display_uses_its_own_capacity_bucket_test() -> TestResult {
        // サムネイルの枠を 1 byte にして、そこへ入れようとした画像が捨てられるようにする。
        let fixture = fixture(ThumbnailOptions {
            cache_capacity_bytes: 1,
            ..ThumbnailOptions::default()
        })
        .await?;
        let file = fixture.dir.path().join("a.png");
        write_png(&file, 600, 300)?;

        let display = request(ThumbnailVariant::Display, None, None, ImageFormatType::Jpeg, ImageResizeType::Fit);
        fixture.generator.generate(&nested(&file), &display).await?;
        fixture.generator.generate(&nested(&file), &display).await?;
        assert_eq!(fixture.generator.conversions(), 1, "表示用画像は自分の枠に入る");

        fixture.generator.generate(&nested(&file), &thumbnail_request(256, 256)).await?;
        fixture.generator.generate(&nested(&file), &thumbnail_request(256, 256)).await?;
        assert_eq!(fixture.generator.conversions(), 3, "サムネイルの枠に入らない画像はキャッシュされない");
        Ok(())
    }

    #[tokio::test]
    async fn concurrent_requests_convert_once_test() -> TestResult {
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let file = fixture.dir.path().join("a.png");
        write_png(&file, 3000, 2000)?;

        let mut tasks = Vec::new();
        for _ in 0..6 {
            let (generator, path) = (fixture.generator.clone(), nested(&file));
            tasks.push(tokio::spawn(async move { generator.generate(&path, &thumbnail_request(256, 256)).await }));
        }
        let results = futures::future::try_join_all(tasks).await?.into_iter().collect::<Result<Vec<_>>>()?;
        assert!(results.windows(2).all(|pair| pair[0] == pair[1]));
        assert_eq!(fixture.generator.conversions(), 1, "同時に送った同じ要求の変換は 1 回で済む");
        Ok(())
    }

    #[tokio::test]
    async fn a_changed_file_gets_a_new_key_test() -> TestResult {
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let file = fixture.dir.path().join("a.png");
        write_png(&file, 600, 300)?;
        let before = fixture.generator.generate(&nested(&file), &thumbnail_request(256, 256)).await?;

        write_png(&file, 300, 600)?;
        tokio::time::sleep(Duration::from_millis(20)).await;
        let after = fixture.generator.generate(&nested(&file), &thumbnail_request(256, 256)).await?;

        assert_ne!(before.key, after.key);
        assert_eq!(size(&before)?, (256, 128));
        assert_eq!(size(&after)?, (128, 256));
        assert_eq!(fixture.generator.conversions(), 2);
        Ok(())
    }

    #[tokio::test]
    async fn rejects_invalid_requests_and_unsupported_files_test() -> TestResult {
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let png = fixture.dir.path().join("a.png");
        write_png(&png, 100, 100)?;
        let text = fixture.dir.path().join("a.txt");
        std::fs::write(&text, b"text")?;
        let broken = fixture.dir.path().join("broken.png");
        std::fs::write(&broken, b"this is not a png")?;

        let invalid = [
            request(ThumbnailVariant::Thumbnail, Some(100), Some(100), ImageFormatType::Jpeg, ImageResizeType::Pad),
            request(ThumbnailVariant::Thumbnail, None, None, ImageFormatType::Jpeg, ImageResizeType::Fit),
            request(ThumbnailVariant::Sprite, Some(100), Some(100), ImageFormatType::Jpeg, ImageResizeType::Fit),
        ];
        for request in invalid {
            let error = fixture.generator.generate(&nested(&png), &request).await.unwrap_err();
            assert_eq!(*error.kind(), ErrorKind::InvalidInput, "{request:?}");
        }

        for path in [&text, &broken] {
            let error = fixture.generator.generate(&nested(path), &thumbnail_request(256, 256)).await.unwrap_err();
            assert_eq!(*error.kind(), ErrorKind::UnsupportedType, "{path:?}");
        }
        assert_eq!(fixture.generator.conversions(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn thumbnails_inside_an_archive_are_cached_by_the_entry_key_test() -> TestResult {
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let mut png = Vec::new();
        RgbaImage::from_pixel(600, 300, Rgba([10, 20, 30, 255])).write_to(&mut Cursor::new(&mut png), ImageFormat::Png)?;
        let archive = fixture.dir.path().join("a.7z");
        let mut writer = sevenz_rust2::ArchiveWriter::create(&archive)?;
        writer.push_archive_entry(sevenz_rust2::ArchiveEntry::new_file("dir/x.png"), Some(Cursor::new(png)))?;
        writer.finish()?;

        let mut inside = nested(&archive);
        inside.push_str("dir/x.png")?;
        let first = fixture.generator.generate(&inside, &thumbnail_request(256, 256)).await?;
        let second = fixture.generator.generate(&inside, &thumbnail_request(256, 256)).await?;
        assert_eq!(size(&first)?, (256, 128));
        assert_eq!(first, second);
        assert_eq!(fixture.generator.conversions(), 1);
        Ok(())
    }

    /// 滑らかではない画素の PNG を置き、`request` で生成した画像の大きさを返す。
    async fn encoded_len(options: ThumbnailOptions, request: &ThumbnailRequest) -> TestResult<usize> {
        let fixture = fixture(options).await?;
        let file = fixture.dir.path().join("noisy.png");
        let mut output = Vec::new();
        RgbaImage::from_fn(256, 256, |x, y| {
            let v = x.wrapping_mul(2654435761).wrapping_add(y.wrapping_mul(40503)) >> 8;
            Rgba([v as u8, (v >> 3) as u8, (v >> 5) as u8, 255])
        })
        .write_to(&mut Cursor::new(&mut output), ImageFormat::Png)?;
        std::fs::write(&file, output)?;
        Ok(fixture.generator.generate(&nested(&file), request).await?.bytes.len())
    }

    #[tokio::test]
    async fn quality_changes_the_key_of_lossy_formats_only_test() -> TestResult {
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let plan = |format, quality| -> TestResult<Plan> {
            let request = request(ThumbnailVariant::Thumbnail, Some(200), Some(200), format, ImageResizeType::Fit);
            let mut plan = fixture.generator.inner.plan(&request)?;
            plan.convert.quality = quality;
            Ok(plan)
        };
        let source = SourceId::Local {
            path: "/photos/a.png".to_string(),
            len: 1,
            modified_secs: 1,
            modified_nanos: 0,
        };
        let key = |plan: Plan| plan.key(&source, ThumbnailVariant::Thumbnail, MediaKind::Image);
        for format in [ImageFormatType::Webp, ImageFormatType::Jpeg] {
            assert_ne!(key(plan(format, 30)?), key(plan(format, 90)?), "{format}");
            assert_eq!(key(plan(format, 30)?), key(plan(format, 30)?), "{format}");
        }
        assert_eq!(key(plan(ImageFormatType::Png, 30)?), key(plan(ImageFormatType::Png, 90)?), "PNG は品質を使わない");
        Ok(())
    }

    #[tokio::test]
    async fn quality_and_display_quality_apply_to_their_own_variants_test() -> TestResult {
        let lens = |options: ThumbnailOptions| async move {
            let request = |variant| request(variant, Some(256), Some(256), ImageFormatType::Webp, ImageResizeType::Fit);
            let thumbnail = encoded_len(options.clone(), &request(ThumbnailVariant::Thumbnail)).await?;
            let display = encoded_len(options, &request(ThumbnailVariant::Display)).await?;
            TestResult::Ok((thumbnail, display))
        };
        let base = lens(ThumbnailOptions::default()).await?;
        let low_quality = lens(ThumbnailOptions {
            quality: 20,
            ..ThumbnailOptions::default()
        })
        .await?;
        let low_display = lens(ThumbnailOptions {
            display_quality: 20,
            ..ThumbnailOptions::default()
        })
        .await?;
        assert!(
            low_quality.0 < base.0 && low_quality.1 == base.1,
            "quality はサムネイルだけを変える: {base:?} {low_quality:?}"
        );
        assert!(
            low_display.1 < base.1 && low_display.0 == base.0,
            "display_quality は表示用画像だけを変える: {base:?} {low_display:?}"
        );
        Ok(())
    }

    #[test]
    fn rejects_invalid_options_test() {
        let invalid = [
            ThumbnailOptions {
                sizes: vec![],
                ..ThumbnailOptions::default()
            },
            ThumbnailOptions {
                cache_capacity_bytes: 0,
                ..ThumbnailOptions::default()
            },
            ThumbnailOptions {
                max_concurrent_images: 0,
                ..ThumbnailOptions::default()
            },
            ThumbnailOptions {
                max_concurrent_videos: 0,
                ..ThumbnailOptions::default()
            },
            ThumbnailOptions {
                video_timeout: Duration::ZERO,
                ..ThumbnailOptions::default()
            },
            ThumbnailOptions {
                format: ImageFormatType::Unknown,
                ..ThumbnailOptions::default()
            },
            ThumbnailOptions {
                quality: 0,
                ..ThumbnailOptions::default()
            },
            ThumbnailOptions {
                display_quality: 101,
                ..ThumbnailOptions::default()
            },
        ];
        for options in invalid {
            assert_eq!(*options.validate().unwrap_err().kind(), ErrorKind::InvalidInput, "{options:?}");
        }
    }

    fn write_7z(path: &Path, entries: &[(&str, &[u8])]) -> TestResult {
        let mut writer = sevenz_rust2::ArchiveWriter::create(path)?;
        for (name, content) in entries {
            writer.push_archive_entry(sevenz_rust2::ArchiveEntry::new_file(name), Some(Cursor::new(content.to_vec())))?;
        }
        writer.finish()?;
        Ok(())
    }

    fn png_bytes(width: u32, height: u32) -> TestResult<Vec<u8>> {
        let mut output = Vec::new();
        RgbaImage::from_pixel(width, height, Rgba([10, 20, 30, 255])).write_to(&mut Cursor::new(&mut output), ImageFormat::Png)?;
        Ok(output)
    }

    #[tokio::test]
    async fn folder_returns_the_first_image_in_natural_order_test() -> TestResult {
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let album = fixture.dir.path().join("album");
        std::fs::create_dir_all(album.join("sub"))?;
        // 連番は数値として比べるので、最初は 1.png になる。隠し file と、サブフォルダの中は選ばない。
        write_png(&album.join("10.png"), 100, 100)?;
        write_png(&album.join("2.png"), 200, 100)?;
        write_png(&album.join("1.png"), 300, 100)?;
        write_png(&album.join(".0.png"), 50, 50)?;
        write_png(&album.join("sub/0.png"), 50, 50)?;
        std::fs::write(album.join("0.txt"), b"text")?;

        let request = thumbnail_request(512, 512);
        let representative = fixture.generator.generate(&nested(&album), &request).await?;
        let first = fixture.generator.generate(&nested(&album.join("1.png")), &request).await?;
        assert_eq!(representative, first, "代表は 1.png のサムネイルで、キャッシュも共有する");
        assert_eq!(size(&representative)?, (300, 100));
        assert_eq!(fixture.generator.conversions(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn cached_representative_disappearance_and_replaced_symlink_reselect_once_test() -> TestResult {
        use std::time::UNIX_EPOCH;
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let album = fixture.dir.path().join("album");
        std::fs::create_dir(&album)?;
        write_png(&album.join("1.png"), 100, 200)?;
        write_png(&album.join("2.png"), 200, 100)?;
        let set_old = || std::fs::File::open(&album)?.set_times(std::fs::FileTimes::new().set_modified(UNIX_EPOCH + Duration::from_secs(100)));
        set_old()?;
        let request = thumbnail_request(128, 128);
        fixture.generator.generate(&nested(&album), &request).await?;
        assert_eq!(fixture.generator.inner.selection.scan_count(), 1);
        std::fs::remove_file(album.join("1.png"))?;
        set_old()?; // 同じ版の cache にある候補が消えた場合も、属性を再確認する。
        let next = fixture.generator.generate(&nested(&album), &request).await?;
        assert_eq!(size(&next)?, (128, 64));
        assert_eq!(fixture.generator.inner.selection.scan_count(), 2);
        #[cfg(unix)]
        {
            std::fs::remove_file(album.join("2.png"))?;
            std::os::unix::fs::symlink("../target.png", album.join("2.png"))?;
            write_png(&fixture.dir.path().join("target.png"), 100, 100)?;
            set_old()?;
            let error = fixture.generator.generate(&nested(&album), &request).await.err().unwrap();
            assert_eq!(error.kind(), &ErrorKind::NotFound);
            assert_eq!(fixture.generator.inner.selection.scan_count(), 3);
        }
        Ok(())
    }

    #[tokio::test]
    async fn folder_without_an_image_is_not_found_test() -> TestResult {
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let folder = fixture.dir.path().join("docs");
        std::fs::create_dir_all(folder.join("sub"))?;
        std::fs::write(folder.join("a.txt"), b"text")?;
        write_png(&folder.join("sub/a.png"), 100, 100)?;

        let error = fixture.generator.generate(&nested(&folder), &thumbnail_request(256, 256)).await.unwrap_err();
        assert_eq!(*error.kind(), ErrorKind::NotFound, "直下に画像がなければ、サブフォルダの中は見ない");
        Ok(())
    }

    #[tokio::test]
    async fn archive_and_its_folders_return_a_representative_test() -> TestResult {
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let archive = fixture.dir.path().join("book.7z");
        write_7z(
            &archive,
            &[
                ("cover.png", &png_bytes(400, 200)?),
                ("readme.txt", b"text"),
                ("dir/10.png", &png_bytes(100, 100)?),
                ("dir/2.png", &png_bytes(200, 100)?),
            ],
        )?;
        let request = thumbnail_request(512, 512);

        let root = fixture.generator.generate(&nested(&archive), &request).await?;
        assert_eq!(size(&root)?, (400, 200), "archive の path は、直下の代表を返す");

        let mut dir = nested(&archive);
        dir.push_str("dir")?;
        let inside = fixture.generator.generate(&dir, &request).await?;
        assert_eq!(size(&inside)?, (200, 100), "archive の内側のフォルダも、自然順の最初の 2.png を返す");

        let scans = fixture.generator.inner.selection.scan_count();
        assert_eq!(fixture.generator.generate(&nested(&archive), &request).await?, root);
        assert_eq!(fixture.generator.inner.selection.scan_count(), scans, "同じ archive 版の選択を再利用する");
        write_7z(&archive, &[("new.png", &png_bytes(120, 90)?)])?;
        let changed = fixture.generator.generate(&nested(&archive), &request).await?;
        assert_eq!(size(&changed)?, (120, 90));
        assert_eq!(fixture.generator.inner.selection.scan_count(), scans + 1);

        let empty = fixture.dir.path().join("only-text.7z");
        write_7z(&empty, &[("a.txt", b"text")])?;
        let error = fixture.generator.generate(&nested(&empty), &request).await.unwrap_err();
        assert_eq!(*error.kind(), ErrorKind::NotFound);
        Ok(())
    }

    /// ローカルでは ffmpeg のない環境を許すが、CI では動画の検証を必須にする。
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

    fn write_video(path: &Path, seconds: u32, width: u32, height: u32) -> TestResult {
        let source = format!("testsrc=duration={seconds}:size={width}x{height}:rate=10");
        let status = std::process::Command::new("ffmpeg")
            .args(["-v", "error", "-y", "-f", "lavfi", "-i", &source, "-c:v", "mpeg4", "-pix_fmt", "yuv420p"])
            .arg(path)
            .status()?;
        assert!(status.success(), "ffmpeg failed to create a test video");
        Ok(())
    }

    fn sprite_request(width: u32, height: u32) -> ThumbnailRequest {
        request(ThumbnailVariant::Sprite, Some(width), Some(height), ImageFormatType::Jpeg, ImageResizeType::Fit)
    }

    #[tokio::test]
    async fn video_thumbnail_and_sprite_come_from_one_ffmpeg_run_test() -> TestResult {
        if skip_without_ffmpeg() {
            return Ok(());
        }
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let video = fixture.dir.path().join("clip.mp4");
        write_video(&video, 10, 320, 240)?;

        let sprite = fixture.generator.generate(&nested(&video), &sprite_request(128, 128)).await?;
        assert_eq!(sprite.frame_count, Some(8));
        assert_eq!(size(&sprite)?, (8 * 128, 96), "フレームが横に 8 枚並び、各フレームは 128 の枠に縦横比を保って収まる");

        let thumbnail = fixture.generator.generate(&nested(&video), &thumbnail_request(128, 128)).await?;
        assert_eq!(thumbnail.frame_count, None);
        assert_eq!(size(&thumbnail)?, (128, 96));
        assert_eq!(fixture.generator.ffmpeg_runs(), 1, "スプライトシートを作ったときに、サムネイルも入っている");
        assert_ne!(sprite.key, thumbnail.key);
        Ok(())
    }

    #[tokio::test]
    async fn media_information_is_shared_and_invalidated_for_local_and_archive_test() -> TestResult {
        if skip_without_ffmpeg() {
            return Ok(());
        }
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let video = fixture.dir.path().join("info.mp4");
        write_video(&video, 3, 160, 120)?;
        let source = fixture.generator.inner.fs.resolve_entry(&nested(&video)).await?;
        let media = &fixture.generator.inner.media_info;
        let (a, b) = tokio::join!(media.probe_video(&source), media.probe_video(&source));
        assert_eq!(a?.duration_secs, b?.duration_secs);
        assert_eq!(media.probe_count(), 1);
        fixture.generator.generate(&nested(&video), &thumbnail_request(128, 128)).await?;
        assert_eq!(media.probe_count(), 1, "thumbnail reuses the playback probe");
        write_video(&video, 4, 160, 120)?;
        assert_eq!(media.probe_video(&source).await?.duration_secs, 4.0);
        assert_eq!(media.probe_count(), 2);
        let archive = fixture.dir.path().join("info.7z");
        write_7z(&archive, &[("clip.mp4", &std::fs::read(&video)?)])?;
        let mut inside = nested(&archive);
        inside.push_str("clip.mp4")?;
        let source = fixture.generator.inner.fs.resolve_entry(&inside).await?;
        media.probe_video(&source).await?;
        fixture.generator.generate(&inside, &thumbnail_request(128, 128)).await?;
        assert_eq!(media.probe_count(), 3, "archive media and thumbnail share the same entry key");
        Ok(())
    }

    #[tokio::test]
    async fn media_information_of_an_image_reads_the_size_and_exif_without_ffmpeg_test() -> TestResult {
        let fixture = fixture_with_media(
            ThumbnailOptions::default(),
            MediaOptions {
                ffmpeg_path: "/nonexistent/ffmpeg".into(),
                ffprobe_path: "/nonexistent/ffprobe".into(),
                heif_dec_path: "/nonexistent/heif-dec".into(),
            },
        )
        .await?;
        let media = &fixture.generator.inner.media_info;
        let png = fixture.dir.path().join("wide.PNG");
        write_png(&png, 40, 20)?;
        let source = fixture.generator.inner.fs.resolve_entry(&nested(&png)).await?;
        let MediaInfo::Image(info) = media.probe(&source).await? else {
            return Err("an image should give image information".into());
        };
        assert_eq!((info.content_type.as_str(), info.width, info.height), ("image/png", 40, 20));
        assert_eq!(info.exif, ImageExif::default());
        assert_eq!(
            *media.probe_video(&source).await.unwrap_err().kind(),
            ErrorKind::UnsupportedType,
            "画像は動画としては読めない"
        );

        let broken = fixture.dir.path().join("broken.png");
        std::fs::write(&broken, b"not an image")?;
        let source = fixture.generator.inner.fs.resolve_entry(&nested(&broken)).await?;
        assert_eq!(*media.probe(&source).await.unwrap_err().kind(), ErrorKind::UnsupportedType);

        let archive = fixture.dir.path().join("pics.7z");
        write_7z(&archive, &[("inner.png", &png_bytes(30, 10)?)])?;
        let mut inside = nested(&archive);
        inside.push_str("inner.png")?;
        let source = fixture.generator.inner.fs.resolve_entry(&inside).await?;
        let MediaInfo::Image(info) = media.probe(&source).await? else {
            return Err("an image in an archive should give image information".into());
        };
        assert_eq!((info.width, info.height), (30, 10));
        Ok(())
    }

    #[tokio::test]
    async fn media_information_of_an_ffmpeg_image_uses_ffprobe_test() -> TestResult {
        if skip_without_ffmpeg() {
            return Ok(());
        }
        let fixture = fixture(ThumbnailOptions::default()).await?;
        // ffprobe は拡張子ではなく中身で形式を決めるので、PNG の中身を HEIC の名前で置いて、ffprobe の経路を通す。
        let heic = fixture.dir.path().join("IMG_0001.HEIC");
        write_png(&heic, 64, 48)?;
        let source = fixture.generator.inner.fs.resolve_entry(&nested(&heic)).await?;
        let MediaInfo::Image(info) = fixture.generator.inner.media_info.probe(&source).await? else {
            return Err("an ffmpeg image should give image information".into());
        };
        assert_eq!((info.content_type.as_str(), info.width, info.height), ("image/heic", 64, 48));
        Ok(())
    }

    #[tokio::test]
    async fn the_rotation_of_an_ffmpeg_image_is_returned_apart_from_its_stored_size_test() -> TestResult {
        if skip_without_ffmpeg() {
            return Ok(());
        }
        let fixture = fixture(ThumbnailOptions::default()).await?;
        // 符号化された寸法は 64×32 で、`irot` が 90 度と 180 度の AVIF。幅と高さは入れ替えず、回転だけを返す。
        let rotated: [(&str, &[u8], u16); 2] = [
            ("rot90.avif", include_bytes!("../../../tests/fixtures/rot90.avif"), 90),
            ("rot180.avif", include_bytes!("../../../tests/fixtures/rot180.avif"), 180),
        ];
        for (name, bytes, _) in rotated {
            std::fs::write(fixture.dir.path().join(name), bytes)?;
        }
        // 確かめた範囲では、ffmpeg 8.1.1 は `irot` を読み、5.1、6.1、7.1 は回転を返さず、デコードでも回さない。
        // 返す回転は、サムネイルを作るデコードが適用する回転と一致していればよいので、90 度の画像のサムネイルが縦長かで期待値を決める。
        let (width, height) = size(
            &fixture
                .generator
                .generate(&nested(&fixture.dir.path().join("rot90.avif")), &thumbnail_request(128, 128))
                .await?,
        )?;
        let applies_irot = height > width;
        for (name, _, rotation) in rotated {
            let source = fixture.generator.inner.fs.resolve_entry(&nested(&fixture.dir.path().join(name))).await?;
            let MediaInfo::Image(info) = fixture.generator.inner.media_info.probe(&source).await? else {
                return Err("a rotated AVIF must be recognized as an image".into());
            };
            let expected = if applies_irot { rotation } else { 0 };
            assert_eq!((info.width, info.height, info.rotation), (64, 32, expected), "{name} (applies irot: {applies_irot})");
        }
        Ok(())
    }

    #[tokio::test]
    async fn concurrent_thumbnail_and_sprite_share_one_ffmpeg_run_test() -> TestResult {
        if skip_without_ffmpeg() {
            return Ok(());
        }
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let video = fixture.dir.path().join("clip.mp4");
        write_video(&video, 10, 320, 240)?;

        let (path, thumbnail_request, sprite_request) = (nested(&video), thumbnail_request(128, 128), sprite_request(128, 128));
        let (thumbnail, sprite) = tokio::join!(fixture.generator.generate(&path, &thumbnail_request), fixture.generator.generate(&path, &sprite_request),);
        assert_eq!(size(&thumbnail?)?, (128, 96));
        assert_eq!(sprite?.frame_count, Some(8));
        assert_eq!(fixture.generator.ffmpeg_runs(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn conditions_that_differ_need_another_ffmpeg_run_test() -> TestResult {
        if skip_without_ffmpeg() {
            return Ok(());
        }
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let video = fixture.dir.path().join("clip.mp4");
        write_video(&video, 10, 320, 240)?;

        fixture.generator.generate(&nested(&video), &sprite_request(128, 128)).await?;
        fixture.generator.generate(&nested(&video), &thumbnail_request(256, 256)).await?;
        assert_eq!(fixture.generator.ffmpeg_runs(), 2, "幅と高さが違うサムネイルは、スプライトシートのキャッシュに当たらない");
        Ok(())
    }

    #[tokio::test]
    async fn video_inside_an_archive_test() -> TestResult {
        if skip_without_ffmpeg() {
            return Ok(());
        }
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let video = fixture.dir.path().join("clip.mp4");
        write_video(&video, 5, 160, 120)?;
        let archive = fixture.dir.path().join("videos.7z");
        write_7z(&archive, &[("dir/clip.mp4", &std::fs::read(&video)?)])?;

        let mut inside = nested(&archive);
        inside.push_str("dir/clip.mp4")?;
        let sprite = fixture.generator.generate(&inside, &sprite_request(128, 128)).await?;
        assert_eq!(sprite.frame_count, Some(8));

        let mut folder = nested(&archive);
        folder.push_str("dir")?;
        let representative = fixture.generator.generate(&folder, &sprite_request(128, 128)).await?;
        assert_eq!(representative, sprite, "フォルダの代表が動画のときも、同じキャッシュに当たる");
        assert_eq!(fixture.generator.ffmpeg_runs(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn display_of_a_video_uses_one_frame_test() -> TestResult {
        if skip_without_ffmpeg() {
            return Ok(());
        }
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let video = fixture.dir.path().join("clip.mp4");
        write_video(&video, 10, 320, 240)?;

        let display = request(ThumbnailVariant::Display, None, None, ImageFormatType::Jpeg, ImageResizeType::Fit);
        let image = fixture.generator.generate(&nested(&video), &display).await?;
        assert_eq!(size(&image)?, (320, 240), "表示用画像は元の大きさを超えない");
        assert_eq!(image.frame_count, None);
        Ok(())
    }

    #[tokio::test]
    async fn an_image_that_ffmpeg_reads_is_thumbnailed_test() -> TestResult {
        if skip_without_ffmpeg() {
            return Ok(());
        }
        let fixture = fixture(ThumbnailOptions::default()).await?;
        // ffmpeg で読むと定めた拡張子でも、中身は ffmpeg が内容から判定するので、PNG を JPEG XL の名前で置いて経路を確かめる。
        let file = fixture.dir.path().join("photo.jxl");
        write_png(&file, 600, 300)?;

        let thumbnail = fixture.generator.generate(&nested(&file), &thumbnail_request(256, 256)).await?;
        assert_eq!(size(&thumbnail)?, (256, 128));
        assert_eq!(fixture.generator.ffmpeg_runs(), 1);
        assert_eq!(fixture.generator.heif_dec_runs(), 0);
        Ok(())
    }

    /// macOS の `sips` が書いた、512×512 のタイル 6 枚で 1280×960 を符号化した HEIC と、その `irot` の角度を 90 度に書き換えた HEIC。
    /// 作り方は docs/knowledge/ffmpeg-heif-tile-grid.md にある。
    /// 名前と、組み立てて回転を反映した後の幅と高さを返す。
    fn write_tiled_heics(dir: &Path) -> TestResult<Vec<(std::path::PathBuf, (u32, u32))>> {
        let files = [
            ("grid.heic", include_bytes!("../../../tests/fixtures/grid.heic").as_slice(), (1280, 960)),
            ("grid-rot90.heic", include_bytes!("../../../tests/fixtures/grid-rot90.heic").as_slice(), (960, 1280)),
        ];
        let mut written = Vec::new();
        for (name, bytes, displayed) in files {
            let path = dir.join(name);
            std::fs::write(&path, bytes)?;
            written.push((path, displayed));
        }
        Ok(written)
    }

    /// PATH にある libheif のデコーダ。1.17 より前の名前の heif-convert も、同じ引数で使える。
    /// ローカルでは libheif のない環境を許すが、CI では HEIF の検証を必須にする。
    fn heif_decoder() -> Option<&'static str> {
        let found = ["heif-dec", "heif-convert"].into_iter().find(|program| {
            std::process::Command::new(program)
                .arg("--version")
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
        });
        if found.is_none() {
            assert!(std::env::var_os("PXNA_REQUIRE_HEIF_DEC").is_none(), "heif-dec or heif-convert is required in CI");
            eprintln!("skipped: neither heif-dec nor heif-convert is available on PATH");
        }
        found
    }

    /// PATH にある ffmpeg が指定した版より前なら、確かめずに飛ばす。版を読めない build も飛ばす。
    /// 確かめた範囲では、タイルを組み立てて読むのは 8.1 と 9.0 で、ffprobe がタイルを束ねた画像の回転を返すのは 9.0 だけである。
    fn skip_before_ffmpeg(required: u32) -> bool {
        if skip_without_ffmpeg() {
            return true;
        }
        let output = std::process::Command::new("ffmpeg")
            .arg("-version")
            .output()
            .map(|output| output.stdout)
            .unwrap_or_default();
        let major = String::from_utf8_lossy(&output)
            .strip_prefix("ffmpeg version ")
            .map(|rest| rest.trim_start_matches('n').chars().take_while(char::is_ascii_digit).collect::<String>())
            .and_then(|digits| digits.parse::<u32>().ok());
        let supported = major.is_some_and(|major| major >= required);
        if !supported {
            eprintln!("skipped: ffmpeg {major:?} is older than {required}");
        }
        !supported
    }

    #[tokio::test]
    async fn a_tiled_heic_is_decoded_whole_by_ffmpeg_test() -> TestResult {
        if skip_before_ffmpeg(9) {
            return Ok(());
        }
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let display = request(ThumbnailVariant::Display, None, None, ImageFormatType::Jpeg, ImageResizeType::Fit);
        for (file, displayed) in write_tiled_heics(fixture.dir.path())? {
            let image = fixture.generator.generate(&nested(&file), &display).await?;
            assert_eq!(size(&image)?, displayed, "{}: タイル 1 枚ではなく、組み立てて回転した画像になる", file.display());
        }
        assert_eq!(fixture.generator.heif_dec_runs(), 0, "アルファのない HEIC は ffmpeg だけで読む");
        assert_eq!(fixture.generator.ffmpeg_runs(), 2);
        Ok(())
    }

    #[tokio::test]
    async fn a_heic_is_read_by_heif_dec_when_ffmpeg_fails_test() -> TestResult {
        let Some(decoder) = heif_decoder() else {
            return Ok(());
        };
        let media = MediaOptions {
            ffmpeg_path: "/nonexistent/ffmpeg".into(),
            heif_dec_path: decoder.into(),
            ..MediaOptions::default()
        };
        let fixture = fixture_with_media(ThumbnailOptions::default(), media).await?;
        let display = request(ThumbnailVariant::Display, None, None, ImageFormatType::Jpeg, ImageResizeType::Fit);
        for (file, displayed) in write_tiled_heics(fixture.dir.path())? {
            let image = fixture.generator.generate(&nested(&file), &display).await?;
            assert_eq!(size(&image)?, displayed, "{}: ffmpeg が読めないときは heif-dec で読み直す", file.display());
        }
        assert_eq!(fixture.generator.heif_dec_runs(), 2);
        assert_eq!(fixture.generator.ffmpeg_runs(), 2);
        Ok(())
    }

    #[tokio::test]
    async fn a_heic_is_read_by_heif_dec_when_the_alpha_probe_fails_test() -> TestResult {
        let Some(decoder) = heif_decoder() else {
            return Ok(());
        };
        let media = MediaOptions {
            ffprobe_path: "/nonexistent/ffprobe".into(),
            heif_dec_path: decoder.into(),
            ..MediaOptions::default()
        };
        let fixture = fixture_with_media(ThumbnailOptions::default(), media).await?;
        let display = request(ThumbnailVariant::Display, None, None, ImageFormatType::Jpeg, ImageResizeType::Fit);
        for (file, displayed) in write_tiled_heics(fixture.dir.path())? {
            let image = fixture.generator.generate(&nested(&file), &display).await?;
            assert_eq!(size(&image)?, displayed, "{}: アルファの判定に失敗したら heif-dec で読む", file.display());
        }
        assert_eq!(fixture.generator.heif_dec_runs(), 2);
        assert_eq!(fixture.generator.ffmpeg_runs(), 0);
        Ok(())
    }

    /// 赤の左半分が不透明で、右半分が透明な画像。HEIC と AVIF、それぞれタイルなしとタイルありを返す。
    fn write_alpha_images(dir: &Path) -> TestResult<Vec<(std::path::PathBuf, (u32, u32))>> {
        let files = [
            ("alpha.heic", include_bytes!("../../../tests/fixtures/alpha.heic").as_slice(), (512, 512)),
            ("alpha-grid.heic", include_bytes!("../../../tests/fixtures/alpha-grid.heic").as_slice(), (1024, 1024)),
            ("alpha.avif", include_bytes!("../../../tests/fixtures/alpha.avif").as_slice(), (512, 512)),
            ("alpha-grid.avif", include_bytes!("../../../tests/fixtures/alpha-grid.avif").as_slice(), (1024, 1024)),
        ];
        let mut written = Vec::new();
        for (name, bytes, displayed) in files {
            let path = dir.join(name);
            std::fs::write(&path, bytes)?;
            written.push((path, displayed));
        }
        Ok(written)
    }

    #[tokio::test]
    async fn an_image_with_alpha_is_read_by_heif_dec_test() -> TestResult {
        if skip_without_ffmpeg() {
            return Ok(());
        }
        let Some(decoder) = heif_decoder() else {
            return Ok(());
        };
        let media = MediaOptions {
            heif_dec_path: decoder.into(),
            ..MediaOptions::default()
        };
        let fixture = fixture_with_media(ThumbnailOptions::default(), media).await?;
        let display = request(ThumbnailVariant::Display, None, None, ImageFormatType::Webp, ImageResizeType::Fit);
        for (file, displayed) in write_alpha_images(fixture.dir.path())? {
            let image = fixture.generator.generate(&nested(&file), &display).await?;
            let decoded = ImageConverter::decode(&image.bytes)?;
            assert_eq!((decoded.width(), decoded.height()), displayed, "{}: 組み立てと回転を反映する", file.display());
            let rgba = decoded.to_rgba8();
            assert_eq!(rgba.get_pixel(decoded.width() / 4, decoded.height() / 2)[3], 255, "{}: 不透明な左半分", file.display());
            assert_eq!(rgba.get_pixel(decoded.width() * 3 / 4, decoded.height() / 2)[3], 0, "{}: 透明な右半分", file.display());
        }
        assert_eq!(fixture.generator.heif_dec_runs(), 4, "アルファ付きは heif-dec に回す");
        assert_eq!(fixture.generator.ffmpeg_runs(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn a_jxl_with_alpha_keeps_it_on_the_ffmpeg_png_path_test() -> TestResult {
        if skip_without_ffmpeg() {
            return Ok(());
        }
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let file = fixture.dir.path().join("alpha.jxl");
        std::fs::write(&file, include_bytes!("../../../tests/fixtures/alpha.jxl"))?;
        let display = request(ThumbnailVariant::Display, None, None, ImageFormatType::Webp, ImageResizeType::Fit);
        let image = fixture.generator.generate(&nested(&file), &display).await?;
        let decoded = ImageConverter::decode(&image.bytes)?;
        let rgba = decoded.to_rgba8();
        assert_eq!((rgba.width(), rgba.height()), (512, 512));
        assert_eq!(rgba.get_pixel(rgba.width() / 4, rgba.height() / 2)[3], 255, "不透明な左半分");
        assert_eq!(rgba.get_pixel(rgba.width() * 3 / 4, rgba.height() / 2)[3], 0, "透明な右半分");
        assert_eq!(fixture.generator.heif_dec_runs(), 0);
        assert_eq!(fixture.generator.ffmpeg_runs(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn an_avif_is_thumbnailed_through_ppm_test() -> TestResult {
        if skip_without_ffmpeg() {
            return Ok(());
        }
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let file = fixture.dir.path().join("blue.avif");
        std::fs::write(&file, include_bytes!("../../../tests/fixtures/blue.avif"))?;
        let thumbnail = fixture.generator.generate(&nested(&file), &thumbnail_request(256, 256)).await?;
        assert_eq!(size(&thumbnail)?, (64, 32), "元の画像より大きくはしない");
        assert_eq!(fixture.generator.heif_dec_runs(), 0, "アルファのない AVIF は ffmpeg の PPM だけで読む");
        assert_eq!(fixture.generator.ffmpeg_runs(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn media_information_of_a_tiled_heic_is_the_assembled_size_test() -> TestResult {
        if skip_before_ffmpeg(9) {
            return Ok(());
        }
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let expected = [(1280, 960, 0), (1280, 960, 90)];
        for ((file, _), expected) in write_tiled_heics(fixture.dir.path())?.into_iter().zip(expected) {
            let source = fixture.generator.inner.fs.resolve_entry(&nested(&file)).await?;
            let MediaInfo::Image(info) = fixture.generator.inner.media_info.probe(&source).await? else {
                return Err("a HEIC must be recognized as an image".into());
            };
            assert_eq!(
                (info.width, info.height, info.rotation),
                expected,
                "{}: 幅と高さは入れ替えず、回転を別に返す",
                file.display()
            );
            assert_eq!(info.content_type, "image/heic");
        }
        Ok(())
    }

    #[tokio::test]
    async fn supported_image_formats_keep_their_thumbnail_paths_test() -> TestResult {
        if skip_without_ffmpeg() {
            return Ok(());
        }
        let fixture = fixture(ThumbnailOptions::default()).await?;
        for (extension, format) in [("png", ImageFormat::Png), ("jpeg", ImageFormat::Jpeg), ("webp", ImageFormat::WebP)] {
            let file = fixture.dir.path().join(format!("photo.{extension}"));
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(64, 32, Rgba([10, 20, 30, 255])))
                .to_rgb8()
                .save_with_format(&file, format)?;
            let thumbnail = fixture.generator.generate(&nested(&file), &thumbnail_request(128, 128)).await?;
            assert_eq!(size(&thumbnail)?, (64, 32));
        }
        assert_eq!(fixture.generator.ffmpeg_runs(), 0);

        let avif = fixture.dir.path().join("photo.avif");
        std::fs::write(&avif, include_bytes!("../../../tests/fixtures/blue.avif"))?;
        let source = fixture.generator.inner.fs.resolve_entry(&nested(&avif)).await?;
        let MediaInfo::Image(info) = fixture.generator.inner.media_info.probe(&source).await? else {
            return Err("AVIF must be recognized as an image".into());
        };
        assert_eq!((info.width, info.height, info.rotation), (64, 32, 0));
        assert_eq!(info.content_type, "image/avif");
        let thumbnail = fixture.generator.generate(&nested(&avif), &thumbnail_request(128, 128)).await?;
        assert_eq!(size(&thumbnail)?, (64, 32));
        assert_eq!(fixture.generator.ffmpeg_runs(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn content_that_ffmpeg_cannot_read_is_unsupported_test() -> TestResult {
        if skip_without_ffmpeg() {
            return Ok(());
        }
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let fake = fixture.dir.path().join("fake.mp4");
        std::fs::write(&fake, b"this is not a video")?;

        let error = fixture.generator.generate(&nested(&fake), &thumbnail_request(128, 128)).await.unwrap_err();
        assert_eq!(*error.kind(), ErrorKind::UnsupportedType);
        Ok(())
    }

    #[tokio::test]
    async fn an_ffmpeg_run_past_the_deadline_times_out_test() -> TestResult {
        if skip_without_ffmpeg() {
            return Ok(());
        }
        let options = ThumbnailOptions {
            video_timeout: Duration::from_nanos(1),
            ..ThumbnailOptions::default()
        };
        let fixture = fixture(options).await?;
        let video = fixture.dir.path().join("clip.mp4");
        write_video(&video, 5, 160, 120)?;

        let error = fixture.generator.generate(&nested(&video), &thumbnail_request(128, 128)).await.unwrap_err();
        assert_eq!(*error.kind(), ErrorKind::Timeout);
        Ok(())
    }

    #[tokio::test]
    async fn a_missing_ffmpeg_is_an_environment_error_test() -> TestResult {
        let media = MediaOptions {
            ffmpeg_path: "/nonexistent/ffmpeg".into(),
            ffprobe_path: "/nonexistent/ffprobe".into(),
            heif_dec_path: "/nonexistent/heif-dec".into(),
        };
        let fixture = fixture_with_media(ThumbnailOptions::default(), media).await?;
        let video = fixture.dir.path().join("clip.mp4");
        std::fs::write(&video, b"anything")?;
        let png = fixture.dir.path().join("a.png");
        write_png(&png, 100, 100)?;

        let error = fixture.generator.generate(&nested(&video), &thumbnail_request(128, 128)).await.unwrap_err();
        assert_eq!(*error.kind(), ErrorKind::UnexpectedError, "実行ファイルがないのは、利用者のファイルの問題ではない");
        fixture.generator.generate(&nested(&png), &thumbnail_request(128, 128)).await?;
        Ok(())
    }
}
