use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::Arc,
    time::Duration,
};

use image::DynamicImage;
use tokio_util::bytes::Bytes;
use tracing::{Instrument as _, Span, field::Empty};

use crate::{
    core::{
        image::{ImageConvertOptions, ImageConverter, SizeSteps},
        media::{FrameSpec, MediaKind, MediaOptions, MediaTools},
        thumbnail::{
            cache::{CacheBucket, CachedImage, ThumbnailCache, ThumbnailCacheOptions},
            key::{ThumbnailKey, ThumbnailKeyHasher},
            request::{Thumbnail, ThumbnailRequest, ThumbnailVariant},
            source_job::{Created, Outcome, SourceJobs},
        },
        work::{WorkClass, WorkKind, WorkOptions, WorkPermit, WorkPools, WorkScheduler},
    },
    perf_span,
    prelude::*,
    service::{EntryCacheKey, MediaInfo, MediaInfoService, NestedFsEntry, ResolvedStoragePath, StorageService},
};

const GIB: u64 = 1024 * 1024 * 1024;

/// How many of the smallest size steps one generation also produces, in both shapes, for the viewer's filmstrip.
const COMPANION_STEPS: usize = 3;

/// How many times a request places itself in source jobs before it gives up. It places again only when the job it waited for did not leave its image.
const MAX_PLACEMENTS: usize = 3;

/// 動画から取り出すフレームの数と位置。設定ファイルからは指定できず、ここから渡す。
const FRAME_SPEC: FrameSpec = FrameSpec::DEFAULT;

/// 生成の中のストレージ読み取りと外部プロセスの段。要求が Interactive でも Foreground に置く。
///
/// 要求の段は生成の枠だけを決める。読み取りまで Interactive にすると、Interactive は同じ資源で
/// Foreground を保留するので、画面の再検証が一覧の列挙を止めてしまう。
const READ_CLASS: WorkClass = WorkClass::Foreground;

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
        let positive = self.cache_capacity_bytes > 0 && self.display_cache_capacity_bytes > 0 && !self.video_timeout.is_zero();
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
/// 生成は要求を受けたときにだけ行う。同じ画像への要求は、どの接続からのものでも 1 つの source job にまとめ、
/// 1 つの画像を同時に 2 回デコードしない（docs/design/thumbnail.md §7）。
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
    /// The configured default format. Companions use it whatever format the request names.
    format: ImageFormatType,
    quality: u8,
    display_quality: u8,
    pools: WorkPools,
    jobs: Jobs,
    #[cfg(test)]
    probe: ConversionProbe,
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

/// How an output of a source job is made from the decoded source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputShape {
    /// One resized image: the decoded image, or a video's first frame.
    Single,
    /// A video's frames resized and laid out in one row.
    Sprite,
}

/// One image that a source job makes.
#[derive(Debug, Clone)]
struct OutputSpec {
    /// `None` when the source has no cache key: the image is made for its request and not stored.
    key: Option<ThumbnailKey>,
    convert: ImageConvertOptions,
    shape: OutputShape,
    bucket: CacheBucket,
    /// A requested image fails its request when it cannot be made. A companion is dropped with a log.
    requested: bool,
}

/// What a source job made, by key.
#[derive(Debug, Default)]
struct JobOutput {
    images: HashMap<ThumbnailKey, CachedImage>,
    /// The requested images that could not be made.
    errors: HashMap<ThumbnailKey, Error>,
}

impl JobOutput {
    /// `results` is aligned with `specs`.
    fn collect(specs: &[OutputSpec], results: Vec<Result<CachedImage>>) -> Self {
        let mut output = Self::default();
        for (spec, result) in specs.iter().zip(results) {
            let Some(key) = &spec.key else {
                continue;
            };
            match result {
                Ok(image) => {
                    output.images.insert(key.clone(), image);
                }
                Err(error) if spec.requested => {
                    output.errors.insert(key.clone(), error);
                }
                Err(error) => warn!(error = ?error, "failed to make a companion thumbnail, so it is dropped"),
            }
        }
        output
    }

    /// `Error` cannot be cloned, so each item gets a copy with the same kind and message.
    fn error_for(&self, key: &ThumbnailKey) -> Option<Error> {
        self.errors.get(key).map(|error| {
            let message = error.message().map(str::to_string).unwrap_or_else(|| error.kind().to_string());
            Error::new(error.kind().clone()).with_message(message)
        })
    }
}

/// Where a request points: the file whose image it asks for, and the plan and key of that image.
struct Located {
    target: ResolvedStoragePath,
    kind: MediaKind,
    plan: Plan,
    /// `None` for a file that cannot be cached.
    source: Option<SourceId>,
    requested_key: Option<ThumbnailKey>,
}

/// What the items of one source job share: the file, its kind, and which source it is.
struct SourceContext {
    target: ResolvedStoragePath,
    kind: MediaKind,
    source: SourceId,
}

/// Source jobs by the source's key. An item carries its plan, and a job's output is read by key.
type Jobs = SourceJobs<ThumbnailKey, ThumbnailKey, Plan, JobOutput>;

/// What the tests watch of the conversions, and the gate that holds them. A job that has started and reached the gate is decoding its source.
#[cfg(test)]
struct ConversionProbe {
    /// How many requests were placed in source jobs.
    placements: std::sync::atomic::AtomicUsize,
    /// How many conversions reached the gate.
    reached: std::sync::atomic::AtomicUsize,
    /// How many conversions run at this moment, and the most that ran at the same time.
    active: std::sync::atomic::AtomicUsize,
    peak: std::sync::atomic::AtomicUsize,
    /// Conversions pass while it has permits. A test takes them all to hold the conversions that start, and gives them back to let them go.
    gate: tokio::sync::Semaphore,
}

#[cfg(test)]
impl ConversionProbe {
    fn new() -> Self {
        Self {
            placements: std::sync::atomic::AtomicUsize::new(0),
            reached: std::sync::atomic::AtomicUsize::new(0),
            active: std::sync::atomic::AtomicUsize::new(0),
            peak: std::sync::atomic::AtomicUsize::new(0),
            gate: tokio::sync::Semaphore::new(tokio::sync::Semaphore::MAX_PERMITS),
        }
    }

    /// Passes the gate and counts the conversion until the returned guard is dropped.
    async fn enter(&self) -> ConversionGuard<'_> {
        use std::sync::atomic::Ordering::SeqCst;
        self.reached.fetch_add(1, SeqCst);
        // The permit is not kept: the gate only holds conversions while a test has taken every permit.
        drop(self.gate.acquire().await);
        let active = self.active.fetch_add(1, SeqCst) + 1;
        self.peak.fetch_max(active, SeqCst);
        ConversionGuard(&self.active)
    }
}

#[cfg(test)]
struct ConversionGuard<'a>(&'a std::sync::atomic::AtomicUsize);

#[cfg(test)]
impl Drop for ConversionGuard<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Ends the job in the table when its task ends, also when the task panics, so that nothing waits for it forever.
struct JobGuard {
    jobs: Jobs,
    source: ThumbnailKey,
    id: u64,
    finished: bool,
}

impl JobGuard {
    fn finish(&mut self, outcome: Outcome<JobOutput>) {
        self.jobs.finish(&self.source, self.id, outcome);
        self.finished = true;
    }
}

impl Drop for JobGuard {
    fn drop(&mut self) {
        if !self.finished {
            self.finish(Err(Arc::new(Error::new(ErrorKind::TaskError).with_message("the source job ended unexpectedly"))));
        }
    }
}

/// キャッシュの key に含める、元の file を指す値。
#[derive(Clone)]
enum SourceId {
    Local { path: String, len: u64, modified_secs: i64, modified_nanos: u32 },
    Archive(EntryCacheKey),
}

impl SourceId {
    /// The key of the source itself, apart from any conversion. Source jobs are placed by it.
    fn key(&self) -> ThumbnailKey {
        let mut hasher = ThumbnailKeyHasher::new();
        hasher.field(b"source");
        match self {
            Self::Local {
                path,
                len,
                modified_secs,
                modified_nanos,
            } => {
                hasher.field(b"local").field(path.as_bytes()).field(&len.to_be_bytes());
                hasher.field(&modified_secs.to_be_bytes()).field(&modified_nanos.to_be_bytes());
            }
            Self::Archive(key) => {
                hasher.field(b"archive").field(key.to_hex().as_bytes());
            }
        }
        hasher.finish()
    }
}

impl ThumbnailGenerator {
    pub async fn open(fs: Arc<StorageService>, cache_dir: &Path, options: ThumbnailOptions, media: MediaOptions) -> Result<Self> {
        let media_info = Arc::new(MediaInfoService::new(media.clone(), options.video_timeout).with_budget(fs.work_budget().clone()));
        let pools = WorkPools::new(&Arc::new(WorkScheduler::default()), &WorkOptions::default());
        Self::open_with_media(fs, cache_dir, options, media, media_info, pools).await
    }

    pub async fn open_with_media(
        fs: Arc<StorageService>,
        cache_dir: &Path,
        options: ThumbnailOptions,
        media: MediaOptions,
        media_info: Arc<MediaInfoService>,
        pools: WorkPools,
    ) -> Result<Self> {
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
                format: options.format,
                quality: options.quality,
                display_quality: options.display_quality,
                pools,
                jobs: SourceJobs::new(),
                #[cfg(test)]
                probe: ConversionProbe::new(),
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
        self.generate_resolved(self.inner.fs.resolve_entry(path, READ_CLASS).await?, request).await
    }

    /// daemon の境界探索で得た driver を、そのまま生成に使う。
    pub async fn generate_resolved(&self, resolved: ResolvedStoragePath, request: &ThumbnailRequest) -> Result<Thumbnail> {
        let span = perf_span!("thumbnail_generate", variant = ?request.variant, name = Empty, kind = Empty, cache_hit = Empty);
        self.generate_traced(resolved, request).instrument(span).await
    }

    /// The key `generate_resolved` would give the image of this request, without reading the cache or generating anything.
    /// `None` for a file that cannot be cached. The key changes when the file is replaced or the representative of a folder changes,
    /// so a client that holds an image with this key knows whether it is still the current one.
    pub async fn current_key(&self, resolved: ResolvedStoragePath, request: &ThumbnailRequest) -> Result<Option<String>> {
        let located = self.inner.locate(resolved, request).await?;
        Ok(located.requested_key.map(|key| key.hex().to_string()))
    }

    /// `generate_resolved` の本体。結果の分かった field を、外側の `thumbnail_generate` の span に記録する。
    async fn generate_traced(&self, resolved: ResolvedStoragePath, request: &ThumbnailRequest) -> Result<Thumbnail> {
        let inner = &self.inner;
        let Located {
            target,
            kind,
            plan,
            source,
            requested_key,
        } = inner.locate(resolved, request).await?;

        if let Some(key) = &requested_key
            && let Some(hit) = inner.cache.get(key).await?
        {
            Span::current().record("cache_hit", true);
            return Ok(plan.thumbnail(hit, Some(key)));
        }
        Span::current().record("cache_hit", false);

        // 生成枠と同じ画像の生成を待つ時間も含む。生成そのものは `thumbnail_production` が測る。
        let class = request.class;
        let image = async {
            match (source, requested_key.clone()) {
                (Some(source), Some(wanted)) => inner.generate_in_job(Arc::new(SourceContext { target, kind, source }), &plan, wanted, class).await,
                _ => inner.generate_alone(&target, kind, &plan, class).await,
            }
        }
        .instrument(perf_span!("thumbnail_generation"))
        .await?;
        Ok(plan.thumbnail(image, requested_key.as_ref()))
    }

    /// 実際に変換した回数。キャッシュと source job が効いているかを、テストが確かめるために使う。
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
    /// Finds the file a request points to, which is the representative for a folder or an archive, and the plan and key of its image.
    /// Records the file name and its kind in the current span.
    async fn locate(&self, resolved: ResolvedStoragePath, request: &ThumbnailRequest) -> Result<Located> {
        let inner = self;
        let path = resolved.path().to_nested_path_buf();
        let entry = resolved.get_entry(READ_CLASS).await?;
        let (target, entry) = match entry.detail.clone() {
            FsEntryDetail::File { archive: false } => (resolved, entry),
            FsEntryDetail::Dir { .. } | FsEntryDetail::File { archive: true } => {
                let mut folder = path.to_nested_path_buf();
                if matches!(entry.detail, FsEntryDetail::File { .. }) {
                    folder.push_archive_root()?;
                }
                let folder = if folder == path { resolved } else { inner.fs.resolve_entry(&folder, READ_CLASS).await? };
                let mut target = inner.selection.select(&folder, READ_CLASS).await?;
                let entry = async {
                    match target.get_listing_entry(READ_CLASS).await {
                        Ok(entry) if matches!(entry.detail, FsEntryDetail::File { .. }) => Ok(entry),
                        Ok(_) => {
                            inner.selection.invalidate();
                            target = inner.selection.select(&folder, READ_CLASS).await?;
                            target.get_listing_entry(READ_CLASS).await
                        }
                        Err(error) if error.kind() == &ErrorKind::NotFound => {
                            inner.selection.invalidate();
                            target = inner.selection.select(&folder, READ_CLASS).await?;
                            target.get_listing_entry(READ_CLASS).await
                        }
                        Err(error) => Err(error),
                    }
                }
                .instrument(perf_span!("representative_metadata"))
                .await?;
                if !matches!(entry.detail, FsEntryDetail::File { .. }) {
                    return Err(Error::new(ErrorKind::UnsupportedType).with_message("representative is no longer a file"));
                }
                (target, entry)
            }
            FsEntryDetail::Link { .. } => return Err(Error::new(ErrorKind::UnsupportedType).with_message(format!("not a file: {path}"))),
        };

        let name = Inner::file_name(&entry.path);
        let kind = MediaKind::of_name(name).ok_or_else(|| Error::new(ErrorKind::UnsupportedType).with_message(format!("not a supported image or video: {name}")))?;
        Span::current().record("name", name).record("kind", tracing::field::debug(kind));
        if request.variant == ThumbnailVariant::Sprite && kind != MediaKind::Video {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("the sprite variant requires a video"));
        }

        let plan = inner.plan(request)?;
        let source = inner.source_id(&target, &entry);
        let requested_key = source.as_ref().map(|source| plan.key(source, request.variant, kind));
        Ok(Located {
            target,
            kind,
            plan,
            source,
            requested_key,
        })
    }

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

    /// Plans the thumbnails that one decode also produces: the smallest size steps in both shapes, which the viewer's filmstrip asks for,
    /// and, for a thumbnail or a sprite, the other shape at the requested size. The keys exclude the primary, so that it is not made twice.
    /// The caller passes a source that has a cache key; a source without one has no companions.
    fn companion_plans(&self, plan: &Plan, kind: MediaKind, source: &SourceId) -> Vec<(ThumbnailKey, ImageConvertOptions)> {
        let option = |resize_type, width, height| ImageConvertOptions {
            format_type: self.format,
            quality: self.quality,
            resize_type,
            resampler_type: ImageResamplerType::Lanczos3,
            anchor_position_type: ImageAnchorPositionType::Center,
            width,
            height,
        };

        let mut options = Vec::new();
        for step in self.sizes.smallest(COMPANION_STEPS) {
            options.push(option(ImageResizeType::Crop, Some(*step), Some(*step)));
            options.push(option(ImageResizeType::Fit, Some(*step), Some(*step)));
        }
        if matches!(plan.variant, ThumbnailVariant::Thumbnail | ThumbnailVariant::Sprite) {
            let other = match plan.convert.resize_type {
                ImageResizeType::Crop => Some(ImageResizeType::Fit),
                ImageResizeType::Fit => Some(ImageResizeType::Crop),
                _ => None,
            };
            if let Some(other) = other {
                options.push(option(other, plan.convert.width, plan.convert.height));
            }
        }

        // A sprite request makes its thumbnail in the same run, so the primary of a sprite is the thumbnail. The sprite's own key has another tag.
        let primary_variant = if plan.variant == ThumbnailVariant::Sprite {
            ThumbnailVariant::Thumbnail
        } else {
            plan.variant
        };
        let mut seen = HashSet::from([plan.key(source, primary_variant, kind)]);
        let companion_plan = |convert: ImageConvertOptions| Plan {
            variant: ThumbnailVariant::Thumbnail,
            bucket: CacheBucket::Thumbnail,
            convert,
        };
        options
            .into_iter()
            .filter_map(|convert| {
                let key = companion_plan(convert.clone()).key(source, ThumbnailVariant::Thumbnail, kind);
                seen.insert(key.clone()).then_some((key, convert))
            })
            .collect()
    }

    /// Drops the outputs that the cache already has, without reading them: the items that want them read the cache after the job.
    /// A failed lookup keeps the output.
    async fn missing_outputs(&self, specs: Vec<OutputSpec>) -> Vec<OutputSpec> {
        let mut missing = Vec::with_capacity(specs.len());
        for spec in specs {
            if let Some(key) = &spec.key {
                match self.cache.contains(key).await {
                    Ok(true) => continue,
                    Ok(false) => {}
                    Err(error) => warn!(error = ?error, "failed to look up a thumbnail in the cache"),
                }
            }
            missing.push(spec);
        }
        missing
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

    /// 画像の変換と、外部プロセスの ffmpeg と heif-dec は、別の資源として数える。
    /// The ffmpeg runs for videos are a third resource, apart from the image decodes of ffmpeg and heif-dec.
    /// 枠を待つ間に要求が drop されたら、何も始まらない。
    fn pool_for(&self, kind: MediaKind) -> &crate::core::work::WorkPriorityPool {
        match kind {
            MediaKind::Image => &self.pools.cpu,
            MediaKind::FfmpegImage | MediaKind::HeifImage => &self.pools.process_image,
            MediaKind::Video => &self.pools.process_video,
        }
    }

    /// The images a request asks for. A video's thumbnail and sprite sheet are made in one ffmpeg run, so each asks for both.
    /// `source` is `None` for a file that cannot be cached, whose images have no keys.
    fn requested_specs(&self, plan: &Plan, kind: MediaKind, source: Option<&SourceId>) -> Vec<OutputSpec> {
        let key_of = |variant| source.map(|source| plan.key(source, variant, kind));
        let spec = |key, shape, bucket| OutputSpec {
            key,
            convert: plan.convert.clone(),
            shape,
            bucket,
            requested: true,
        };
        if kind == MediaKind::Video && plan.variant != ThumbnailVariant::Display {
            vec![
                spec(key_of(ThumbnailVariant::Thumbnail), OutputShape::Single, plan.bucket),
                spec(key_of(ThumbnailVariant::Sprite), OutputShape::Sprite, CacheBucket::Thumbnail),
            ]
        } else {
            vec![spec(key_of(plan.variant), OutputShape::Single, plan.bucket)]
        }
    }

    /// Fixes what a job makes from its items: every image they ask for, and the companions of each, without duplicates.
    /// Returns the keys of all of them, which later items use to find that this job makes their image.
    fn plan_job(&self, context: &SourceContext, items: &[Plan]) -> (HashSet<ThumbnailKey>, Vec<OutputSpec>) {
        let mut keys = HashSet::new();
        let mut specs = Vec::new();
        for plan in items {
            for spec in self.requested_specs(plan, context.kind, Some(&context.source)) {
                if spec.key.as_ref().is_none_or(|key| keys.insert(key.clone())) {
                    specs.push(spec);
                }
            }
        }
        for plan in items {
            for (key, convert) in self.companion_plans(plan, context.kind, &context.source) {
                if keys.insert(key.clone()) {
                    specs.push(OutputSpec {
                        key: Some(key),
                        convert,
                        shape: OutputShape::Single,
                        bucket: CacheBucket::Thumbnail,
                        requested: false,
                    });
                }
            }
        }
        (keys, specs)
    }

    /// Gets the image of `wanted` through the source job of its source, and makes a job if none can make it.
    /// Dropping the returned future takes the item out of its job; a job that has not started and has no item left never starts.
    async fn generate_in_job(self: &Arc<Self>, context: Arc<SourceContext>, plan: &Plan, wanted: ThumbnailKey, class: WorkClass) -> Result<CachedImage> {
        let source_key = context.source.key();
        for _ in 0..MAX_PLACEMENTS {
            let placed = self.jobs.place(&source_key, &wanted, plan.clone(), class);
            #[cfg(test)]
            self.probe.placements.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let created = placed.created.is_some();
            if let Some(created) = placed.created {
                tokio::spawn(self.clone().run_job(context.clone(), source_key.clone(), created).in_current_span());
            }
            // The time to wait for a job of the source, which the item may have started itself or joined.
            let output = placed.handle.wait().instrument(perf_span!("thumbnail_job_wait", created)).await?;
            if let Some(image) = output.images.get(&wanted) {
                return Ok(image.clone());
            }
            if let Some(error) = output.error_for(&wanted) {
                return Err(error);
            }
            // The job found the image in the cache, or dropped it as a companion, or its store failed.
            if let Some(image) = self.cache.get(&wanted).await? {
                return Ok(image);
            }
        }
        Err(Error::new(ErrorKind::UnexpectedError).with_message("the image was not produced"))
    }

    /// Runs a source job in a task of its own, so that it runs to the end once it has started, whatever happens to its items.
    async fn run_job(self: Arc<Self>, context: Arc<SourceContext>, source_key: ThumbnailKey, created: Created<JobOutput>) {
        let mut guard = JobGuard {
            jobs: self.jobs.clone(),
            source: source_key.clone(),
            id: created.id,
            finished: false,
        };
        let outcome = self.wait_and_produce(&context, &source_key, &created).await;
        guard.finish(outcome);
    }

    async fn wait_and_produce(self: &Arc<Self>, context: &SourceContext, source_key: &ThumbnailKey, created: &Created<JobOutput>) -> Outcome<JobOutput> {
        let discarded = || Arc::new(Error::new(ErrorKind::UnexpectedError).with_message("the source job was discarded"));

        // The next job of a source starts waiting for its slot when the job decoding the source ends.
        if let Some(previous) = &created.previous {
            let _ = previous.wait().await;
        }

        // Wait for the slot with the class of the job. An Interactive item that joins the job raises its ticket meanwhile.
        let pool = self.pool_for(context.kind);
        let Some(class) = self.jobs.class_of(source_key, created.id) else {
            return Err(discarded());
        };
        let ticket = pool.ticket(class).counted(WorkKind::Convert);
        let handle = ticket.handle();
        // An item may have raised the job before the ticket existed, so the ticket takes the job's class when it is attached.
        let Some(class) = self.jobs.attach(source_key, created.id, handle.clone()) else {
            return Err(discarded());
        };
        handle.raise(class);
        let permit = ticket
            .acquire()
            .instrument(perf_span!("thumbnail_permit_wait", class = ?class))
            .await
            .map_err(|_| discarded())?;

        let Some(specs) = self.jobs.start(source_key, created.id, |items| self.plan_job(context, items)) else {
            return Err(discarded());
        };
        self.produce(context, specs, permit)
            .instrument(perf_span!("thumbnail_production"))
            .await
            .map(Arc::new)
            .map_err(Arc::new)
    }

    /// Makes the images of a job and puts them in the cache.
    async fn produce(self: &Arc<Self>, context: &SourceContext, specs: Vec<OutputSpec>, permit: WorkPermit) -> Result<JobOutput> {
        // The images that another job stored meanwhile are not made again. Their items read them from the cache.
        let specs = self.missing_outputs(specs).await;
        if specs.is_empty() {
            return Ok(JobOutput::default());
        }
        let results = self.convert_outputs(&context.target, context.kind, &specs, permit).await?;
        let output = JobOutput::collect(&specs, results);
        self.store(&specs, &output).instrument(perf_span!("thumbnail_store")).await;
        Ok(output)
    }

    /// Gets the image of a file that cannot be cached, for its request alone. If the request is dropped, the generation stops.
    async fn generate_alone(self: &Arc<Self>, target: &ResolvedStoragePath, kind: MediaKind, plan: &Plan, class: WorkClass) -> Result<CachedImage> {
        let permit = self.pool_for(kind).ticket(class).acquire().await?;
        let specs = self.requested_specs(plan, kind, None);
        let results = self.convert_outputs(target, kind, &specs, permit).instrument(perf_span!("thumbnail_production")).await?;
        let wanted = if plan.variant == ThumbnailVariant::Sprite {
            OutputShape::Sprite
        } else {
            OutputShape::Single
        };
        specs
            .iter()
            .zip(results)
            .find_map(|(spec, result)| (spec.shape == wanted).then_some(result))
            .unwrap_or_else(|| Err(Error::new(ErrorKind::UnexpectedError).with_message("the image was not produced")))
    }

    /// Decodes the file once and makes every image of `specs` from it. The results are aligned with `specs`.
    /// An error is a failure of the file, not of one image: the file could not be read or decoded.
    async fn convert_outputs(&self, target: &ResolvedStoragePath, kind: MediaKind, specs: &[OutputSpec], permit: WorkPermit) -> Result<Vec<Result<CachedImage>>> {
        #[cfg(test)]
        let _counted = self.probe.enter().await;
        // 変換が終わるまで、archive 内の file の lease を保つ。
        let file = target.materialize_file(READ_CLASS).instrument(perf_span!("thumbnail_materialize")).await?;
        let results = async {
            Ok::<_, Error>(match kind {
                MediaKind::Image => {
                    let read = perf_span!("thumbnail_source_read", bytes = Empty);
                    let input = tokio::fs::read(file.path()).instrument(read.clone()).await?;
                    read.record("bytes", input.len());
                    self.convert_many(permit, input, specs.to_vec()).await?
                }
                MediaKind::FfmpegImage | MediaKind::HeifImage => {
                    let decode = perf_span!("thumbnail_external_decode", bytes = Empty);
                    let input = self.decode_ffmpeg_image(target, file.path(), kind, READ_CLASS).instrument(decode.clone()).await?;
                    decode.record("bytes", input.len());
                    self.convert_many(permit, input, specs.to_vec()).await?
                }
                MediaKind::Video => self.produce_video(permit, file.path(), target, specs.to_vec()).await?,
            })
        }
        .instrument(perf_span!("thumbnail_convert"))
        .await?;
        #[cfg(test)]
        self.conversions.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        // The conversion is done and its slot went back with the blocking job, so the next generation can start while this one stores.
        Ok(results)
    }

    /// キャッシュへ入れられなくても、生成した画像は返す。
    ///
    /// All entries go in one batch: the companions first, then the requested images, so that a small bucket
    /// evicts the oldest companions and not the images that were just requested.
    /// An entry that exceeds the bucket is skipped, and a failed store is logged. Neither fails the response.
    async fn store(&self, specs: &[OutputSpec], output: &JobOutput) {
        let mut entries: Vec<(&ThumbnailKey, CacheBucket, &CachedImage)> = Vec::new();
        for requested in [false, true] {
            for spec in specs.iter().filter(|spec| spec.requested == requested) {
                if let Some(key) = &spec.key
                    && let Some(image) = output.images.get(key)
                {
                    entries.push((key, spec.bucket, image));
                }
            }
        }

        match self.cache.put_many(&entries).await {
            Ok(stored) => {
                if stored.contains(&false) {
                    debug!("a thumbnail is larger than the cache bucket, so it is not stored");
                }
            }
            Err(error) => warn!(error = ?error, "failed to store thumbnails in the cache"),
        }
    }

    /// Decodes the input once, then resizes and encodes every image of `specs` from that one decoded image.
    /// The results are aligned with `specs`. A decode failure fails the whole call.
    async fn convert_many(&self, permit: WorkPermit, input: Vec<u8>, specs: Vec<OutputSpec>) -> Result<Vec<Result<CachedImage>>> {
        permit
            .run_blocking(move || -> Result<Vec<Result<CachedImage>>> {
                let image = ImageConverter::decode(&input).map_err(Self::decode_error)?;
                Ok(specs
                    .iter()
                    .map(|spec| match spec.shape {
                        OutputShape::Single => Self::convert_decoded(&image, &spec.convert).map_err(Self::decode_error),
                        OutputShape::Sprite => Err(Error::new(ErrorKind::InvalidInput).with_message("a sprite sheet requires a video")),
                    })
                    .collect())
            })
            .await?
    }

    fn convert_decoded(image: &DynamicImage, options: &ImageConvertOptions) -> Result<CachedImage> {
        let resized = ImageConverter::resize_ref(image, options)?;
        Ok(CachedImage {
            bytes: Bytes::from(ImageConverter::encode(&resized, options.format_type, options.quality)?),
            frame_count: None,
        })
    }

    /// 動画のフレームを取り出し、`specs` の画像を作る。結果は `specs` と同じ並びで返す。
    /// サムネイルと表示用画像は最初のフレームから、スプライトシートはすべてのフレームを縮小して横に並べて作る。
    /// スプライトシートがなければ、最初のフレームだけを取り出す。
    ///
    /// Only the first frame is kept at full size, for the single images and the heads of the sprite sheets. The other frames are decoded, resized, and dropped one at a time.
    async fn produce_video(&self, permit: WorkPermit, file: &Path, source: &ResolvedStoragePath, specs: Vec<OutputSpec>) -> Result<Vec<Result<CachedImage>>> {
        let duration = self.media_info.probe_video(source, READ_CLASS).await?.duration_secs;
        let mut positions = FRAME_SPEC.positions(duration);
        if !specs.iter().any(|spec| spec.shape == OutputShape::Sprite) {
            positions.truncate(1);
        }
        let frames = self.extract_frames(file, &positions, READ_CLASS).await?;

        permit
            .run_blocking(move || -> Result<Vec<Result<CachedImage>>> {
                let first = frames
                    .first()
                    .ok_or_else(|| Error::new(ErrorKind::UnexpectedError).with_message("no frame was extracted from the video"))
                    .and_then(|frame| ImageConverter::decode(frame))
                    .map_err(Self::decode_error)?;

                // The head of a sprite sheet is its first frame, resized once and also used for the single image of the same conditions.
                let heads: Vec<Option<Result<DynamicImage>>> = specs
                    .iter()
                    .map(|spec| (spec.shape == OutputShape::Sprite).then(|| ImageConverter::resize_ref(&first, &spec.convert).map(std::borrow::Cow::into_owned)))
                    .collect();
                let mut results: Vec<Option<Result<CachedImage>>> = specs
                    .iter()
                    .map(|spec| {
                        (spec.shape == OutputShape::Single).then(|| {
                            let shared_head = specs.iter().zip(&heads).find_map(|(other, head)| match head {
                                Some(Ok(head)) if other.shape == OutputShape::Sprite && other.convert == spec.convert => Some(head),
                                _ => None,
                            });
                            match shared_head {
                                Some(head) => ImageConverter::encode(head, spec.convert.format_type, spec.convert.quality).map(|bytes| CachedImage {
                                    bytes: Bytes::from(bytes),
                                    frame_count: None,
                                }),
                                None => Self::convert_decoded(&first, &spec.convert),
                            }
                            .map_err(Self::decode_error)
                        })
                    })
                    .collect();
                drop(first);

                for (index, (spec, head)) in specs.iter().zip(heads).enumerate() {
                    let Some(head) = head else {
                        continue;
                    };
                    results[index] = Some(head.map_err(Self::decode_error).and_then(|head| {
                        let mut resized = vec![head];
                        for frame in &frames[1..] {
                            resized.push(
                                ImageConverter::decode(frame)
                                    .and_then(|image| ImageConverter::resize(image, &spec.convert))
                                    .map_err(Self::decode_error)?,
                            );
                        }
                        Ok(CachedImage {
                            bytes: Bytes::from(ImageConverter::encode(
                                &ImageConverter::tile_horizontally(&resized),
                                spec.convert.format_type,
                                spec.convert.quality,
                            )?),
                            frame_count: Some(resized.len() as u32),
                        })
                    }));
                }
                Ok(results
                    .into_iter()
                    .map(|result| result.expect("every output is made from the first frame or the frames"))
                    .collect())
            })
            .await?
    }

    async fn extract_frames(&self, file: &Path, positions_secs: &[f64], class: WorkClass) -> Result<Vec<Vec<u8>>> {
        #[cfg(test)]
        self.ffmpeg_runs.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.tools.extract_frames(file, positions_secs, class).await
    }

    /// ffmpeg で読む画像をデコードする。HEIC/HEIF/AVIF は PPM、それ以外（JXL、JP2）は PNG を使う。
    /// アルファ付きの HEIC/HEIF/AVIF は heif-dec に回し、ffmpeg が失敗したときも heif-dec で読み直す。
    async fn decode_ffmpeg_image(&self, source: &ResolvedStoragePath, file: &Path, kind: MediaKind, class: WorkClass) -> Result<Vec<u8>> {
        let heif_decodable = kind == MediaKind::HeifImage || Self::is_avif(source);
        if !heif_decodable {
            return self.extract_image(file, class).await;
        }
        if self.has_alpha(source, class).await? {
            return self.decode_heif(file, class).await;
        }
        match self.extract_image_pixels(file, class).await {
            Ok(image) => Ok(image),
            Err(error) => {
                warn!(error = ?error, "ffmpeg failed to decode a HEIF or AVIF image, so heif-dec reads it instead");
                self.decode_heif(file, class).await
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
    async fn has_alpha(&self, source: &ResolvedStoragePath, class: WorkClass) -> Result<bool> {
        match self.media_info.probe(source, class).await {
            Ok(MediaInfo::Image(info)) => Ok(info.has_alpha),
            Ok(_) => Ok(false),
            Err(error) => {
                warn!(error = ?error, "failed to probe an image for alpha, so heif-dec reads it");
                Ok(true)
            }
        }
    }

    async fn extract_image(&self, file: &Path, class: WorkClass) -> Result<Vec<u8>> {
        #[cfg(test)]
        self.ffmpeg_runs.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.tools.extract_image(file, class).await
    }

    async fn extract_image_pixels(&self, file: &Path, class: WorkClass) -> Result<Vec<u8>> {
        #[cfg(test)]
        self.ffmpeg_runs.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.tools.extract_image_pixels(file, class).await
    }

    /// heif-dec で読む。アルファ付きの HEIC/HEIF/AVIF と、ffmpeg が読めなかったときのフォールバックに使う。
    async fn decode_heif(&self, file: &Path, class: WorkClass) -> Result<Vec<u8>> {
        #[cfg(test)]
        self.heif_dec_runs.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.tools.decode_heif(file, class).await
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
        let dimension = |value: Option<u32>| value.unwrap_or(0).to_be_bytes();
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
        core::work::WorkClass,
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
            class: if variant == ThumbnailVariant::Display {
                WorkClass::Interactive
            } else {
                WorkClass::Foreground
            },
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
    async fn the_current_key_is_the_key_of_the_image_and_costs_no_conversion_test() -> TestResult {
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let generator = &fixture.generator;
        let album = fixture.dir.path().join("album");
        std::fs::create_dir_all(&album)?;
        let file = album.join("1.png");
        write_png(&file, 600, 300)?;
        std::fs::write(album.join("note.txt"), b"text")?;
        let request = thumbnail_request(256, 256);
        let resolve = |path: std::path::PathBuf| {
            let fs = generator.inner.fs.clone();
            async move { fs.resolve_entry(&nested(&path), WorkClass::Foreground).await }
        };

        let key = generator.current_key(resolve(file.clone()).await?, &request).await?.expect("a file has a key");
        assert_eq!(generator.conversions(), 0, "the key is found without converting");
        assert_eq!(generator.inner.pools.cpu.running(WorkClass::Foreground), 0);

        // The key is the one the generated image carries, and a folder gives the key of its representative.
        let generated = generator.generate(&nested(&file), &request).await?;
        assert_eq!(generated.key.as_deref(), Some(key.as_str()));
        assert_eq!(generator.current_key(resolve(album.clone()).await?, &request).await?, Some(key.clone()));

        // A replaced file has another key, and a folder without an image has none to give.
        write_png(&file, 300, 600)?;
        tokio::time::sleep(Duration::from_millis(20)).await;
        let replaced = generator.current_key(resolve(file.clone()).await?, &request).await?;
        assert_ne!(replaced, Some(key));
        let empty = fixture.dir.path().join("empty");
        std::fs::create_dir_all(&empty)?;
        let error = generator.current_key(resolve(empty).await?, &request).await.unwrap_err();
        assert_eq!(*error.kind(), ErrorKind::NotFound);
        assert_eq!(generator.conversions(), 1, "only the generation above converted");
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
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let album = fixture.dir.path().join("album");
        std::fs::create_dir(&album)?;
        write_png(&album.join("1.png"), 100, 200)?;
        write_png(&album.join("2.png"), 200, 100)?;
        let set_old = || filetime::set_file_mtime(&album, filetime::FileTime::from_unix_time(100, 0));
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
        let source = fixture.generator.inner.fs.resolve_entry(&nested(&video), WorkClass::Foreground).await?;
        let media = &fixture.generator.inner.media_info;
        let (a, b) = tokio::join!(media.probe_video(&source, WorkClass::Foreground), media.probe_video(&source, WorkClass::Foreground));
        assert_eq!(a?.duration_secs, b?.duration_secs);
        assert_eq!(media.probe_count(), 1);
        fixture.generator.generate(&nested(&video), &thumbnail_request(128, 128)).await?;
        assert_eq!(media.probe_count(), 1, "thumbnail reuses the playback probe");
        write_video(&video, 4, 160, 120)?;
        assert_eq!(media.probe_video(&source, WorkClass::Foreground).await?.duration_secs, 4.0);
        assert_eq!(media.probe_count(), 2);
        let archive = fixture.dir.path().join("info.7z");
        write_7z(&archive, &[("clip.mp4", &std::fs::read(&video)?)])?;
        let mut inside = nested(&archive);
        inside.push_str("clip.mp4")?;
        let source = fixture.generator.inner.fs.resolve_entry(&inside, WorkClass::Foreground).await?;
        media.probe_video(&source, WorkClass::Foreground).await?;
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
        let source = fixture.generator.inner.fs.resolve_entry(&nested(&png), WorkClass::Foreground).await?;
        let MediaInfo::Image(info) = media.probe(&source, WorkClass::Foreground).await? else {
            return Err("an image should give image information".into());
        };
        assert_eq!((info.content_type.as_str(), info.width, info.height), ("image/png", 40, 20));
        assert_eq!(info.exif, ImageExif::default());
        assert_eq!(
            *media.probe_video(&source, WorkClass::Foreground).await.unwrap_err().kind(),
            ErrorKind::UnsupportedType,
            "画像は動画としては読めない"
        );

        let broken = fixture.dir.path().join("broken.png");
        std::fs::write(&broken, b"not an image")?;
        let source = fixture.generator.inner.fs.resolve_entry(&nested(&broken), WorkClass::Foreground).await?;
        assert_eq!(*media.probe(&source, WorkClass::Foreground).await.unwrap_err().kind(), ErrorKind::UnsupportedType);

        let archive = fixture.dir.path().join("pics.7z");
        write_7z(&archive, &[("inner.png", &png_bytes(30, 10)?)])?;
        let mut inside = nested(&archive);
        inside.push_str("inner.png")?;
        let source = fixture.generator.inner.fs.resolve_entry(&inside, WorkClass::Foreground).await?;
        let MediaInfo::Image(info) = media.probe(&source, WorkClass::Foreground).await? else {
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
        let source = fixture.generator.inner.fs.resolve_entry(&nested(&heic), WorkClass::Foreground).await?;
        let MediaInfo::Image(info) = fixture.generator.inner.media_info.probe(&source, WorkClass::Foreground).await? else {
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
            let source = fixture
                .generator
                .inner
                .fs
                .resolve_entry(&nested(&fixture.dir.path().join(name)), WorkClass::Foreground)
                .await?;
            let MediaInfo::Image(info) = fixture.generator.inner.media_info.probe(&source, WorkClass::Foreground).await? else {
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
            let source = fixture.generator.inner.fs.resolve_entry(&nested(&file), WorkClass::Foreground).await?;
            let MediaInfo::Image(info) = fixture.generator.inner.media_info.probe(&source, WorkClass::Foreground).await? else {
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
        let source = fixture.generator.inner.fs.resolve_entry(&nested(&avif), WorkClass::Foreground).await?;
        let MediaInfo::Image(info) = fixture.generator.inner.media_info.probe(&source, WorkClass::Foreground).await? else {
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

    // The tests below pin the companions: the thumbnails that one decode also produces. They request WebP, because the configured default
    // format is WebP and a companion never matches a request for another format.

    fn webp(variant: ThumbnailVariant, side: u32, resize: ImageResizeType) -> ThumbnailRequest {
        request(variant, Some(side), Some(side), ImageFormatType::Webp, resize)
    }

    fn webp_display() -> ThumbnailRequest {
        request(ThumbnailVariant::Display, None, None, ImageFormatType::Webp, ImageResizeType::Fit)
    }

    fn write_noisy_png(path: &Path, width: u32, height: u32) -> TestResult {
        let mut output = Vec::new();
        RgbaImage::from_fn(width, height, |x, y| {
            let v = x.wrapping_mul(2654435761).wrapping_add(y.wrapping_mul(40503)) >> 8;
            Rgba([v as u8, (v >> 3) as u8, (v >> 5) as u8, 255])
        })
        .write_to(&mut Cursor::new(&mut output), ImageFormat::Png)?;
        std::fs::write(path, output)?;
        Ok(())
    }

    async fn source_of(fixture: &Fixture, file: &Path) -> TestResult<SourceId> {
        let inner = &fixture.generator.inner;
        let resolved = inner.fs.resolve_entry(&nested(file), WorkClass::Foreground).await?;
        let entry = resolved.get_entry(WorkClass::Foreground).await?;
        Ok(inner.source_id(&resolved, &entry).ok_or("the file has no cache key")?)
    }

    /// The key of a default-format thumbnail, which is what a companion is.
    fn companion_key(source: &SourceId, kind: MediaKind, resize: ImageResizeType, side: u32) -> ThumbnailKey {
        Plan {
            variant: ThumbnailVariant::Thumbnail,
            bucket: CacheBucket::Thumbnail,
            convert: ImageConvertOptions {
                format_type: ImageFormatType::Webp,
                quality: 80,
                resize_type: resize,
                resampler_type: ImageResamplerType::Lanczos3,
                anchor_position_type: ImageAnchorPositionType::Center,
                width: Some(side),
                height: Some(side),
            },
        }
        .key(source, ThumbnailVariant::Thumbnail, kind)
    }

    fn companion_keys_of(fixture: &Fixture, source: &SourceId, kind: MediaKind, request: &ThumbnailRequest) -> TestResult<Vec<ThumbnailKey>> {
        let inner = &fixture.generator.inner;
        let plan = inner.plan(request)?;
        Ok(inner.companion_plans(&plan, kind, source).into_iter().map(|(key, _)| key).collect())
    }

    #[tokio::test]
    async fn videos_and_images_use_separate_external_process_pools_test() -> TestResult {
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let inner = &fixture.generator.inner;
        let _held = inner.pools.process_video.ticket(WorkClass::Foreground).acquire().await;
        assert!(
            inner.pool_for(MediaKind::Video).try_acquire(WorkClass::Foreground).is_none(),
            "the video pool has one shared slot"
        );
        for kind in [MediaKind::FfmpegImage, MediaKind::HeifImage, MediaKind::Image] {
            assert!(inner.pool_for(kind).try_acquire(WorkClass::Foreground).is_some(), "{kind:?}");
        }
        Ok(())
    }

    #[tokio::test]
    async fn companion_plans_cover_the_filmstrip_sizes_and_the_other_shape_test() -> TestResult {
        use ImageResizeType::{Crop, Fit};
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let file = fixture.dir.path().join("a.png");
        write_png(&file, 1000, 500)?;
        let source = source_of(&fixture, &file).await?;
        let key = |resize, side| companion_key(&source, MediaKind::Image, resize, side);

        let filmstrip = [key(Crop, 128), key(Fit, 128), key(Crop, 256), key(Fit, 256), key(Crop, 512), key(Fit, 512)];
        let listing = companion_keys_of(&fixture, &source, MediaKind::Image, &webp(ThumbnailVariant::Thumbnail, 1024, Crop))?;
        assert_eq!(
            listing,
            [filmstrip.as_slice(), &[key(Fit, 1024)]].concat(),
            "the smallest three steps in both shapes and the other shape at 1024"
        );
        assert_eq!(listing.iter().collect::<HashSet<_>>().len(), listing.len(), "the keys are unique");

        let display = companion_keys_of(&fixture, &source, MediaKind::Image, &webp_display())?;
        assert_eq!(display, filmstrip, "a display request has only the smallest three steps in both shapes");

        let at_the_smallest = companion_keys_of(&fixture, &source, MediaKind::Image, &webp(ThumbnailVariant::Thumbnail, 128, Crop))?;
        assert_eq!(
            at_the_smallest,
            filmstrip[1..],
            "the primary key is removed, and so is the other shape that is already a filmstrip size"
        );
        assert!(!at_the_smallest.contains(&key(Crop, 128)));

        assert!(!listing.contains(&key(Crop, 1024)), "the primary key is not in the companions");
        Ok(())
    }

    #[tokio::test]
    async fn companion_plans_of_a_sprite_leave_out_the_thumbnail_it_makes_and_never_the_sprite_key_test() -> TestResult {
        use ImageResizeType::{Crop, Fit};
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let file = fixture.dir.path().join("a.png");
        write_png(&file, 1000, 500)?;
        let source = source_of(&fixture, &file).await?;
        let key = |resize, side| companion_key(&source, MediaKind::Video, resize, side);

        let request = webp(ThumbnailVariant::Sprite, 128, Fit);
        let keys = companion_keys_of(&fixture, &source, MediaKind::Video, &request)?;
        assert_eq!(keys, [key(Crop, 128), key(Crop, 256), key(Fit, 256), key(Crop, 512), key(Fit, 512)]);
        let plan = fixture.generator.inner.plan(&request)?;
        assert!(!keys.contains(&plan.key(&source, ThumbnailVariant::Sprite, MediaKind::Video)));
        Ok(())
    }

    #[tokio::test]
    async fn switching_the_shape_is_served_from_the_cache_test() -> TestResult {
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let file = fixture.dir.path().join("a.png");
        write_png(&file, 1000, 500)?;

        let crop = fixture
            .generator
            .generate(&nested(&file), &webp(ThumbnailVariant::Thumbnail, 512, ImageResizeType::Crop))
            .await?;
        assert_eq!(size(&crop)?, (512, 512));
        assert_eq!(fixture.generator.conversions(), 1);

        let fit = fixture
            .generator
            .generate(&nested(&file), &webp(ThumbnailVariant::Thumbnail, 512, ImageResizeType::Fit))
            .await?;
        assert_eq!(size(&fit)?, (512, 256));
        assert_eq!(fixture.generator.conversions(), 1, "the other shape was made from the same decode");

        let direct = self::fixture(ThumbnailOptions::default()).await?;
        let other = direct.dir.path().join("a.png");
        write_png(&other, 1000, 500)?;
        let expected = direct
            .generator
            .generate(&nested(&other), &webp(ThumbnailVariant::Thumbnail, 512, ImageResizeType::Fit))
            .await?;
        assert_eq!(fit.bytes, expected.bytes, "the companion is the same image as a direct request");
        assert_eq!(fit.format, ImageFormatType::Webp);
        Ok(())
    }

    #[tokio::test]
    async fn a_filmstrip_request_hits_after_a_listing_request_test() -> TestResult {
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let file = fixture.dir.path().join("a.png");
        write_png(&file, 3000, 2000)?;

        let listing = fixture
            .generator
            .generate(&nested(&file), &webp(ThumbnailVariant::Thumbnail, 1024, ImageResizeType::Crop))
            .await?;
        assert_eq!(size(&listing)?, (1024, 1024));
        for side in [128, 256, 512] {
            let thumbnail = fixture
                .generator
                .generate(&nested(&file), &webp(ThumbnailVariant::Thumbnail, side, ImageResizeType::Crop))
                .await?;
            assert_eq!(size(&thumbnail)?, (side, side));
        }
        assert_eq!(fixture.generator.conversions(), 1);
        assert_eq!((fixture.generator.ffmpeg_runs(), fixture.generator.heif_dec_runs()), (0, 0));
        Ok(())
    }

    #[tokio::test]
    async fn a_display_request_warms_the_companions_test() -> TestResult {
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let file = fixture.dir.path().join("a.png");
        write_png(&file, 3000, 2000)?;

        fixture.generator.generate(&nested(&file), &webp_display()).await?;
        for resize in [ImageResizeType::Crop, ImageResizeType::Fit] {
            for side in [128, 256, 512] {
                fixture.generator.generate(&nested(&file), &webp(ThumbnailVariant::Thumbnail, side, resize)).await?;
            }
        }
        assert_eq!(fixture.generator.conversions(), 1, "the filmstrip is warmed by the display image's decode");
        Ok(())
    }

    #[tokio::test]
    async fn a_primary_cache_hit_does_not_decode_for_companions_test() -> TestResult {
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let file = fixture.dir.path().join("a.png");
        write_png(&file, 1000, 500)?;
        let request = webp(ThumbnailVariant::Thumbnail, 1024, ImageResizeType::Crop);

        fixture.generator.generate(&nested(&file), &request).await?;
        let source = source_of(&fixture, &file).await?;
        // Forget one companion so that a request for the primary would have a reason to decode again.
        let forgotten = companion_key(&source, MediaKind::Image, ImageResizeType::Crop, 128);
        fixture.generator.inner.cache.forget(&forgotten).await?;
        assert!(!fixture.generator.inner.cache.contains(&forgotten).await?);

        fixture.generator.generate(&nested(&file), &request).await?;
        assert_eq!(fixture.generator.conversions(), 1, "the primary hit returns without generating companions");
        assert!(!fixture.generator.inner.cache.contains(&forgotten).await?);
        Ok(())
    }

    #[tokio::test]
    async fn a_cached_companion_is_not_made_again_test() -> TestResult {
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let file = fixture.dir.path().join("a.png");
        write_png(&file, 1000, 500)?;
        let source = source_of(&fixture, &file).await?;

        let sentinel = CachedImage {
            bytes: Bytes::from_static(b"already cached"),
            frame_count: None,
        };
        let cached = companion_key(&source, MediaKind::Image, ImageResizeType::Crop, 256);
        fixture.generator.inner.cache.put(&cached, CacheBucket::Thumbnail, &sentinel).await?;

        fixture
            .generator
            .generate(&nested(&file), &webp(ThumbnailVariant::Thumbnail, 1024, ImageResizeType::Crop))
            .await?;
        assert_eq!(
            fixture.generator.inner.cache.get(&cached).await?,
            Some(sentinel),
            "the cached companion is neither encoded nor overwritten"
        );
        let fresh = companion_key(&source, MediaKind::Image, ImageResizeType::Fit, 256);
        assert!(fixture.generator.inner.cache.contains(&fresh).await?, "the companions that were missing are made");
        Ok(())
    }

    #[tokio::test]
    async fn a_companion_that_fails_to_convert_is_dropped_and_the_primary_survives_test() -> TestResult {
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let file = fixture.dir.path().join("a.png");
        write_png(&file, 1000, 500)?;
        let source = source_of(&fixture, &file).await?;
        let inner = &fixture.generator.inner;
        let plan = inner.plan(&webp(ThumbnailVariant::Thumbnail, 256, ImageResizeType::Crop))?;

        let mut companions = inner.companion_plans(&plan, MediaKind::Image, &source);
        let valid = companions.len();
        // JPEG has no transparency, so it cannot be combined with a pad.
        let invalid = ImageConvertOptions {
            format_type: ImageFormatType::Jpeg,
            resize_type: ImageResizeType::Pad,
            ..companions[0].1.clone()
        };
        companions.insert(0, (companion_key(&source, MediaKind::Image, ImageResizeType::Manual, 64), invalid));

        let companion = |(key, convert): (ThumbnailKey, ImageConvertOptions)| OutputSpec {
            key: Some(key),
            convert,
            shape: OutputShape::Single,
            bucket: CacheBucket::Thumbnail,
            requested: false,
        };
        let mut specs = inner.requested_specs(&plan, MediaKind::Image, Some(&source));
        specs.extend(companions.into_iter().map(companion));
        let permit = inner.pools.cpu.ticket(WorkClass::Foreground).acquire().await?;
        let results = inner.convert_many(permit, std::fs::read(&file)?, specs.clone()).await?;
        assert_eq!(results.len(), specs.len(), "the results are aligned with the specs");
        let primary = results[0].as_ref().expect("the primary is made");
        assert_eq!(image::guess_format(&primary.bytes)?, ImageFormat::WebP);
        assert!(results[1].is_err(), "the invalid companion fails to convert");

        // The job drops the failed companion and keeps the primary.
        let output = JobOutput::collect(&specs, results);
        assert_eq!(output.images.len(), 1 + valid, "only the invalid companion is dropped");
        assert!(output.errors.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn a_primary_that_fails_to_decode_fails_the_request_test() -> TestResult {
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let file = fixture.dir.path().join("broken.png");
        std::fs::write(&file, b"not a png")?;
        let error = fixture
            .generator
            .generate(&nested(&file), &webp(ThumbnailVariant::Thumbnail, 256, ImageResizeType::Crop))
            .await
            .unwrap_err();
        assert_eq!(*error.kind(), ErrorKind::UnsupportedType);
        Ok(())
    }

    #[tokio::test]
    async fn a_bucket_that_cannot_hold_any_entry_still_returns_the_primary_test() -> TestResult {
        let fixture = fixture(ThumbnailOptions {
            cache_capacity_bytes: 1,
            ..ThumbnailOptions::default()
        })
        .await?;
        let file = fixture.dir.path().join("a.png");
        write_png(&file, 1000, 500)?;

        let thumbnail = fixture
            .generator
            .generate(&nested(&file), &webp(ThumbnailVariant::Thumbnail, 512, ImageResizeType::Crop))
            .await?;
        assert_eq!(size(&thumbnail)?, (512, 512));
        Ok(())
    }

    fn cached_bytes(root: &Path) -> u64 {
        fn walk(dir: &Path) -> u64 {
            std::fs::read_dir(dir)
                .into_iter()
                .flatten()
                .flatten()
                .map(|entry| match entry.metadata() {
                    Ok(meta) if meta.is_dir() => walk(&entry.path()),
                    Ok(meta) => meta.len(),
                    Err(_) => 0,
                })
                .sum()
        }
        walk(&root.join("files"))
    }

    #[tokio::test]
    async fn a_small_bucket_keeps_the_primary_and_evicts_companions_test() -> TestResult {
        let request = webp(ThumbnailVariant::Thumbnail, 1024, ImageResizeType::Crop);

        // Measure the bytes of the primary and of all the companions with a bucket that holds everything.
        let measured = fixture(ThumbnailOptions::default()).await?;
        let file = measured.dir.path().join("a.png");
        write_noisy_png(&file, 1500, 1200)?;
        let primary = measured.generator.generate(&nested(&file), &request).await?.bytes.len() as u64;
        let all = cached_bytes(&measured.dir.path().join("thumbnail"));
        let companions = all - primary;
        assert!(companions > 0);

        // The primary fits, and all the companions together do not.
        let capacity = primary + companions / 2;
        let small = fixture(ThumbnailOptions {
            cache_capacity_bytes: capacity,
            ..ThumbnailOptions::default()
        })
        .await?;
        let other = small.dir.path().join("a.png");
        write_noisy_png(&other, 1500, 1200)?;
        let first = small.generator.generate(&nested(&other), &request).await?;
        assert_eq!(first.bytes.len() as u64, primary);
        let stored = cached_bytes(&small.dir.path().join("thumbnail"));
        assert!(stored <= capacity && stored < all, "the bucket evicted companions: {stored} of {all}, capacity {capacity}");

        let second = small.generator.generate(&nested(&other), &request).await?;
        assert_eq!(second, first);
        assert_eq!(small.generator.conversions(), 1, "the primary stayed in the cache");
        Ok(())
    }

    #[tokio::test]
    async fn a_failing_store_does_not_fail_the_response_test() -> TestResult {
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let file = fixture.dir.path().join("a.png");
        write_png(&file, 1000, 500)?;
        // Every store writes through the staging directory first.
        std::fs::remove_dir_all(fixture.dir.path().join("thumbnail").join("staging"))?;

        let thumbnail = fixture
            .generator
            .generate(&nested(&file), &webp(ThumbnailVariant::Thumbnail, 512, ImageResizeType::Crop))
            .await?;
        assert_eq!(size(&thumbnail)?, (512, 512));
        assert_eq!(image::guess_format(&thumbnail.bytes)?, ImageFormat::WebP);
        Ok(())
    }

    // The tests of source jobs wait on the counters of `ConversionProbe` and on the pool, not on time,
    // so a slow machine only makes them slower.
    async fn until(condition: impl Fn() -> bool) {
        tokio::time::timeout(Duration::from_secs(30), async {
            while !condition() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("the condition was not reached in time");
    }

    /// Takes every slot of a pool, so that the jobs that ask for it wait.
    /// The shared slots go first: Foreground does not start while Interactive runs, so it could not take them after the reserved slot.
    async fn hold_all(pool: &crate::core::work::WorkPriorityPool) -> Vec<WorkPermit> {
        let limits = pool.limits();
        let mut held = Vec::new();
        for _ in 0..limits.shared {
            held.push(pool.ticket(WorkClass::Foreground).acquire().await.unwrap());
        }
        for _ in 0..limits.interactive {
            held.push(pool.ticket(WorkClass::Interactive).acquire().await.unwrap());
        }
        held
    }

    fn placements(fixture: &Fixture) -> usize {
        fixture.generator.inner.probe.placements.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// How many conversions reached the gate, that is, how many jobs started to decode.
    fn reached(fixture: &Fixture) -> usize {
        fixture.generator.inner.probe.reached.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn peak(fixture: &Fixture) -> usize {
        fixture.generator.inner.probe.peak.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Holds the conversions that start from now on at the gate.
    fn close_the_gate(fixture: &Fixture) {
        fixture.generator.inner.probe.gate.forget_permits(tokio::sync::Semaphore::MAX_PERMITS);
    }

    fn open_the_gate(fixture: &Fixture) {
        fixture.generator.inner.probe.gate.add_permits(tokio::sync::Semaphore::MAX_PERMITS);
    }

    fn spawn_generate(fixture: &Fixture, file: &Path, request: ThumbnailRequest) -> tokio::task::JoinHandle<Result<Thumbnail>> {
        let generator = fixture.generator.clone();
        let path = nested(file);
        tokio::spawn(async move { generator.generate(&path, &request).await })
    }

    #[tokio::test]
    async fn requests_for_two_sizes_that_wait_for_a_slot_share_one_job_test() -> TestResult {
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let file = fixture.dir.path().join("a.png");
        write_png(&file, 1200, 800)?;
        let held = hold_all(&fixture.generator.inner.pools.cpu).await;

        // 1024 is not a companion of 512, so only a job that holds both requests can make both from one decode.
        let large = spawn_generate(&fixture, &file, webp(ThumbnailVariant::Thumbnail, 1024, ImageResizeType::Crop));
        let small = spawn_generate(&fixture, &file, webp(ThumbnailVariant::Thumbnail, 128, ImageResizeType::Crop));
        until(|| placements(&fixture) == 2).await;
        drop(held);

        assert_eq!(size(&large.await??)?, (1024, 1024));
        assert_eq!(size(&small.await??)?, (128, 128));
        assert_eq!(fixture.generator.conversions(), 1);
        until(|| fixture.generator.inner.jobs.sources() == 0).await;
        Ok(())
    }

    #[tokio::test]
    async fn a_request_for_a_companion_of_the_running_job_waits_for_it_test() -> TestResult {
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let file = fixture.dir.path().join("a.png");
        write_png(&file, 1200, 800)?;
        close_the_gate(&fixture);

        let large = spawn_generate(&fixture, &file, webp(ThumbnailVariant::Thumbnail, 512, ImageResizeType::Crop));
        until(|| reached(&fixture) == 1).await;
        // 128 `crop` is a companion of the running job, which has fixed what it makes.
        let small = spawn_generate(&fixture, &file, webp(ThumbnailVariant::Thumbnail, 128, ImageResizeType::Crop));
        until(|| placements(&fixture) == 2).await;
        assert_eq!(reached(&fixture), 1, "the second request did not start a decode of its own");
        open_the_gate(&fixture);

        assert_eq!(size(&large.await??)?, (512, 512));
        assert_eq!(size(&small.await??)?, (128, 128));
        assert_eq!(fixture.generator.conversions(), 1);
        assert_eq!(peak(&fixture), 1);
        Ok(())
    }

    #[tokio::test]
    async fn other_sizes_and_display_images_wait_for_the_running_job_and_then_share_the_next_one_test() -> TestResult {
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let file = fixture.dir.path().join("a.png");
        write_png(&file, 1200, 800)?;
        close_the_gate(&fixture);

        let first = spawn_generate(&fixture, &file, webp(ThumbnailVariant::Thumbnail, 512, ImageResizeType::Crop));
        until(|| reached(&fixture) == 1).await;
        // 1024 is not a companion of the running job, and a display image is not one either.
        let other_size = spawn_generate(&fixture, &file, webp(ThumbnailVariant::Thumbnail, 1024, ImageResizeType::Crop));
        let display = spawn_generate(&fixture, &file, webp_display());
        until(|| placements(&fixture) == 3).await;
        assert_eq!(reached(&fixture), 1, "the source is decoded by one job at a time");
        open_the_gate(&fixture);

        assert_eq!(size(&first.await??)?, (512, 512));
        assert_eq!(size(&other_size.await??)?, (1024, 1024));
        assert_eq!(image::guess_format(&display.await??.bytes)?, ImageFormat::WebP);
        assert_eq!(fixture.generator.conversions(), 2, "the running job, and one job for the two requests that waited");
        assert_eq!(peak(&fixture), 1, "two conversions of one source never ran at the same time");
        until(|| fixture.generator.inner.jobs.sources() == 0).await;
        Ok(())
    }

    #[tokio::test]
    async fn an_interactive_request_moves_its_waiting_job_ahead_of_an_older_foreground_job_test() -> TestResult {
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let (older, newer) = (fixture.dir.path().join("a.png"), fixture.dir.path().join("b.png"));
        write_png(&older, 1000, 500)?;
        write_png(&newer, 1000, 500)?;
        let inner = &fixture.generator.inner;
        let pool = &inner.pools.cpu;
        let mut held = hold_all(pool).await;
        close_the_gate(&fixture);

        // Both jobs wait as Foreground, the older one first.
        let older_task = spawn_generate(&fixture, &older, webp(ThumbnailVariant::Thumbnail, 256, ImageResizeType::Crop));
        until(|| pool.waiting(WorkClass::Foreground) == 1).await;
        let newer_task = spawn_generate(&fixture, &newer, webp(ThumbnailVariant::Thumbnail, 256, ImageResizeType::Crop));
        until(|| pool.waiting(WorkClass::Foreground) == 2).await;
        // The display image of the newer file joins its waiting job, which then asks for its slot as Interactive.
        let display = spawn_generate(&fixture, &newer, webp_display());
        until(|| pool.waiting(WorkClass::Interactive) == 1 && pool.waiting(WorkClass::Foreground) == 1).await;

        // One shared slot frees up. It goes to the Interactive job, not to the Foreground job that has waited longer.
        drop(held.remove(0));
        until(|| reached(&fixture) == 1).await;
        assert_eq!(pool.running(WorkClass::Interactive), 2, "the held slot and the newer file's job");
        assert_eq!(pool.waiting(WorkClass::Foreground), 1, "the older file's job still waits");
        assert!(!older_task.is_finished());

        open_the_gate(&fixture);
        drop(held);
        assert_eq!(size(&newer_task.await??)?, (256, 256));
        assert_eq!(image::guess_format(&display.await??.bytes)?, ImageFormat::WebP);
        assert_eq!(size(&older_task.await??)?, (256, 256));
        assert_eq!(fixture.generator.conversions(), 2);
        Ok(())
    }

    #[tokio::test]
    async fn a_waiting_job_whose_requests_were_all_cut_starts_nothing_test() -> TestResult {
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let file = fixture.dir.path().join("a.png");
        write_png(&file, 1000, 500)?;
        let pool = &fixture.generator.inner.pools.cpu;
        let held = hold_all(pool).await;

        let first = spawn_generate(&fixture, &file, webp(ThumbnailVariant::Thumbnail, 512, ImageResizeType::Crop));
        let second = spawn_generate(&fixture, &file, webp(ThumbnailVariant::Thumbnail, 128, ImageResizeType::Crop));
        until(|| placements(&fixture) == 2 && pool.waiting(WorkClass::Foreground) == 1).await;
        first.abort();
        assert!(first.await.unwrap_err().is_cancelled());
        assert_eq!(fixture.generator.inner.jobs.sources(), 1, "the second request still waits in the job");
        second.abort();
        assert!(second.await.unwrap_err().is_cancelled());

        until(|| fixture.generator.inner.jobs.sources() == 0 && pool.waiting(WorkClass::Foreground) == 0).await;
        drop(held);
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(reached(&fixture), 0);
        assert_eq!(fixture.generator.conversions(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn a_job_that_started_stores_its_images_when_its_requests_were_all_cut_test() -> TestResult {
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let file = fixture.dir.path().join("a.png");
        write_png(&file, 1000, 500)?;
        let source = source_of(&fixture, &file).await?;
        close_the_gate(&fixture);

        let task = spawn_generate(&fixture, &file, webp(ThumbnailVariant::Thumbnail, 512, ImageResizeType::Crop));
        until(|| reached(&fixture) == 1).await;
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        open_the_gate(&fixture);

        let primary = companion_key(&source, MediaKind::Image, ImageResizeType::Crop, 512);
        until(|| fixture.generator.conversions() == 1).await;
        // The store follows the conversion.
        tokio::time::timeout(Duration::from_secs(30), async {
            while !fixture.generator.inner.cache.contains(&primary).await.unwrap_or(false) {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await?;
        until(|| fixture.generator.inner.jobs.sources() == 0).await;
        Ok(())
    }

    #[tokio::test]
    async fn a_file_without_a_cache_key_is_generated_for_its_request_alone_test() -> TestResult {
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let file = fixture.dir.path().join("a.png");
        write_png(&file, 1000, 500)?;
        let inner = &fixture.generator.inner;
        let target = inner.fs.resolve_entry(&nested(&file), WorkClass::Foreground).await?;
        let plan = inner.plan(&webp(ThumbnailVariant::Thumbnail, 256, ImageResizeType::Crop))?;

        let image = inner.clone().generate_alone(&target, MediaKind::Image, &plan, WorkClass::Foreground).await?;
        assert_eq!(image::guess_format(&image.bytes)?, ImageFormat::WebP);
        assert_eq!(cached_bytes(&fixture.dir.path().join("thumbnail")), 0, "an image without a key is not stored");

        // If the request is cut while it waits for a slot, nothing is generated.
        let held = hold_all(&inner.pools.cpu).await;
        let waiting = {
            let (inner, target, plan) = (inner.clone(), target.clone(), plan.clone());
            tokio::spawn(async move { inner.generate_alone(&target, MediaKind::Image, &plan, WorkClass::Foreground).await })
        };
        until(|| inner.pools.cpu.waiting(WorkClass::Foreground) == 1).await;
        waiting.abort();
        assert!(waiting.await.unwrap_err().is_cancelled());
        until(|| inner.pools.cpu.waiting(WorkClass::Foreground) == 0).await;
        drop(held);
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(fixture.generator.conversions(), 1, "only the first request was converted");
        Ok(())
    }

    #[tokio::test]
    async fn a_source_that_cannot_be_decoded_fails_every_request_for_it_test() -> TestResult {
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let file = fixture.dir.path().join("broken.png");
        std::fs::write(&file, b"not a png")?;
        let held = hold_all(&fixture.generator.inner.pools.cpu).await;

        let first = spawn_generate(&fixture, &file, webp(ThumbnailVariant::Thumbnail, 512, ImageResizeType::Crop));
        let second = spawn_generate(&fixture, &file, webp(ThumbnailVariant::Thumbnail, 1024, ImageResizeType::Crop));
        until(|| placements(&fixture) == 2).await;
        drop(held);

        for task in [first, second] {
            assert_eq!(*task.await?.unwrap_err().kind(), ErrorKind::UnsupportedType);
        }
        until(|| fixture.generator.inner.jobs.sources() == 0).await;
        Ok(())
    }

    #[tokio::test]
    async fn an_ffmpeg_image_fans_out_from_one_ffmpeg_run_test() -> TestResult {
        if skip_without_ffmpeg() {
            return Ok(());
        }
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let file = fixture.dir.path().join("photo.jxl");
        write_png(&file, 1000, 500)?;

        fixture
            .generator
            .generate(&nested(&file), &webp(ThumbnailVariant::Thumbnail, 512, ImageResizeType::Crop))
            .await?;
        let fit = fixture
            .generator
            .generate(&nested(&file), &webp(ThumbnailVariant::Thumbnail, 512, ImageResizeType::Fit))
            .await?;
        fixture
            .generator
            .generate(&nested(&file), &webp(ThumbnailVariant::Thumbnail, 128, ImageResizeType::Crop))
            .await?;
        assert_eq!(size(&fit)?, (512, 256));
        assert_eq!(fixture.generator.conversions(), 1);
        assert_eq!(fixture.generator.ffmpeg_runs(), 1, "the companions do not run ffmpeg again");
        assert_eq!(fixture.generator.heif_dec_runs(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn a_video_fans_out_from_the_first_frame_test() -> TestResult {
        if skip_without_ffmpeg() {
            return Ok(());
        }
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let video = fixture.dir.path().join("clip.mp4");
        write_video(&video, 10, 320, 240)?;

        let crop = fixture
            .generator
            .generate(&nested(&video), &webp(ThumbnailVariant::Thumbnail, 128, ImageResizeType::Crop))
            .await?;
        assert_eq!(size(&crop)?, (128, 128));
        let fit = fixture
            .generator
            .generate(&nested(&video), &webp(ThumbnailVariant::Thumbnail, 128, ImageResizeType::Fit))
            .await?;
        assert_eq!(size(&fit)?, (128, 96));
        let large = fixture
            .generator
            .generate(&nested(&video), &webp(ThumbnailVariant::Thumbnail, 512, ImageResizeType::Crop))
            .await?;
        assert_eq!(size(&large)?, (512, 512));
        assert_eq!(fixture.generator.ffmpeg_runs(), 1, "the other shape and the filmstrip sizes come from the same run");
        Ok(())
    }

    #[tokio::test]
    async fn a_video_sprite_warms_the_same_companions_and_keeps_its_frames_test() -> TestResult {
        if skip_without_ffmpeg() {
            return Ok(());
        }
        let fixture = fixture(ThumbnailOptions::default()).await?;
        let video = fixture.dir.path().join("clip.mp4");
        write_video(&video, 10, 320, 240)?;

        let sprite = fixture
            .generator
            .generate(&nested(&video), &webp(ThumbnailVariant::Sprite, 128, ImageResizeType::Fit))
            .await?;
        assert_eq!(sprite.frame_count, Some(8));
        assert_eq!(size(&sprite)?, (8 * 128, 96));
        for (side, resize) in [(128, ImageResizeType::Crop), (256, ImageResizeType::Fit), (512, ImageResizeType::Crop)] {
            fixture.generator.generate(&nested(&video), &webp(ThumbnailVariant::Thumbnail, side, resize)).await?;
        }
        assert_eq!(fixture.generator.ffmpeg_runs(), 1);
        assert_eq!(fixture.generator.conversions(), 1);
        Ok(())
    }
}
