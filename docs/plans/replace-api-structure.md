# Replace the API Contract with the Structure of API Design

## 1. About This Plan

This work replaces the HTTP contract under `/api/v1` with the structure of [API Design](../design/api.md#1-about-this-document), area by area. In each step the daemon, `daemon/openapi.yaml`, the generated interface, and the web client change together.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The steps, completion conditions, and scope of this work |
| [API Design §3 to §13](../design/api.md#3-resource-model), [§14](../design/api.md#14-design-decisions) | The target contract and the decisions behind it |
| [API Design §15](../design/api.md#15-current-state-and-remaining-work) | The roadmap rows A1 to A5 that the steps implement, and the statements of other documents each step rewrites |
| [design.md §12.2](../design.md#122-roadmap) | M34, which this work is |
| [daemon/openapi.yaml](../../daemon/openapi.yaml#L1) | The contract each step rewrites |

[design.md](../design.md#11-division-of-responsibility-between-documents) holds the map of the document family.

### 1.2 Creation Date and Target Commit

Created 2026-11-27 / Target commit `09f819a`

## 2. Purpose and Completion Conditions

A client learns one model to use the API: one Entry namespace, one listing request with one page, one result for every batch command, and one error shape. The web client uses only that model, and everything the current contract offers still works.

### 2.1 Completion Conditions

1. `daemon/openapi.yaml` contains none of the current paths in the left column of API Design §13, and contains every path of API Design §4
2. Every error response in `daemon/openapi.yaml` is `application/problem+json`, and an unknown route, a disallowed method, a JSON body the generated router rejects, an unauthenticated request, and a cookie `POST` without `Origin` each answer a Problem body with `code` and `requestId`
3. One `POST /entries/copy`, `/move`, or `/delete` request takes a file and a folder together and answers `202` with an Operation, and a move of one file within its Library answers `202` with `status` already `succeeded`
4. `POST /entries` returns, in the Entry page of API Design §6.3, a folder, the tree with hidden Entries, the summary of direct children, a search, and the timeline
5. `POST /timeline/scans` answers `202` with an Operation of kind `timelineScan`, which `GET /operations/{operationId}` returns
6. A request for a video session that the daemon ended to make way for a newer one answers `410` with `code` `SessionEvicted`, and an unknown session id answers `404`
7. The rating, the tags, and the folder settings of one Entry are read and written under `/libraries/{libraryId}/entries/` with the path in the query, and their `PUT` bodies carry only the value
8. `daemon/openapi.yaml` has the twelve tags of API Design §12 in that order, and every operation has one of them
9. Every API call of the web client names a path of the replaced contract, and in a browser the folder grid, the tree, a search, the timeline, the viewer with a transcoded video, a rating and tags, an upload, and a copy, move, and delete through the activity panel work
10. API Design §15.1 lists no statement of another document, and `docs/design.md`, `docs/design/listing.md`, `docs/design/timeline.md`, `docs/design/rating-tags.md`, `docs/design/video.md`, and `docs/terms.md` name no path that the left column of API Design §13 lists
11. The daemon lint and tests and the web client lint, type check, and tests pass

### 2.2 Non-Goals

- The additions API Design §14.2 defers: `QUERY`, commands across several Libraries, idempotency keys, session and password management, resuming uploads across restarts, and cancelling a timeline scan.
- Persisting Operations and preconditions on writes. They are deferred in [design.md §11.2](../design.md#persisting-operations).
- Changing the identity of ratings and tags. It is M26 of [design.md §12.2](../design.md#122-roadmap).
- Rebuilding the web client's screens around the Shelves. It is M30, which follows this work. Each step moves the current screens to the replaced contract without changing what they show.
- The iOS app. It follows the contract in its own repository.

## 3. Current State

The contract has 18 tags ([openapi.yaml#L42](../../daemon/openapi.yaml#L42)), separate `files` and `folders` namespaces ([openapi.yaml#L702](../../daemon/openapi.yaml#L702), [openapi.yaml#L1335](../../daemon/openapi.yaml#L1335)), and the folder, search, timeline, and Scope listings side by side ([openapi.yaml#L770](../../daemon/openapi.yaml#L770), [openapi.yaml#L815](../../daemon/openapi.yaml#L815), [openapi.yaml#L499](../../daemon/openapi.yaml#L499), [openapi.yaml#L881](../../daemon/openapi.yaml#L881)).
The rating and tags of an Entry are at the Library level ([openapi.yaml#L2035](../../daemon/openapi.yaml#L2035)), and the folder settings name a folder in a body under `/settings/folder` ([openapi.yaml#L427](../../daemon/openapi.yaml#L427)).
Errors are documented as `ErrorResponse` (`daemon/openapi.yaml` at the target commit).

The generated router answers an unauthenticated request with an empty body ([server/mod.rs#L145](../../daemon/entrypoints/interface/src/server/mod.rs#L145)) and a rejected body with plain text ([server/mod.rs#L154](../../daemon/entrypoints/interface/src/server/mod.rs#L154)). Only the 401 gets a JSON body, from the `UnauthorizedBody` middleware outside the router (`middleware/unauthorized.rs` at the target commit, [server.rs#L72](../../daemon/entrypoints/daemon/src/interface/server.rs#L72)).
The interface is generated by `rust-axum` without custom templates ([gen-interface.sh#L9](../../daemon/entrypoints/gen-interface.sh#L9)), and CI regenerates it and checks the diff ([ci.yml#L315](../../.github/workflows/ci.yml#L315)).

A timeline refresh starts a scan unless one is running for the Library, and the scan takes no cancellation ([timeline.rs#L184](../../daemon/entrypoints/daemon/src/service/timeline.rs#L184)).
Creating a video session at the limit ends the oldest session, after which that session is unknown ([video.rs#L290](../../daemon/entrypoints/daemon/src/service/video.rs#L290)).

The web client calls the current paths from `/api/v1` ([client.ts#L29](../../ui-web/src/api/client.ts#L29)) and reads errors as `kind` and `message` ([client.ts#L43](../../ui-web/src/api/client.ts#L43)).
Its listings use the folder, summary, search, and timeline listings ([entries.ts#L138](../../ui-web/src/library/entries.ts#L138), [entries.ts#L151](../../ui-web/src/library/entries.ts#L151), [entries.ts#L227](../../ui-web/src/library/entries.ts#L227), [timeline.ts#L410](../../ui-web/src/library/timeline.ts#L410), [timeline.ts#L497](../../ui-web/src/library/timeline.ts#L497)). Its Entry operations choose a `files` or `folders` route per Entry ([entryActions.ts#L45](../../ui-web/src/library/entryActions.ts#L45), [entryActions.ts#L145](../../ui-web/src/library/entryActions.ts#L145)). It does not call `/entries` or the folder settings.

The same Entries are therefore returned by five listings in four envelopes, and the same concept sits in different places by endpoint, which is what API Design replaces.

## 4. Change Policy

Each step takes one row of API Design §15.2. It rewrites `daemon/openapi.yaml` for that area, regenerates the interface, moves the daemon handlers onto the existing services, moves the web client's calls, and rewrites the statements of other documents that API Design §15.1 lists for that area. An operation a step adds gets its final tag from API Design §12, so the generated module of a handler moves once. No step duplicates path resolution, snapshots, the name index, timeline aggregation, or staging.

### 4.1 Decisions Assumed

| Decision | Source |
| --- | --- |
| Replace the contract in place, area by area, without a second version | [API Design §14.1](../design/api.md#replace-the-contract-in-place-instead-of-serving-a-second-version) |
| One Entry namespace with `view` | [API Design §14.1](../design/api.md#keep-one-entry-namespace-with-a-view-where-a-path-has-two-faces) |
| One Entry under its Library with the path in the query | [API Design §14.1](../design/api.md#address-one-entry-under-its-library-with-the-path-in-the-query) |
| One listing operation into one Entry page | [API Design §14.1](../design/api.md#list-through-one-operation-into-one-entry-page) |
| `202` with an Operation for every batch command, timeline scans included | [API Design §14.1](../design/api.md#answer-every-batch-command-with-202-and-an-operation), [§14.1](../design/api.md#make-a-timeline-scan-an-operation) |
| Problem Details for every error | [API Design §14.1](../design/api.md#use-problem-details-for-every-error) |
| End the oldest video session and answer it with `410` | [API Design §14.1](../design/api.md#end-the-oldest-video-session-and-answer-it-with-410) |
| The body-carrying reads keep the `Origin` check | [API Design §14.1](../design/api.md#use-post-for-body-carrying-reads-check-them-like-any-post-and-defer-query) |

### 4.2 Scope of Impact

The contract, the generated interface, the daemon's interface layer (`interface/features`, `interface/middleware`, `error.rs`), the timeline and video services, the web client's API calls and listing stores, and the design documents change.
The engine, the storage layer, the SQLite tables, and the configuration do not change; the Shelves and the folder settings keep their stored form, because a start and a folder setting are always a folder face.
The API is an externally visible change that breaks compatibility: a client built against the current contract, the iOS app included, fails against the replaced one.
No term is renamed, and `EntryRef` and View are already in [terms.md](../terms.md#2-term-list), but the external representations of Folder settings, Media info, Vocabulary, and Aggregation, and the API names in the Timeline entry, change; steps 2 and 4 update them.

## 5. Steps

| No. | Content | Dependencies | State |
| --- | --- | --- | --- |
| [1](#s-1) | Answer every error with Problem Details | - | Done |
| [2](#s-2) | Move the single-Entry operations into one Entry namespace | - | Done |
| [3](#s-3) | List through `POST /entries` into the Entry page | 2 | Done |
| [4](#s-4) | Run copy, move, delete, and timeline scans as Operations | 2 | Done |
| [5](#s-5) | Answer ended video sessions with `410`, rename the session read, and finish the tags | 2, 3, 4 | Done |

<a id="s-1"></a>
### Step 1. Answer Every Error with Problem Details

**Locations to change**
`ErrorResponse` and the shared error responses of `daemon/openapi.yaml`, [error.rs](../../daemon/entrypoints/daemon/src/error.rs#L1), the `UnauthorizedBody` middleware, which becomes [problem.rs#L17](../../daemon/entrypoints/daemon/src/interface/middleware/problem.rs#L17), [origin.rs#L21](../../daemon/entrypoints/daemon/src/interface/middleware/origin.rs#L21), [server.rs#L72](../../daemon/entrypoints/daemon/src/interface/server.rs#L72), [client.ts#L37](../../ui-web/src/api/client.ts#L37), and [design.md §4.4](../design.md#44-error-model-and-http-mapping).

**What to do**
Replace `ErrorResponse` with a Problem schema served as `application/problem+json`, with `code` taking the values of `ErrorKind`, and set `requestId` on every error. Generalize the 401 middleware into one that gives every error response without the Problem media type a Problem body: the generated router's empty and plain-text rejections and the framework's 404 and 405. Have the web client read `code` and `detail`. Rewrite the plain-text paragraph of design.md §4.4, and remove its row from API Design §15.1.

**Completion condition**
New HTTP tests through the router see a Problem body with `code` and `requestId` for an unknown route, a disallowed method, a malformed JSON body, a missing session, and a cookie `POST` without `Origin`, and the daemon and web client checks pass.

<a id="s-2"></a>
### Step 2. Move the Single-Entry Operations into One Entry Namespace

**Locations to change**
The single-Entry paths of [openapi.yaml#L702](../../daemon/openapi.yaml#L702), [openapi.yaml#L1335](../../daemon/openapi.yaml#L1335), [openapi.yaml#L1882](../../daemon/openapi.yaml#L1882), [openapi.yaml#L2035](../../daemon/openapi.yaml#L2035), [openapi.yaml#L2171](../../daemon/openapi.yaml#L2171), and [openapi.yaml#L427](../../daemon/openapi.yaml#L427); [files.rs](../../daemon/entrypoints/daemon/src/interface/features/files.rs#L1), [folders.rs](../../daemon/entrypoints/daemon/src/interface/features/folders.rs#L1), [rating_tags.rs](../../daemon/entrypoints/daemon/src/interface/features/rating_tags.rs#L1), [shelves.rs](../../daemon/entrypoints/daemon/src/interface/features/shelves.rs#L1), and [video.rs](../../daemon/entrypoints/daemon/src/interface/features/video.rs#L1); [queries.ts#L19](../../ui-web/src/library/queries.ts#L19), [ratingTags.ts#L36](../../ui-web/src/library/ratingTags.ts#L36), [childProbes.ts#L37](../../ui-web/src/library/childProbes.ts#L37), [upload.ts#L55](../../ui-web/src/library/upload.ts#L55), and [entryActions.ts#L28](../../ui-web/src/library/entryActions.ts#L28); [design.md §6.5](../design.md#65-shelves-and-folder-settings), [§8.7](../design.md#87-uploads), and [§11.1](../design.md#111-decided), [Rating and Tag Design §7](../design/rating-tags.md#7-api), and the Folder settings, Media info, and Vocabulary rows of [terms.md](../terms.md#2-term-list).

**What to do**
Replace the file and folder reads with `GET .../entries/metadata?view=`. Move content, one-shot writes with `rename` split into `POST .../entries/files`, folder creation, the child probe, media information, ratings, tags, and folder settings to the paths of API Design §4, add `GET /folder-settings`, and rename `GET /tags` to `GET /tags/words`. Move the web client's calls. Remove the decision on `/settings/folder` from design.md §11.1, whose replacement is in API Design §14.1, and rewrite the paths of design.md §6.5 and §8.7, Rating and Tag Design §7, and the Folder settings, Media info, and Vocabulary rows of terms.md. Remove their entries from API Design §15.1.

**Completion condition**
The current single-Entry paths are absent from `daemon/openapi.yaml`, HTTP tests read an archive as a file and as its root through `view`, and write a rating, tags, and folder settings at the new paths, design.md §6.5 and §8.7, Rating and Tag Design §7, and those three rows of terms.md name only the new paths, and the daemon and web client checks pass.

<a id="s-3"></a>
### Step 3. List Through `POST /entries` into the Entry Page

**Locations to change**
[openapi.yaml#L770](../../daemon/openapi.yaml#L770), [openapi.yaml#L815](../../daemon/openapi.yaml#L815), [openapi.yaml#L499](../../daemon/openapi.yaml#L499), [openapi.yaml#L570](../../daemon/openapi.yaml#L570), [openapi.yaml#L881](../../daemon/openapi.yaml#L881), and the summary and details paths; [entries.rs](../../daemon/entrypoints/daemon/src/interface/features/entries.rs#L1), `features/search.rs`, which this step removes, [folders.rs](../../daemon/entrypoints/daemon/src/interface/features/folders.rs#L1), [timeline.rs](../../daemon/entrypoints/daemon/src/interface/features/timeline.rs#L46), and [listing.rs](../../daemon/entrypoints/daemon/src/service/listing.rs#L1); [entries.ts#L138](../../ui-web/src/library/entries.ts#L138), [entries.ts#L227](../../ui-web/src/library/entries.ts#L227), [entries.ts#L278](../../ui-web/src/library/entries.ts#L278), [timeline.ts#L410](../../ui-web/src/library/timeline.ts#L410), and [listing.ts](../../ui-web/src/library/listing.ts#L1); [Listing and Search Design](../design/listing.md#1-about-this-document) and [Timeline Design](../design/timeline.md#1-about-this-document).

**What to do**
Give `POST /entries` the page union, the options with `includeHidden` and `fields=summary`, and the Entry page of API Design §6. Add `POST /entries/metadata`. Remove the folder, summary, search, and timeline listings, the timeline months, and the details batch. Move the web client's listing stores onto `POST /entries` without changing what the screens show. Rewrite the listing names in Listing and Search Design and the timeline listing and month endpoints in Timeline Design §10.1. Mark L5 and TL3 done there, and remove their entries from API Design §15.1.

**Completion condition**
HTTP tests read a folder, the tree with a hidden Entry, the summary, a search, and the timeline through `POST /entries` in the Entry page. The removed listing paths are absent from `daemon/openapi.yaml` and from Listing and Search Design and Timeline Design, and the daemon and web client checks pass.

<a id="s-4"></a>
### Step 4. Run Copy, Move, Delete, and Timeline Scans as Operations

**Locations to change**
[openapi.yaml#L1479](../../daemon/openapi.yaml#L1479), [openapi.yaml#L1293](../../daemon/openapi.yaml#L1293), the other copy, move, and delete paths, and [openapi.yaml#L625](../../daemon/openapi.yaml#L625); [files.rs](../../daemon/entrypoints/daemon/src/interface/features/files.rs#L1), [folders.rs](../../daemon/entrypoints/daemon/src/interface/features/folders.rs#L1), [timeline.rs](../../daemon/entrypoints/daemon/src/interface/features/timeline.rs#L90), [operation.rs](../../daemon/entrypoints/daemon/src/service/mutation/operation.rs#L1), and [timeline.rs](../../daemon/entrypoints/daemon/src/service/timeline.rs#L224); [entryActions.ts#L45](../../ui-web/src/library/entryActions.ts#L45), [entryActions.ts#L83](../../ui-web/src/library/entryActions.ts#L83), [upload.ts#L135](../../ui-web/src/library/upload.ts#L135), and [timeline.ts#L520](../../ui-web/src/library/timeline.ts#L520); [design.md §8](../design.md#8-conventions-for-mutating-apis), [§11.1](../design.md#decide-sync-and-async-by-the-shape-of-the-request), and [§11.1](../design.md#delete-moves-the-entry-to-a-library-local-trash-unified-on-post), [Timeline Design §6.3](../design/timeline.md#63-when-aggregation-starts) and [§10.1](../design/timeline.md#101-current-code), and the Aggregation row and the Timeline entry of [terms.md](../terms.md#2-term-list).

**What to do**
Add `POST /entries/copy`, `/move`, and `/delete` with the bodies of API Design §7, which answer `202` with an Operation in every case. Add the Operation kind `timelineScan` and `POST /timeline/scans`, whose Operation follows the scan already running for a Library. Remove the six namespace commands and the refresh. Send a mixed selection as one request from the web client. Rewrite the introduction of design.md §8, §8.1, §8.2, and §8.4, and the paths of the decision "Delete Moves the Entry to a Library-Local Trash, Unified on POST", to the one namespace and the single status, and remove the decision "Decide Sync and Async by the Shape of the Request", whose replacement is in API Design §14.1. Rewrite the refresh in Timeline Design §6.3 and §10.1 and in the Aggregation row and the Timeline entry of terms.md. Remove their entries from API Design §15.1.

**Completion condition**
HTTP tests copy, move, and delete a file and a folder in one request each with `202`, see a same-Library move of one file answer `202` with `status` `succeeded`, and read a `timelineScan` Operation. design.md §8 and §11.1, Timeline Design, and terms.md name no namespace command or refresh, and the daemon and web client checks pass.

<a id="s-5"></a>
### Step 5. Answer Ended Video Sessions with `410`, Rename the Session Read, and Finish the Tags

**Locations to change**
[video.rs#L290](../../daemon/entrypoints/daemon/src/service/video.rs#L290), [openapi.yaml#L204](../../daemon/openapi.yaml#L204), [openapi.yaml#L42](../../daemon/openapi.yaml#L42), [session.ts#L8](../../ui-web/src/auth/session.ts#L8), [video.ts#L37](../../ui-web/src/viewer/video.ts#L37), [design.md §4.4](../design.md#44-error-model-and-http-mapping), and [Video Playback Design §5.4](../design/video.md#54-concurrency).

**What to do**
Keep the ids of sessions ended to make way for `session_idle_timeout_secs`, as API Design §9.3 decides, and answer their playlist, segments, and deletion with `410` and `SessionEvicted`. Rename `GET /auth/me` to `GET /auth/session`. Move the remaining operations to the tags of API Design §12 and remove the tags left empty. Have the web client's viewer report an ended session. Add `SessionEvicted` with `410` to the table of design.md §4.4, rewrite Video Playback Design §5.4, and remove the last entries of API Design §15.1 and the not-implemented sentence there.

**Completion condition**
An HTTP test sees `410` with `SessionEvicted` for an ended session and `404` for an unknown one, `daemon/openapi.yaml` has the twelve tags in order, API Design §15.1 lists no statement, and the daemon and web client checks pass.

## 6. Acceptance Verification

| Condition | Command | Expected result |
| --- | --- | --- |
| 1 | `rg -n '^    /(libraries/\{libraryId\}/(files\|folders\|media-info\|rating\|tags\|entries/details)\|search/\|timeline/(entries\|months\|refresh)\|settings/folder\|auth/me\|tags:)' daemon/openapi.yaml` | No output |
| 1 | `rg -o '^    /[^:]+' daemon/openapi.yaml \| sort`, read against the tree of API Design §4 | Every path of the tree is listed |
| 2 | `rg -n 'ErrorResponse' daemon/openapi.yaml`, `rg -c 'application/problem\+json' daemon/openapi.yaml`, and the Problem tests of step 1 in `cd daemon && RUSTC_WRAPPER= cargo make test` | No `ErrorResponse`, a non-zero count, and the tests pass |
| 3 | The copy, move, and delete tests of step 4 in `cd daemon && RUSTC_WRAPPER= cargo make test` | Pass |
| 4 | The listing tests of step 3 in `cd daemon && RUSTC_WRAPPER= cargo make test` | Pass |
| 5 | The `timelineScan` test of step 4 in `cd daemon && RUSTC_WRAPPER= cargo make test` | Pass |
| 6 | The ended-session test of step 5 in `cd daemon && RUSTC_WRAPPER= cargo make test` | Pass |
| 7 | The rating, tag, and folder-settings tests of step 2 in `cd daemon && RUSTC_WRAPPER= cargo make test` | Pass |
| 8 | `awk '/^tags:/,/^security:/' daemon/openapi.yaml \| rg -o 'name: .*'` | The twelve names of API Design §12 in its order |
| 9 | `cd ui-web && npm run gen:api && npm run typecheck`, and a browser session on the daemon with the web build that opens a folder, the tree, a search, the timeline, a transcoded video, rates and tags an image, uploads a file, and copies, moves, and deletes a file and a folder together | The type check passes against the replaced contract, and each screen shows its Entries and each command ends as `succeeded` in the activity panel |
| 10 | `rg -n '/files/\|/folders/\|/search/entries\|/timeline/(entries\|months\|refresh)\|/settings/folder\|/auth/me\|[^-]media-info\|GET /tags[^/]\|entries/details' docs/design.md docs/design/listing.md docs/design/timeline.md docs/design/rating-tags.md docs/design/video.md docs/terms.md`, and reading API Design §15.1 | No output, and §15.1 lists nothing |
| 11 | `cd daemon && RUSTC_WRAPPER= cargo make lint && RUSTC_WRAPPER= cargo make test`, and `cd ui-web && npm run lint && npm run typecheck && npm run test` | All pass |

The new HTTP tests protect the following: the Problem tests keep every error decodable with one type; the `view` tests keep an omitted `view` from reading an archive's inside; the batch tests keep one status for every command; the listing tests keep the five listings in one envelope; the ended-session test keeps an ended session distinguishable from an unknown one.

## 7. Risks and Rollback

Each step changes a published API, which `git revert` undoes in this repository but not in a client built against it.
An iOS app built against the contract before a step fails against a daemon after it, until the app is rebuilt against the replaced contract or the daemon is rolled back to the image before the step.
No step changes stored data, so rolling back the daemon loses nothing.
