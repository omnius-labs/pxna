# Find the Page of an Entry Named by a URL with `locate`

## 1. About This Plan

This work lets the server return the page that holds a named Entry (`locate`), and switches the viewer opened by URL and "Show in folder" to it, so that an Entry on any page can be opened directly.

### 1.1 Document Responsibilities

| Document | What it covers |
| --- | --- |
| This document | The steps and acceptance checks of this work |
| [Listing and Search Design §4.5](../design/listing.md#45-locating-an-entry) | What `locate` and the response's `offset` mean |
| [Listing and Search Design §8.1](../design/listing.md#locate-an-entry-from-its-position-in-the-snapshot) | The decision and its reasons |
| [Client Design §3.5](../design/client.md#35-loading-listings) | How the client sends the path and uses the page it receives |
| [Client Design §5.1](../design/client.md#ask-the-server-where-an-entry-opened-by-url-is) | The decision on the client side |
| [terms.md](../terms.md#2-term-list) | The name Locate |
| [issues.md](../issues.md) | The defect this work fixes |

[design.md](../design.md#11-division-of-responsibility-between-documents) holds the map of the document family.

### 1.2 Creation Date and Target Commit

Created 2026-11-04 / Target commit `ef9459e`

## 2. Purpose and Completion Conditions

The owner can reload or share a viewer URL for an Entry anywhere in a large folder or search result, and "Show in folder" finds an Entry in one request, instead of the viewer showing "Not found." beyond the first 200 Entries.

### 2.1 Completion Conditions

1. For `listFolderEntries` and `listFolderSummaries`, a request with `locate` returns 200 with the page that holds the Entry, an `offset` that is a multiple of `limit`, and the same `entries`, `totalCount`, and `snapshot` as a request for that `offset`
2. For `listSearchEntries`, a request with `locate` and `locateLibraryId` returns the page of the merged sequence that holds the Entry, equal to the page of the same `offset`, including Entries that tie across Libraries and a search whose start is not the Library root
3. `locate` together with `offset`, `locateLibraryId` without `locate`, `locate` without `locateLibraryId` in a search, and a `locateLibraryId` outside `libraryIds` return 400 `InvalidInput`; a path that is not in the sequence after the sort and filters, such as a hidden Entry or a file under `foldersOnly`, returns 404 `NotFound`
4. Every response of the three APIs carries `offset`, equal to the requested `offset` when `locate` is not given
5. Opening `/browse/{libraryId}/{folder}?view=<Entry beyond the first 200>` shows that Entry's image, and the page requests of the folder are the grid's first page and one `locate` request, with no other page in between
6. Opening `/search?...&view=<Entry beyond the first 200>` shows that Entry the same way, including a search whose start is not the Library root and whose Entry is in a subfolder of the start
7. An Entry that is not in the listing opens the viewer with "Not found."
8. "Show in folder" for an Entry beyond the first 200 selects it and scrolls to it with one `locate` request
9. The page loops of `findEntry` and `findIndex` are gone, and the timeline viewer's tests pass unchanged
10. The defect's file and its row are deleted, the stale current-state sentences of [Client Design §6.1](../design/client.md#61-current-code) and [Listing and Search Design §9](../design/listing.md#9-current-state-and-remaining-work) are gone, and L2, C13, and M24 are closed in the roadmaps as their tables do for finished items
11. `cd daemon && cargo make lint` and `cd ui-web && npm run lint && npm run typecheck && npm test && npm run build` pass

### 2.2 Non-Goals

- A path-to-position index. Whether to build one is a deferred item of the design ([Listing and Search Design §8.2](../design/listing.md#whether-to-index-entry-positions)).
- A capability flag for `locate`. The daemon serves the web client from the same process, so the two always match.
- Renaming the web client's `Listing.locate`, which finds an Entry among the pages already read. The terms row separates the two names.
- Changing the timeline's own search around an Entry, which has no `offset`.

## 3. Current State

The folder and search store keeps pages by index, and its [locate at listing.ts#L170](../../ui-web/src/library/listing.ts#L170) answers only from pages already read.
[findEntry at viewer.ts#L12](../../ui-web/src/viewer/viewer.ts#L12) uses `locate` for any listing that has it, and returns from the [branch at viewer.ts#L19](../../ui-web/src/viewer/viewer.ts#L19), so the [page-by-page search at viewer.ts#L26](../../ui-web/src/viewer/viewer.ts#L26) is not reached for folder and search listings; an Entry beyond the first page is "Not found." ([the recorded defect](../issues/viewer-url-beyond-first-page-not-found.md)).
"Show in folder" does reach the Entry, by [findIndex at useRevealEntry.ts#L61](../../ui-web/src/library/useRevealEntry.ts#L61), which reads pages one request at a time.
The server cuts a page with `offset` and `limit` from a snapshot: [ListingSnapshot::page at snapshot.rs#L120](../../daemon/entrypoints/daemon/src/service/listing/snapshot.rs#L120) and [ListOptions::page at listing.rs#L110](../../daemon/entrypoints/daemon/src/service/listing.rs#L110) for the detail listing, [the summary index at summary.rs#L259](../../daemon/entrypoints/daemon/src/service/listing/summary.rs#L259) for the summary listing, and [SearchResults::page at search.rs#L36](../../daemon/entrypoints/daemon/src/service/listing/search.rs#L36), which merges the per-Library orders from the head on every page.
None of the three responses ([FolderListing at openapi.yaml#L2884](../../daemon/openapi.yaml#L2884), [SearchListing at openapi.yaml#L2919](../../daemon/openapi.yaml#L2919), [FolderSummaryListing at openapi.yaml#L3055](../../daemon/openapi.yaml#L3055)) carries the offset, and no request names the Entry whose position is wanted, so the only way to a position through the API is to read pages until the Entry appears.

## 4. Change Policy

Add `locate` to the three requests and compute the position from the same snapshot the page is cut from, then give the client store one method that requests the page with `locate` and files it as the page at its `offset`, so that the viewer and "Show in folder" share it.

### 4.1 Decisions Assumed

| Decision | Source |
| --- | --- |
| `locate` in place of `offset`, the path relative to the start of the listing, the page-aligned response, `offset` in every response of the three APIs, the `NotFound` and `InvalidInput` cases | [Listing and Search Design §4.5](../design/listing.md#45-locating-an-entry), [§8.1](../design/listing.md#locate-an-entry-from-its-position-in-the-snapshot) |
| The client converts the path, keeps the returned page as the page at its `offset`, and the timeline does not use it | [Client Design §3.5](../design/client.md#35-loading-listings), [§5.1](../design/client.md#ask-the-server-where-an-entry-opened-by-url-is) |
| `viewer-display-modes` is done | [viewer-display-modes](./viewer-display-modes.md) |

### 4.2 Scope of Impact

`daemon/openapi.yaml`, the generated Rust interface and the web client's `schema.d.ts`, the daemon's listing service and routes, the web client's listing store, `findEntry`, `findIndex`, and their tests change.
The response gains a required `offset`, and the request parameters are all optional, so existing requests keep their meaning.
Applying the change is compatible for the web client, whose types are regenerated with it, and for any client that ignores unknown fields; a client that rejects unknown response fields would stop working, and none exists in this repository.
A revert removes `locate` and `offset`, so a client that started using either stops working (§7).
[terms.md](../terms.md#2-term-list) already defines Locate.

## 5. Steps

| No. | Content | Dependencies | State |
| --- | --- | --- | --- |
| [1](#s-1) | Add the contract and regenerate the types | - | Not started |
| [2](#s-2) | Return the page of an Entry from a detail snapshot | 1 | Not started |
| [3](#s-3) | Return the page of an Entry from the summary index | 1 | Not started |
| [4](#s-4) | Return the page of an Entry from a search | 1, 2 | Not started |
| [5](#s-5) | Wire `locate` into the routes | 2, 3, 4 | Not started |
| [6](#s-6) | Request the page in the client store | 5 | Not started |
| [7](#s-7) | Switch the viewer and "Show in folder" | 6 | Not started |
| [8](#s-8) | Confirm in a browser and update the documents | 7 | Not started |

<a id="s-1"></a>
### Step 1. Add the Contract and Regenerate the Types

**Locations to change**
[openapi.yaml#L571](../../daemon/openapi.yaml#L571), [openapi.yaml#L632](../../daemon/openapi.yaml#L632), [openapi.yaml#L724](../../daemon/openapi.yaml#L724), [openapi.yaml#L2884](../../daemon/openapi.yaml#L2884), [openapi.yaml#L2919](../../daemon/openapi.yaml#L2919), [openapi.yaml#L3055](../../daemon/openapi.yaml#L3055), the generated interface (`daemon/entrypoints/gen-interface.sh`), the web client's types (`npm run gen:api`), the three places that build the responses: [listing.rs#L123](../../daemon/entrypoints/daemon/src/service/listing.rs#L123), [summary.rs#L244](../../daemon/entrypoints/daemon/src/service/listing/summary.rs#L244), and [search.rs#L83](../../daemon/entrypoints/daemon/src/service/listing/search.rs#L83), and the test code that builds the query parameter structs by literal: [listing.rs#L498](../../daemon/entrypoints/daemon/src/service/listing.rs#L498), [search.rs#L96](../../daemon/entrypoints/daemon/src/service/listing/search.rs#L96), [folders.rs#L278](../../daemon/entrypoints/daemon/src/interface/features/folders.rs#L278), and [folders.rs#L305](../../daemon/entrypoints/daemon/src/interface/features/folders.rs#L305).

**What to do**
Add the `locate` parameter to the folder and summary listings, `locate` and `locateLibraryId` to the search, and the required `offset` to the three response schemas. Regenerate, give the new query fields their empty value in the test literals, and have the existing code put the requested `offset` in every response. `locate` is accepted by the contract but not yet acted on.

**Completion condition**
The daemon and the web client build, existing tests pass, and tests named `offset_in_response_*` show every response of the three APIs carrying the requested `offset`.

<a id="s-2"></a>
### Step 2. Return the Page of an Entry from a Detail Snapshot

**Locations to change**
[snapshot.rs#L120](../../daemon/entrypoints/daemon/src/service/listing/snapshot.rs#L120), [listing.rs#L110](../../daemon/entrypoints/daemon/src/service/listing.rs#L110), and their tests.

**What to do**
Add a function that returns the position of a start-relative path in the sequence a page is cut from, after the conditions are applied, and cut the page that holds it at a multiple of `limit`, with the `offset` it was cut at.

**Completion condition**
Unit tests named `locate_folder_*` show, for sorts by name, kind, size, and modification date, ascending and descending, with `foldersFirst`, `foldersOnly`, `query`, hidden Entries, and a `recursive` listing, that the page equals the page of the same `offset`, and that an Entry outside the sequence gives no position.

<a id="s-3"></a>
### Step 3. Return the Page of an Entry from the Summary Index

**Locations to change**
[summary.rs#L259](../../daemon/entrypoints/daemon/src/service/listing/summary.rs#L259) and its tests.

**What to do**
Do the same for the summary listing's name order and kind order, from the snapshot that its pages come from.

**Completion condition**
Unit tests named `locate_summary_*` show the page equals the page of the same `offset` for both orders and for `foldersFirst` and `foldersOnly`, and that an excluded Entry gives no position.

<a id="s-4"></a>
### Step 4. Return the Page of an Entry from a Search

**Locations to change**
[search.rs#L36](../../daemon/entrypoints/daemon/src/service/listing/search.rs#L36) and its tests.

**What to do**
Find the position of an Entry of one Library in the merged order as the number of Entries that precede it, from the per-Library listings, without cutting the pages before it, and cut the page from there. Ties across Libraries follow the order of `libraryIds`, and the start of a single-Library search is the base of the path.

**Completion condition**
Tests named `locate_search_*`, with several Libraries, including the same path in two of them, and with a start that is not the Library root, show the page equals the page of the same `offset` over the same snapshots.

<a id="s-5"></a>
### Step 5. Wire `locate` into the Routes

**Locations to change**
[folders.rs#L90](../../daemon/entrypoints/daemon/src/interface/features/folders.rs#L90), [folders.rs#L110](../../daemon/entrypoints/daemon/src/interface/features/folders.rs#L110), [search.rs#L19](../../daemon/entrypoints/daemon/src/interface/features/search.rs#L19), [library.rs#L69](../../daemon/entrypoints/daemon/src/service/library.rs#L69), [library.rs#L75](../../daemon/entrypoints/daemon/src/service/library.rs#L75), [library.rs#L95](../../daemon/entrypoints/daemon/src/service/library.rs#L95).

**What to do**
Pass `locate` and `locateLibraryId` to steps 2 to 4, and return `InvalidInput` and `NotFound` for the cases of §2.1.

**Completion condition**
Route tests named `locate_errors_*` show the `InvalidInput` and `NotFound` cases of §2.1 for the three APIs, tests named `locate_*` show conditions 1 and 2 through the routes, and `cd daemon && cargo make lint` passes.

<a id="s-6"></a>
### Step 6. Request the Page in the Client Store

**Locations to change**
[entries.ts#L109](../../ui-web/src/library/entries.ts#L109), [entries.ts#L123](../../ui-web/src/library/entries.ts#L123), [entries.ts#L187](../../ui-web/src/library/entries.ts#L187), [listing.ts#L196](../../ui-web/src/library/listing.ts#L196), [listing.ts#L242](../../ui-web/src/library/listing.ts#L242), and their tests.

**What to do**
Let the page fetchers pass `locate`, and add a store method that converts an Entry path to a path relative to the start of the listing, requests the page with `locate`, files the answer as the page at its `offset` through the path a page read by `offset` takes, and returns the Entry's position. A changed snapshot id is handled as for any page.

**Completion condition**
Store tests show the page appearing in `entryAt` at its offset, the position returned, the converted path for a folder, for a search from the Library root, and for a search from a subfolder with an Entry in a deeper folder, a `NotFound` answer returning no position, and a changed snapshot id discarding the other pages.

<a id="s-7"></a>
### Step 7. Switch the Viewer and "Show in Folder"

**Locations to change**
[viewer.ts#L12](../../ui-web/src/viewer/viewer.ts#L12), [Viewer.tsx#L234](../../ui-web/src/components/Viewer.tsx#L234), [useRevealEntry.ts#L61](../../ui-web/src/library/useRevealEntry.ts#L61), [viewer.test.ts#L83](../../ui-web/src/viewer/viewer.test.ts#L83), and the viewer and reveal tests.

**What to do**
Make `findEntry` ask the store for the Entry's page for folder and search listings, and keep the timeline's `locate` branch. Make `findIndex` do the same. Remove the page-by-page search and the page scan.

**Completion condition**
A viewer test over a listing of 300 Entries that opens an Entry on the second page records one `locate` request and no `offset=200` request, and shows the image. A test shows "Not found." for an Entry the server does not know. The existing viewer, reveal, and timeline viewer tests pass.

<a id="s-8"></a>
### Step 8. Confirm in a Browser and Update the Documents

**Locations to change**
[client.md §6.1](../design/client.md#61-current-code), [client.md §6.2](../design/client.md#62-roadmap), [listing.md §9](../design/listing.md#9-current-state-and-remaining-work), [design.md §12.2](../design.md#122-roadmap), [issues.md](../issues.md), `docs/issues/viewer-url-beyond-first-page-not-found.md`, [plans.md](../plans.md#2-plan-list).

**What to do**
Run the browser rows of §6, then update the current-state text to match, set L2 and M24 to Done, close C13 the way the table of Client Design §6.2 closes finished items (delete its row, and name it in the sentence after the table as "M24, which corresponds to C13" among the completed milestones), delete the issue and its row, and fold this plan after its acceptance checks pass.

**Completion condition**
Every observation of §6 matches, including the document checks.

## 6. Acceptance Checks

| Condition | Command | Expected result |
| --- | --- | --- |
| 1 | `cd daemon && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon locate_folder && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon locate_summary` | The detail and summary tests pass |
| 2 | `cd daemon && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon locate_search` | The merged-order tests, including ties across Libraries and a start that is not the root, pass |
| 3 | `cd daemon && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon locate_errors` | The `InvalidInput` and `NotFound` tests pass |
| 4 | `cd daemon && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon offset_in_response` | The tests pass |
| 5 | playwright-cli at 1440x900 on the web client served by the daemon, with a folder of 700 images: open `?view=` of an Entry on the second and the fourth page and reload on it; read the requests | The image shows each time, and the folder's page requests are `offset=0` and one `locate` request |
| 6 | The same with `/search` over that folder from the Library root, and from a subfolder with an Entry in a deeper folder | The image shows each time, with the same requests |
| 7 | Open `?view=` of a name that is not in the folder | The viewer shows "Not found." |
| 8 | In the timeline, choose "Show in folder" for an Entry beyond the first 200 | The folder opens with the Entry selected and scrolled to, and the page requests are `offset=0` and one `locate` request |
| 9 | `! rg -n 'for \(let page' ui-web/src/viewer/viewer.ts ui-web/src/library/useRevealEntry.ts && (cd ui-web && npx vitest run src/viewer src/components/Viewer.test.tsx src/library/timeline.test.ts)` | The search finds nothing, and the viewer and timeline tests pass |
| 10 | `! test -e docs/issues/viewer-url-beyond-first-page-not-found.md && ! rg -n 'viewer-url-beyond-first-page' docs --glob '!docs/plans/**' && ! rg -n 'beyond it is not found' docs/design/client.md && ! rg -n 'not implemented yet' docs/design/listing.md && ! rg -n '^[|] C13 ' docs/design/client.md && rg -n 'M24, which corresponds to C13' docs/design/client.md && rg -n '^[|] L2 .*Done' docs/design/listing.md && rg -n '^[|] M24 .*Done' docs/design.md` | Every command succeeds: the three searches for stale text and the C13 row find nothing, the sentence after the Client Design §6.2 table names M24 as complete, and the L2 and M24 rows are found with Done |
| 11 | `cd daemon && cargo make lint && cd ../ui-web && npm run lint && npm run typecheck && npm test && npm run build` | All exit with 0 |

## 7. Risks and Rollback

The change is additive, and `git revert` of the implementation removes the parameters and the field again.
The web client and the daemon are released together, so reverting both leaves them consistent.
A client other than the web client that starts using `locate` or reading `offset` would stop working after a revert; none exists in this repository, and the bearer authentication for native clients ([design.md §7.2](../design.md#72-sessions)) only prepares for them.
