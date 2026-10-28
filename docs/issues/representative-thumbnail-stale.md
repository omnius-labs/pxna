# Representative Thumbnails Are Not Reflected on Screen After Their Contents Are Deleted

**Severity: medium** (visible today)

Investigated 2026-10-23 / Target commit `47b81b0`

One item of [issues.md](../issues.md).

## Symptom

While looking at folder and archive tiles in folder view, if you delete the images and videos inside them or move them elsewhere, the tile's representative thumbnail stays as the image from before the deletion.
It remains for as long as the row stays in the visible range.
Reading the code, a request made after the row leaves the screen and returns can also receive the old image from the browser's HTTP cache for up to a day after the first response. This has not been confirmed in a browser.

## Locations

- [queue.ts#L127](../../ui-web/src/thumbnails/queue.ts#L127) — where `#next` excludes keys that have a state (both ready and failed) from the requests
- [queue.ts#L85](../../ui-web/src/thumbnails/queue.ts#L85) — a ready state is discarded only when the row leaves the request targets. A failed state is not discarded
- [queue.ts#L1](../../ui-web/src/thumbnails/queue.ts#L1) — a comment that says only of failed that it "is not requested again while the queue is alive"
- [useThumbnailQueue.ts#L29](../../ui-web/src/thumbnails/useThumbnailQueue.ts#L29) — the dependencies for rebuilding the queue. They do not include reloading the listing
- [route.rs#L113](../../daemon/entrypoints/daemon/src/interface/features/thumbnails/route.rs#L113) — every thumbnail response, including folders and archives, carries `Cache-Control: private, max-age=86400`
- [api.ts#L28](../../ui-web/src/thumbnails/api.ts#L28) — `fetchThumbnail` does not specify a cache mode, so the request goes through the browser's HTTP cache
- A request for the representative after a change returns the re-selected representative if images and videos remain, and only when they run out does [selection.rs#L140](../../daemon/modules/engine/src/core/thumbnail/selection.rs#L140) return `NotFound` with no candidate. The server side follows changes at [selection.rs#L86](../../daemon/modules/engine/src/core/thumbnail/selection.rs#L86) (stops reusing an update within 2 seconds), [selection.rs#L18](../../daemon/modules/engine/src/core/thumbnail/selection.rs#L18) (a 30-second expiry), and [selection.rs#L127](../../daemon/modules/engine/src/core/thumbnail/selection.rs#L127) (forbids publishing a selection whose generation advanced because of the daemon's own change)

## Cause

There are two layers.

As long as a row remains a request target, `ThumbnailQueue` keeps the ready image without requesting again, and it does not request a failed one again while the queue is alive.
Reloading the listing does not rebuild the queue, so the old image is reused even on the first redraw after a change.
There is no description in the code of how the request suppression came about, and it reads as an extension of the design that suppresses requests for off-screen tiles ([queue.ts#L21](../../ui-web/src/thumbnails/queue.ts#L21)) (this is a reading of intent, not a stated reason).

The URL of a representative is the folder's or archive's path, so it does not change when the chosen Entry changes.
The response is allowed to be cached for a day, the same as a file's, so a request made again can be answered by the browser cache without reaching the server.
The comment at [route.rs#L112](../../daemon/entrypoints/daemon/src/interface/features/thumbnails/route.rs#L112) gives the reason for the one-day expiry only in terms of files, whose key changes when they are replaced.

## Impact

There is no data loss.
The server's response (the re-selected representative, or `NotFound` after the candidates have run out) and the image the screen shows disagree while the row stays in the visible range, and possibly for up to a day after that through the browser cache.
Representative thumbnails are used only for folder and archive tiles in folder view, so the timeline is not affected.

## Handling Policy

Fix both layers as decided in [Thumbnail Design §10.1](../design/thumbnail.md#representative-revalidation).
The server-side layer goes away with the image socket: every want of a folder or archive is revalidated against the representative chosen now, so no HTTP cache answers it ([Image Socket Design §7](../design/image-socket.md#7-revalidation)).
The web client discards the states of folder and archive tiles and wants them again with the `ETag` of the image it holds when it rereads the listing after the daemon's own change (C11 of [Client Design §6.2](../design/client.md#62-roadmap), after C13).
A test that a want with the old `ETag` receives the new representative, and a test that the client wants folder tiles again after its states are discarded, prevent a recurrence.
