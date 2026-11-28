# API Design

## 1. About This Document

This document defines the structure of the Pxna HTTP API that replaces the current contract in place: the resource model, URL hierarchy, method conventions, listing and mutation contracts, upload and operation lifecycles, error shape, tag grouping, and the mapping from the current paths.
The intended readers are developers changing the API contract, the generated interface, the daemon handlers, or a client.

This document assumes the vocabulary of [terms.md](../terms.md#2-term-list).
Definitions of words are not in this document.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The resource model, URL hierarchy, method conventions, listing and mutation contracts, upload and operation lifecycles, error shape, tag grouping, and the mapping from the current paths, together with the decisions behind them |
| [design.md](../design.md#8-conventions-for-mutating-apis) | The conventions of the current contract for mutating APIs, the error kinds and their statuses (§4.4), CSRF (§8.5), and the cross-cutting decisions this document leaves in place |
| [daemon/openapi.yaml](../../daemon/openapi.yaml) | The authoritative contract that is served: paths, parameters, schemas, status codes |
| [Storage Layer Design](./storage.md#1-about-this-document) | Path and archive resolution, staging, atomic publication, and the limits this document assumes |
| [Listing and Search Design](./listing.md#1-about-this-document) | Snapshots, name matching, the name index, and recursion limits behind the listing contract |
| [Timeline Design](./timeline.md#1-about-this-document) | Timeline dates, aggregation, and the timeline index behind the timeline contract |
| [Video Playback Design](./video.md#1-about-this-document) | Transcode sessions and their concurrency |
| [Image Socket Design](./image-socket.md#1-about-this-document) | The only path for thumbnails, sprite sheets, and display images |
| [Rating and Tag Design](./rating-tags.md#1-about-this-document) | What can be rated and tagged, value rules, and the content hash that identifies marks |
| [Client Design](./client.md#1-about-this-document) | How the web client consumes the API |
| [terms.md](../terms.md#2-term-list) | Definitions of terms, spelling, aliases, and boundaries between neighboring terms |
| [issues.md](../issues.md) | Confirmed defects. This document is not a defect list |

[design.md](../design.md#11-division-of-responsibility-between-documents) holds the map of the document family.

### 1.2 Tense of This Document

§3 through §13 describe the finished API in the present tense.
§14 records the decisions and deferred items behind it.
§15 records what is implemented, and which statements in other design documents still describe the current contract.
If you want to know what works now, read §15 first.

## 2. Responsibility and Boundary

The API layer owns the HTTP contract: how a Library and an Entry are addressed, which method expresses which intent, how a page is selected, how a batch command reports its result, how an error is shaped, and how operations are grouped.
It does not own how a path is resolved inside an archive, how a listing snapshot is built, how the name index answers a condition, how a timeline date is chosen, how a thumbnail is generated, or how marks are stored.
Those concerns stay in the documents of §1.1.

The API stays under `/api/v1`, and `daemon/openapi.yaml` is rewritten in place; no second version is served alongside it (§14.1).
Every capability of the current contract has a place in this structure, and the structure adds none of its own; the additions it leaves out are deferred in §14.2.
The deployment premise is unchanged: a single owner, a LAN-first daemon, and the web client served from the same origin as the API ([design.md §10.1](../design.md#101-web)).

## 3. Resource Model

### 3.1 Entry and EntryRef

The API keeps one Entry type, `FileSystemEntry`, and one namespace for it; files and folders are not separate top-level resources.

A request names an Entry by its Library id and its Library-relative path.
Where a path has two faces, the request also carries `view`, and the three together are an `EntryRef`.
Only an archive has two faces: `view=file` names the archive file, and `view=folder` names its root.
For a folder, `view=folder` is the only valid value; for any other file, `view=file` is.
When `view` is omitted, a folder is `folder` and every file, an archive included, is `file`, so an omitted `view` never lists, copies, or deletes the inside of an archive by accident.

`view` appears only where both faces are meaningful: the single Entry read (§6.5), and the sources and targets of copy, move, and delete (§7).
An operation that reads or writes one face only does not take it:

| Face | Operations |
| --- | --- |
| Folder only | The starts of a Scope, the child probe, folder creation, and the folder settings. An archive given here is its root |
| File only | Content, media information, ratings, and tags. An archive given here is the archive file |

The readonly rules are those of [design.md §8.6](../design.md#86-archives-and-readonly).

### 3.2 Library and Path

A path is always relative to the Library named in the same request, starts with `/`, and is checked by the Library boundary of [design.md §4.3](../design.md#43-resolving-library-and-archive-boundaries).
An archive's contents are addressed by joining paths, as in `/photos/a.zip/dir/x.png`.

A path is an address, not an identity.
A rename changes it, and ratings and tags are identified inside the daemon by the content hash of [Rating and Tag Design §6.1](./rating-tags.md#61-what-is-stored), which the API does not expose.

### 3.3 Scope and Filter

A listing request carries a Scope, a Filter, the listing options, the sort, and a page.

The Scope names what is read:

| Field | Meaning |
| --- | --- |
| `starts` | One to 64 places, each a Library id and a Library-relative path, read as folders |
| `recursive` | Whether the descendants of each start are read |
| `insideArchives` | Whether archives under the starts are read like folders; nothing without `recursive` |

The Filter names what is kept: `query`, `rating`, `tags`, and `kinds`, with the meanings of [Listing and Search Design §3](./listing.md#3-sorting-and-filtering) and [Rating and Tag Design §7.1](./rating-tags.md#71-filtering-listings).

The listing options are `foldersFirst`, `includeHidden`, and `fields`.
`includeHidden` reads the Entries whose names start with `.` and the Entries inside them, and is `false` by default.
`fields` is `full` by default, or `summary` for the summary of direct children (§6.1).
The sort is `name`, `kind`, `size`, `modified_at`, or `timeline_date`, and `order` is `asc` or `desc`; `timeline_date` is descending only.

A saved Shelf holds a Scope and a Filter of these shapes, so a Shelf is read by sending them as it holds them.
`includeHidden` is an option rather than a part of the Scope (§14.1), so a Shelf keeps excluding hidden Entries; the tree is the listing that sets it.
A snapshot is still built per value of it ([Listing and Search Design §4.1](./listing.md#41-the-unit-that-is-built)), and a recursive listing that reads hidden Entries is walked, because the name index holds no hidden Entries.
With `sort=timeline_date`, `includeHidden=true` is `400`, because the timeline index holds no hidden Entries either and the listing could not return them.

## 4. URL Hierarchy

All paths are relative to `/api/v1`.

```text
/api/v1
├─ GET    /health
├─ GET    /capabilities
├─ GET    /setup/status
├─ POST   /setup
├─ POST   /auth/login
├─ GET    /auth/session
├─ DELETE /auth/session
├─ GET    /preferences
├─ PUT    /preferences
├─ GET    /shelves
├─ POST   /shelves
├─ PUT    /shelves/order
├─ PUT    /shelves/{shelfId}
├─ DELETE /shelves/{shelfId}
├─ GET    /folder-settings
├─ GET    /libraries
├─ GET    /libraries/{libraryId}
│
├─ GET    /libraries/{libraryId}/entries/metadata?path=&view=
├─ GET    /libraries/{libraryId}/entries/content?path=
├─ PUT    /libraries/{libraryId}/entries/content?path=&conflict=
├─ POST   /libraries/{libraryId}/entries/files?path=
├─ POST   /libraries/{libraryId}/entries/folders
├─ GET    /libraries/{libraryId}/entries/child-probe?path=&includeHidden=
├─ GET    /libraries/{libraryId}/entries/media?path=
├─ GET    /libraries/{libraryId}/entries/rating?path=
├─ PUT    /libraries/{libraryId}/entries/rating?path=
├─ GET    /libraries/{libraryId}/entries/tags?path=
├─ PUT    /libraries/{libraryId}/entries/tags?path=
├─ PUT    /libraries/{libraryId}/entries/folder-settings?path=
├─ DELETE /libraries/{libraryId}/entries/folder-settings?path=
│
├─ POST   /libraries/{libraryId}/uploads
├─ GET    /libraries/{libraryId}/uploads/{uploadId}
├─ DELETE /libraries/{libraryId}/uploads/{uploadId}
├─ PUT    /libraries/{libraryId}/uploads/{uploadId}/content?offset=
├─ POST   /libraries/{libraryId}/uploads/{uploadId}/commit
│
├─ POST   /libraries/{libraryId}/video/sessions
├─ GET    /libraries/{libraryId}/video/sessions/{sessionId}
├─ DELETE /libraries/{libraryId}/video/sessions/{sessionId}
├─ GET    /libraries/{libraryId}/video/sessions/{sessionId}/playlist.m3u8
├─ GET    /libraries/{libraryId}/video/sessions/{sessionId}/segments/{segmentName}
│
├─ POST   /entries
├─ POST   /entries/months
├─ POST   /entries/metadata
├─ POST   /entries/copy
├─ POST   /entries/move
├─ POST   /entries/delete
│
├─ GET    /timeline/status?libraryIds=
├─ POST   /timeline/scans
│
├─ GET    /operations
├─ GET    /operations/{operationId}
├─ POST   /operations/{operationId}/cancel
│
├─ GET    /tags/words
├─ GET    /sockets/image
└─ POST   /performance/client-timings
```

The hierarchy follows one rule: an operation on one Entry, or on one Library's upload or video session, is under `/libraries/{libraryId}` with the path in the query, while a read or a command whose body names its Libraries is global.
The rule is what lets `POST /entries/copy` carry its source and its destination in the same shape, and what puts a rating, the tags, and the folder settings of one Entry beside its metadata.
The daemon's own state that names no Entry, such as the display preferences, the Shelves, and the list of folder settings, is global as well.

## 5. Method Conventions

### 5.1 Read Methods

`GET` is used for every read that a URI can carry: one Entry's metadata, content, media information, rating, tags, and folder settings, and the status and lists of the daemon's own state.

A read that carries a structured body uses `POST`.
The three operations are `POST /entries`, `POST /entries/months`, and `POST /entries/metadata`, and they change no state.
A Scope may name 64 starts with paths of up to 4,096 characters, and a metadata batch 200 places, which a URI cannot carry reliably.
These reads meet the same `Origin` check as every other `POST` that carries a session cookie ([design.md §8.5](../design.md#85-csrf)); the web client sends `Origin` on them, and bearer requests are not checked.

The HTTP `QUERY` method would state that these reads are safe and idempotent; it is deferred in §14.2.
A client treats the three operations as reads by contract, not by method.

### 5.2 Mutation Methods

`PUT` replaces a target that the request identifies, and its effect is idempotent.
It covers content at an exact path, the display preferences, a Shelf, the order of the Shelves, an Entry's rating, an Entry's tags, and a folder's settings.

`POST` creates under a name the server may choose, starts a batch command, or performs an action.
It covers file creation under a chosen name, folder creation, Shelf creation, copy, move, delete, upload sessions, video sessions, login, setup, timeline scans, and cancellation.

`DELETE` removes a resource that its URL identifies: the current session, a Shelf, a folder's settings, an upload session, and a video session.
Deleting Entries is the batch command `POST /entries/delete`, because its targets and its `recursive` flag are carried together ([design.md §8.4](../design.md#84-deletion)).

### 5.3 Status Codes

The statuses are those of the current contract, with the error kinds mapped as in [design.md §4.4](../design.md#44-error-model-and-http-mapping).
This structure fixes three of them.

- Every batch command answers `202` with an Operation, including one that finishes before the response (§7.4).
- A request for a video session that the daemon ended to make way for another is `410` (§9.3).
- A missing route, a method the route does not allow, and a body the generated router rejects answer with the Problem shape of §11.1, not with an empty or plain-text body.

## 6. Listings

### 6.1 One Listing API

`POST /entries` is the only listing operation.
A folder, a search, the timeline, a Shelf, and the summary of a folder's direct children are all one Scope, Filter, sort, and page:

| Listing | Request |
| --- | --- |
| A folder | One start, `recursive=false` |
| The tree | One start, `recursive=false`, `kinds` of `folder` and `archive`, `includeHidden` as the tree shows |
| A search | The chosen starts, `recursive=true`, the Filter |
| The timeline | The chosen Libraries' roots, `recursive=true`, `insideArchives=true`, `kinds` of `image` and `video`, `sort=timeline_date` |
| A Shelf | The Shelf's Scope and Filter as saved |
| The summary of direct children | One start, `recursive=false`, `fields=summary`, `sort` of `name` or `kind` |

A request has this shape:

```json
{
  "scope": {
    "starts": [{ "libraryId": "photos", "path": "/" }],
    "recursive": true,
    "insideArchives": true
  },
  "filter": {
    "query": "trip",
    "rating": "good",
    "tags": ["family"],
    "kinds": ["image", "video"]
  },
  "options": { "foldersFirst": false, "includeHidden": false, "fields": "full" },
  "sort": "timeline_date",
  "order": "desc",
  "page": { "kind": "cursor", "limit": 200 }
}
```

The listing reads the same snapshots, name index, and timeline index as before ([Listing and Search Design](./listing.md#1-about-this-document)); this structure changes how a listing is requested and returned, not how it is built.

`fields=summary` keeps the rules of the summary listing ([Listing and Search Design §8.1](./listing.md#make-the-summary-listing-of-direct-children-a-separate-response-from-the-existing-detail-listing)): it reads the direct children of one start in name or kind order, and the page carries `summaries` in place of `items`, each with the path, name, kind, and archive hint, not `FileSystemEntry` values with placeholder sizes and times.
A summary request with `recursive`, several starts, or another sort is `400`.
The details of the listed Entries are read with `POST /entries/metadata` (§6.5).

### 6.2 Page Anchors

A page request is a discriminated union:

| `kind` | Anchors | Valid with |
| --- | --- | --- |
| `offset` | `offset`, or `locate` naming an Entry | Every sort except `timeline_date` |
| `cursor` | `after` or `before`, `month`, or `around` naming an Entry | `timeline_date` |

At most one anchor is given, and `limit` defaults to 200 with a maximum of 1,000.
The rules of each anchor, and why the two kinds exist, are in [Listing and Search Design §4.5](./listing.md#45-locating-an-entry), [§4.6](./listing.md#46-pages-in-timeline-date-order), and the decision "Cut Pages by Offset, and by Cursor in Timeline-Date Order" there.

### 6.3 Entry Page

Every listing returns the same envelope:

```json
{
  "items": [
    {
      "libraryId": "photos",
      "entry": { "...": "FileSystemEntry" },
      "timelineDate": "2026-05-01T12:34:56",
      "timelineDateSource": "exif"
    }
  ],
  "page": { "kind": "offset", "offset": 0, "totalCount": 1234, "snapshot": "..." },
  "warnings": {
    "cutOffStarts": [],
    "walkedStarts": [],
    "missingStarts": [{ "libraryId": "photos", "path": "/missing" }]
  }
}
```

- Every item carries its `libraryId`, so a listing of one start has the same type as a listing of several. A summary carries `summaries`, each a `libraryId` and a `summary`, in place of `items`.
- `timelineDate` and `timelineDateSource` are present only with `sort=timeline_date`, in the form of [Timeline Design §4](./timeline.md#4-timeline-dates).
- `page` echoes the effective page. An `offset` page carries `offset`, `totalCount`, and `snapshot`; a `cursor` page carries `previous` and `next`, either absent at an end.
- `snapshot` identifies the snapshot an `offset` page was cut from, and a client detects a change by comparing it between pages, as it does now. A `cursor` page has none, because it follows the timeline index by cursor rather than a snapshot ([Listing and Search Design §4.6](./listing.md#46-pages-in-timeline-date-order)).
- The response carries no `ETag`, because a `POST` is not revalidated.
- `warnings` is always present, with the three lists of starts that [Listing and Search Design §5.3](./listing.md#53-listing-several-starts) defines.

### 6.4 Child Probe

`GET /libraries/{libraryId}/entries/child-probe?path=&includeHidden=` returns whether a folder has a child to show in the tree, as `{hasChildren, maxAgeMs}`.

`hasChildren` is `true`, `false`, or `null`.
`false` means the probe reached the end without a child; `null` means the answer is not known now, and a client treats it as "may have children", not as `false`.
`maxAgeMs` is how long the answer may be reused.
The name says that it is a probe, not a listing; the children themselves are read with `POST /entries`.

### 6.5 Months and Metadata

`POST /entries/months` takes the same Scope and Filter as `POST /entries` and returns the count of each month that the matching `timeline_date` listing would return.

`GET /libraries/{libraryId}/entries/metadata?path=&view=` returns one Entry.
It replaces the separate file and folder reads, and `view` chooses an archive's face (§3.1).

`POST /entries/metadata` takes 1 to 200 places, each a Library id and a Library-relative path, and returns one result per place in request order, with its `libraryId` and `path`, and either `detail`, the Entry as a listing shows it, or `error`, an error kind. A Library that does not exist fails only its places, and a place given twice, after its path is normalized, is `400`.
The request succeeds with `200` even when some places fail, and the rules of the current batch ([Listing and Search Design §8.1](./listing.md#make-the-summary-listing-of-direct-children-a-separate-response-from-the-existing-detail-listing)) apply unchanged.

## 7. Mutations

### 7.1 Copy and Move

`POST /entries/copy` and `POST /entries/move` take the same body:

```json
{
  "sources": [
    { "libraryId": "photos", "path": "/a.jpg" },
    { "libraryId": "photos", "path": "/2026/a.zip", "view": "folder" }
  ],
  "destination": { "libraryId": "backup", "folderPath": "/2026" },
  "conflict": "fail"
}
```

- `sources` is 1 to 1,000 `EntryRef` values in one Library. A source with `view=folder` is a folder, or an archive's root whose contents are copied or moved; any other source is a file, an archive file included.
- `destination` names its Library and exactly one of `folderPath`, into which each source keeps its name, or `path`, an exact target valid only with one source. A destination is never inside an archive, so it takes no `view`.
- `conflict` is the policy of [design.md §8.3](../design.md#83-conflict-policy), and the tree rules there apply to a folder source.
- The checks the current commands make before they start keep their statuses. The response is then `202` with an Operation (§7.4).

A mixed selection of files and folders is one request.

### 7.2 Delete

`POST /entries/delete` takes 1 to 1,000 `EntryRef` targets in one Library and `recursive`:

```json
{
  "targets": [
    { "libraryId": "photos", "path": "/a.jpg" },
    { "libraryId": "photos", "path": "/old", "view": "folder" }
  ],
  "recursive": true
}
```

The rules of [design.md §8.4](../design.md#84-deletion) apply: a non-empty folder needs `recursive=true`, and a target in a `readonly` Library or inside an archive is `403`.
An archive's root (`view=folder` on an archive) is inside the archive, so deleting it is `403`; the archive itself is deleted with `view=file`.
The response is `202` with an Operation.

### 7.3 Write and Create

`PUT /libraries/{libraryId}/entries/content?path=&conflict=` writes the body to the exact path.
`conflict` is `fail`, `skip`, or `overwrite`; `rename` is not accepted, because a write that may publish under another name is not idempotent.
The size limit and the switch to an upload session are those of [design.md §8.7](../design.md#87-uploads).

`POST /libraries/{libraryId}/entries/files?path=` writes the body under the path, or under a name the daemon chooses when the path exists, and returns `201` with the published Entry.
It is the former `conflict=rename` of the one-shot write.

`POST /libraries/{libraryId}/entries/folders` creates a folder from `{path, createParents}` and returns `201` with the Entry.

### 7.4 Operation Results

An Operation is the one result of every batch command: copy, move, delete, and timeline scan.
A command always answers `202` with `{operation}`, whether or not the work has finished.
When it has, the body already carries the terminal status and the result, and a client does not poll.

The Operation keeps the fields and states of [design.md §8.3](../design.md#83-conflict-policy) and [§8.8](../design.md#88-operations): `processedCount`, `totalCount`, `skippedCount`, `failedEntries`, the published `resultEntries`, and the destination Library.
A failure in one entry does not discard the others.
Operations stay in memory, as they are now; persisting them is deferred in [design.md §11.2](../design.md#persisting-operations).

## 8. Uploads and Operations

### 8.1 Uploads

An upload session keeps its library-scoped lifecycle and its rules ([design.md §8.7](../design.md#87-uploads)): create, read, append a chunk at `offset`, commit, and delete.
A session is a write that the client advances explicitly, not an Operation.

### 8.2 Operations

`GET /operations`, `GET /operations/{operationId}`, and `POST /operations/{operationId}/cancel` serve every batch command, with the state transitions of [design.md §8.8](../design.md#88-operations).

The API therefore has three lifecycles with distinct meanings: the batch Operation, the upload session, and the streaming video session.

## 9. Timeline, Media, and Video

### 9.1 Timeline

The timeline is read with `POST /entries` and `sort=timeline_date` (§6.1), and its months with `POST /entries/months`; there is no separate timeline listing.

`POST /timeline/scans` takes `{libraryIds}`, starts aggregation of each Library as the current refresh does ([Timeline Design §6.3](./timeline.md#63-when-aggregation-starts)), and answers `202` with an Operation of kind `timelineScan`.
A Library already being aggregated joins its running scan instead of starting another.
Aggregation that the daemon starts by itself creates no Operation.
`GET /timeline/status?libraryIds=` keeps its per-Library projection of whether aggregation runs, how many files it has processed, and when it last completed, for a client that shows the state without holding an Operation.

### 9.2 Media

`GET /libraries/{libraryId}/entries/media?path=` returns the media information of an image or a video, as `getMediaInfo` does now ([Video Playback Design §3](./video.md#3-media-info)).
It belongs to the Media tag with the image socket and the video sessions.

Thumbnails, sprite sheets, and display images have no HTTP path; they are served only over the image socket ([Image Socket Design §2.1](./image-socket.md#21-what-is-not-built)).

### 9.3 Video

Video sessions keep their library-scoped lifecycle: create, read, delete, the playlist, and the segments.

When a new session would exceed `max_concurrent_transcodes`, the daemon still ends the oldest session to make way ([Video Playback Design §5.4](./video.md#54-concurrency)).
The ended session does not disappear silently: its read, playlist, and segments answer `410` with the error kind `SessionEvicted`, so the client that owned it can tell an ended session from one that never existed or expired, which stay `404`.
The daemon remembers an ended session's id for `session_idle_timeout_secs` ([Video Playback Design §5.1](./video.md#51-transcode-sessions)), the time after which an idle session would have expired anyway, so the record is bounded by the same limit as the sessions.

## 10. Ratings, Tags, Folder Settings, and Preferences

### 10.1 Ratings and Tags

A rating and the tags are data of one image or video, addressed beside its metadata:

```text
GET /libraries/{libraryId}/entries/rating?path=
PUT /libraries/{libraryId}/entries/rating?path=
GET /libraries/{libraryId}/entries/tags?path=
PUT /libraries/{libraryId}/entries/tags?path=
```

The path is in the query for both the read and the write, and a `PUT` body carries only the value, `{"rating": "good"}` or `{"tags": ["family", "trip"]}`.
The value rules and the targets are those of [Rating and Tag Design](./rating-tags.md#3-targets); the daemon resolves the Entry to its content hash before reading or writing.

`GET /tags/words` returns the user's vocabulary, which is separate from the tags of one Entry.

### 10.2 Preferences

`GET /preferences` and `PUT /preferences` keep the full-replacement contract.

### 10.3 Shelves and Folder Settings

The Shelves keep their paths and their whole-replacement rules ([design.md §6.5](../design.md#65-shelves-and-folder-settings)); a Shelf's `scope` and `filter` are the Scope and the Filter of §3.3.

The folder settings are data of one folder, addressed like a rating:

```text
GET    /folder-settings
PUT    /libraries/{libraryId}/entries/folder-settings?path=
DELETE /libraries/{libraryId}/entries/folder-settings?path=
```

`GET /folder-settings` returns every folder setting of the user with its Library id and path, because the client resolves the values of a folder itself.
A `PUT` body carries only the display values, and a `DELETE` removes the folder's settings.

## 11. Errors and Conditional Requests

### 11.1 Errors

Every error is `application/problem+json`:

```json
{
  "type": "about:blank",
  "title": "Conflict",
  "status": 409,
  "code": "Conflict",
  "detail": "The destination already exists.",
  "requestId": "..."
}
```

- `code` is the API error kind of [design.md §4.4](../design.md#44-error-model-and-http-mapping), and a client branches on it.
- `detail` never contains a host path; the daemon logs an internal error, with its details, under its `requestId`.
- A body that the generated router rejects carries the router's reason in `detail`; the router reports no field, so the Problem names none.
- A missing route, a method not allowed, a body the generated router rejects, an authentication failure, and a size limit use the same shape; no error is plain text or an empty body.
- `requestId` is present on every error and distinct per error.

A per-item failure inside a successful response, such as one place of `POST /entries/metadata`, is the error kind alone, not a Problem.

### 11.2 Conditional Requests

Content keeps `ETag`, `Last-Modified`, `Range`, and the conditional reads of the current contract.
No other response carries an `ETag`; an `offset` page of a listing detects changes by `snapshot` (§6.3).
Preconditions on writes, such as `If-Match` on the display preferences or a snapshot that a destructive command must still match, are deferred in [design.md §11.2](../design.md#optimistic-locking-with-etag--if-match).

## 12. Grouping and Tags

The contract has twelve tags.
Each groups the operations a reader looks for together, and each tag description covers every operation under it.

| Tag | Operations |
| --- | --- |
| Meta | `GET /health`, `GET /capabilities`, `GET /setup/status` |
| Auth | `POST /setup`, `POST /auth/login`, `GET /auth/session`, `DELETE /auth/session` |
| Libraries | `GET /libraries`, `GET /libraries/{libraryId}` |
| Entries | `POST /entries`, `/entries/months`, `/entries/metadata`, `/entries/copy`, `/entries/move`, `/entries/delete`, and `.../entries/metadata`, `content`, `files`, `folders`, `child-probe` |
| Uploads | The upload lifecycle |
| Operations | `GET /operations`, `GET /operations/{operationId}`, `POST /operations/{operationId}/cancel` |
| Timeline | `GET /timeline/status`, `POST /timeline/scans` |
| Media | `.../entries/media`, `GET /sockets/image`, and the video sessions |
| Ratings & Tags | `.../entries/rating`, `.../entries/tags`, `GET /tags/words` |
| Shelves | The Shelves, `GET /folder-settings`, and `.../entries/folder-settings` |
| Preferences | `GET /preferences`, `PUT /preferences` |
| Performance | `POST /performance/client-timings` |

The tags are ordered as a reader follows them: Meta, Auth, Libraries, Entries, Uploads, Operations, Timeline, Media, Ratings & Tags, Shelves, Preferences, Performance.
The generated interface is split by tag, so the same grouping decides where the handlers and their tests live ([design.md §4.2](../design.md#42-openapi-driven-code-generation)).

## 13. Mapping from the Current Contract

The contract is replaced in place, area by area, and each area moves its daemon handlers, its contract, and the web client together.
The iOS app follows the contract it is built against, in its own repository.

| Current | Replacement |
| --- | --- |
| `GET /auth/me` | `GET /auth/session` |
| `GET /libraries/{id}/files?path=` | `GET /libraries/{id}/entries/metadata?path=` |
| `GET /libraries/{id}/folders?path=` | `GET /libraries/{id}/entries/metadata?path=&view=folder` |
| `GET /libraries/{id}/files/content` | `GET /libraries/{id}/entries/content` |
| `PUT /libraries/{id}/files/content` with `conflict` of `fail`, `skip`, or `overwrite` | `PUT /libraries/{id}/entries/content` |
| `PUT /libraries/{id}/files/content` with `conflict=rename` | `POST /libraries/{id}/entries/files` |
| `POST /libraries/{id}/folders` | `POST /libraries/{id}/entries/folders` |
| `POST /libraries/{id}/files/copy` and `/folders/copy` | `POST /entries/copy` |
| `POST /libraries/{id}/files/move` and `/folders/move` | `POST /entries/move` |
| `POST /libraries/{id}/files/delete` and `/folders/delete` | `POST /entries/delete` |
| `GET /libraries/{id}/folders/entries` | `POST /entries` with one start |
| `GET /libraries/{id}/folders/summaries` | `POST /entries` with `fields=summary` |
| `GET /libraries/{id}/folders/children` | `GET /libraries/{id}/entries/child-probe` |
| `POST /libraries/{id}/entries/details` | `POST /entries/metadata` |
| `GET /search/entries` | `POST /entries` with the same Filter |
| `GET /timeline/entries` | `POST /entries` with `sort=timeline_date` |
| `GET /timeline/months` | `POST /entries/months` |
| `POST /timeline/refresh` | `POST /timeline/scans` |
| `GET /libraries/{id}/media-info` | `GET /libraries/{id}/entries/media` |
| `GET/PUT /libraries/{id}/rating` | `GET/PUT /libraries/{id}/entries/rating` |
| `GET/PUT /libraries/{id}/tags` | `GET/PUT /libraries/{id}/entries/tags` |
| `GET /tags` | `GET /tags/words` |
| `GET /settings/folder` | `GET /folder-settings` |
| `PUT /settings/folder` and `POST /settings/folder/delete` | `PUT` and `DELETE /libraries/{id}/entries/folder-settings` |

Paths not in the table keep their place.
No current path is redirected; a removed path answers `404` in the Problem shape.

## 14. Design Decisions

### 14.1 Decided

#### Replace the Contract in Place Instead of Serving a Second Version

**Decision**
The structure of this document replaces `daemon/openapi.yaml` under `/api/v1`, area by area, with the daemon and the web client changed in the same change.
No second version is served alongside it, and removed paths carry no deprecation period.

**Reason**
Pxna is not yet in use, the web client ships in the same image as the daemon, and the iOS app follows the contract it is built against, so no client needs the old paths after a change.
A second version would double the contract, the generated interface, and the handlers for as long as both are served, to protect clients that do not exist.

**Rejected alternatives**
Serving `/api/v2` beside a frozen `/api/v1` with `Deprecation` and `Sunset` was rejected for that cost.
Building the whole replacement first and switching the web client at once was rejected because the web client would be broken on `main` until the switch.

#### Keep One Entry Namespace, with a View Where a Path Has Two Faces

**Decision**
Files, folders, links, archive files, and archive roots are all Entries in one namespace.
A request that can mean either an archive file or its root carries `view`, and an omitted `view` means the archive file (§3.1).
This replaces the separate namespaces of [design.md §8.1](../design.md#81-namespaces).

**Reason**
The two namespaces doubled the copy, move, and delete commands, split a mixed selection into two requests, and made the same `/a.zip` mean different things by endpoint.
`view` keeps the archive ambiguity explicit in the request, and only where it exists.
The validation that the namespaces chose by endpoint is chosen by the face the request names.

**Rejected alternatives**
A trailing slash on the path was rejected because forgetting it would turn an operation on an archive's root into one on the archive file, which is unsafe for delete.
Keeping the namespaces and only aligning the transfer body was rejected because it keeps six commands and two requests per mixed selection.
Carrying `view` on every request was rejected because an operation with one face would accept a value it can only refuse.

#### Address One Entry Under Its Library with the Path in the Query

**Decision**
An operation on one Entry, including the daemon's own data about it (the rating, the tags, and the folder settings), is under `/libraries/{libraryId}/entries/` with the path in the query.
A read or a command whose body names its Libraries is global.
This replaced the earlier decision that named a folder of the folder settings in the body of requests under `/settings/folder`.

**Reason**
The current contract put the same concept in different places: the path of a rating was in the query to read and in the body to write, and the folder settings named a folder in a body while the rating named an Entry in the query.
One rule, decided by what an operation targets rather than by whether it changes a Library, lets a client build every single-Entry request the same way.
The objection that a folder is named in two parts, unlike a start, now applies equally to every single-Entry operation, so it no longer marks an exception.

**Rejected alternatives**
Keeping the folder settings under `/settings/folder` with the folder in the body was rejected because it would be the one single-Entry operation in another shape.
Putting every single-Entry operation in a body was rejected because a `GET` has no body, and the reads would become `POST`.

#### Use POST for Body-Carrying Reads, Check Them Like Any POST, and Defer QUERY

**Decision**
`POST /entries`, `POST /entries/months`, and `POST /entries/metadata` are reads that change no state.
They meet the `Origin` check of [design.md §8.5](../design.md#85-csrf) like every other `POST`.

**Reason**
A Scope of 64 starts and a batch of 200 places do not fit a URI reliably, and the decision "Carry a Scope and a Filter in the JSON Body of One POST Request" of [Listing and Search Design §8.1](./listing.md#carry-a-scope-and-a-filter-in-the-json-body-of-one-post-request) already chose a body for them.
The check depends only on the method and the session cookie; the web client sends `Origin` on every `POST`, and bearer requests are not checked, so an exception for these paths would help no client.

**Rejected alternatives**
Exempting the three paths from the `Origin` check was rejected because it adds a path-based exception to a security check for no client.
`QUERY` is deferred in §14.2, not rejected.

#### List Through One Operation into One Entry Page

**Decision**
`POST /entries` is the only listing operation, and every listing returns `items`, a discriminated `page`, and `warnings` (§6).
An `offset` page carries `snapshot` for detecting a change, as now; listings carry no `ETag`.

**Reason**
The current contract has five listing operations, four page envelopes, and three entry wrappers for the same Entries, so a fix to a page field is repeated per schema and a client keeps one store per listing.
The Scope and the Filter already express every difference, and the summary of direct children is a projection of the same request.
A `POST` is not revalidated by HTTP caches, and the web client already compares the `snapshot` of offset pages.

**Rejected alternatives**
Keeping a `GET` for one folder beside `POST /entries` was rejected because a listing would again have two entrances, and no client relies on HTTP caching of a listing.
Keeping the per-use operations with one shared envelope was rejected because their filters and defaults would still differ.
An `ETag` with `304` on the listing was rejected because a conditional `POST` is outside HTTP's caching model.

#### Carry `includeHidden` as a Listing Option, Not in the Scope

**Decision**
`includeHidden` is a listing option of `POST /entries` (§3.3), and a Scope has no field for it.

**Reason**
A Shelf holds a Scope and a Filter and is read by sending them as it holds them; with the field in the Scope, the Shelves' table would have to store it and every saved Shelf would gain a value it has never had.
The tree is the only listing that reads hidden Entries, and it is not a Shelf.

**Rejected alternatives**
Putting `includeHidden` in the Scope, where `recursive` is, was rejected because it changes the stored form of the Shelves for a value no Shelf uses.

#### Keep the Timeline Status a Global Read over Several Libraries

**Decision**
The status of aggregation stays `GET /timeline/status?libraryIds=`, returning one projection per Library.

**Reason**
The web client shows the state of the chosen Libraries together, and the rule of §4 puts a read that names its Libraries under the global path.

**Rejected alternatives**
`GET /libraries/{libraryId}/timeline` per Library was rejected because a client would send one request per Library for one screen.

#### Group Operations into Twelve Tags by the Resource a Reader Looks For

**Decision**
The contract has the twelve tags of §12. The Shelves and the folder settings keep a tag of their own, `GET /capabilities` and `GET /setup/status` move to Meta, and the session read is `GET /auth/session`, beside `DELETE /auth/session`.

**Reason**
The current tags follow the order in which features were added: a reader looking for a listing has to open four tags, image media information sits under Video, and Libraries holds the capabilities.
The Shelves and the folder settings are the daemon's own state that the Shelves screen reads together, which no other tag covers.
`GET /auth/me` and `DELETE /auth/session` read and end the same session under two names; one path for one resource lets a reader find both.

**Rejected alternatives**
The eleven tags of the first proposal were rejected because they had no place for the Shelves and the folder settings.
Keeping `GET /auth/me` was rejected because the read and the delete of the session would stay under different paths.

#### Answer Every Batch Command with 202 and an Operation

**Decision**
Copy, move, delete, and timeline scans answer `202` with an Operation, even when the work finishes before the response; the body then carries the terminal state.
This replaces the decision "Decide Sync and Async by the Shape of the Request" of [design.md §11.1](../design.md#decide-sync-and-async-by-the-shape-of-the-request).

**Reason**
The current contract chooses `200`, `202`, or `204` for the same command by the number of sources and the Libraries, so a client needs a branch per status and has no single place for skipped and failed entries.
A terminal body keeps the one round trip that the earlier decision protected for a rename within a Library.

**Rejected alternatives**
Deciding by the shape of the request, the earlier decision, was rejected for the branches it leaves in every client.
A synchronous `204` for a delete was rejected because it cannot report a partial result.

#### Make a Timeline Scan an Operation

**Decision**
A scan that a client asks for is an Operation of kind `timelineScan`, started by `POST /timeline/scans`, while `GET /timeline/status` stays as a projection (§9.1).

**Reason**
The refresh and the status were a fourth lifecycle with its own polling, unlike every other long-running request.

**Rejected alternatives**
Removing the status projection was rejected because a client that shows aggregation started by the daemon itself has no Operation to read.

#### Use Problem Details for Every Error

**Decision**
Every error is `application/problem+json` with `code`, `detail`, and `requestId` (§11.1).

**Reason**
The current contract documents `ErrorResponse`, but the generated router answers a rejected body in plain text, a missing route and a disallowed method with an empty body, and a 401 only gets a body from an outer middleware, while `requestId` is never set.
A generated client cannot decode every error with one type.

**Rejected alternatives**
Keeping `ErrorResponse` and wrapping the failing paths was rejected because the routing and extraction failures would bypass it unless each is wrapped by hand; a media type of its own makes the boundary visible.

#### End the Oldest Video Session and Answer It with 410

**Decision**
The daemon keeps ending the oldest transcode session to make way for a new one, and answers requests for the ended session with `410` and `SessionEvicted` (§9.3).

**Reason**
The limit defaults to one session, and opening a new video means the previous one was finished or abandoned ([Video Playback Design §5.4](./video.md#54-concurrency)); a session whose `DELETE` was lost would otherwise block the next video until it expired.
The defect was only that the owner of the ended session could not tell what happened.

**Rejected alternatives**
Refusing a new session with `429` and `Retry-After` was rejected because a lost `DELETE` would block playback.
Keeping the silent `404` was rejected because it is indistinguishable from an unknown session.

#### Keep Images on the Image Socket Only

**Decision**
The API has no HTTP path for thumbnails, sprite sheets, or display images.

**Reason**
The image socket is their only path by the decision of [Image Socket Design](./image-socket.md#carry-thumbnails-and-display-images-over-one-websocket-per-tab), and no client needs another.

**Rejected alternatives**
A cacheable `GET .../entries/thumbnail` for clients that cannot keep a WebSocket was rejected because no such client exists and it would reverse that decision without a new reason.

#### Keep the Identity of Ratings and Tags Inside the Daemon

**Decision**
The API addresses a rating and the tags by the Entry's path and does not expose the content hash that identifies them.

**Reason**
The daemon resolves the path to the content hash before reading or writing ([Rating and Tag Design §6.1](./rating-tags.md#61-what-is-stored)), so a client never needs the hash, and the change of identity in M26 of [design.md §12.2](../design.md#122-roadmap) does not change the contract.

**Rejected alternatives**
A `contentId` on the Entry was rejected because no client uses it, and it would make the daemon's storage key part of the contract.

### 14.2 Deferred

#### Adopting the HTTP QUERY Method

**Current state**
The three body-carrying reads are `POST` (§5.1).

**Why not decide now**
The repository's OpenAPI 3.0.3 pipeline and its generator have no `QUERY` operation, and axum's method filter has no `QUERY`, so routing it needs hand-written work; no client needs automatic retry or caching of these reads.

**Condition for deciding**
When the OpenAPI pipeline and axum support `QUERY`, and a client needs safe-method semantics for these reads.

#### Commands with Sources in Several Libraries

**Current state**
The sources of a copy or move and the targets of a delete are in one Library (§7.1, §7.2).

**Why not decide now**
The current commands have one source Library, the Operation reports one, and no screen selects Entries across Libraries.

**Condition for deciding**
When a client offers a selection across Libraries, such as from a Shelf with starts in several Libraries.

#### Idempotency Keys for Commands

**Current state**
A command retried after a lost response may run twice, and its result can be found only through `GET /operations`.

**Why not decide now**
A key is worth keeping only as long as the Operation it names, and Operations stay in memory ([design.md §11.2](../design.md#persisting-operations)).

**Condition for deciding**
When Operations are persisted.

#### Managing Sessions and the Password Through the API

**Current state**
The API reads and revokes only the current session, and has no password change.

**Why not decide now**
The current contract has neither, and this structure adds no capability of its own (§2).

**Condition for deciding**
When the owner needs to revoke a session of another device, or to change the password without the daemon's command line.

#### Resuming an Upload Across Restarts, and Retrying a Commit with Another Policy

**Current state**
Upload sessions are in memory, so there is nothing to list after a restart, and a commit that meets a conflict ends with `409`.

**Why not decide now**
Listing sessions to resume needs them persisted, which is the question of [design.md §11.2](../design.md#persisting-operations), and no client has asked for a commit-time policy.

**Condition for deciding**
When an upload interrupted by a daemon restart has to be resumed, or a client needs to resolve a commit-time conflict without uploading again.

#### Cancelling a Timeline Scan

**Current state**
`POST /operations/{operationId}/cancel` on a `timelineScan` answers its current state and stops nothing.

**Why not decide now**
A scan runs to its end once started and takes no cancellation, and the next scan skips the files it already read ([Timeline Design §6.1](./timeline.md#61-scanning-and-skipping-rereads)).

**Condition for deciding**
When the control of background work ([design.md §11.2](../design.md#controlling-background-work)) is decided.

## 15. Current State and Remaining Work

### 15.1 Current Code

`daemon/openapi.yaml` answers every error with the Problem of §11.1 (A1), serves the single-Entry operations of §4 with `view`, ratings, tags, and folder settings beside them (A2), and lists only through `POST /entries` into the Entry page, with its summary and the metadata batch (A3); the rest of §3 through §13 is not implemented, and the contract it serves is otherwise the one this document replaces.
Until each area is replaced, the following statements in other documents describe the current contract, and this document the replacement:

- The namespaces of [design.md §8.1](../design.md#81-namespaces) and the sync and async table of [§8.2](../design.md#82-determining-sync-and-async), with the decision "Decide Sync and Async by the Shape of the Request"
- The silent end of a transcode session in [Video Playback Design §5.4](./video.md#54-concurrency), and the error kinds of [design.md §4.4](../design.md#44-error-model-and-http-mapping), which lack `SessionEvicted`
- The paths of the deletions in [design.md §8.4](../design.md#84-deletion) and the decision "Delete Moves the Entry to a Library-Local Trash, Unified on POST", and the example path in the introduction of [§8](../design.md#8-conventions-for-mutating-apis)
- The refresh in [Timeline Design §6.3](./timeline.md#63-when-aggregation-starts) and [§10.1](./timeline.md#101-current-code)
- The external representation of Aggregation in the term list of [terms.md](../terms.md#2-term-list), and the names of the API in its Timeline entry

Each area's change rewrites the statements it replaces.

### 15.2 Roadmap

The work is M34 of [design.md §12.2](../design.md#122-roadmap).
The order is provisional; only the dependencies must hold.

| No. | Content | Dependencies assumed |
| --- | --- | --- |
| A1 | Problem Details for every error, including the generated router's rejections | None |
| A2 | One Entry namespace with `view`, the single-Entry operations, ratings, tags, and folder settings beside them, and `GET /tags/words` | None |
| A3 | `POST /entries` with the Entry page and `fields=summary`, the months and the metadata batch, and the removal of the folder, summary, search, and timeline listings | A2 |
| A4 | `POST /entries/copy`, `move`, and `delete`, `202` with an Operation for every batch command, and timeline scans as Operations | A2 |
| A5 | `410 SessionEvicted`, `GET /auth/session`, and the twelve tags with the tags left empty removed | A2, A3, A4 |

Defects are not recorded here.
See [issues.md](../issues.md).
