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
    /// Builds the index, synthesizing missing parent directories.
    ///
    /// The entry count is checked on every insertion, so building never holds more than `max_entry_count` entries.
    pub fn build(mut raw_entries: Vec<ArchiveEntry>, max_entry_count: usize) -> Result<Self> {
        if raw_entries.len() > max_entry_count {
            return Err(ArchiveError::limit_exceeded(format!("archive has more than {max_entry_count} entries")));
        }

        // A path sorts after all of its ancestors, so every entry already present has all of its own ancestors.
        raw_entries.sort_unstable_by(|a, b| a.path().cmp(b.path()));

        let mut by_path = BTreeMap::from([(String::new(), ArchiveEntry::synthetic_dir(String::new()))]);
        for entry in raw_entries {
            if let Some(previous) = by_path.get(entry.path()) {
                let message = if previous.fs_entry.kind() == entry.fs_entry.kind() {
                    format!("duplicate archive entry path: {}", entry.path())
                } else {
                    format!("archive entry kind collision: {}", entry.path())
                };
                return Err(ArchiveError::invalid_format(message));
            }
            Self::insert_parents(&mut by_path, entry.path(), max_entry_count)?;
            Self::ensure_capacity(&by_path, max_entry_count)?;
            by_path.insert(entry.path().to_string(), entry);
        }

        Ok(Self {
            entries: by_path.into_values().collect(),
        })
    }

    /// Inserts the missing ancestors of `path` from the deepest one up, and stops at the first ancestor that exists.
    fn insert_parents(by_path: &mut BTreeMap<String, ArchiveEntry>, path: &str, max_entry_count: usize) -> Result<()> {
        let mut parent = path;
        while let Some(end) = parent.rfind('/') {
            parent = &parent[..end];
            if let Some(existing) = by_path.get(parent) {
                if !existing.fs_entry.is_dir() {
                    return Err(ArchiveError::invalid_format(format!("archive entry kind collision: {parent}")));
                }
                return Ok(());
            }
            Self::ensure_capacity(by_path, max_entry_count)?;
            by_path.insert(parent.to_string(), ArchiveEntry::synthetic_dir(parent.to_string()));
        }
        Ok(())
    }

    fn ensure_capacity(by_path: &BTreeMap<String, ArchiveEntry>, max_entry_count: usize) -> Result<()> {
        if by_path.len() >= max_entry_count {
            return Err(ArchiveError::limit_exceeded(format!(
                "archive has more than {max_entry_count} entries after parent synthesis"
            )));
        }
        Ok(())
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

    fn dir(path: &str) -> ArchiveEntry {
        ArchiveEntry::new(path.to_string(), FsEntryKind::Dir, 0, Some(DateTime::UNIX_EPOCH), None)
    }

    /// A path just under `FsEntry::MAX_PATH_LENGTH` with about 2,000 segments.
    fn deep_path(root: &str) -> String {
        let mut path = root.to_string();
        while path.len() + 4 <= FsEntry::MAX_PATH_LENGTH {
            path.push_str("/a");
        }
        path.push_str("/f");
        path
    }

    #[test]
    fn rejects_kind_collision_in_any_input_order_test() {
        assert_eq!(ArchiveIndex::build(vec![file("a/b"), file("a")], 10).unwrap_err().kind(), &ErrorKind::InvalidFormat);
        assert_eq!(ArchiveIndex::build(vec![file("a/b/c"), file("a")], 10).unwrap_err().kind(), &ErrorKind::InvalidFormat);
    }

    #[test]
    fn keeps_raw_directory_listed_after_its_descendant_test() {
        let index = ArchiveIndex::build(vec![file("a/b/c.txt"), dir("a/b")], 10).unwrap();
        assert_eq!(index.get("a/b").unwrap().fs_entry.meta.modified, Some(DateTime::UNIX_EPOCH));
        assert_eq!(index.get("a").unwrap().fs_entry.meta.modified, None);
    }

    #[test]
    fn enforces_limit_after_parent_synthesis_test() {
        assert_eq!(ArchiveIndex::build(vec![file("a/b")], 2).unwrap_err().kind(), &ErrorKind::LimitExceeded);
        assert_eq!(ArchiveIndex::build(vec![file("a/b")], 3).unwrap().entries.len(), 3);
    }

    #[test]
    fn rejects_deep_paths_before_synthesizing_past_limit_test() {
        // Synthesizing every parent before the limit check would ask for about 2 million directories here.
        let entries = (0..1_000).map(|i| file(&deep_path(&format!("e{i}")))).collect::<Vec<_>>();
        assert_eq!(ArchiveIndex::build(entries, 100_000).unwrap_err().kind(), &ErrorKind::LimitExceeded);
    }

    #[test]
    fn synthesizes_shared_deep_parents_once_test() {
        let base = deep_path("d");
        let base = base.strip_suffix("/f").unwrap();
        let depth = base.split('/').count();
        let entries = (0..1_000).map(|i| file(&format!("{base}/f{i}"))).collect::<Vec<_>>();
        let index = ArchiveIndex::build(entries, 1_000 + depth + 1).unwrap();
        assert_eq!(index.entries.len(), 1_000 + depth + 1);
    }
}
