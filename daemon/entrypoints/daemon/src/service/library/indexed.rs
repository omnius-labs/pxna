use std::sync::Arc;

use futures::StreamExt as _;
use omnius_pxna_engine::service::WorkClass;
use omnius_pxna_interface::models;
use tokio::sync::Semaphore;

use crate::{
    config::LibraryConfig,
    prelude::*,
    service::{
        library::{EntryConverter, LibraryPathResolver, LibraryRelativePath},
        listing::{IndexCandidates, IndexConditions, ListingEntries, NameCursor, NameHit, NameIndex, NameQuery},
    },
};

/// How many matches have their attributes read at once, within the limit of the detail batch.
const READS: usize = 4;
/// How many candidates found by path are read before the limit is checked again.
const READ_BATCH: usize = 64;

/// What the matches read so far came to.
#[derive(Default)]
struct Found {
    entries: Vec<models::FileSystemEntry>,
    /// There were more matches than the limit.
    truncated: bool,
}

/// Answers a recursive listing with conditions from the name index.
///
/// The index holds names and kinds only, so the attributes of the matches are read from the file system with the workers and limit of the
/// detail batch. A match whose Entry no longer exists is dropped, and the cost beyond the index query is proportional to the matches.
pub struct IndexedListing {
    names: Arc<NameIndex>,
    resolver: Arc<LibraryPathResolver>,
    details: Arc<Semaphore>,
}

impl IndexedListing {
    pub fn new(names: Arc<NameIndex>, resolver: Arc<LibraryPathResolver>, details: Arc<Semaphore>) -> Self {
        Self { names, resolver, details }
    }

    /// Whether the Library's index can answer.
    pub async fn is_usable(&self, library: &str) -> bool {
        self.names.is_usable(library).await
    }

    /// The Entries under `start` that match `conditions`, at most `max_entries` of them. `None` means the index could not answer, so the caller walks.
    ///
    /// The limit counts matches that still exist: the index is read in batches, and an Entry that has gone does not take a place.
    pub async fn entries(&self, library: &LibraryConfig, start: &str, conditions: &IndexConditions, max_entries: usize) -> Result<Option<ListingEntries>> {
        let terms = conditions.terms().to_vec();
        let query = |only_media| NameQuery {
            library: library.id.clone(),
            start: start.to_string(),
            terms: terms.clone(),
            only_media,
        };
        let mut found = Found::default();
        let answered = match conditions.candidates() {
            IndexCandidates::Names => match self.names.search(query(false)) {
                Some(cursor) => self.drain(library, cursor, |_| true, &mut found, max_entries).await,
                None => false,
            },
            IndexCandidates::Unrated { rated, tagged } => match self.names.search(query(true)) {
                Some(cursor) => {
                    let keep = |hit: &NameHit| !rated.contains(&hit.path) && tagged.is_none_or(|tagged| tagged.contains(&hit.path));
                    self.drain(library, cursor, keep, &mut found, max_entries).await
                }
                None => false,
            },
            IndexCandidates::Paths(paths) => {
                let under = paths.into_iter().filter(|path| Self::is_under(start, path)).collect::<Vec<_>>();
                match self.names.lookup(&library.id, &under, &terms, true).await {
                    Some(hits) => {
                        for batch in hits.chunks(READ_BATCH) {
                            if !self.collect(library, batch.to_vec(), &mut found, max_entries).await {
                                break;
                            }
                        }
                        true
                    }
                    None => false,
                }
            }
        };
        // The index may have failed while the attributes were read, and then its answer is not to be trusted.
        Ok((answered && self.names.is_serving()).then(|| ListingEntries::from_index(found.entries, found.truncated)))
    }

    /// Reads a search batch by batch until the limit is passed or the matches run out. Returns whether the index answered.
    async fn drain(&self, library: &LibraryConfig, mut cursor: NameCursor, keep: impl Fn(&NameHit) -> bool, found: &mut Found, max_entries: usize) -> bool {
        loop {
            match cursor.next().await {
                Ok(Some(hits)) => {
                    let hits = hits.into_iter().filter(&keep).collect();
                    if !self.collect(library, hits, found, max_entries).await {
                        return true;
                    }
                }
                Ok(None) => return true,
                Err(_) => return false,
            }
        }
    }

    /// Reads the attributes of a batch of matches and keeps those that exist. Returns whether to go on, which it does not once there are more than `max_entries`.
    async fn collect(&self, library: &LibraryConfig, hits: Vec<NameHit>, found: &mut Found, max_entries: usize) -> bool {
        let read = futures::stream::iter(hits)
            .map(|hit| self.read(library, hit))
            .buffered(READS)
            .filter_map(|entry| async move { entry })
            .collect::<Vec<_>>()
            .await;
        for entry in read {
            if found.entries.len() >= max_entries {
                found.truncated = true;
                return false;
            }
            found.entries.push(entry);
        }
        true
    }

    /// Whether `path` is inside the folder `start`, not the folder itself.
    fn is_under(start: &str, path: &str) -> bool {
        if start == "/" {
            return path.len() > 1 && path.starts_with('/');
        }
        path.strip_prefix(start).is_some_and(|rest| rest.starts_with('/'))
    }

    /// The entry as a detail listing returns it, or `None` for an Entry that has gone or cannot be read.
    async fn read(&self, library: &LibraryConfig, hit: NameHit) -> Option<models::FileSystemEntry> {
        let _permit = self.details.acquire().await.ok()?;
        let relative = LibraryRelativePath::parse(&hit.path).ok()?;
        let result = async {
            let resolved = self.resolver.resolve_listing_entry(&library.path, &relative).await?;
            let entry = resolved.get_listing_entry(WorkClass::Foreground).await?;
            EntryConverter::new(&library.path).convert(&entry)
        }
        .await;
        match result {
            Ok(entry) => Some(entry),
            Err(error) => {
                debug!(library = %library.id, path = %hit.path, ?error, "name index: dropped a match that cannot be read");
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, io::Cursor, path::Path, time::Duration};

    use omnius_pxna_engine::model::Rating;
    use testresult::TestResult;

    use super::*;
    use crate::{
        config::ListingConfig,
        service::{
            listing::ListOptions,
            mutation::tests::{state, state_with_clock},
        },
        state::DaemonState,
    };

    const LIBRARY: &str = "test";
    const USER: &str = "owner";

    fn write(root: &Path, relative: &str, bytes: &[u8]) -> TestResult {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("a file has a parent"))?;
        std::fs::write(path, bytes)?;
        Ok(())
    }

    fn write_7z(path: &Path, entries: &[(&str, &[u8])]) -> TestResult {
        let mut writer = sevenz_rust2::ArchiveWriter::create(path)?;
        writer.push_archive_entries(
            entries.iter().map(|(name, _)| sevenz_rust2::ArchiveEntry::new_file(name)).collect(),
            entries.iter().map(|(_, bytes)| sevenz_rust2::SourceReader::new(Cursor::new(bytes.to_vec()))).collect(),
        )?;
        writer.finish()?;
        Ok(())
    }

    /// A search of the test Library, as the search route builds it: the rating and tag condition is read for the user first.
    async fn search(state: &DaemonState, query: Option<&str>, rating: Option<&str>, tags: &[&str], recursive: bool, path: &str) -> TestResult<models::SearchListing> {
        let params = models::ListSearchEntriesQueryParams {
            library_ids: vec![models::LibraryId(LIBRARY.to_string())],
            path: None,
            recursive: Some(recursive),
            query: query.map(str::to_string),
            rating: rating.map(str::to_string),
            tag: tags.iter().map(|tag| models::TagName(tag.to_string())).collect(),
            sort: None,
            order: None,
            folders_first: None,
            offset: None,
            locate: None,
            locate_library_id: None,
            limit: Some(1000),
        };
        let options = ListOptions::from_search(&params)?;
        let selection = options.marks().select(&state.rating_tags, USER, LIBRARY).await?;
        let options = options.with_selection(selection);
        Ok(state.library.search_entries(vec![(LIBRARY.to_string(), options.clone())], path, &options).await?)
    }

    async fn find(state: &DaemonState, query: &str) -> TestResult<models::SearchListing> {
        search(state, Some(query), None, &[], true, "/").await
    }

    fn paths(listing: &models::SearchListing) -> BTreeSet<String> {
        listing.entries.iter().map(|entry| entry.entry.path.clone()).collect()
    }

    fn set(paths: &[&str]) -> BTreeSet<String> {
        paths.iter().map(|path| path.to_string()).collect()
    }

    /// Runs a full aggregation of the test Library and waits for it, which makes its name index usable.
    async fn aggregate(state: &DaemonState) -> TestResult {
        state.names.verified().await;
        assert!(!state.names.is_usable(LIBRARY).await);
        state.timeline.refresh(LIBRARY).await?;
        for _ in 0..400 {
            let status = state.timeline.status(LIBRARY).await?;
            if !status.running && status.completed_at.is_some() {
                assert!(state.names.is_usable(LIBRARY).await);
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        Err("the aggregation did not finish".into())
    }

    fn root(state: &DaemonState) -> std::path::PathBuf {
        state.conf.libraries[0].path.clone()
    }

    #[tokio::test]
    async fn the_index_returns_the_entries_a_walk_returns_test() -> TestResult {
        let (_dir, state) = state().await?;
        let root = root(&state);
        write(&root, "ＡＢＣ.txt", b"1")?;
        write(&root, "か\u{3099}.png", b"2")?;
        write(&root, "日本語メモ.txt", b"3")?;
        write(&root, "ab.txt", b"4")?;
        write(&root, "notes/Deep Folder/Report_2024.pdf", b"5")?;
        write(&root, "notes/x.jpg", b"6")?;
        write(&root, ".hidden/abc.txt", b"7")?;
        write(&root, "UPPER/lower.TXT", b"8")?;
        write_7z(&root.join("pack.7z"), &[("abc.png", b"inside")])?;

        // 1, 2, and 3 characters and a multi-term query, Japanese, decomposed, and full-width names.
        let queries = [
            "abc",
            "が",
            "日本",
            "メモ",
            "ab",
            "a",
            "re",
            "report 2024",
            "txt",
            "deep",
            "d",
            "notes",
            "upper",
            "LOWER",
            "zzz",
            "r 20",
        ];
        let mut walked = Vec::new();
        for query in queries {
            let listing = find(&state, query).await?;
            assert!(!listing.indexed, "{query}: the index is not usable yet, so the search walks");
            walked.push(paths(&listing));
        }
        assert!(walked[0].contains("/ＡＢＣ.txt"), "full-width names match: {:?}", walked[0]);
        assert!(!walked[0].iter().any(|path| path.contains(".hidden") || path.contains("pack.7z/")), "{:?}", walked[0]);

        aggregate(&state).await?;
        for (query, expected) in queries.iter().zip(&walked) {
            let listing = find(&state, query).await?;
            assert!(listing.indexed, "{query}");
            assert_eq!(&paths(&listing), expected, "{query}");
            assert!(listing.truncated_library_ids.is_empty());
        }
        Ok(())
    }

    #[tokio::test]
    async fn searches_the_index_does_not_answer_walk_and_say_so_test() -> TestResult {
        let (_dir, state) = state().await?;
        let root = root(&state);
        write(&root, "top.txt", b"1")?;
        write(&root, "sub/inner.txt", b"2")?;
        write_7z(&root.join("pack.7z"), &[("inside.txt", b"x")])?;
        aggregate(&state).await?;

        let indexed = find(&state, "txt").await?;
        assert!(indexed.indexed);
        assert_eq!(paths(&indexed), set(&["/top.txt", "/sub/inner.txt"]));

        let flat = search(&state, Some("txt"), None, &[], false, "/").await?;
        assert!(!flat.indexed, "a search without subfolders is a listing of the folder");
        assert_eq!(paths(&flat), set(&["/top.txt"]));

        let no_condition = search(&state, None, None, &[], true, "/").await?;
        assert!(!no_condition.indexed);

        let in_archive = search(&state, Some("inside"), None, &[], true, "/pack.7z").await?;
        assert!(!in_archive.indexed, "the index holds no archive contents");
        assert_eq!(paths(&in_archive), set(&["/pack.7z/inside.txt"]));

        let below = search(&state, Some("inner"), None, &[], true, "/sub").await?;
        assert!(below.indexed, "a local folder below the root is answered from the index too");
        assert_eq!(paths(&below), set(&["/sub/inner.txt"]));
        Ok(())
    }

    #[tokio::test]
    async fn a_start_only_finds_what_is_under_it_test() -> TestResult {
        let (_dir, state) = state().await?;
        let root = root(&state);
        write(&root, "a/target.txt", b"1")?;
        write(&root, "ab/target.txt", b"2")?;
        write(&root, "a/b/target.txt", b"3")?;
        aggregate(&state).await?;
        let listing = search(&state, Some("target"), None, &[], true, "/a").await?;
        assert!(listing.indexed);
        assert_eq!(paths(&listing), set(&["/a/target.txt", "/a/b/target.txt"]), "/ab is not under /a");
        Ok(())
    }

    #[tokio::test]
    async fn an_entry_changed_after_aggregation_shows_its_current_state_test() -> TestResult {
        let (_dir, state) = state().await?;
        let root = root(&state);
        write(&root, "keep.txt", b"short")?;
        write(&root, "gone.txt", b"gone")?;
        write(&root, "folder/inner.txt", b"x")?;
        aggregate(&state).await?;

        std::fs::remove_file(root.join("gone.txt"))?;
        std::fs::remove_dir_all(root.join("folder"))?;
        std::fs::write(root.join("keep.txt"), b"much longer content than before")?;
        let listing = find(&state, "txt").await?;
        assert!(listing.indexed);
        assert_eq!(paths(&listing), set(&["/keep.txt"]), "Entries deleted since the aggregation are dropped");
        assert_eq!(listing.total_count, 1);
        assert_eq!(listing.entries[0].entry.metadata.size, 31, "the size is read when the snapshot is built");
        Ok(())
    }

    #[tokio::test]
    async fn the_limit_counts_matches_not_walked_entries_test() -> TestResult {
        let listing = ListingConfig {
            recursive_max_entries: 10,
            snapshot_capacity_entries: 100,
            ..Default::default()
        };
        let (_dir, state) = state_with_clock(listing, Arc::new(omnius_core_base::clock::ClockUtc)).await?;
        let root = root(&state);
        for number in 0..30 {
            write(&root, &format!("keep_{number:02}.txt"), b"x")?;
        }
        for number in 0..3 {
            write(&root, &format!("sub/needle_{number}.txt"), b"x")?;
        }
        aggregate(&state).await?;

        let rare = find(&state, "needle").await?;
        assert!(rare.indexed);
        assert_eq!(
            (rare.total_count, rare.truncated_library_ids.len()),
            (3, 0),
            "fewer matches than the limit are not cut off, on a Library larger than the limit"
        );

        let common = find(&state, "keep").await?;
        assert!(common.indexed);
        assert_eq!(common.total_count, 10);
        assert_eq!(common.truncated_library_ids, vec![models::LibraryId(LIBRARY.to_string())]);
        Ok(())
    }

    #[tokio::test]
    async fn the_limit_counts_matches_that_still_exist_test() -> TestResult {
        let listing = ListingConfig {
            recursive_max_entries: 5,
            snapshot_capacity_entries: 100,
            ..Default::default()
        };
        let (_dir, state) = state_with_clock(listing, Arc::new(omnius_core_base::clock::ClockUtc)).await?;
        let root = root(&state);
        for number in 0..10 {
            write(&root, &format!("gone_{number}.txt"), b"x")?;
            write(&root, &format!("some_{number}.txt"), b"x")?;
            write(&root, &format!("most_{number}.txt"), b"x")?;
        }
        aggregate(&state).await?;
        // The index still holds 10 of each, though most are gone. Whichever are listed first, the limit is of the ones that remain.
        for number in 0..10 {
            std::fs::remove_file(root.join(format!("gone_{number}.txt")))?;
        }
        for number in 0..7 {
            std::fs::remove_file(root.join(format!("some_{number}.txt")))?;
        }
        for number in 0..3 {
            std::fs::remove_file(root.join(format!("most_{number}.txt")))?;
        }
        let none = find(&state, "gone_").await?;
        assert!(none.indexed);
        assert_eq!((none.total_count, none.truncated_library_ids.len()), (0, 0));
        let few = find(&state, "some_").await?;
        assert_eq!((few.total_count, few.truncated_library_ids.len()), (3, 0), "three remain, which is below the limit of five");
        let many = find(&state, "most_").await?;
        assert_eq!(many.total_count, 5, "seven remain, and the limit is of five");
        assert_eq!(many.truncated_library_ids.len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn an_answer_is_dropped_when_the_index_fails_while_the_attributes_are_read_test() -> TestResult {
        let (_dir, state) = state().await?;
        let root = root(&state);
        write(&root, "a.txt", b"1")?;
        write(&root, "b.txt", b"2")?;
        aggregate(&state).await?;
        let params = models::ListSearchEntriesQueryParams {
            library_ids: vec![models::LibraryId(LIBRARY.to_string())],
            path: None,
            recursive: Some(true),
            query: Some("txt".to_string()),
            rating: None,
            tag: vec![],
            sort: None,
            order: None,
            folders_first: None,
            offset: None,
            locate: None,
            locate_library_id: None,
            limit: Some(10),
        };
        let conditions = ListOptions::from_search(&params)?.index_conditions().expect("a name condition is one the index answers");
        let indexed = state.library.indexed.clone();
        let library = state.conf.libraries[0].clone();
        // Hold every permit of the detail batch, so the reads of the attributes wait.
        let permits = state.library.details.clone().acquire_many_owned(4).await?;
        let reading = tokio::spawn(async move { indexed.entries(&library, "/", &conditions, 100).await });
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(!reading.is_finished());
        state.names.fail_for_test();
        drop(permits);
        assert!(reading.await??.is_none(), "an answer read from an index that failed meanwhile is not used");
        Ok(())
    }

    #[tokio::test]
    async fn an_entry_renamed_to_a_hidden_name_is_not_found_test() -> TestResult {
        let (_dir, state) = state().await?;
        let root = root(&state);
        write(&root, "visible.png", b"1")?;
        write(&root, "other.png", b"2")?;
        aggregate(&state).await?;
        std::fs::rename(root.join("visible.png"), root.join(".visible.png"))?;
        state.mutation.renamed(LIBRARY, "/visible.png", "/.visible.png").await;
        let listing = find(&state, "png").await?;
        assert!(listing.indexed);
        assert_eq!(paths(&listing), set(&["/other.png"]));
        Ok(())
    }

    #[tokio::test]
    async fn rating_and_tag_conditions_match_a_walk_and_follow_writes_test() -> TestResult {
        let (_dir, state) = state().await?;
        let root = root(&state);
        write(&root, "a.png", b"1")?;
        write(&root, "b.png", b"2")?;
        write(&root, "c.png", b"3")?;
        write(&root, "sub/d.png", b"4")?;
        write(&root, "sub/e.txt", b"5")?;
        state.rating_tags.set_rating(USER, LIBRARY, "/a.png", Some(Rating::Good)).await?;
        state.rating_tags.set_rating(USER, LIBRARY, "/b.png", Some(Rating::Bad)).await?;
        state.rating_tags.set_entry_tags(USER, LIBRARY, "/sub/d.png", &["trip".to_string()]).await?;
        state.rating_tags.set_entry_tags(USER, LIBRARY, "/a.png", &["trip".to_string()]).await?;

        let cases: [(Option<&str>, Option<&str>, &[&str], &[&str]); 9] = [
            (None, Some("good"), &[], &["/a.png"]),
            (None, Some("bad"), &[], &["/b.png"]),
            (None, Some("none"), &[], &["/c.png", "/sub/d.png"]),
            (Some("d"), Some("none"), &[], &["/sub/d.png"]),
            (None, None, &["trip"], &["/a.png", "/sub/d.png"]),
            (None, Some("good"), &["trip"], &["/a.png"]),
            (Some("a"), Some("good"), &[], &["/a.png"]),
            (Some("zzz"), Some("good"), &[], &[]),
            (Some("png"), Some("none"), &["trip"], &["/sub/d.png"]),
        ];
        for (query, rating, tags, expected) in cases {
            let walked = search(&state, query, rating, tags, true, "/").await?;
            assert!(!walked.indexed);
            assert_eq!(paths(&walked), set(expected), "walk: {query:?} {rating:?} {tags:?}");
        }
        aggregate(&state).await?;
        for (query, rating, tags, expected) in cases {
            let listing = search(&state, query, rating, tags, true, "/").await?;
            assert!(listing.indexed, "{query:?} {rating:?} {tags:?}");
            assert_eq!(paths(&listing), set(expected), "index: {query:?} {rating:?} {tags:?}");
        }

        // A rating write changes the snapshot id and the result, because the version of ratings and tags is part of the combination.
        let before = search(&state, None, Some("good"), &[], true, "/").await?;
        state.rating_tags.set_rating(USER, LIBRARY, "/c.png", Some(Rating::Good)).await?;
        let after = search(&state, None, Some("good"), &[], true, "/").await?;
        assert_ne!(before.snapshot, after.snapshot);
        assert_eq!(paths(&after), set(&["/a.png", "/c.png"]));
        let unrated = search(&state, None, Some("none"), &[], true, "/").await?;
        assert_eq!(paths(&unrated), set(&["/sub/d.png"]));
        Ok(())
    }

    #[tokio::test]
    async fn a_rated_entry_deleted_externally_does_not_appear_test() -> TestResult {
        let (_dir, state) = state().await?;
        let root = root(&state);
        write(&root, "keep.png", b"1")?;
        write(&root, "gone.png", b"2")?;
        state.rating_tags.set_rating(USER, LIBRARY, "/keep.png", Some(Rating::Good)).await?;
        state.rating_tags.set_rating(USER, LIBRARY, "/gone.png", Some(Rating::Good)).await?;
        aggregate(&state).await?;
        std::fs::remove_file(root.join("gone.png"))?;
        let listing = search(&state, None, Some("good"), &[], true, "/").await?;
        assert!(listing.indexed);
        assert_eq!(paths(&listing), set(&["/keep.png"]));
        Ok(())
    }
}
