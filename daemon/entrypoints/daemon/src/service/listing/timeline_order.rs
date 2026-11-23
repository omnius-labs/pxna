use std::{cmp::Ordering, cmp::Reverse, sync::Arc};

use omnius_pxna_interface::models;

use crate::{
    prelude::*,
    service::{
        listing::{IndexKey, ListingEntries, ListingSnapshot},
        timeline::{TimelineCursor, TimelineDate, TimelineIndex},
    },
};

/// An Entry of a snapshot with the timeline date of its row, which is what a start with a name, rating, or tag condition lists in timeline-date order.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineMatch {
    pub library: String,
    pub date: TimelineDate,
    /// The Entry the snapshot read from the file system, not the row's.
    pub entry: models::FileSystemEntry,
}

impl TimelineMatch {
    /// Puts `matches` in the order of the sequence, newest timeline date first and then by Library id and path, keeps a match that several sources listed once,
    /// and keeps at most `limit` matches.
    pub fn merge(mut matches: Vec<Self>, limit: usize) -> Vec<Self> {
        matches.sort_by_cached_key(|found| (Reverse(found.date.text()), found.library.clone(), found.entry.path.clone()));
        matches.dedup_by(|later, earlier| later.library == earlier.library && later.entry.path == earlier.entry.path);
        matches.truncate(limit);
        matches
    }

    pub fn cursor(&self) -> TimelineCursor {
        TimelineCursor {
            date: self.date.text(),
            library: self.library.clone(),
            path: self.entry.path.clone(),
        }
    }
}

/// A match of the snapshot: where its Entry is among the snapshot's, and the timeline date of its row.
struct OrderedMatch {
    position: usize,
    date: TimelineDate,
}

/// The Entries of a snapshot that match a listing's conditions and have a row in the timeline index, newest timeline date first, with ties by path.
///
/// Within one Library, which is a snapshot's, this is the order of the sequence of `listEntries` in timeline-date order, whose ties go by Library id and then by path.
pub struct TimelineOrder {
    library: String,
    key: IndexKey,
    items: Vec<OrderedMatch>,
}

impl TimelineOrder {
    /// Orders `positions`, the matches among `entries`, by the dates of their rows in `index`, and leaves out the matches without a row.
    pub async fn build(entries: &ListingEntries, positions: &[usize], library: &str, key: IndexKey, index: &TimelineIndex) -> Result<Self> {
        let paths = positions.iter().map(|&position| entries.entries[position].entry.path.clone()).collect::<Vec<_>>();
        let rows = index.rows_of(library, &paths).await?;
        let mut items = positions
            .iter()
            .zip(&paths)
            .filter_map(|(&position, path)| rows.get(path).map(|row| OrderedMatch { position, date: row.date }))
            .collect::<Vec<_>>();
        items.sort_by_cached_key(|item| (Reverse(item.date.text()), entries.entries[item.position].entry.path.clone()));
        Ok(Self {
            library: library.to_string(),
            key,
            items,
        })
    }

    pub fn is_for(&self, library: &str, key: &IndexKey) -> bool {
        self.library == library && self.key == *key
    }
}

/// A snapshot with its order in timeline-date order, cut by the anchors of the timeline index.
pub struct SnapshotTimeline {
    snapshot: Arc<ListingSnapshot>,
    order: Arc<TimelineOrder>,
}

impl SnapshotTimeline {
    pub fn new(snapshot: Arc<ListingSnapshot>, order: Arc<TimelineOrder>) -> Self {
        Self { snapshot, order }
    }

    /// The Library of the snapshot's start.
    pub fn library(&self) -> &str {
        &self.order.library
    }

    /// The matches older than `cursor`, or the newest without one, newest first, at most `limit`.
    pub fn older(&self, cursor: Option<&TimelineCursor>, limit: usize) -> Vec<TimelineMatch> {
        let start = cursor.map_or(0, |cursor| self.after(cursor));
        self.matches(start, start.saturating_add(limit))
    }

    /// The matches dated before `date`, which is `YYYY-MM-DDTHH:MM:SS`, newest first, at most `limit`.
    pub fn older_than_date(&self, date: &str, limit: usize) -> Vec<TimelineMatch> {
        let start = self.order.items.partition_point(|item| item.date.text().as_str() >= date);
        self.matches(start, start.saturating_add(limit))
    }

    /// The `limit` matches newer than `cursor` that are nearest to it, newest first.
    pub fn newer(&self, cursor: &TimelineCursor, limit: usize) -> Vec<TimelineMatch> {
        let end = self.before(cursor);
        self.matches(end.saturating_sub(limit), end)
    }

    pub fn exists_newer(&self, cursor: &TimelineCursor) -> bool {
        self.before(cursor) > 0
    }

    pub fn exists_older(&self, cursor: &TimelineCursor) -> bool {
        self.after(cursor) < self.order.items.len()
    }

    /// The match whose Entry is at `path`.
    pub fn find(&self, path: &str) -> Option<TimelineMatch> {
        let index = self.order.items.iter().position(|item| self.path(item) == path)?;
        self.matches(index, index + 1).pop()
    }

    /// The number of matches in each month, newest month first.
    pub fn months(&self) -> Vec<(String, u64)> {
        let mut months = Vec::<(String, u64)>::new();
        for item in &self.order.items {
            let month = item.date.text()[..7].to_string();
            match months.last_mut() {
                Some((last, count)) if *last == month => *count += 1,
                _ => months.push((month, 1)),
            }
        }
        months
    }

    fn path(&self, item: &OrderedMatch) -> &str {
        &self.snapshot.entries().entries[item.position].entry.path
    }

    /// Where `item` is in the sequence relative to `cursor`: ordered by date newest first, then by Library id, then by path.
    fn compare(&self, item: &OrderedMatch, cursor: &TimelineCursor) -> Ordering {
        cursor
            .date
            .as_str()
            .cmp(item.date.text().as_str())
            .then_with(|| self.order.library.as_str().cmp(cursor.library.as_str()))
            .then_with(|| self.path(item).cmp(cursor.path.as_str()))
    }

    /// The number of matches before `cursor` in the sequence.
    fn before(&self, cursor: &TimelineCursor) -> usize {
        self.order.items.partition_point(|item| self.compare(item, cursor) == Ordering::Less)
    }

    /// The number of matches before `cursor` in the sequence, or at it.
    fn after(&self, cursor: &TimelineCursor) -> usize {
        self.order.items.partition_point(|item| self.compare(item, cursor) != Ordering::Greater)
    }

    fn matches(&self, start: usize, end: usize) -> Vec<TimelineMatch> {
        let items = &self.order.items;
        items[start.min(items.len())..end.min(items.len())]
            .iter()
            .map(|item| TimelineMatch {
                library: self.order.library.clone(),
                date: item.date,
                entry: self.snapshot.entries().entries[item.position].entry.clone(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use omnius_core_base::clock::FakeClockUtc;
    use testresult::TestResult;

    use super::*;
    use crate::service::{
        listing::{
            EntryKinds, ListOptions, ListingSnapshots, MarkSelection, SnapshotKey,
            tests::{entry, query},
        },
        timeline::{TimelineBatch, TimelineFileVersion, TimelineKind, TimelineRow},
    };

    fn row(path: &str, date: &str) -> TimelineRow {
        TimelineRow {
            path: path.to_string(),
            file_path: path.to_string(),
            kind: TimelineKind::Image,
            date: TimelineDate::from_exif(date).unwrap(),
            size: 1,
            modified: None,
            readonly: false,
        }
    }

    async fn write(index: &TimelineIndex, library: &str, rows: &[TimelineRow]) -> Result<()> {
        let mut batch = TimelineBatch::default();
        for row in rows {
            batch.replaced.push(row.file_path.clone());
            batch.files.push((row.file_path.clone(), TimelineFileVersion::Media { size: 1, modified_ns: 1 }));
            batch.rows.push(row.clone());
        }
        index.apply(library, 1, &batch).await
    }

    fn options(name: Option<&str>) -> ListOptions {
        let mut q = query(Some(0), Some(100));
        q.recursive = Some(true);
        q.query = name.map(str::to_string);
        ListOptions::from_query(&q).unwrap().with_scope(EntryKinds::default(), false)
    }

    async fn snapshot(names: &[&str]) -> Arc<ListingSnapshot> {
        let snapshots = ListingSnapshots::new(Default::default(), Arc::new(FakeClockUtc::new(DateTime::from_timestamp(1_800_000_000, 0).unwrap())));
        let key = SnapshotKey::new("lib", "/", &options(None)).unwrap();
        let entries = ListingEntries::from_entries(names.iter().map(|name| entry(name)).collect());
        snapshots.get_or_scan(key, None, move || async move { Ok(entries) }).await.unwrap()
    }

    async fn fixture() -> TestResult<(tempfile::TempDir, TimelineIndex, Arc<ListingSnapshot>)> {
        let dir = tempfile::tempdir()?;
        let index = TimelineIndex::open(dir.path()).await?;
        // `/none.png` and `/notes.txt` have no row. `/a.png` and `/b.png` have the same timeline date, and `/old.png` is in another month.
        write(
            &index,
            "lib",
            &[
                row("/new.png", "2024-03-10T00:00:00"),
                row("/b.png", "2024-03-05T00:00:00"),
                row("/a.png", "2024-03-05T00:00:00"),
                row("/mid.png", "2024-03-01T00:00:00"),
                row("/old.png", "2024-02-15T00:00:00"),
                row("/other.png", "2024-01-01T00:00:00"),
            ],
        )
        .await?;
        let snapshot = snapshot(&["old.png", "none.png", "b.png", "notes.txt", "new.png", "a.png", "mid.png"]).await;
        Ok((dir, index, snapshot))
    }

    fn paths(matches: &[TimelineMatch]) -> Vec<&str> {
        matches.iter().map(|found| found.entry.path.as_str()).collect()
    }

    #[tokio::test]
    async fn the_matches_are_ordered_by_the_dates_of_their_rows_and_those_without_a_row_are_left_out_test() -> TestResult {
        let (_dir, index, snapshot) = fixture().await?;
        let timeline = snapshot.timeline("lib", &options(None), &index).await?;
        let all = timeline.older(None, 100);
        assert_eq!(
            paths(&all),
            ["/new.png", "/a.png", "/b.png", "/mid.png", "/old.png"],
            "newest first, ties by path, `/none.png` and `/notes.txt` left out"
        );
        assert_eq!(all[0].date.text(), "2024-03-10T00:00:00");
        assert_eq!(all[0].library, "lib");

        // The Entry is the snapshot's, which was read from the file system, and not the row, whose size is 1.
        assert_eq!(all[0].entry, snapshot.entries().entries[4].entry);
        assert_eq!(all[0].entry.metadata.size, 0);

        // Only the matches of the listing's conditions are ordered.
        let named = snapshot.timeline("lib", &options(Some("png")), &index).await?;
        assert_eq!(named.older(None, 100).len(), 5);
        let narrowed = snapshot.timeline("lib", &options(Some("m")), &index).await?;
        assert_eq!(paths(&narrowed.older(None, 100)), ["/mid.png"], "`new.png` has no `m`");
        Ok(())
    }

    #[tokio::test]
    async fn the_order_is_built_once_and_replaced_by_another_key_test() -> TestResult {
        let (_dir, index, snapshot) = fixture().await?;
        let first = snapshot.timeline("lib", &options(None), &index).await?;
        // The index changes, but the order the snapshot kept does not until the key changes.
        write(&index, "lib", &[row("/new.png", "2020-01-01T00:00:00")]).await?;
        let again = snapshot.timeline("lib", &options(None), &index).await?;
        assert!(Arc::ptr_eq(&first.order, &again.order), "reused for the same key");
        assert_eq!(paths(&again.older(None, 1)), ["/new.png"]);

        let other = snapshot.timeline("lib", &options(Some("png")), &index).await?;
        assert!(!Arc::ptr_eq(&first.order, &other.order), "retyped terms build it again");
        assert_eq!(paths(&other.older(None, 1)), ["/a.png"], "from the rows as they are now");
        let back = snapshot.timeline("lib", &options(None), &index).await?;
        assert!(!Arc::ptr_eq(&first.order, &back.order), "one order is kept, so the older one was not held");

        // A write of ratings and tags changes the version of the selection, which changes the key.
        let rated = |version: u64| {
            options(None).with_selection(MarkSelection {
                active: Some(version),
                ..Default::default()
            })
        };
        let first = snapshot.timeline("lib", &rated(1), &index).await?;
        let same = snapshot.timeline("lib", &rated(1), &index).await?;
        let next = snapshot.timeline("lib", &rated(2), &index).await?;
        assert!(Arc::ptr_eq(&first.order, &same.order));
        assert!(!Arc::ptr_eq(&first.order, &next.order));
        // Another Library is another order.
        let elsewhere = snapshot.timeline("other", &rated(2), &index).await?;
        assert!(elsewhere.older(None, 10).is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn the_order_is_cut_by_the_anchors_of_the_timeline_index_test() -> TestResult {
        let (_dir, index, snapshot) = fixture().await?;
        let timeline = snapshot.timeline("lib", &options(None), &index).await?;
        let all = timeline.older(None, 100);

        // Cursors join pages without a repeat or a gap, forward and backward.
        for limit in 1..=3 {
            let mut forward = timeline.older(None, limit);
            while let Some(last) = forward.last().filter(|last| timeline.exists_older(&last.cursor())).map(TimelineMatch::cursor) {
                forward.extend(timeline.older(Some(&last), limit));
            }
            assert_eq!(forward, all, "forward by {limit}");
            let mut backward = timeline.newer(&all.last().unwrap().cursor(), limit);
            backward.push(all.last().unwrap().clone());
            while let Some(first) = backward.first().filter(|first| timeline.exists_newer(&first.cursor())).map(TimelineMatch::cursor) {
                let mut page = timeline.newer(&first, limit);
                page.extend(backward);
                backward = page;
            }
            assert_eq!(backward, all, "backward by {limit}");
        }
        assert!(!timeline.exists_newer(&all[0].cursor()) && timeline.exists_older(&all[0].cursor()));
        assert!(timeline.exists_newer(&all[4].cursor()) && !timeline.exists_older(&all[4].cursor()));
        assert_eq!(paths(&timeline.newer(&all[3].cursor(), 2)), ["/a.png", "/b.png"], "the matches nearest to the cursor");

        // A cursor of another Library splits the ties by Library id.
        let tie = |library: &str| TimelineCursor {
            date: "2024-03-05T00:00:00".to_string(),
            library: library.to_string(),
            path: "/a.png".to_string(),
        };
        assert_eq!(paths(&timeline.older(Some(&tie("lib")), 1)), ["/b.png"]);
        assert_eq!(
            paths(&timeline.older(Some(&tie("zzz")), 1)),
            ["/mid.png"],
            "`lib` comes before `zzz`, so both ties precede the cursor"
        );
        assert_eq!(
            paths(&timeline.older(Some(&tie("aaa")), 1)),
            ["/a.png"],
            "`lib` comes after `aaa`, so both ties follow the cursor"
        );
        assert_eq!(paths(&timeline.newer(&tie("aaa"), 1)), ["/new.png"]);

        // A month starts at the newest match before the end of that month, or of the next older month that has any.
        assert_eq!(paths(&timeline.older_than_date(&TimelineIndex::month_end("2024-03")?, 2)), ["/new.png", "/a.png"]);
        assert_eq!(paths(&timeline.older_than_date(&TimelineIndex::month_end("2024-02")?, 5)), ["/old.png"]);
        assert!(timeline.older_than_date(&TimelineIndex::month_end("2024-01")?, 5).is_empty());

        // Around: the named match, and none for a path that is not among the matches.
        assert_eq!(timeline.find("/mid.png").map(|found| found.entry.path), Some("/mid.png".to_string()));
        assert!(timeline.find("/none.png").is_none() && timeline.find("/other.png").is_none());
        Ok(())
    }

    #[tokio::test]
    async fn the_months_are_counted_from_the_order_test() -> TestResult {
        let (_dir, index, snapshot) = fixture().await?;
        let timeline = snapshot.timeline("lib", &options(None), &index).await?;
        assert_eq!(timeline.months(), [("2024-03".to_string(), 4), ("2024-02".to_string(), 1)]);
        let narrowed = snapshot.timeline("lib", &options(Some("old")), &index).await?;
        assert_eq!(narrowed.months(), [("2024-02".to_string(), 1)]);
        let none = snapshot.timeline("lib", &options(Some("zzz")), &index).await?;
        assert!(none.months().is_empty() && none.older(None, 5).is_empty());
        // The months equal the matches that the cut returns.
        let mut counted = Vec::<(String, u64)>::new();
        for found in timeline.older(None, 100) {
            let month = found.date.text()[..7].to_string();
            match counted.last_mut() {
                Some((last, count)) if *last == month => *count += 1,
                _ => counted.push((month, 1)),
            }
        }
        assert_eq!(counted, timeline.months());
        Ok(())
    }

    #[tokio::test]
    async fn rows_of_looks_up_many_paths_in_batches_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let index = TimelineIndex::open(dir.path()).await?;
        let rows = (0..1200).map(|i| row(&format!("/p{i}.png"), "2024-01-01T00:00:00")).collect::<Vec<_>>();
        write(&index, "lib", &rows).await?;
        write(&index, "other", &[row("/p0.png", "2023-01-01T00:00:00")]).await?;
        let mut paths = rows.iter().map(|row| row.path.clone()).collect::<Vec<_>>();
        paths.push("/missing.png".to_string());
        let found = index.rows_of("lib", &paths).await?;
        assert_eq!(found.len(), 1200);
        assert_eq!(found["/p0.png"].date.text(), "2024-01-01T00:00:00", "the rows of this Library only");
        assert!(index.rows_of("lib", &[]).await?.is_empty());
        Ok(())
    }
}
