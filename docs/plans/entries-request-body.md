# Carry the Scope and the Filter of `listEntries` in a POST Body

## 1. About This Plan

This work is L6 of Listing and Search Design and the first part of M29 of the roadmap: `listEntries` and `listEntryMonths` take their Scope and Filter in a JSON body of a POST request, with every start and Entry named as an object of a Library id and a Library-relative path, so that a saved Shelf can be sent as it is held.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The steps and acceptance checks of moving the request of `listEntries` and `listEntryMonths` into a POST body |
| [Listing and Search Design §3](../design/listing.md#3-sorting-and-filtering), [§4.5](../design/listing.md#45-locating-an-entry), [§5.3](../design/listing.md#53-listing-several-starts) | The body, `locate`, and the starts the response names |
| [Listing and Search Design §8.1](../design/listing.md#carry-a-scope-and-a-filter-in-the-json-body-of-one-post-request) | The decision this plan follows and its rejected alternatives |
| [openapi.yaml](../../daemon/openapi.yaml#L675-L980) | The contract this plan changes |
| [design.md §8.5](../design.md#85-csrf) | The `Origin` check that a POST with a session cookie meets |

[design.md](../design.md#11-division-of-responsibility-between-documents) holds the map of the document family.

### 1.2 Creation Date and Target Commit

Created 2026-11-24 / Target commit `7f9aadd`, the fold of the plans of M27 and M28 on top of `aae0764`; the decisions of §4.1 are written in the design documents in the same change as this document

## 2. Purpose and Completion Conditions

A client reads a Scope and its months by sending the Scope and the Filter as JSON in the shapes a Shelf holds them, names every start and Entry by a Library id and a path, and gets the same Entries, counts, and cursors as before.

### 2.1 Completion Conditions

1. `POST /entries` and `POST /entries/months` answer every request that `GET /entries` and `GET /entries/months` answered, except a start whose Library id is not of the form of `LibraryId`, which was a missing start and is `InvalidInput` (condition 4), with the same Entries, `totalCount`, `offset`, `snapshot`, cursors, lists of starts, and months, the route tests of both carried over with their expectations unchanged
2. The body holds `scope` (`starts`, `recursive`, `insideArchives`) and `filter` (`query`, `rating`, `tags`, `kinds`), and `listEntries` also the sort, `offset`, `limit`, `locate`, `after`, `before`, `month`, and `around`; every start, `locate`, `around`, and every start in `cutOffStarts`, `walkedStarts`, and `missingStarts` is an object of `libraryId` and `path`, and a start is named in a response with the `path` it was given
3. `GET /entries` and `GET /entries/months` no longer exist
4. No start, more than 64 starts, a start or `locate` or `around` whose `libraryId` is not of the form of `LibraryId` or whose `path` is not a Library-relative path, and the cases that were `InvalidInput` before are `InvalidInput`
5. A request with a session cookie and an `Origin` matching the `Host` is answered, and one without an `Origin` is 403, as for any other POST
6. `cd daemon && RUSTC_WRAPPER= cargo make lint` and `RUSTC_WRAPPER= cargo test` pass, and the committed interface crate and web client API types are what the contract generates

### 2.2 Non-Goals

- The Shelves and the folder settings, which are the rest of M29 and reuse the schemas this plan adds.
- Reading `listEntries` from the web client or the iOS app, which are M30 and the iOS app's I6; no client calls it yet, so no one depends on the GET form.
- Changing what `listEntries` answers: the Scope, the Filter, the orders, and the pages keep their meaning.

## 3. Current State

`listEntries` is `GET /entries` and `listEntryMonths` is `GET /entries/months`, whose queries carry each start as a repeated `start` of a Library id followed by a path, `locate` and `around` in the same form, and the Filter as `query`, `rating`, repeated `tag`, and repeated `kind` ([openapi.yaml#L675-L980](../../daemon/openapi.yaml#L675-L980)); `EntryListing` names the starts cut off, walked, and missing by those strings ([openapi.yaml#L3330-L3392](../../daemon/openapi.yaml#L3330-L3392)).
The routes parse the starts with `Scope::parse`, `locate` and `around` with `ScopeStart::parse`, and the rest with `ListOptions::from_entries` and `timeline_anchor` ([entries.rs#L26-L134](../../daemon/entrypoints/daemon/src/interface/features/entries.rs#L26-L134), [listing.rs#L106-L135](../../daemon/entrypoints/daemon/src/service/listing.rs#L106-L135)), and their tests send GET requests with query strings ([entries.rs#L221-L270](../../daemon/entrypoints/daemon/src/interface/features/entries.rs#L221-L270)).
`ScopeStart` keeps the string it was parsed from as its value ([scope.rs#L10-L80](../../daemon/entrypoints/daemon/src/service/listing/scope.rs#L10-L80)), and that value is what `resolve_scope` names a missing start by and what `ScopeListing` and `ScopeTimelinePage` list the starts by ([library.rs#L401-L460](../../daemon/entrypoints/daemon/src/service/library.rs#L401-L460), [scope_results.rs#L14-L72](../../daemon/entrypoints/daemon/src/service/listing/scope_results.rs#L14-L72), [timeline/scope.rs#L115-L131](../../daemon/entrypoints/daemon/src/service/timeline/scope.rs#L115-L131)).
The `Origin` check is a daemon layer outside the generated router that applies to every request other than GET and HEAD with a session cookie ([design.md §8.5](../design.md#85-csrf)).

The request therefore names starts and Entries by strings that the server splits and that a client would build from each saved Shelf.

## 4. Change Policy

A start becomes a pair of a Library id and a Library-relative path inside the service, kept as given so a response can name it back, and the lists of starts hold those pairs.
The contract then replaces both GET operations by POST operations with a JSON body whose `scope` and `filter` are schemas of their own, which the Shelves of M29 will hold, and every start and Entry it names is one schema of a Library id and a path.

### 4.1 Decisions Assumed

| Decision | Source |
| --- | --- |
| `listEntries` and `listEntryMonths` are POST requests with the Scope and the Filter in a JSON body, and starts and Entries are objects of a Library id and a path | [Listing and Search Design §3](../design/listing.md#3-sorting-and-filtering), [§8.1](../design/listing.md#carry-a-scope-and-a-filter-in-the-json-body-of-one-post-request) |
| The paths stay `/entries` and `/entries/months` | [Listing and Search Design §8.1](../design/listing.md#serve-the-listing-at-entries-and-its-per-month-counts-at-entriesmonths) |
| A start whose Library id is not of the form of a configured one or whose path is invalid is `InvalidInput`, and at most 64 starts; a path has its leading `/` | [Listing and Search Design §8.1](../design/listing.md#treat-a-start-that-names-a-plain-file-as-missing), [§8.1](../design/listing.md#accept-at-most-64-starts) |
| A start is named in the response as it was given, and one given twice once | [Listing and Search Design §4.2](../design/listing.md#42-extracting-a-page), [§5.3](../design/listing.md#53-listing-several-starts) |

### 4.2 Scope of Impact

[openapi.yaml](../../daemon/openapi.yaml#L675-L980) replaces `GET /entries` and `GET /entries/months` with `POST /entries` and `POST /entries/months` and adds their request schemas, so the generated interface crate and `ui-web/src/api/schema.d.ts` change; `service/listing/scope.rs`, `service/listing/scope_results.rs`, `service/listing.rs`, `service/library.rs`, `service/timeline/scope.rs`, `interface/features/entries.rs`, their tests, Listing and Search Design §9, and design.md §12.1 change.
The contract of `listEntries` and `listEntryMonths` breaks, which no client depends on yet; the other APIs, the configuration, and the stored formats do not change.

## 5. Steps

| No. | Content | Depends on | Status |
| --- | --- | --- | --- |
| [1](#s-1) | Name starts by a Library id and a path in the service | - | Not started |
| [2](#s-2) | Take the Scope and the Filter in a POST body | 1 | Not started |
| [3](#s-3) | Describe the state | 2 | Not started |

<a id="s-1"></a>
### Step 1. Name Starts by a Library Id and a Path in the Service

**Locations to change**
[scope.rs#L10-L80](../../daemon/entrypoints/daemon/src/service/listing/scope.rs#L10-L80), [library.rs#L401-L460](../../daemon/entrypoints/daemon/src/service/library.rs#L401-L460), [scope_results.rs#L14-L72](../../daemon/entrypoints/daemon/src/service/listing/scope_results.rs#L14-L72), [timeline/scope.rs#L115-L131](../../daemon/entrypoints/daemon/src/service/timeline/scope.rs#L115-L131), and [entries.rs#L26-L134](../../daemon/entrypoints/daemon/src/interface/features/entries.rs#L26-L134).

**What to do**
Give `ScopeStart` a constructor from a Library id and a Library-relative path that keeps the path as given, rejects a Library id not of the form the configuration accepts and an invalid path with `InvalidInput`, and compares two starts by the Library id and the path as given; give `Scope` a constructor from such starts with the limit of 1 to 64.
Make `resolve_scope`, `ScopeListing`, and `ScopeTimelinePage` name the starts cut off, walked, and missing by `ScopeStart`, or by a pair of a Library id and a path, instead of a string, a start given twice named once as now.
Keep the routes answering as now, building the starts from the query strings and turning the named starts back into those strings, so that this step changes no answer.

**Completion condition**
Unit tests show the constructor's `InvalidInput` cases, a start named back with the path as given, and a missing start given twice named once, and the existing tests of `service::listing`, `service::library`, `service::timeline`, and `interface::features::entries` pass with their expectations unchanged, test helpers that return the named starts adjusted to the new type.

<a id="s-2"></a>
### Step 2. Take the Scope and the Filter in a POST Body

**Locations to change**
[openapi.yaml#L675-L980](../../daemon/openapi.yaml#L675-L980), [openapi.yaml#L3304-L3392](../../daemon/openapi.yaml#L3304-L3392), [listing.rs#L106-L150](../../daemon/entrypoints/daemon/src/service/listing.rs#L106-L150), [scope.rs#L10-L80](../../daemon/entrypoints/daemon/src/service/listing/scope.rs#L10-L80), and [entries.rs#L1-L1168](../../daemon/entrypoints/daemon/src/interface/features/entries.rs#L1-L1168) with its tests; the generated interface crate under `daemon/entrypoints/interface` and the generated `ui-web/src/api/schema.d.ts` are written by `daemon/entrypoints/gen-interface.sh` and `npm run gen:api`.

**What to do**
Add to the contract the schema `LibraryPath` with the required `libraryId` (`LibraryId`) and `path` (`RelativePath`, with its leading `/`, `/` for the root), `EntryScope` with `starts` (1 to 64 `LibraryPath`) and the optional `recursive` and `insideArchives`, `EntryFilter` with the optional `query`, `rating`, `tags` (`TagName` items), and `kinds`, `EntryListRequest` with the required `scope` and the optional `filter`, `sort`, `order`, `foldersFirst`, `offset`, `limit`, `locate` (`LibraryPath`), `after`, `before`, `month`, and `around` (`LibraryPath`), with the values and ranges the query parameters have now, and `EntryMonthsRequest` with the required `scope` and the optional `filter`.
Replace `GET /entries` by `POST /entries` taking `EntryListRequest` and `GET /entries/months` by `POST /entries/months` taking `EntryMonthsRequest`, keeping the operation ids, the responses, and the descriptions with the parameters renamed, and make the three lists of `EntryListing` arrays of `LibraryPath`.
Regenerate the interface crate and the web client's API types, and make `ListOptions::from_entries`, `ListOptions::from_entry_months`, `timeline_anchor`, and the routes read the body; remove `ScopeStart::parse` and `Scope::parse`, which nothing calls afterwards.
Carry every route test of `interface::features::entries` over to POST requests with the same Scopes, Filters, and expectations, the starts and Entries as objects, and add tests for the `InvalidInput` cases of condition 4, for the GET operations no longer existing, and for a request with a session cookie with and without a matching `Origin`.

**Completion condition**
The route tests show conditions 1 to 5, and regenerating the interface crate and the web client's API types leaves no difference.

<a id="s-3"></a>
### Step 3. Describe the State

**Locations to change**
[Listing and Search Design §9.1](../design/listing.md#91-current-code), [§9.2](../design/listing.md#92-roadmap), and [design.md §12.1](../design.md#121-implementation-status).

**What to do**
Describe in Listing and Search Design §9.1 that `listEntries` and `listEntryMonths` are POST requests with the Scope and the Filter in the body, mark L6 done, and remove from the gap of design.md §12.1 that the Scope and the Filter are carried in the query.

**Completion condition**
§9.1 names `POST /entries` and `POST /entries/months`, L6 is done, and §12.1 no longer names the query.

## 6. Acceptance Checks

| Condition | Command | Expected result |
| --- | --- | --- |
| 1 to 5 | `cd daemon && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon interface::features::entries` | The route tests of step 2 pass: every carried-over test with its former expectations, the objects in the body and the response, the `InvalidInput` cases, the GET operations gone, and the `Origin` check on a cookie request |
| 2, 4 | `cd daemon && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon service::listing && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon service::library` | The unit tests of step 1 pass: the constructor's cases and the starts named back as given |
| 6 | `cd daemon && RUSTC_WRAPPER= cargo make lint && RUSTC_WRAPPER= cargo test` | Both exit 0 |
| 6 | `bash daemon/entrypoints/gen-interface.sh && git diff --exit-code -- daemon/entrypoints/interface` | Exits 0, as the CI job `interface` checks |
| 6 | `cd ui-web && npm run gen:api && git diff --exit-code -- src/api && npm run typecheck && npm test` | Exits 0, as the CI job `ui` checks |

The new tests protect the following.

- The starts: built from a Library id and a path, kept as given, and named back once
- The body: every carried-over request answering as before, the objects, and the `InvalidInput` cases
- The method: the GET operations gone, and a cookie request meeting the `Origin` check
