use std::sync::Arc;

use omnius_pxna_interface::models;

use crate::service::listing::{EntryPosition, ListOptions, ListSort, ListingSnapshot};

/// 検索する 1 つの Library の一覧と、その Library の評価とタグで引いた条件。
pub struct SearchSource {
    library_id: String,
    snapshot: Arc<ListingSnapshot>,
    options: ListOptions,
}

impl SearchSource {
    pub fn new(library_id: &str, snapshot: Arc<ListingSnapshot>, options: ListOptions) -> Self {
        Self {
            library_id: library_id.to_string(),
            snapshot,
            options,
        }
    }
}

/// Library ごとの一覧を、並べ方の順に 1 つの列へまとめる。まとめた列は持たず、ページを切るたびに先頭からまとめ直す。
pub struct SearchResults {
    /// 要求で与えた Library の順。同じ並びの値の Entry は、この順に置く。
    sources: Vec<SearchSource>,
}

/// Merges the matches of the Libraries into one sequence in sort order. It keeps no copy and yields from the head.
struct Merge<'a> {
    sources: &'a [SearchSource],
    sort: ListSort,
    matching: Vec<Vec<usize>>,
    cursors: Vec<usize>,
    heads: Vec<Option<EntryPosition>>,
}

impl<'a> Merge<'a> {
    fn new(sources: &'a [SearchSource], sort: ListSort) -> Self {
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

    /// Only the same path in different Libraries compares equal, and the Library earlier in the request order wins.
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

    fn entry(&self, source: usize, index: usize) -> models::SearchEntry {
        let entry = self.sources[source].snapshot.entries().entries[index].entry.clone();
        models::SearchEntry::new(self.sources[source].library_id.clone(), entry)
    }
}

impl SearchResults {
    pub fn new(sources: Vec<SearchSource>) -> Self {
        Self { sources }
    }

    /// `options` の並べ方、`offset`、`limit` でページを切る。条件は Library ごとの `SearchSource` が持つ。
    pub fn page(&self, options: &ListOptions) -> models::SearchListing {
        let mut merge = Merge::new(&self.sources, options.sort());
        let total = merge.total();
        let end = options.offset.saturating_add(options.limit as u64).min(total);
        let mut entries = Vec::new();
        let mut produced = 0_u64;
        while produced < end {
            let Some((source, index)) = merge.next() else {
                break;
            };
            if produced >= options.offset {
                entries.push(merge.entry(source, index));
            }
            produced += 1;
        }
        self.listing(entries, total, options.offset)
    }

    /// Finds the position of the Entry at `path` and cuts the page that holds it at a multiple of `limit`.
    /// `library_id` selects the Library when the same path is in several.
    pub fn locate(&self, options: &ListOptions, library_id: &str, path: &str) -> Option<models::SearchListing> {
        let mut merge = Merge::new(&self.sources, options.sort());
        let mut position = 0_u64;
        let found = loop {
            let (source, index) = merge.next()?;
            if self.sources[source].library_id == library_id && self.sources[source].snapshot.entries().entries[index].entry.path == path {
                break position;
            }
            position += 1;
        };
        let limit = options.limit as u64;
        let offset = found / limit * limit;

        let mut merge = Merge::new(&self.sources, options.sort());
        let total = merge.total();
        let end = offset.saturating_add(limit).min(total);
        let mut entries = Vec::new();
        let mut produced = 0_u64;
        while produced < end {
            let Some((source, index)) = merge.next() else {
                break;
            };
            if produced >= offset {
                entries.push(merge.entry(source, index));
            }
            produced += 1;
        }
        Some(self.listing(entries, total, offset))
    }

    fn listing(&self, entries: Vec<models::SearchEntry>, total: u64, offset: u64) -> models::SearchListing {
        let snapshot = self
            .sources
            .iter()
            .map(|source| source.options.selection.snapshot_id(source.snapshot.id().to_string()))
            .collect::<Vec<_>>()
            .join(",");
        let truncated = self
            .sources
            .iter()
            .filter(|source| source.snapshot.entries().truncated)
            .map(|source| models::LibraryId(source.library_id.clone()))
            .collect();
        let indexed = self.sources.iter().all(|source| source.snapshot.entries().indexed());
        models::SearchListing::new(entries, total, offset, snapshot, truncated, indexed)
    }
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use omnius_core_base::clock::FakeClockUtc;

    use super::*;
    use crate::service::listing::{ListingEntries, ListingSnapshots, SnapshotKey, tests::entry};

    fn options(offset: u64, limit: u32) -> ListOptions {
        let query = models::ListSearchEntriesQueryParams {
            library_ids: vec![],
            path: None,
            recursive: None,
            query: None,
            rating: None,
            tag: vec![],
            sort: None,
            order: None,
            folders_first: None,
            offset: Some(offset),
            locate: None,
            locate_library_id: None,
            limit: Some(limit),
        };
        ListOptions::from_search(&query).unwrap()
    }

    async fn snapshot(snapshots: &Arc<ListingSnapshots>, library: &str, entries: ListingEntries) -> Arc<ListingSnapshot> {
        let key = SnapshotKey::new(library, "/", &options(0, 1)).unwrap();
        snapshots.get_or_scan(key, None, move || async move { Ok(entries) }).await.unwrap()
    }

    async fn results(truncate_b: bool) -> (SearchResults, Vec<Arc<ListingSnapshot>>) {
        let snapshots = ListingSnapshots::new(Default::default(), Arc::new(FakeClockUtc::new(DateTime::from_timestamp(1_800_000_000, 0).unwrap())));
        let a = snapshot(&snapshots, "a", ListingEntries::from_entries(vec![entry("3.png"), entry("1.png"), entry("same.png")])).await;
        let b_entries = vec![entry("2.png"), entry("same.png"), entry("4.png")];
        let b = snapshot(
            &snapshots,
            "b",
            if truncate_b {
                ListingEntries::truncated_from_entries(b_entries)
            } else {
                ListingEntries::from_entries(b_entries)
            },
        )
        .await;
        let sources = vec![SearchSource::new("a", a.clone(), options(0, 1)), SearchSource::new("b", b.clone(), options(0, 1))];
        (SearchResults::new(sources), vec![a, b])
    }

    fn keys(listing: &models::SearchListing) -> Vec<String> {
        listing.entries.iter().map(|entry| format!("{}:{}", entry.library_id, entry.entry.name)).collect()
    }

    #[tokio::test]
    async fn merges_libraries_in_sort_order_with_ties_in_library_order_test() {
        let (results, snapshots) = results(false).await;
        let all = results.page(&options(0, 100));
        assert_eq!(keys(&all), ["a:1.png", "b:2.png", "a:3.png", "b:4.png", "a:same.png", "b:same.png"]);
        assert_eq!(all.total_count, 6);
        assert_eq!(all.snapshot, format!("{},{}", snapshots[0].id(), snapshots[1].id()));
        assert!(all.truncated_library_ids.is_empty());
    }

    #[tokio::test]
    async fn pages_neither_repeat_nor_skip_entries_test() {
        let (results, _) = results(false).await;
        let all = keys(&results.page(&options(0, 100)));
        for limit in 1..=4 {
            let mut joined = Vec::new();
            for offset in (0..8).step_by(limit as usize) {
                let page = results.page(&options(offset, limit));
                assert_eq!(page.total_count, 6);
                joined.extend(keys(&page));
            }
            assert_eq!(joined, all, "limit {limit}");
        }
    }

    #[tokio::test]
    async fn lists_the_truncated_libraries_test() {
        let (results, _) = results(true).await;
        let page = results.page(&options(0, 1));
        assert_eq!(page.truncated_library_ids, [models::LibraryId("b".to_string())]);
    }

    #[tokio::test]
    async fn offset_in_response_search_listing_test() {
        let (results, _) = results(false).await;
        assert_eq!(results.page(&options(0, 2)).offset, 0);
        assert_eq!(results.page(&options(2, 2)).offset, 2);
        assert_eq!(results.page(&options(50, 2)).offset, 50);
    }

    #[tokio::test]
    async fn locate_search_matches_the_page_of_the_same_offset_test() {
        let (results, _) = results(false).await;
        let targets = [("a", "/1.png"), ("b", "/2.png"), ("a", "/3.png"), ("b", "/4.png"), ("a", "/same.png"), ("b", "/same.png")];
        for (library, path) in targets {
            for limit in [1, 2, 3] {
                let found = results.locate(&options(0, limit), library, path).expect("an Entry of the merged sequence has a position");
                assert_eq!(found.offset % limit as u64, 0);
                assert_eq!(found, results.page(&options(found.offset, limit)), "{library} {path} limit {limit}");
                assert!(found.entries.iter().any(|entry| entry.library_id == library && entry.entry.path == path));
            }
        }
        assert!(results.locate(&options(0, 2), "a", "/missing.png").is_none());
        assert!(results.locate(&options(0, 2), "c", "/1.png").is_none());
    }

    #[tokio::test]
    async fn locate_search_matches_an_entry_below_a_start_test() {
        let snapshots = ListingSnapshots::new(Default::default(), Arc::new(FakeClockUtc::new(DateTime::from_timestamp(1_800_000_000, 0).unwrap())));
        let a = snapshot(&snapshots, "a", ListingEntries::from_entries(vec![entry("sub/2.png"), entry("sub/1.png")])).await;
        let results = SearchResults::new(vec![SearchSource::new("a", a, options(0, 1))]);
        let found = results.locate(&options(0, 1), "a", "/sub/2.png").expect("the Entry below the start has a position");
        assert_eq!(found.offset, 1);
        assert_eq!(found, results.page(&options(1, 1)));
    }
}
