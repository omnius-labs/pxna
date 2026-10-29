use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use chrono::{DateTime, Utc};
use futures::TryStreamExt as _;
use tokio::sync::Semaphore;

use omnius_pxna_engine::{
    model::{FsEntry, FsEntryDetail, NestedPathBuf},
    service::{ArchiveIdentity, FileStorageArchive, ImageInspector, MediaKind, MediaTools, SourceFingerprint, StorageService, WorkClass, WorkKind},
};

use super::{DateRules, TimelineBatch, TimelineDate, TimelineFileVersion, TimelineIndex, TimelineKind, TimelineRow, TimelineZone};
use crate::{config::LibraryConfig, prelude::*};

/// 1 回の書き込みにまとめるファイルと行の数。
const BATCH_SIZE: usize = 200;

/// 同じディレクトリの local の画像と動画の日時を、同時に読む数。
const LOCAL_READS: usize = 4;

/// 集計する範囲と、記録する世代。
pub struct TimelineScan<'a> {
    pub library: &'a LibraryConfig,
    /// 走査を始める Library 相対パス。Library のルートは `/` である。
    pub scope: &'a str,
    pub generation: i64,
    /// 変更した範囲だけの集計か。全体の集計は、集計の最中に付け替えた行を掃除で消さない。
    pub scoped: bool,
    /// 処理したファイルと archive の中の Entry の数。
    pub progress: &'a AtomicU64,
}

/// 走査で見つけた、読み直すかを決める前の local のファイル。
struct Candidate {
    local: PathBuf,
    relative: String,
    size: u64,
    modified: Option<DateTime<Utc>>,
    readonly: bool,
    content: CandidateKind,
}

enum CandidateKind {
    Media(MediaKind),
    Archive,
}

/// 開いている archive と、その中の Entry の Library 相対パスの接頭辞、日時の最後の手がかり。
struct OpenArchive {
    archive: FileStorageArchive,
    prefix: String,
    fallback: Option<DateTime<Utc>>,
    depth: usize,
}

/// Library を走査し、画像と動画の並びの日時を読んでタイムラインの索引へ書き込む。
///
/// 書き込む直前に、行が属するファイルが読んだときから変わっていないかを Library ごとの排他の中で確かめる。
/// daemon 自身の変更の反映も同じ排他の中で行うので、変更の前に読んだ結果は書き込まれない。
pub struct TimelineScanner {
    storage: Arc<StorageService>,
    index: Arc<TimelineIndex>,
    tools: MediaTools,
    zone: TimelineZone,
    rules: DateRules,
    extractions: Arc<Semaphore>,
    probes: Arc<Semaphore>,
    locks: Mutex<HashMap<String, Arc<TokioMutex<()>>>>,
    slots: Mutex<HashMap<String, Arc<TokioMutex<()>>>>,
}

impl TimelineScanner {
    pub fn new(
        storage: Arc<StorageService>,
        index: Arc<TimelineIndex>,
        tools: MediaTools,
        zone: TimelineZone,
        rules: DateRules,
        max_extractions: usize,
        max_probes: usize,
    ) -> Self {
        Self {
            storage,
            index,
            tools,
            zone,
            rules,
            extractions: Arc::new(Semaphore::new(max_extractions)),
            probes: Arc::new(Semaphore::new(max_probes)),
            locks: Mutex::new(HashMap::new()),
            slots: Mutex::new(HashMap::new()),
        }
    }

    /// Library ごとの排他。集計の書き込みと、daemon 自身の変更の反映が使う。
    pub fn lock(&self, library: &str) -> Arc<TokioMutex<()>> {
        self.locks.lock().entry(library.to_string()).or_default().clone()
    }

    /// 同じ Library で走る集計の順番。集計はフォルダ 1 つを読み終えるごとに手放し、待っている集計へ譲る。
    fn slot(&self, library: &str) -> Arc<TokioMutex<()>> {
        self.slots.lock().entry(library.to_string()).or_default().clone()
    }

    pub async fn scan(&self, target: &TimelineScan<'_>) -> Result<()> {
        self.scan_background(target).await
    }

    async fn scan_background(&self, target: &TimelineScan<'_>) -> Result<()> {
        let local = Self::local_path(target.library, target.scope);
        if !Self::is_hidden(target.scope) {
            match tokio::fs::symlink_metadata(&local).await {
                Ok(meta) if meta.is_dir() => self.walk(target, local).await?,
                Ok(meta) if meta.is_file() => {
                    let slot = self.slot(&target.library.id);
                    let _turn = slot.lock().await;
                    let parent = local.parent().map(Path::to_path_buf).unwrap_or_else(|| local.clone());
                    let name = local.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
                    let candidates = self
                        .list(target, &parent)
                        .await?
                        .into_iter()
                        .filter(|candidate| candidate.local.file_name().is_some_and(|candidate| candidate.to_string_lossy() == name))
                        .collect::<Vec<_>>();
                    self.process(target, candidates).await?;
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        self.index.sweep(&target.library.id, target.scope, target.generation, target.scoped).await
    }

    async fn walk(&self, target: &TimelineScan<'_>, root: PathBuf) -> Result<()> {
        let slot = self.slot(&target.library.id);
        let mut folders = vec![root];
        while let Some(folder) = folders.pop() {
            let _turn = slot.lock().await;
            let entries = match self.entries(&folder).await {
                Ok(entries) => entries,
                Err(error) => {
                    warn!(folder = %folder.display(), ?error, "timeline: cannot list folder");
                    continue;
                }
            };
            for entry in &entries {
                if matches!(entry.detail, FsEntryDetail::Dir { .. }) && !Self::name(&entry.path).starts_with('.') {
                    folders.push(PathBuf::from(&entry.path));
                }
            }
            let candidates = entries.into_iter().filter_map(|entry| self.candidate(target.library, entry)).collect::<Vec<_>>();
            self.process(target, candidates).await?;
        }
        Ok(())
    }

    async fn list(&self, target: &TimelineScan<'_>, folder: &Path) -> Result<Vec<Candidate>> {
        Ok(self.entries(folder).await?.into_iter().filter_map(|entry| self.candidate(target.library, entry)).collect())
    }

    /// local のフォルダの直下。storage の local driver を通し、走査中の symlink を追跡しない規則にそろえる。
    async fn entries(&self, folder: &Path) -> Result<Vec<FsEntry>> {
        let nested = Self::nested(folder)?;
        let entries = self.storage.find_entries(&nested, false, WorkClass::Background).await?.try_collect::<Vec<_>>().await?;
        Ok(entries
            .into_iter()
            .map(|entry| FsEntry {
                path: entry.path.iter().next().unwrap_or_default().to_string(),
                meta: entry.meta,
                detail: entry.detail,
            })
            .collect())
    }

    fn candidate(&self, library: &LibraryConfig, entry: FsEntry) -> Option<Candidate> {
        let name = Self::name(&entry.path);
        if name.starts_with('.') {
            return None;
        }
        let content = match entry.detail {
            FsEntryDetail::File { archive: true } => CandidateKind::Archive,
            FsEntryDetail::File { archive: false } => CandidateKind::Media(MediaKind::of_name(name)?),
            _ => return None,
        };
        let local = PathBuf::from(&entry.path);
        let relative = Self::relative(library, &local)?;
        Some(Candidate {
            local,
            relative,
            size: entry.meta.len,
            modified: entry.meta.modified,
            readonly: entry.meta.readonly,
            content,
        })
    }

    /// ディレクトリの候補を、読み直すものと世代だけを記録するものに分け、`BATCH_SIZE` 件ずつ書き込む。
    async fn process(&self, target: &TimelineScan<'_>, candidates: Vec<Candidate>) -> Result<()> {
        for chunk in candidates.chunks(BATCH_SIZE) {
            let paths = chunk.iter().map(|candidate| candidate.relative.clone()).collect::<Vec<_>>();
            let known = self.index.versions(&target.library.id, &paths).await?;
            let mut batch = TimelineBatch::default();
            let mut media = Vec::new();
            for candidate in chunk {
                let Some(version) = Self::listed_version(candidate).await else {
                    continue;
                };
                if known.get(&candidate.relative) == Some(&version) {
                    batch.touched.push(candidate.relative.clone());
                    continue;
                }
                match candidate.content {
                    CandidateKind::Media(kind) => media.push((candidate, kind, version)),
                    CandidateKind::Archive => {
                        let TimelineFileVersion::Archive(identity) = version else {
                            continue;
                        };
                        self.read_archive(target, candidate, identity).await?;
                    }
                }
            }

            let mut rows = Vec::with_capacity(media.len());
            for group in media.chunks(LOCAL_READS) {
                rows.extend(futures::future::join_all(group.iter().map(|(candidate, kind, version)| self.read_media(target.library, candidate, *kind, version))).await);
            }

            let lock = self.lock(&target.library.id);
            let _guard = lock.lock().await;
            for (candidate, version, row) in rows {
                // 読んでいる間に変わったファイルは書き込まない。変更の反映か、次の集計が扱う。
                if Self::version(candidate).await.as_ref() != Some(&version) {
                    continue;
                }
                batch.replaced.push(candidate.relative.clone());
                batch.files.push((candidate.relative.clone(), version));
                batch.rows.extend(row);
            }
            if !batch.is_empty() {
                self.index.apply(&target.library.id, target.generation, &batch).await?;
            }
            target.progress.fetch_add(chunk.len() as u64, Ordering::Relaxed);
        }
        Ok(())
    }

    /// local の画像か動画から並びの日時を読み、Entry の行を作る。
    async fn read_media<'a>(
        &self,
        library: &LibraryConfig,
        candidate: &'a Candidate,
        kind: MediaKind,
        version: &TimelineFileVersion,
    ) -> (&'a Candidate, TimelineFileVersion, Option<TimelineRow>) {
        let file_name = self.rules.date(&self.zone, &library.id, &candidate.relative, Self::name(&candidate.relative));
        // A file whose name decided the date is not inspected for Exif or creation_time.
        let content = match file_name {
            Some(_) => None,
            None => self.content_date(kind, &candidate.local).await,
        };
        let row = TimelineDate::choose(&self.zone, file_name, content, candidate.modified, None).map(|date| TimelineRow {
            path: candidate.relative.clone(),
            file_path: candidate.relative.clone(),
            kind: Self::kind(kind),
            date,
            size: candidate.size,
            modified: candidate.modified,
            readonly: candidate.readonly,
        });
        (candidate, version.clone(), row)
    }

    /// archive を開き、中と入れ子の archive の画像と動画を block ごとに読んで、その archive の行を置き換える。
    ///
    /// 開けない archive は、中の Entry を持たない archive として記録し、変わるまで読み直さない。
    async fn read_archive(&self, target: &TimelineScan<'_>, candidate: &Candidate, identity: String) -> Result<()> {
        let file = (candidate.relative.clone(), TimelineFileVersion::Archive(identity.clone()));
        let local = candidate.local.to_string_lossy().into_owned();
        let archive = match self.storage.open_archive(&local, WorkClass::Background).await {
            Ok(archive) => archive,
            Err(error) => {
                debug!(archive = %local, ?error, "timeline: cannot open archive");
                let batch = TimelineBatch {
                    replaced: vec![candidate.relative.clone()],
                    files: vec![file],
                    ..Default::default()
                };
                self.commit_archive(target, candidate, &identity, &batch).await?;
                return Ok(());
            }
        };

        let mut batch = TimelineBatch {
            replaced: vec![candidate.relative.clone()],
            ..Default::default()
        };
        let mut stack = vec![OpenArchive {
            archive,
            prefix: candidate.relative.clone(),
            fallback: candidate.modified,
            depth: 1,
        }];
        let max_depth = self.storage.archive_options().max_nested_depth;
        while let Some(open) = stack.pop() {
            let entries = open
                .archive
                .files()
                .into_iter()
                .filter(|entry| !Self::is_hidden(&entry.path))
                .map(|entry| (entry.path.clone(), entry))
                .collect::<HashMap<_, _>>();
            let wanted = entries
                .values()
                .filter(|entry| Self::archive_media(entry).is_some() || (Self::is_archive(entry) && open.depth < max_depth))
                .map(|entry| entry.path.clone())
                .collect::<Vec<_>>();
            for group in open.archive.block_groups(wanted) {
                let files = match open.archive.extract_group(&group, self.extractions.clone(), WorkClass::Background).await {
                    Ok(files) => files,
                    Err(error) if *error.kind() == omnius_pxna_engine::ErrorKind::SourceChanged => {
                        debug!(archive = %local, "timeline: archive changed while reading");
                        return Ok(());
                    }
                    Err(error) => {
                        debug!(archive = %local, ?error, "timeline: cannot extract archive block");
                        HashMap::new()
                    }
                };
                for path in group.paths() {
                    let Some(entry) = entries.get(path) else {
                        continue;
                    };
                    let relative = format!("{}/{}", open.prefix, entry.path);
                    if let Some(kind) = Self::archive_media(entry) {
                        let file_name = self.rules.date(&self.zone, &target.library.id, &relative, Self::name(&relative));
                        let content = match (file_name, files.get(path)) {
                            (Some(_), _) | (None, None) => None,
                            (None, Some(file)) => self.content_date(kind, file.path()).await,
                        };
                        if let Some(date) = TimelineDate::choose(&self.zone, file_name, content, entry.meta.modified, open.fallback) {
                            batch.rows.push(TimelineRow {
                                path: relative,
                                file_path: candidate.relative.clone(),
                                kind: Self::kind(kind),
                                date,
                                size: entry.meta.len,
                                modified: entry.meta.modified,
                                readonly: true,
                            });
                        }
                        target.progress.fetch_add(1, Ordering::Relaxed);
                    } else if let Some(source) = files.get(path) {
                        match open.archive.open_inner(path, source.clone(), WorkClass::Background).await {
                            Ok(inner) => stack.push(OpenArchive {
                                archive: inner,
                                prefix: relative,
                                fallback: entry.meta.modified.or(open.fallback),
                                depth: open.depth + 1,
                            }),
                            Err(error) => debug!(archive = %relative, ?error, "timeline: cannot open nested archive"),
                        }
                    }
                }
                if batch.rows.len() >= BATCH_SIZE {
                    // 途中で公開する行も、ファイルの行（読み終えていないことを示す、どの identity にも一致しない値）に属させる。
                    // 中断しても、次の集計の読み直しと掃除の対象になる。
                    batch.files.push((candidate.relative.clone(), TimelineFileVersion::Archive(String::new())));
                    if !self.commit_archive(target, candidate, &identity, &batch).await? {
                        return Ok(());
                    }
                    batch = TimelineBatch::default();
                }
            }
        }
        batch.files.push(file);
        self.commit_archive(target, candidate, &identity, &batch).await?;
        Ok(())
    }

    /// archive の行を書き込む。archive が読み始めたときから変わっていれば書き込まず、`false` を返す。
    async fn commit_archive(&self, target: &TimelineScan<'_>, candidate: &Candidate, identity: &str, batch: &TimelineBatch) -> Result<bool> {
        let lock = self.lock(&target.library.id);
        let _guard = lock.lock().await;
        if Self::version(candidate).await != Some(TimelineFileVersion::Archive(identity.to_string())) {
            return Ok(false);
        }
        self.index.apply(&target.library.id, target.generation, batch).await?;
        Ok(true)
    }

    /// 中身から読む撮影日時か `creation_time`。読めなければ `None` にする。
    async fn content_date(&self, kind: MediaKind, file: &Path) -> Option<TimelineDate> {
        match kind {
            MediaKind::Image | MediaKind::FfmpegImage | MediaKind::HeifImage => {
                let path = file.to_path_buf();
                let exif = self
                    .storage
                    .work_budget()
                    .io()
                    .ticket(WorkClass::Background)
                    .counted(WorkKind::Exif)
                    .acquire()
                    .await
                    .ok()?
                    .run_blocking(move || ImageInspector::read_exif(&path))
                    .await
                    .ok()?;
                exif.taken_at.as_deref().and_then(TimelineDate::from_exif)
            }
            MediaKind::Video => {
                let background = self
                    .storage
                    .work_budget()
                    .io()
                    .ticket(WorkClass::Background)
                    .counted(WorkKind::Probe)
                    .acquire()
                    .await
                    .ok()?;
                let permit = self.probes.clone().acquire_owned().await.ok()?;
                let tools = self.tools.clone();
                let path = file.to_path_buf();
                let result = background
                    .run_with(permit, async move { tools.probe_creation_time(&path, WorkClass::Background).await })
                    .await
                    .ok()?;
                match result {
                    Ok(time) => time.and_then(|time| TimelineDate::from_creation_time(&self.zone, time)),
                    Err(error) => {
                        debug!(file = %file.display(), ?error, "timeline: cannot probe creation_time");
                        None
                    }
                }
            }
        }
    }

    /// 走査で得た値から決める、読み直しを省く判定に使う値。archive は `ArchiveIdentity` を計算するために stat し直す。
    async fn listed_version(candidate: &Candidate) -> Option<TimelineFileVersion> {
        match candidate.content {
            CandidateKind::Media(_) => Some(TimelineFileVersion::Media {
                size: candidate.size,
                modified_ns: Self::nanos(candidate.modified),
            }),
            CandidateKind::Archive => Self::version(candidate).await,
        }
    }

    /// 今のファイルの、読み直しを省く判定に使う値。ファイルがなくなっていれば `None` にする。
    async fn version(candidate: &Candidate) -> Option<TimelineFileVersion> {
        Self::file_version(&candidate.local, matches!(candidate.content, CandidateKind::Archive)).await
    }

    async fn file_version(local: &Path, archive: bool) -> Option<TimelineFileVersion> {
        let meta = tokio::fs::metadata(local).await.ok()?;
        if !meta.is_file() {
            return None;
        }
        if archive {
            let fingerprint = SourceFingerprint::read(local).await.ok()?;
            return Some(TimelineFileVersion::Archive(ArchiveIdentity::from_local(&local.to_string_lossy(), &fingerprint).to_hex()));
        }
        Some(TimelineFileVersion::Media {
            size: meta.len(),
            modified_ns: Self::nanos(meta.modified().ok().map(DateTime::<Utc>::from)),
        })
    }

    /// 索引が覚えているファイルの値が、今のファイルと違うか。上書きされたファイルの行を、応答の前に消すかを決める。
    /// 索引に行のないファイルと、名前の変更で付け替えた行（値は同じ）は、違わないものとして扱う。
    pub async fn is_replaced(&self, library: &LibraryConfig, relative: &str) -> Result<bool> {
        let recorded = self.index.versions(&library.id, &[relative.to_string()]).await?;
        let Some(recorded) = recorded.get(relative) else {
            return Ok(false);
        };
        let archive = matches!(recorded, TimelineFileVersion::Archive(_));
        Ok(Self::file_version(&Self::local_path(library, relative), archive).await.as_ref() != Some(recorded))
    }

    /// 更新時刻を持たない filesystem では 0 にする。
    fn nanos(time: Option<DateTime<Utc>>) -> i64 {
        time.and_then(|time| time.timestamp_nanos_opt()).unwrap_or(0)
    }

    fn archive_media(entry: &FsEntry) -> Option<MediaKind> {
        match entry.detail {
            FsEntryDetail::File { archive: false } => MediaKind::of_name(Self::name(&entry.path)),
            _ => None,
        }
    }

    fn is_archive(entry: &FsEntry) -> bool {
        matches!(entry.detail, FsEntryDetail::File { archive: true })
    }

    fn kind(kind: MediaKind) -> TimelineKind {
        match kind {
            MediaKind::Image | MediaKind::FfmpegImage | MediaKind::HeifImage => TimelineKind::Image,
            MediaKind::Video => TimelineKind::Video,
        }
    }

    /// path の最後の segment。
    fn name(path: &str) -> &str {
        path.rsplit(['/', '\\']).next().unwrap_or_default()
    }

    /// 名前が `.` で始まる segment を含むか。
    fn is_hidden(path: &str) -> bool {
        path.split('/').any(|segment| segment.starts_with('.') && segment != "." && segment != "..")
    }

    fn local_path(library: &LibraryConfig, relative: &str) -> PathBuf {
        let mut path = library.path.clone();
        for segment in relative.split('/').filter(|segment| !segment.is_empty()) {
            path.push(segment);
        }
        path
    }

    fn relative(library: &LibraryConfig, local: &Path) -> Option<String> {
        let relative = local.strip_prefix(&library.path).ok()?;
        let segments = relative.iter().map(|segment| segment.to_str()).collect::<Option<Vec<_>>>()?;
        Some(format!("/{}", segments.join("/")))
    }

    fn nested(path: &Path) -> Result<NestedPathBuf> {
        let mut nested = NestedPathBuf::new();
        nested
            .push_str(path.to_str().ok_or_else(|| Error::new(ErrorKind::InvalidInput).with_message("path is not UTF-8"))?)
            .map_err(|error| Error::from_error(error, ErrorKind::InvalidInput))?;
        Ok(nested)
    }
}

#[cfg(test)]
mod budget_tests {
    use super::*;
    use omnius_core_base::{clock::ClockUtc, tsid::TsidProviderImpl};
    use omnius_pxna_engine::service::{ArchiveRuntime, LocalStorageDriver, MediaOptions, WorkBudget};
    use rand::{SeedableRng as _, rngs::ChaCha20Rng};
    use testresult::TestResult;

    #[tokio::test]
    async fn exif_and_probe_production_paths_share_background_budget_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let budget = Arc::new(WorkBudget::default());
        let tsid = Arc::new(Mutex::new(TsidProviderImpl::new(ClockUtc, ChaCha20Rng::seed_from_u64(0), 16)));
        let storage = Arc::new(StorageService::new(
            Arc::new(LocalStorageDriver::with_budget(tsid, budget.clone())),
            Arc::new(ArchiveRuntime::default().with_budget(budget.clone())),
        ));
        let index = Arc::new(TimelineIndex::open(&dir.path().join("timeline")).await?);
        let scanner = TimelineScanner::new(
            storage,
            index,
            MediaTools::new(
                MediaOptions {
                    // A missing executable is a deterministic failed probe after admission.
                    ffprobe_path: dir.path().join("absent-ffprobe"),
                    ..Default::default()
                },
                std::time::Duration::from_secs(1),
            ),
            TimelineZone::new(Some("UTC"))?,
            DateRules::new(&[], &[])?,
            1,
            1,
        );
        let file = dir.path().join("image.jpg");
        std::fs::write(&file, b"invalid image")?;
        let held = budget.io().ticket(WorkClass::Background).counted(WorkKind::Summary).acquire().await?;
        let exif = scanner.content_date(MediaKind::Image, &file);
        tokio::pin!(exif);
        assert!(futures::poll!(&mut exif).is_pending());
        assert_eq!(budget.counts(WorkKind::Exif), (0, 0));
        drop(held);
        assert!(exif.await.is_none());
        assert_eq!(budget.counts(WorkKind::Exif), (1, 1));
        let probe_slot = scanner.probes.acquire().await?;
        let probe = scanner.content_date(MediaKind::Video, &file);
        tokio::pin!(probe);
        assert!(futures::poll!(&mut probe).is_pending());
        assert_eq!(budget.counts(WorkKind::Probe), (1, 0));
        assert!(budget.try_probe().is_none());
        drop(probe_slot);
        assert!(probe.await.is_none());
        assert_eq!(budget.counts(WorkKind::Probe), (1, 1));
        Ok(())
    }

    #[tokio::test]
    async fn local_enumeration_runs_on_the_background_budget_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let folder = dir.path().join("photos");
        std::fs::create_dir(&folder)?;
        std::fs::write(folder.join("a.jpg"), b"")?;
        let budget = Arc::new(WorkBudget::default());
        let tsid = Arc::new(Mutex::new(TsidProviderImpl::new(ClockUtc, ChaCha20Rng::seed_from_u64(0), 16)));
        let storage = Arc::new(StorageService::new(
            Arc::new(LocalStorageDriver::with_budget(tsid, budget.clone())),
            Arc::new(ArchiveRuntime::default().with_budget(budget.clone())),
        ));
        let index = Arc::new(TimelineIndex::open(&dir.path().join("timeline")).await?);
        let scanner = TimelineScanner::new(
            storage,
            index,
            MediaTools::new(MediaOptions::default(), std::time::Duration::from_secs(1)),
            TimelineZone::new(Some("UTC"))?,
            DateRules::new(&[], &[])?,
            1,
            1,
        );

        let held = budget.io().ticket(WorkClass::Background).counted(WorkKind::Summary).acquire().await?;
        let entries = scanner.entries(&folder);
        tokio::pin!(entries);
        // metadata の取得を越えさせ、io の枠を待つところまで進める。Foreground なら枠は空いているので、ここで Ready になる。
        let mut queued = false;
        for _ in 0..200 {
            assert!(futures::poll!(&mut entries).is_pending(), "枠を保持している間は列挙を終えない");
            if budget.io().waiting(WorkClass::Background) == 1 {
                queued = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        assert!(queued, "列挙は Background の枠を Background として待つ");
        assert_eq!(budget.io().counts(WorkKind::Listing), (0, 0), "枠を得るまで数えない");
        drop(held);
        let entries = entries.await?;
        assert!(entries.iter().any(|entry| entry.path.ends_with("a.jpg")), "列挙は Background の枠を得てから終わる");
        Ok(())
    }
}
