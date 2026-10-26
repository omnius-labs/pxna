# Show the New Representative Thumbnail Right After Our Own Changes

## 1. About This Plan

This work makes folder and archive tiles show the re-selected representative thumbnail, or the icon, right after the daemon's own change to their contents.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The steps and acceptance checks of this work |
| [Thumbnail Design §5](../design/thumbnail.md#5-representative-thumbnails), [§10.1](../design/thumbnail.md#representative-revalidation) | The cache header of representative responses and the decision with its reasons |
| [Client Design §4](../design/client.md#4-thumbnail-requests) | When the web client requests tiles again |
| [issues.md](../issues.md) | The defect `representative-thumbnail-stale` that this work fixes |

[design.md](../design.md#11-division-of-responsibility-between-documents) holds the map of the document family.

### 1.2 Creation Date and Target Commit

Created 2026-10-23 / Target commit `47b81b0`

## 2. Purpose and Completion Conditions

After deleting or moving the images inside a folder through Pxna, the folder's tile in the open listing shows the new representative, or the icon, without leaving the screen.

### 2.1 Completion Conditions

1. A thumbnail response for a folder or an archive carries `Cache-Control: private, no-cache` and an `ETag`, and a response for a file still carries `Cache-Control: private, max-age=86400`
2. A request with an `If-None-Match` equal to the current `ETag` returns 304 without a body
3. After `ThumbnailQueue` is told to discard folder and archive keys, those keys in the request targets are requested again, whether they were read or failed, and image and video keys are not
4. In Chromium, deleting the only image in a visible folder through the web client turns its tile into the icon, and deleting one of two images shows the remaining one, without scrolling
5. `docs/issues/representative-thumbnail-stale.md` and its row in `docs/issues.md` are gone, and the current-state sections of Thumbnail Design §11.1 and Client Design §6.1 no longer describe the old behavior
6. `cd daemon && cargo make lint`, the daemon tests, and `cd ui-web && npm run lint && npm run typecheck && npm test` pass

### 2.2 Non-Goals

- Reflecting external changes immediately. They appear when the tile leaves the request targets and returns, as decided in Thumbnail Design §10.1.
- Avoiding the server's selection and generation on revalidation. The server still computes the key before answering 304; only the transfer is saved.

## 3. Current State

[route.rs#L113](../../daemon/entrypoints/daemon/src/interface/features/thumbnails/route.rs#L113) sets `private, max-age=86400` on every response, and the handler at [route.rs#L34](../../daemon/entrypoints/daemon/src/interface/features/thumbnails/route.rs#L34) does not read `If-None-Match`.
[request.rs#L29](../../daemon/modules/engine/src/core/thumbnail/request.rs#L29) `Thumbnail` does not tell whether it answered a folder or archive. That is known only inside [generator.rs#L204](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L204).
[queue.ts#L127](../../ui-web/src/thumbnails/queue.ts#L127) never requests a key that has a state, and [queue.ts#L85](../../ui-web/src/thumbnails/queue.ts#L85) drops a ready state only when the key leaves the targets.
The folder and archive keys are the `delayed` set that [Grid.tsx#L138](../../ui-web/src/components/Grid.tsx#L138) and [List.tsx#L119](../../ui-web/src/components/List.tsx#L119) pass to `setWanted`.
After the daemon's own change, the web client calls [refreshLibrary at listing.ts#L475](../../ui-web/src/library/listing.ts#L475), which does not reach the thumbnail queue.
So a tile that stays visible keeps its old image, and a request made again could be answered from the browser cache.

## 4. Change Policy

The engine tells the route whether a response is a representative, the route chooses the cache header and answers conditional requests, and the web client discards folder and archive states when `refreshLibrary` runs for that Library.

### 4.1 Decisions Assumed

| Decision | Source |
| --- | --- |
| Revalidate representative responses and request folder and archive tiles again after our own changes | [Thumbnail Design §10.1](../design/thumbnail.md#representative-revalidation) |
| Image and video tiles are not requested again | [Client Design §4](../design/client.md#4-thumbnail-requests) |

### 4.2 Scope of Impact

The engine's `Thumbnail`, the daemon's thumbnail route, `ThumbnailQueue`, `useThumbnailQueue`, and `refreshLibrary` change.
The HTTP contract of `getThumbnail` gains a 304 response and a different `Cache-Control` for folders and archives. Existing clients that ignore these headers keep working.
`openapi.yaml` documents the 304 response, and the generated interface is regenerated.

## 5. Steps

| No. | Content | Dependencies | State |
| --- | --- | --- | --- |
| [1](#s-1) | Confirm the stale image in a browser | - | Not started |
| [2](#s-2) | Tell the route whether a thumbnail is a representative | - | Not started |
| [3](#s-3) | Choose the cache header and answer `If-None-Match` | 2 | Not started |
| [4](#s-4) | Let the queue discard folder and archive states | - | Not started |
| [5](#s-5) | Discard them when the Library is refreshed | 4 | Not started |
| [6](#s-6) | Confirm in a browser and close the defect | 1, 3, 5 | Not started |

<a id="s-1"></a>
### Step 1. Confirm the Stale Image in a Browser

**Locations to change**
None. The result goes into the Cause section of `docs/issues/representative-thumbnail-stale.md`.

**What to do**
With the daemon at the target commit and Chromium driven by playwright-cli, open a folder whose tile has a representative, delete that image through the web client, scroll the tile away and back, and check in the network log whether the request is served from the disk cache.

**Completion condition**
The issue's Cause states whether the browser cache served the old image, and the sentence marked as not confirmed is replaced by the observation.

<a id="s-2"></a>
### Step 2. Tell the Route Whether a Thumbnail Is a Representative

**Locations to change**
[request.rs#L29](../../daemon/modules/engine/src/core/thumbnail/request.rs#L29), [generator.rs#L204](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L204)

**What to do**
Add `pub representative: bool` to `Thumbnail`, set to true on the folder and archive branch of `generate_resolved` and false elsewhere.

**Completion condition**
An engine test shows that a folder and an archive give `representative == true` and a file gives false.

<a id="s-3"></a>
### Step 3. Choose the Cache Header and Answer `If-None-Match`

**Locations to change**
[route.rs#L34](../../daemon/entrypoints/daemon/src/interface/features/thumbnails/route.rs#L34), [route.rs#L113](../../daemon/entrypoints/daemon/src/interface/features/thumbnails/route.rs#L113), the `getThumbnail` responses in [openapi.yaml#L1169](../../daemon/openapi.yaml#L1169), [thumbnails.rs#L36](../../daemon/entrypoints/daemon/src/interface/features/thumbnails.rs#L36)

**What to do**
Use `private, no-cache` for representatives and keep `private, max-age=86400` for files. When the request's `If-None-Match` contains the response's `ETag`, return 304 with the `ETag` and `Cache-Control` and no body. Document 304 in `openapi.yaml` and regenerate with `daemon/entrypoints/gen-interface.sh`.

**Completion condition**
Router tests cover the header of a folder, an archive, and a file, and a matching and a non-matching `If-None-Match`.

<a id="s-4"></a>
### Step 4. Let the Queue Discard Folder and Archive States

**Locations to change**
[queue.ts#L54](../../ui-web/src/thumbnails/queue.ts#L54), [queue.test.ts](../../ui-web/src/thumbnails/queue.test.ts#L1)

**What to do**
Add `discardDelayed(): void`, which, for the keys of the latest `delayed` set, revokes ready URLs, drops ready and failed states, aborts in-flight requests, and pumps the queue again so that keys still wanted are requested.

**Completion condition**
Unit tests show that a ready and a failed folder key are requested again after `discardDelayed`, and an image key is not.

<a id="s-5"></a>
### Step 5. Discard Them When the Library Is Refreshed

**Locations to change**
[listing.ts#L475](../../ui-web/src/library/listing.ts#L475), [useThumbnailQueue.ts#L29](../../ui-web/src/thumbnails/useThumbnailQueue.ts#L29), [useThumbnailQueue.test.tsx](../../ui-web/src/thumbnails/useThumbnailQueue.test.tsx#L1)

**What to do**
Have `refreshLibrary` notify subscribers per Library after the listing refresh resolves, and have `useThumbnailQueue` subscribe for its `libraryId` and call `discardDelayed` on its queue.

**Completion condition**
A hook test shows that `refreshLibrary` for the queue's Library causes a folder key to be requested again, and a refresh of another Library does not.

<a id="s-6"></a>
### Step 6. Confirm in a Browser and Close the Defect

**Locations to change**
`docs/issues/representative-thumbnail-stale.md` (deleted), [issues.md#L25](../issues.md#L25), [thumbnail.md §11](../design/thumbnail.md#11-current-state-and-remaining-work), [client.md §6](../design/client.md#6-current-state-and-remaining-work), [design.md §12.2](../design.md#122-roadmap)

**What to do**
Repeat the observation of Step 1 with the change, then delete the issue and its row, remove the current-state sentences about the old behavior, and mark T1, C11, and M20 as done.

**Completion condition**
Condition 4 is observed, and `rg -n 'representative-thumbnail-stale' docs` returns only this plan.

## 6. Acceptance Checks

| Condition | Command | Expected result |
| --- | --- | --- |
| 1, 2 | `cd daemon && RUSTC_WRAPPER= cargo nextest run -p omnius-pxna-daemon thumbnail` | The router tests for headers and `If-None-Match` pass |
| 3 | `cd ui-web && npx vitest run src/thumbnails` | The queue and hook tests for discarding pass |
| 4 | playwright-cli on the web client served by the daemon, following Step 6 | The tile becomes the icon, or the remaining image, without scrolling, and the network log shows a 200 or 304 from the server, not the disk cache |
| 5 | `rg -n 'representative-thumbnail-stale' docs` and `rg -n 'same .Cache-Control: private, max-age=86400. as files\|does not discard the states of folder and archive tiles' docs/design` | Only `docs/plans/representative-thumbnail-refresh.md` matches the first, and the second returns nothing |
| 6 | `cd daemon && cargo make lint && RUSTC_WRAPPER= cargo nextest run --all-features` and `cd ui-web && npm run lint && npm run typecheck && npm test` | All exit with 0 |
