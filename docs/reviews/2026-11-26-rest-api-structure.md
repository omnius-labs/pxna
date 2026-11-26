# Review of the REST API Structure, Symmetry, and Grouping

## 1. About This Review

This document records an audit of the structure of the v1 HTTP API: the resource hierarchy, the symmetry between related operations, and the validity of the OpenAPI tags.
It is not a security review and not a defect list; it asks whether the API is organized around one model that a client can learn once.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The reviewed scope, viewpoints, basis, findings, judgment, and subsequent handling |
| [API Design](../design/api.md#1-about-this-document) | The target v2 resource model, method and listing conventions, mutations, uploads and operations, errors, grouping, and migration. Destination for every design finding |
| [design.md](../design.md#8-conventions-for-mutating-apis) | The current v1 conventions for mutating APIs, which the v2 target leaves in place for v1 |
| [daemon/openapi.yaml](../../daemon/openapi.yaml#L1) | The authoritative v1 contract under review |
| [Listing and Search Design](../design/listing.md#1-about-this-document) | The snapshot and name-index behavior behind the listing endpoints |
| [Timeline Design](../design/timeline.md#1-about-this-document) | The timeline index and aggregation behind the timeline endpoints |
| [Rating and Tag Design](../design/rating-tags.md#1-about-this-document) | The value rules and storage behind the rating and tag endpoints |
| [reviews.md](../reviews.md) | The review history and transcription status |
| [terms.md](../terms.md#2-term-list) | The vocabulary; no terminology finding is raised here |
| [issues.md](../issues.md) | Confirmed defects; no defect is filed by this review |

[design.md](../design.md#11-division-of-responsibility-between-documents) holds the map of the document family.

### 1.2 Review Date and Subject

- Review date: 2026-11-26
- Review type: audit review
- Target commit: `7438f4ace0aa85b17cb1db8158de395cc11581d2`
- Reviewer: OpenCode (DeepSeek V4.1 Flash 1017)
- Independent verification: GPT-6 Luna (read-only verifier) on 2026-11-26; verdict: pass
- Method: source-first reading of `daemon/openapi.yaml`, the generated interface and the daemon handlers, the web client's API call sites, and the design documents. No daemon, test, or browser session was run.

## 2. Scope and Viewpoints

### 2.1 What Was Examined

[daemon/openapi.yaml](../../daemon/openapi.yaml#L1) was read for all 53 operations and 17 tags, the parameter components, and the request and response schemas cited below.

The following were read to confirm how the contract is produced and consumed:

- [server/mod.rs](../../daemon/entrypoints/interface/src/server/mod.rs#L1) for routing, authentication, validation, and the generated responses
- [server.rs](../../daemon/entrypoints/daemon/src/interface/server.rs#L1), [origin.rs](../../daemon/entrypoints/daemon/src/interface/middleware/origin.rs#L1), [unauthorized.rs](../../daemon/entrypoints/daemon/src/interface/middleware/unauthorized.rs#L1), and the feature handlers for [entries](../../daemon/entrypoints/daemon/src/interface/features/entries.rs#L1), [folders](../../daemon/entrypoints/daemon/src/interface/features/folders.rs#L1), [files](../../daemon/entrypoints/daemon/src/interface/features/files.rs#L1), [timeline](../../daemon/entrypoints/daemon/src/interface/features/timeline.rs#L1), [rating_tags](../../daemon/entrypoints/daemon/src/interface/features/rating_tags.rs#L1), [video](../../daemon/entrypoints/daemon/src/interface/features/video.rs#L1), and [operations](../../daemon/entrypoints/daemon/src/interface/features/operations.rs#L1)
- [operation.rs](../../daemon/entrypoints/daemon/src/service/mutation/operation.rs#L1), [upload.rs](../../daemon/entrypoints/daemon/src/service/mutation/upload.rs#L1), [listing.rs](../../daemon/entrypoints/daemon/src/service/listing.rs#L1), [video.rs](../../daemon/entrypoints/daemon/src/service/video.rs#L1), and [rating_tags.rs](../../daemon/modules/engine/src/service/rating_tags.rs#L1)
- [client.ts](../../ui-web/src/api/client.ts#L1), [entries.ts](../../ui-web/src/library/entries.ts#L1), and [timeline.ts](../../ui-web/src/library/timeline.ts#L1) for which v1 endpoints the web client actually calls
- `docs/design.md` §4 and §8, `docs/design/listing.md` §5 and §8, `docs/design/timeline.md`, `docs/design/rating-tags.md`, `docs/terms.md`, `docs/reviews.md`, and the existing review documents for the format and the document boundaries

### 2.2 What Was Not Examined

- No daemon, build, test, lint, browser session, or request was executed. Every behavioral statement is from reading code and contracts; runtime behavior is out of scope because this is a structural source review.
- The engine's storage, archive, media, thumbnail, and authentication internals were not re-read beyond the API-facing functions needed for the findings. Their contracts are assumed from their design documents and the separate reviews that cover them.
- The native client is not in this repository, so its use of the API and its migration constraints were not examined. It is outside this workspace.
- The web client was examined only at its API call sites; its screens, state stores, and error handling were not reviewed because the review concerns the API contract, not screen behavior.
- The actual support of `QUERY`, OpenAPI 3.2, axum method filters, and reverse proxies was not tested. R-8 is a deferral decision; the tooling statement is based on the repository versions and public documentation, not on execution.
- Performance, concurrency, memory, and long-running behavior were not measured; they are outside the structural viewpoints and would require load.
- Security properties are covered by the [2026-10-24 security audit](./2026-10-24-security-audit.md) and were not re-audited, to avoid duplicating that authority.
- The v1 error behavior was read in the generated code but not exercised; R-9 only needs the code paths that bypass the documented schema.

### 2.3 Viewpoints

- Whether the URL hierarchy expresses one resource model rather than the history of its features.
- Whether related operations are symmetric in method, parameter location, response shape, and status.
- Whether the tags group operations by what a reader is trying to do, and whether each tag description covers every operation under it.
- Whether the listing, mutation, upload, operation, and timeline contracts can share one client abstraction.
- Whether v1 and the target v2 can coexist and be migrated without a silent contract change.

## 3. Overall Assessment

The library-scoped hierarchy is sound, and as a v1 contract the area remains usable as it is.
The API is not coherent around one model: three listing generations, four page shapes, three entry wrappers, and four asynchronous lifecycle models coexist, and the tags follow the order in which the features were added rather than the resources a client uses.
A target v2 structure is recorded in [API Design](../design/api.md#3-resource-model); v1 should be frozen except for critical fixes, and the HTTP `QUERY` method is recorded there as a deferred alternative, not adopted for v2.

## 4. Findings

| ID | Summary | Type | Confidence | Handling |
| --- | --- | --- | --- | --- |
| [R-1](#r-1) | Three listing generations coexist under one v1 | Design issue | High | API Design §6.1, §13 |
| [R-2](#r-2) | Four page envelopes and three entry wrappers represent the same listing data | Design issue | High | API Design §6.3 |
| [R-3](#r-3) | Tags group by history, not by resource, and five paths sit under a tag whose scope or name does not match | Improvement proposal | High | API Design §12 |
| [R-4](#r-4) | The same concept is carried in different places: path, recursive, libraryIds, conflict, limit | Design issue | High | API Design §5, §7, §10 |
| [R-5](#r-5) | Asynchronous work has four lifecycle models, and timeline scans bypass Operations | Design issue | High | API Design §8, §9 |
| [R-6](#r-6) | The Files/Folders split produces six mutation endpoints and an asymmetric source/destination model | Improvement proposal | High | API Design §3.1, §7 |
| [R-7](#r-7) | Ratings and tags are entry-scoped data addressed as library-level resources | Design issue | High | API Design §3.1, §10.1 |
| [R-8](#r-8) | Read-only POSTs leave safe and idempotent semantics unexpressed; QUERY is deferred | Design issue | High | API Design §5.1, §14.2 |
| [R-9](#r-9) | Error responses do not share one schema across extraction, validation, routing, and handlers | Design issue | High | API Design §11.1 |
| [R-10](#r-10) | `folders/children` is a boolean probe named like a listing | Improvement proposal | High | API Design §6.4 |

R-1, R-2, and R-3 form the listing and grouping group: the same Entry is returned by several operations, and the tags do not say which operation is canonical.
R-4, R-5, R-6, and R-7 form the symmetry group: the same concept changes method, parameter location, or lifecycle depending on the endpoint.
R-8 and R-9 are the method and error contracts that must be settled before a v2 client can be generated.
R-10 is a naming detail that follows from the same grouping problem.

<a id="r-1"></a>
### R-1. Three Listing Generations Coexist Under One v1

**Type: Design issue** / **Confidence: High**

**Location**
[timeline/entries](../../daemon/openapi.yaml#L293), [timeline/months](../../daemon/openapi.yaml#L364), [folders/entries](../../daemon/openapi.yaml#L564), [search/entries](../../daemon/openapi.yaml#L609), [entries](../../daemon/openapi.yaml#L675), and [entries/months](../../daemon/openapi.yaml#L778).
The web client's use of the legacy paths is at [entries.ts](../../ui-web/src/library/entries.ts#L138), [entries.ts](../../ui-web/src/library/entries.ts#L151), [timeline.ts](../../ui-web/src/library/timeline.ts#L410), and [timeline.ts](../../ui-web/src/library/timeline.ts#L497).

**What is wrong**
The API exposes a folder listing, a search listing, a timeline listing, and a Scope/Filter listing as separate operations.
They have different defaults, different filters, and different page shapes, and the newer `/entries` operation does not replace the older ones.
A reader cannot tell which operation is canonical, and a client must keep several listing stores alive.

**Basis**
The paths and descriptions were read in full.
The web client still calls `folders/entries`, `search/entries`, `timeline/entries`, and `timeline/months`; it does not call `/entries`.
`/entries` is therefore an unused second-generation surface at this commit.

**Impact**
Every listing fix has to be made in several places, and the same Entry can appear with different fields or freshness depending on the operation.
Migrating a client later requires a mapping between four page shapes and four filters.

**Handling**
Transcribe to API Design §6.1 and §13.
The target keeps one listing operation and one page model, and treats the legacy paths as v1 only.

<a id="r-2"></a>
### R-2. Four Page Envelopes and Three Entry Wrappers Represent the Same Listing Data

**Type: Design issue** / **Confidence: High**

**Location**
[TimelineListing](../../daemon/openapi.yaml#L2950), [TimelineEntry](../../daemon/openapi.yaml#L2966), [FolderListing](../../daemon/openapi.yaml#L3062), [SearchEntry](../../daemon/openapi.yaml#L3092), [SearchListing](../../daemon/openapi.yaml#L3103), [ScopedEntry](../../daemon/openapi.yaml#L3318), and [EntryListing](../../daemon/openapi.yaml#L3344).

**What is wrong**
`FolderListing`, `SearchListing`, `TimelineListing`, and `EntryListing` carry different combinations of `totalCount`, `offset`, `snapshot`, `previous`, and `next`.
`TimelineEntry` and `SearchEntry` are both a `libraryId` plus an Entry, which `ScopedEntry` already expresses.
A single listing result therefore has four envelopes and three entry wrappers.

**Basis**
The schemas were read and compared field by field.
`EntryListing` already has optional timeline fields and both page forms, so the other shapes are subsets or duplicates of it.

**Impact**
A client cannot share one decoder, one page store, or one empty/truncated representation across the listing screens.
A fix to a page field has to be repeated in each schema.

**Handling**
Transcribe to API Design §6.3.
The target uses one `EntryPage` with `items`, a discriminated `page`, and `warnings`.

<a id="r-3"></a>
### R-3. Tags Group by History, Not by Resource, and Five Paths Sit Under a Tag Whose Scope or Name Does Not Match

**Type: Improvement proposal** / **Confidence: High**

**Location**
The [tag list](../../daemon/openapi.yaml#L51) and the [Folders](../../daemon/openapi.yaml#L53), [Files](../../daemon/openapi.yaml#L55), and [Tags](../../daemon/openapi.yaml#L73) descriptions.
The mismatches are [capabilities](../../daemon/openapi.yaml#L823) under `Libraries`, [entry details](../../daemon/openapi.yaml#L923) under `Folders`, [media-info](../../daemon/openapi.yaml#L1676) under `Video`, [tags](../../daemon/openapi.yaml#L1965) under `Tags`, and [children](../../daemon/openapi.yaml#L964) under `Folders`.

**What is wrong**
`Libraries` contains a feature-discovery operation that has nothing to do with a Library, and its description does not cover it.
`Folders` contains a batch Entry-metadata operation, which is not folder metadata.
`Video` contains image media information; its description was widened to "media information" instead of the tag being renamed.
`Tags` contains both one Entry's tags and the user's tag vocabulary, so the description covers two resources that a reader will look for separately.
`Folders` also contains a boolean child probe whose name reads like a child listing.
The `Folders` and `Files` descriptions mention only metadata and listing, while the tags also own copy, move, and delete.

**Basis**
The tags and operation tags were read against their descriptions.
The generated interface is split by tag (`interface/features`), so the same grouping decides where implementation and tests live.

**Impact**
A reader looking for "list entries" has to inspect `Folders`, `Search`, `Timeline`, and `Entries`.
A reader looking for image metadata looks under `Video`.
The tag descriptions cannot be used as a reliable index.

**Handling**
Transcribe to API Design §12.
The target has eleven tags: Meta, Auth, Libraries, Entries, Uploads, Operations, Timeline, Media, Ratings & Tags, Preferences, and Performance.

<a id="r-4"></a>
### R-4. The Same Concept Is Carried in Different Places

**Type: Design issue** / **Confidence: High**

**Location**
[PathQuery](../../daemon/openapi.yaml#L2062), [RecursiveQuery](../../daemon/openapi.yaml#L2071), [ListLimitQuery](../../daemon/openapi.yaml#L2125), [ConflictQuery](../../daemon/openapi.yaml#L2268), [getRating](../../daemon/openapi.yaml#L1839), [putRating](../../daemon/openapi.yaml#L1860), [getEntryTags](../../daemon/openapi.yaml#L1897), [putEntryTags](../../daemon/openapi.yaml#L1927), [getTimelineStatus](../../daemon/openapi.yaml#L394), [refreshTimeline](../../daemon/openapi.yaml#L419), and [EntryTransferRequest](../../daemon/openapi.yaml#L3812).

**What is wrong**
The Entry path is a query parameter on a GET and a body field on the rating and tag PUT.
`libraryIds` is a repeated query parameter for the timeline status and the search, a body field for the timeline refresh, and a nested field in the Scope for `/entries`.
`recursive` is a query parameter for folder listing and search, a body field for folder delete, and a Scope field for `/entries`.
`conflict` is a query parameter for the one-shot write and a body field for copy, move, and upload.
`limit` is a query parameter for the legacy listings and a body field for `/entries`.

**Basis**
The parameter declarations and request schemas were read.
The differences are not all mistakes: a raw-body write cannot put `conflict` in JSON.
The inconsistency that remains is the one where a JSON body already exists, as in the rating and tag PUT.

**Impact**
A client needs a different request builder per endpoint.
Validation rules and documentation for the same concept are repeated.
A change to the meaning of `recursive` or `path` has to be applied in several places.

**Handling**
Transcribe to API Design §5, §7, and §10.
The target puts the Entry path in the query for both rating and tag reads and writes, and puts `libraryIds` and `recursive` in the Scope or the command body consistently.

<a id="r-5"></a>
### R-5. Asynchronous Work Has Four Lifecycle Models

**Type: Design issue** / **Confidence: High**

**Location**
[operations](../../daemon/openapi.yaml#L1605), [timeline/status](../../daemon/openapi.yaml#L394), [timeline/refresh](../../daemon/openapi.yaml#L419), [uploads](../../daemon/openapi.yaml#L1437), and [video/sessions](../../daemon/openapi.yaml#L1706).
The silent video eviction is at [video.rs](../../daemon/entrypoints/daemon/src/service/video.rs#L289); the Operation terminal state is at [operation.rs](../../daemon/entrypoints/daemon/src/service/mutation/operation.rs#L611).

**What is wrong**
Copy, move, and delete use Operations with `GET /operations/{id}` and `POST /operations/{id}/cancel`.
Timeline scans use `POST /timeline/refresh` and `GET /timeline/status`, with no Operation and no cancel.
Uploads use create, get, content, commit, and delete.
Video sessions use create, delete, playlist, and segments, and silently close the oldest session when the concurrent limit is reached.
The same client therefore needs four different polling and cancellation stories.

**Basis**
The paths and handlers were read.
The video service's `while sessions.len() >= max` loop closes the oldest session before starting a new one.
The Operation finish path sets a terminal status from `failed_entries`, so Operations already have the result model that a scan and a copy share.

**Impact**
A client cannot use one activity panel or one retry rule.
A video session can disappear with no error to the client that owned it.
Timeline progress is observable in a different shape from every other long-running action.

**Handling**
Transcribe to API Design §8 and §9.
The target makes timeline scans Operations, keeps uploads and video sessions as distinct lifecycle resources, and replaces silent video eviction with `429`.

<a id="r-6"></a>
### R-6. The Files/Folders Split Produces Six Mutation Endpoints and an Asymmetric Source/Destination Model

**Type: Improvement proposal** / **Confidence: High**

**Location**
[folders/copy](../../daemon/openapi.yaml#L1003), [folders/move](../../daemon/openapi.yaml#L1040), [folders/delete](../../daemon/openapi.yaml#L1087), [files/copy](../../daemon/openapi.yaml#L1273), [files/move](../../daemon/openapi.yaml#L1311), [files/delete](../../daemon/openapi.yaml#L1358), and [EntryTransferRequest](../../daemon/openapi.yaml#L3812).
The design rationale for the split is in [design.md §8.1](../design.md#81-namespaces).

**What is wrong**
The split is defensible: a file and a folder need different validation, and an archive path means the archive file in the files namespace and its root in the folders namespace.
The cost is six mutation endpoints, a client split for a mixed selection, and an asymmetric transfer request where the source Library is in the path but `dstLibraryId` is in the body.

**Basis**
The paths and the `EntryTransferRequest` schema were read.
The design document states the namespace rule and the archive reason; this finding does not dispute that the split works for v1.

**Impact**
A mixed selection becomes two requests with separate results.
A copy or move from library A to library B has the source in the URL and the destination in the body, so a generic client cannot treat both sides the same way.

**Handling**
Transcribe to API Design §3.1 and §7.
The target keeps one Entry resource with `view=file|folder` and one `POST /entries/copy|move|delete` command; the split is not treated as a defect in v1.

<a id="r-7"></a>
### R-7. Ratings and Tags Are Entry-Scoped Data Addressed as Library-Level Resources

**Type: Design issue** / **Confidence: High**

**Location**
[rating](../../daemon/openapi.yaml#L1829), [tags](../../daemon/openapi.yaml#L1897), and the path-keyed storage in [rating_tags.rs](../../daemon/modules/engine/src/service/rating_tags.rs#L61).

**What is wrong**
`/libraries/{libraryId}/rating` and `/libraries/{libraryId}/tags` read as ratings and tags of the Library.
The actual target is one Entry, supplied in the query on GET and in the body on PUT.
The storage key is `(user, library, path)`, so an external rename loses the mark, and a new file at the same path inherits the old mark.

**Basis**
The paths, the PUT request schemas, and the SQL migration in `rating_tags.rs` were read.
The daemon's own move path does carry the marks, but the API cannot repair an external rename because it has only the old path.

**Impact**
The URL misstates the resource, the path is repeated in two locations, and a user can see a rating disappear or move to a different file after a rename outside Pxna.

**Handling**
Transcribe to API Design §3.1 and §10.1.
The target addresses `.../entries/rating` and `.../entries/tags` with the path in the query, and keys marks by `contentId`.

<a id="r-8"></a>
### R-8. Read-Only POSTs Leave Safe and Idempotent Semantics Unexpressed

**Type: Design issue** / **Confidence: High**

**Location**
[entries](../../daemon/openapi.yaml#L675), [entries/months](../../daemon/openapi.yaml#L778), and [entry details](../../daemon/openapi.yaml#L923).

**What is wrong**
These three operations change no state, but they use POST because they carry a body.
POST does not tell an HTTP client or an intermediary that the request is safe or idempotent, and a cookie request also needs an `Origin` under the current CSRF check.
The HTTP `QUERY` method (RFC 10008) would express the intent, but the repository's OpenAPI 3.0.3 pipeline, custom generator, and axum 0.8 `MethodFilter` do not support it without hand-written routing and client work.

**Basis**
The paths and the generated server were read.
The `QUERY` facts are from RFC 10008 and OpenAPI 3.2; the tooling limitation is from the repository's versions and the axum API. No `QUERY` request was executed.
v2 keeps POST for this version; the alternative is recorded rather than rejected.

**Impact**
A client cannot rely on automatic retry for these reads.
The contract has to state in prose that the POSTs are reads, and a non-browser client using a cookie must send `Origin`.

**Handling**
Transcribe to API Design §5.1 and §14.2.
The target keeps POST for v2 and records `QUERY` as deferred with observable conditions.

<a id="r-9"></a>
### R-9. Error Responses Do Not Share One Schema

**Type: Design issue** / **Confidence: High**

**Location**
The generated validation response at [server/mod.rs](../../daemon/entrypoints/interface/src/server/mod.rs#L148), the empty 401 at [server/mod.rs](../../daemon/entrypoints/interface/src/server/mod.rs#L139), the plain-text preference 400 at [openapi.yaml](../../daemon/openapi.yaml#L270), and the 401 rewrite at [unauthorized.rs](../../daemon/entrypoints/daemon/src/interface/middleware/unauthorized.rs#L10).

**What is wrong**
The contract documents a JSON `ErrorResponse` for failures, but validator and extractor failures are plain text, an unmatched route and a disallowed method have empty bodies, and the 401 body is only repaired by an outer middleware.
The preferences endpoint even documents that its 400 is plain text.
`requestId` is defined in the schema but never populated.

**Basis**
The generated validation branch writes `validation.unwrap_err().to_string()` without a content type.
The routing fallback returns the framework default.
The unauthorized middleware replaces a 401 only when it has no content type.

**Impact**
A generated client cannot deserialize every error with one type.
A client must special-case endpoints and extraction failures, and a user-facing message may be missing.

**Handling**
Transcribe to API Design §11.1.
The target uses `application/problem+json` for every error, including routing, extraction, validation, and rate limits.

<a id="r-10"></a>
### R-10. `folders/children` Is a Boolean Probe Named Like a Listing

**Type: Improvement proposal** / **Confidence: High**

**Location**
[folders/children](../../daemon/openapi.yaml#L964) and [FolderChildrenProbe](../../daemon/openapi.yaml#L3572).

**What is wrong**
The path suggests a list of child Entries, but the response is `{hasChildren, maxAgeMs}`.
The tri-state `true`/`false`/`null` is easy to miss, and `maxAgeMs` is a cache detail that the name does not suggest.

**Basis**
The path, schema, and handler were read.
The probe is an optimization for the tree, not a listing; the v1 folder listing already returns children.

**Impact**
A client author may call it expecting entries, or may treat `null` as `false` and show an empty folder.

**Handling**
Transcribe to API Design §6.4.
The target names it `entries/child-probe` and documents the tri-state and `maxAgeMs`.

## 5. What Was Checked and Found Sound

- The Library-scoped URL root is consistent: [LibraryIdPath](../../daemon/openapi.yaml#L2022) and [PathQuery](../../daemon/openapi.yaml#L2062) keep the Library id and the path relative, and content paths never expose a host path.
- The action suffix convention is consistent: [copy](../../daemon/openapi.yaml#L1003), [move](../../daemon/openapi.yaml#L1040), [delete](../../daemon/openapi.yaml#L1087), [cancel](../../daemon/openapi.yaml#L1650), [commit](../../daemon/openapi.yaml#L1571), and [refresh](../../daemon/openapi.yaml#L419) are used the same way across tags.
- The Files/Folders split is defensible for validation and archive semantics, even though R-6 records its cost. The finding is an improvement proposal, not a claim that v1 is broken; [design.md §8.1](../design.md#81-namespaces) records the namespace reason.
- Uploads are a coherent lifecycle (`create`, `get`, `content`, `commit`, `delete`) at [uploads](../../daemon/openapi.yaml#L1437), and Operations are a coherent lifecycle (`list`, `get`, `cancel`) at [operations](../../daemon/openapi.yaml#L1605); each is internally symmetric.
- `Scope`, `Filter`, and `Start` in [terms.md](../terms.md#2-term-list) match the shapes used by the listing endpoints; no terminology finding was raised.
- The tag descriptions for Health, Setup, Auth, Preferences, Uploads, Operations, Performance, and ImageSocket cover their operations in the [tag list](../../daemon/openapi.yaml#L51).
- Every operation in [openapi.yaml](../../daemon/openapi.yaml#L1) has a tag and an operation id, and no path is untagged.
- The web client uses the legacy listing paths consistently at [entries.ts](../../ui-web/src/library/entries.ts#L138), [entries.ts](../../ui-web/src/library/entries.ts#L151), [timeline.ts](../../ui-web/src/library/timeline.ts#L410), and [timeline.ts](../../ui-web/src/library/timeline.ts#L497); no web client call site depends on `/entries`, which supports treating it as a target rather than a compatibility constraint. The native client was not examined.

## 6. Afterwards
