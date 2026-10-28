# Open the Image Socket on the Daemon

## 1. About This Plan

The daemon returns thumbnails, sprite sheets, and display images only as one `GET` each. This work adds the image socket: a WebSocket on which a client wants items with a request class and an order, and the daemon passes them to source jobs in that order, sends each image in chunks, and revalidates held images by `ETag`. The web client does not use it yet.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The steps, completion conditions, and scope of this work |
| [Image Socket Design §3 to §8](../design/image-socket.md#3-connection), [§9.1](../design/image-socket.md#91-decided), [§10](../design/image-socket.md#10-current-state-and-remaining-work) | The connection, wanted sets, request classes, delivery, revalidation, contract, the decisions, and the roadmap row IC1 |
| [Thumbnail Design §7](../design/thumbnail.md#7-order-and-concurrency-of-generation) | The source jobs the socket passes items to |
| [design.md §8.5](../design.md#85-csrf), [§4.2](../design.md#42-openapi-driven-code-generation) | The `Origin` validation and the place of the socket's contract |

[design.md](../design.md#11-division-of-responsibility-between-documents) holds the map of the document family.

### 1.2 Creation Date and Target Commit

Created 2026-10-26 / Target commit `d4030c6` with the uncommitted work of the same day

## 2. Purpose and Completion Conditions

A client can want thumbnails, sprite sheets, and display images over one WebSocket and receive them in the order of request class and order, with images it already holds answered by `unchanged`.

### 2.1 Completion Conditions

1. `daemon/sockets/image.schema.json` defines every text message in both directions and the binary frame header, and a sample of every message and header the daemon serializes validates against it in a test
2. An upgrade of `GET /api/v1/sockets/image` without a session gets 401; with a session cookie and a missing or mismatched `Origin` it gets 403; with a bearer token or a cookie and a matching `Origin` it upgrades; a ninth open socket of one session gets 429
3. For an item, the chunks of its image reassemble to the same bytes and `ETag` as `GET /libraries/{libraryId}/thumbnails` returns for the same request, and a sprite sheet's header carries its frame count
4. An item wanted with the image's current `ETag` gets `unchanged` and causes no conversion
5. With the generation slots held, a connection has at most 8 `visible` and `prefetch` items passed to generation, and an `interactive` item wanted after 10 `visible` ones is passed to generation at once and gets the first slot that frees
6. An item dropped before it is passed to generation is never generated; an item dropped while its job decodes gets no frames, and the job still stores its images
7. A display image larger than 64 KiB is sent in several frames, and a `visible` thumbnail that completes meanwhile is sent before the display image's last chunk
8. A connection is closed with 4400 on an item over the item limit, on a text message over the text limit, on a message that is not in the contract, and on a `want` that changes what an existing id asks for
9. After logout, the socket is closed with 4401 within one ping interval; on daemon shutdown it is closed with 1001; a socket whose pings get no pong twice is closed with 1011, and the client still reads the close code
10. `openapi.yaml` lists the upgrade path with a pointer to the schema, and the regenerated interface and client schema have no diff beyond that path
11. `cd daemon && RUSTC_WRAPPER= cargo make lint` and `cd daemon && RUSTC_WRAPPER= cargo make test` pass

### 2.2 Non-Goals

- The web client's use of the socket and its image cache. They are IC2, planned in [image-socket-client](./image-socket-client.md).
- Removing `GET /libraries/{libraryId}/thumbnails`. It is IC3, planned in [thumbnail-get-removal](./thumbnail-get-removal.md).

## 3. Current State

The hand-written routes are mounted in an outer router ([server.rs#L58](../../daemon/entrypoints/daemon/src/interface/server.rs#L58)), including the thumbnail route ([server.rs#L63](../../daemon/entrypoints/daemon/src/interface/server.rs#L63), [route.rs#L31](../../daemon/entrypoints/daemon/src/interface/features/thumbnails/route.rs#L31)), and the whole is wrapped by `OriginCheck` ([server.rs#L73](../../daemon/entrypoints/daemon/src/interface/server.rs#L73)), which lets every `GET` and `HEAD` through ([origin.rs#L33](../../daemon/entrypoints/daemon/src/interface/middleware/origin.rs#L33)).
Routes authenticate with `SessionAuth::authenticate` from the cookie or the bearer token ([session.rs#L75](../../daemon/entrypoints/daemon/src/interface/session.rs#L75)), and a revoked or expired session no longer authenticates ([auth.rs#L97](../../daemon/modules/engine/src/service/auth.rs#L97)).
The thumbnail route builds the `ETag` from the thumbnail's key ([route.rs#L142](../../daemon/entrypoints/daemon/src/interface/features/thumbnails/route.rs#L142)) and calls `ThumbnailGenerator::generate`, which returns the image only after generation ([generator.rs#L207](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L207)); no call gives the current key without generating.
`axum` is built without its `ws` feature ([Cargo.toml#L130](../../daemon/Cargo.toml#L130)), and the server stops on the cancellation token ([server.rs#L81](../../daemon/entrypoints/daemon/src/interface/server.rs#L81)).
Without a socket, the daemon cannot see the wants of a tab together or reorder them, so this work is needed before the client can switch.

## 4. Change Policy

A hand-written route upgrades the connection after authenticating it and checking `Origin` and the socket count. Each connection runs one task that owns its wanted set: it reads messages, revalidates items with an `ETag` through a new key-only call of the generator, passes up to 8 items to the generator in class and order, and feeds a sender that interleaves 64 KiB chunks by class and order under a 4 MiB cap, reading an image from the cache when it does not fit. The message types are hand-written serde types in the daemon, validated against the schema in tests.

### 4.1 Decisions Assumed

| Decision | Source |
| --- | --- |
| One WebSocket per tab, no HTTP fallback, and the server holding the wanted set | [Image Socket Design §9.1](../design/image-socket.md#91-decided) |
| Authentication, `Origin` check, close codes, reconnection, pings, and limits | [Image Socket Design §3](../design/image-socket.md#3-connection) |
| Items, ids, and their end | [Image Socket Design §4](../design/image-socket.md#4-wanted-set) |
| Classes, order, and at most 8 items in generation per connection | [Image Socket Design §5](../design/image-socket.md#5-request-classes-and-order) |
| Chunks of 64 KiB ordered by class and order, and 4 MiB of unsent chunks | [Image Socket Design §6](../design/image-socket.md#6-delivery) |
| `unchanged` for a matching `ETag` | [Image Socket Design §7](../design/image-socket.md#7-revalidation) |
| The JSON Schema contract and the frame layout | [Image Socket Design §8](../design/image-socket.md#8-contract) |
| `source-jobs` is done | [source-jobs](./source-jobs.md) |

### 4.2 Scope of Impact

The daemon gains the socket route, its connection task, and its message types, and the engine's thumbnail generator gains a call that returns the current key of a request.
`openapi.yaml` gains the upgrade path, and the generated interface and the web client's generated schema change with it.
`axum` gains its `ws` feature, and a JSON Schema validator and a WebSocket client are added as development dependencies of the daemon.
`GET /libraries/{libraryId}/thumbnails` does not change, so compatibility does not break.

## 5. Steps

| No. | Content | Dependencies | State |
| --- | --- | --- | --- |
| [1](#s-1) | Write the socket's JSON Schema and the daemon's message types | - | Done |
| [2](#s-2) | Add the upgrade route with authentication, `Origin`, and the socket count | 1 | Done |
| [3](#s-3) | Add the key-only call to the thumbnail generator | - | Done |
| [4](#s-4) | Hold the wanted set and pass items to generation by class and order | 2, 3 | Done |
| [5](#s-5) | Send images in interleaved chunks under the cap | 4 | Done |
| [6](#s-6) | Add pings, the session check, limits, and shutdown | 4 | Done |
| [7](#s-7) | List the upgrade path in `openapi.yaml` | 1 | Done |
| [8](#s-8) | Add the socket tests | 5, 6 | Done |
| [9](#s-9) | Update the documents | 7, 8 | Done |

<a id="s-1"></a>
### Step 1. Write the Socket's JSON Schema and the Daemon's Message Types

**Locations to change**
`daemon/sockets/image.schema.json` (new, the contract), `daemon/entrypoints/daemon/src/interface/features/sockets/image/message.rs` (new, the message types), `daemon/entrypoints/daemon/src/service/sockets/image.rs` (new, the limits and the count of open sockets), [features.rs#L12](../../daemon/entrypoints/daemon/src/interface/features.rs#L12)

**What to do**
Define `want` with its items, `drop`, `unchanged`, `failed`, and the binary frame header with the fields of Image Socket Design §4, §6, and §7, with the ranges of `class`, `variant`, `resize`, width and height, and the limits of §3. Write the matching serde types, and the reading and writing of the frame layout of §8.

Turn the mentions of `daemon/sockets/image.schema.json` in the responsibility tables of design.md, Image Socket Design, Thumbnail Design, and Client Design, and in Image Socket Design §8, into links to the file.

**Completion condition**
Completion condition 1 holds for every message and the header, and the link check finds the schema linked from those documents.

<a id="s-2"></a>
### Step 2. Add the Upgrade Route with Authentication, `Origin`, and the Socket Count

**Locations to change**
[server.rs#L58](../../daemon/entrypoints/daemon/src/interface/server.rs#L58), [origin.rs#L33](../../daemon/entrypoints/daemon/src/interface/middleware/origin.rs#L33), [session.rs#L75](../../daemon/entrypoints/daemon/src/interface/session.rs#L75), [Cargo.toml#L130](../../daemon/Cargo.toml#L130), [Cargo.toml#L68](../../daemon/entrypoints/daemon/Cargo.toml#L68)

**What to do**
Enable the `ws` feature of `axum`, add a WebSocket client as a development dependency for the tests, and mount the route at `/api/v1/sockets/image`. Authenticate the upgrade with `SessionAuth::authenticate`, require a matching `Origin` when it carries a session cookie, using the same comparison as `OriginCheck`, and count open sockets per session to answer the ninth with 429.

**Completion condition**
Completion condition 2 holds in tests through the HTTP router.

<a id="s-3"></a>
### Step 3. Add the Key-Only Call to the Thumbnail Generator

**Locations to change**
[generator.rs#L207](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L207), [service/thumbnail.rs#L5](../../daemon/modules/engine/src/service/thumbnail.rs#L5)

**What to do**
Add a call that resolves a request to its target, including the representative of a folder or an archive, and returns the key that `generate` would use, without reading the cache or generating. Return no key for a local file without a modification time.

**Completion condition**
An engine test shows the call returns the same key as `generate` for a file, a folder, and an archive entry, a new key after the file is replaced, and no conversion.

<a id="s-4"></a>
### Step 4. Hold the Wanted Set and Pass Items to Generation by Class and Order

**Locations to change**
`daemon/entrypoints/daemon/src/interface/features/sockets/image/connection.rs` (new, the connection task)

**What to do**
Run one task per connection that applies `want` and `drop` to its wanted set, answers items with a matching `ETag` by `unchanged` before they count, and passes items to `ThumbnailGenerator` in class, order, and arrival: up to 8 `visible` and `prefetch` items, and up to 8 `interactive` items in a budget of their own, with `interactive` as Interactive work and the others as Foreground. Dropping an item that is passed cuts its request, which leaves the job's own rules to decide whether it starts. Answer errors with `failed` and the API's error kind.

**Completion condition**
Completion conditions 4, 5, and 6 hold in the tests of step 8.

<a id="s-5"></a>
### Step 5. Send Images in Interleaved Chunks Under the Cap

**Locations to change**
`daemon/entrypoints/daemon/src/service/sockets/image.rs`

**What to do**
Split each finished image into chunks of up to 64 KiB with the header of step 1, and send the next chunk of the highest item by class and order. Hand the socket one message at a time, so that the next chunk is chosen when it takes the last, and pass no more items to generation while 4 MiB of finished images wait to be sent.

**Completion condition**
Completion conditions 3 and 7 hold in the tests of step 8.

<a id="s-6"></a>
### Step 6. Add Pings, the Session Check, Limits, and Shutdown

**Locations to change**
`daemon/entrypoints/daemon/src/interface/features/sockets/image/connection.rs`, [server.rs#L81](../../daemon/entrypoints/daemon/src/interface/server.rs#L81)

**What to do**
Ping every 30 seconds, close with 1011 after two pings without an answer, and check the session at each ping, closing with 4401 once it no longer authenticates. Close with 4400 on the limits and contract errors of completion condition 8. Close every socket with 1001 when the server's cancellation token fires, and after sending a close frame wait up to 2 seconds for the client's close.

**Completion condition**
Completion conditions 8 and 9 hold in the tests of step 8.

<a id="s-7"></a>
### Step 7. List the Upgrade Path in `openapi.yaml`

**Locations to change**
[openapi.yaml#L1242](../../daemon/openapi.yaml#L1242), `daemon/entrypoints/interface` (generated), `ui-web/src/api/schema.d.ts` (generated)

**What to do**
Add `GET /api/v1/sockets/image` with a description that points to the schema and the 101, 401, 403, and 429 responses. Regenerate with `daemon/entrypoints/gen-interface.sh` and `cd ui-web && npm run gen:api`, and give the generated trait an uncalled implementation, as for the other hand-written routes.

**Completion condition**
Completion condition 10 holds.

<a id="s-8"></a>
### Step 8. Add the Socket Tests

**Locations to change**
`daemon/entrypoints/daemon/src/interface/features/sockets/image/tests.rs` (new, the socket through a WebSocket), `daemon/entrypoints/daemon/src/interface/features/sockets/image/connection.rs` (the tests of the connection's state), [Cargo.toml#L68](../../daemon/entrypoints/daemon/Cargo.toml#L68)

**What to do**
Add tests that open a socket with a WebSocket client against the daemon's router served on a port, and cover completion conditions 3 to 9. Hold the generation slots where the result would otherwise depend on timing, and shorten the ping interval and lower the limits through the limits of the state. Test the order of chunks, the budgets, and the hold-back on unsent bytes on the connection's state without a socket.

**Completion condition**
The tests pass, and the test of condition 5 fails if the limit of 8 is removed.

<a id="s-9"></a>
### Step 9. Update the Documents

**Locations to change**
[image-socket.md](../design/image-socket.md), [thumbnail.md](../design/thumbnail.md), [design.md](../design.md), [plans.md](../plans.md)

**What to do**
Rewrite Image Socket Design §10.1 to state that the daemon side exists and the web client does not use it, and mark IC1 done in §10.2. Update the current state of design.md §12.1 and Thumbnail Design §11.1 where they say the socket does not exist. Set the state of this plan to done in `plans.md`. The plan and its row are deleted when the plan is folded, after the review of the change.

**Completion condition**
No current-state section says the daemon has no socket, and `plans.md` shows this plan done.

## 6. Acceptance Verification

| Condition | Command | Expected result |
| --- | --- | --- |
| 1 | `cd daemon && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon sockets::image::` | The schema tests pass |
| 2, 3, 4, 5, 6, 7, 8, 9 | `cd daemon && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon sockets::image` | The tests of step 8 pass |
| 10 | `daemon/entrypoints/gen-interface.sh && (cd ui-web && npm run gen:api) && git status --porcelain -- daemon/entrypoints/interface ui-web/src/api` | No change after the commit of step 7 |
| 11 | `cd daemon && RUSTC_WRAPPER= cargo make lint && RUSTC_WRAPPER= cargo make test` | Both pass |

The tests of step 8 protect the following: the authentication and `Origin` of the upgrade, the same bytes and `ETag` as the HTTP route, `unchanged` without conversion, the order and the limit of 8, the two kinds of drop, the interleaving, the limits, and the three kinds of close.
