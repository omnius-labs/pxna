# Image Channel Design

## 1. About This Document

This document covers the image channel, the one connection through which the web client receives thumbnails, sprite sheets, and display images, and defines its connection, wanted set, priority, delivery, revalidation, contract, and the reasons for adoption.
The intended readers are developers and AI agents who implement or review this concern.
It assumes the vocabulary of [terms.md](../terms.md).
The definitions of words are not in this document.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The image channel's connection and authentication, wanted sets, request classes and ordering, delivery in chunks, revalidation, limits, the contract, reasons for adoption, deferred items |
| [design.md](../design.md) | Pxna's overall scope, overall structure, sessions, CSRF, and cross-cutting design decisions |
| [Thumbnail Design](./thumbnail.md) | How a wanted item is generated: source jobs, size steps, companions, caches, keys |
| [Client Design](./client.md) | Which images the web client wants and when, how it merges the wants of its parts, and its image cache |
| [terms.md](../terms.md) | Definitions of terms |
| [issues.md](../issues.md) | Clear defects confirmed in code and handling policies |
| [daemon/image-channel.schema.json](../../daemon/image-channel.schema.json) | The authoritative messages, fields, and value ranges of the image channel |

The entry point of the design document family is [design.md](../design.md).

### 1.2 Tense of This Document

§1 through §9 describe the completed form in the present tense.
The gap between the current code and the completed form, and the remaining work in dependency order, are gathered in §10.
If you want to know what works now, read §10 first.

## 2. Responsibilities and Boundaries

A listing asks for the thumbnails of as many Entries as it shows, the viewer's filmstrip asks for other sizes of the same images, and the viewer asks for display images, all at the same time.
The image channel carries all of these through one WebSocket per browser tab.
The client tells the server which images it wants and how urgently, and the server decides what to generate and send next, and sends each image as it is ready.

Holding every want of a tab in one place lets the server merge the wants for one source into one generation ([Thumbnail Design §7](./thumbnail.md#7-order-and-concurrency-of-generation)) and reorder or drop wants without a request being cut.
Because the channel is one connection, it also leaves the browser's six connections per origin over plain HTTP/1.1 to the listing, the content API, and video.

The boundaries with other concerns are as follows.

| Matter | Authoritative source |
| --- | --- |
| How an item's image is generated, merged with other items, and cached | [Thumbnail Design §7](./thumbnail.md#7-order-and-concurrency-of-generation), [§8](./thumbnail.md#8-cache) |
| Which images each part of the web client wants, and the client's image cache | [Client Design §4](./client.md#4-thumbnail-requests) |
| Sessions and the `Origin` validation | [design.md §7.2](../design.md#72-sessions), [§8.5](../design.md#85-csrf) |
| The original image the viewer swaps in | The content API ([Client Design §3.4](./client.md#34-viewer)) |
| Building a `NestedPath` from a Library-relative path and checking the boundary | [design.md §4.3](../design.md#43-resolving-library-and-archive-boundaries) |

### 2.1 What Is Not Built

- There is no HTTP path for thumbnails, sprite sheets, or display images. A client that cannot open a WebSocket cannot show them.
  Keeping one path keeps one set of priority, revalidation, and limit rules.
- One channel is not shared between tabs. Each tab opens its own.
  The server merges the generation of the same source across connections anyway (§5), so sharing would save only the transfer.
- The server does not send images that no item asks for, such as a notice that a file changed. A client learns of a change only when it wants the image again (§6).

## 3. Connection

The client opens the channel with a WebSocket upgrade of `GET /api/v1/image-channel` on the same origin as the web client.
The daemon authenticates the upgrade like any other API request: the session cookie, or a bearer token for clients that can set headers ([design.md §7.2](../design.md#72-sessions)).
When the upgrade carries a session cookie, the daemon requires an `Origin` whose host and port match the `Host`, and answers 403 otherwise, including when `Origin` is missing ([design.md §8.5](../design.md#85-csrf)).
The upgrade is a `GET`, so this check is made by the channel itself and not by the validation that covers only other methods.
Without it, a page on another site could open a channel with the user's cookie; `SameSite=Strict` already keeps the cookie from such a page, and the check keeps that from being the only guard.

While the channel is open, the daemon checks the session at every ping (below), and once it has ended by logout or expiry, closes the channel with close code 4401; the client returns to login.
On daemon shutdown it closes with 1001.
A message that breaks the contract or a limit closes the channel with 4400; the client does not reconnect after 4400 and shows the images as failed.

After any other close, the client reconnects after 1 second, doubling the wait up to 30 seconds with random jitter, and sends its whole wanted set again.
The server keeps no state for a closed connection, so reconnecting is the same as connecting for the first time.

Both sides send a ping every 30 seconds. The daemon closes the channel with 1011 when two of its pings in a row get no answer from the client, and the client closes it when two of its pings get no pong.
A home server's NAT and a phone that sleeps can drop a connection without a close, and without pings the client would wait for images that never come.

When the daemon closes a channel, it sends the close frame and then waits up to 2 seconds for the client's close.
A socket closed while the client's last messages are unread resets the connection, and the reset can make the client lose the close frame, and with it the code.

The daemon limits each connection to 4096 wanted items and to 256 KiB per text message, and a session to 8 open channels; a ninth upgrade is answered with 429.
These keep one client from holding unbounded state in the daemon.

## 4. Wanted Set

Each connection has one wanted set on the server.
The client adds items with `want` and removes them with `drop`; the server never adds items.

An item is one image the client wants: a Library, a Library-relative path, the kind (`thumbnail`, `sprite`, or `display`), the width and height, and the shape (`crop` or `fit`).
The client names each item with an id unique within the connection, and every message about the item carries that id.
A `want` with an id already in the set replaces its request class, order, and `ETag`; it may not change what the item asks for, and doing so is a contract error.
The output format, resampler, and anchor are the server's configuration, so items do not carry them ([Thumbnail Design §4.2](./thumbnail.md#42-shape-and-output-format)).

An item leaves the set when the server has sent its image, its `unchanged`, or its `failed`, or when the client drops it.
To receive the image again, the client wants it again with a new id.
This keeps the server from holding finished items and makes every item's end explicit.

Dropping an item that has not been passed to generation removes it with no other effect.
Dropping an item already passed to generation does not stop a source job that has started, and its result is still cached ([Thumbnail Design §7](./thumbnail.md#7-order-and-concurrency-of-generation)); if the job is still waiting for a slot and no other item needs it, it does not start. An item for a local file without a modification time has no job, so dropping it stops its generation.

## 5. Request Classes and Order

Each item has a request class and an order.
The class is `interactive` for the display image the viewer shows now, `visible` for what is on screen, and `prefetch` for what the client expects to need soon.
The order is a non-negative number that the client's part assigns within its class, smaller first, such as the distance from the center of the screen.

The server passes items to generation in the order of class, then order, then arrival.
`interactive` items generate as Interactive work, which has slots of its own, and `visible` and `prefetch` items as Foreground work ([Storage Layer Design §9.1](./storage.md#listing-work-budget)).

A connection passes at most 8 `visible` and `prefetch` items to generation at a time, and at most 8 `interactive` items, which have a budget of their own.
The rest stay in the wanted set, where a change of class or order, or a drop, takes effect at once.
If every item went to generation as soon as it arrived, the generation slots would fill in arrival order, and items that left the screen would stay ahead of the ones now on it.
The budget of the `interactive` items is apart so that the image the user waits for is not held behind the eight that wait for a slot as Foreground work.
An item with an `ETag` is checked before it counts against these (§7), at most 16 at a time, so revalidating a screen of cached images does not wait behind generation.

Across connections, items meet in the generation slots, which run each class in arrival order; with at most 8 items of a budget each, no connection keeps the slots from the others.

## 6. Delivery

The server sends each image as one or more binary frames.
Each frame holds a header and up to 64 KiB of the image; the header gives the item's id, the image's `ETag`, `Content-Type`, the frame count of a sprite sheet, the offset of the chunk, and the total size.
The image is complete when the chunks cover the total size.
`unchanged` and `failed` are text messages with the item's id; `failed` carries the error kind of the API ([design.md §4.4](../design.md#44-error-model-and-http-mapping)), such as `NotFound` or `Timeout`.

The server picks the next chunk to send by the class and order of the items, so a thumbnail on screen goes out between the chunks of a prefetched display image.
Without chunks, a display image of several hundred KB would hold the connection while the thumbnails the screen waits for queue behind it.

The server hands the socket one message at a time and chooses the next chunk when the socket takes the last, so a slow client does not fix the order in advance.
While 4 MiB of finished images wait to be sent, the connection passes no more items to generation. A slow client then holds back only its own generations, and the memory it costs is bounded by those 4 MiB and the images that are already being made.

## 7. Revalidation

The `ETag` of an image is derived from its cache key ([Thumbnail Design §8.3](./thumbnail.md#83-keys)), so it changes when the file is replaced, when a folder's representative changes, or when the conversion settings change.
A client that already holds an image wants it with that `ETag`.
The server computes the item's current key, which needs the Entry's attributes and, for a folder or an archive, the representative selection, but no image.
If the key matches, the server sends `unchanged` without reading the cache or generating; otherwise it handles the item as if it had no `ETag`.

A local file whose modification time cannot be obtained has no key; its items are always generated and sent.

## 8. Contract

[daemon/image-channel.schema.json](../../daemon/image-channel.schema.json) is the authoritative contract: the text messages in both directions, the binary frame header, and their value ranges.
A binary frame is a 4-byte big-endian length, the header as JSON of that length, and the chunk.
The web client's types are generated from the schema, and a check confirms that the generated file has no diff, as with `openapi.yaml` ([design.md §4.2](../design.md#42-openapi-driven-code-generation)).
The daemon's message types are written by hand, and tests validate their serialized form against the schema.
`openapi.yaml` lists the upgrade path with a pointer to the schema, so that the HTTP contract still names every path.

## 9. Design Decisions

### 9.1 Decided

#### Carry Thumbnails and Display Images over One WebSocket per Tab

**Decision**
The web client receives thumbnails, sprite sheets, and display images only through the image channel, one WebSocket per tab, and there is no HTTP path for them (§2.1).

**Reason**
The listing, the filmstrip, the list, the info panel, the timeline, and the viewer each sent their own requests for the same images. In the 2026-10-26 log of browsing iPhone HEIC with the viewer, 8 of 90 external decodes ran while another decode of the same source ran, mostly the listing's 512 `fit` beside the filmstrip's 128 `crop`.
With every want of a tab on one connection, the server sees them together and merges them by source, and a request no longer costs an HTTP request: over plain HTTP/1.1 the browser opens at most six connections to the daemon, and the thumbnail queues of the listing and the filmstrip, four each, and the display images competed for them.
A WebSocket carries both directions on one connection, so the client can change what it wants while images are coming, which the other forms below cannot without new requests.

**Rejected alternative 1**
One `GET` per image from a hand-written route, with the client's queues deciding the order and cutting requests to cancel them. This was the form before the channel. The browser cached each URL for a day, but the parts of the client could not see each other's requests, and two sizes of one image decoded twice.

**Rejected alternative 2**
A `POST` that lists many images and streams them back in one response as they are ready. It merges the wants of one moment, but every change of the visible range would cut the response and send a new list, and a part that wants an image later could not join the response already running.

**Rejected alternative 3**
A batch request that only asks the server to generate, followed by one `GET` per image. It keeps the browser cache, but it does not reduce the number of requests.

**Rejected alternative 4**
Reusing the listing's loaded thumbnail in the filmstrip on the client. The server would not change, but images the listing has not loaded, and the different shape of the filmstrip, would still need their own requests.

**Rejected alternative 5**
Keeping the `GET` path as a fallback when a WebSocket cannot be opened. It would keep two sets of priority, revalidation, and limit rules for one feature.

#### Let the Server Hold the Wanted Set and Order Generation

**Decision**
The client sends which images it wants with a request class and an order, and the server holds the wanted set per connection and passes items to generation in that order, a few at a time (§4, §5).

**Reason**
Only the client knows what is on screen, so the client still decides the class and the order. Holding the set on the server lets it merge items of different parts and connections by source, and lets a change of order or a drop take effect without cutting a request.

**Rejected alternative**
Keeping the client's queue, which sends a few requests at a time and cancels them, and using the channel only as the transport. It would keep the client's limit and its reordering, but the server could not merge what it has not been sent.

#### Order Items by Request Class and an Order Within It

**Decision**
An item carries one of three classes and a number within the class (§5).

**Reason**
The class tells the server which item the user is waiting for now, so the server can give it the Interactive slots. The number lets each part of the client keep its own rule, such as the distance from the center, without agreeing on one scale with the others.

**Rejected alternative**
One number computed by the client for all items. It is more flexible, but the server could not tell which item the user is waiting for, so it could not use the Interactive slots.

#### Send Images in Chunks Ordered by Priority

**Decision**
Images are sent in chunks of up to 64 KiB, and the next chunk is chosen by class and order (§6).

**Reason**
Display images and thumbnails share the connection. Whole images in completion order would let one display image hold back the thumbnails on screen.

**Rejected alternative**
A second channel for display images. It would avoid the interleaving, but the two channels would be ordered separately, and the viewer and the listing would compete again.

#### Revalidate with the ETag in the Want

**Decision**
A client that holds an image wants it with its `ETag`, and the server answers `unchanged` when the key is the same (§7).

**Reason**
The browser's HTTP cache does not keep what a WebSocket carries, so the client keeps images itself ([Client Design §4](./client.md#4-thumbnail-requests)). Revalidating each want costs the server attributes and no image, and a replaced file or a changed representative shows at once instead of after the day that the HTTP cache held a file's thumbnail.

**Rejected alternative 1**
Using a kept image without asking for a period, as the HTTP cache did. It saves the check, but a replaced file would show the old image until the period ends.

**Rejected alternative 2**
Keeping images only in memory for the page. It is simpler, but every reload and every new tab would transfer all images again.

#### Make a JSON Schema the Contract of the Channel

**Decision**
The messages are defined in `daemon/image-channel.schema.json`, the web client's types are generated from it, and the daemon's types are tested against it (§8).

**Reason**
The HTTP API is written contract-first in `openapi.yaml` ([design.md §11.1](../design.md#run-openapi-spec-first)), and OpenAPI cannot describe WebSocket messages. A JSON Schema keeps the contract first and the client's types generated, with tools the web client already uses for JSON.

**Rejected alternative 1**
AsyncAPI. It describes the connection and both directions in one document, but it needs a new generator and tool chain.

**Rejected alternative 2**
Hand-written types on both sides with the shape in this document. No new tools are needed, but nothing would detect when the two sides drift apart.

#### Authenticate the Upgrade like Any Request and Check Its Origin

**Decision**
The upgrade is authenticated by the session cookie or a bearer token, and an upgrade with a cookie needs a matching `Origin` (§3).

**Reason**
Browsers send cookies on a WebSocket upgrade but cannot set headers on it, so the cookie is the web client's credential. The upgrade is a `GET`, which the existing `Origin` validation does not cover.

**Rejected alternative**
A token in the query string of the upgrade. It works without cookies, but the token would appear in access logs and browser history.

### 9.2 Deferred

#### Sharing One Channel Between Tabs

**Current state**
Each tab opens its own channel, and the server merges generation across them (§2.1).

**Why not decide now**
Sharing needs a `SharedWorker` or a leader tab, and saves only the transfer of images that two tabs show at the same time.

**Conditions for deciding**
When the limit of 8 channels per session is reached in normal use, or a log shows the same images sent to several tabs of one session at the same time.

## 10. Current State and Remaining Work

### 10.1 Current Code

The daemon serves the image channel at `GET /api/v1/image-channel` with everything in §3 to §8, and `daemon/image-channel.schema.json` is its contract.
The web client uses it for every thumbnail, sprite sheet, and display image ([Client Design §6.1](./client.md#61-current-code)), and there is no HTTP route for them.

### 10.2 Roadmap

The order is provisional.
Only the dependencies must be respected.

| No. | Content | Dependencies assumed | State |
| --- | --- | --- | --- |
| IC1 | Open the channel, authenticate it, hold the wanted sets, pass items to generation by class and order, and send images in chunks with revalidation (§3 to §8, [plan](../plans/image-channel-server.md)) | None | Done |
| IC2 | Switch every part of the web client to the channel and keep images in its own cache ([Client Design §6.1](./client.md#61-current-code)) | IC1 | Done |
| IC3 | Remove the thumbnail HTTP route ([plan](../plans/thumbnail-get-removal.md)) | IC2 | Done |

For confirmed defects, see [issues.md](../issues.md).
