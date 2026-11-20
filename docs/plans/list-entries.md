# List the Entries of a Scope Through `listEntries`

## 1. About This Plan

This work is the first half of M28 of the roadmap: `GET /entries` lists the Entries of a Scope through a Filter as one sequence in every order cut from snapshots, with `locate` and the notices of the starts cut off, walked, and missing.
Timeline-date order and the per-month counts over a Scope are the second half, which a later plan carries out on top of this one.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The steps and acceptance checks of `listEntries` in the orders cut from snapshots |
| [Listing and Search Design §3](../design/listing.md#3-sorting-and-filtering) | The request of `listEntries`, its parameters, and the limit on starts |
| [Listing and Search Design §4](../design/listing.md#4-snapshots) | Snapshots, pages, the response, and `locate` |
| [Listing and Search Design §5](../design/listing.md#5-name-search), [§5.2](../design/listing.md#52-answering-recursive-listings-with-conditions), [§5.3](../design/listing.md#53-listing-several-starts) | The Scope and the Filter, when the name index answers, archive starts, overlapping and missing starts |
| [Listing and Search Design §8.1](../design/listing.md#81-decided) | The decisions this plan follows |
| [openapi.yaml](../../daemon/openapi.yaml) | The contract this plan adds `listEntries` to |
| [design.md §12.2](../design.md#122-roadmap) | M28 and what follows it |

[design.md](../design.md#11-division-of-responsibility-between-documents) holds the map of the document family.

### 1.2 Creation Date and Target Commit

Created 2026-11-20 / Target commit `04b773b`, the merge of `index-archive-insides`; the decisions of §4.1 are written in the design documents in the same change as this document

## 2. Purpose and Completion Conditions

A client can read any Scope, of one or more starts in one or more Libraries, through a Filter of name, rating, tags, and kind, with or without the insides of archives, as one sequence in name, kind, size, or modification order, page by page or at a named Entry, and learns from the response which starts were cut off, walked, or missing.

### 2.1 Completion Conditions

1. `GET /entries` with starts in two Libraries returns one sequence in the requested sort, with `totalCount`, `offset`, and `snapshot`, each Entry with its Library id; pages of one `limit` joined in `offset` order neither repeat nor skip an Entry, and Entries with the same path in the two Libraries follow the order of their starts
2. A recursive listing without `kind` returns no folder, and with `insideArchives` none of the archives it reads like folders, while a start walked because its Library's name index is not usable still returns its archives; with `kind` it returns exactly the named kinds, symbolic links counted as `other`; a listing of direct children without `kind` returns every kind; no listing returns a hidden Entry
3. A recursive listing with `insideArchives` over a folder that holds archives returns the Entries inside them, nested archives down to `max_nested_depth`, from the name index, with no name, rating, or tag condition as well; without `insideArchives`, the same listing returns each archive as one Entry and nothing inside it
4. A start that is an archive returns its inside by walking it, with or without `insideArchives`, and reads the archives nested in it only with `insideArchives`
5. A start covered by another start, the same start given twice included, adds no Entry and does not change `totalCount`; a start inside an archive is covered by a recursive start above it only with `insideArchives`
6. A start whose folder was deleted, a start in a Library that is not configured, and a start that names a plain file are named in `missingStarts`, and the other starts are listed; with every start missing, the result is empty and every start is named
7. A start whose Library's name index is not usable, in a listing the index would answer, is walked and named in `walkedStarts`; a start whose listing reaches `recursive_max_entries` is named in `cutOffStarts`
8. `locate` with a start-form value returns the same page as the `offset` it gives, and a value outside the sequence is `NotFound`; `locate` with `offset`, more than 64 starts, a malformed start value, and a sort or kind outside the contract are `InvalidInput`
9. `listSearchEntries`, `listTimelineEntries`, and `listTimelineMonths` answer as before
10. `cd daemon && RUSTC_WRAPPER= cargo make lint` and `RUSTC_WRAPPER= cargo test` pass, and the committed interface crate and web client API types are what the contract generates

### 2.2 Non-Goals

- Timeline-date order, its cursors, `month`, `around`, and `listEntryMonths`, and the per-month counts per kind and inside-archive. They are the second half of M28, planned after this one so that its steps start from this plan's code.
- Removing `listSearchEntries`, `listTimelineEntries`, and `listTimelineMonths`. M31 removes them once no client calls them ([design.md §12.2](../design.md#122-roadmap)).
- Reading `listEntries` from the web client or the iOS app. They are M30 and the iOS app's I6.
- Shelves and folder settings, which are M29.

## 3. Current State

The search API takes `libraryIds`, one `path` that may name a start other than the root only with one Library, `recursive`, name, rating, and tag conditions, a sort, and `offset` or `locate` with `locateLibraryId` ([openapi.yaml#L607-L671](../../daemon/openapi.yaml#L607-L671)), and answers `SearchListing` with `truncatedLibraryIds` and `indexed` ([openapi.yaml#L2942-L2995](../../daemon/openapi.yaml#L2942-L2995)).
Its route builds one `ListOptions` per Library with that Library's rating and tag selection ([search.rs#L21-L63](../../daemon/entrypoints/daemon/src/interface/features/search.rs#L21-L63)), and `LibraryBrowser::search_results` rejects a duplicate Library and builds one snapshot per Library from the same path ([library.rs#L109-L127](../../daemon/entrypoints/daemon/src/service/library.rs#L109-L127)).
`SearchResults` merges the per-Library snapshots in sort order without keeping the merged sequence, cuts a page by `offset` or `locate`, and joins the snapshot ids ([listing/search.rs#L25-L156](../../daemon/entrypoints/daemon/src/service/listing/search.rs#L25-L156)).
An unknown Library is `NotFound` ([library.rs#L264-L269](../../daemon/entrypoints/daemon/src/service/library.rs#L264-L269)), and a path that names a file that is neither a folder nor an archive is `InvalidInput` ([library.rs#L287-L303](../../daemon/entrypoints/daemon/src/service/library.rs#L287-L303)).

`ListOptions` carries `foldersOnly` and no kind ([listing.rs#L34-L76](../../daemon/entrypoints/daemon/src/service/listing.rs#L34-L76)), and its page-time match applies `foldersOnly`, the name terms, and the rating and tag selection ([listing.rs#L168-L170](../../daemon/entrypoints/daemon/src/service/listing.rs#L168-L170)).
A listing is answered from the name index only when it is recursive, without hidden Entries, and has a name, rating, or tag condition ([listing.rs#L83-L92](../../daemon/entrypoints/daemon/src/service/listing.rs#L83-L92)), from a local folder whose Library's index is usable ([library.rs#L229-L262](../../daemon/entrypoints/daemon/src/service/library.rs#L229-L262)); the snapshot key holds the Library, the path, `recursive`, `includeHidden`, and the index conditions ([snapshot.rs#L68-L94](../../daemon/entrypoints/daemon/src/service/listing/snapshot.rs#L68-L94)).
The walk descends only into folders, so it reads no archive under a folder and no archive nested in an archive start ([listing.rs#L382-L437](../../daemon/entrypoints/daemon/src/service/listing.rs#L382-L437)).

The name index's search takes a Library, a start, the terms, and whether only images and videos are wanted, and its lookup takes a Library, paths, the terms, and the same choice, with no other kind ([name_index.rs#L190-L198](../../daemon/entrypoints/daemon/src/service/listing/name_index.rs#L190-L198), [name_index.rs#L675-L679](../../daemon/entrypoints/daemon/src/service/listing/name_index.rs#L675-L679)), and both leave out the Entries inside archives through `outside_archives` ([name_index.rs#L681-L762](../../daemon/entrypoints/daemon/src/service/listing/name_index.rs#L681-L762)).
The index holds a kind per Entry: folder, file, archive, image, or video, with a symbolic link held as a file ([name_index.rs#L94-L128](../../daemon/entrypoints/daemon/src/service/listing/name_index.rs#L94-L128), [scanner.rs#L282-L298](../../daemon/entrypoints/daemon/src/service/timeline/scanner.rs#L282-L298)).
`IndexedListing::entries` reads the attributes of the matches through `resolve_listing_entry`, which also resolves a path inside an archive ([indexed.rs#L53-L154](../../daemon/entrypoints/daemon/src/service/library/indexed.rs#L53-L154), [path.rs#L100-L133](../../daemon/entrypoints/daemon/src/service/library/path.rs#L100-L133)).

There is therefore no request that lists several starts of one Library, chooses kinds, or reads the insides of archives, and no response that names the starts cut off, walked, or missing.

## 4. Change Policy

`listEntries` is a new route over the existing per-start snapshots.
A Scope parses the `start` values, drops the starts another start covers, and resolves each remaining start to a snapshot or to a missing start.
The Filter's kind becomes part of the page-time match for walked snapshots and part of the index query for answered ones, and `insideArchives` becomes part of the snapshot key, turns off `outside_archives` in the index query, makes the index answer a recursive local start even without a condition, and lets the walk of an archive start descend into nested archives.
The merge of `SearchResults` becomes a merge over starts that both the search API and `listEntries` use, so the search API keeps its answers.

### 4.1 Decisions Assumed

| Decision | Source |
| --- | --- |
| `index-archive-insides` is done | [index-archive-insides](./index-archive-insides.md) |
| `listEntries` is `GET /entries` with `start`, `recursive`, `insideArchives`, the Filter, the sort, and the page in its query, and at most 64 starts | [Listing and Search Design §3](../design/listing.md#3-sorting-and-filtering), [§8.1](../design/listing.md#serve-the-listing-at-entries-and-its-per-month-counts-at-entriesmonths), [§8.1](../design/listing.md#accept-at-most-64-starts) |
| One response schema `EntryListing` with `cutOffStarts`, `walkedStarts`, and `missingStarts`, and no `indexed` | [Listing and Search Design §4.2](../design/listing.md#42-extracting-a-page), [§8.1](../design/listing.md#answer-both-kinds-of-pages-with-one-response-schema) |
| A recursive listing without a kind returns no folder, and no archive when the insides are read | [Listing and Search Design §5](../design/listing.md#5-name-search), [§8.1](../design/listing.md#list-only-files-recursively-unless-the-filter-asks-for-folders), [§8.1](../design/listing.md#read-archives-like-folders-when-a-scope-reads-their-insides) |
| The index answers a recursive local start when the Scope reads the insides of archives, and an archive start is walked, nested archives read only with the insides | [Listing and Search Design §5.2](../design/listing.md#52-answering-recursive-listings-with-conditions), [§8.1](../design/listing.md#walk-a-start-that-is-an-archive) |
| Overlapping starts list an Entry once, and missing starts, plain files and unconfigured Libraries included, are skipped and named | [Listing and Search Design §5.3](../design/listing.md#53-listing-several-starts), [§8.1](../design/listing.md#list-an-entry-once-when-starts-overlap), [§8.1](../design/listing.md#treat-a-start-that-names-a-plain-file-as-missing) |
| `/capabilities` does not announce `listEntries` | [Listing and Search Design §8.1](../design/listing.md#do-not-announce-listentries-in-the-capabilities) |

### 4.2 Scope of Impact

[openapi.yaml](../../daemon/openapi.yaml) gains `GET /entries` and its schemas, so the generated `daemon/entrypoints/interface` changes; `service/listing.rs`, `listing/search.rs`, `listing/snapshot.rs`, `listing/name_index.rs`, `service/library.rs`, `library/indexed.rs`, a new route module under `interface/features/`, their tests, Listing and Search Design §9, and design.md §12 change.
The existing APIs, the configuration, and the stored formats do not change, and compatibility does not break.
[terms.md](../terms.md) already names `insideArchives` in the Scope's row.

## 5. Steps

| No. | Content | Depends on | Status |
| --- | --- | --- | --- |
| [1](#s-1) | Give listings a kind and the choice of reading the insides of archives | - | Done |
| [2](#s-2) | Let the name index answer with kinds and the insides of archives | 1 | Done |
| [3](#s-3) | Walk the archives nested in an archive start | 1 | Done |
| [4](#s-4) | Parse a Scope and resolve its starts | - | Not started |
| [5](#s-5) | Merge the listings of the starts | 2, 3, 4 | Not started |
| [6](#s-6) | Serve `listEntries` | 5 | Not started |
| [7](#s-7) | Describe the state | 6 | Not started |

<a id="s-1"></a>
### Step 1. Give Listings a Kind and the Choice of Reading the Insides of Archives

**Locations to change**
[listing.rs#L34-L76](../../daemon/entrypoints/daemon/src/service/listing.rs#L34-L76), [listing.rs#L83-L92](../../daemon/entrypoints/daemon/src/service/listing.rs#L83-L92), [listing.rs#L168-L170](../../daemon/entrypoints/daemon/src/service/listing.rs#L168-L170), and [snapshot.rs#L68-L94](../../daemon/entrypoints/daemon/src/service/listing/snapshot.rs#L68-L94).

**What to do**
Add to `ListOptions` the kinds of the Filter, whether the Scope reads the insides of archives, and whether the listing is a `listEntries` one, all empty or false for the folder listing and the search API, so that their answers do not change; a `listEntries` listing never includes hidden Entries, as the search API does not.
Classify an Entry as a folder, an archive (a file whose `archive` is true), an image or a video (by `MediaKind::of_name` on a file that is not an archive), or other (any other file and a symbolic link).
Add to `ListingEntries` whether it read archives like folders, which is true for an answer of the name index with the insides of archives (step 2) and for a walk that descended into archives (step 3), and false otherwise.
With kinds given, the page-time match keeps only those kinds; with none, a recursive `listEntries` listing keeps every kind except folders, and also except archives when its `ListingEntries` read archives like folders, so that a walk taken because the index is not usable returns archives like other files; a listing of direct children keeps every kind.
Without `recursive`, `insideArchives` changes nothing in the listing of a start: the direct children are listed, an archive among them as one Entry.
Put the insides choice in the snapshot key, and add the kinds and the insides choice to the index conditions and their key, so that two listings that the index answers differently do not share a snapshot.
Make `index_conditions` also return conditions for a recursive listing without hidden Entries whose Scope reads the insides of archives, with no name, rating, or tag condition.

**Completion condition**
Unit tests show the classification of each kind, the default kinds of a recursive listing over entries that read archives like folders and over entries that did not, and of a listing of direct children, the snapshot keys that differ by the insides and the kinds, and `index_conditions` for the insides alone; the existing listing and search tests pass.

<a id="s-2"></a>
### Step 2. Let the Name Index Answer with Kinds and the Insides of Archives

**Locations to change**
[name_index.rs#L190-L198](../../daemon/entrypoints/daemon/src/service/listing/name_index.rs#L190-L198), [name_index.rs#L659-L762](../../daemon/entrypoints/daemon/src/service/listing/name_index.rs#L659-L762), and [indexed.rs#L53-L92](../../daemon/entrypoints/daemon/src/service/library/indexed.rs#L53-L92).

**What to do**
Replace `only_media` in `NameQuery` and in `lookup` by the set of kinds to return, and add whether the insides of archives are read.
Build the kind condition in SQL from the set, with the images and videos that the rating and tag conditions keep intersected with it, and leave out `outside_archives` when the insides are read.
Pass the kinds and the insides choice from the index conditions in `IndexedListing::entries`, so the search API asks for what it asks for now: every kind without a rating or tag condition, images and videos with one, outside archives.
Mark the `ListingEntries` of an answer with the insides as having read archives like folders (step 1).

**Completion condition**
Tests over a Library with archives show a recursive index answer that returns the Entries inside archives, nested ones included, only when the insides are read, the kinds limited to the set, a rating or tag condition intersected with the kinds, and an answer with the insides but no other condition; the existing `name_index` and `library::indexed` tests pass.

<a id="s-3"></a>
### Step 3. Walk the Archives Nested in an Archive Start

**Locations to change**
[listing.rs#L382-L437](../../daemon/entrypoints/daemon/src/service/listing.rs#L382-L437), its caller in the tests at [listing.rs#L770](../../daemon/entrypoints/daemon/src/service/listing.rs#L770), and [library.rs#L229-L262](../../daemon/entrypoints/daemon/src/service/library.rs#L229-L262).

**What to do**
Give `ListingEntries::scan` whether to descend into archives, and the nested depth limit; when it descends, it walks each archive it meets like a folder, through the archive's root as `resolve_folder` reaches it, while the depth of nesting is under `max_nested_depth`.
In `LibraryBrowser::snapshot`, descend only when the listing is recursive, the start is an archive or a path inside one, and the Scope reads the insides of archives, so a walk under a local folder still reads no archive; mark the resulting `ListingEntries` as having read archives like folders.
Put the tests in the tests of `listing.rs`, under `service::listing`.

**Completion condition**
Tests show a recursive walk of an archive start that lists the Entries of a nested archive with the insides and lists the nested archive as one Entry without them, the depth limit respected, and a walk of a local folder that reads no archive in either case.

<a id="s-4"></a>
### Step 4. Parse a Scope and Resolve Its Starts

**Locations to change**
A new module `service/listing/scope.rs` for the Scope, and [library.rs#L264-L303](../../daemon/entrypoints/daemon/src/service/library.rs#L264-L303).

**What to do**
Parse each `start` value into a Library id, up to the first `/`, and a Library-relative path, the root when there is none, keeping the value as it was given to name the start in responses; a value with an empty Library id or a path that `LibraryRelativePath::parse` rejects, and more than 64 starts, are `InvalidInput`.
In `LibraryBrowser`, first resolve every start as `resolve_folder` does, and turn an unknown Library, a path that does not exist, and a path that names a file that is neither a folder nor an archive into a missing start, which keeps its start value and is named in `missingStarts` whatever other start covers its path; other errors fail the request as they do now.
Then, among the resolved starts only, drop each start that another one covers, keeping the covering start at its own position and, of two identical starts, the first.
A start covers itself, and a recursive start covers every start in the same Library whose path is at or under its own, unless an archive boundary lies between them and the Scope does not read the insides of archives.
An archive boundary lies between a start and a start under it when an archive file on the lower start's path, the lower start itself when it is an archive, is strictly under the upper start's path; the archive files on a start's path are the archive segments of its resolved `NestedPath` ([storage.rs#L34-L36](../../daemon/modules/engine/src/service/storage.rs#L34-L36)).
A missing start covers nothing.

**Completion condition**
Unit tests show the parsing of start values with and without a path, the `InvalidInput` cases, the dropped starts for duplicates, nested paths, non-recursive starts, an archive start and a start inside an archive under a recursive start with and without the insides, the three kinds of missing start, and a missing start under a covering start that is still named.

<a id="s-5"></a>
### Step 5. Merge the Listings of the Starts

**Locations to change**
[listing/search.rs#L1-L156](../../daemon/entrypoints/daemon/src/service/listing/search.rs#L1-L156) and [library.rs#L90-L127](../../daemon/entrypoints/daemon/src/service/library.rs#L90-L127).

**What to do**
Generalize `SearchSource` and `Merge` to sources that carry a start value, a Library id, a snapshot, and that start's options, and keep `SearchResults::page` and `locate` answering the search API as now on top of it.
Add a listing of a Scope that merges its sources in the sort with ties in the order of the starts, cuts a page by `offset` or by `locate` given as a Library id and a Library-relative path, and builds the response parts of `EntryListing`: the Entries with their Library ids, `totalCount`, `offset`, the joined snapshot id, `cutOffStarts` from the truncated snapshots, `walkedStarts`, and `missingStarts` from step 4.
A start is in `walkedStarts` when its options give `index_conditions`, the start is a local folder (its resolved path has no archive identity), and its snapshot's `ListingEntries` is not `indexed`, which covers both an index that was not usable when the snapshot was built and an index that failed while it answered; this needs no change to what `LibraryBrowser::snapshot` returns.
Add to `LibraryBrowser` the entry points that build this listing from a Scope, with each start's options carrying the rating and tag selection of its Library.

**Completion condition**
Tests show the merged order over starts of one and of two Libraries, joined pages without a repeat or a gap, ties ordered by start, `locate` matching the page of its `offset`, the three lists of starts, and an empty listing that names every start when all are missing; the existing search tests pass unchanged.

<a id="s-6"></a>
### Step 6. Serve `listEntries`

**Locations to change**
[openapi.yaml](../../daemon/openapi.yaml), the generated `daemon/entrypoints/interface` through [gen-interface.sh](../../daemon/entrypoints/gen-interface.sh), the generated [ui-web/src/api/schema.d.ts](../../ui-web/src/api/schema.d.ts) through `npm run gen:api` ([package.json#L14](../../ui-web/package.json#L14)), and a new route module `interface/features/entries.rs` registered beside [search.rs](../../daemon/entrypoints/daemon/src/interface/features/search.rs).

**What to do**
Add to the contract `GET /entries` with operation `listEntries` under a new tag `Entries`, with the query `start` (repeated, required, `minItems: 1`, `maxItems: 64`, the route checking the count too), `recursive`, `insideArchives`, `query`, `rating`, `tag` (repeated), `kind` (repeated, `folder`, `archive`, `image`, `video`, `other`), `sort`, `order`, `foldersFirst`, `offset`, `locate`, and `limit`, the responses of the search API, and the schemas of [Listing and Search Design §8.1](../design/listing.md#answer-both-kinds-of-pages-with-one-response-schema).
`ScopedEntry` has the required `libraryId` and `entry` (a `FileSystemEntry`); its timeline fields are left to the later plan.
`EntryListing` has the required `entries` (`ScopedEntry` items), `cutOffStarts`, `walkedStarts`, and `missingStarts` (arrays of start values as given), and the optional `totalCount`, `offset`, and `snapshot`, which this plan always fills and timeline-date order will omit.
Regenerate the interface crate and the web client's API types, and implement the route: parse the Scope and the Filter, select each start's ratings and tags for the requesting user, and answer `InvalidInput` for `locate` with `offset` and for the cases of step 4.

**Completion condition**
Route tests over two Libraries show conditions 1 to 8 through HTTP, the existing route tests of the search and timeline APIs pass, and regenerating the interface crate and the web client's API types leaves no difference.

<a id="s-7"></a>
### Step 7. Describe the State

**Locations to change**
[Listing and Search Design §9.1](../design/listing.md#91-current-code), [§9.2](../design/listing.md#92-roadmap), [design.md §12.1](../design.md#121-implementation-status), and [§12.2](../design.md#122-roadmap).

**What to do**
Describe in Listing and Search Design §9.1 that `listEntries` answers the orders cut from snapshots over a Scope and a Filter, with the kinds, the insides of archives, archive starts, overlapping and missing starts, and the three lists of starts, and that timeline-date order is not yet accepted.
Mark L4 and M28 in progress, and change the gap in design.md §12.1 so that it names timeline-date order and the per-month counts over a Scope instead of search mode over `listSearchEntries`.

**Completion condition**
§9.1 describes `listEntries` as implemented in the orders cut from snapshots, L4 and M28 are in progress, and §12.1 names only what is left.

## 6. Acceptance Checks

| Condition | Command | Expected result |
| --- | --- | --- |
| 1 to 8 | `cd daemon && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon interface::features::entries` | The route tests of step 6 pass: one merged sequence over two Libraries with joined pages, the default and named kinds, the insides of archives from the index and of an archive start, covered and duplicated starts adding nothing, the three kinds of missing start named, a walked and a cut-off start named, `locate` matching its `offset` page, and the `InvalidInput` and `NotFound` cases |
| 1, 5, 6, 7, 8 | `cd daemon && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon service::listing && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon service::library` | The unit tests of steps 4 and 5 pass |
| 2 | `cd daemon && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon service::listing` | The tests of step 1 pass: each kind classified, the default kinds of recursive and direct listings, and the named kinds alone |
| 3 | `cd daemon && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon name_index && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon library::indexed` | The tests of step 2 pass: the insides of archives returned only with the insides, nested ones included, also without a condition |
| 4 | `cd daemon && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon service::listing` | The tests of step 3 pass: the nested archive of an archive start read only with the insides, within the depth limit |
| 9 | `cd daemon && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon interface::features::search && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon interface::features::timeline && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon listing::search` | The existing tests of the search and timeline APIs pass without change |
| 10 | `cd daemon && RUSTC_WRAPPER= cargo make lint && RUSTC_WRAPPER= cargo test` | Both exit 0 |
| 10 | `daemon/entrypoints/gen-interface.sh && git diff --exit-code -- daemon/entrypoints/interface` | Exits 0: the committed interface crate is what the contract generates, as the CI job `interface` checks |
| 10 | `cd ui-web && npm run gen:api && git diff --exit-code -- src/api` | Exits 0: the committed API types are what the contract generates, as the CI job `ui` checks |

The new tests protect the following.

- The kinds: the classification of every Entry, and the defaults of recursive and direct listings
- The insides of archives: the index's answer with and without them, and the walk of an archive start
- The Scope: the parsing of start values, the starts covered by others, and the missing starts
- The merge: the order over starts and Libraries, pages, `locate`, and the three lists of starts
- The route: `listEntries` over HTTP with its error cases, and the old APIs unchanged
