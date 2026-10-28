# Switch the Web Client to the Image Socket

## 1. About This Plan

Each part of the web client fetches its own thumbnails, sprite sheets, and display images with one `GET` each and keeps them in the browser's HTTP cache. This work gathers every image want of a tab in one image socket client that talks to the image socket and keeps received images in IndexedDB.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The steps, completion conditions, and scope of this work |
| [Client Design §3.4](../design/client.md#34-viewer), [§4](../design/client.md#4-thumbnail-requests), [§5.1](../design/client.md#51-decided), [§6](../design/client.md#6-current-state-and-remaining-work) | What each part wants, the merge, the image cache, the decisions, and the roadmap row C13 |
| [Image Socket Design](../design/image-socket.md) | The connection, request classes, delivery, revalidation, and contract the client follows |
| [design.md §10.1](../design.md#101-web) | The development server's forwarding |
| [measurements.md](../measurements.md) | The record of the duplicate decodes measured after the switch |

[design.md](../design.md#11-division-of-responsibility-between-documents) holds the map of the document family.

### 1.2 Creation Date and Target Commit

Created 2026-10-26 / Target commit `d4030c6` with the uncommitted work of the same day

## 2. Purpose and Completion Conditions

While browsing a folder and its viewer, the web client sends no HTTP request for thumbnails, sprite sheets, or display images, the daemon never decodes one image twice at the same time, and a reload shows the images already seen without transferring them again.

### 2.1 Completion Conditions

1. The web client's types of the socket are generated from `daemon/sockets/image.schema.json` by `npm run gen:image-socket`, and CI fails when the generated file has a diff
2. While browsing a folder in the grid and the list, the timeline, the info panel, flipbook playback, and the viewer with its filmstrip, the browser sends no request to `/api/v1/libraries/{libraryId}/thumbnails`, and all images appear
3. An image that two parts want at the same width, height, shape, and kind is one item on the socket
4. The viewer's shown image is wanted as `interactive` and its neighbors as `prefetch`; the visible tiles are `visible` and ordered by distance from the center, and the tiles in the prefetch margin are `prefetch`
5. After a reload, reopening a folder already seen shows its tiles from IndexedDB, every want carries an `ETag`, and the daemon answers `unchanged` and sends no image frame for files that did not change
6. The cache drops the least recently used thumbnails and sprite sheets beyond 256 MiB and display images beyond 512 MiB, each kind on its own
7. Logging out deletes the cache
8. After the socket closes, the client reconnects after 1 second, doubling up to 30 seconds, and wants every unanswered item again; after 4401 it shows login; after 4400 the unanswered tiles become icons
9. `thumbnail_image`, `display_image`, and `viewer_display` client timings are still reported while performance logging is on, measured from the want to the complete image
10. `npm run dev` serves the socket through Vite's forwarding to the daemon
11. In a log of browsing a folder of iPhone HEIC with the viewer open, no two external decodes of one source overlap, and the measurement record states the counts
12. `cd ui-web && npm run lint && npm run typecheck && npm test` and `cd daemon && RUSTC_WRAPPER= cargo make test` pass

### 2.2 Non-Goals

- Removing `GET /libraries/{libraryId}/thumbnails` from the daemon. It is IC3, planned in [thumbnail-get-removal](./thumbnail-get-removal.md).
- Sharing one socket between tabs. It is deferred ([Image Socket Design §9.2](../design/image-socket.md#sharing-one-socket-between-tabs)).
- Wanting folder and archive tiles again after the daemon's own changes. It is C11, which follows this work.

## 3. Current State

Each listing part makes its own `ThumbnailQueue` through `useThumbnailQueue` ([useThumbnailQueue.ts#L11](../../ui-web/src/thumbnails/useThumbnailQueue.ts#L11)): the grid ([Grid.tsx#L102](../../ui-web/src/components/Grid.tsx#L102)), the list ([List.tsx#L86](../../ui-web/src/components/List.tsx#L86)), the timeline ([TimelineGrid.tsx#L166](../../ui-web/src/components/TimelineGrid.tsx#L166)), the info panel ([InfoPanel.tsx#L60](../../ui-web/src/components/InfoPanel.tsx#L60)), and the viewer's filmstrip ([Viewer.tsx#L238](../../ui-web/src/components/Viewer.tsx#L238)).
A queue sends at most 4 requests and cancels with `AbortController` ([queue.ts#L26](../../ui-web/src/thumbnails/queue.ts#L26)), and each request is one `GET` by `fetchThumbnail` ([api.ts#L24](../../ui-web/src/thumbnails/api.ts#L24)); flipbook playback fetches the sprite sheet the same way ([useFlipbook.ts#L30](../../ui-web/src/thumbnails/useFlipbook.ts#L30)).
The viewer's display images are fetched by `loadDisplayImage` with `Pxna-Priority` ([displayImages.ts#L5](../../ui-web/src/viewer/displayImages.ts#L5), [Viewer.tsx#L259](../../ui-web/src/components/Viewer.tsx#L259)) from URLs built by `displayImageUrl` ([urls.ts#L12](../../ui-web/src/viewer/urls.ts#L12)), and their timings are taken from resource timing by URL ([clientTimings.ts#L55](../../ui-web/src/api/clientTimings.ts#L55), [Viewer.tsx#L983](../../ui-web/src/components/Viewer.tsx#L983)).
Logging out is `logout` ([session.ts#L30](../../ui-web/src/auth/session.ts#L30)), and Vite forwards `/api` without WebSocket forwarding ([vite.config.ts#L22](../../ui-web/vite.config.ts#L22)).
The generated API types come only from `openapi.yaml` ([package.json#L14](../../ui-web/package.json#L14)), and CI checks only `src/api` for a diff ([test-web.yml#L66](../../.github/workflows/test-web.yml#L66)).
As long as the parts keep their own queues, the daemon cannot see that the listing and the filmstrip want the same image, so the duplicate decodes remain.

## 4. Change Policy

An `ImageSocketClient` made once per tab owns the socket, the merged items, and the IndexedDB cache. Parts subscribe to the images they need with a hook that gives a blob URL and a state, and pass their class and order. The client looks each image up in the cache first, shows a hit at once, and wants the item with the hit's `ETag`. The viewer's display images use the same client, so `DisplayImages` keeps only which images to keep loaded. The per-part queues, `fetchThumbnail`, and `loadDisplayImage` are removed.

### 4.1 Decisions Assumed

| Decision | Source |
| --- | --- |
| One image socket client per tab that merges the wants of every part | [Client Design §5.1](../design/client.md#gather-every-image-want-of-a-tab-in-one-image-socket-client) |
| The IndexedDB cache, its budgets, revalidation, and deletion on logout | [Client Design §5.1](../design/client.md#keep-received-images-in-indexeddb-and-revalidate-them) |
| The classes of the viewer's images and of tiles, and the order by distance from the center | [Client Design §3.4](../design/client.md#34-viewer), [§4](../design/client.md#4-thumbnail-requests) |
| Reconnection, close codes, and the end of items | [Image Socket Design §3](../design/image-socket.md#3-connection), [§4](../design/image-socket.md#4-wanted-set) |
| The generated types from the JSON Schema | [Image Socket Design §8](../design/image-socket.md#8-contract) |
| `image-socket-server` is done | [image-socket-server](./image-socket-server.md) |

### 4.2 Scope of Impact

Every part of the web client that shows a thumbnail, a sprite sheet, or a display image changes, together with client timings, logout, the Vite configuration, and CI.
A JSON Schema to TypeScript generator and an IndexedDB fake for tests are added as development dependencies.
The browser's HTTP cache of thumbnails becomes unused, so the first load after the switch transfers every image once again.

## 5. Steps

| No. | Content | Dependencies | State |
| --- | --- | --- | --- |
| [1](#s-1) | Generate the socket's types and check them in CI | - | Done |
| [2](#s-2) | Add the IndexedDB image cache with budgets per kind | - | Done |
| [3](#s-3) | Add the image socket client | 1, 2 | Done |
| [4](#s-4) | Move the grid, list, timeline, and info panel to the client | 3 | Done |
| [5](#s-5) | Move the filmstrip, flipbook playback, and display images to the client | 3 | Done |
| [6](#s-6) | Clear the cache on logout, keep client timings, and forward the socket in Vite | 3 | Done |
| [7](#s-7) | Remove the per-part queues and add the tests | 4, 5, 6 | Done |
| [8](#s-8) | Measure the duplicate decodes and update the documents | 7 | Done |

<a id="s-1"></a>
### Step 1. Generate the Socket's Types and Check Them in CI

**Locations to change**
[package.json#L14](../../ui-web/package.json#L14), `ui-web/src/api/imageSocket.d.ts` (new, generated), [test-web.yml#L66](../../.github/workflows/test-web.yml#L66)

**What to do**
Add `gen:image-socket`, which generates the TypeScript types from `../daemon/sockets/image.schema.json`, and run it in CI beside `gen:api`, failing on a diff under `src/api`.

**Completion condition**
Completion condition 1 holds.

<a id="s-2"></a>
### Step 2. Add the IndexedDB Image Cache with Budgets per Kind

**Locations to change**
`ui-web/src/images/cache.ts` (new, the image cache)

**What to do**
Store each image's blob, `ETag`, content type, frame count, size, kind, and last use, keyed by Library, path, kind, width, height, and shape. Keep the total size per budget, thumbnails and sprite sheets together and display images apart, and drop the least recently used beyond 256 MiB and 512 MiB. Provide a call that deletes the whole cache.

**Completion condition**
Unit tests with an IndexedDB fake show lookup, update on a new `ETag`, eviction per kind, and deletion.

<a id="s-3"></a>
### Step 3. Add the Image Socket Client

**Locations to change**
`ui-web/src/sockets/image.ts` (new, the image socket client), `ui-web/src/images/useImage.ts` (new, the hook for parts)

**What to do**
Open the socket once per tab, merge the subscriptions of parts into items with the highest class and the smallest order, give each item a new id, and send `want` and `drop` for the changes. Look the cache up before wanting, show a hit at once, and want it with its `ETag`. Reassemble chunks by id and offset, store complete images, and keep an image on `unchanged`. Reconnect with the backoff of Image Socket Design §3 and want every unanswered item again, show login on 4401, and fail the unanswered items on 4400. Release a blob URL when no part subscribes to its image.

**Completion condition**
Unit tests with a fake socket show the merge, the class and order of a merged item, the cache hit with its `ETag`, the reassembly, the reconnection, and the two close codes.

<a id="s-4"></a>
### Step 4. Move the Grid, List, Timeline, and Info Panel to the Client

**Locations to change**
[Grid.tsx#L102](../../ui-web/src/components/Grid.tsx#L102), [List.tsx#L86](../../ui-web/src/components/List.tsx#L86), [TimelineGrid.tsx#L166](../../ui-web/src/components/TimelineGrid.tsx#L166), [InfoPanel.tsx#L60](../../ui-web/src/components/InfoPanel.tsx#L60), [Tile.tsx#L28](../../ui-web/src/components/Tile.tsx#L28)

**What to do**
Subscribe each visible tile as `visible` and each tile in the prefetch margin as `prefetch`, with the distance from the center as the order, keeping the 200 ms delay of folder tiles and the sizes of Client Design §4. The info panel's larger thumbnail is `visible`. A `Timeout` failure makes the tile an icon for the rest of the folder's display.

**Completion condition**
The existing tests of these parts pass with the fake socket, and a tile shows its image from the socket.

<a id="s-5"></a>
### Step 5. Move the Filmstrip, Flipbook Playback, and Display Images to the Client

**Locations to change**
[Viewer.tsx#L238](../../ui-web/src/components/Viewer.tsx#L238), [Viewer.tsx#L259](../../ui-web/src/components/Viewer.tsx#L259), [displayImages.ts#L5](../../ui-web/src/viewer/displayImages.ts#L5), [urls.ts#L12](../../ui-web/src/viewer/urls.ts#L12), [useFlipbook.ts#L30](../../ui-web/src/thumbnails/useFlipbook.ts#L30)

**What to do**
Subscribe the filmstrip's thumbnails like the listing's. Subscribe the sprite sheet when flipbook playback starts, `visible`, with the tile's width, height, and shape. Subscribe the shown display image as `interactive` and its neighbors as `prefetch`, raise a prefetched image to `interactive` when it is shown, and keep up to 6 loaded display images besides them.

**Completion condition**
The existing tests of the viewer, the filmstrip, and the flipbook pass with the fake socket.

<a id="s-6"></a>
### Step 6. Clear the Cache on Logout, Keep Client Timings, and Forward the Socket in Vite

**Locations to change**
[session.ts#L30](../../ui-web/src/auth/session.ts#L30), [clientTimings.ts#L55](../../ui-web/src/api/clientTimings.ts#L55), [Viewer.tsx#L983](../../ui-web/src/components/Viewer.tsx#L983), [vite.config.ts#L22](../../ui-web/vite.config.ts#L22)

**What to do**
Delete the image cache in `logout`. Record `thumbnail_image`, `display_image`, and `viewer_display` from the time an image is wanted to the time it is complete or shown, instead of resource timing by URL. Enable WebSocket forwarding for `/api` in Vite.

**Completion condition**
Completion conditions 7, 9, and 10 hold.

<a id="s-7"></a>
### Step 7. Remove the Per-Part Queues and Add the Tests

**Locations to change**
[queue.ts#L26](../../ui-web/src/thumbnails/queue.ts#L26), [useThumbnailQueue.ts#L11](../../ui-web/src/thumbnails/useThumbnailQueue.ts#L11), [api.ts#L24](../../ui-web/src/thumbnails/api.ts#L24), [displayImages.ts#L5](../../ui-web/src/viewer/displayImages.ts#L5), and their tests

**What to do**
Remove `ThumbnailQueue`, `useThumbnailQueue`, `fetchThumbnail`, and `loadDisplayImage`, and move the behaviors their tests protect, the order by distance, the drop on leaving the margin, and the folder delay, to tests of the client and the hook. Add a browser check with Playwright that browses a folder, opens the viewer, and finds no request to the thumbnail path.

**Completion condition**
Completion conditions 2 to 6 and 8 hold, and `rg -n 'fetchThumbnail|ThumbnailQueue|loadDisplayImage' ui-web/src` finds nothing.

<a id="s-8"></a>
### Step 8. Measure the Duplicate Decodes and Update the Documents

**Locations to change**
[measurements.md](../measurements.md), `docs/measurements/<date>-image-socket.md` (new, the record), [client.md](../design/client.md), [image-socket.md](../design/image-socket.md), [plans.md](../plans.md)

**What to do**
Run the image with this change, browse a folder of iPhone HEIC in the grid, open the viewer on several images, reload, and browse it again. From `pxna.log`, count the external decodes, those that overlap another decode of the same source, and the items answered `unchanged` after the reload; record the conditions, counts, and limits, and add a row to `measurements.md`. Rewrite Client Design §6.1 for the client, remove C13 from §6.2 and from the dependencies of C11, mark IC2 done in Image Socket Design §10.2, and remove this plan's row from `plans.md` and its slug from the predecessor column of `thumbnail-get-removal`.

**Completion condition**
The record states 0 overlapping decodes of one source and the number of `unchanged` answers after the reload, and `rg -n 'image-socket-client' --glob '!docs/plans/**' docs` finds nothing.

## 6. Acceptance Verification

| Condition | Command | Expected result |
| --- | --- | --- |
| 1 | `cd ui-web && npm run gen:image-socket && git status --porcelain -- src/api` | No change |
| 2 | The Playwright check of step 7 against `npm run dev` with the daemon | No request to the thumbnail path, and every tile, filmstrip item, flipbook, and display image appears |
| 3, 4, 5, 6, 8 | `cd ui-web && npm test` | The tests of steps 2, 3, and 7 pass |
| 7 | `cd ui-web && npm test` | The logout test shows the cache empty |
| 9 | `cd ui-web && npm test` | The client timing tests pass with the socket |
| 10 | `cd ui-web && npm run dev`, then open the viewer | Images appear through the forwarded socket |
| 11 | Read the record of step 8 in `docs/measurements/` | 0 overlapping decodes of one source |
| 12 | `cd ui-web && npm run lint && npm run typecheck && npm test` and `cd daemon && RUSTC_WRAPPER= cargo make test` | All pass |

The new tests protect the following: the cache's eviction per kind and deletion, the merge of one image wanted by two parts, the class and order of each part's wants, revalidation of a cache hit, reassembly of chunks, reconnection, the close codes, and the absence of thumbnail requests.
