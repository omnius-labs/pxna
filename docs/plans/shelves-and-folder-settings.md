# Store the Shelves and the Folder Settings in the Daemon

## 1. About This Plan

This work is the rest of M29 of the roadmap: the daemon stores each user's Shelves and folder settings, serves them through their own requests, creates the default Shelf once, and keeps both in step with its own renames, moves, and deletes; the display preferences also come to hold how an archive opens.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The steps and acceptance checks of storing and serving the Shelves and the folder settings |
| [design.md §6.5](../design.md#65-shelves-and-folder-settings) | The tables, the requests, the checks, the following of the daemon's changes, and the default Shelf |
| [design.md §11.1](../design.md#store-shelves-and-folder-settings-per-user-in-tables-of-the-daemons-database) | The decisions this plan follows and their rejected alternatives |
| [Client Design §3.9](../design/client.md#39-display-preferences) | The display values and how an archive opens in the display preferences |
| [Listing and Search Design §8.1](../design/listing.md#carry-a-scope-and-a-filter-in-the-json-body-of-one-post-request) | The Scope and the Filter that a Shelf holds and sends to `listEntries` |
| [openapi.yaml](../../daemon/openapi.yaml#L241-L291) | The contract this plan extends |

[design.md](../design.md#11-division-of-responsibility-between-documents) holds the map of the document family.

### 1.2 Creation Date and Target Commit

Created 2026-11-26 / Target commit `bc5cf3f`; the decisions of §4.1 not yet in the design documents are written there in the same change as this document

## 2. Purpose and Completion Conditions

A client keeps the owner's Shelves and folder settings in the daemon, so the same ones are read and edited from every device, a new owner finds the Timeline Shelf, and a folder renamed, moved, or deleted through Pxna keeps its settings and its place in the Shelves.

### 2.1 Completion Conditions

1. `GET /preferences` returns `archiveOpen` as it was saved by `PUT /preferences`, `folder` or `viewer`, and without it when it was never saved; another value is rejected with 422 and leaves the saved value
2. The first `GET /shelves` of a user returns the default Shelf named Timeline, recursive, reading the insides of archives, with the kinds `image` and `video` and the timeline Layout in both sets, whose starts are the roots of the configured Libraries named in `timelineLibraries`, or of every configured Library when none is saved, none remains, or the display preferences cannot be read, in the order of the configuration and at most 64; after it is deleted, later reads do not create it again, two first reads at the same time create one, and a read while no Library is configured returns no Shelf and leaves the next read with a Library to create it
3. `POST /shelves` returns 201 with the created Shelf and its id and puts it last, `PUT /shelves/{shelfId}` replaces a Shelf whole and returns it, `DELETE /shelves/{shelfId}` returns 204, both are 404 for an id the user has no Shelf of, `PUT /shelves/order` with every id of the user once reorders and returns the Shelves, and `GET /shelves` returns a user's own Shelves only, in their order
4. Saving a Shelf whose name trimmed is empty or longer than 100 characters, a 201st Shelf of a user, a `PUT /shelves/order` that does not name every Shelf of the user exactly once, and a Scope or Filter that `listEntryMonths` refuses are `InvalidInput`; a start that names nothing, a start given twice, and the timeline with any kinds are saved, and every start is returned by its path in normal form
5. `GET /settings/folder` returns the user's folder settings, `PUT /settings/folder` saves one folder's display values by its path in normal form, replacing what it had, and returns them, the timeline Layout in them is `InvalidInput`, and `POST /settings/folder/delete` returns 204, or 404 when the folder has no settings
6. After a rename or a move through Pxna, in one Library or across Libraries, the folder settings and the Shelf starts at or under the old path are at the new path; a move that replaces its destination leaves no folder settings of the replaced folder and keeps a start that named the destination; after a delete, the folder settings at or under the path are gone and the Shelf starts are kept
7. A request to these paths with a session cookie and an `Origin` matching the `Host` is answered, and one other than GET without an `Origin` is 403
8. `cd daemon && RUSTC_WRAPPER= cargo make lint` and `RUSTC_WRAPPER= cargo test` pass, and the committed interface crate and web client API types are what the contract generates
9. The terms documents name the paths of the Shelves and the folder settings, `ShelfService`, and `archiveOpen`, design.md §12.1 no longer says the code has no Shelves, and M29 is done in §12.2

### 2.2 Non-Goals

- The web client's Shelves, conditions bar, folder settings, and opening of archives, which are M30 and C15 to C17 of [Client Design §6](../design/client.md#6-current-state-and-remaining-work).
- The iOS app's Shelves, which come with its reading of `listEntries`.
- Removing `timelineLibraries` and the timeline APIs, which the web client reads until M30 and M31 replace them.
- Conditional updates by version, a deferred item of [design.md §11.2](../design.md#optimistic-locking-with-etag--if-match).

## 3. Current State

The display preferences are one row of JSON that `PreferencesService` replaces whole, and `Preferences` holds the two sets, the sort, and `timelineLibraries` but not how an archive opens ([preferences.rs#L1-L54](../../daemon/modules/engine/src/service/preferences.rs#L1-L54), [model/preferences.rs#L66-L82](../../daemon/modules/engine/src/model/preferences.rs#L66-L82), [openapi.yaml#L2480-L2558](../../daemon/openapi.yaml#L2480-L2558)); the route maps it field by field ([features/preferences.rs#L68-L110](../../daemon/entrypoints/daemon/src/interface/features/preferences.rs#L68-L110)).
There are no Shelves or folder settings in the code, the contract, or `sqlite.db`.
`RatingTagService` keys its rows by user, Library id, and path, and `moved` and `forget` reassign or remove the rows at or under a path for every user by comparing the prefix `path/` ([rating_tags.rs#L254-L314](../../daemon/modules/engine/src/service/rating_tags.rs#L254-L314)).
`LibraryMutation` calls them after an operation: `renamed` and `moved_across` for renames and moves, and `forget_missing` for the changed Entries that no longer exist ([mutation.rs#L55-L102](../../daemon/entrypoints/daemon/src/service/mutation.rs#L55-L102)).
Every service records its migrations by name in one `_migrations` table of `sqlite.db`, and `DaemonState` creates `AuthService`, whose migration creates `users`, last, after `LibraryMutation` ([state.rs#L86-L131](../../daemon/entrypoints/daemon/src/state.rs#L86-L131), [auth/repo.rs#L55-L62](../../daemon/modules/engine/src/service/auth/repo.rs#L55-L62)).
`listEntryMonths` checks its Scope and Filter with `Scope::from_request` and `ListOptions::from_entry_months`, and `LibraryRelativePath::absolute` gives a path in normal form ([scope.rs#L20-L88](../../daemon/entrypoints/daemon/src/service/listing/scope.rs#L20-L88), [listing.rs#L129-L165](../../daemon/entrypoints/daemon/src/service/listing.rs#L129-L165), [path.rs#L11-L60](../../daemon/entrypoints/daemon/src/service/library/path.rs#L11-L60)).

The design of §6.5 therefore has nothing in the daemon to keep its Shelves and folder settings, and M30 cannot start without it.

## 4. Change Policy

The engine gains a `ShelfService` that owns the three tables and the column of `users`, and holds the Shelves, the folder settings, the default Shelf, and the reassignment by path, as `RatingTagService` holds ratings and tags.
The daemon checks what the engine cannot know, the Scope, the Filter, and the paths, with the checks of `listEntries`, and gives the engine the paths in normal form and the starts of the default Shelf.
The contract gains the Shelves and the folder settings, whose Scope and Filter are the schemas `listEntries` takes, and `LibraryMutation` calls `ShelfService` where it calls `RatingTagService`.

### 4.1 Decisions Assumed

| Decision | Source |
| --- | --- |
| `entries-request-body` is done | [entries-request-body](./entries-request-body.md) |
| Three tables keyed by the user, and the default Shelf recorded on the user's row | [design.md §6.5](../design.md#65-shelves-and-folder-settings), [§11.1](../design.md#store-shelves-and-folder-settings-per-user-in-tables-of-the-daemons-database), [§11.1](../design.md#create-the-default-shelf-once-from-timelinelibraries) |
| The display values in the shape of the display preferences, each optional, the timeline only in a Shelf, and how an archive opens in the display preferences too | [design.md §6.5](../design.md#65-shelves-and-folder-settings), [§11.1](../design.md#hold-folder-settings-and-shelves-in-the-shape-of-the-display-preferences), [Client Design §3.9](../design/client.md#39-display-preferences) |
| The paths and verbs of the Shelves and the folder settings, a Shelf replaced whole, and the order replaced by naming every Shelf once | [design.md §6.5](../design.md#65-shelves-and-folder-settings), [§11.1](../design.md#replace-a-shelf-whole-and-resolve-folder-settings-on-the-client), [§11.1](../design.md#name-a-folder-of-the-folder-settings-in-the-body-of-requests-under-settingsfolder), [§11.1](../design.md#replace-the-order-of-the-shelves-by-naming-every-shelf-once) |
| A Shelf checked as the body of `listEntryMonths`, its starts for form only, its name of 1 to 100 characters, and at most 200 Shelves per user | [design.md §6.5](../design.md#65-shelves-and-folder-settings), [§11.1](../design.md#check-a-shelf-as-the-body-of-listentries-and-its-starts-for-form-only) |
| A Shelf's id a TSID string, 201 for a created Shelf, and `NotFound` for a Shelf or folder settings that do not exist | [design.md §6.5](../design.md#65-shelves-and-folder-settings) |
| Starts and folders stored by their paths in normal form | [design.md §11.1](../design.md#store-starts-and-folders-by-the-normal-form-of-their-paths) |
| Renames and moves reassign both, a replacing move removes the destination's folder settings, and a delete removes folder settings only | [design.md §6.5](../design.md#65-shelves-and-folder-settings), [§11.1](../design.md#move-folder-settings-and-shelf-starts-with-the-daemons-own-changes) |
| No version: the later write wins | [design.md §11.2](../design.md#optimistic-locking-with-etag--if-match) |

### 4.2 Scope of Impact

[openapi.yaml](../../daemon/openapi.yaml#L241-L291) gains `archiveOpen` in `Preferences`, the paths `/shelves`, `/shelves/{shelfId}`, `/shelves/order`, `/settings/folder`, and `/settings/folder/delete`, and their schemas, so the generated interface crate and `ui-web/src/api/schema.d.ts` change; the engine gains `service/shelves.rs` and its model, and `model/preferences.rs` changes; the daemon gains `interface/features/shelves.rs`, and `interface/features/preferences.rs`, `service/mutation.rs`, and `state.rs` change.
`sqlite.db` gains the tables `shelves`, `shelf_starts`, and `folder_settings` and the column `users.default_shelf_created_at`, which a daemon of an earlier commit ignores.
No existing request or response changes its meaning, so compatibility holds; the display preferences saved before read back without `archiveOpen`.

## 5. Steps

| No. | Content | Depends on | Status |
| --- | --- | --- | --- |
| [1](#s-1) | Hold how an archive opens in the display preferences | - | Done |
| [2](#s-2) | Store the Shelves and create the default Shelf | - | Not started |
| [3](#s-3) | Store the folder settings and reassign both by path | 2 | Not started |
| [4](#s-4) | Serve the Shelves and the folder settings | 1, 3 | Not started |
| [5](#s-5) | Follow the daemon's own changes | 4 | Not started |
| [6](#s-6) | Describe the state | 5 | Not started |

<a id="s-1"></a>
### Step 1. Hold How an Archive Opens in the Display Preferences

**Locations to change**
[openapi.yaml#L2480-L2558](../../daemon/openapi.yaml#L2480-L2558), [model/preferences.rs#L66-L150](../../daemon/modules/engine/src/model/preferences.rs#L66-L150), [features/preferences.rs#L68-L262](../../daemon/entrypoints/daemon/src/interface/features/preferences.rs#L68-L262), and the generated interface crate and `ui-web/src/api/schema.d.ts`.

**What to do**
Add to the contract the schema `ArchiveOpen` with the values `folder` and `viewer`, and to `Preferences` the optional `archiveOpen` of it, described as opening as a folder when absent.
Give `Preferences` of the engine an optional value of it that is read as absent from a value saved before it, and map it in the route both ways.
Regenerate the interface crate and the web client's API types.

**Completion condition**
Tests show `archiveOpen` saved and read back for both values, absent when never saved and when read from a value saved before it, and another value rejected with 422; the existing tests of the display preferences pass.

<a id="s-2"></a>
### Step 2. Store the Shelves and Create the Default Shelf

**Locations to change**
New `daemon/modules/engine/src/service/shelves.rs` (the `ShelfService`) and `daemon/modules/engine/src/model/shelf.rs` (the Shelf and the display values), exported from [service.rs#L1-L13](../../daemon/modules/engine/src/service.rs#L1-L13) and [model.rs#L1-L17](../../daemon/modules/engine/src/model.rs#L1-L17).

**What to do**
Model a Shelf as its id, name, Scope (starts of a Library id and a path, `recursive`, `insideArchives`), Filter (`query`, `rating`, `tags`, `kinds`), and display values, and the display values as an optional set per screen of an optional Layout of `grid`, `list`, or `timeline`, tile size of 1 to 5, and thumbnail shape, an optional sort of the display preferences' shape, and an optional `archiveOpen`.
Give `ShelfService` a migration that creates `shelves` (id, user, name, position, the Scope's two choices, the Filter and the display values as JSON), `shelf_starts` (Shelf, position, Library id, path, keyed by the Shelf and the position, indexed by the Library id and the path), and `folder_settings` (user, Library id, path, the display values as JSON, keyed by the three), and adds `default_shelf_created_at` to `users`; the service needs `users`, so its tests create `AuthService` on the same database first.
Read a user's Shelves in their order; create one with a TSID id, last; replace one whole, keeping its id and position; delete one with its starts; and replace the order from a list of ids; a name trimmed of surrounding whitespace outside 1 to 100 characters, a 201st Shelf, a list of ids that does not name every Shelf of the user once, and a tile size out of range are `InvalidInput`, and an id the user has no Shelf of is `NotFound`.
On reading, when the user's `default_shelf_created_at` is unset and the caller gives at least one start, set it and create the default Shelf of §2.1 condition 2 from the given starts in one transaction whose first statement is the update of that column, so a second reader finds it set; with no start given, create and record nothing.

**Completion condition**
Unit tests show the cases of §2.1 conditions 2 to 4 that the engine decides: the default Shelf created once from the given starts, not again after its deletion, once under two concurrent first reads, and not with no start; creating last, replacing whole, deleting, reordering, the order list cases, the name and count limits, `NotFound`, and Shelves kept apart by user.

<a id="s-3"></a>
### Step 3. Store the Folder Settings and Reassign Both by Path

**Locations to change**
`daemon/modules/engine/src/service/shelves.rs` and its tests.

**What to do**
Read a user's folder settings, put one folder's display values replacing what it had, and remove one folder's, which is `NotFound` when it has none; display values with the timeline Layout are `InvalidInput` here.
Add `moved`, which for every user first removes the folder settings at or under the destination, then reassigns the folder settings and the Shelf starts at or under the source to the destination, across Libraries too, and `forget`, which removes the folder settings at or under a path and keeps the starts; both compare the path or its prefix followed by `/`, as `RatingTagService::moved` and `forget` do.

**Completion condition**
Unit tests show putting, replacing, reading, and removing folder settings, the timeline refused, `NotFound` on removing, a move reassigning settings and starts at and under the path but not a sibling whose name only begins with it, across Libraries, a replacing move dropping the destination's settings and keeping a start that named it, and `forget` removing settings and keeping starts.

<a id="s-4"></a>
### Step 4. Serve the Shelves and the Folder Settings

**Locations to change**
[openapi.yaml#L241-L291](../../daemon/openapi.yaml#L241-L291) and its schemas, [state.rs#L86-L146](../../daemon/entrypoints/daemon/src/state.rs#L86-L146), [features.rs#L1-L16](../../daemon/entrypoints/daemon/src/interface/features.rs#L1-L16), new `daemon/entrypoints/daemon/src/interface/features/shelves.rs` and its tests, and the generated interface crate and `ui-web/src/api/schema.d.ts`.

**What to do**
Add to the contract, under the tag `Shelves`, the schemas `DisplayLayout` (`grid`, `list`, `timeline`), `DisplayValueSet` (optional `view`, `tileSize` of 1 to 5, `thumbnailShape`), `DisplayValues` (optional `wide`, `narrow`, `sort` as `SortPreference`, `archiveOpen`), `ShelfInput` (required `name` and `scope` as `EntryScope`, optional `filter` as `EntryFilter` and `display`), `Shelf` (`id` and the fields of `ShelfInput`, `filter` and `display` always present), `ShelfList`, `ShelfOrder` (required `ids` of at most 200), `FolderSetting` (required `libraryId`, `path`, and `display`), and `FolderSettingList`, each closed to other properties as `Preferences` is, and say in the descriptions of `Shelf` and `FolderSetting`, and of `LibraryPath`, which keeps a start of a listing as given, that a Shelf's starts and a folder are returned by their paths in normal form; and the operations `GET /shelves` (200 `ShelfList`), `POST /shelves` (201 `Shelf`), `PUT /shelves/{shelfId}` (200 `Shelf`, 404), `DELETE /shelves/{shelfId}` (204, 404), `PUT /shelves/order` (200 `ShelfList`), `GET /settings/folder` (200 `FolderSettingList`), `PUT /settings/folder` (200 `FolderSetting`), and `POST /settings/folder/delete` taking a `LibraryPath` (204, 404), each with 400 where it takes a body, 401, and 500.
Regenerate the interface crate and the web client's API types.
Create the services in the order `AuthService`, `ShelfService`, then `LibraryMutation`, because the migration of `ShelfService` alters `users` and step 5 gives `LibraryMutation` the `ShelfService`, passing `AuthService::new` clones of the database pool, the clock, and the TSID provider, which the later services still use, and keep `ShelfService` in `DaemonState`.
Implement the routes with the user of the claims: before saving a Shelf, check its Scope and Filter with `Scope::from_request` and `ListOptions::from_entry_months` on the body they make, and turn each start's path into normal form with `LibraryRelativePath::absolute`; before saving or removing folder settings, check the Library id with `LibraryConfig::is_valid_id` and turn the path into normal form; for `GET /shelves`, give as the default Shelf's starts the roots of the configured Libraries that `timelineLibraries` names, or of every configured Library when it is absent, names none of them, or the display preferences cannot be read, in the order of the configuration and at most 64.

**Completion condition**
Route tests show §2.1 conditions 2 to 5 through the HTTP router, including the default Shelf's starts from the display preferences and the configuration, with display preferences that cannot be read, and the `InvalidInput` of a Scope or Filter that `listEntryMonths` refuses, and condition 7 for each operation other than GET; regenerating the interface crate and the web client's API types leaves no difference.

<a id="s-5"></a>
### Step 5. Follow the Daemon's Own Changes

**Locations to change**
[mutation.rs#L26-L102](../../daemon/entrypoints/daemon/src/service/mutation.rs#L26-L102), [state.rs#L86-L146](../../daemon/entrypoints/daemon/src/state.rs#L86-L146), and the tests of `interface::features::shelves`.

**What to do**
Give `LibraryMutation` the `ShelfService`, and call its `moved` from `moved_across` and its `forget` from `forget_missing`, next to the calls of `RatingTagService`, logging a failure without failing the operation, as those calls do.

**Completion condition**
Route tests that rename, move in one Library, move across Libraries, move over an existing folder with the conflict policy that replaces it, and delete through the HTTP router show §2.1 condition 6 by `GET /settings/folder` and `GET /shelves`.

<a id="s-6"></a>
### Step 6. Describe the State

**Locations to change**
[terms.md §2](../terms.md#2-term-list), [design.md §12.1](../design.md#121-implementation-status), [§12.2](../design.md#122-roadmap), and [plans.md](../plans.md).

**What to do**
Add `/api/v1/shelves` and `ShelfService` to the row of Shelf, `/api/v1/settings/folder` to the row of Folder settings, whose scope becomes `all`, and `archiveOpen` to the row of Display preferences in the terms list, describe in design.md §12.1 that the daemon stores and serves the Shelves and the folder settings while no client reads them yet, mark M29 done in §12.2, and advance this plan's row.

**Completion condition**
The terms list names the two paths, `ShelfService`, and `archiveOpen`, Folder settings is of the scope `all`, §12.1 no longer says the code has no Shelves, and M29 is done.

## 6. Acceptance Checks

| Condition | Command | Expected result |
| --- | --- | --- |
| 1 | `cd daemon && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon interface::features::preferences && RUSTC_WRAPPER= cargo test -p omnius-pxna-engine model::preferences` | The tests of step 1 pass: both values round-trip, absent when never saved or saved before, and another value is 422 |
| 2 to 5 | `cd daemon && RUSTC_WRAPPER= cargo test -p omnius-pxna-engine service::shelves` | The unit tests of steps 2 and 3 pass |
| 2 to 7 | `cd daemon && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon interface::features::shelves` | The route tests of steps 4 and 5 pass: the default Shelf, the operations and their errors, the checks, the folder settings, the following of renames, moves, and deletes, and the `Origin` check |
| 8 | `cd daemon && RUSTC_WRAPPER= cargo make lint && RUSTC_WRAPPER= cargo test` | Both exit 0 |
| 8 | `bash daemon/entrypoints/gen-interface.sh && git diff --exit-code -- daemon/entrypoints/interface` | Exits 0, as the CI job `interface` checks |
| 8 | `cd ui-web && npm run gen:api && git diff --exit-code -- src/api && npm run typecheck && npm test` | Exits 0, as the CI job `ui` checks |
| 9 | `rg -n -e api/v1/shelves -e api/v1/settings/folder -e ShelfService -e archiveOpen docs/terms.md` | Finds the two paths, `ShelfService`, and `archiveOpen` |
| 9 | `rg -n 'has no Shelves' docs/design.md` | Finds nothing |
| 9 | `rg -n '^. M29 ' docs/design.md` | The row of M29 ends in `Done` |

The new tests protect the following.

- How an archive opens: saved, read back, and absent from older values
- The default Shelf: its starts, created once, under concurrent first reads, and with no Library
- The Shelves: their operations, order, limits, checks, and separation by user
- The folder settings: their operations and the refused timeline
- The following of changes: renames, moves in and across Libraries, replacing moves, and deletes
- The method: each mutating operation meeting the `Origin` check
