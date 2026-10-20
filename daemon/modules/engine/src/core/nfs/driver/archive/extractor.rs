use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};

use tokio::sync::Semaphore;

use crate::{core::nfs::MaterializedFile, prelude::*};

use super::{
    ArchiveError, ArchiveReader, InspectedArchive,
    identity::{ArchiveIdentity, EntryCacheKey, SourceFingerprint},
    index::{ArchiveBlockId, ArchiveEntry},
    options::ArchiveRuntime,
};

/// 1 回の decode で block から取り出した file の集まり。
///
/// 同じ block を待っていた要求者は、ここから自分の entry の clone を受け取る。
/// 集まりが drop されると、cache へ登録できなかった entry の一時 file は消える。
pub(super) struct ExtractedBlock {
    files: HashMap<String, MaterializedFile>,
}

pub(super) struct ArchiveExtractor;

impl ArchiveExtractor {
    pub(super) async fn materialize(
        identity: ArchiveIdentity,
        source_path: PathBuf,
        source_check: Option<SourceFingerprint>,
        runtime: Arc<ArchiveRuntime>,
        inspected: Arc<InspectedArchive>,
        entry: ArchiveEntry,
    ) -> Result<MaterializedFile> {
        let options = runtime.options();
        if entry.fs_entry.meta.len > options.max_materialized_file_size {
            return Err(ArchiveError::limit_exceeded(format!(
                "archive entry exceeds materialized file limit: {}",
                entry.fs_entry.path
            )));
        }

        let Some(block_id) = entry.block_id else {
            if entry.fs_entry.meta.len == 0 {
                return MaterializedFile::empty_temp();
            }
            return Err(ArchiveError::invalid_format(format!("archive file has no data block: {}", entry.fs_entry.path)));
        };
        let block_size = match (&inspected.reader, block_id) {
            (ArchiveReader::Zip(_), ArchiveBlockId::Zip(_)) => entry.fs_entry.meta.len,
            (ArchiveReader::SevenZip(reader), ArchiveBlockId::SevenZip(index)) => reader.block_size(index)?,
            _ => return Err(ArchiveError::invalid_format("archive entry block belongs to a different format")),
        };
        if block_size > options.max_block_extract_bytes {
            return Err(ArchiveError::limit_exceeded(format!("archive block exceeds extract limit: {block_size}")));
        }

        let target = entry.fs_entry.path;
        if let Some(cache) = runtime.extraction_cache() {
            let key = EntryCacheKey::new(&identity, &target);
            if let Some(hit) = cache.lookup(&key, &identity, &target).await? {
                return Ok(hit);
            }
        }

        let work = Self::extract_block(identity, source_path, source_check, runtime.clone(), inspected, block_id, target.clone());
        let block = runtime
            .block_flights()
            .run((identity, block_id), crate::core::work::WorkClass::current().scope(work))
            .await?;
        block
            .files
            .get(&target)
            .cloned()
            .ok_or_else(|| ArchiveError::invalid_format(format!("archive block did not produce target entry: {target}")))
    }
}

impl ArchiveExtractor {
    /// block を 1 回 decode し、検証した file を cache へ登録する。
    ///
    /// 容量が block 全体に足りない場合も要求された entry を確実に返せるよう、要求された entry を先に登録する。
    async fn extract_block(
        identity: ArchiveIdentity,
        source_path: PathBuf,
        source_check: Option<SourceFingerprint>,
        runtime: Arc<ArchiveRuntime>,
        inspected: Arc<InspectedArchive>,
        block_id: ArchiveBlockId,
        requested: String,
    ) -> Result<ExtractedBlock> {
        if let Some(fingerprint) = &source_check {
            fingerprint.ensure_unchanged(&source_path).await?;
        }

        let staging_dir = runtime.staging_dir();
        let check_path = source_path.clone();
        let reader = inspected.reader.clone();
        let requested_len = inspected.index.get(&requested).map(|entry| entry.fs_entry.meta.len).unwrap_or(0);
        let requested_path = requested.clone();
        let outputs = runtime
            .run_blocking(move || match (reader, block_id) {
                (ArchiveReader::Zip(reader), ArchiveBlockId::Zip(index)) => {
                    let output = reader.extract(&source_path, index, requested_len, &staging_dir)?;
                    Ok(vec![(requested_path, output)])
                }
                (ArchiveReader::SevenZip(reader), ArchiveBlockId::SevenZip(index)) => reader.extract(&source_path, index, &staging_dir),
                _ => Err(ArchiveError::invalid_format("archive entry block belongs to a different format")),
            })
            .await?;
        #[cfg(test)]
        runtime.record_decoded_block();

        // decode 中に source が変わった結果は登録しない。outputs を drop すると staging も消える。
        if let Some(fingerprint) = &source_check {
            fingerprint.ensure_unchanged(&check_path).await?;
        }

        let mut verified = Vec::with_capacity(outputs.len());
        for (path, output) in outputs {
            let Some(entry) = inspected.index.get(&path) else {
                continue;
            };
            if !entry.fs_entry.is_file() || entry.block_id != Some(block_id) {
                continue;
            }
            let size = output.as_file().metadata()?.len();
            if size != entry.fs_entry.meta.len {
                return Err(ArchiveError::invalid_format(format!(
                    "archive entry size mismatch: {path}: expected {}, got {size}",
                    entry.fs_entry.meta.len
                )));
            }
            verified.push((path, output, size));
        }
        verified.sort_by_key(|(path, _, _)| *path != requested);

        let mut files = HashMap::with_capacity(verified.len());
        for (path, output, size) in verified {
            let staged = output.into_temp_path().keep().map_err(|error| Error::from(error.error))?;
            let materialized = match runtime.extraction_cache() {
                Some(cache) => cache.publish(&EntryCacheKey::new(&identity, &path), &identity, &path, staged, size).await,
                None => MaterializedFile::owned_temp(staged),
            };
            files.insert(path, materialized);
        }

        Ok(ExtractedBlock { files })
    }

    /// 1 つの block を 1 回だけ展開し、`wanted` のうちその block に属する file を一時 file として返す。
    ///
    /// タイムラインの集計が使う。extraction cache を引かず登録もせず、`permits` の上限で decoder を動かす。
    /// 1 つの file の上限を超える entry は返さない。
    pub(super) async fn extract_uncached(
        source_path: PathBuf,
        source_check: Option<SourceFingerprint>,
        runtime: Arc<ArchiveRuntime>,
        inspected: Arc<InspectedArchive>,
        block_id: Option<ArchiveBlockId>,
        wanted: &[String],
        permits: Arc<Semaphore>,
    ) -> Result<HashMap<String, MaterializedFile>> {
        let options = runtime.options();
        let wanted = wanted
            .iter()
            .filter(|path| {
                inspected
                    .index
                    .get(path)
                    .is_some_and(|entry| entry.fs_entry.is_file() && entry.block_id == block_id && entry.fs_entry.meta.len <= options.max_materialized_file_size)
            })
            .cloned()
            .collect::<HashSet<_>>();
        let mut files = HashMap::with_capacity(wanted.len());

        // block を持たない file は空の file だけである。
        let Some(block_id) = block_id else {
            for path in wanted {
                files.insert(path, MaterializedFile::empty_temp()?);
            }
            return Ok(files);
        };
        let Some(first) = wanted.iter().min().cloned() else {
            return Ok(files);
        };
        let block_size = match (&inspected.reader, block_id) {
            (ArchiveReader::Zip(_), ArchiveBlockId::Zip(_)) => inspected.index.get(&first).map(|entry| entry.fs_entry.meta.len).unwrap_or(0),
            (ArchiveReader::SevenZip(reader), ArchiveBlockId::SevenZip(index)) => reader.block_size(index)?,
            _ => return Err(ArchiveError::invalid_format("archive entry block belongs to a different format")),
        };
        if block_size > options.max_block_extract_bytes {
            return Err(ArchiveError::limit_exceeded(format!("archive block exceeds extract limit: {block_size}")));
        }

        if let Some(fingerprint) = &source_check {
            fingerprint.ensure_unchanged(&source_path).await?;
        }
        let staging_dir = runtime.staging_dir();
        let check_path = source_path.clone();
        let reader = inspected.reader.clone();
        let outputs = runtime
            .run_blocking_with(&permits, move || match (reader, block_id) {
                (ArchiveReader::Zip(reader), ArchiveBlockId::Zip(index)) => {
                    let output = reader.extract(&source_path, index, block_size, &staging_dir)?;
                    Ok(vec![(first, output)])
                }
                (ArchiveReader::SevenZip(reader), ArchiveBlockId::SevenZip(index)) => reader.extract(&source_path, index, &staging_dir),
                _ => Err(ArchiveError::invalid_format("archive entry block belongs to a different format")),
            })
            .await?;
        #[cfg(test)]
        runtime.record_decoded_block();
        if let Some(fingerprint) = &source_check {
            fingerprint.ensure_unchanged(&check_path).await?;
        }

        for (path, output) in outputs {
            if !wanted.contains(&path) {
                continue;
            }
            let expected = inspected.index.get(&path).map(|entry| entry.fs_entry.meta.len).unwrap_or(0);
            let size = output.as_file().metadata()?.len();
            if size != expected {
                return Err(ArchiveError::invalid_format(format!(
                    "archive entry size mismatch: {path}: expected {expected}, got {size}"
                )));
            }
            files.insert(path, MaterializedFile::keep_temp(output)?);
        }
        Ok(files)
    }
}
