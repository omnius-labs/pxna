# API Design

## 1. About This Document

This document defines the target structure of the Pxna HTTP API: the resource model, URL hierarchy, method conventions, listing and mutation contracts, upload and operation lifecycles, error shape, tag grouping, and migration from v1.
It is the design for the next major API, not a description of the current `daemon/openapi.yaml`.
The intended readers are developers changing the API contract, the generated interface, the daemon handlers, or a client.

This document assumes the vocabulary of [terms.md](../terms.md#2-term-list).
Definitions of words are not in this document.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The target v2 resource model, URL hierarchy, method conventions, listing and mutation contracts, upload and operation lifecycles, error shape, tag grouping, and v1 migration |
| [design.md](../design.md#8-conventions-for-mutating-apis) | The current v1 conventions for mutating APIs. This document supersedes them only for v2; v1 remains as recorded there |
| [daemon/openapi.yaml](../../daemon/openapi.yaml) | The authoritative v1 contract: paths, parameters, schemas, status codes |
| [Storage Layer Design](./storage.md#1-about-this-document) | Path and archive resolution, staging, atomic publication, and the limits this document assumes |
| [Listing and Search Design](./listing.md#1-about-this-document) | Snapshot construction, name matching, the name index, and recursive limits behind the listing contract |
| [Timeline Design](./timeline.md#1-about-this-document) | Timeline dates, aggregation, the timeline index, and the freshness this document exposes |
| [Rating and Tag Design](./rating-tags.md#1-about-this-document) | What can be rated and tagged, value rules, storage, and how marks follow changes |
| [Client Design](./client.md#1-about-this-document) | How the web client consumes the API and how it migrates |
| [terms.md](../terms.md#2-term-list) | Definitions of terms, spelling, aliases, and boundaries between neighboring terms |
| [issues.md](../issues.md) | Confirmed defects. This document is not a defect list |

[design.md](../design.md#11-division-of-responsibility-between-documents) holds the map of the document family.

### 1.2 Tense of This Document

§3 through §13 describe the target v2 API in the present tense.
§14 records the decisions and deferred items behind that target.
§15 records what is implemented and in what order the work is expected to proceed.
If you want to know what works today, read §15 first.

## 2. Responsibility and Boundary

The API layer owns the HTTP contract: how a Library and an Entry are addressed, which method expresses which intent, how a page is selected, how a mutation reports its result, how an error is shaped, and how clients discover limits and optional features.
It does not own how a path is resolved inside an archive, how a listing snapshot is built, how the name index answers a condition, how a timeline date is chosen, how a thumbnail is generated, or how marks are stored.
Those concerns stay in [Storage Layer Design](./storage.md#1-about-this-document), [Listing and Search Design](./listing.md#1-about-this-document), [Timeline Design](./timeline.md#1-about-this-document), [Thumbnail Design](./thumbnail.md#1-about-this-document), [Video Playback Design](./video.md#1-about-this-document), and [Rating and Tag Design](./rating-tags.md#1-about-this-document).

The target is served under `/api/v2`, alongside the frozen `/api/v1`.
A v1 path is never reinterpreted as a v2 path.
The v1 contract stays in `daemon/openapi.yaml`; the v2 contract is a separate OpenAPI document so that the generated v1 interface does not silently change.
The deployment premise is unchanged: a single owner, a LAN-first daemon, and the web client served from the same origin as the API.
Cross-origin browser access is not a goal, and the design does not add CORS support for arbitrary origins.

## 3. Resource Model

### 3.1 Entry and EntryRef

The API keeps one Entry type; it does not split files and folders into separate top-level resources.
The API addresses an Entry by an `EntryRef`; the vocabulary and the external form are defined in [terms.md](../terms.md#2-term-list).

A request uses the `view` field to choose the file face or the folder face of a path.
For `/photos/a.zip`, `view=file` names the archive file and `view=folder` names the archive's root.
For a real folder, `view=folder` is the normal value; `view=file` is invalid.
For a normal file, `view=file` is the normal value; `view=folder` is invalid.
When `view` is omitted, a folder is treated as `folder`, a file as `file`, and an archive as `file`; the last rule is the safe default because it never lists or expands an archive by accident.

The Entry carries `container` to make the archive relationship visible:

| `container` | Meaning |
| --- | --- |
| `local` | The Entry is in the Library's local filesystem |
| `archive` | The Entry is inside an archive |
| `archiveRoot` | The Entry is the root view of an archive |

`metadata.readonly` is the effective value.
The server combines the Library's `readonly`, the archive boundary, and the archive root rule; a client does not combine them itself.
`metadata.readonlyReason` is `library`, `archive`, or null.

For an image or a video, the Entry may carry `contentId`.
`contentId` is the identity used by ratings and tags.
It is derived from the content, not from the path, so a daemon-driven move or copy keeps the marks and a path reused by a different file does not inherit them.
An Entry without `contentId` cannot be rated or tagged.

### 3.2 Library and Path

A path is always relative to the Library named in the same `EntryRef`.
It starts with `/`, has no empty segment, no `.` or `..`, and is at most 4,096 characters.
The root is `/`.

An archive is addressed by plain concatenation, as in `/photos/a.zip/dir/x.png`.
The boundary between the archive and its contents is resolved by probing the filesystem, which is unambiguous because a file and a directory cannot share a name in one directory.
`view` chooses between the archive file and the archive root; it does not change the path grammar.

A path is an address, not a stable identity.
A rename changes it.
Ratings and tags therefore use `contentId` for identity, while the API still addresses the Entry by path.

### 3.3 Scope and Filter

A listing request has a Scope, a Filter, listing options, a sort, and a page.

A Scope names what is read:

| Field | Meaning |
| --- | --- |
| `starts` | One to 64 `EntryRef` values. Each start is a folder view; an archive start is its root |
| `recursive` | Whether the listing reads descendants of each start |
| `insideArchives` | Whether the listing reads archives under the starts like folders, down to the configured nesting limit |

A Filter names what is kept:

| Field | Meaning |
| --- | --- |
| `query` | Whitespace-separated terms that must all occur in the name after NFKC normalization and case folding |
| `rating` | `good`, `bad`, or `none` for the current user; only images and videos match |
| `tags` | Up to 32 tag words; all must be present; only images and videos match |
| `kinds` | `file`, `folder`, `link`, `image`, `video`, `archive`, or `other`; `other` is a file that is not an image, video, or archive |
| `hidden` | `exclude`, `include`, or `only`; the default is `exclude` |

The listing options are `foldersFirst` and `fields`.
`fields` is `full` or `summary`; `summary` returns only the path, name, kind, archive hint, and effective readonly value.
The summary projection is the replacement for the separate summary listing API.

The sort is `name`, `kind`, `size`, `modifiedAt`, or `timelineDate`.
`order` is `asc` or `desc`.
`timelineDate` is descending only.
Entries that compare equal are ordered by `libraryId` and then by `path`, so a page is deterministic.

A start that does not exist, that names a file rather than a folder view, or that names an unknown Library is not a request failure.
It is returned in `warnings.missingStarts` with a reason, and the other starts are listed.
The same rule applies to a start cut off at the entry limit and to a start that was walked because the name index was not usable.

## 4. URL Hierarchy

All paths are relative to `/api/v2`.

```
/api/v2
├─ GET    /health
├─ GET    /capabilities
├─ POST   /setup
├─ GET    /setup/status
├─ POST   /auth/login
├─ GET    /auth/session
├─ DELETE /auth/session
├─ GET    /auth/sessions
├─ DELETE /auth/sessions/{sessionId}
├─ PUT    /auth/password
├─ GET    /preferences
├─ PUT    /preferences
├─ GET    /libraries
├─ GET    /libraries/{libraryId}
│
├─ GET    /libraries/{libraryId}/entries/metadata?path=&view=
├─ GET    /libraries/{libraryId}/entries/content?path=
├─ HEAD   /libraries/{libraryId}/entries/content?path=
├─ PUT    /libraries/{libraryId}/entries/content?path=&conflict=
├─ POST   /libraries/{libraryId}/entries/files?path=&conflict=
├─ POST   /libraries/{libraryId}/entries/folders
├─ GET    /libraries/{libraryId}/entries/child-probe?path=&hidden=
├─ GET    /libraries/{libraryId}/entries/media?path=&view=
├─ GET    /libraries/{libraryId}/entries/thumbnail?path=&view=&size=&shape=
├─ GET    /libraries/{libraryId}/entries/rating?path=&view=
├─ PUT    /libraries/{libraryId}/entries/rating?path=&view=
├─ GET    /libraries/{libraryId}/entries/tags?path=&view=
├─ PUT    /libraries/{libraryId}/entries/tags?path=&view=
│
├─ POST   /libraries/{libraryId}/uploads
├─ GET    /libraries/{libraryId}/uploads
├─ GET    /uploads/{uploadId}
├─ DELETE /uploads/{uploadId}
├─ PUT    /uploads/{uploadId}/content?offset=
├─ POST   /uploads/{uploadId}/commit
│
├─ POST   /libraries/{libraryId}/video/sessions
├─ GET    /libraries/{libraryId}/video/sessions/{sessionId}
├─ DELETE /libraries/{libraryId}/video/sessions/{sessionId}
├─ GET    /libraries/{libraryId}/video/sessions/{sessionId}/playlist.m3u8
├─ GET    /libraries/{libraryId}/video/sessions/{sessionId}/segments/{name}
│
├─ POST   /entries
├─ POST   /entries/months
├─ POST   /entries/metadata
├─ POST   /entries/copy
├─ POST   /entries/move
├─ POST   /entries/delete
│
├─ POST   /timeline/scans
├─ GET    /libraries/{libraryId}/timeline
│
├─ GET    /operations
├─ GET    /operations/{operationId}
├─ POST   /operations/{operationId}/cancel
│
├─ GET    /tags/words
├─ GET    /sockets/image
└─ POST   /performance/client-timings
```

The hierarchy follows one rule: a single-Entry or single-Library operation is library-scoped, while a query or a command that can span Libraries is global.
The rule is what makes `POST /entries/copy` carry both source and destination Libraries without putting one of them in the path.

## 5. Method Conventions

### 5.1 Read Methods

`GET` and `HEAD` are used for a single Entry's metadata, content, media information, rating, tags, and thumbnail.
They are cacheable and support conditional requests as described in §11.3.

A read that carries a structured body uses `POST`.
The three operations are `POST /entries`, `POST /entries/months`, and `POST /entries/metadata`.
They change no state.
`POST` is used because the body is too large or too structured for a URI: a Scope may name up to 64 starts with paths up to 4,096 characters, and a metadata batch may carry up to 200 references.
The v2 CSRF check treats these three paths as read-only and does not require `Origin`; the check remains on the mutation paths.

The HTTP `QUERY` method (RFC 10008) would express these reads as safe and idempotent.
It is not adopted for v2 now; §14.2 records the conditions under which it is revisited.
A client must therefore treat these three operations as reads by contract, not by method.

### 5.2 Mutation Methods

`PUT` is used when the target is identified by the request and the effect is idempotent.
This covers content at an exact path, display preferences, an Entry's rating, and an Entry's tags.
`PUT` content accepts `fail`, `overwrite`, and `skip`; `rename` is a creation command and is handled by `POST /libraries/{libraryId}/entries/files`.

`POST` is used for creation with a server-chosen name, batch commands, and actions.
This covers folder creation, copy, move, delete, upload sessions, video sessions, login, setup, timeline scans, and operation cancellation.

`DELETE` is used for a lifecycle resource: the current session, another session, an upload session, and a video session.
File and folder deletion is a batch command under `POST /entries/delete`; the API does not use `DELETE` for it because the target list, conflict policy, and recursive flag are carried together.

Every batch command returns `202` with an Operation, including a command that completes immediately.
The Operation body carries the terminal state when the work is already done, so a client does not have to poll.
This removes the v1 choice between `200` and `202` for the same copy, move, or delete operation.

### 5.3 Status Codes

| Status | Use |
| --- | --- |
| 200 | A read, a replacement, or a preference, rating, or tag write |
| 201 | A created Entry, upload, session, or video session |
| 202 | An Operation was accepted; inspect `operation.status` for whether it is already terminal |
| 204 | A lifecycle resource was deleted, or a write had no body |
| 304 | A conditional read or listing is unchanged |
| 400 | The request syntax or a field value is invalid |
| 401 | The session is missing, expired, or invalid; `WWW-Authenticate` is present for bearer |
| 403 | The Library is readonly, the path is inside an archive, or the operation is not permitted |
| 404 | The target does not exist for a single-Entry read |
| 409 | The request conflicts with the current state, such as a non-empty folder without `recursive` |
| 412 | A precondition such as `If-Match` or `ifSnapshot` failed |
| 413 | The body exceeds the one-shot or chunk limit |
| 415 | The media type is not supported for the request |
| 422 | The syntax is valid but the combination cannot be processed, such as `timelineDate` with an offset page |
| 429 | A rate or concurrency limit was reached; `Retry-After` is present |
| 504 | A blocking extraction or probe exceeded its deadline; `Retry-After` is present |

A missing target in a batch command is not a top-level `404`.
It is reported in the Operation result so that a retry after a lost response remains successful.

## 6. Listings

### 6.1 One Listing API

`POST /entries` is the only cross-Library listing API.
A single-Library folder tree uses the same operation with one start, `fields=summary`, and `hidden=include`.
The v1 folder listing, search listing, and timeline listing are not carried into v2.

A request has this shape:

```json
{
  "scope": {
    "starts": [
      { "libraryId": "photos", "path": "/", "view": "folder" }
    ],
    "recursive": true,
    "insideArchives": true
  },
  "filter": {
    "query": "trip",
    "rating": "good",
    "tags": ["family"],
    "kinds": ["image", "video"],
    "hidden": "exclude"
  },
  "options": {
    "foldersFirst": false,
    "fields": "full"
  },
  "sort": "timelineDate",
  "order": "desc",
  "page": {
    "kind": "cursor",
    "limit": 200
  }
}
```

The response is an Entry page, defined in §6.3.

The listing reads the same snapshots and name index as v1.
It does not change how a listing is built; it changes how a listing is requested and returned.
A listing with no name, rating, or tag condition may answer from the timeline index; the response says whether the answer was indexed and when the aggregation was taken.

### 6.2 Page Anchors

A page request is a discriminated union:

| `kind` | Fields | Valid when |
| --- | --- | --- |
| `offset` | `offset`, `limit` | Any sort except `timelineDate` |
| `cursor` | `after` or `before`, `limit` | `timelineDate` |
| `month` | `month`, `limit` | `timelineDate` |
| `around` | `entry`, `limit` | `timelineDate`; the Entry is near the middle of the page |

At most one anchor is given.
`offset` and `cursor` may not be combined.
`month` and `around` are not accepted with a non-timeline sort.
`limit` defaults to 200 and is at most 1,000.

The response echoes the effective page:

```json
{
  "kind": "offset",
  "offset": 0,
  "limit": 200,
  "totalCount": 1234,
  "snapshot": "..."
}
```

```json
{
  "kind": "cursor",
  "previous": null,
  "next": "eyJkYXRlIjoiMjAyNi0wNS0wMVQxMjozNDo1NiJ9"
}
```

A cursor is a base64url string.
A `month` or `around` request returns a cursor page with an `anchor` object that names the month or the Entry.
`totalCount` is present only for an offset page.

### 6.3 Entry Page

Every listing returns the same envelope:

```json
{
  "items": [
    {
      "libraryId": "photos",
      "entry": { "...": "FileSystemEntry" },
      "timelineDate": "2026-05-01T12:34:56+09:00",
      "timelineDateSource": "exif"
    }
  ],
  "page": {
    "kind": "cursor",
    "previous": null,
    "next": "..."
  },
  "warnings": {
    "cutOffStarts": [],
    "walkedStarts": [],
    "missingStarts": [
      {
        "start": { "libraryId": "photos", "path": "/missing", "view": "folder" },
        "reason": "notFound"
      }
    ],
    "indexed": true,
    "asOf": "2026-11-26T03:04:05Z"
  }
}
```

- `items` always carries `libraryId` and the Entry, so a single-Library result uses the same type as a multi-Library result.
- `timelineDate` and `timelineDateSource` are present only when the sort is `timelineDate`.
- `warnings` is always present. Each warning has a `reason`: `notFound`, `notFolder`, `unknownLibrary`, `entryLimit`, or `indexUnavailable`.
- `indexed` says whether every start answered from the name or timeline index.
- `asOf` is the aggregation time when an index answered; it is absent for a full walk.
- The response carries an `ETag` derived from the snapshot, so a client can revalidate with `If-None-Match` and receive `304`.

A `fields=summary` page replaces the v1 summary listing.
Its items carry the path, name, kind, archive hint, and effective readonly value, and omit sizes and timestamps.

### 6.4 Child Probe

`GET /libraries/{libraryId}/entries/child-probe?path=&hidden=` returns a tri-state hint:

```json
{
  "hasChildren": true,
  "maxAgeMs": 500
}
```

`hasChildren` is `true`, `false`, or `null`.
`false` means the probe reached the end without a matching child; `null` means the result is unknown because the probe was busy, incomplete, or interrupted.
`maxAgeMs` is zero when the result is unknown and otherwise the remaining validity of the cached result.

The probe does not open or extract an archive for this request alone.
A timed-out blocking call keeps its work slot until it exits.

### 6.5 Months and Metadata

`POST /entries/months` takes the same Scope and Filter as `/entries` and returns month counts:

```json
{
  "timeZone": "Asia/Tokyo",
  "months": [
    { "month": "2026-05", "count": 123 }
  ]
}
```

The `timeZone` is explicit and defaults to the value in `GET /capabilities`.
A month count is the number of Entries that the matching `timelineDate` listing returns for that month.

`POST /entries/metadata` takes up to 200 references and returns one result per reference:

```json
{
  "items": [
    {
      "ref": { "libraryId": "photos", "path": "/a.jpg", "view": "file" },
      "entry": { "...": "FileSystemEntry" },
      "error": null
    },
    {
      "ref": { "libraryId": "photos", "path": "/missing.jpg", "view": "file" },
      "entry": null,
      "error": { "code": "NotFound" }
    }
  ]
}
```

The request succeeds with `200` even when individual entries fail.
The per-item `error` is a code, not a full Problem document, because the request itself succeeded.

## 7. Mutations

### 7.1 Copy and Move

Copy and move are cross-Library commands:

```json
{
  "sources": [
    { "libraryId": "photos", "path": "/a.jpg", "view": "file" },
    { "libraryId": "photos", "path": "/2026/a.zip", "view": "folder" }
  ],
  "destination": {
    "libraryId": "photos",
    "path": "/backup",
    "view": "folder"
  },
  "conflict": "fail"
}
```

- A source with `view=folder` copies or moves the tree; an archive root copies or moves the archive's contents.
- A source with `view=file` copies or moves the file; an archive copies the archive file itself.
- A destination with `view=folder` is a folder destination and keeps each source's name.
- A destination with `view=file` is an exact target and is valid only with one source.
- `conflict` is `fail`, `skip`, `overwrite`, or `rename`.
- The response is `202` with an Operation. The Operation result carries `entries`, `skipped`, `failed`, `missing`, and `warnings`.
- `rename` reports the actual published paths in `entries`.

A copy or move whose destination is inside a source is rejected before the Operation starts.
A cross-Library move copies the bytes and then removes the source; the Operation reports a partial result if it fails midway.

### 7.2 Delete

Delete is one cross-Library command:

```json
{
  "targets": [
    { "libraryId": "photos", "path": "/a.jpg", "view": "file" },
    { "libraryId": "photos", "path": "/old", "view": "folder" }
  ],
  "recursive": true
}
```

- A missing target is reported in `result.missing`; it is not a top-level `404`.
- A non-empty folder with `recursive=false` is a `409` before the Operation starts.
- An archive root (`view=folder` on an archive) cannot be deleted; it is a `400`.
- Duplicate or nested targets are normalized and deduplicated; the removed duplicates are reported in `warnings`.
- The response is `202` with an Operation. Entries already deleted are not rolled back.

The v1 rule that a non-empty folder without `recursive` is a `409` is kept.
The v1 behaviour that returned `400` was a contract mismatch; v2 makes the contract and the status agree.

### 7.3 Write and Create

A one-shot file write addresses an exact path:

```
PUT /libraries/{libraryId}/entries/content?path=/new.jpg&conflict=overwrite
Content-Type: image/jpeg
```

```json
{
  "outcome": "created",
  "entry": { "...": "FileSystemEntry" }
}
```

- `outcome` is `created`, `replaced`, or `skipped`.
- A new Entry returns `201`; a replacement or skip returns `200`.
- `conflict` is `fail`, `overwrite`, or `skip`. `rename` is not accepted here because `PUT` would no longer be idempotent.
- A body over 32 MiB returns `413`; use an upload session.

A create that may choose a different name is a command:

```
POST /libraries/{libraryId}/entries/files?path=/new.jpg&conflict=rename
Content-Type: image/jpeg
```

This accepts the same conflict policies as the v1 write, but because it can publish under another name it is a `POST`.
It returns `201` with the published Entry.
An `Idempotency-Key` is required for this operation.

A folder is created with:

```json
{
  "path": "/new/deep",
  "parents": true
}
```

`POST /libraries/{libraryId}/entries/folders` returns `201` with the Entry; an existing path returns `409`.

### 7.4 Operation Results

An Operation is the single result shape for batch work:

```json
{
  "operation": {
    "id": "op_...",
    "kind": "copy",
    "status": "succeeded",
    "createdAt": "2026-11-26T03:00:00Z",
    "startedAt": "2026-11-26T03:00:00Z",
    "finishedAt": "2026-11-26T03:00:01Z",
    "progress": { "processed": 2, "total": 2, "bytes": 12345 },
    "result": {
      "entries": [
        { "libraryId": "photos", "path": "/backup/a.jpg" }
      ],
      "skipped": [],
      "failed": [],
      "missing": [],
      "warnings": [],
      "destination": { "libraryId": "photos", "path": "/backup", "view": "folder" }
    }
  }
}
```

`result` is absent until the Operation is terminal.
A `202` body may already carry `status=succeeded` and `result`; a client then skips polling.
A failure in one entry does not discard the others; `status` is `failed` when `failed` is non-empty.

## 8. Uploads and Operations

### 8.1 Uploads

An upload session is a resumable write lifecycle:

```
POST   /libraries/{libraryId}/uploads
GET    /libraries/{libraryId}/uploads
GET    /uploads/{uploadId}
PUT    /uploads/{uploadId}/content?offset=
POST   /uploads/{uploadId}/commit
DELETE /uploads/{uploadId}
```

A create request carries `path`, `size`, `conflict`, and an optional `sha256`.
`offset` must equal `committedSize`; a mismatch is a `409`.
`commit` verifies the declared size and the digest, rechecks the destination conflict, and publishes atomically.
A conflict at commit does not abort the session; the client may retry with an explicit `conflict` in the commit body.
An expired session is `410 Gone`.
Sessions and staging files survive a daemon restart; `GET /libraries/{libraryId}/uploads` lists the active sessions so a client can resume.

The chunk limit is 32 MiB.
A chunk over the limit is a `400` with `LimitExceeded`.

### 8.2 Operations

All long-running batch work uses the same resource:

```
GET  /operations
GET  /operations/{operationId}
POST /operations/{operationId}/cancel
```

`GET /operations` supports `kind`, `status`, `libraryId`, `limit`, and `cursor`.
Operations are persisted, so a restart does not lose a finished result.
`POST` commands accept an `Idempotency-Key`; a retry with the same key and body returns the original Operation, and the same key with a different body is a `422`.
A `202` carries `Location` and, when not terminal, `Retry-After`.

Timeline scans are Operations of kind `timelineScan`.
Video sessions are not Operations; they are long-lived streaming resources with their own create, get, and delete lifecycle.
Uploads are not Operations; they are write sessions that the client advances explicitly.
This leaves three lifecycle models with clear meanings: batch Operation, upload session, and streaming session.

## 9. Timeline, Media, and Video

### 9.1 Timeline

Timeline reads use `POST /entries` with `sort=timelineDate`.
There is no separate timeline entry listing.
The timeline index remains the source when the Filter has no name, rating, or tag condition; `warnings.indexed` and `warnings.asOf` say whether that answer was used.

A scan is started with:

```json
{
  "libraryIds": ["photos", "videos"]
}
```

`POST /timeline/scans` returns `202` with an Operation of kind `timelineScan`.
`GET /libraries/{libraryId}/timeline` returns a lightweight status projection:

```json
{
  "running": false,
  "processedFiles": 1234,
  "completedAt": "2026-11-26T03:00:00Z",
  "timeZone": "Asia/Tokyo"
}
```

The Operation is authoritative for completion; the status projection is for clients that do not want to poll an Operation.

`timelineDate` is an RFC 3339 string with an offset, such as `2026-05-01T12:34:56+09:00`.
The daemon's configured zone is returned by `GET /capabilities` and by the timeline status.
A client never has to infer the zone from a bare local time.

### 9.2 Media

`GET /libraries/{libraryId}/entries/media?path=&view=` returns a `MediaInfo` union of image and video information.
It works for an Entry inside an archive.

`GET /libraries/{libraryId}/entries/thumbnail?path=&view=&size=&shape=` returns a cacheable image with `ETag`.
The image socket remains the high-throughput path for a grid; the thumbnail endpoint is the simple REST path for a client that cannot keep a socket open.
The two share the same cache keys.

### 9.3 Video

Video sessions are library-scoped:

```
POST   /libraries/{libraryId}/video/sessions
GET    /libraries/{libraryId}/video/sessions/{sessionId}
DELETE /libraries/{libraryId}/video/sessions/{sessionId}
GET    .../playlist.m3u8
GET    .../segments/{name}
```

A create request names one Entry.
The response carries the session id, duration, and playlist URL.
`GET` returns the current state, so a client can recover after a lost create response.
The concurrent session limit is enforced with `429` and `Retry-After`; the daemon does not silently evict the oldest session.

## 10. Ratings, Tags, and Preferences

### 10.1 Ratings and Tags

Ratings and tags are Entry-scoped data:

```
GET /libraries/{libraryId}/entries/rating?path=&view=
PUT /libraries/{libraryId}/entries/rating?path=&view=
GET /libraries/{libraryId}/entries/tags?path=&view=
PUT /libraries/{libraryId}/entries/tags?path=&view=
```

The `path` is in the query for both the read and the write, so the target is in one place.
A `PUT` body carries only the value:

```json
{ "rating": "good" }
```

```json
{ "tags": ["family", "trip"] }
```

`rating` is `good`, `bad`, or `none`; `none` clears it.
Only images and videos can be rated or tagged; a folder or other file is a `400`.
The server resolves the Entry's `contentId` and stores the mark against it.
A daemon-driven move or copy keeps the marks; a path reused by a different file does not inherit them.
An Entry without `contentId` cannot be rated or tagged.

`GET /tags/words` returns the user's tag vocabulary:

```json
{
  "words": ["family", "trip"]
}
```

A word remains after it is removed from the last Entry that used it.
The vocabulary is separate from the tags of one Entry.

### 10.2 Preferences

`GET /preferences` and `PUT /preferences` keep the full-replacement contract.
`GET` returns an `ETag`; `PUT` accepts `If-Match` and returns `412` when the saved value changed.
The read returns the defaults when nothing has been saved.

## 11. Errors, Pagination, and Idempotency

### 11.1 Errors

Every error uses `application/problem+json`:

```json
{
  "type": "https://pxna.dev/problems/conflict",
  "title": "Conflict",
  "status": 409,
  "code": "Conflict",
  "detail": "The destination already exists.",
  "instance": "/api/v2/entries/copy",
  "requestId": "req_...",
  "errors": [
    { "pointer": "/destination/path", "code": "AlreadyExists" }
  ]
}
```

- `code` is the stable client-facing branch, equivalent to the v1 `ErrorKind`.
- `detail` never contains a host path; host paths are logged by `requestId`.
- `errors` is present for field validation and names the JSON Pointer.
- A `404` from routing, a `405`, a `413`, a `415`, and a `429` use the same shape; there is no plain-text error.
- `401` carries `WWW-Authenticate` for a bearer request.
- `429`, `503`, and `504` carry `Retry-After`.

A per-item error inside a successful batch is a compact `{ code, detail? }`, not a Problem document.

### 11.2 Pagination

A page is the discriminated union in §6.2.
`limit` defaults to 200 and is at most 1,000.
An offset page returns `totalCount` and `snapshot`.
A cursor page returns `previous` and `next`; the cursors are base64url.
An `around` or `month` request returns the effective cursor page and an `anchor`.
A snapshot is opaque and changes when any listing that contributes to it changes.

### 11.3 Conditional Requests and Idempotency

- Content returns `ETag`, `Last-Modified`, and `Accept-Ranges`, and handles `If-None-Match`, `If-Modified-Since`, `If-Range`, and `Range`.
- A listing returns an `ETag` derived from its snapshot; `If-None-Match` returns `304`.
- Preferences return an `ETag`; `If-Match` protects a replacement.
- A destructive batch accepts `ifSnapshot`; a mismatch is `412`.
- `POST` mutations accept `Idempotency-Key`; the key and a hash of the request are stored with the Operation for 24 hours.
- A retry with the same key and body returns the original Operation with `202`; the same key with a different body is `422`.

## 12. Grouping and Tags

The v2 contract has eleven tags.
Each tag groups operations a reader would look for together, and each tag description covers every operation under it.

| Tag | Operations |
| --- | --- |
| Meta | `GET /health`, `GET /capabilities`, `GET /setup/status` |
| Auth | `POST /setup`, `POST /auth/login`, `GET/DELETE /auth/session`, `GET /auth/sessions`, `DELETE /auth/sessions/{sessionId}`, `PUT /auth/password` |
| Libraries | `GET /libraries`, `GET /libraries/{id}` |
| Entries | `POST /entries`, `/entries/months`, `/entries/metadata`, `GET .../entries/metadata`, `GET/HEAD/PUT .../entries/content`, `POST .../entries/files`, `POST .../entries/folders`, `GET .../entries/child-probe`, `POST /entries/copy`, `POST /entries/move`, `POST /entries/delete` |
| Uploads | The upload lifecycle |
| Operations | `GET /operations`, `GET /operations/{id}`, `POST /operations/{id}/cancel` |
| Timeline | `POST /timeline/scans`, `GET /libraries/{id}/timeline` |
| Media | `GET .../entries/media`, `GET .../entries/thumbnail`, `GET /sockets/image`, video sessions |
| Ratings & Tags | `GET/PUT .../entries/rating`, `GET/PUT .../entries/tags`, `GET /tags/words` |
| Preferences | `GET/PUT /preferences` |
| Performance | `POST /performance/client-timings` |

The following v1 groupings do not carry into v2:

- `GET /capabilities` leaves `Libraries`; it is a meta operation, not a Library read.
- `POST /libraries/{id}/entries/details` becomes `POST /entries/metadata` under Entries.
- `GET /libraries/{id}/media-info` becomes `GET .../entries/media` under Media; the tag is not `Video`.
- `GET /tags` becomes `GET /tags/words`; the vocabulary is separate from an Entry's tags.
- `GET /folders/children` becomes `GET .../entries/child-probe`; the name says it is a probe, not a listing.
- `Search` and `Folders` no longer own listing operations; listings are Entries. `Timeline` keeps only the scan and status operations; the timeline entry listing is Entries, and the scan itself is an Operation.
- `Files` and `Folders` no longer exist as tags; the Entry's `kind` and `view` carry the distinction.

The tag order is the order a reader follows: Meta, Auth, Libraries, Entries, Uploads, Operations, Timeline, Media, Ratings & Tags, Preferences, Performance.

## 13. Migration from v1

v1 and v2 are served together.
v1 is frozen: it receives only critical fixes, and every v1 response carries `Deprecation` and `Sunset` once a removal date exists.
The web client moves first, then the native client; v1 is removed after both have used v2 for one release.

| v1 | v2 |
| --- | --- |
| `GET /libraries/{id}/files?path=` | `GET /libraries/{id}/entries/metadata?path=&view=file` |
| `GET /libraries/{id}/folders?path=` | `GET /libraries/{id}/entries/metadata?path=&view=folder` |
| `GET/PUT /libraries/{id}/files/content` | `GET/PUT /libraries/{id}/entries/content` |
| `POST /libraries/{id}/files/copy` and `/folders/copy` | `POST /entries/copy` |
| `POST /libraries/{id}/files/move` and `/folders/move` | `POST /entries/move` |
| `POST /libraries/{id}/files/delete` and `/folders/delete` | `POST /entries/delete` |
| `GET /libraries/{id}/folders/entries` | `POST /entries` with one start |
| `GET /libraries/{id}/folders/summaries` | `POST /entries` with `fields=summary` |
| `GET /libraries/{id}/folders/children` | `GET /libraries/{id}/entries/child-probe` |
| `POST /libraries/{id}/entries/details` | `POST /entries/metadata` |
| `GET /search/entries` | `POST /entries` with the same Filter |
| `GET /timeline/entries` | `POST /entries` with `sort=timelineDate` |
| `GET /timeline/months` and `POST /entries/months` | `POST /entries/months` |
| `GET /timeline/status` and `POST /timeline/refresh` | `GET /libraries/{id}/timeline` and `POST /timeline/scans` |
| `GET/PUT /libraries/{id}/rating` | `GET/PUT /libraries/{id}/entries/rating` |
| `GET/PUT /libraries/{id}/tags` | `GET/PUT /libraries/{id}/entries/tags` |
| `GET /tags` | `GET /tags/words` |
| `GET /libraries/{id}/media-info` | `GET /libraries/{id}/entries/media` |

`daemon/openapi.yaml` remains the v1 contract.
The v2 contract is added as a separate document and a separate generated interface; a path in v1 is not silently redirected to v2.
The v2 implementation starts at the service layer and wraps the existing listing, mutation, upload, and operation services rather than duplicating them.

## 14. Design Decisions

### 14.1 Decided

#### Use one Entry resource with a view discriminator

**Decision**
Files, folders, links, archive files, and archive roots are all Entries.
A request that can mean either an archive file or its root carries `view=file` or `view=folder`.

**Reason**
The v1 split into `/files` and `/folders` doubles the mutation endpoints and makes the same `/a.zip` mean different things by endpoint.
A `view` on the reference keeps the archive ambiguity explicit without a second namespace.

**Rejected alternatives**
A trailing slash on the path was rejected because forgetting it would turn an archive-root operation into a file operation, which is unsafe for delete.
Keeping the two namespaces was rejected because it preserves six mutation endpoints and the source-library asymmetry.

#### Use POST for body-carrying reads and defer QUERY

**Decision**
`POST /entries`, `POST /entries/months`, and `POST /entries/metadata` are the read operations.
They change no state.
The HTTP `QUERY` method is not adopted for v2 now.

**Reason**
A Scope can carry 64 starts with paths up to 4,096 characters, and a metadata batch can carry 200 references; a URI cannot hold them reliably.
`QUERY` would express the intended safety, but the repository's OpenAPI 3.0.3 pipeline, custom generator, and axum 0.8 `MethodFilter` do not support it without hand-written routing and client work.
The v2 contract does not need the semantics yet, and the migration cost is not justified.
The cost is that automatic retry and cache semantics are not available; the contract must state that these POSTs are reads.

**Rejected alternatives**
`GET` with query parameters was rejected because the worst-case body does not fit a URL.
`QUERY` is recorded as a deferred item in §14.2 rather than rejected permanently.

#### Use one listing operation and one Entry page

**Decision**
`POST /entries` is the only listing operation.
Every listing returns `items`, `page`, and `warnings`, with the page as a discriminated union.

**Reason**
v1 has five listing operations, four page shapes, and three entry wrappers.
One operation makes the tree, search, and timeline share a store and a decoder.
The Scope and Filter already express the differences.

**Rejected alternatives**
Keeping the legacy search and timeline listing endpoints was rejected because they are subsets of the same Scope and Filter and double every fix.
Keeping separate envelopes was rejected because the client would still need one decoder per listing.

#### Return 202 with an Operation for every batch mutation

**Decision**
Copy, move, delete, and timeline scans return `202` with an Operation, even when the work finishes immediately.
The Operation body carries the terminal state when it is already done.

**Reason**
v1 chooses between `200` and `202` by source count and Library, so a client cannot use one retry rule.
A single status with a terminal body gives one rule and one place for `skipped`, `failed`, and `missing`.

**Rejected alternatives**
Synchronous `200` for small batches was rejected because the threshold is a server implementation detail and would still split the client.
Synchronous `204` for deletes was rejected because it cannot report partial results.

#### Use Problem Details for every error

**Decision**
All errors use `application/problem+json` with `code`, `detail`, `requestId`, and optional field errors.

**Reason**
v1 returns plain text for validation and extraction failures, empty bodies for routing failures, and a JSON `ErrorResponse` elsewhere.
A generated client cannot rely on one decoder.

**Rejected alternatives**
Keeping `ErrorResponse` and adding fields was rejected because the routing and extractor failures would still bypass it unless every path is wrapped; a new media type makes the boundary explicit.

#### Expose a cacheable REST thumbnail endpoint

**Decision**
v2 provides `GET /libraries/{libraryId}/entries/thumbnail?path=&view=&size=&shape=` as a cacheable image response with `ETag`.
The image socket remains the high-throughput path for a grid.

**Reason**
A client that cannot keep a WebSocket open still needs thumbnails, and an HTTP endpoint can be revalidated with the same cache keys as the socket.
A single REST path also makes the API usable by a generic client.

**Rejected alternatives**
Socket-only delivery was rejected because it forces every client to implement the binary protocol for a single image.
A separate thumbnail service was rejected because the socket already shares the cache keys.

#### Key ratings and tags by content identity

**Decision**
Ratings and tags are stored against `contentId` for images and videos.
The API still addresses the Entry by path.

**Reason**
A path changes on an external rename and can be reused by a different file.
Content identity keeps marks with the content and avoids inheriting a mark from a deleted file.

**Rejected alternatives**
Path-keyed marks were rejected because the loss on rename is visible to the user and cannot be repaired reliably after the fact.

#### Persist uploads and operations

**Decision**
Upload sessions and Operations are stored in SQLite and survive a daemon restart.

**Reason**
A client that loses a response can resume an upload or read the result of a copy after a restart.
Without persistence, a lost response or a restart leaves the client unable to distinguish an unknown id from a lost result.

**Rejected alternatives**
Keeping them in memory was rejected because a restart during a long copy or a large upload loses the only record of the work.

#### Serve v2 alongside v1 and freeze v1

**Decision**
`/api/v1` remains the current contract; `/api/v2` is added as a separate contract and generated interface.
v1 is frozen except for critical fixes and is removed after the web and native clients have used v2 for one release.

**Reason**
A breaking API change cannot be made in place for two clients.
A separate base path and a separate contract make the migration observable.

**Rejected alternatives**
Changing v1 in place was rejected because it breaks clients without a migration window.
A single generated interface for both versions was rejected because the tag and method split would make the generator and handlers ambiguous.

### 14.2 Deferred

#### Adopting the HTTP QUERY method

**Current**
`POST /entries`, `POST /entries/months`, and `POST /entries/metadata` are read-only POSTs.
They are safe and idempotent in effect, but HTTP cannot express that.

**Why it is not decided now**
RFC 10008 defines `QUERY`, and OpenAPI 3.2 has a `query` operation, but this repository uses OpenAPI 3.0.3 and a custom generator.
axum 0.8's `MethodFilter` has no `QUERY`, so routing would need a hand-written path or an `any` handler with a method check.
The v2 contract does not need the semantics yet, and no client currently needs automatic retry of a listing.

**Conditions for deciding**
Adopt `QUERY` when the OpenAPI pipeline supports query operations, axum has a first-class method filter, and a client needs the safe-method semantics for automatic retry or cache revalidation.
At that point, add a `QUERY` alias to the three read operations, keep `POST` during a migration window, and deprecate it.

#### Content identity for ratings and tags

**Current**
The v2 API shape uses `contentId`, but the hash input, the cache key, and the invalidation rule are not decided.

**Why it is not decided now**
The content hash depends on the M26 work in the roadmap and on how an archive Entry's content is keyed without re-reading the archive on every request.

**Conditions for deciding**
When M26 is implemented, define the hash input, the cache key, and how a content change invalidates a mark.

#### Removing v1

**Current**
v1 is the current contract and v2 is the target.

**Why it is not decided now**
The web client and the native client have different release cycles, and no removal date has been agreed.

**Conditions for deciding**
When both clients have used v2 for one release and no external client is known to call v1.
At that point, set `Sunset`, keep the v1 contract for read-only compatibility for one release, and remove it.

## 15. Current State and Remaining Work

v1 is implemented.
The current API is still v1 in `daemon/openapi.yaml`.
v2 is a target: the resource model, URL hierarchy, and contracts in §3 through §13 are not implemented.
`daemon/openapi.yaml` remains the authoritative contract until a v2 OpenAPI document is added.

The review and this target design are the prerequisites; they are already recorded.
The remaining work is expected in this order.
The order is provisional; only the dependencies must hold.

| Step | Content | Depends on |
| --- | --- | --- |
| A1 | Write the v2 OpenAPI document from §3 through §13 | None |
| A2 | Implement `EntryRef`, `view`, and effective readonly over the existing storage and listing services | A1 |
| A3 | Implement `POST /entries`, `/entries/months`, `/entries/metadata`, the Entry page, and the child probe | A2 |
| A4 | Implement `POST /entries/copy`, `/move`, `/delete`, and the Operation result | A2 |
| A5 | Implement upload listing and commit conflict handling, and persist uploads and Operations | A4 |
| A6 | Implement Problem Details, idempotency keys, conditional requests, and the v2 tag set | A1 |
| A7 | Migrate the web client, then the native client; freeze and later remove v1 | A3, A4, A5, A6 |

The v2 implementation wraps the existing services.
It does not duplicate path resolution, listing snapshots, the name index, timeline aggregation, or staging.

Defects are not recorded here.
See [issues.md](../issues.md).
