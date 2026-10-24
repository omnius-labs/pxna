# Answer Recursive Searches from the Name Index

## 1. About This Plan

This work answers recursive listings with name, rating, or tag conditions from the name index, and shows in the web client when a result was walked instead.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The steps and acceptance checks of answering listings from the name index |
| [Listing and Search Design §5.2](../design/listing.md#52-answering-recursive-listings-with-conditions) | When a listing is answered from the index, how its snapshot is built |
| [Listing and Search Design §6](../design/listing.md#6-recursion-limit) | Counting the limit by matches |
| [Listing and Search Design §8.1](../design/listing.md#name-index) | The decisions on the index, attributes, matching, and walking while it is not usable |
| [Rating and Tag Design §7.1](../design/rating-tags.md#71-filtering-listings) | Rating and tag conditions in listings answered from the index |
| [Client Design §3.6](../design/client.md#36-search) | The notice for walked results |
| [openapi.yaml#L2998](../../daemon/openapi.yaml#L2998) | The `SearchListing` schema that gains `indexed` |

[design.md](../design.md#11-division-of-responsibility-between-documents) holds the map of the document family.

### 1.2 Creation Date and Target Commit

Created 2026-10-23 / Target commit `47b81b0`. Current State and the steps were rewritten on 2026-10-25 for the search API of `search-condition-panel` at commit `1773c4a` plus that work.

## 2. Purpose and Completion Conditions

A search from the root of a Library of 10 million Entries returns its first page within a second for terms of 3 or more characters, and is not cut off unless the matches themselves exceed the limit.

### 2.1 Completion Conditions

1. A recursive search with conditions and a local start in a Library with a usable index returns `indexed: true` for it and the same set of Entries as a walk of the same tree with no external changes since the last aggregation
2. A recursive search whose index is not usable, or whose start is an archive, returns `indexed: false` and the walked result; with several Libraries, `indexed` is true only when every Library was answered from its index
3. A listing answered from the index drops a match whose Entry was deleted externally after aggregation, and returns current sizes and times
4. With good, bad, tag, and unrated conditions, with and without name terms, a listing answered from the index returns the same Entries as the walk, and a rating write changes its snapshot id
5. `truncated` is set by the number of matches, so a search from the root whose matches are fewer than `recursive_max_entries` is not truncated on a Library larger than that
6. On the 10-million-Entry fixture, with terms matching at most 1,000 Entries, the p95 of the first page is at most 1 second for a term of 3 or more characters and at most 5 seconds for terms of 2 characters or fewer, and a following page is at most 100 ms, recorded in `docs/measurements`
7. In a subtree search from a folder outside archives, the web client displays "索引を作成中のため、結果が不完全な場合があります。" above the results when `indexed` is false, and does not display it when `indexed` is true, with "サブフォルダを含めない", or from inside an archive
8. The current-state sections and roadmaps of Listing and Search Design, Timeline Design, Client Design, and design.md describe the index as implemented
9. `cd daemon && cargo make lint`, the daemon tests, and `cd ui-web && npm run lint && npm run typecheck && npm test` pass

### 2.2 Non-Goals

- Answering non-recursive listings or the timeline from the index. Rating and Tag Design §8.2 keeps timeline conditions deferred.
- Changing the meaning of a name match. Listing and Search Design §5 stays as it is.

## 3. Current State

[library.rs#L183](../../daemon/entrypoints/daemon/src/service/library.rs#L183) builds every listing, including each Library's listing of `search_entries` at [library.rs#L74](../../daemon/entrypoints/daemon/src/service/library.rs#L74), with `ListingEntries::scan`, which walks and stops at `recursive_max_entries` walked Entries.
Name terms are matched at [listing.rs#L259](../../daemon/entrypoints/daemon/src/service/listing.rs#L259) and rating and tag sets are read at [listing.rs#L172](../../daemon/entrypoints/daemon/src/service/listing.rs#L172) when cutting the page.
The response schema `SearchListing` at [openapi.yaml#L2998](../../daemon/openapi.yaml#L2998) has no field telling how it was answered.
The web client shows the truncation notice at [Grid.tsx#L163](../../ui-web/src/components/Grid.tsx#L163) and [List.tsx#L150](../../ui-web/src/components/List.tsx#L150).
A search from the root of a large Library therefore waits for the walk and is cut off by its size.

## 4. Change Policy

Branch in `LibraryBrowser::snapshot` by the conditions of Listing and Search Design §5.2, build the snapshot from an index query plus attribute reads of the matches, and report the branch in the response.

### 4.1 Decisions Assumed

| Decision | Source |
| --- | --- |
| `name-index-store` is complete | [name-index-store](./name-index-store.md) |
| `search-condition-panel` is complete | [search-condition-panel](./search-condition-panel.md) |
| When to answer from the index, the snapshot of matches, rating and tag candidates | [Listing and Search Design §5.2](../design/listing.md#52-answering-recursive-listings-with-conditions) |
| Reading the attributes of matches from the file system | [Listing and Search Design §8.1](../design/listing.md#read-the-attributes-of-matching-entries-from-the-file-system) |
| Trigram matching for terms of 3 or more characters, direct comparison for shorter terms | [Listing and Search Design §8.1](../design/listing.md#match-names-in-the-index-with-trigrams-and-compare-short-terms-directly) |
| Walking while the index is not usable | [Listing and Search Design §8.1](../design/listing.md#walk-while-the-name-index-is-not-usable) |
| Counting the limit by matches | [Listing and Search Design §8.1](../design/listing.md#cut-off-recursive-walks-by-count) |
| The notice for walked results | [Client Design §3.6](../design/client.md#36-search) |

### 4.2 Scope of Impact

The daemon's listing service, `openapi.yaml`, the generated interface, the web client's generated schema, and its Grid and List change.
`SearchListing` gains a required response field `indexed`. Clients that ignore unknown fields keep working; the generated clients are regenerated in this work.
The term `indexed` is already in [terms.md](../terms.md#2-term-list).

## 5. Steps

| No. | Content | Dependencies | State |
| --- | --- | --- | --- |
| [1](#s-1) | Add `indexed` to the listing contract | - | Not started |
| [2](#s-2) | Query matches from the index | - | Not started |
| [3](#s-3) | Build snapshots of matches | 1, 2 | Not started |
| [4](#s-4) | Start from ratings and tags | 3 | Not started |
| [5](#s-5) | Show the notice for walked results | 1 | Not started |
| [6](#s-6) | Measure on the fixture and update the current state | 3, 4, 5 | Not started |

<a id="s-1"></a>
### Step 1. Add `indexed` to the Listing Contract

**Locations to change**
[openapi.yaml#L2998](../../daemon/openapi.yaml#L2998), [search.rs](../../daemon/entrypoints/daemon/src/service/listing/search.rs), generated `daemon/entrypoints/interface` and `ui-web/src/api/schema.d.ts`

**What to do**
Add the required boolean `indexed` to `SearchListing`, described as true when every searched Library was answered from its name index, and have the merge in `search.rs` combine the per-Library flags. Regenerate with `daemon/entrypoints/gen-interface.sh` and `npm run gen:api`, and return `false` from the existing walk.

**Completion condition**
CI's check that the generated interface has no diff passes, and existing listing tests assert `indexed == false`.

<a id="s-2"></a>
### Step 2. Query Matches from the Index

**Locations to change**
`daemon/entrypoints/daemon/src/service/listing/name_index.rs` from `name-index-store`

**What to do**
Add a query that takes a Library, a start, normalized terms, an optional candidate set, and a cancellation token, and returns matching paths and kinds under the start. Terms of 3 or more characters go to `MATCH` as quoted phrases, and shorter ones are compared with `instr` on the normalized name. Add a query for the image and video rows under the start for the unrated condition.

**Completion condition**
Unit tests show the same matches as `NameFilter` at [listing.rs#L210](../../daemon/entrypoints/daemon/src/service/listing.rs#L210) for Japanese, decomposed, full-width, 1-, 2-, and 3-character terms, and that a cancelled query stops.

<a id="s-3"></a>
### Step 3. Build Snapshots of Matches

**Locations to change**
[library.rs#L183](../../daemon/entrypoints/daemon/src/service/library.rs#L183), [snapshot.rs#L242](../../daemon/entrypoints/daemon/src/service/listing/snapshot.rs#L242)

**What to do**
When Listing and Search Design §5.2 applies, build a snapshot keyed by the conditions from the index query, read the attributes of the matches with the detail batch's workers, drop missing Entries, stop at `recursive_max_entries` matches, and set `indexed: true`. Otherwise keep the walk.

**Completion condition**
Tests show conditions 1, 2, 3, and 5 against a temporary Library aggregated once and then changed externally.

<a id="s-4"></a>
### Step 4. Start from Ratings and Tags

**Locations to change**
[listing.rs#L172](../../daemon/entrypoints/daemon/src/service/listing.rs#L172)

**What to do**
For listings answered from the index, pass the rated or tagged paths under the start as candidates, or for unrated the index's images and videos minus the rated paths, and include the version of ratings and tags in the snapshot's key.

**Completion condition**
Tests show condition 4.

<a id="s-5"></a>
### Step 5. Show the Notice for Walked Results

**Locations to change**
[Grid.tsx#L163](../../ui-web/src/components/Grid.tsx#L163), [List.tsx#L150](../../ui-web/src/components/List.tsx#L150)

**What to do**
In search mode with a recursive scope and a start outside archives, display the notice of Client Design §3.6 when the first page's `indexed` is false.

**Completion condition**
Component tests show the notice for `indexed: false` and its absence for `indexed: true`, for "サブフォルダを含めない", for a start inside an archive, and for folder view.

<a id="s-6"></a>
### Step 6. Measure on the Fixture and Update the Current State

**Locations to change**
A new record under `docs/measurements/` and its row in [measurements.md](../measurements.md), [listing.md §9](../design/listing.md#9-current-state-and-remaining-work), [timeline.md §10.1](../design/timeline.md#101-current-code), [client.md §6](../design/client.md#6-current-state-and-remaining-work), [design.md §12.2](../design.md#122-roadmap)

**What to do**
On the fixture of `name-index-store` after a full aggregation, run 20 searches per term class from the root through the HTTP API and record the p95 of the first page and of a following page. Then rewrite the current-state sentences and mark L1, C12, and M19 as done.

**Completion condition**
The record meets condition 6, and `rg -n 'name index of §5.1 and §5.2 is not implemented' docs` returns nothing.

## 6. Acceptance Checks

| Condition | Command | Expected result |
| --- | --- | --- |
| 1, 2, 3, 5 | `cd daemon && RUSTC_WRAPPER= cargo nextest run -p omnius-pxna-daemon listing` | The tests comparing index answers with walks, the fallbacks, external deletion, and match-count truncation pass |
| 4 | `cd daemon && RUSTC_WRAPPER= cargo nextest run -p omnius-pxna-daemon rating` | The tests for good, bad, tag, and unrated conditions and the snapshot id after a write pass |
| 6 | Read the new record in `docs/measurements/` | p95 of the first page is 1 s or less for 3-character terms and 5 s or less for short terms, and a following page is 100 ms or less |
| 7 | `cd ui-web && npx vitest run src/components` | The notice tests pass |
| 8 | `rg -n 'name index of §5.1 and §5.2 is not implemented\|does not pass enumerated names\|does not display the notice' docs/design` and read design.md §12.2 | No match, and M19 is Done |
| 9 | `cd daemon && cargo make lint && RUSTC_WRAPPER= cargo nextest run --all-features` and `cd ui-web && npm run lint && npm run typecheck && npm test` | All exit with 0 |
