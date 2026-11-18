# Redesign the Documents Around Shelves

## 1. About This Plan

This work rewrites the design documents so that they describe Pxna browsed through the tree and Shelves, with search and the timeline absorbed into Shelves, one listing API over a Scope and a Filter, a name index that holds the insides of archives, and display settings resolved over three layers.
It changes documents only; the contract and the code follow in later plans.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The order of the rewrite, the questions each step settles, and how to check that the documents agree |
| [design.md §2](../design.md#2-what-pxna-is), [§11.1](../design.md#111-decided), [§11.2](../design.md#112-deferred), [§12.2](../design.md#122-roadmap) | What Pxna is, the cross-cutting decisions this plan assumes, the deferred items, and the roadmap |
| [Listing and Search Design](../design/listing.md#2-responsibilities-and-boundaries) | The name index, recursive listings, search, and the decisions on `listEntries` |
| [Timeline Design](../design/timeline.md#2-responsibilities-and-boundaries) | The timeline index, aggregation, and reading in timeline-date order |
| [Client Design](../design/client.md#2-responsibilities-and-boundaries) | The web client's screens, entrances, and display preferences |
| [Rating and Tag Design](../design/rating-tags.md#2-responsibilities-and-boundaries) | Ratings and tags, and their conditions in listings |
| [terms.md](../terms.md) | Shelf, Scope, Filter, Layout, Unsaved Shelf, Folder settings, and Background work |

[design.md](../design.md#11-division-of-responsibility-between-documents) holds the map of the document family.

### 1.2 Creation Date and Target Commit

Created 2026-11-17 / Target commit `a65c1f9`, with the decisions of §4.1 and their terms added on top of it

## 2. Purpose and Completion Conditions

Someone returning to Pxna, or an AI agent about to write the contract, can read in the design documents one consistent description of browsing through the tree and Shelves, without meeting the separate search mode, the timeline over chosen Libraries, or the rule that indexes do not descend into archives as the current design.

### 2.1 Completion Conditions

1. design.md §2 explains what Pxna is by sovereignty as the foundation, viewing existing folders and archives without importing or waiting as the main strength, and classifying and acting as its extensions, and contrasts Pxna with Immich in facts about Immich's design only
2. Outside rejected alternatives and replaced decisions, no design document describes `listSearchEntries`, `listTimelineEntries`, search mode, the switch of three choices, or a timeline over chosen Libraries as the current design
3. Listing and Search Design describes the name index with the insides of archives, the Scope, the Filter with its kind, and `listEntries` with its paging, and has no deferred item on how `listEntries` cuts pages
4. Timeline Design describes reading in timeline-date order through `listEntries` over a Scope and a Filter, and the per-month counts over the same Scope and Filter
5. design.md describes how Shelves and folder settings are stored, read, and written, how folder settings are inherited and follow the daemon's own moves, and how the default Shelf is created from `timelineLibraries`
6. Client Design describes the entrances of the Libraries, the Shelves, and the settings, the Unsaved Shelf, the editing of Shelves, the Layouts, and the folder settings, and the web client's role of organizing
7. Deferred items exist for reading an archive as a book with its reading position, exporting ratings and tags, and the host on which local AI runs
8. terms.md has no row or individual entry that describes search mode or the timeline as a separate screen, and has boundaries between Shelf and album and between Layout and display mode
9. design.md §12.2 and the roadmaps of the detail documents list the work that brings the code to the rewritten design, in the order of the Shelves, the controlling of background work, and the Trash with exporting

### 2.2 Non-Goals

- Changing `openapi.yaml`, the daemon, or the web client. Each is a later plan built on the rewritten documents.
- Designing the control of background work beyond the principle of [design.md §11.1](../design.md#keep-background-work-quiet-and-under-the-owners-control); it stays the deferred item [Controlling Background Work](../design.md#controlling-background-work).
- Designing the reader for archives, exporting ratings and tags, and local AI beyond their deferred items (condition 7).
- Searching by embedded metadata, which stays the deferred item [Searching by Embedded Metadata](../design.md#searching-by-embedded-metadata).
- The iOS app's documents, which follow in its own repository after this plan.

## 3. Current State

The decisions of §4.1 sit in the decision sections of design.md, Listing and Search Design, Timeline Design, and Client Design, and the new terms are in terms.md.
The bodies of those documents still describe the earlier design.

- design.md §2 explains Pxna by contrast with a NAS and Google Photos, §2.1 says the name index holds only the names of Entries, without the insides of archives that Listing and Search Design §2.1 excludes, and §6.3 describes the timeline over the chosen Libraries.
- Listing and Search Design §2.1 says neither a recursive walk nor the name index descends into archives, §5.3 and the decision "Search Several Libraries by Merging Per-Library Listings on the Server" describe `listSearchEntries`, and §5.1 describes the name index without archives.
- Listing and Search Design §4.5 and the decisions "Locate an Entry from Its Position in the Snapshot" and "Treat Name Search as a Listing Condition" describe locating and name search for folder listings and `listSearchEntries`.
- Timeline Design §1, §2, and §7 and the decisions "List the Chosen Libraries in One Timeline" and "Point at a Page's Position with a Cursor" describe `listTimelineEntries` over chosen Libraries.
- Rating and Tag Design's decision "Do Filtering in Search Mode and Apply It When Cutting the Page" ties rating and tag conditions to search mode, and its deferred item "Filtering in the Timeline, and Sorting by Ratings and Tags" leaves the timeline without them.
- Client Design §3.1, §3.4, §3.5, §3.6, and §3.10 and the decisions on search mode, the toolbar's switch of three choices, the timeline's route and Libraries, and the viewer's URL and `locate` in search mode describe separate screens, and §5.2 still defers filtering listings by kind.
- terms.md defines Search mode, Search listing, and Subtree search, and its Timeline entry describes the chosen Libraries and a screen.
- design.md §12.2 and the roadmaps of the detail documents have no row for the Shelves.

A reader who follows the bodies builds the earlier design, so the contract cannot be written from them yet.

## 4. Change Policy

The bodies are rewritten from the bottom of the dependency up: the listing first, because the timeline reads through it, then the timeline, then the Shelves and folder settings that use both, then the clients' screens, and the vocabulary and the roadmap last.
Each step settles the open questions of its own document and writes them as decisions there, so no question is left for the contract.
A decision that a step replaces stays in its document as a rejected alternative of the new decision, with the reason it gave way.

### 4.1 Decisions Assumed

| Decision | Source |
| --- | --- |
| A feature is in scope only if the owner keeps sovereignty over the files | [design.md §11.1](../design.md#judge-scope-by-whether-the-owner-keeps-sovereignty) |
| Background work is quiet, within a budget, visible, stoppable, and opt-in | [design.md §11.1](../design.md#keep-background-work-quiet-and-under-the-owners-control) |
| The tree is authoritative, and Shelves of a Scope, a Filter, and a Layout are the other way to browse, stored by the daemon | [design.md §11.1](../design.md#browse-through-the-tree-and-shelves) |
| Display settings are resolved over the display preferences, inherited folder settings, and the Shelf | [design.md §11.1](../design.md#resolve-display-settings-over-the-display-preferences-folder-settings-and-the-shelf) |
| The web client organizes, and the iOS app views, reads, classifies in passing, edits Shelves in full, and only deletes | [design.md §11.1](../design.md#organize-on-the-web-client-and-view-on-the-phone) |
| The name index holds the insides of archives | [Listing and Search Design §8.1](../design/listing.md#index-the-entries-inside-archives-too) |
| One listing API, `listEntries`, over a Scope, a Filter, and a sort, with the per-month counts apart | [Listing and Search Design §8.1](../design/listing.md#list-the-entries-of-a-scope-through-one-api) |
| A recursive listing returns folders only when the Filter's kind names them | [Listing and Search Design §8.1](../design/listing.md#list-only-files-recursively-unless-the-filter-asks-for-folders) |
| The timeline is a Layout of a Scope | [Timeline Design §9.1](../design/timeline.md#make-the-timeline-a-layout-of-a-scope) |
| The entrances are the Libraries, the Shelves, and the settings, with an editable default Shelf named Timeline | [Client Design §5.1](../design/client.md#enter-through-the-libraries-and-the-shelves) |
| The Layouts are the grid, the list, and the timeline, and how an archive opens is a setting | [Client Design §5.1](../design/client.md#decide-how-an-archive-opens-by-settings-not-by-layout) |

### 4.2 Scope of Impact

Only `docs/` changes: design.md, `design/listing.md`, `design/timeline.md`, `design/client.md`, `design/rating-tags.md`, terms.md, and the plans list.
No contract, configuration, or stored format changes.
The documents then describe a design whose contract breaks compatibility with the present web client and iOS app by removing `listSearchEntries` and `listTimelineEntries`; that break happens in the later plans.

## 5. Steps

| No. | Content | Depends on | Status |
| --- | --- | --- | --- |
| [1](#s-1) | Rewrite what Pxna is | | Done |
| [2](#s-2) | Rewrite the name index and recursive listings over a Scope and a Filter | | Done |
| [3](#s-3) | Decide the parameters and paging of `listEntries` | 2 | Done |
| [4](#s-4) | Rewrite the timeline as reading in timeline-date order | 3 | Done |
| [5](#s-5) | Describe the storage and API of Shelves and folder settings | 3 | Done |
| [6](#s-6) | Rewrite the web client's screens around the entrances and Shelves | 4, 5 | Done |
| [7](#s-7) | Add the deferred items of the later bets | 1 | Done |
| [8](#s-8) | Bring the vocabulary in line | 6 | Done |
| [9](#s-9) | Rewrite the roadmaps | 6, 7 | Not started |

<a id="s-1"></a>
### Step 1. Rewrite What Pxna Is

**Locations to change**
[design.md §2](../design.md#2-what-pxna-is), [§2.1](../design.md#21-out-of-scope), [§8.9](../design.md#89-search), [§10](../design.md#10-clients)

**What to do**
Rewrite §2 so that sovereignty is the foundation, viewing existing folders and archives without importing or waiting is the main strength, and classifying and acting extend it, each pointing to its decision in §11.1 instead of restating the reason.
Add Immich to the contrast with a NAS and Google Photos, stating only what its own documentation says about its design: it imports or scans into a library and generates thumbnails in jobs over the whole library, and it does not list the insides of archives; check these against Immich's documentation before writing them.
Replace "not entrusting home files to the cloud" with keeping sovereignty, and name the Shelves as the second way to browse.
In §2.1, rewrite the paragraph on the indexes so that the name index holds the insides of archives and metadata search points to its deferred item.
In §8.9, describe search as an Unsaved Shelf over `listEntries`.
In §10, state the web client's role of organizing and point to the iOS app's role in §11.1.

**Completion condition**
§2 links to the five decisions of §11.1 named in §4.1, names Immich with only facts found in its documentation, and no longer contains "entrusting"; §2.1 says the name index holds the insides of archives, and §8.9 describes search as an Unsaved Shelf.

<a id="s-2"></a>
### Step 2. Rewrite the Name Index and Recursive Listings over a Scope and a Filter

**Locations to change**
[Listing and Search Design §2](../design/listing.md#2-responsibilities-and-boundaries), [§2.1](../design/listing.md#21-what-is-not-built), [§5](../design/listing.md#5-name-search), [§5.1](../design/listing.md#51-name-index), [§5.2](../design/listing.md#52-answering-recursive-listings-with-conditions), [§5.3](../design/listing.md#53-listing-several-starts), [§6](../design/listing.md#6-recursion-limit), [§7](../design/listing.md#7-configuration), [§8.1](../design/listing.md#81-decided), [Rating and Tag Design §7.1](../design/rating-tags.md#71-filtering-listings), [§8.1](../design/rating-tags.md#81-decided)

**What to do**
Describe the Scope as one or more starts, each a Library and a Library-relative path, with the choice of descendants and of reading inside archives, and the Filter as name, rating, tags, and kind with the default kind of a recursive listing.
Rewrite §5.1 so that the name index holds the insides of archives down to `max_nested_depth`, and decide what it holds for an archive that cannot be opened and how an archive's rows are replaced when its identity changes.
Estimate the index's growth from the archives of the `contents` test data and from the assumed 10 million Entries, and decide whether its size targets change; if the estimate needs a measurement, file it under `docs/measurements/`.
Decide whether a walk while the name index is not usable descends into archives, and how the recursion limit of §6 counts several starts and the insides of archives.
Replace §5.3 with listing several starts, and rewrite the decisions "Search Several Libraries by Merging Per-Library Listings on the Server" and "Answer Recursive Searches from a Library-Wide Name Index Updated by Aggregation" to the Scope, keeping what still holds and moving what gave way to rejected alternatives.
Rewrite "Treat Name Search as a Listing Condition" to the Filter.
Move the rating and tag conditions of Rating and Tag Design §7.1 to the Filter, and rewrite its decision "Do Filtering in Search Mode and Apply It When Cutting the Page" so that the conditions belong to the Filter of any listing, keeping how they apply when cutting a page.

**Completion condition**
Listing and Search Design §2.1 no longer contains "Neither a recursive walk nor the name index descends into archives", §5 describes the Scope, the Filter, and the index with the insides of archives, every question named above is a decided item in §8.1 with its reason and rejected alternatives, and Rating and Tag Design §7.1 and §8.1 tie the conditions to the Filter instead of search mode.

<a id="s-3"></a>
### Step 3. Decide the Parameters and Paging of `listEntries`

**Locations to change**
[Listing and Search Design §4](../design/listing.md#4-snapshots), [§4.5](../design/listing.md#45-locating-an-entry), [§8.1](../design/listing.md#81-decided), [§8.2](../design/listing.md#82-deferred)

**What to do**
Decide how `listEntries` cuts pages for every sort, including the timeline date, and how a client opens a page around a named Entry and at a month, by settling the conditions of the deferred item "How listEntries Cuts Pages": whether a timeline-date sequence can be cut at any `offset` at the assumed scale, and whether the clients reserve the overall height for every Layout.
Decide how the request carries several starts and the Filter, within the conventions of design.md §4.2, and how the response carries each Entry's Library and its notices of an unusable index and of truncation.
Rewrite §4.5 and "Locate an Entry from Its Position in the Snapshot" for `listEntries`.
Write the result as a decided item in §8.1 with the rejected paging as an alternative, and delete the deferred item.

**Completion condition**
Listing and Search Design §8.2 has no item named "How listEntries Cuts Pages", and §8.1 has a decided item that names the paging of `listEntries` and the request parameters of a Scope and a Filter.

<a id="s-4"></a>
### Step 4. Rewrite the Timeline as Reading in Timeline-Date Order

**Locations to change**
[Timeline Design §1](../design/timeline.md#1-about-this-document), [§2](../design/timeline.md#2-responsibilities-and-boundaries), [§3](../design/timeline.md#3-what-is-listed), [§5](../design/timeline.md#5-the-timeline-index), [§7](../design/timeline.md#7-reading), [§9.1](../design/timeline.md#91-decided), [design.md §6.3](../design.md#63-timeline), [Rating and Tag Design §8.2](../design/rating-tags.md#82-deferred)

**What to do**
Describe what the timeline index answers: `listEntries` sorted by timeline date over a Scope and a Filter whose kind is within images and videos.
Decide how the index narrows to the starts of a Scope, how it applies rating and tag conditions, and how it excludes the insides of archives when the Scope does not read them, and write the per-month counts over the same Scope and Filter.
Rewrite "List the Chosen Libraries in One Timeline" into a rejected alternative of the new decisions, keep "Point at a Page's Position with a Cursor" as a decided item updated for the paging of step 3, which keeps cursors, and rewrite design.md §6.3 to the timeline Layout.
Close Rating and Tag Design's deferred item "Filtering in the Timeline, and Sorting by Ratings and Tags" for filtering, keeping sorting by ratings and tags and any-of tags deferred.

**Completion condition**
Timeline Design §7 describes reading through `listEntries` and the per-month counts over a Scope and a Filter, §9.1 has the decisions on narrowing to starts, rating and tag conditions, and the insides of archives, each with its reason and rejected alternatives, and no decided item named "List the Chosen Libraries in One Timeline"; Rating and Tag Design §8.2 no longer defers filtering in the timeline.

<a id="s-5"></a>
### Step 5. Describe the Storage and API of Shelves and Folder Settings

**Locations to change**
[design.md §6](../design.md#6-thumbnails-and-video), [§8](../design.md#8-conventions-for-mutating-apis), [§9.2](../design.md#92-where-state-lives), [§11.1](../design.md#111-decided)

**What to do**
Add a section to design.md §6 on Shelves and folder settings: their tables in the daemon's database, the read and write API under the conventions of §8, how folder settings are resolved for a folder from the nearest folder above and how `LibraryMutation` moves them with the daemon's own moves and renames, and how the default Shelf is created once from `timelineLibraries`.
Decide whether folder settings, like the display preferences, hold two sets by screen width, and how a Shelf whose start no longer exists is shown.
Add the stored state to §9.2.

**Completion condition**
design.md has a section on Shelves and folder settings that covers their storage, their API, inheritance, the daemon's own moves, the default Shelf, the sets by screen width, and missing starts, §11.1 has the decisions on the sets by screen width and on missing starts and any other choice the section makes, each with its reason and rejected alternatives, and §9.2 lists their state.

<a id="s-6"></a>
### Step 6. Rewrite the Web Client's Screens Around the Entrances and Shelves

**Locations to change**
[Client Design §2](../design/client.md#2-responsibilities-and-boundaries), [§3.1](../design/client.md#31-web), [§3.4](../design/client.md#34-viewer), [§3.5](../design/client.md#35-loading-listings), [§3.6](../design/client.md#36-shelves-and-search), [§3.9](../design/client.md#39-display-preferences), [§3.10](../design/client.md#310-timeline-layout), [§5.1](../design/client.md#51-decided), [§5.2](../design/client.md#52-deferred)

**What to do**
Describe the entrances of the Libraries, the Shelves, and the settings, the Unsaved Shelf opened by search and by search from a folder, the screen that creates and edits a Shelf, the three Layouts, and the folder settings in §3.9.
State in §2 that the web client is where the owner organizes.
Replace §3.6 and §3.10 with the Shelf screen and its timeline Layout, and change §3.5 to read every Shelf through one store.
Rewrite every decided item that names search mode, the toolbar's switch of three choices, or the timeline's route and Libraries, among them those on the last-used conditions, the start chosen only for one Library, narrow screens, the operations on results, and the viewer's URL and `locate`, into rejected alternatives or into decisions on the Shelf screen, and close the deferred item "Whether to Filter Listings by Kind" by the Filter's kind.

**Completion condition**
Client Design has no section or decided item that describes search mode, the toolbar's switch of three choices, or the timeline over chosen Libraries as the current design, describes the entrances, the Unsaved Shelf, Shelf editing, the Layouts, and folder settings, has the decisions on the Shelf screen with their reasons and rejected alternatives, and has no deferred item named "Whether to Filter Listings by Kind".

<a id="s-7"></a>
### Step 7. Add the Deferred Items of the Later Bets

**Locations to change**
[Client Design §5.2](../design/client.md#52-deferred), [Rating and Tag Design](../design/rating-tags.md#8-design-decisions), [design.md §11.2](../design.md#local-ai-for-organization)

**What to do**
Add a deferred item to Client Design for reading an archive as a book, with the reading direction, two-page spreads, and resuming at the last page, and where the reading position is stored.
Add a deferred item to Rating and Tag Design for exporting ratings and tags in a standard form, which the sovereignty decision requires.
Rewrite "Local AI for Organization" so that inference may run on another host the owner controls, and so that it runs under the deferred control of background work.

**Completion condition**
Client Design §5.2 and Rating and Tag Design each have the new deferred item with its conditions for deciding, and "Local AI for Organization" names hosts the owner controls and the control of background work.

<a id="s-8"></a>
### Step 8. Bring the Vocabulary in Line

**Locations to change**
[terms.md §2](../terms.md#2-term-list), [§3](../terms.md#3-boundaries-between-neighboring-terms), [§4](../terms.md#4-individual-terms)

**What to do**
Remove or rewrite Search mode, Search listing, and Subtree search to the Shelf, the Unsaved Shelf, and recursive listings over a Scope, and rewrite the Timeline entry as a Layout.
Add the boundaries between Shelf and album, between Layout and display mode, and between Scope and the term table's own scope column.
Give terms to the names the earlier steps added, such as the API of Shelves, if they are externally visible.

**Completion condition**
terms.md has no row or individual entry that describes search mode or the timeline as a separate screen, and has the three boundaries.

<a id="s-9"></a>
### Step 9. Rewrite the Roadmaps

**Locations to change**
[design.md §12.1](../design.md#121-implementation-status), [§12.2](../design.md#122-roadmap), [Listing and Search Design §9.2](../design/listing.md#92-roadmap), [Timeline Design §10.2](../design/timeline.md#102-roadmap), [Client Design §6.2](../design/client.md#62-roadmap)

**What to do**
Add rows for the name index with the insides of archives, `listEntries` with the per-month counts, the storage and API of Shelves and folder settings, the web client's Shelves, and the removal of the old listing APIs, then for the control of background work, then for exporting ratings and tags after the Trash (M25), each with its dependencies.
State in §12.1 that the documents describe the Shelves while the code has search mode and the timeline over chosen Libraries.

**Completion condition**
design.md §12.2 has rows for each piece of work named above with dependencies that order the Shelves before the control of background work and the Trash before exporting, and §12.1 names the gap.

## 6. Acceptance Checks

| Condition | Command | Expected result |
| --- | --- | --- |
| 1 | `rg -n 'Immich\|sovereignty\|entrusting' docs/design.md` | Immich and sovereignty appear in §2, "entrusting" appears nowhere, and each statement about Immich matches its documentation when read |
| 2 | `rg -n "listSearchEntries\|listTimelineEntries\|[Ss]earch mode\|chosen Libraries\|toolbar's switch" docs/design.md docs/design docs/terms.md` | Every line is inside a **Rejected alternative** block, a decision that names what it replaced, a Current Code section or design.md §12.1, or a done row of a roadmap |
| 3 | `rg -n 'How listEntries Cuts Pages\|^- Neither a recursive walk' docs/design/listing.md` | No match |
| 4 | Read Timeline Design §7 | It reads through `listEntries` and gives the per-month counts over a Scope and a Filter |
| 5 | Read the section of design.md §6 on Shelves and folder settings | It covers storage, API, inheritance, the daemon's own moves, the default Shelf, the sets by screen width, and missing starts |
| 6 | `rg -n 'Whether to Filter Listings by Kind\|Unsaved Shelf\|Folder settings' docs/design/client.md` | No deferred item on filtering by kind, and the Unsaved Shelf and folder settings appear in §3 |
| 7 | `rg -n '^#### ' docs/design/client.md docs/design/rating-tags.md docs/design.md` | The deferred items on reading an archive as a book and on exporting ratings and tags are listed, and "Local AI for Organization" names hosts the owner controls when read |
| 8 | `rg -n 'Search mode\|Search listing\|Subtree search\|Shelf and\|Layout and\|Scope and' docs/terms.md` | No row for search mode, and the three boundaries exist |
| 9 | Read design.md §12.1 and §12.2 | The rows and the gap of step 9 are present, in the stated order |
