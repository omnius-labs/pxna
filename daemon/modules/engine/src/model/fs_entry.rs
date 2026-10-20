use chrono::{DateTime, Utc};

/// driver namespace 内の 1 entry。
///
/// `path` は basename ではなく driver namespace 内の full path である。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FsEntry {
    pub path: String,
    pub meta: FsMetadata,
    pub detail: FsEntryDetail,
}

impl FsEntry {
    pub const MAX_PATH_LENGTH: usize = 4096;

    pub fn kind(&self) -> FsEntryKind {
        self.detail.kind()
    }

    pub fn is_file(&self) -> bool {
        matches!(self.detail, FsEntryDetail::File { .. })
    }
    pub fn is_dir(&self) -> bool {
        matches!(self.detail, FsEntryDetail::Dir { .. })
    }
    pub fn is_symlink(&self) -> bool {
        matches!(self.detail, FsEntryDetail::Link { .. })
    }
}

/// 属性を待たずに列挙できる名前と種別。未知の属性を仮値で表さない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FsEntrySummary {
    pub path: String,
    pub kind: FsEntryKind,
    pub archive: bool,
}

impl FsEntrySummary {
    pub fn from_entry(entry: FsEntry) -> Self {
        Self {
            kind: entry.kind(),
            archive: matches!(entry.detail, FsEntryDetail::File { archive: true }),
            path: entry.path,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FsMetadata {
    pub len: u64,
    pub readonly: bool,
    pub created: Option<DateTime<Utc>>,
    pub modified: Option<DateTime<Utc>>,
    pub accessed: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsEntryKind {
    File,
    Dir,
    Link,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsEntryDetail {
    /// `archive` は、この file を archive として開ける見込みがあるかどうかの hint である。
    /// 判定は拡張子だけで行い、内容を読まない。実際に開けるかどうかは開いた時点で決まる。
    File {
        archive: bool,
    },
    Dir {
        has_children: Option<bool>,
    },
    /// `target` は取得できた場合の driver namespace 内の生の文字列である。
    /// Library 相対 path へ解釈するのは daemon 層の責務であり、driver は Library を知らない。
    Link {
        target: Option<String>,
        target_kind: Option<FsEntryKind>,
    },
}

impl FsEntryDetail {
    pub fn kind(&self) -> FsEntryKind {
        match self {
            FsEntryDetail::File { .. } => FsEntryKind::File,
            FsEntryDetail::Dir { .. } => FsEntryKind::Dir,
            FsEntryDetail::Link { .. } => FsEntryKind::Link,
        }
    }
}
