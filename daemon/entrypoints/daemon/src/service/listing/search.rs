use std::sync::Arc;

use omnius_pxna_interface::models;

use crate::service::listing::{EntryPosition, ListOptions, ListingSnapshot};

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

impl SearchResults {
    pub fn new(sources: Vec<SearchSource>) -> Self {
        Self { sources }
    }

    /// `options` の並べ方、`offset`、`limit` でページを切る。条件は Library ごとの `SearchSource` が持つ。
    pub fn page(&self, options: &ListOptions) -> models::SearchListing {
        let sort = options.sort();
        let matching = self.sources.iter().map(|source| source.snapshot.matching(&source.options)).collect::<Vec<_>>();
        let total = matching.iter().map(|indices| indices.len() as u64).sum::<u64>();
        let end = options.offset.saturating_add(options.limit as u64).min(total);

        let position = |source: usize, cursor: usize| -> Option<EntryPosition> {
            let index = *matching[source].get(cursor)?;
            Some(sort.position(&self.sources[source].snapshot.entries().entries[index].entry))
        };
        let mut cursors = vec![0_usize; self.sources.len()];
        let mut heads = (0..self.sources.len()).map(|source| position(source, 0)).collect::<Vec<_>>();
        let mut entries = Vec::new();
        let mut produced = 0_u64;
        while produced < end {
            // 並びの値が等しいのは別の Library の同じ path だけで、そのときは Library の順で先のものを取る。
            let next = heads
                .iter()
                .enumerate()
                .filter_map(|(source, head)| head.as_ref().map(|head| (source, head)))
                .min_by(|(a, a_head), (b, b_head)| sort.compare(a_head, b_head).then(a.cmp(b)))
                .map(|(source, _)| source);
            let Some(source) = next else {
                break;
            };
            if produced >= options.offset {
                let index = matching[source][cursors[source]];
                let entry = self.sources[source].snapshot.entries().entries[index].entry.clone();
                entries.push(models::SearchEntry::new(self.sources[source].library_id.clone(), entry));
            }
            produced += 1;
            cursors[source] += 1;
            heads[source] = position(source, cursors[source]);
        }

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
        models::SearchListing::new(entries, total, snapshot, truncated)
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
}
