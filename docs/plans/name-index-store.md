# Build the Name Index from Aggregation and Our Own Changes

## 1. About This Plan

This work adds the name index and keeps it updated by aggregation's walk and by the daemon's own changes. Answering listings from it is the following plan, `name-index-search`.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The steps and acceptance checks of building and updating the name index |
| [Listing and Search Design §5.1](../design/listing.md#51-name-index) | What the index holds, how it is updated, when it is usable, how it fails |
| [Listing and Search Design §8.1](../design/listing.md#name-index) | The decision to use a Library-wide name index and its reasons |
| [Timeline Design §6](../design/timeline.md#6-aggregation) | The walk that passes names to the index, and when aggregation starts |
| [design.md §9.2](../design.md#92-where-state-lives) | Where the index lives and the impact of losing it |
| [SQLite FTS5 trigram](../knowledge/sqlite-fts5-trigram.md) | The behavior of the trigram tokenizer the index depends on |

[design.md](../design.md#11-division-of-responsibility-between-documents) holds the map of the document family.

### 1.2 Creation Date and Target Commit

Created 2026-10-23 / Target commit `47b81b0`

## 2. Purpose and Completion Conditions

After the first full aggregation, every non-hidden Entry outside archives of a Library is held in a persistent name index that follows the daemon's own changes, without adding a walk of the Library.

### 2.1 Completion Conditions

1. After a full aggregation of a test Library, the index holds exactly the non-hidden Entries outside archives, with their normalized names and kinds, and reports the Library as usable
2. Before a response, a delete through Pxna removes the rows of the path and everything under it, and a rename or a move within the Library reassigns them to the destination
3. A full aggregation removes the rows of folders and files that it did not see, and a range aggregation adds the Entries of the folder it enumerates
4. A corrupt or unknown-format index file is discarded at startup and rebuilt by the next full aggregation, the authentication DB is untouched, and an I/O error while running stops the use of the index for the process
5. At startup, a Library whose name index is not usable starts a full aggregation even when its timeline index has files and `scan_interval_secs` is 0
6. On the 10-million-Entry fixture, a full aggregation takes at most 20% longer than at the target commit, and the index file takes at most 200 bytes per Entry, recorded in `docs/measurements`
7. `cd daemon && cargo make lint` and the daemon tests pass

### 2.2 Non-Goals

- Answering listings from the index. That is `name-index-search`.
- Holding the contents of archives or hidden Entries. Listing and Search Design §2.1 excludes them.
- Following external changes without aggregation. Listing and Search Design §8.2 keeps it deferred.

## 3. Current State

The listing state directory is opened at [state.rs#L71](../../daemon/entrypoints/daemon/src/state.rs#L71) and holds only the browse index.
Aggregation walks every non-hidden folder at [scanner.rs#L132](../../daemon/entrypoints/daemon/src/service/timeline/scanner.rs#L132), enumerating each with [scanner.rs#L160](../../daemon/entrypoints/daemon/src/service/timeline/scanner.rs#L160), but keeps only image, video, and archive candidates at [scanner.rs#L149](../../daemon/entrypoints/daemon/src/service/timeline/scanner.rs#L149) and drops the other names.
A scan ends with the timeline sweep at [scanner.rs#L129](../../daemon/entrypoints/daemon/src/service/timeline/scanner.rs#L129), and a full scan is completed at [timeline.rs#L193](../../daemon/entrypoints/daemon/src/service/timeline.rs#L193).
Startup starts a full aggregation only for Libraries without timeline files at [timeline.rs#L120](../../daemon/entrypoints/daemon/src/service/timeline.rs#L120).
The daemon's own changes reach the timeline and the browse caches at [mutation.rs#L57](../../daemon/entrypoints/daemon/src/service/mutation.rs#L57), and renames at [mutation.rs#L87](../../daemon/entrypoints/daemon/src/service/mutation.rs#L87).
There is no name index, so a recursive search can only walk.

## 4. Change Policy

Add a `NameIndex` component under the listing service that owns its own SQLite file, give it to the timeline scanner and to `LibraryMutation`, and have each feed it at the points where they already update the timeline.

### 4.1 Decisions Assumed

| Decision | Source |
| --- | --- |
| A Library-wide name index updated by aggregation and our own changes | [Listing and Search Design §8.1](../design/listing.md#name-index) |
| Contents, update points, usability, failure handling, no capacity limit | [Listing and Search Design §5.1](../design/listing.md#51-name-index) |
| Matching through a trigram index with `case_sensitive 1` | [Listing and Search Design §8.1](../design/listing.md#match-names-in-the-index-with-trigrams-and-compare-short-terms-directly) |
| Aggregation passes names and starts for a Library without a usable name index | [Timeline Design §6.1](../design/timeline.md#61-scanning-and-skipping-rereads), [§6.3](../design/timeline.md#63-when-aggregation-starts) |

### 4.2 Scope of Impact

The daemon's listing service, timeline service and scanner, `LibraryMutation`, and `DaemonState` change.
A new file `state_dir/listing/names.db` appears. Older daemons ignore it. No API or configuration changes.
Disk use grows by the size of the index, which is proportional to the number of Entries.

## 5. Steps

| No. | Content | Dependencies | State |
| --- | --- | --- | --- |
| [1](#s-1) | Add the name index store | - | Not started |
| [2](#s-2) | Open it with the daemon and hand it out | 1 | Not started |
| [3](#s-3) | Feed it from aggregation | 2 | Not started |
| [4](#s-4) | Feed it from our own changes | 2 | Not started |
| [5](#s-5) | Start aggregation for a Library without a usable index | 3 | Not started |
| [6](#s-6) | Measure on the 10-million-Entry fixture | 3, 4 | Not started |

<a id="s-1"></a>
### Step 1. Add the Name Index Store

**Locations to change**
New `daemon/entrypoints/daemon/src/service/listing/name_index.rs`, declared next to the summary index at [listing.rs#L17](../../daemon/entrypoints/daemon/src/service/listing.rs#L17).

**What to do**
Create `NameIndex` with a table of Library, Library-relative path, parent path, normalized name, kind, and generation, an FTS5 trigram table with `case_sensitive 1` over the normalized name, and per-Library state of the generation and whether it is usable. Normalize with the same function as name matching. Provide these operations, all of which disable the index for the process on error:

```rust
pub async fn replace_children(&self, library: &str, parent: &str, children: &[NameRow], generation: i64);
pub async fn sweep(&self, library: &str, scope: &str, generation: i64, scoped: bool);
pub async fn mark_usable(&self, library: &str, generation: i64);
pub async fn is_usable(&self, library: &str) -> bool;
pub async fn remove(&self, library: &str, path: &str);
pub async fn rename(&self, library: &str, src: &str, dst: &str);
```

Verify the format version at open, and discard the file when it is missing a version, has another version, or fails an integrity check.

**Completion condition**
Unit tests cover each operation, the delimiter rule of `rename` (`/a` does not move `/ab`), discarding a corrupt file, and disabling after an injected error.

<a id="s-2"></a>
### Step 2. Open It with the Daemon and Hand It Out

**Locations to change**
[state.rs#L71](../../daemon/entrypoints/daemon/src/state.rs#L71), the constructors of `TimelineService` in [timeline.rs](../../daemon/entrypoints/daemon/src/service/timeline.rs#L83) and `LibraryMutation` in [mutation.rs#L36](../../daemon/entrypoints/daemon/src/service/mutation.rs#L36)

**What to do**
Open `NameIndex` at `state_dir/listing/names.db` and pass an `Arc` of it to the timeline service, its scanner, and `LibraryMutation`. If it cannot be opened, pass a disabled index so that everything else starts.

**Completion condition**
The daemon starts with and without a writable listing directory, and existing tests pass.

<a id="s-3"></a>
### Step 3. Feed It from Aggregation

**Locations to change**
[scanner.rs#L137](../../daemon/entrypoints/daemon/src/service/timeline/scanner.rs#L137), [scanner.rs#L129](../../daemon/entrypoints/daemon/src/service/timeline/scanner.rs#L129), [timeline.rs#L193](../../daemon/entrypoints/daemon/src/service/timeline.rs#L193)

**What to do**
After each successful enumeration in `walk`, call `replace_children` with every non-hidden Entry of the folder and the scan's generation. Call `sweep` next to the timeline sweep. After `complete_scan`, call `mark_usable` with the generation.

**Completion condition**
A scanner test shows conditions 1 and 3 on a temporary Library with folders, non-media files, an archive, and hidden Entries.

<a id="s-4"></a>
### Step 4. Feed It from Our Own Changes

**Locations to change**
[mutation.rs#L57](../../daemon/entrypoints/daemon/src/service/mutation.rs#L57), [mutation.rs#L87](../../daemon/entrypoints/daemon/src/service/mutation.rs#L87)

**What to do**
In `renamed`, call `NameIndex::rename` before the timeline. In `apply_changes`, call `remove` for each changed path that no longer exists, using the same existence check as `forget_missing`.

**Completion condition**
Mutation tests show condition 2 for delete, rename, a move within a Library, and a move across Libraries.

<a id="s-5"></a>
### Step 5. Start Aggregation for a Library Without a Usable Index

**Locations to change**
[timeline.rs#L120](../../daemon/entrypoints/daemon/src/service/timeline.rs#L120)

**What to do**
Start a full aggregation at startup when the timeline index has no files or the name index is not usable for the Library.

**Completion condition**
A test like [timeline.rs#L760](../../daemon/entrypoints/daemon/src/service/timeline.rs#L760) shows condition 5.

<a id="s-6"></a>
### Step 6. Measure on the 10-Million-Entry Fixture

**Locations to change**
New `tools/performance/name_index_fixture.py`, a new record under `docs/measurements/` and its row in [measurements.md](../measurements.md)

**What to do**
Write a generator that creates, under a temporary directory given as an argument, 10 million empty files in about a million folders with Japanese and ASCII names, about 1% of them with image extensions, and that refuses to overwrite an existing directory. Run a full aggregation at the target commit and after the change, three times each on the same SSD, and record the medians and the index file size.

**Completion condition**
The record shows the ratio of the medians and the bytes per Entry, and both meet condition 6.

## 6. Acceptance Checks

| Condition | Command | Expected result |
| --- | --- | --- |
| 1, 3 | `cd daemon && RUSTC_WRAPPER= cargo nextest run -p omnius-pxna-daemon name_index timeline` | The store and scanner tests for contents and sweeping pass |
| 2 | `cd daemon && RUSTC_WRAPPER= cargo nextest run -p omnius-pxna-daemon mutation` | The tests for delete, rename, and moves pass |
| 4 | `cd daemon && RUSTC_WRAPPER= cargo nextest run -p omnius-pxna-daemon name_index` | The tests for a corrupt file and an injected error pass, and the authentication DB file is unchanged |
| 5 | `cd daemon && RUSTC_WRAPPER= cargo nextest run -p omnius-pxna-daemon startup` | The startup test with a usable timeline and an unusable name index passes |
| 6 | Read the new record in `docs/measurements/` | Aggregation median ratio is 1.20 or less, and the index is 200 bytes per Entry or less |
| 7 | `cd daemon && cargo make lint && RUSTC_WRAPPER= cargo nextest run --all-features` | Both exit with 0 |
