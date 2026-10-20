use std::collections::BTreeMap;

use chrono::{DateTime, Utc};

use crate::prelude::*;

use super::{ArchiveError, format::ArchiveFormat};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum ArchiveBlockId {
    Zip(usize),
    SevenZip(usize),
}

#[derive(Debug, Clone)]
pub(super) struct ArchiveEntry {
    pub fs_entry: FsEntry,
    pub block_id: Option<ArchiveBlockId>,
}

impl ArchiveEntry {
    pub fn new(path: String, kind: FsEntryKind, size: u64, modified: Option<DateTime<Utc>>, block_id: Option<ArchiveBlockId>) -> Self {
        let detail = match kind {
            FsEntryKind::File => FsEntryDetail::File {
                archive: ArchiveFormat::supports_path(&path),
            },
            FsEntryKind::Dir => FsEntryDetail::Dir { has_children: None },
            FsEntryKind::Link => FsEntryDetail::Link { target: None, target_kind: None },
        };
        Self {
            fs_entry: FsEntry {
                path,
                meta: FsMetadata {
                    len: size,
                    readonly: true,
                    created: None,
                    modified,
                    accessed: None,
                },
                detail,
            },
            block_id,
        }
    }

    fn synthetic_dir(path: String) -> Self {
        Self::new(path, FsEntryKind::Dir, 0, None, None)
    }

    pub fn path(&self) -> &str {
        &self.fs_entry.path
    }
}

#[derive(Debug)]
pub(super) struct ArchiveIndex {
    entries: Vec<ArchiveEntry>,
}

impl ArchiveIndex {
    pub fn build(raw_entries: Vec<ArchiveEntry>, max_entry_count: usize) -> Result<Self> {
        if raw_entries.len() > max_entry_count {
            return Err(ArchiveError::limit_exceeded(format!("archive has more than {max_entry_count} entries")));
        }

        let mut by_path: BTreeMap<String, ArchiveEntry> = BTreeMap::new();
        for entry in raw_entries {
            let path = entry.path().to_string();
            if let Some(previous) = by_path.get(&path) {
                let message = if previous.fs_entry.kind() == entry.fs_entry.kind() {
                    format!("duplicate archive entry path: {path}")
                } else {
                    format!("archive entry kind collision: {path}")
                };
                return Err(ArchiveError::invalid_format(message));
            }
            by_path.insert(path, entry);
        }

        let paths = by_path.keys().cloned().collect::<Vec<_>>();
        for path in paths {
            let segments = path.split('/').collect::<Vec<_>>();
            for end in 1..segments.len() {
                let parent = segments[..end].join("/");
                if let Some(existing) = by_path.get(&parent) {
                    if !existing.fs_entry.is_dir() {
                        return Err(ArchiveError::invalid_format(format!("archive entry kind collision: {parent}")));
                    }
                } else {
                    by_path.insert(parent.clone(), ArchiveEntry::synthetic_dir(parent));
                }
            }
        }
        by_path.insert(String::new(), ArchiveEntry::synthetic_dir(String::new()));

        if by_path.len() > max_entry_count {
            return Err(ArchiveError::limit_exceeded(format!(
                "archive has more than {max_entry_count} entries after parent synthesis"
            )));
        }
        Ok(Self {
            entries: by_path.into_values().collect(),
        })
    }

    pub fn estimated_size(&self) -> u64 {
        self.entries.iter().map(|entry| (std::mem::size_of::<ArchiveEntry>() + entry.path().len()) as u64).sum()
    }

    pub fn get(&self, path: &str) -> Option<&ArchiveEntry> {
        self.entries.binary_search_by(|entry| entry.path().cmp(path)).ok().map(|index| &self.entries[index])
    }

    pub fn descendants<'a>(&'a self, path: &str, recursive: bool) -> impl Iterator<Item = &'a ArchiveEntry> + 'a {
        let base = path.to_string();
        let prefix = if path.is_empty() { String::new() } else { format!("{path}/") };
        let start = if prefix.is_empty() {
            0
        } else {
            self.entries.partition_point(|entry| entry.path() < prefix.as_str())
        };
        self.entries[start..]
            .iter()
            .take_while(move |entry| prefix.is_empty() || entry.path().starts_with(&prefix))
            .filter(move |entry| {
                if entry.path().is_empty() || entry.path() == base {
                    return false;
                }
                if recursive {
                    return true;
                }
                let relative = if base.is_empty() { entry.path() } else { &entry.path()[base.len() + 1..] };
                !relative.contains('/')
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str) -> ArchiveEntry {
        ArchiveEntry::new(path.to_string(), FsEntryKind::File, 1, None, Some(ArchiveBlockId::Zip(0)))
    }

    #[test]
    fn synthetic_parent_and_descendants_test() {
        let index = ArchiveIndex::build(vec![file("a/b/c.txt"), file("a/d.txt")], 10).unwrap();
        assert!(index.get("").unwrap().fs_entry.is_dir());
        assert!(index.get("a").unwrap().fs_entry.is_dir());
        assert!(index.get("a/b").unwrap().fs_entry.is_dir());
        assert_eq!(index.descendants("a", false).map(ArchiveEntry::path).collect::<Vec<_>>(), ["a/b", "a/d.txt"]);
        assert_eq!(index.descendants("a", true).map(ArchiveEntry::path).collect::<Vec<_>>(), ["a/b", "a/b/c.txt", "a/d.txt"]);
    }

    #[test]
    fn rejects_duplicate_and_kind_collision_test() {
        assert_eq!(ArchiveIndex::build(vec![file("a"), file("a")], 10).unwrap_err().kind(), &ErrorKind::InvalidFormat);
        assert_eq!(ArchiveIndex::build(vec![file("a"), file("a/b")], 10).unwrap_err().kind(), &ErrorKind::InvalidFormat);
    }

    #[test]
    fn enforces_limit_after_parent_synthesis_test() {
        assert_eq!(ArchiveIndex::build(vec![file("a/b")], 2).unwrap_err().kind(), &ErrorKind::LimitExceeded);
    }
}
