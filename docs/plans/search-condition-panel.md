# Search Across Libraries with Conditions in the Left Area

## 1. About This Plan

This work adds a search API that spans Libraries, and rebuilds the web client's search mode so that its conditions sit in the left area in place of the tree and so that folder view, the timeline, and search mode are three choices of one switch.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The steps and acceptance checks of the search API across Libraries and the new search mode |
| [Listing and Search Design §5.3](../design/listing.md#53-searching-across-libraries) | How the search API merges per-Library listings, its start, its snapshot id, and its limit |
| [Rating and Tag Design §7.1](../design/rating-tags.md#71-filtering-listings) | Rating and tag conditions, accepted only by the search API, with several tags combined as all of them |
| [Client Design §3.6](../design/client.md#36-search) | The URL, the left area, the entrances, the narrow-screen layout, and the operations of search mode |
| [Client Design §3.1](../design/client.md#31-web) | The three-choice switch |
| [daemon/openapi.yaml#L558](../../daemon/openapi.yaml#L558) | The listing contract that `listSearchEntries` sits beside |

[design.md](../design.md#11-division-of-responsibility-between-documents) holds the map of the document family.

### 1.2 Creation Date and Target Commit

Created 2026-10-25 / Target commit `1773c4a`

## 2. Purpose and Completion Conditions

A user can search every Library, or chosen ones, by name, rating, and several tags from one screen, choosing the conditions in the left area, and can move between folder view, the timeline, and search with one switch.

### 2.1 Completion Conditions

1. `GET /search/entries` with two `libraryIds` returns the matching Entries of both Libraries in one sequence, each with its `libraryId`, ordered by the requested sort with ties between Libraries broken by the order of `libraryIds`, with `totalCount` equal to the sum of the per-Library counts and pages at any `offset` that neither repeat nor skip Entries
2. `GET /search/entries` returns 400 for a `path` other than `/` with two Libraries, 404 for an unknown Library id, and lists in `truncatedLibraryIds` exactly the Libraries whose walk reached `recursive_max_entries`
3. With two `tag` values, only images and videos that have both tags are returned, and a rating write changes the response's `snapshot`
4. `GET /libraries/{libraryId}/folders/entries` no longer accepts `rating` or `tag`, and its `query` still filters names
5. `/search` with no query shows every Library checked in the left area and no tree, and checking Libraries, choosing a start from the popover tree, toggling "サブフォルダを含めない", choosing a rating, and checking tags each replace the URL with the matching `lib`, `path`, `flat`, `rating`, and `tag` values and request `listSearchEntries` with them
6. The start can be chosen only while one Library is checked, and checking a second Library returns `path` to the whole Library
7. The toolbar shows "フォルダ", "タイムライン", and "検索" side by side with the current one highlighted, and has no separate search button; "検索" opens the last search of the tab, or every Library with no conditions the first time
8. In folder view, `/` and "このフォルダ内を検索" in the context menus of a tree folder and of empty listing space open `/search` with that Library and folder as the start and no conditions
9. Search results from two Libraries show the Library name and parent path as the location, open images in a viewer that moves among the results, and allow delete, cut, and copy on a selection that spans both Libraries, without new folder, paste, or upload
10. On a narrow screen, search mode shows only the terms field under the header, the other conditions in the drawer, and the number of active non-term conditions on the drawer's button
11. `/search/{libraryId}` and `/browse/...?q=` URLs are replaced by the corresponding `/search` URL, keeping the open result
12. The current-state sections of Listing and Search Design, Rating and Tag Design, and Client Design, and M22 in design.md §12.2, describe this work as implemented
13. `cd daemon && cargo make lint`, the daemon tests, and `cd ui-web && npm run lint && npm run typecheck && npm test` pass

### 2.2 Non-Goals

- Answering the search API from the name index. The `name-index-search` plan does it after this work, as Listing and Search Design §9.2 L1 keeps it.
- Search conditions that match any one of several tags. Rating and Tag Design §8.2 keeps it deferred.
- Filtering the timeline by conditions. Rating and Tag Design §8.2 keeps it deferred.

## 3. Current State

Search runs through the folder listing: [folders.rs#L90](../../daemon/entrypoints/daemon/src/interface/features/folders.rs#L90) looks up the rating and tag sets of one Library and calls [library.rs#L66](../../daemon/entrypoints/daemon/src/service/library.rs#L66), which cuts a page with [snapshot.rs#L120](../../daemon/entrypoints/daemon/src/service/listing/snapshot.rs#L120).
`MarkFilter` at [listing.rs#L108](../../daemon/entrypoints/daemon/src/service/listing.rs#L108) holds one tag, and `TagFilterQuery` at [openapi.yaml#L1963](../../daemon/openapi.yaml#L1963) is a single word on `listFolderEntries` at [openapi.yaml#L558](../../daemon/openapi.yaml#L558).
In the web client, [SearchPage.tsx#L6](../../ui-web/src/pages/SearchPage.tsx#L6) renders `BrowseScreen` at [BrowsePage.tsx#L56](../../ui-web/src/pages/BrowsePage.tsx#L56) with the folder tree, and the conditions are a toolbar row in [SearchBox.tsx#L49](../../ui-web/src/components/SearchBox.tsx#L49) for one Library at `/search/{libraryId}` ([location.ts#L121](../../ui-web/src/location.ts#L121)).
[ModeSwitch.tsx#L20](../../ui-web/src/components/ModeSwitch.tsx#L20) has only folder and timeline, and the timeline already handles selections across Libraries with the `*` scope and keyed paths of [location.ts#L52](../../ui-web/src/location.ts#L52).
The router's [parseSearch at router.tsx#L27](../../ui-web/src/router.tsx#L27) keeps only the last value of a repeated query key.
Search is therefore confined to one Library and one tag, and its conditions do not fit the toolbar row.

## 4. Change Policy

Add `listSearchEntries`, which reuses the per-Library snapshots and merges their filtered orders on each page, and move rating and tag conditions there.
In the web client, give the paged listing store a search source alongside the folder source, build search mode as its own screen on the `*` scope like the timeline, put its conditions in a left-area panel, and make the mode switch three choices.

### 4.1 Decisions Assumed

| Decision | Source |
| --- | --- |
| Merging per-Library listings on the server, the start, the snapshot id, and the per-Library limit | [Listing and Search Design §8.1](../design/listing.md#search-several-libraries-by-merging-per-library-listings-on-the-server) |
| Name search as a listing condition | [Listing and Search Design §8.1](../design/listing.md#treat-name-search-as-a-listing-condition) |
| Several tags combined as all of them | [Rating and Tag Design §8.1](../design/rating-tags.md#combine-several-tag-conditions-as-all-of-them) |
| Conditions in the left area | [Client Design §5.1](../design/client.md#put-the-search-conditions-in-the-left-area-in-place-of-the-tree) |
| The three-choice switch | [Client Design §5.1](../design/client.md#switch-folder-view-the-timeline-and-search-mode-with-one-switch-of-three-choices) |
| The start chosen only for one Library, from a popover tree | [Client Design §5.1](../design/client.md#choose-the-start-only-while-one-library-is-searched-with-a-tree-in-a-popover) |
| Opening with the last conditions, and the folder entrances | [Client Design §5.1](../design/client.md#open-search-mode-with-the-conditions-last-used) |
| The timeline's operations in search results | [Client Design §5.1](../design/client.md#search-results-allow-the-same-operations-as-the-timeline) |
| The narrow-screen layout | [Client Design §5.1](../design/client.md#search-on-narrow-screens-puts-only-the-terms-field-under-the-header) |
| `view` as the Library id and Library-relative path | [Client Design §5.1](../design/client.md#put-the-entry-open-in-the-viewer-in-the-url-as-a-path-relative-to-the-current-folder) |

### 4.2 Scope of Impact

The daemon's listing service, a new search handler, `openapi.yaml`, the generated interface, and the web client's generated schema, router, location helpers, listing store, Grid, List, Viewer, Tree, toolbar, mode switch, and search page change.
`GET /search/entries` and the schemas `SearchListing` and `SearchEntry` are added, and `rating` and `tag` are removed from `listFolderEntries`. This breaks compatibility for clients that filtered folder listings by rating or tag; the bundled web client is the only one and changes in the same work.
The `/search/{libraryId}` URL is replaced by `/search`, and old URLs are redirected.
The terms "Search listing" and "Search mode" are already updated in [terms.md](../terms.md#2-term-list).

## 5. Steps

| No. | Content | Dependencies | State |
| --- | --- | --- | --- |
| [1](#s-1) | Define the search contract and regenerate | - | Done |
| [2](#s-2) | Merge per-Library listings in the daemon | 1 | Done |
| [3](#s-3) | Give the listing store a search source | 1 | Done |
| [4](#s-4) | Move search URLs to `/search` | - | Done |
| [5](#s-5) | Build the search screen and its condition panel | 3, 4 | Done |
| [6](#s-6) | Make the switch three choices and add the folder entrances | 4, 5 | Done |
| [7](#s-7) | Update the current state and the roadmap | 2, 5, 6 | Done |

<a id="s-1"></a>
### Step 1. Define the Search Contract and Regenerate

**Locations to change**
[openapi.yaml#L558](../../daemon/openapi.yaml#L558), [openapi.yaml#L1963](../../daemon/openapi.yaml#L1963), [openapi.yaml#L2884](../../daemon/openapi.yaml#L2884), generated `daemon/entrypoints/interface` and `ui-web/src/api/schema.d.ts`

**What to do**
Add `GET /search/entries` (`listSearchEntries`, tag `Search`) with `libraryIds` (repeated, 1 to 64), optional `path`, `recursive`, `query`, `rating`, `tag` (repeated, up to 32), `sort`, `order`, `foldersFirst`, `offset`, and `limit`, returning `SearchListing` with `entries` of `SearchEntry` (`libraryId`, `entry`), `totalCount`, `snapshot`, and `truncatedLibraryIds`. Remove `rating` and `tag` from `listFolderEntries`. Regenerate with `daemon/entrypoints/gen-interface.sh` and `npm run gen:api`.

**Completion condition**
The generated `apis::search::Search` trait and the `/search/entries` path in `schema.d.ts` exist, and the regeneration leaves no diff when repeated.
This step does not build on its own: the daemon compiles again at step 2, and the web client typechecks again at step 3.

<a id="s-2"></a>
### Step 2. Merge Per-Library Listings in the Daemon

**Locations to change**
[listing.rs#L40](../../daemon/entrypoints/daemon/src/service/listing.rs#L40), [listing.rs#L108](../../daemon/entrypoints/daemon/src/service/listing.rs#L108), [snapshot.rs#L120](../../daemon/entrypoints/daemon/src/service/listing/snapshot.rs#L120), [library.rs#L66](../../daemon/entrypoints/daemon/src/service/library.rs#L66), [folders.rs#L90](../../daemon/entrypoints/daemon/src/interface/features/folders.rs#L90), new `daemon/entrypoints/daemon/src/service/listing/search.rs` that merges the filtered orders of per-Library snapshots, and new `daemon/entrypoints/daemon/src/interface/features/search.rs` that implements the handler

**What to do**
Let `MarkFilter` hold several tags and intersect their sets. Build `ListOptions` from the search query, look up the rating and tag selection per Library, take each Library's snapshot with the existing key, and merge the filtered sorted indices with ties broken by the order of `libraryIds`. Join the per-Library snapshot ids, sum the counts, and collect the truncated Libraries. Stop the folder handler from reading conditions.

**Completion condition**
Daemon tests show conditions 1 to 4 on temporary Libraries.

<a id="s-3"></a>
### Step 3. Give the Listing Store a Search Source

**Locations to change**
[listing.ts#L110](../../ui-web/src/library/listing.ts#L110), [entries.ts#L100](../../ui-web/src/library/entries.ts#L100), [Grid.tsx#L38](../../ui-web/src/components/Grid.tsx#L38), [List.tsx#L51](../../ui-web/src/components/List.tsx#L51), [Viewer.tsx#L67](../../ui-web/src/components/Viewer.tsx#L67)

**What to do**
Replace the filter of the paged store with a source, which is a folder or a search. A search source fetches `listSearchEntries`, turns each Entry's path into the keyed path of the `*` scope, and reports truncation from `truncatedLibraryIds`. Grid, List, and Viewer take the source and a function for the location column. `refreshLibrary` also refreshes search stores. Rename the timeline-only names of the `*` scope (`TIMELINE_SCOPE`, `timelineKey`, the `timeline` flag of the action context) to names that cover both screens.

**Completion condition**
Listing store tests show search pages with keyed paths, snapshot changes rereading, and folder listings unchanged; existing Grid, List, and Viewer tests pass.

<a id="s-4"></a>
### Step 4. Move Search URLs to `/search`

**Locations to change**
[location.ts#L121](../../ui-web/src/location.ts#L121), [router.tsx#L27](../../ui-web/src/router.tsx#L27), [router.tsx#L78](../../ui-web/src/router.tsx#L78), [router.tsx#L112](../../ui-web/src/router.tsx#L112)

**What to do**
Let `parseSearch` and `stringifySearch` keep repeated keys as arrays. Make `SearchLocation` hold `libraryIds`, `path`, `flat`, `query`, `rating`, `tags`, and a keyed `view`, with `searchHref` and `searchLocation` for `/search`. Redirect `/search/{libraryId}` and `/browse/...?q=` to it.

**Completion condition**
Location and router tests show the round trip of every query.
The screens that read the old `SearchLocation` typecheck again at step 5, so this step's own tests are run with `npx vitest run src/router.test.ts src/location.test.ts`. The redirects of condition 11 are tested with the search screen in step 5.

<a id="s-5"></a>
### Step 5. Build the Search Screen and Its Condition Panel

**Locations to change**
[SearchPage.tsx#L6](../../ui-web/src/pages/SearchPage.tsx#L6), [SearchBox.tsx](../../ui-web/src/components/SearchBox.tsx#L49) (replaced), new `ui-web/src/components/SearchPanel.tsx` for the left area with its test `SearchPanel.test.tsx`, new `ui-web/src/pages/SearchPage.test.tsx`, [Tree.tsx#L30](../../ui-web/src/components/Tree.tsx#L30), [AppShell.tsx#L38](../../ui-web/src/components/AppShell.tsx#L38), [Toolbar.tsx#L48](../../ui-web/src/components/Toolbar.tsx#L48), [BrowsePage.tsx#L56](../../ui-web/src/pages/BrowsePage.tsx#L56)

**What to do**
Build the search screen on the `*` scope with the timeline's operations and "フォルダで表示", the empty-condition prompt, and the location column. Build the panel with the debounced terms field, Library checkboxes, the start button with a picking Tree in a popover limited to one Library, the scope checkbox, the rating choice, and tag checkboxes. On narrow screens show the terms row under the header and the rest in the drawer with the count. Remove search handling from `BrowseScreen`, and remember the last search URL of the tab.

**Completion condition**
Component tests show conditions 5, 6, 9, 10, and 11.

<a id="s-6"></a>
### Step 6. Make the Switch Three Choices and Add the Folder Entrances

**Locations to change**
[ModeSwitch.tsx#L20](../../ui-web/src/components/ModeSwitch.tsx#L20) with a new test `ModeSwitch.test.tsx`, [Toolbar.tsx#L141](../../ui-web/src/components/Toolbar.tsx#L141), [TreeMenu.tsx#L9](../../ui-web/src/components/TreeMenu.tsx#L9), [EntryMenu.tsx#L10](../../ui-web/src/components/EntryMenu.tsx#L10)

**What to do**
Add "検索" to the switch with the remembered search as its target, remove the separate search button, keep `/` in folder view as an entrance with the current folder, and add "このフォルダ内を検索" to the tree's folder menu and the listing's empty-space menu.

**Completion condition**
Component tests show conditions 7 and 8.

<a id="s-7"></a>
### Step 7. Update the Current State and the Roadmap

**Locations to change**
[listing.md §9](../design/listing.md#9-current-state-and-remaining-work), [rating-tags.md §9](../design/rating-tags.md#9-current-state-and-remaining-work), [client.md §6](../design/client.md#6-current-state-and-remaining-work), [design.md §12.2](../design.md#122-roadmap), [name-index-search.md](./name-index-search.md)

**What to do**
Rewrite the current-state sentences on search for the new API and screen, mark M22 as done, and point the `name-index-search` plan's listing contract and notice at `listSearchEntries`.

**Completion condition**
`rg -n '/search/\{libraryId\}|filtering conditions on the folder listing' docs/design` returns only the redirect descriptions in Client Design §3.6 and §6.1, and M22 is Done.

## 6. Acceptance Checks

| Condition | Command | Expected result |
| --- | --- | --- |
| 1, 2, 3, 4 | `cd daemon && RUSTC_WRAPPER= cargo nextest run -p omnius-pxna-daemon search listing` | The tests for merged order, totals, paging, path and unknown-Library errors, per-Library truncation, two tags, the snapshot after a rating write, and folder listing names pass |
| 5, 6, 9, 10, 11 | `cd ui-web && npx vitest run src/pages/SearchPage.test.tsx src/components/SearchPanel.test.tsx src/components/AppShell.test.tsx` | The tests for the panel's URL updates, the start rule, the cross-Library results and operations, the narrow layout, and the redirects of old URLs pass |
| 7, 8 | `cd ui-web && npx vitest run src/components/ModeSwitch.test.tsx src/components/AppShell.test.tsx src/pages/BrowsePage.test.tsx src/components/Tree.test.tsx src/components/EntryMenu.test.tsx` | The tests for the three choices, the remembered search, `/`, and the context menu entrances pass |
| 11 | `cd ui-web && npx vitest run src/router.test.ts src/location.test.ts src/pages/SearchPage.test.tsx` | The round-trip and redirect tests pass |
| 12 | `rg -n '/search/\{libraryId\}\|filtering conditions on the folder listing' docs/design` and read design.md §12.2 | Only the redirect descriptions in Client Design §3.6 and §6.1 match, and M22 is Done |
| 13 | `cd daemon && cargo make lint && RUSTC_WRAPPER= cargo nextest run --all-features` and `cd ui-web && npm run lint && npm run typecheck && npm test` | All exit with 0 |

New tests protect the following.
The daemon search tests protect the merged order across Libraries and the per-Library limit.
The tag test protects the all-of-them rule.
The panel tests protect the mapping between the left-area controls and the URL.
The router tests protect old bookmarks.
