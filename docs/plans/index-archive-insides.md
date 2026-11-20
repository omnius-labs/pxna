# Hold the Insides of Archives in the Name Index

## 1. About This Plan

This work is M27 of the roadmap: aggregation fills the name index with the folders and Entries inside archives, the index keeps them in step with the archives, and every query of the index leaves them out until `listEntries` lets a Scope read them (M28).

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The steps and acceptance checks of holding the insides of archives in the name index |
| [Listing and Search Design §5.1](../design/listing.md#51-name-index) | What the index holds, the mark of archive folders, the record of archives, and how they are updated |
| [Listing and Search Design §8.1](../design/listing.md#81-decided) | The decisions on indexing the insides of archives, unopenable archives, replacing an archive's rows, the mark of archive folders, and counting the insides toward the targets |
| [Timeline Design §6.1](../design/timeline.md#61-scanning-and-skipping-rereads), [§6.2](../design/timeline.md#62-archive-extraction) | The walk that skips unchanged archives and reads archives in blocks |
| [design.md §12.2](../design.md#122-roadmap) | M27 and what follows it |

[design.md](../design.md#11-division-of-responsibility-between-documents) holds the map of the document family.

### 1.2 Creation Date and Target Commit

Created 2026-11-20 / Target commit `908b4c8`, with the decisions of §4.1 added on top of it

## 2. Purpose and Completion Conditions

After a full aggregation, the name index holds the folders and Entries inside every archive of a Library that can be opened, down to `max_nested_depth`, and keeps them while the archive does not change, so that M28 can answer a Scope that reads the insides of archives without opening them.

### 2.1 Completion Conditions

1. After a full aggregation of a test Library that has a ZIP with nested folders, a 7z, a ZIP inside a ZIP, a ZIP with a hidden folder, and an archive that cannot be opened, the index holds every non-hidden Entry and folder inside the archives that open, down to `max_nested_depth`, with their joined paths; the unopenable archive is an Entry without contents; every archive keeps its kind and has a folder row; and the folder rows of the archives and of everything inside them carry the mark, while no other folder row does
2. A second full aggregation with no change reads no archive, neither for the timeline nor for the name index, and keeps every row inside the archives; after an archive's file is replaced, the next full aggregation reads that archive alone and its rows are exactly the new contents
3. With the timeline index already holding every archive, a name index file that is deleted and rebuilt holds the insides after one full aggregation, which enumerates every archive for the name index alone and reads no archive for the timeline
4. A delete through Pxna of an archive, or of a folder that holds one, removes the rows and the record inside it; a rename or a move within the Library reassigns them, and the next full aggregation reads no archive again
5. A sweep, a change of kind, or a move to a hidden name that removes an archive removes its record too, so the archive is read again when it returns
6. A recursive listing with a name, rating, or tag condition, answered from the index, returns no Entry inside an archive
7. An index file of format 1 is discarded at startup and rebuilt by the next full aggregation
8. `cd daemon && RUSTC_WRAPPER= cargo make lint` and `RUSTC_WRAPPER= cargo test` pass

### 2.2 Non-Goals

- Answering a Scope that reads the insides of archives, and `listEntries` itself. They are M28.
- Measuring the index with archives. [Listing and Search Design §8.1](../design/listing.md#count-the-entries-inside-archives-toward-the-name-indexs-targets) decides against it until a user meets a slow case.
- Changing the timeline index or how it rereads archives.
- Following external changes inside archives without aggregation, which stays deferred ([Listing and Search Design §8.2](../design/listing.md#whether-to-reflect-external-changes-in-listings-immediately)).

## 3. Current State

The name index holds `name_entries` and `name_folders` without any mark of archives ([name_index.rs#L45-L80](../../daemon/entrypoints/daemon/src/service/listing/name_index.rs#L45-L80)), at `FORMAT_VERSION` 1 ([name_index.rs#L25](../../daemon/entrypoints/daemon/src/service/listing/name_index.rs#L25)), and a file of another version is discarded at startup ([name_index.rs#L488-L503](../../daemon/entrypoints/daemon/src/service/listing/name_index.rs#L488-L503)).
Only an Entry of kind `Folder` gets a folder row and is treated as having contents: when it is inserted ([name_index.rs#L961-L967](../../daemon/entrypoints/daemon/src/service/listing/name_index.rs#L961-L967), [name_index.rs#L1079-L1085](../../daemon/entrypoints/daemon/src/service/listing/name_index.rs#L1079-L1085)), when its kind changes ([name_index.rs#L1093-L1113](../../daemon/entrypoints/daemon/src/service/listing/name_index.rs#L1093-L1113)), when it is deleted ([name_index.rs#L1252-L1263](../../daemon/entrypoints/daemon/src/service/listing/name_index.rs#L1252-L1263)), and when it is moved ([name_index.rs#L1186-L1250](../../daemon/entrypoints/daemon/src/service/listing/name_index.rs#L1186-L1250)); `ensure_folder` turns an Entry it meets on a path into a `Folder` ([name_index.rs#L1008-L1055](../../daemon/entrypoints/daemon/src/service/listing/name_index.rs#L1008-L1055)).
An archive is an Entry of kind `Archive` ([scanner.rs#L232](../../daemon/entrypoints/daemon/src/service/timeline/scanner.rs#L232)), so it has no folder row and no contents in the index.
Aggregation writes names only when it enumerates a local folder ([scanner.rs#L183-L186](../../daemon/entrypoints/daemon/src/service/timeline/scanner.rs#L183-L186)).
It reads an archive's entries only in `read_archive`, for the timeline, extracting images, videos, and nested archives by block and committing in batches ([scanner.rs#L341-L446](../../daemon/entrypoints/daemon/src/service/timeline/scanner.rs#L341-L446), [scanner.rs#L448-L457](../../daemon/entrypoints/daemon/src/service/timeline/scanner.rs#L448-L457)); it skips an archive whose `ArchiveIdentity` the timeline index already has ([scanner.rs#L270-L290](../../daemon/entrypoints/daemon/src/service/timeline/scanner.rs#L270-L290)), and records an archive that cannot be opened without inner Entries ([scanner.rs#L347-L358](../../daemon/entrypoints/daemon/src/service/timeline/scanner.rs#L347-L358)).
A full aggregation stamps what it saw, `keep` stamps every Entry under a folder ([name_index.rs#L746-L754](../../daemon/entrypoints/daemon/src/service/listing/name_index.rs#L746-L754), [name_index.rs#L1127-L1136](../../daemon/entrypoints/daemon/src/service/listing/name_index.rs#L1127-L1136)), and the sweep deletes the rest through `delete_entry` ([name_index.rs#L1138-L1156](../../daemon/entrypoints/daemon/src/service/listing/name_index.rs#L1138-L1156)).
The daemon's own deletes reach the index by path ([timeline.rs#L286-L299](../../daemon/entrypoints/daemon/src/service/timeline.rs#L286-L299)), and its renames call the index's `rename` before the timeline recomputes the `ArchiveIdentity` of each archive it moved, skipping an archive whose file it cannot read ([timeline.rs#L316-L331](../../daemon/entrypoints/daemon/src/service/timeline.rs#L316-L331)).
The queries of the index select every row under a start ([name_index.rs#L635-L710](../../daemon/entrypoints/daemon/src/service/listing/name_index.rs#L635-L710)).
A recursive walk enters only folders, so it already does not descend into archives ([listing.rs#L410-L411](../../daemon/entrypoints/daemon/src/service/listing.rs#L410-L411)).
The index therefore cannot answer the insides of archives that M28 needs.

## 4. Change Policy

An archive becomes an Entry that keeps its kind and has a folder row, like a folder, and every folder row records whether it is an archive or inside one.
A table records the `ArchiveIdentity` of each archive file whose insides the index holds, under a new format version, so an earlier file is discarded and rebuilt.
Aggregation collects an archive's names while it reads the archive and writes them with the record in one transaction after the timeline's rows, or, when the timeline skips the archive, reads the archive for its names alone if the record is missing or differs, and otherwise keeps what is under it.
The record goes wherever the rows go, and the queries leave out the rows under marked folders.

### 4.1 Decisions Assumed

| Decision | Source |
| --- | --- |
| The name index holds the insides of archives down to `max_nested_depth` | [Listing and Search Design §8.1](../design/listing.md#index-the-entries-inside-archives-too) |
| An archive keeps its kind and has a folder row, and the record of an archive goes with its rows | [Listing and Search Design §5.1](../design/listing.md#51-name-index) |
| An unopenable archive is an Entry without contents | [Listing and Search Design §8.1](../design/listing.md#hold-an-archive-that-cannot-be-opened-as-an-entry-without-contents) |
| An archive's rows are replaced by `ArchiveIdentity`, and the name index keeps its own record of it | [Listing and Search Design §8.1](../design/listing.md#replace-the-rows-under-an-archive-when-its-archiveidentity-changes) |
| Folder rows of archives are marked, and a listing that does not read archives leaves out what is under them | [Listing and Search Design §8.1](../design/listing.md#mark-the-folders-of-archives-in-the-name-index) |
| The insides count toward the targets, and the index is not measured now | [Listing and Search Design §8.1](../design/listing.md#count-the-entries-inside-archives-toward-the-name-indexs-targets) |
| A walk while the index is not usable does not descend into archives | [Listing and Search Design §8.1](../design/listing.md#walk-while-the-name-index-is-not-usable) |

### 4.2 Scope of Impact

`daemon/entrypoints/daemon/src/service/listing/name_index.rs`, `service/timeline/scanner.rs`, `service/timeline.rs`, their tests, Listing and Search Design §9, and design.md §12 change.
No API, configuration, or stored format other than the name index file changes, and listings return the same Entries as before.
The name index file changes format, so the first startup discards it and recursive listings with conditions walk until the next full aggregation finishes; the daemon is not released, so this is accepted.

## 5. Steps

| No. | Content | Depends on | Status |
| --- | --- | --- | --- |
| [1](#s-1) | Give archives folder rows, the mark, and the record | - | Done |
| [2](#s-2) | Write the insides of archives from aggregation | 1 | Done |
| [3](#s-3) | Keep the insides and the records with every removal and move | 2 | Done |
| [4](#s-4) | Leave the insides out of the queries | 2 | Done |
| [5](#s-5) | Describe the state | 3, 4 | Not started |

<a id="s-1"></a>
### Step 1. Give Archives Folder Rows, the Mark, and the Record

**Locations to change**
[name_index.rs#L25](../../daemon/entrypoints/daemon/src/service/listing/name_index.rs#L25), [name_index.rs#L45-L80](../../daemon/entrypoints/daemon/src/service/listing/name_index.rs#L45-L80), [name_index.rs#L488-L503](../../daemon/entrypoints/daemon/src/service/listing/name_index.rs#L488-L503), and the folder rows written at [name_index.rs#L961-L967](../../daemon/entrypoints/daemon/src/service/listing/name_index.rs#L961-L967), [name_index.rs#L1008-L1055](../../daemon/entrypoints/daemon/src/service/listing/name_index.rs#L1008-L1055), [name_index.rs#L1079-L1085](../../daemon/entrypoints/daemon/src/service/listing/name_index.rs#L1079-L1085), and [name_index.rs#L1093-L1113](../../daemon/entrypoints/daemon/src/service/listing/name_index.rs#L1093-L1113).

**What to do**
Raise `FORMAT_VERSION` to 2, add to `name_folders` an integer column `archive` that is 1 for a folder row that is an archive or under one, and add a table `name_archives (library_id, path, identity)` keyed by the Library and the path, and read a row of it in `prepare` as the other tables are.
Give a folder row to an Entry of kind `Archive` wherever one is given to a `Folder`, with `archive` set, and set `archive` on every folder row whose parent folder row has it.
Make `ensure_folder` use an `Archive` Entry it meets on a path as the folder it is, without changing its kind.
Add the operations that aggregation and the daemon's changes use, queued through the writer as the other writes are:

```rust
/// Replaces everything the index holds under the archive at `path` with `folders`, each a folder path under the archive
/// with its direct children, and sets the archive's record to `identity`, in one transaction.
/// The archive's own folder row exists afterwards even when `folders` is empty, as for an archive that cannot be opened.
pub async fn replace_archive(&self, library: &str, path: &str, identity: &str, folders: &[(String, Vec<NameRow>)], generation: i64);
/// The `ArchiveIdentity` the index holds the insides of the archive at `path` for, if any.
pub async fn archive_identity(&self, library: &str, path: &str) -> Option<String>;
/// Updates the record of the archive at `path` to `identity` when a record exists at that path, as after a rename,
/// and does nothing when none exists, so that an archive whose insides the index does not hold is still read.
pub async fn set_archive_identity(&self, library: &str, path: &str, identity: &str);
/// Removes the record of the archive at `path`, leaving its rows, so the next aggregation reads it again.
pub async fn forget_archive(&self, library: &str, path: &str);
```

**Completion condition**
Tests show a file of format 1 discarded and a new file created at format 2, an `Archive` Entry with a folder row that keeps its kind, `archive` set on the folder rows of an archive and of what is under it and on no other, `replace_archive` replacing the rows under an archive and setting its record, and the record read, set, and forgotten.

<a id="s-2"></a>
### Step 2. Write the Insides of Archives from Aggregation

**Locations to change**
[scanner.rs#L270-L290](../../daemon/entrypoints/daemon/src/service/timeline/scanner.rs#L270-L290), [scanner.rs#L341-L446](../../daemon/entrypoints/daemon/src/service/timeline/scanner.rs#L341-L446), [scanner.rs#L448-L457](../../daemon/entrypoints/daemon/src/service/timeline/scanner.rs#L448-L457), [name_index.rs#L746-L754](../../daemon/entrypoints/daemon/src/service/listing/name_index.rs#L746-L754), and a test Library in the scanner's tests.

**What to do**
In `read_archive`, collect the non-hidden folders and Entries of every archive it opens, nested archives down to `max_nested_depth` included, by their joined paths, deriving the folders from the paths of the entries.
After the last `commit_archive` of the archive returns `true`, call `replace_archive` with them and the archive's identity; when any `commit_archive` returns `false`, write nothing to the name index.
For an archive that cannot be opened, call `replace_archive` with no folders, so that the archive is an Entry without contents and is not read again until it changes.
When the timeline skips an archive, compare the identity with `archive_identity`: when it is missing or differs, read the archive for the name index alone, which is `read_archive`'s walk extracting only the nested archives whose depth is under `max_nested_depth`, as `read_archive` decides with `open.depth < max_depth`, and no image or video, no timeline row, and no `commit_archive`, and then call `replace_archive`; when it matches, call `keep` on the archive's path, which stamps every Entry under it.
Count, for the tests, the archives read for the timeline and the archives read for the name index alone in each aggregation.
Add a test Library with the archives of condition 1.

**Completion condition**
Tests show conditions 1, 2, and 3: the held rows and marks of condition 1, the two counts at zero and the rows kept for the second aggregation of condition 2 with one archive read after its file is replaced, and for condition 3 the count of the name index alone equal to the number of archive files with the timeline's count at zero.

<a id="s-3"></a>
### Step 3. Keep the Insides and the Records with Every Removal and Move

**Locations to change**
[name_index.rs#L1093-L1113](../../daemon/entrypoints/daemon/src/service/listing/name_index.rs#L1093-L1113), [name_index.rs#L1138-L1156](../../daemon/entrypoints/daemon/src/service/listing/name_index.rs#L1138-L1156), [name_index.rs#L1186-L1278](../../daemon/entrypoints/daemon/src/service/listing/name_index.rs#L1186-L1278), and [timeline.rs#L316-L331](../../daemon/entrypoints/daemon/src/service/timeline.rs#L316-L331).

**What to do**
Make `delete_entry`, `delete_inside`, and `change_kind` treat an `Archive` Entry as having contents, as a `Folder`, and remove the records of `name_archives` at or under the path in the same transaction, so a sweep, a delete, a change of kind, and a move to a hidden name remove them.
Make `move_path` move the folder rows of an `Archive` as it moves a `Folder`'s, and move the records at or under the source path to the destination.
In `renamed`, after `self.index.set_archive_identity`, call `self.names.set_archive_identity` with the same recomputed identity for each archive the timeline moved, and call `self.names.forget_archive` for an archive whose file it cannot read, so that the next aggregation reads it again.

**Completion condition**
Tests show conditions 4 and 5, and that renaming an archive the index holds no record of creates no record.

<a id="s-4"></a>
### Step 4. Leave the Insides Out of the Queries

**Locations to change**
[name_index.rs#L611-L710](../../daemon/entrypoints/daemon/src/service/listing/name_index.rs#L611-L710).

**What to do**
Make the search and the lookup leave out every Entry whose folder row has `archive` set, with the condition built in one place that M28 can turn off for a Scope that reads the insides of archives.

**Completion condition**
Tests over the Library of step 2 show condition 6, and the existing listing and search tests pass.

<a id="s-5"></a>
### Step 5. Describe the State

**Locations to change**
[Listing and Search Design §9.1](../design/listing.md#91-current-code), [§9.2](../design/listing.md#92-roadmap), [design.md §12.1](../design.md#121-implementation-status), and [§12.2](../design.md#122-roadmap).

**What to do**
Describe in Listing and Search Design §9.1 that the index holds the insides of archives and that the queries leave them out, and that the index's size with archives is unmeasured.
Mark L3 and M27 done, and remove "a name index without the insides of archives" from the gap in design.md §12.1.

**Completion condition**
§9.1 describes the insides and their exclusion, L3 of §9.2 and M27 of design.md §12.2 are done, and §12.1 no longer names the name index in the gap.

## 6. Acceptance Checks

| Condition | Command | Expected result |
| --- | --- | --- |
| 1, 2, 3 | `cd daemon && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon timeline::scanner` | The tests of step 2 pass: the rows and marks of condition 1, zero archives read in the second aggregation and one after a replaced file, and only name-index reads after the name index is rebuilt |
| 4, 5 | `cd daemon && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon name_index && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon timeline::` | The tests of step 3 pass: no row or record left after a delete, a sweep, a change of kind, or a hidden move, the rows and records at the destination after a rename with zero archives read by the next aggregation, and no record created by renaming an archive without one |
| 6 | `cd daemon && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon library::indexed && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon listing` | The tests of step 4 pass, with no Entry inside an archive in a recursive listing answered from the index, and the existing tests pass |
| 7 | `cd daemon && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon name_index` | The test of a format 1 file discarded and rebuilt passes |
| 8 | `cd daemon && RUSTC_WRAPPER= cargo make lint && RUSTC_WRAPPER= cargo test` | Both exit 0 |

The new tests protect the following.

- The format: an index of format 1 is discarded, and the mark and the record exist at format 2
- The archive's folder row: an `Archive` Entry keeps its kind and holds its insides
- The insides from aggregation: what is held inside each kind of archive, the unopenable archive, the nested depth, and the hidden folder
- Skipping: no archive read again without a change, the rows kept, and the rows of a replaced archive replaced
- The rebuilt index: the insides filled by reading for names alone, without the timeline's reads
- Removals and moves: the rows and records removed by every removal and moved by renames
- The queries: no Entry inside an archive in a recursive listing answered from the index
