use std::{collections::BTreeMap, collections::HashMap, sync::Arc};

use omnius_pxna_engine::service::MediaKind;
use omnius_pxna_interface::{models, types::Nullable};

use crate::{
    prelude::*,
    service::{
        library::LibraryBrowser,
        listing::{ListOptions, MarkSelection, ResolvedScope, Scope, SnapshotTimeline, TimelineMatch},
        timeline::{TimelineAnchor, TimelineCursor, TimelineIndex, TimelineKind, TimelineListedRow, TimelineReading, TimelineService, TimelineStart},
    },
};

impl TimelineListedRow {
    /// The Entry of a row, in the form of a listing's, built from the row and not read from the file system.
    pub fn file_system_entry(&self) -> models::FileSystemEntry {
        let row = &self.row;
        let name = row.path.rsplit('/').next().unwrap_or_default().to_string();
        let mut metadata = models::FileSystemMetadata::new(row.size, row.readonly);
        metadata.modified_at = row.modified.map(Nullable::Present);
        metadata.content_type = MediaKind::content_type(&name).map(|content_type| Nullable::Present(content_type.to_string()));
        let mut entry = models::FileSystemEntry::new(row.path.clone(), name, metadata, models::FileSystemEntryKind::File);
        entry.file = Some(Nullable::Present(models::FileSystemEntryFileDetail::new(false)));
        entry
    }

    fn to_match(&self) -> TimelineMatch {
        TimelineMatch {
            library: self.library.clone(),
            date: self.row.date,
            entry: self.file_system_entry(),
        }
    }
}

/// The rows of a reading of the timeline index, as one of the sequences that a Scope's timeline is merged from.
pub struct RowSequence {
    index: Arc<TimelineIndex>,
    reading: TimelineReading,
}

impl RowSequence {
    pub fn new(index: Arc<TimelineIndex>, reading: TimelineReading) -> Self {
        Self { index, reading }
    }
}

/// One of the sequences that a Scope's timeline is merged from: the rows of the starts without a condition, read together from the timeline index,
/// or the order of the matches that the snapshot of a start with a condition keeps. Each is in timeline-date order.
pub enum TimelineSequence {
    Rows(RowSequence),
    Snapshot(SnapshotTimeline),
}

impl TimelineSequence {
    async fn older(&self, cursor: Option<&TimelineCursor>, limit: usize) -> Result<Vec<TimelineMatch>> {
        Ok(match self {
            Self::Rows(rows) => rows.index.older_rows(&rows.reading, cursor, limit).await?.iter().map(TimelineListedRow::to_match).collect(),
            Self::Snapshot(timeline) => timeline.older(cursor, limit),
        })
    }

    async fn newer(&self, cursor: &TimelineCursor, limit: usize) -> Result<Vec<TimelineMatch>> {
        Ok(match self {
            Self::Rows(rows) => rows.index.newer_rows(&rows.reading, cursor, limit).await?.iter().map(TimelineListedRow::to_match).collect(),
            Self::Snapshot(timeline) => timeline.newer(cursor, limit),
        })
    }

    async fn older_than_date(&self, date: &str, limit: usize) -> Result<Vec<TimelineMatch>> {
        Ok(match self {
            Self::Rows(rows) => rows
                .index
                .older_than_date_rows(&rows.reading, date, limit)
                .await?
                .iter()
                .map(TimelineListedRow::to_match)
                .collect(),
            Self::Snapshot(timeline) => timeline.older_than_date(date, limit),
        })
    }

    async fn exists_newer(&self, cursor: &TimelineCursor) -> Result<bool> {
        match self {
            Self::Rows(rows) => rows.index.exists_newer(&rows.reading, cursor).await,
            Self::Snapshot(timeline) => Ok(timeline.exists_newer(cursor)),
        }
    }

    async fn exists_older(&self, cursor: &TimelineCursor) -> Result<bool> {
        match self {
            Self::Rows(rows) => rows.index.exists_older(&rows.reading, cursor).await,
            Self::Snapshot(timeline) => Ok(timeline.exists_older(cursor)),
        }
    }

    /// The Entry at `path` of `library` when this sequence has it.
    async fn find(&self, library: &str, path: &str) -> Result<Option<TimelineMatch>> {
        Ok(match self {
            Self::Rows(rows) => rows.index.row_of(&rows.reading, library, path).await?.map(|row| row.to_match()),
            Self::Snapshot(timeline) if timeline.library() == library => timeline.find(path),
            Self::Snapshot(_) => None,
        })
    }

    async fn months(&self) -> Result<Vec<(String, u64)>> {
        match self {
            Self::Rows(rows) => rows.index.count_months(&rows.reading).await,
            Self::Snapshot(timeline) => Ok(timeline.months()),
        }
    }
}

/// A page of a Scope in timeline-date order, with what the response says about the starts.
#[derive(Debug, Clone, PartialEq)]
pub struct ScopeTimelinePage {
    /// The Entries newest first, each with its Library, timeline date, and the date's source.
    pub entries: Vec<TimelineMatch>,
    /// The cursor of the page before this one, which is none when there is no Entry newer than the first.
    pub previous: Option<TimelineCursor>,
    /// The cursor of the page after this one, which is none when there is no Entry older than the last.
    pub next: Option<TimelineCursor>,
    /// The starts with a condition whose matches reached `recursive_max_entries`, by start value.
    pub cut_off_starts: Vec<String>,
    /// The starts with a condition that the name index would answer from a local folder, which were walked because the index was not usable.
    pub walked_starts: Vec<String>,
    /// The starts that do not exist, by start value.
    pub missing_starts: Vec<String>,
}

/// What the starts of a Scope came to: the sequences to merge, and the starts to name.
struct ScopeSequences {
    sequences: Vec<TimelineSequence>,
    cut_off_starts: Vec<String>,
    walked_starts: Vec<String>,
    missing_starts: Vec<String>,
}

/// Reads the Entries of a Scope through a Filter in timeline-date order and counts their months.
///
/// The starts without a name, rating, or tag condition are read from the rows of the timeline index, which hold what the last aggregation found. With one,
/// each start is read from the order of its snapshot's matches, which holds the Entries as the file system had them when the snapshot was built.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the route of listEntries reads a Scope in timeline-date order through this from step 6")
)]
pub struct ScopeTimeline<'a> {
    library: &'a LibraryBrowser,
    timeline: &'a TimelineService,
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the route of listEntries reads a Scope in timeline-date order through this from step 6")
)]
impl<'a> ScopeTimeline<'a> {
    pub fn new(library: &'a LibraryBrowser, timeline: &'a TimelineService) -> Self {
        Self { library, timeline }
    }

    /// The page of the Scope at `anchor`, with at most `limit` Entries. `selections` holds, by Library id, the rating and tag selection that `options`' conditions
    /// came to for the requesting user in that Library, as for `LibraryBrowser::list_entries`.
    ///
    /// A kind other than images and videos in the Filter is `InvalidInput`, and a named Entry of `anchor` that is not in the sequence is `NotFound`.
    pub async fn page(
        &self,
        scope: &Scope,
        selections: &HashMap<String, MarkSelection>,
        options: &ListOptions,
        anchor: &TimelineAnchor,
        limit: usize,
    ) -> Result<ScopeTimelinePage> {
        let prepared = self.prepare(scope, selections, options).await?;
        let sequences = &prepared.sequences;
        let limit = limit.max(1);
        let entries = match anchor {
            TimelineAnchor::Newest => Self::older(sequences, None, limit).await?,
            TimelineAnchor::After(cursor) => Self::older(sequences, Some(cursor), limit).await?,
            TimelineAnchor::Before(cursor) => Self::newer(sequences, cursor, limit).await?,
            TimelineAnchor::Month(month) => {
                let end = TimelineIndex::month_end(month)?;
                let mut found = Vec::new();
                for sequence in sequences {
                    found.extend(sequence.older_than_date(&end, limit).await?);
                }
                TimelineMatch::merge(found, limit)
            }
            TimelineAnchor::Around { library, path } => {
                let mut center = None;
                for sequence in sequences {
                    if let Some(found) = sequence.find(library, path).await? {
                        center = Some(found);
                        break;
                    }
                }
                let Some(center) = center else {
                    return Err(Error::new(ErrorKind::NotFound).with_message(format!("timeline entry not found: {library}{path}")));
                };
                let cursor = center.cursor();
                let mut entries = Self::newer(sequences, &cursor, (limit - 1) / 2).await?;
                entries.push(center);
                let older_count = limit - entries.len();
                entries.extend(Self::older(sequences, Some(&cursor), older_count).await?);
                entries
            }
        };
        let previous = match entries.first() {
            Some(first) if Self::any_newer(sequences, &first.cursor()).await? => Some(first.cursor()),
            _ => None,
        };
        let next = match entries.last() {
            Some(last) if Self::any_older(sequences, &last.cursor()).await? => Some(last.cursor()),
            _ => None,
        };
        Ok(ScopeTimelinePage {
            entries,
            previous,
            next,
            cut_off_starts: prepared.cut_off_starts,
            walked_starts: prepared.walked_starts,
            missing_starts: prepared.missing_starts,
        })
    }

    /// The number of Entries of the Scope in each month, newest month first, without the months that have none. They equal the Entries `page` returns for each month.
    pub async fn months(&self, scope: &Scope, selections: &HashMap<String, MarkSelection>, options: &ListOptions) -> Result<Vec<(String, u64)>> {
        let prepared = self.prepare(scope, selections, options).await?;
        let mut months = BTreeMap::<String, u64>::new();
        for sequence in &prepared.sequences {
            for (month, count) in sequence.months().await? {
                *months.entry(month).or_default() += count;
            }
        }
        Ok(months.into_iter().rev().collect())
    }

    /// Resolves the Scope, which drops the starts that other starts cover and names the missing ones, and makes the sequences to merge.
    async fn prepare(&self, scope: &Scope, selections: &HashMap<String, MarkSelection>, options: &ListOptions) -> Result<ScopeSequences> {
        let kinds = options
            .kinds()
            .iter()
            .map(|kind| match kind {
                crate::service::listing::EntryKind::Image => Ok(TimelineKind::Image),
                crate::service::listing::EntryKind::Video => Ok(TimelineKind::Video),
                _ => Err(Error::new(ErrorKind::InvalidInput).with_message("kind must be image or video in timeline-date order")),
            })
            .collect::<Result<Vec<_>>>()?;
        let resolved = self.library.resolve_scope(scope).await?;
        let mut prepared = ScopeSequences {
            sequences: Vec::new(),
            cut_off_starts: Vec::new(),
            walked_starts: Vec::new(),
            missing_starts: resolved.missing().to_vec(),
        };
        if options.has_conditions() {
            for source in self.library.scope_sources(&resolved, selections, options).await? {
                let timeline = self.timeline.snapshot_timeline(source.snapshot(), source.library_id(), source.options()).await?;
                prepared.sequences.push(TimelineSequence::Snapshot(timeline));
                if source.is_cut_off() {
                    prepared.cut_off_starts.push(source.start().to_string());
                }
                if source.is_walked() {
                    prepared.walked_starts.push(source.start().to_string());
                }
            }
        } else if !resolved.starts().is_empty() {
            let starts = Self::reading_starts(&resolved, options.recursive());
            let reading = TimelineReading::new(starts, options.recursive(), options.inside_archives(), kinds);
            prepared.sequences.push(TimelineSequence::Rows(self.timeline.rows(reading)?));
        }
        Ok(prepared)
    }

    /// The starts of `resolved` that the rows are read from. A recursive start that is an archive or inside one reads every row under it, whether or not the
    /// Scope reads the insides of archives, so another start at or under its path adds nothing, and `resolve_scope` keeps such a start when the Scope does not read
    /// the insides. Of identical starts the first is kept, as `resolve_scope` does.
    fn reading_starts(resolved: &ResolvedScope, recursive: bool) -> Vec<TimelineStart> {
        let starts = resolved
            .starts()
            .iter()
            .map(|start| TimelineStart::new(start.start().library_id(), &start.start().path().absolute(), start.folder().archive_identity().is_some()))
            .collect::<Vec<_>>();
        let mut kept = Vec::<TimelineStart>::new();
        for (index, start) in starts.iter().enumerate() {
            let covered = recursive
                && starts
                    .iter()
                    .enumerate()
                    .any(|(other, upper)| other != index && upper.reads_every_row_under(start) && (!start.reads_every_row_under(upper) || other < index));
            if !covered {
                kept.push(start.clone());
            }
        }
        kept
    }

    async fn older(sequences: &[TimelineSequence], cursor: Option<&TimelineCursor>, limit: usize) -> Result<Vec<TimelineMatch>> {
        let mut found = Vec::new();
        for sequence in sequences {
            found.extend(sequence.older(cursor, limit).await?);
        }
        Ok(TimelineMatch::merge(found, limit))
    }

    /// The `limit` Entries newer than `cursor` that are nearest to it, newest first.
    async fn newer(sequences: &[TimelineSequence], cursor: &TimelineCursor, limit: usize) -> Result<Vec<TimelineMatch>> {
        let mut found = Vec::new();
        for sequence in sequences {
            found.extend(sequence.newer(cursor, limit).await?);
        }
        let merged = TimelineMatch::merge(found, usize::MAX);
        let skipped = merged.len().saturating_sub(limit);
        Ok(merged.into_iter().skip(skipped).collect())
    }

    async fn any_newer(sequences: &[TimelineSequence], cursor: &TimelineCursor) -> Result<bool> {
        for sequence in sequences {
            if sequence.exists_newer(cursor).await? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn any_older(sequences: &[TimelineSequence], cursor: &TimelineCursor) -> Result<bool> {
        for sequence in sequences {
            if sequence.exists_older(cursor).await? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone as _, Utc};
    use omnius_pxna_engine::model::Rating;
    use omnius_pxna_interface::models;
    use testresult::TestResult;

    use super::*;
    use crate::{
        config::{CoreConfig, DaemonConfig, LibraryConfig, ListingConfig, LoggingConfig, StorageConfig},
        service::listing::tests::write_nested_archives,
        state::DaemonState,
    };

    /// The daemon over two Libraries, `photos` at `dir/library` and `videos` at `dir/videos`, and where they are.
    struct Fixture {
        dir: tempfile::TempDir,
        state: Arc<DaemonState>,
    }

    async fn fixture_with(listing: ListingConfig, clock: Arc<dyn omnius_core_base::clock::Clock<Utc> + Send + Sync>) -> TestResult<Fixture> {
        let dir = tempfile::tempdir()?;
        let library = |id: &str, path: std::path::PathBuf| LibraryConfig {
            id: id.to_string(),
            name: id.to_string(),
            path,
            readonly: false,
        };
        std::fs::create_dir_all(dir.path().join("library"))?;
        std::fs::create_dir_all(dir.path().join("videos"))?;
        let conf = DaemonConfig {
            core: CoreConfig {
                state_dir: dir.path().join("state"),
                listen_addr: "127.0.0.1:0".to_string(),
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                json: false,
                file: None,
                performance: false,
            },
            libraries: vec![library("photos", dir.path().join("library")), library("videos", dir.path().join("videos"))],
            storage: StorageConfig { archive: Default::default() },
            listing,
            timeline: Default::default(),
            thumbnail: Default::default(),
            scheduler: Default::default(),
            media: Default::default(),
            video: Default::default(),
            web: Default::default(),
        };
        Ok(Fixture {
            state: Arc::new(DaemonState::new_with_clock(conf, clock).await?),
            dir,
        })
    }

    /// Writes a file of `size` bytes whose modification time, which is its timeline date, is noon UTC on `date`, as `year-month-day`.
    fn write(path: &std::path::Path, size: usize, date: (i32, u32, u32)) -> TestResult {
        std::fs::create_dir_all(path.parent().ok_or("a path under a folder")?)?;
        std::fs::write(path, vec![b'x'; size])?;
        let time = Utc.with_ymd_and_hms(date.0, date.1, date.2, 12, 0, 0).single().ok_or("a date")?;
        std::fs::File::options().write(true).open(path)?.set_modified(time.into())?;
        Ok(())
    }

    /// Images and videos dated by their modification times, and the archive `pack.7z` in the folder `photos` of `photos`, which holds `dir/b.png` and `nested.7z`,
    /// whose `c.png` is the only other image.
    fn files(fixture: &Fixture) -> TestResult {
        let dir = fixture.dir.path();
        write(&dir.join("library/photos/p1.png"), 1, (2024, 3, 15))?;
        write(&dir.join("library/photos/p2.mp4"), 2, (2024, 3, 14))?;
        write(&dir.join("library/photos/sub/p3.png"), 3, (2024, 2, 15))?;
        write(&dir.join("library/top.png"), 4, (2024, 1, 15))?;
        write(&dir.join("videos/x/v1.mp4"), 5, (2024, 3, 15))?;
        write(&dir.join("videos/v2.png"), 6, (2023, 12, 15))?;
        // The helper writes under the root of a Library and its scratch files beside it, so the archive is moved into the folder afterwards.
        write_nested_archives(&dir.join("library"))?;
        std::fs::rename(dir.join("library/pack.7z"), dir.join("library/photos/pack.7z"))?;
        Ok(())
    }

    /// Runs a full aggregation of `library` and waits for it, which makes its name index usable.
    async fn aggregate(state: &DaemonState, library: &str) -> TestResult {
        state.names.verified().await;
        state.timeline.refresh(library).await?;
        for _ in 0..400 {
            let status = state.timeline.status(library).await?;
            if !status.running && status.completed_at.is_some() {
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        Err("the aggregation did not finish".into())
    }

    async fn aggregated() -> TestResult<Fixture> {
        aggregated_with_clock(Arc::new(omnius_core_base::clock::ClockUtc)).await
    }

    async fn aggregated_with_clock(clock: Arc<dyn omnius_core_base::clock::Clock<Utc> + Send + Sync>) -> TestResult<Fixture> {
        let fixture = fixture_with(Default::default(), clock).await?;
        files(&fixture)?;
        aggregate(&fixture.state, "photos").await?;
        aggregate(&fixture.state, "videos").await?;
        Ok(fixture)
    }

    /// The parameters of `listEntries` that the Filter and the Scope's choices come from.
    fn options(recursive: bool, inside_archives: bool, query: Option<&str>, rating: Option<&str>, kinds: &[&str]) -> TestResult<ListOptions> {
        Ok(ListOptions::from_entries(&models::ListEntriesQueryParams {
            start: vec!["photos".to_string()],
            recursive: Some(recursive),
            inside_archives: Some(inside_archives),
            query: query.map(str::to_string),
            rating: rating.map(str::to_string),
            tag: Vec::new(),
            kind: kinds.iter().map(|kind| kind.to_string()).collect(),
            sort: None,
            order: None,
            folders_first: None,
            offset: None,
            locate: None,
            limit: None,
        })?)
    }

    fn scope(starts: &[&str], options: &ListOptions) -> TestResult<Scope> {
        let values = starts.iter().map(|start| start.to_string()).collect::<Vec<_>>();
        Ok(Scope::parse(&values, options.recursive(), options.inside_archives())?)
    }

    fn listed(page: &ScopeTimelinePage) -> Vec<String> {
        page.entries.iter().map(|found| format!("{}:{}", found.library, found.entry.path)).collect()
    }

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    async fn page(state: &DaemonState, starts: &[&str], options: &ListOptions, anchor: &TimelineAnchor, limit: usize) -> Result<ScopeTimelinePage> {
        let scope = Scope::parse(
            &starts.iter().map(|start| start.to_string()).collect::<Vec<_>>(),
            options.recursive(),
            options.inside_archives(),
        )?;
        ScopeTimeline::new(&state.library, &state.timeline)
            .page(&scope, &HashMap::new(), options, anchor, limit)
            .await
    }

    async fn months(state: &DaemonState, starts: &[&str], options: &ListOptions) -> Result<Vec<(String, u64)>> {
        let scope = Scope::parse(
            &starts.iter().map(|start| start.to_string()).collect::<Vec<_>>(),
            options.recursive(),
            options.inside_archives(),
        )?;
        ScopeTimeline::new(&state.library, &state.timeline).months(&scope, &HashMap::new(), options).await
    }

    /// Every Entry of the Scope, read page by page by the cursors, and the months of what was read, which must join without a repeat or a gap.
    async fn read_all(state: &DaemonState, starts: &[&str], options: &ListOptions, limit: usize) -> TestResult<(Vec<String>, Vec<(String, u64)>)> {
        let mut seen = Vec::new();
        let mut dates = Vec::new();
        let mut page = page(state, starts, options, &TimelineAnchor::Newest, limit).await?;
        assert_eq!(page.previous, None);
        loop {
            seen.extend(listed(&page));
            dates.extend(page.entries.iter().map(|found| found.date.text()));
            let Some(next) = page.next.clone() else { break };
            page = self::page(state, starts, options, &TimelineAnchor::After(next), limit).await?;
        }
        let mut months = Vec::<(String, u64)>::new();
        for date in dates {
            match months.last_mut() {
                Some((month, count)) if *month == date[..7] => *count += 1,
                _ => months.push((date[..7].to_string(), 1)),
            }
        }
        Ok((seen, months))
    }

    /// Reads the sequence backward from its last Entry by `before`, which must give the same sequence.
    async fn read_back(state: &DaemonState, starts: &[&str], options: &ListOptions, limit: usize) -> TestResult<Vec<String>> {
        let all = page(state, starts, options, &TimelineAnchor::Newest, 1000).await?;
        let Some(last) = all.entries.last() else { return Ok(Vec::new()) };
        let mut back = vec![format!("{}:{}", last.library, last.entry.path)];
        let mut page = page(state, starts, options, &TimelineAnchor::Before(last.cursor()), limit).await?;
        loop {
            let mut current = listed(&page);
            current.extend(back);
            back = current;
            let Some(previous) = page.previous.clone() else { break };
            page = self::page(state, starts, options, &TimelineAnchor::Before(previous), limit).await?;
        }
        Ok(back)
    }

    #[tokio::test]
    async fn the_rows_of_two_libraries_are_one_sequence_in_timeline_date_order_test() -> TestResult {
        let fixture = aggregated().await?;
        let state = &fixture.state;
        let options = options(true, false, None, None, &[])?;

        let all = page(state, &["photos", "videos"], &options, &TimelineAnchor::Newest, 100).await?;
        assert_eq!(
            listed(&all),
            strings(&[
                "photos:/photos/p1.png",
                "videos:/x/v1.mp4",
                "photos:/photos/p2.mp4",
                "photos:/photos/sub/p3.png",
                "photos:/top.png",
                "videos:/v2.png"
            ]),
            "newest first, the same timeline date by Library id, and no Entry inside an archive"
        );
        assert!(all.previous.is_none() && all.next.is_none());
        assert!(all.cut_off_starts.is_empty() && all.walked_starts.is_empty() && all.missing_starts.is_empty());
        let first = &all.entries[0];
        assert_eq!(first.library, "photos");
        assert_eq!(first.date.source.as_str(), "modified");
        assert_eq!((first.entry.name.as_str(), first.entry.metadata.size), ("p1.png", 1), "the Entry is built from the row");
        assert_eq!(
            all.entries[1].entry.metadata.content_type,
            Some(omnius_pxna_interface::types::Nullable::Present("video/mp4".to_string()))
        );

        // Pages join in both directions without a repeat or a gap.
        for limit in 1..=4 {
            let (forward, counted) = read_all(state, &["photos", "videos"], &options, limit).await?;
            assert_eq!(forward, listed(&all), "forward by {limit}");
            assert_eq!(counted, months(state, &["photos", "videos"], &options).await?);
            assert_eq!(read_back(state, &["photos", "videos"], &options, limit).await?, listed(&all), "backward by {limit}");
        }
        let two = page(state, &["photos", "videos"], &options, &TimelineAnchor::Newest, 2).await?;
        assert!(two.previous.is_none() && two.next.is_some());

        // A month starts at its newest Entry, or at the next older month that has one.
        let february = page(state, &["photos", "videos"], &options, &TimelineAnchor::Month("2024-02".to_string()), 2).await?;
        assert_eq!(listed(&february), strings(&["photos:/photos/sub/p3.png", "photos:/top.png"]));
        assert!(february.previous.is_some() && february.next.is_some());
        let gap = page(state, &["photos", "videos"], &options, &TimelineAnchor::Month("2024-01".to_string()), 5).await?;
        assert_eq!(listed(&gap), strings(&["photos:/top.png", "videos:/v2.png"]));
        let april = page(state, &["photos", "videos"], &options, &TimelineAnchor::Month("2024-04".to_string()), 1).await?;
        assert_eq!(listed(&april), strings(&["photos:/photos/p1.png"]));

        // Around: the named Entry near the middle, and NotFound for one that the sequence does not hold.
        let around = |library: &str, path: &str| TimelineAnchor::Around {
            library: library.to_string(),
            path: path.to_string(),
        };
        let middle = page(state, &["photos", "videos"], &options, &around("photos", "/photos/p2.mp4"), 3).await?;
        assert_eq!(listed(&middle), strings(&["videos:/x/v1.mp4", "photos:/photos/p2.mp4", "photos:/photos/sub/p3.png"]));
        for (library, path) in [
            ("photos", "/photos/notes.txt"),
            ("photos", "/photos/a.7z/dir/x.png"),
            ("videos", "/photos/p1.png"),
            ("photos", "/missing.png"),
        ] {
            let error = page(state, &["photos", "videos"], &options, &around(library, path), 3).await.unwrap_err();
            assert_eq!(*error.kind(), ErrorKind::NotFound, "{library}{path}");
        }
        Ok(())
    }

    #[tokio::test]
    async fn folder_archive_and_direct_children_starts_read_their_rows_test() -> TestResult {
        let fixture = aggregated().await?;
        let state = &fixture.state;
        let read = |starts: &'static [&'static str], recursive: bool, inside: bool, kinds: &'static [&'static str]| async move {
            let options = options(recursive, inside, None, None, kinds)?;
            let found = page(state, starts, &options, &TimelineAnchor::Newest, 100).await?;
            TestResult::Ok((listed(&found), found.missing_starts))
        };

        let folder = read(&["photos/photos"], true, false, &[]).await?.0;
        assert_eq!(folder, strings(&["photos:/photos/p1.png", "photos:/photos/p2.mp4", "photos:/photos/sub/p3.png"]));
        let inside = read(&["photos/photos"], true, true, &[]).await?.0;
        assert_eq!(inside.len(), 3 + 2, "the images inside pack.7z and in its nested archive");
        assert!(inside.contains(&"photos:/photos/pack.7z/dir/b.png".to_string()) && inside.contains(&"photos:/photos/pack.7z/nested.7z/c.png".to_string()));
        assert_eq!(
            read(&["photos/photos"], false, true, &[]).await?.0,
            strings(&["photos:/photos/p1.png", "photos:/photos/p2.mp4"]),
            "direct children, whose archives' insides are descendants"
        );
        assert_eq!(
            read(&["photos"], false, false, &[]).await?.0,
            strings(&["photos:/top.png"]),
            "the direct children of a root"
        );

        // An archive start reads every row under it, the Entries of the nested archives included, with or without insideArchives.
        let sorted = |mut found: Vec<String>| {
            found.sort();
            found
        };
        let archive = sorted(read(&["photos/photos/pack.7z"], true, false, &[]).await?.0);
        assert_eq!(archive, strings(&["photos:/photos/pack.7z/dir/b.png", "photos:/photos/pack.7z/nested.7z/c.png"]));
        assert_eq!(sorted(read(&["photos/photos/pack.7z"], true, true, &[]).await?.0), archive);
        assert_eq!(
            read(&["photos/photos/pack.7z/nested.7z"], true, false, &[]).await?.0,
            strings(&["photos:/photos/pack.7z/nested.7z/c.png"]),
            "a path inside an archive"
        );
        assert!(
            read(&["photos/photos/pack.7z"], false, false, &[]).await?.0.is_empty(),
            "`a.txt` is no image and `dir/b.png` is not a direct child"
        );

        // The kinds.
        assert_eq!(
            read(&["photos", "videos"], true, false, &["video"]).await?.0,
            strings(&["videos:/x/v1.mp4", "photos:/photos/p2.mp4"])
        );
        let images = read(&["photos", "videos"], true, false, &["image"]).await?.0;
        assert_eq!(
            images,
            strings(&["photos:/photos/p1.png", "photos:/photos/sub/p3.png", "photos:/top.png", "videos:/v2.png"])
        );

        // A kind that is not an image or a video is InvalidInput, whether or not there is a condition.
        for kind in ["folder", "archive", "other"] {
            for query in [None, Some("png")] {
                let options = options(true, false, query, None, &["image", kind])?;
                let error = page(state, &["photos"], &options, &TimelineAnchor::Newest, 10).await.unwrap_err();
                assert_eq!(*error.kind(), ErrorKind::InvalidInput, "{kind} {query:?}");
                assert_eq!(*months(state, &["photos"], &options).await.unwrap_err().kind(), ErrorKind::InvalidInput);
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn missing_and_covered_starts_are_named_or_add_nothing_test() -> TestResult {
        let fixture = aggregated().await?;
        let state = &fixture.state;
        let options = options(true, false, None, None, &[])?;
        let base = page(state, &["photos"], &options, &TimelineAnchor::Newest, 100).await?;
        let with = page(
            state,
            &["photos", "photos/photos", "nope/x", "photos", "photos/gone", "photos/photos/notes.txt"],
            &options,
            &TimelineAnchor::Newest,
            100,
        )
        .await?;
        assert_eq!(listed(&with), listed(&base), "starts that `photos` covers add nothing");
        assert_eq!(with.missing_starts, strings(&["nope/x", "photos/gone", "photos/photos/notes.txt"]));
        // Every start missing is an empty sequence that names them.
        let none = page(state, &["nope"], &options, &TimelineAnchor::Newest, 10).await?;
        assert!(none.entries.is_empty() && none.previous.is_none() && none.next.is_none());
        assert_eq!(none.missing_starts, strings(&["nope"]));
        assert!(months(state, &["nope"], &options).await?.is_empty());

        // A start inside an archive is covered by a recursive start above it only with the insides.
        let archive = "photos/photos/pack.7z";
        let without = page(state, &["photos", archive], &options, &TimelineAnchor::Newest, 100).await?;
        assert_eq!(
            without.entries.len(),
            base.entries.len() + 2,
            "the archive start adds its insides to a root that leaves them out"
        );
        let inside = self::options(true, true, None, None, &[])?;
        let root = page(state, &["photos"], &inside, &TimelineAnchor::Newest, 100).await?;
        let covered = page(state, &["photos", archive], &inside, &TimelineAnchor::Newest, 100).await?;
        assert_eq!(listed(&covered), listed(&root));
        Ok(())
    }

    #[tokio::test]
    async fn a_start_under_an_archive_start_adds_neither_rows_nor_months_test() -> TestResult {
        let fixture = aggregated().await?;
        let state = &fixture.state;
        // Without the insides of archives resolve_scope keeps a start inside an archive, but an archive start reads every row under it already.
        let archive = "photos/photos/pack.7z";
        let nested = "photos/photos/pack.7z/nested.7z";
        let folder = "photos/photos/pack.7z/dir";
        for (recursive, inside) in [(true, false), (true, true), (false, false)] {
            let options = options(recursive, inside, None, None, &[])?;
            for starts in [
                vec![archive],
                vec![archive, nested],
                vec![nested, archive],
                vec![archive, nested, folder],
                vec![folder, nested, archive, archive],
                vec!["photos/photos", archive, nested],
                vec![nested, nested, folder],
            ] {
                let all = page(state, &starts, &options, &TimelineAnchor::Newest, 100).await?;
                let (seen, counted) = read_all(state, &starts, &options, 1).await?;
                assert_eq!(seen, listed(&all), "{starts:?} {recursive} {inside}");
                assert_eq!(months(state, &starts, &options).await?, counted, "{starts:?} {recursive} {inside}");
            }
        }
        // The rows under the archive are read once whatever starts lie under it, and a start that is not under it still adds its own.
        let options = options(true, false, None, None, &[])?;
        let under = page(state, &[archive, nested, folder], &options, &TimelineAnchor::Newest, 100).await?;
        let alone = page(state, &[archive], &options, &TimelineAnchor::Newest, 100).await?;
        assert_eq!(listed(&under), listed(&alone));
        assert_eq!(months(state, &[archive, nested, folder], &options).await?, months(state, &[archive], &options).await?);
        assert_eq!(alone.entries.len(), 2);
        let with_folder = months(state, &["photos/photos/sub", archive, nested], &options).await?;
        assert_eq!(with_folder.iter().map(|(_, count)| count).sum::<u64>(), 3);
        Ok(())
    }

    #[tokio::test]
    async fn the_months_equal_the_pages_for_every_way_of_reading_test() -> TestResult {
        let fixture = aggregated().await?;
        let state = &fixture.state;
        let mut checked = 0;
        for starts in [
            vec!["photos", "videos"],
            vec!["videos"],
            vec!["photos/photos"],
            vec!["photos/photos", "videos/x"],
            vec!["photos/photos/pack.7z", "videos"],
            vec!["photos/photos/pack.7z/nested.7z"],
        ] {
            for (recursive, inside) in [(true, false), (true, true), (false, false)] {
                for kinds in [&[][..], &["image"], &["video"], &["image", "video"]] {
                    for query in [None, Some("png"), Some("x")] {
                        let options = options(recursive, inside, query, None, kinds)?;
                        let (seen, counted) = read_all(state, &starts, &options, 2).await?;
                        assert_eq!(months(state, &starts, &options).await?, counted, "{starts:?} {recursive} {inside} {kinds:?} {query:?}");
                        assert_eq!(read_back(state, &starts, &options, 3).await?, seen);
                        checked += seen.len();
                    }
                }
            }
        }
        assert!(checked > 100, "the readings are not all empty");
        Ok(())
    }

    #[tokio::test]
    async fn a_condition_orders_the_matches_of_each_start_with_the_snapshots_entries_test() -> TestResult {
        let clock = Arc::new(omnius_core_base::clock::FakeClockUtc::new(Utc::now()));
        let fixture = aggregated_with_clock(clock.clone()).await?;
        let state = &fixture.state;
        let plain = options(true, false, None, None, &["image"])?;
        let named = options(true, false, Some("png"), None, &[])?;

        let before = page(state, &["photos", "videos"], &named, &TimelineAnchor::Newest, 100).await?;
        let rows = page(state, &["photos", "videos"], &plain, &TimelineAnchor::Newest, 100).await?;
        assert_eq!(listed(&before), listed(&rows), "the matches of a name are the images the rows list, in the same order");
        assert!(before.walked_starts.is_empty() && before.cut_off_starts.is_empty());

        // The Entry of a match is read from the file system by the snapshot, so it is newer than its row, which is as old as the last aggregation.
        let path = fixture.dir.path().join("library/photos/p1.png");
        std::fs::write(&path, vec![b'y'; 100])?;
        write(&path, 100, (2024, 3, 15))?;
        let rebuilt = page(state, &["photos", "videos"], &named, &TimelineAnchor::Newest, 100).await?;
        let sizes = |found: &ScopeTimelinePage| found.entries.iter().map(|found| found.entry.metadata.size).collect::<Vec<_>>();
        assert_eq!(sizes(&rebuilt)[0], 1, "the snapshot of the start is reused until it expires");
        clock.advance(std::time::Duration::from_secs(ListingConfig::default().snapshot_max_age_secs + 1));
        let current = page(state, &["photos", "videos"], &named, &TimelineAnchor::Newest, 100).await?;
        assert_eq!(sizes(&current)[0], 100, "the Entry of the snapshot, and not the row's size 1");
        assert_eq!(listed(&current), listed(&rows));
        assert_eq!(current.entries[0].date.text(), rows.entries[0].date.text(), "with the date of its row");
        Ok(())
    }

    #[tokio::test]
    async fn the_starts_with_a_condition_that_were_walked_or_cut_off_are_named_test() -> TestResult {
        let listing = ListingConfig {
            recursive_max_entries: 3,
            ..Default::default()
        };
        let fixture = fixture_with(listing, Arc::new(omnius_core_base::clock::ClockUtc)).await?;
        let dir = fixture.dir.path();
        for number in 0..6 {
            write(&dir.join(format!("library/big/sun-{number}.png")), 1, (2024, 3, 10 + number))?;
        }
        write(&dir.join("videos/sun.mp4"), 1, (2024, 3, 1))?;
        let state = &fixture.state;
        let named = options(true, false, Some("sun"), None, &[])?;

        // The index of `photos` is not usable, and nothing is aggregated, so the matches are found by a walk but have no row to be dated by.
        let before = page(state, &["photos/big", "videos"], &named, &TimelineAnchor::Newest, 100).await?;
        assert!(before.entries.is_empty(), "a match without a row is not returned");
        assert_eq!(before.walked_starts, strings(&["photos/big", "videos"]));
        assert_eq!(before.cut_off_starts, strings(&["photos/big"]));
        assert!(months(state, &["photos/big", "videos"], &named).await?.is_empty());

        aggregate(state, "photos").await?;
        aggregate(state, "videos").await?;
        let after = page(state, &["photos/big", "videos"], &named, &TimelineAnchor::Newest, 100).await?;
        assert!(after.walked_starts.is_empty());
        assert_eq!(after.cut_off_starts, strings(&["photos/big"]), "the limit counts the matches of the index too");
        assert_eq!(after.entries.len(), 3 + 1, "the matches up to the limit of the start, and the other start's");
        assert_eq!(months(state, &["photos/big", "videos"], &named).await?, [("2024-03".to_string(), 4)]);
        // Without a condition the rows have no limit and no walk.
        let plain = options(true, false, None, None, &[])?;
        let rows = page(state, &["photos/big", "videos"], &plain, &TimelineAnchor::Newest, 100).await?;
        assert_eq!(rows.entries.len(), 7);
        assert!(rows.cut_off_starts.is_empty() && rows.walked_starts.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn a_rating_condition_keeps_the_rated_matches_in_timeline_date_order_test() -> TestResult {
        let fixture = aggregated().await?;
        let state = &fixture.state;
        for path in ["/photos/p1.png", "/top.png"] {
            state.rating_tags.set_rating("owner", "photos", path, Some(Rating::Good)).await?;
        }
        state.rating_tags.set_rating("owner", "videos", "/v2.png", Some(Rating::Good)).await?;
        let good = options(true, false, None, Some("good"), &[])?;
        let mut selections = HashMap::new();
        for library in ["photos", "videos"] {
            selections.insert(library.to_string(), good.marks().select(&state.rating_tags, "owner", library).await?);
        }
        let timeline = ScopeTimeline::new(&state.library, &state.timeline);
        let scope = scope(&["photos", "videos"], &good)?;
        let found = timeline.page(&scope, &selections, &good, &TimelineAnchor::Newest, 100).await?;
        assert_eq!(listed(&found), strings(&["photos:/photos/p1.png", "photos:/top.png", "videos:/v2.png"]));
        assert_eq!(
            timeline.months(&scope, &selections, &good).await?,
            [("2024-03".to_string(), 1), ("2024-01".to_string(), 1), ("2023-12".to_string(), 1)]
        );
        let unrated = options(true, false, None, Some("none"), &["image"])?;
        let mut selections = HashMap::new();
        for library in ["photos", "videos"] {
            selections.insert(library.to_string(), unrated.marks().select(&state.rating_tags, "owner", library).await?);
        }
        let found = timeline.page(&scope, &selections, &unrated, &TimelineAnchor::Newest, 100).await?;
        assert_eq!(listed(&found), strings(&["photos:/photos/sub/p3.png"]));
        Ok(())
    }
}
