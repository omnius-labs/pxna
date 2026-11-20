use omnius_pxna_interface::models;

use crate::service::listing::{ListOptions, ListingSource, MergedPage, MergedSources};

/// An Entry of a Scope's listing, with the Library it is in.
#[derive(Debug, Clone, PartialEq)]
pub struct ScopeEntry {
    pub library_id: String,
    pub entry: models::FileSystemEntry,
}

/// A page of a Scope's listing in the orders cut from snapshots, with what the response says about the starts.
#[derive(Debug, Clone, PartialEq)]
pub struct ScopeListing {
    pub entries: Vec<ScopeEntry>,
    pub total_count: u64,
    pub offset: u64,
    /// The identifiers of the starts' listings joined with commas.
    pub snapshot: String,
    /// The starts whose listing reached `recursive_max_entries`, by start value.
    pub cut_off_starts: Vec<String>,
    /// The starts the name index would answer from a local folder, which were walked because the index was not usable or failed meanwhile.
    pub walked_starts: Vec<String>,
    /// The starts that do not exist, by start value.
    pub missing_starts: Vec<String>,
}

/// The listings of the starts of a Scope as one sequence in sort order, with ties in the order of the starts.
pub struct ScopeResults {
    merged: MergedSources,
    missing: Vec<String>,
}

impl ScopeResults {
    /// `sources` are the starts that add Entries in the order they were given, and `missing` the values of the starts that do not exist.
    pub fn new(sources: Vec<ListingSource>, missing: Vec<String>) -> Self {
        Self {
            merged: MergedSources::new(sources),
            missing,
        }
    }

    /// The page at `options`' `offset`, sorted by its sort, with at most `limit` Entries. The conditions are the sources' own.
    pub fn page(&self, options: &ListOptions) -> ScopeListing {
        self.listing(self.merged.page(options))
    }

    /// Finds the position of the Entry at the Library-relative `path` of `library_id` and cuts the page that holds it at a multiple of `limit`.
    pub fn locate(&self, options: &ListOptions, library_id: &str, path: &str) -> Option<ScopeListing> {
        self.merged.locate(options, library_id, path).map(|page| self.listing(page))
    }

    fn listing(&self, page: MergedPage) -> ScopeListing {
        let sources = self.merged.sources();
        let starts = |keep: fn(&ListingSource) -> bool| sources.iter().filter(|source| keep(source)).map(|source| source.start().to_string()).collect::<Vec<_>>();
        ScopeListing {
            entries: page
                .entries
                .into_iter()
                .map(|merged| ScopeEntry {
                    library_id: sources[merged.source].library_id().to_string(),
                    entry: merged.entry,
                })
                .collect(),
            total_count: page.total_count,
            offset: page.offset,
            snapshot: self.merged.snapshot_id(),
            cut_off_starts: starts(ListingSource::is_cut_off),
            walked_starts: starts(ListingSource::is_walked),
            missing_starts: self.missing.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::DateTime;
    use omnius_core_base::clock::FakeClockUtc;

    use super::*;
    use crate::service::listing::{
        EntryKinds, ListingEntries, ListingSnapshot, ListingSnapshots, SnapshotKey,
        tests::{entry, query},
    };

    fn options(offset: u64, limit: u32, name: Option<&str>) -> ListOptions {
        let mut q = query(Some(offset), Some(limit));
        q.recursive = Some(true);
        q.query = name.map(str::to_string);
        ListOptions::from_query(&q).unwrap().with_scope(EntryKinds::default(), false)
    }

    async fn snapshot(snapshots: &Arc<ListingSnapshots>, library: &str, path: &str, entries: ListingEntries) -> Arc<ListingSnapshot> {
        let key = SnapshotKey::new(library, path, &options(0, 1, None)).unwrap();
        snapshots.get_or_scan(key, None, move || async move { Ok(entries) }).await.unwrap()
    }

    fn snapshots() -> Arc<ListingSnapshots> {
        ListingSnapshots::new(Default::default(), Arc::new(FakeClockUtc::new(DateTime::from_timestamp(1_800_000_000, 0).unwrap())))
    }

    fn entries(names: &[&str]) -> ListingEntries {
        ListingEntries::from_entries(names.iter().map(|name| entry(name)).collect())
    }

    fn keys(listing: &ScopeListing) -> Vec<String> {
        listing.entries.iter().map(|scoped| format!("{}:{}", scoped.library_id, scoped.entry.name)).collect()
    }

    /// Three starts: two in library `a`, and one in `b` that has the same names as the first of `a`.
    async fn three_starts() -> (ScopeResults, Vec<Arc<ListingSnapshot>>) {
        let snapshots = snapshots();
        let first = snapshot(&snapshots, "a", "/x", entries(&["3.png", "1.png", "same.png"])).await;
        let second = snapshot(&snapshots, "a", "/y", entries(&["2.png", "10.png"])).await;
        let other = snapshot(&snapshots, "b", "/", entries(&["same.png", "4.png"])).await;
        let sources = vec![
            ListingSource::for_start("a/x", "a", first.clone(), options(0, 1, None), true),
            ListingSource::for_start("a/y", "a", second.clone(), options(0, 1, None), true),
            ListingSource::for_start("b", "b", other.clone(), options(0, 1, None), true),
        ];
        (ScopeResults::new(sources, vec![]), vec![first, second, other])
    }

    #[tokio::test]
    async fn merges_starts_of_one_library_and_of_two_in_sort_order_with_ties_in_start_order_test() {
        let (results, snapshots) = three_starts().await;
        let all = results.page(&options(0, 100, None));
        assert_eq!(
            keys(&all),
            ["a:1.png", "a:2.png", "a:3.png", "b:4.png", "a:10.png", "a:same.png", "b:same.png"],
            "natural order, and the same name in start order"
        );
        assert_eq!((all.total_count, all.offset), (7, 0));
        assert_eq!(all.snapshot, snapshots.iter().map(|snapshot| snapshot.id().to_string()).collect::<Vec<_>>().join(","));

        // One start of one Library, and the same two Libraries with their starts in the other order.
        let one = ScopeResults::new(vec![ListingSource::for_start("a/y", "a", snapshots[1].clone(), options(0, 1, None), true)], vec![]);
        assert_eq!(keys(&one.page(&options(0, 100, None))), ["a:2.png", "a:10.png"]);
        let reversed = ScopeResults::new(
            vec![
                ListingSource::for_start("b", "b", snapshots[2].clone(), options(0, 1, None), true),
                ListingSource::for_start("a/x", "a", snapshots[0].clone(), options(0, 1, None), true),
            ],
            vec![],
        );
        assert_eq!(keys(&reversed.page(&options(0, 100, None))), ["a:1.png", "a:3.png", "b:4.png", "b:same.png", "a:same.png"]);
    }

    #[tokio::test]
    async fn pages_neither_repeat_nor_skip_entries_test() {
        let (results, _) = three_starts().await;
        let all = keys(&results.page(&options(0, 100, None)));
        for limit in 1..=5 {
            let mut joined = Vec::new();
            for offset in (0..10).step_by(limit as usize) {
                let page = results.page(&options(offset, limit, None));
                assert_eq!((page.total_count, page.offset), (7, offset));
                joined.extend(keys(&page));
            }
            assert_eq!(joined, all, "limit {limit}");
        }
        assert!(results.page(&options(50, 3, None)).entries.is_empty());
    }

    #[tokio::test]
    async fn the_conditions_of_a_source_limit_the_merge_test() {
        let (_, snapshots) = three_starts().await;
        // Each source carries its own conditions: the start of `b` has a name condition, and the starts of `a` have none.
        let results = ScopeResults::new(
            vec![
                ListingSource::for_start("a/x", "a", snapshots[0].clone(), options(0, 1, None), true),
                ListingSource::for_start("b", "b", snapshots[2].clone(), options(0, 1, Some("same")), true),
            ],
            vec![],
        );
        let page = results.page(&options(0, 100, None));
        assert_eq!(keys(&page), ["a:1.png", "a:3.png", "a:same.png", "b:same.png"]);
        assert_eq!(page.total_count, 4);
    }

    #[tokio::test]
    async fn locate_matches_the_page_of_the_same_offset_test() {
        let (results, snapshots) = three_starts().await;
        let targets = [
            ("a", "/1.png"),
            ("a", "/2.png"),
            ("a", "/3.png"),
            ("b", "/4.png"),
            ("a", "/10.png"),
            ("a", "/same.png"),
            ("b", "/same.png"),
        ];
        for (library, path) in targets {
            for limit in [1, 2, 3, 7] {
                let found = results
                    .locate(&options(0, limit, None), library, path)
                    .expect("an Entry of the merged sequence has a position");
                assert_eq!(found.offset % limit as u64, 0);
                assert_eq!(found, results.page(&options(found.offset, limit, None)), "{library} {path} limit {limit}");
                assert!(found.entries.iter().any(|scoped| scoped.library_id == library && scoped.entry.path == path));
            }
        }
        assert!(results.locate(&options(0, 2, None), "a", "/missing.png").is_none());
        assert!(results.locate(&options(0, 2, None), "c", "/1.png").is_none());
        let filtered = ScopeResults::new(vec![ListingSource::for_start("b", "b", snapshots[2].clone(), options(0, 1, Some("same")), true)], vec![]);
        assert!(
            filtered.locate(&options(0, 2, None), "b", "/4.png").is_none(),
            "an Entry outside the Filter has no position"
        );
        assert!(filtered.locate(&options(0, 2, None), "b", "/same.png").is_some());
    }

    #[tokio::test]
    async fn lists_the_starts_cut_off_walked_and_missing_test() {
        let snapshots = snapshots();
        let name = Some("png");
        // A recursive listing with a name condition, which the index answers from a local folder, so a walk of it is named. One cut off, one answered by the index.
        let walked = snapshot(&snapshots, "a", "/walked", entries(&["w.png"])).await;
        let cut_off = snapshot(&snapshots, "a", "/cut", ListingEntries::truncated_from_entries(vec![entry("c.png")])).await;
        let indexed = snapshot(&snapshots, "a", "/indexed", ListingEntries::from_index(vec![entry("i.png")], false, false)).await;
        // A start that is an archive is always walked, and a listing without a condition is walked as intended, so neither is named.
        let archive = snapshot(&snapshots, "a", "/pack.7z", entries(&["p.png"])).await;
        let plain = snapshot(&snapshots, "b", "/", entries(&["n.png"])).await;
        let sources = vec![
            ListingSource::for_start("a/walked", "a", walked, options(0, 1, name), true),
            ListingSource::for_start("a/cut", "a", cut_off, options(0, 1, name), true),
            ListingSource::for_start("a/indexed", "a", indexed, options(0, 1, name), true),
            ListingSource::for_start("a/pack.7z", "a", archive, options(0, 1, name), false),
            ListingSource::for_start("b", "b", plain, options(0, 1, None), true),
        ];
        let results = ScopeResults::new(sources, vec!["gone/x".to_string(), "a/deleted".to_string()]);
        let page = results.page(&options(0, 100, name));
        assert_eq!(page.cut_off_starts, ["a/cut"]);
        assert_eq!(page.walked_starts, ["a/walked", "a/cut"], "a cut-off walk is a walk too");
        assert_eq!(page.missing_starts, ["gone/x", "a/deleted"]);
        assert_eq!(page.total_count, 5, "the start without the name condition lists its Entry too");
        // The lists are the same on every page, not only the first.
        assert_eq!(results.page(&options(3, 1, name)).missing_starts, ["gone/x", "a/deleted"]);
        assert_eq!(results.locate(&options(0, 1, name), "a", "/p.png").expect("found").cut_off_starts, ["a/cut"]);
    }

    #[tokio::test]
    async fn a_scope_with_every_start_missing_is_empty_and_names_them_all_test() {
        let results = ScopeResults::new(vec![], vec!["a/x".to_string(), "b".to_string()]);
        let page = results.page(&options(0, 10, None));
        assert!(page.entries.is_empty());
        assert_eq!((page.total_count, page.offset), (0, 0));
        assert_eq!(page.missing_starts, ["a/x", "b"]);
        assert!(page.cut_off_starts.is_empty() && page.walked_starts.is_empty());
        assert!(results.locate(&options(0, 10, None), "a", "/x.png").is_none());
    }
}
