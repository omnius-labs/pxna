use omnius_pxna_interface::models;
use serde::{Deserialize, Serialize};

#[cfg(test)]
/// One page of the Entries of one folder, cut from its snapshot by `offset`.
#[derive(Debug, Clone, PartialEq)]
pub struct FolderPage {
    pub entries: Vec<models::FileSystemEntry>,
    pub total_count: u64,
    pub offset: u64,
    pub snapshot: String,
    pub truncated: bool,
}

#[cfg(test)]
impl FolderPage {
    pub fn new(entries: Vec<models::FileSystemEntry>, total_count: u64, offset: u64, snapshot: String, truncated: bool) -> Self {
        Self {
            entries,
            total_count,
            offset,
            snapshot,
            truncated,
        }
    }
}

/// One page of the summary of a folder's direct children, cut from its summary snapshot by `offset`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryPage {
    pub entries: Vec<models::FileSystemEntrySummary>,
    pub total_count: u64,
    pub offset: u64,
    pub snapshot: String,
}

impl SummaryPage {
    pub fn new(entries: Vec<models::FileSystemEntrySummary>, total_count: u64, offset: u64, snapshot: String) -> Self {
        Self {
            entries,
            total_count,
            offset,
            snapshot,
        }
    }
}

/// The metadata of one requested path: the Entry, or the error kind of its failure.
#[derive(Debug, Clone, PartialEq)]
pub struct EntryDetail {
    pub path: String,
    pub detail: Option<models::FileSystemEntry>,
    pub error: Option<models::ErrorKind>,
}

impl EntryDetail {
    pub fn new(path: String) -> Self {
        Self { path, detail: None, error: None }
    }
}

/// The conditions of the summary of a folder's direct children.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SummaryQuery {
    pub offset: Option<u64>,
    pub query: Option<String>,
    pub limit: Option<u32>,
    pub sort: Option<String>,
    pub order: Option<String>,
    pub include_hidden: Option<bool>,
    pub folders_first: Option<bool>,
    pub folders_only: Option<bool>,
}

/// One page of the name search over several Libraries, which only the tests of the name index read now that `listEntries` is the listing.
#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub struct SearchPage {
    pub entries: Vec<SearchItem>,
    pub total_count: u64,
    pub offset: u64,
    pub snapshot: String,
    pub truncated_library_ids: Vec<models::LibraryId>,
    pub indexed: bool,
}

#[cfg(test)]
impl SearchPage {
    pub fn new(entries: Vec<SearchItem>, total_count: u64, offset: u64, snapshot: String, truncated_library_ids: Vec<models::LibraryId>, indexed: bool) -> Self {
        Self {
            entries,
            total_count,
            offset,
            snapshot,
            truncated_library_ids,
            indexed,
        }
    }
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub struct SearchItem {
    pub library_id: String,
    pub entry: models::FileSystemEntry,
}

#[cfg(test)]
impl SearchItem {
    pub fn new(library_id: String, entry: models::FileSystemEntry) -> Self {
        Self { library_id, entry }
    }
}

/// The conditions of the name search that the tests of the name index read through.
#[cfg(test)]
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SearchQuery {
    pub library_ids: Vec<models::LibraryId>,
    pub path: Option<String>,
    pub recursive: Option<bool>,
    pub query: Option<String>,
    pub rating: Option<String>,
    pub tag: Vec<models::TagName>,
    pub sort: Option<String>,
    pub order: Option<String>,
    pub folders_first: Option<bool>,
    pub offset: Option<u64>,
    pub locate: Option<String>,
    pub locate_library_id: Option<String>,
    pub limit: Option<u32>,
}

/// The conditions of a listing of one folder that the tests of sorting and filtering read through.
#[cfg(test)]
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FolderQuery {
    pub path: String,
    pub recursive: Option<bool>,
    pub offset: Option<u64>,
    pub locate: Option<String>,
    pub query: Option<String>,
    pub limit: Option<u32>,
    pub sort: Option<String>,
    pub order: Option<String>,
    pub include_hidden: Option<bool>,
    pub folders_first: Option<bool>,
    pub folders_only: Option<bool>,
}
