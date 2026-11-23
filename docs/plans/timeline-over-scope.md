# Read a Scope in Timeline-Date Order and Count Its Months

## 1. About This Plan

This work is the second half of M28 of the roadmap: `listEntries` reads a Scope through a Filter in timeline-date order by cursor, month, or around an Entry, and `GET /entries/months` returns the per-month counts over the same Scope and Filter, from counts held per kind and per inside-archive.
It completes L4 of Listing and Search Design and TL1 and TL2 of Timeline Design, and with them M28.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The steps and acceptance checks of timeline-date order and the per-month counts over a Scope |
| [Listing and Search Design §3](../design/listing.md#3-sorting-and-filtering), [§4.6](../design/listing.md#46-pages-in-timeline-date-order) | The sort value, the cursors, `month`, `around`, and what the response carries in this order |
| [Timeline Design §5](../design/timeline.md#5-the-timeline-index), [§7](../design/timeline.md#7-reading) | The rows, the per-month counts, how each kind of start is read, conditions, and the counts over a Scope |
| [Timeline Design §9.1](../design/timeline.md#91-decided) | The decisions this plan follows |
| [openapi.yaml](../../daemon/openapi.yaml#L675-L811) | The contract this plan extends, from `listEntries` on |
| [design.md §12.2](../design.md#122-roadmap) | M28 and what follows it |

[design.md](../design.md#11-division-of-responsibility-between-documents) holds the map of the document family.

### 1.2 Creation Date and Target Commit

Created 2026-11-23 / Target commit `b5250db`, the merge of `list-entries`; the decisions of §4.1 are written in the design documents in the same change as this document

## 2. Purpose and Completion Conditions

A client can show any Scope and Filter as a timeline, newest first, jump to a month or to an Entry, read the continuation in both directions, and list the months with their counts, for Library roots at the cost of the counts held in advance and for folders and conditions at the cost of what they hold.

### 2.1 Completion Conditions

1. `listEntries` with `sort=timeline_date` over the roots of two Libraries returns their images and videos newest first, Entries of the same timeline date ordered by Library id and then by path, each with `timelineDate` and `timelineDateSource`, with `previous` and `next` and without `totalCount`, `offset`, and `snapshot`; pages read by `after` from `next` and by `before` from `previous` join without a repeat or a gap; `month` starts at the newest Entry of that month or of the next older month that has Entries; `around` returns a page with the named Entry near its middle, and an Entry outside the sequence is `NotFound`
2. A folder start returns only the rows under its path, and without `recursive` only its direct children; without `insideArchives` no Entry inside an archive under a folder start is returned; a start that is an archive or a path inside one returns every row under it, the Entries of nested archives included, with or without `insideArchives`; covered starts add nothing, and missing starts are named
3. Without `kind` both images and videos are returned, with `kind` only the named ones, and a kind other than `image` and `video` is `InvalidInput`
4. With a name, rating, or tag condition, the matches of each start are returned in timeline-date order, an archive start's matches including the Entries of nested archives only with `insideArchives`, a match without a timeline row is not returned, each Entry has the attributes its snapshot read, and the starts cut off and walked are named; pages joined by the cursors neither repeat nor skip a match
5. `GET /entries/months` takes the Scope and Filter parameters of `listEntries`, and its count for every month equals the number of Entries that `listEntries` in timeline-date order returns for that month with the same parameters, for Library roots, for folder and archive starts, with and without `insideArchives`, with each kind, and with conditions; a kind other than `image` and `video` is `InvalidInput`
6. An index whose per-month counts were held per Library and month is migrated at startup to counts per Library, month, kind, and inside-archive that equal the rows counted that way, and the counts stay equal to the rows after rows are added, removed, and re-dated; `listTimelineMonths` returns what it returned before the migration
7. `order=asc`, `offset`, `locate`, or `foldersFirst` with `sort=timeline_date`, any of `after`, `before`, `month`, and `around` with another sort, more than one of them, and a malformed cursor, month, or `around` are `InvalidInput`; the folder listing APIs still reject the timeline date as a sort
8. `listFolderEntries`, `listSearchEntries`, `listTimelineEntries`, `listTimelineMonths`, and `listEntries` in the orders cut from snapshots answer as before
9. `cd daemon && RUSTC_WRAPPER= cargo make lint` and `RUSTC_WRAPPER= cargo test` pass, and the committed interface crate and web client API types are what the contract generates

### 2.2 Non-Goals

- Removing `listSearchEntries`, `listTimelineEntries`, and `listTimelineMonths`. M31 removes them once no client calls them ([design.md §12.2](../design.md#122-roadmap)).
- Reading `listEntries` and `listEntryMonths` from the web client or the iOS app. They are M30 and the iOS app's I6.
- Making timeline-date order fast over large folders and conditions, which stays deferred ([Timeline Design §9.2](../design/timeline.md#whether-timeline-date-order-stays-fast-over-folders-and-conditions)).

## 3. Current State

The timeline index holds a row per Entry keyed by Library and path, with its local file, kind, and timeline date, a date index across Libraries, and per-month counts keyed by Library and month only ([index.rs#L296-L347](../../daemon/entrypoints/daemon/src/service/timeline/index.rs#L296-L347)); triggers created after the migrations keep the counts in step with the rows ([index.rs#L24-L46](../../daemon/entrypoints/daemon/src/service/timeline/index.rs#L24-L46), [index.rs#L294-L356](../../daemon/entrypoints/daemon/src/service/timeline/index.rs#L294-L356)).
Pages are read over a set of whole Libraries only, by `TimelineAnchor` and `TimelineCursor` ([index.rs#L162-L210](../../daemon/entrypoints/daemon/src/service/timeline/index.rs#L162-L210), [index.rs#L247-L280](../../daemon/entrypoints/daemon/src/service/timeline/index.rs#L247-L280), [index.rs#L772-L809](../../daemon/entrypoints/daemon/src/service/timeline/index.rs#L772-L809), [index.rs#L837-L933](../../daemon/entrypoints/daemon/src/service/timeline/index.rs#L837-L933)), the months are summed over whole Libraries ([index.rs#L811-L824](../../daemon/entrypoints/daemon/src/service/timeline/index.rs#L811-L824)), and `PathRange` already expresses a path and everything under it ([index.rs#L220-L245](../../daemon/entrypoints/daemon/src/service/timeline/index.rs#L220-L245)).
A row is written by an upsert that can change its local file and kind under the same path ([index.rs#L505-L515](../../daemon/entrypoints/daemon/src/service/timeline/index.rs#L505-L515)), while the update trigger fires only on a change of Library or month ([index.rs#L37-L44](../../daemon/entrypoints/daemon/src/service/timeline/index.rs#L37-L44)).
The timeline route parses the anchors and builds a `FileSystemEntry` from a row in the interface layer ([timeline.rs#L123-L166](../../daemon/entrypoints/daemon/src/interface/features/timeline.rs#L123-L166)), and `TimelineService` keeps its index private and exposes the index's pages and months by Library ([service/timeline.rs#L80-L89](../../daemon/entrypoints/daemon/src/service/timeline.rs#L80-L89), [service/timeline.rs#L235-L246](../../daemon/entrypoints/daemon/src/service/timeline.rs#L235-L246)); `DaemonState` holds the `LibraryBrowser` and the `TimelineService` side by side ([state.rs#L30-L50](../../daemon/entrypoints/daemon/src/state.rs#L30-L50)).

`listEntries` takes the shared `SortQuery` of the folder listings, without the timeline date, and has no cursor, month, or around parameter ([openapi.yaml#L675-L811](../../daemon/openapi.yaml#L675-L811), [openapi.yaml#L2126-L2138](../../daemon/openapi.yaml#L2126-L2138)); `ScopedEntry` has only `libraryId` and `entry`, and `EntryListing` has no cursors ([openapi.yaml#L3135-L3193](../../daemon/openapi.yaml#L3135-L3193)).
`ListOptions::from_entries` parses the sort with the folder listings' `ListSort::parse` ([listing.rs#L98-L117](../../daemon/entrypoints/daemon/src/service/listing.rs#L98-L117), [listing.rs#L687-L701](../../daemon/entrypoints/daemon/src/service/listing.rs#L687-L701)).
`LibraryBrowser` resolves a Scope's starts, drops the covered ones, and builds a snapshot per start ([library.rs#L134-L155](../../daemon/entrypoints/daemon/src/service/library.rs#L134-L155), [library.rs#L396-L400](../../daemon/entrypoints/daemon/src/service/library.rs#L396-L400)), and a snapshot keeps each order it is asked for ([snapshot.rs#L122-L156](../../daemon/entrypoints/daemon/src/service/listing/snapshot.rs#L122-L156)); `ScopeResults` cuts pages by `offset` only ([scope_results.rs#L1-L80](../../daemon/entrypoints/daemon/src/service/listing/scope_results.rs#L1-L80)).

There is therefore no way to read the timeline of a folder, of an archive, or of a Filter, nor to count its months, and the counts cannot tell kinds or the insides of archives apart.

## 4. Change Policy

The per-month counts gain the kind and whether the Entry is inside an archive, by a migration that recounts the rows.
The timeline index learns to read and count over starts: Library roots together through the date index, and each folder or archive start through its primary-key range, with the kinds, `recursive`, and `insideArchives` applied, merged newest first.
A start with a name, rating, or tag condition is read instead from the matches of its snapshot, put in timeline-date order once from their rows and kept in the snapshot.
A new `ScopeTimeline` joins the two: it resolves the Scope with `LibraryBrowser`, reads the rows or the snapshots, merges them, and builds the page and the months; the routes of `listEntries` and the new `listEntryMonths` call it for the timeline date.

### 4.1 Decisions Assumed

| Decision | Source |
| --- | --- |
| `list-entries` is done | [list-entries](./list-entries.md) |
| The sort value `timeline_date`, descending only, in a sort parameter of `listEntries` alone, with `after`, `before`, `month`, and `around` | [Listing and Search Design §3](../design/listing.md#3-sorting-and-filtering), [§4.6](../design/listing.md#46-pages-in-timeline-date-order), [§8.1](../design/listing.md#cut-pages-by-offset-and-by-cursor-in-timeline-date-order) |
| One response schema, with `previous` and `next` and the timeline fields of `ScopedEntry` in this order | [Listing and Search Design §8.1](../design/listing.md#answer-both-kinds-of-pages-with-one-response-schema) |
| Library roots by the date order, folder starts by their paths, direct children and the insides of archives by the rows' paths | [Timeline Design §7](../design/timeline.md#7-reading), [§9.1](../design/timeline.md#read-a-library-root-by-the-date-order-and-a-folder-start-by-its-path), [§9.1](../design/timeline.md#leave-out-the-insides-of-archives-by-the-entrys-local-file) |
| An archive start reads every row under it | [Timeline Design §9.1](../design/timeline.md#read-every-row-under-an-archive-start) |
| Conditions over the matches of each start, ordered once per snapshot, with the snapshot's attributes | [Timeline Design §9.1](../design/timeline.md#apply-name-rating-and-tag-conditions-over-the-matches-of-the-name-index), [§9.1](../design/timeline.md#order-the-matches-of-a-condition-once-per-snapshot), [§9.1](../design/timeline.md#return-the-attributes-the-snapshot-read-for-the-matches-of-a-condition) |
| Counts per Library, month, kind, and inside-archive, migrated by recounting rows, summed for whole Libraries and counted otherwise | [Timeline Design §9.1](../design/timeline.md#sum-the-per-month-counts-in-advance-only-for-whole-libraries), [§9.1](../design/timeline.md#recount-the-per-month-counts-from-the-rows-when-their-columns-change) |
| `listEntryMonths` at `GET /entries/months` returns `TimelineMonths` alone | [Listing and Search Design §8.1](../design/listing.md#serve-the-listing-at-entries-and-its-per-month-counts-at-entriesmonths), [Timeline Design §9.1](../design/timeline.md#return-only-the-months-from-the-per-month-counts) |

### 4.2 Scope of Impact

[openapi.yaml](../../daemon/openapi.yaml#L675-L811) gains a sort parameter of `listEntries`, its anchors, the timeline fields of `ScopedEntry`, the cursors of `EntryListing`, and `GET /entries/months`, so the generated interface crate and `ui-web/src/api/schema.d.ts` change; `service/timeline/index.rs`, `service/timeline.rs`, `service/listing.rs`, `listing/snapshot.rs`, `service/library.rs`, a new `service/timeline/scope.rs`, `interface/features/entries.rs`, `interface/features/timeline.rs`, their tests, Listing and Search Design §9, Timeline Design §10, and design.md §12 change.
The timeline index's stored format changes by a migration at startup that cannot be undone by `git revert` (§7); the existing APIs and the configuration do not change, and compatibility does not break.

## 5. Steps

| No. | Content | Depends on | Status |
| --- | --- | --- | --- |
| [1](#s-1) | Hold the per-month counts per kind and inside-archive | - | Done |
| [2](#s-2) | Read rows over starts in timeline-date order | - | Not started |
| [3](#s-3) | Count the months over starts | 1, 2 | Not started |
| [4](#s-4) | Keep the timeline-date order of a snapshot's matches | - | Not started |
| [5](#s-5) | Read a Scope in timeline-date order | 3, 4 | Not started |
| [6](#s-6) | Serve timeline-date order and `listEntryMonths` | 5 | Not started |
| [7](#s-7) | Describe the state | 6 | Not started |

<a id="s-1"></a>
### Step 1. Hold the Per-Month Counts per Kind and Inside-Archive

**Locations to change**
[index.rs#L24-L46](../../daemon/entrypoints/daemon/src/service/timeline/index.rs#L24-L46), [index.rs#L294-L356](../../daemon/entrypoints/daemon/src/service/timeline/index.rs#L294-L356), and [index.rs#L811-L824](../../daemon/entrypoints/daemon/src/service/timeline/index.rs#L811-L824).

**What to do**
Add a migration, as one more `MigrationRequest` after the existing two, whose statements contain no `;` inside them, since the migrator splits on it: `DROP TRIGGER IF EXISTS` for each of the three triggers, so that a new file, which has none yet, migrates too; create a table under a new name keyed by Library, month, kind, and `inside_archive` (1 when the row's path differs from its local file's); fill it by counting the existing rows grouped that way; drop the old table; and rename the new one to `timeline_months`.
Rewrite the triggers in `TRIGGERS` for the new key; they are created after the migrations with `IF NOT EXISTS`, so the new definitions replace the dropped ones at that startup and are kept afterwards.
The insert and delete triggers move the count of the row's month, kind, and inside-archive; the update trigger fires on `library_id`, `path`, `file_path`, `kind`, and `date`, and moves a count whenever the Library, the month, the kind, or the inside-archive value differs between the old and the new row, since the upsert of a row can change its local file and kind under the same path.
Make `months` sum the new counts over every kind and both values of inside-archive, so `listTimelineMonths` answers as before.

**Completion condition**
Tests show an index created with the old migrations and rows migrated to counts equal to the rows grouped by Library, month, kind, and inside-archive, a new index created with every migration, the counts following inserts, deletes, re-dating, and upserts that change only the kind or only the local file, and `months` returning the same sums before and after the migration.

<a id="s-2"></a>
### Step 2. Read Rows over Starts in Timeline-Date Order

**Locations to change**
[index.rs#L162-L280](../../daemon/entrypoints/daemon/src/service/timeline/index.rs#L162-L280) and [index.rs#L772-L933](../../daemon/entrypoints/daemon/src/service/timeline/index.rs#L772-L933).

**What to do**
Add a reading of the index over starts, each a Library with its root or a path in it, and for a path whether it is an archive or inside one, with `recursive`, `insideArchives`, and the kinds of images and videos to keep, by the anchors of `TimelineAnchor`.
Read the root starts together, as `page` reads Libraries now, and each path start by its `PathRange` on the primary key, ordered by the date order, taking `limit` rows and one more to know whether more follow; without `recursive` keep only the rows whose path is a direct child of the start's path; without `insideArchives` keep only the rows whose path is their local file's, except under a start that is an archive or inside one, which keeps every row under it.
Merge the starts newest first with ties by Library id and path and cut the page at `limit`; for `before`, each start reads the `limit` rows nearest the cursor, and the merge keeps the `limit` nearest of them.
`previous` is the cursor of the page's first row when any start has a row newer than it, and `next` the cursor of its last row when any start has an older one, each asked of every start as `page` asks its Libraries.
For `around`, find the named row among the rows the starts read, with their conditions applied, and answer `NotFound` when it is not among them; for `month`, start at the end of that month as `page` does.
Keep `page` answering `listTimelineEntries` as now.
Nothing calls this reading until step 5, so it carries a dead-code expectation that step 5 removes, as do the additions of steps 3 and 4.

**Completion condition**
Tests over two Libraries with folders, archives, a nested archive, images, and videos show the order and ties, every anchor, cursors joining without a repeat or a gap, folder and archive starts, direct children, the insides with and without `insideArchives`, the kinds, and `around` outside the starts.

<a id="s-3"></a>
### Step 3. Count the Months over Starts

**Locations to change**
[index.rs#L811-L824](../../daemon/entrypoints/daemon/src/service/timeline/index.rs#L811-L824).

**What to do**
Add a count of the months over the starts of step 2: when every start is a root and the reading is recursive, sum the counts of step 1 over the kinds kept, and over inside-archive 0 alone without `insideArchives`; otherwise count the rows that the same reading would return, grouped by `substr(date, 1, 7)`, newest month first, leaving out months without Entries.

**Completion condition**
Tests show, for root starts, folder starts, archive starts, direct children, each kind, and with and without `insideArchives`, counts equal to the rows that step 2 returns for each month.

<a id="s-4"></a>
### Step 4. Keep the Timeline-Date Order of a Snapshot's Matches

**Locations to change**
[snapshot.rs#L122-L156](../../daemon/entrypoints/daemon/src/service/listing/snapshot.rs#L122-L156) and [index.rs#L837-L842](../../daemon/entrypoints/daemon/src/service/timeline/index.rs#L837-L842).

**What to do**
Add to the timeline index a lookup of the rows of many paths of one Library by the primary key, in batches.
Add to `ListingSnapshot` a timeline-date order that is built at the first request for it from the snapshot's Entries that match a listing's options, with the timeline date and its source of each match's row, matches without a row left out, newest first with ties by path.
Key it by what decides the matches: the name terms, the rating and tag condition with the version of ratings and tags its selection was read at, the kinds, and the insides choice, as `IndexKey` and the kinds are; a snapshot keeps one such order, and a request with another key replaces it, so a write of ratings and tags or retyped terms builds the order again without holding older ones.
Give it a cut by the anchors of `TimelineAnchor`, `around` finding the named Entry among the order's matches, and a count by month, so that a start with a condition can be paged and counted from it.

**Completion condition**
Tests show the order built once and reused, a match without a row left out, the Entry of each match being the snapshot's, and the cut and the count by month over it.

<a id="s-5"></a>
### Step 5. Read a Scope in Timeline-Date Order

**Locations to change**
A new module `service/timeline/scope.rs` declared beside the others at [service/timeline.rs#L19-L27](../../daemon/entrypoints/daemon/src/service/timeline.rs#L19-L27), [service/timeline.rs#L235-L246](../../daemon/entrypoints/daemon/src/service/timeline.rs#L235-L246), [library.rs#L134-L155](../../daemon/entrypoints/daemon/src/service/library.rs#L134-L155), [scope_results.rs#L1-L80](../../daemon/entrypoints/daemon/src/service/listing/scope_results.rs#L1-L80), and [timeline.rs#L156-L166](../../daemon/entrypoints/daemon/src/interface/features/timeline.rs#L156-L166).

**What to do**
Add to `TimelineService` the methods that `ScopeTimeline` reads the index through, beside `page` and `months`: the reading of step 2, the count of step 3, and the lookup of step 4, each checking the Libraries as `page` does.
Move the building of a `FileSystemEntry` from a row out of the timeline route into the service layer, as a method of `TimelineListedRow`, and have the timeline route call it, so that both routes build the same Entry.
Add `ScopeTimeline`, which borrows the `LibraryBrowser` and the `TimelineService` of `DaemonState`.
It resolves the Scope with `resolve_scope`, so the covered starts are dropped and the missing ones named, and rejects a Filter whose kinds are not only images and videos with `InvalidInput`.
Without a name, rating, or tag condition, it reads the rows of the resolved starts by step 2 and counts by step 3, building each Entry from its row as the timeline route does now, and names no start cut off or walked.
With one, it builds each start's snapshot as `list_entries` does, with the rating and tag selection of its Library, pages and counts each from the order of step 4, merges them as in step 2, and names the starts cut off and walked as `ScopeResults` does.
It returns a page of Entries with their Library ids, timeline dates, and sources, `previous`, `next`, and the three lists of starts, and the months as `(month, count)` pairs.
Remove the dead-code expectations of steps 2 to 4.

**Completion condition**
Tests show conditions 1 to 5 over the service: rows and conditions, the anchors, the kinds, missing, covered, cut-off, and walked starts, and the months equal to the pages for each case.

<a id="s-6"></a>
### Step 6. Serve Timeline-Date Order and `listEntryMonths`

**Locations to change**
[openapi.yaml#L675-L811](../../daemon/openapi.yaml#L675-L811), [openapi.yaml#L2982-L3005](../../daemon/openapi.yaml#L2982-L3005), [openapi.yaml#L3135-L3193](../../daemon/openapi.yaml#L3135-L3193), [listing.rs#L98-L117](../../daemon/entrypoints/daemon/src/service/listing.rs#L98-L117), [listing.rs#L687-L701](../../daemon/entrypoints/daemon/src/service/listing.rs#L687-L701), and [entries.rs#L19-L101](../../daemon/entrypoints/daemon/src/interface/features/entries.rs#L19-L101); the generated interface crate under `daemon/entrypoints/interface` and the generated `ui-web/src/api/schema.d.ts` are written by `daemon/entrypoints/gen-interface.sh` and `npm run gen:api`.

**What to do**
Give `listEntries` a sort parameter of its own with `name`, `kind`, `size`, `modified_at`, and `timeline_date`, leaving `SortQuery` of the folder listings as it is, and the query parameters `after` and `before` (cursors), `month` (`YYYY-MM`), and `around` (an Entry in the form of a start).
Add to `ScopedEntry` the optional `timelineDate` and `timelineDateSource`, with the values of `TimelineEntry`, and to `EntryListing` the optional `previous` and `next`.
Add `GET /entries/months` with operation `listEntryMonths` under the tag `Entries`, taking `start`, `recursive`, `insideArchives`, `query`, `rating`, `tag`, and `kind` as `listEntries` does, and returning `TimelineMonths`.
Regenerate the interface crate and the web client's API types.
Let `ListOptions::from_entries` accept `timeline_date` and record that the listing is in timeline-date order, keeping the name order as its `ListSort`, which the snapshots of a start with a condition are built and matched with; `ListSort::parse` keeps rejecting it for the folder listings.
In the route of `listEntries`, call `ScopeTimeline` for `timeline_date` and the existing path otherwise, and answer `InvalidInput` for the cases of condition 7; implement the route of `listEntryMonths` over `ScopeTimeline`.

**Completion condition**
Route tests over two Libraries show conditions 1 to 8 through HTTP, the existing route tests pass, and regenerating the interface crate and the web client's API types leaves no difference.

<a id="s-7"></a>
### Step 7. Describe the State

**Locations to change**
[Listing and Search Design §9.1](../design/listing.md#91-current-code), [§9.2](../design/listing.md#92-roadmap), [Timeline Design §10.1](../design/timeline.md#101-current-code), [§10.2](../design/timeline.md#102-roadmap), [design.md §12.1](../design.md#121-implementation-status), and [§12.2](../design.md#122-roadmap).

**What to do**
Describe in Listing and Search Design §9.1 and Timeline Design §10.1 that `listEntries` reads every order, that the months over a Scope are served, and that the counts are held per kind and inside-archive.
Mark L4, TL1, TL2, and M28 done, and leave in the gap of design.md §12.1 only what M29 to M31 close.

**Completion condition**
§9.1 and §10.1 describe timeline-date order and the months over a Scope as implemented, L4, TL1, TL2, and M28 are done, and §12.1 no longer names timeline-date order or the months.

## 6. Acceptance Checks

| Condition | Command | Expected result |
| --- | --- | --- |
| 1 to 5, 7 | `cd daemon && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon interface::features::entries` | The route tests of step 6 pass: the order, ties, cursors, `month`, and `around` over two Libraries, folder, archive, and direct-children starts with and without `insideArchives`, the kinds, conditions with their cut-off and walked starts, the months equal to the pages, and every `InvalidInput` case |
| 1 to 5 | `cd daemon && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon service::timeline` | The tests of steps 2, 3, and 5 pass |
| 4 | `cd daemon && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon service::listing` | The tests of step 4 pass: the order of the matches built once, matches without a row left out, and the snapshot's Entries |
| 6 | `cd daemon && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon timeline::index` | The tests of step 1 pass: the migrated counts equal to the rows, kept by the triggers, and `months` unchanged |
| 7, 8 | `cd daemon && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon interface::features` | The route tests of the folder listings, search, the timeline, and `listEntries` in the orders cut from snapshots pass without change |
| 9 | `cd daemon && RUSTC_WRAPPER= cargo make lint && RUSTC_WRAPPER= cargo test` | Both exit 0 |
| 9 | `bash daemon/entrypoints/gen-interface.sh && git diff --exit-code -- daemon/entrypoints/interface` | Exits 0: the committed interface crate is what the contract generates, as the CI job `interface` checks |
| 9 | `cd ui-web && npm run gen:api && git diff --exit-code -- src/api` | Exits 0: the committed API types are what the contract generates, as the CI job `ui` checks |

The new tests protect the following.

- The counts: the migration, the triggers by kind and inside-archive, and the old sums
- The rows over starts: order, ties, anchors, cursors, folder, archive, and direct-children starts, the insides, and the kinds
- The months over starts: equal to the pages in every way of reading
- The snapshot's order: built once from the rows of the matches, with the snapshot's Entries
- The Scope in timeline-date order: rows and conditions together with the lists of starts
- The routes: the new parameters and their `InvalidInput` cases, and the old APIs unchanged

## 7. Risks and Rollback

The migration of step 1 rewrites the per-month counts of an existing timeline index, and `git revert` does not undo it: the earlier code would find a table of another shape.
If the migration fails or the counts are wrong, delete `state_dir/timeline/index.db`, as Timeline Design §5 allows for a broken invariant, and the next aggregation rebuilds the index, by which the earlier code also runs again; there is no way back to the old table without that rebuild.
