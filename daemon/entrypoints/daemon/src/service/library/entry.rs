use std::path::{Component, Path, PathBuf};

use omnius_pxna_engine::{
    model::{FsEntryDetail, FsEntryKind, NestedPathBuf},
    service::{MediaKind, NestedFsEntry, NestedFsEntrySummary},
};
use omnius_pxna_interface::{models, types::Nullable};

use crate::prelude::*;

/// engine の entry を、Library 相対の連結 path を持つ API の entry へ写す。
pub struct EntryConverter<'a> {
    library_root: &'a Path,
}

impl<'a> EntryConverter<'a> {
    pub fn new(library_root: &'a Path) -> Self {
        Self { library_root }
    }

    pub fn convert(&self, entry: &NestedFsEntry) -> Result<models::FileSystemEntry> {
        let segments = self.relative_segments(&entry.path)?;
        let path = format!("/{}", segments.join("/"));
        let name = segments.last().cloned().unwrap_or_default();

        let mut metadata = models::FileSystemMetadata::new(entry.meta.len, entry.meta.readonly);
        metadata.created_at = entry.meta.created.map(Nullable::Present);
        metadata.modified_at = entry.meta.modified.map(Nullable::Present);
        metadata.accessed_at = entry.meta.accessed.map(Nullable::Present);
        // 画像と動画は、サムネイルの生成の判定と同じ拡張子の表から MIME type を決める。クライアントは接頭辞でタイルの種類を決める。
        if matches!(entry.detail, FsEntryDetail::File { .. }) {
            metadata.content_type = MediaKind::content_type(&name).map(|content_type| Nullable::Present(content_type.to_string()));
        }

        let kind = Self::kind(entry.detail.kind());
        let mut converted = models::FileSystemEntry::new(path, name, metadata, kind);
        match &entry.detail {
            FsEntryDetail::File { archive } => {
                converted.file = Some(Nullable::Present(models::FileSystemEntryFileDetail::new(*archive)));
            }
            FsEntryDetail::Dir { has_children } => {
                let mut folder = models::FileSystemEntryFolderDetail::new();
                folder.has_children = has_children.map(Nullable::Present);
                converted.folder = Some(Nullable::Present(folder));
            }
            FsEntryDetail::Link { target, target_kind } => {
                converted.link = Some(Nullable::Present(self.link_detail(&entry.path, target.as_deref(), *target_kind)));
            }
        }

        Ok(converted)
    }

    pub fn convert_summary(&self, entry: &NestedFsEntrySummary) -> Result<models::FileSystemEntrySummary> {
        let segments = self.relative_segments(&entry.path)?;
        let path = format!("/{}", segments.join("/"));
        let name = segments.last().cloned().unwrap_or_default();
        Ok(models::FileSystemEntrySummary::new(path, name, Self::kind(entry.kind), entry.archive))
    }

    /// 最外の host path を Library 相対の segment に直し、archive の内側の segment を連結する。
    fn relative_segments(&self, path: &NestedPathBuf) -> Result<Vec<String>> {
        let mut iter = path.iter();
        let local = iter.next().unwrap_or_default();
        let relative = Path::new(local)
            .strip_prefix(self.library_root)
            .map_err(|_| Error::new(ErrorKind::UnexpectedError).with_message(format!("entry is outside of the library: {local}")))?;

        let mut segments = relative.iter().map(|segment| segment.to_string_lossy().into_owned()).collect::<Vec<_>>();
        for inner in iter {
            segments.extend(inner.split('/').filter(|segment| !segment.is_empty()).map(str::to_string));
        }
        Ok(segments)
    }

    /// Library の外を指す target や、archive 内の target は host の path を漏らさないよう null にする。
    /// target が存在しない壊れたリンクも、辿れない path を返さないよう null にする。
    fn link_detail(&self, link_path: &NestedPathBuf, target: Option<&str>, target_kind: Option<FsEntryKind>) -> models::FileSystemEntryLinkDetail {
        let target_path = match (link_path.rank(), target, target_kind) {
            (0, Some(target), Some(_)) => self.link_target(link_path, target),
            _ => None,
        };

        let mut link = models::FileSystemEntryLinkDetail::new();
        link.target_kind = Some(match (&target_path, target_kind) {
            (Some(_), Some(kind)) => Nullable::Present(Self::kind(kind)),
            _ => Nullable::Null,
        });
        link.target_path = Some(target_path.map(Nullable::Present).unwrap_or(Nullable::Null));
        link
    }

    fn link_target(&self, link_path: &NestedPathBuf, target: &str) -> Option<String> {
        let link_path = Path::new(link_path.iter().next()?);
        let joined = match Path::new(target).is_absolute() {
            true => PathBuf::from(target),
            false => link_path.parent()?.join(target),
        };

        let mut normalized = PathBuf::new();
        for component in joined.components() {
            match component {
                Component::ParentDir => {
                    normalized.pop();
                }
                Component::CurDir => {}
                other => normalized.push(other),
            }
        }

        let relative = normalized.strip_prefix(self.library_root).ok()?;
        let segments = relative.iter().map(|segment| segment.to_str()).collect::<Option<Vec<_>>>()?;
        Some(format!("/{}", segments.join("/")))
    }

    fn kind(kind: FsEntryKind) -> models::FileSystemEntryKind {
        match kind {
            FsEntryKind::File => models::FileSystemEntryKind::File,
            FsEntryKind::Dir => models::FileSystemEntryKind::Folder,
            FsEntryKind::Link => models::FileSystemEntryKind::Link,
        }
    }
}

#[cfg(test)]
mod tests {
    use testresult::TestResult;

    use super::*;

    fn nested(segments: &[&str]) -> TestResult<NestedPathBuf> {
        let mut path = NestedPathBuf::new();
        for segment in segments {
            path.push_str(segment)?;
        }
        Ok(path)
    }

    fn target_path(link: &models::FileSystemEntryLinkDetail) -> Option<&str> {
        match &link.target_path {
            Some(Nullable::Present(path)) => Some(path),
            _ => None,
        }
    }

    #[test]
    fn joins_archive_segments_to_library_relative_path_test() -> TestResult {
        let root = std::env::current_dir()?.join("library");
        let converter = EntryConverter::new(&root);

        let local = root.join("photos").join("a.7z");
        let path = nested(&[local.to_str().ok_or("non-utf8")?, "dir/x.png"])?;
        assert_eq!(converter.relative_segments(&path)?, ["photos", "a.7z", "dir", "x.png"]);

        let outside = std::env::current_dir()?.join("outside");
        let path = nested(&[outside.to_str().ok_or("non-utf8")?])?;
        assert_eq!(*converter.relative_segments(&path).unwrap_err().kind(), ErrorKind::UnexpectedError);
        Ok(())
    }

    #[test]
    fn hides_link_target_outside_of_library_test() -> TestResult {
        let root = std::env::current_dir()?.join("library");
        let converter = EntryConverter::new(&root);
        let link = root.join("photos").join("link");
        let link_path = nested(&[link.to_str().ok_or("non-utf8")?])?;

        let inside = converter.link_detail(&link_path, Some("../docs/a.txt"), Some(FsEntryKind::File));
        assert_eq!(target_path(&inside), Some("/docs/a.txt"));
        assert_eq!(inside.target_kind, Some(Nullable::Present(models::FileSystemEntryKind::File)));

        let escaped = converter.link_detail(&link_path, Some("../../outside/a.txt"), Some(FsEntryKind::File));
        assert_eq!(target_path(&escaped), None);
        assert_eq!(escaped.target_kind, Some(Nullable::Null));

        let outside = std::env::current_dir()?.join("outside");
        let absolute = converter.link_detail(&link_path, outside.to_str(), Some(FsEntryKind::Dir));
        assert_eq!(target_path(&absolute), None);

        let archived = nested(&[root.join("a.7z").to_str().ok_or("non-utf8")?, "link"])?;
        let in_archive = converter.link_detail(&archived, Some("x.png"), Some(FsEntryKind::File));
        assert_eq!(target_path(&in_archive), None);

        let broken = converter.link_detail(&link_path, Some("../docs/missing.txt"), None);
        assert_eq!(target_path(&broken), None);
        assert_eq!(broken.target_path, Some(Nullable::Null));
        assert_eq!(broken.target_kind, Some(Nullable::Null));
        Ok(())
    }
}
