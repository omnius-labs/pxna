# Representative Thumbnails Are Not Reflected on Screen After Their Contents Are Deleted

**Severity: medium** (visible today; fixing it needs a design decision on the approach)

Investigated 2026-10-16 / Target commit `5668d4a`

One item of [issues.md](../issues.md).

## Symptom

While looking at folder and archive tiles in folder view, if you delete the images and videos inside them or move them elsewhere, the tile's representative thumbnail stays as the image from before the deletion.
It remains for as long as the row stays in the visible range.
If you move the row off screen and bring it back, the result of requesting again is reflected.

## Locations

- [queue.ts#L127](../../ui-web/src/thumbnails/queue.ts#L127) — where `#next` excludes keys that have a state (both ready and failed) from the requests
- [queue.ts#L84](../../ui-web/src/thumbnails/queue.ts#L84) — a ready state is discarded only when the row leaves the request targets. A failed state is not discarded
- [queue.ts#L1](../../ui-web/src/thumbnails/queue.ts#L1) — a comment that says only of failed that it "is not requested again while the queue is alive"
- [useThumbnailQueue.ts#L29](../../ui-web/src/thumbnails/useThumbnailQueue.ts#L29) — the dependencies for rebuilding the queue. They do not include reloading the listing
- A request for the representative after a change returns the re-selected representative if images and videos remain, and only when they run out does [selection.rs#L137](../../daemon/modules/engine/src/core/thumbnail/selection.rs#L137) return `NotFound` with no candidate. The server side follows changes at [selection.rs#L86](../../daemon/modules/engine/src/core/thumbnail/selection.rs#L86) (stops reusing an update within 2 seconds), [selection.rs#L18](../../daemon/modules/engine/src/core/thumbnail/selection.rs#L18) (a 30-second expiry), and [selection.rs#L127](../../daemon/modules/engine/src/core/thumbnail/selection.rs#L127) (forbids publishing a selection whose generation advanced because of the daemon's own change)

## Cause

As long as a row remains a request target, `ThumbnailQueue` keeps the ready image without requesting again, and it does not request a failed one again while the queue is alive.
Reloading the listing does not rebuild the queue, so the old image is reused even on the first redraw after a change.
For a request to a folder whose images and videos have run out, the representative selection returns `NotFound`, and when other candidates remain it returns a re-selected representative (selection.rs#L137).
In every case, the cause of the stale display is that the screen does not make a request.
There is no description in the code of how the request suppression came about, and it reads as an extension of the design that suppresses requests for off-screen tiles ([queue.ts#L23](../../ui-web/src/thumbnails/queue.ts#L23)) (this is a reading of intent, not a stated reason).

## Impact

There is no data loss.
A period occurs in which the server's response (`NotFound` for a request after the candidates have run out) and the image the screen holds disagree, only while the row stays in the visible range.
Representative thumbnails are used only for folder and archive tiles in folder view, so the timeline is not affected.

## Handling Policy

For now, this item is not addressed (status quo).
The tile shows the stale image after a content change only until the row leaves the visible range, and there is no data loss.
The question of choosing the approach to take (such as discarding and re-requesting after a change) belongs to the deferred item "Whether to Reflect a Folder's Representative Thumbnail on Screen Immediately" in [Thumbnail Design §10.2](../design/thumbnail.md#whether-to-reflect-a-folders-representative-thumbnail-on-screen-immediately), and this section will be updated when that is decided.
