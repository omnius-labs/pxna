use super::StorageService;
use crate::core::nfs::{ArchiveFormat, ArchiveIdentity, CachedArchiveFolder, SourceFingerprint};
use chrono::{DateTime, Utc};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime},
};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChildProbeVersion {
    Local(DateTime<Utc>),
    Archive(ArchiveIdentity),
}

/// local の属性と既存の archive 索引だけを読む。archive を新たに開かない。
pub struct ChildProbeTarget {
    version_path: PathBuf,
    source: SourceFingerprint,
    local: PathBuf,
    version: ChildProbeVersion,
    archive: Option<CachedArchiveFolder>,
}
impl ChildProbeTarget {
    pub fn version(&self) -> &ChildProbeVersion {
        &self.version
    }
    pub fn unchanged(&self) -> bool {
        std::fs::metadata(&self.version_path)
            .ok()
            .and_then(|meta| SourceFingerprint::from_metadata(&meta).ok())
            .as_ref()
            == Some(&self.source)
    }
    pub fn source_version(&self) -> String {
        match self.version {
            ChildProbeVersion::Local(modified) => format!("local:{}:{}", modified.timestamp(), modified.timestamp_subsec_nanos()),
            ChildProbeVersion::Archive(identity) => format!("archive:{}", identity.to_hex()),
        }
    }
    pub fn has_children(&self, include_hidden: bool, deadline: Instant, cancellation: &CancellationToken) -> Option<bool> {
        let alive = || !cancellation.is_cancelled() && Instant::now() < deadline;
        if let Some(archive) = &self.archive {
            return archive.has_children(include_hidden, alive);
        }
        if !alive() {
            return None;
        }
        let mut entries = std::fs::read_dir(&self.local).ok()?;
        for _ in 0..128 {
            if !alive() {
                return None;
            }
            let Some(entry) = entries.next() else {
                return Some(false);
            };
            let entry = entry.ok()?;
            let name = entry.file_name();
            let name = name.to_str()?;
            if !include_hidden && name.starts_with('.') {
                continue;
            }
            let kind = entry.file_type().ok()?;
            if !alive() {
                return None;
            }
            if kind.is_dir() || (kind.is_file() && ArchiveFormat::supports_path(name)) {
                return Some(true);
            }
            if !kind.is_file() && !kind.is_symlink() {
                return None;
            }
        }
        None
    }
}
impl StorageService {
    pub fn child_probe_target(&self, root: &Path, segments: &[String], deadline: Instant, cancellation: &CancellationToken) -> Option<ChildProbeTarget> {
        let alive = || !cancellation.is_cancelled() && Instant::now() < deadline;
        if !alive() {
            return None;
        }
        let mut local = root.to_path_buf();
        local.extend(segments);
        if let Ok(meta) = std::fs::symlink_metadata(&local) {
            if meta.is_dir() {
                let modified = meta.modified().ok()?;
                if SystemTime::now().duration_since(modified).ok()? < Duration::from_secs(2) {
                    return None;
                }
                return Some(ChildProbeTarget {
                    version_path: local.clone(),
                    source: SourceFingerprint::from_metadata(&meta).ok()?,
                    local,
                    version: ChildProbeVersion::Local(modified.into()),
                    archive: None,
                });
            }
            if meta.is_symlink() {
                return None;
            }
        }
        let mut outer = root.to_path_buf();
        for (index, segment) in segments.iter().enumerate() {
            if !alive() {
                return None;
            }
            outer.push(segment);
            let meta = std::fs::symlink_metadata(&outer).ok()?;
            if meta.is_dir() {
                continue;
            }
            if !meta.is_file() {
                return None;
            }
            let source = SourceFingerprint::from_metadata(&meta).ok()?;
            let identity = ArchiveIdentity::from_local(outer.to_str()?, &source);
            let archive = self.archive.cached_folder(identity, &segments[index + 1..])?;
            return Some(ChildProbeTarget {
                version_path: outer.clone(),
                source,
                local: outer,
                version: ChildProbeVersion::Archive(archive.identity()),
                archive: Some(archive),
            });
        }
        None
    }
}
