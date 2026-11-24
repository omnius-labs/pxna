use std::sync::Arc;

use omnius_pxna_interface::models;

use crate::service::listing::{EntryPosition, ListOptions, ListSort, ListingSnapshot, ScopeStart};

/// One listing that is merged with others: a start's snapshot with that start's options, which carry the rating and tag selection of its Library.
pub struct ListingSource {
    /// The start the client gave, which the search API does not have.
    start: Option<ScopeStart>,
    library_id: String,
    snapshot: Arc<ListingSnapshot>,
    options: ListOptions,
    /// Whether the start is a local folder, which is where the name index can answer.
    local: bool,
}

impl ListingSource {
    /// A Library's listing for the search API.
    pub fn new(library_id: &str, snapshot: Arc<ListingSnapshot>, options: ListOptions) -> Self {
        Self {
            start: None,
            library_id: library_id.to_string(),
            snapshot,
            options,
            local: false,
        }
    }

    /// The listing of `start`, which is `local` when it is a local folder and not an archive or a path inside one.
    pub fn for_start(start: ScopeStart, snapshot: Arc<ListingSnapshot>, options: ListOptions, local: bool) -> Self {
        Self {
            library_id: start.library_id().to_string(),
            start: Some(start),
            snapshot,
            options,
            local,
        }
    }

    /// The start of the Scope this listing is of, or none for the search API.
    pub fn start(&self) -> Option<&ScopeStart> {
        self.start.as_ref()
    }

    pub fn library_id(&self) -> &str {
        &self.library_id
    }

    pub fn snapshot(&self) -> &Arc<ListingSnapshot> {
        &self.snapshot
    }

    pub fn options(&self) -> &ListOptions {
        &self.options
    }

    /// The identifier of the listing, with the version of ratings and tags while a rating or tag condition is given.
    pub fn snapshot_id(&self) -> String {
        self.options.selection.snapshot_id(self.snapshot.id().to_string())
    }

    /// Whether the walk or the index answer reached the recursion limit.
    pub fn is_cut_off(&self) -> bool {
        self.snapshot.entries().truncated
    }

    pub fn is_indexed(&self) -> bool {
        self.snapshot.entries().indexed()
    }

    /// Whether the listing is one the name index answers, from a local folder, but a walk answered it because the index was not usable or failed.
    pub fn is_walked(&self) -> bool {
        self.local && self.options.index_conditions().is_some() && !self.is_indexed()
    }
}

/// An Entry of a merged sequence, with the source it came from.
pub struct MergedEntry {
    /// The position of the source in the order the sources were given.
    pub source: usize,
    pub entry: models::FileSystemEntry,
}

/// A page cut from a merged sequence.
pub struct MergedPage {
    pub entries: Vec<MergedEntry>,
    pub total_count: u64,
    pub offset: u64,
}

/// The listings of several sources as one sequence in sort order. It keeps no copy of the sequence and merges from the head on every page.
pub struct MergedSources {
    /// The order the sources were given in. Entries of the same value in sort order are placed by it.
    sources: Vec<ListingSource>,
}

/// Merges the matches of the sources into one sequence in sort order. It keeps no copy and yields from the head.
struct Merge<'a> {
    sources: &'a [ListingSource],
    sort: ListSort,
    matching: Vec<Vec<usize>>,
    cursors: Vec<usize>,
    heads: Vec<Option<EntryPosition>>,
}

impl<'a> Merge<'a> {
    fn new(sources: &'a [ListingSource], sort: ListSort) -> Self {
        let matching = sources.iter().map(|source| source.snapshot.matching(&source.options)).collect::<Vec<_>>();
        let mut merge = Self {
            sources,
            sort,
            matching,
            cursors: vec![0_usize; sources.len()],
            heads: Vec::new(),
        };
        merge.heads = (0..sources.len()).map(|source| merge.position(source, 0)).collect();
        merge
    }

    fn position(&self, source: usize, cursor: usize) -> Option<EntryPosition> {
        let index = *self.matching[source].get(cursor)?;
        Some(self.sort.position(&self.sources[source].snapshot.entries().entries[index].entry))
    }

    fn total(&self) -> u64 {
        self.matching.iter().map(|indices| indices.len() as u64).sum()
    }

    /// Only the same path in different Libraries compares equal, and the source earlier in the given order wins.
    fn next(&mut self) -> Option<(usize, usize)> {
        let source = self
            .heads
            .iter()
            .enumerate()
            .filter_map(|(source, head)| head.as_ref().map(|head| (source, head)))
            .min_by(|(a, a_head), (b, b_head)| self.sort.compare(a_head, b_head).then(a.cmp(b)))
            .map(|(source, _)| source)?;
        let index = self.matching[source][self.cursors[source]];
        self.cursors[source] += 1;
        self.heads[source] = self.position(source, self.cursors[source]);
        Some((source, index))
    }

    fn entry(&self, source: usize, index: usize) -> &models::FileSystemEntry {
        &self.sources[source].snapshot.entries().entries[index].entry
    }
}

impl MergedSources {
    pub fn new(sources: Vec<ListingSource>) -> Self {
        Self { sources }
    }

    pub fn sources(&self) -> &[ListingSource] {
        &self.sources
    }

    /// The identifiers of the sources' listings joined with commas. When any one of them is rebuilt, it changes.
    pub fn snapshot_id(&self) -> String {
        self.sources.iter().map(ListingSource::snapshot_id).collect::<Vec<_>>().join(",")
    }

    /// The page of `options`' sort, `offset`, and `limit`. The conditions are the sources' own.
    pub fn page(&self, options: &ListOptions) -> MergedPage {
        self.cut(options, options.offset)
    }

    /// Finds the position of the Entry at `path` in `library_id` and cuts the page that holds it at a multiple of `limit`.
    pub fn locate(&self, options: &ListOptions, library_id: &str, path: &str) -> Option<MergedPage> {
        let mut merge = Merge::new(&self.sources, options.sort());
        let mut position = 0_u64;
        let found = loop {
            let (source, index) = merge.next()?;
            if self.sources[source].library_id == library_id && merge.entry(source, index).path == path {
                break position;
            }
            position += 1;
        };
        let limit = options.limit as u64;
        Some(self.cut(options, found / limit * limit))
    }

    fn cut(&self, options: &ListOptions, offset: u64) -> MergedPage {
        let mut merge = Merge::new(&self.sources, options.sort());
        let total_count = merge.total();
        let end = offset.saturating_add(options.limit as u64).min(total_count);
        let mut entries = Vec::new();
        let mut produced = 0_u64;
        while produced < end {
            let Some((source, index)) = merge.next() else {
                break;
            };
            if produced >= offset {
                entries.push(MergedEntry {
                    source,
                    entry: merge.entry(source, index).clone(),
                });
            }
            produced += 1;
        }
        MergedPage { entries, total_count, offset }
    }
}
