# Warm the Viewer's Thumbnails from One Decode and Add Viewer Settings

## 1. About This Plan

This work makes one decode of an image or video produce the thumbnail the listing asked for, the small thumbnails the viewer's filmstrip asks for, and both thumbnail shapes, so that switching shape or opening the viewer does not decode the source again. It also moves the viewer-specific display preferences into a settings panel inside the viewer and adds the filmstrip's size to them.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The steps, completion conditions, and scope of this work |
| [Thumbnail Design §3](../design/thumbnail.md#3-determining-the-target), [§6.2](../design/thumbnail.md#62-generate-together), [§7](../design/thumbnail.md#7-order-and-concurrency-of-generation), [§8.3](../design/thumbnail.md#83-keys), [§10.1](../design/thumbnail.md#101-decided) | How many images one decode produces, their cache keys, their scheduling cost, and the decision with its reasons |
| [Client Design §3.2](../design/client.md#32-tiles), [§3.4](../design/client.md#34-viewer), [§3.9](../design/client.md#39-display-preferences), [§5.1](../design/client.md#51-decided), [§6.1](../design/client.md#61-current-code) | Tiles, the viewer's filmstrip, display preferences, where each setting is changed, and the current code |
| [design.md §9.2](../design.md#92-where-state-lives) | Where display preferences are saved, and what else shares the database |
| [daemon/openapi.yaml](../../daemon/openapi.yaml) | The authoritative shape and ranges of display preferences |
| [measurements.md](../measurements.md) | The record of the fan-out cost measured after the change |

[design.md](../design.md#11-division-of-responsibility-between-documents) holds the map of the document family.

### 1.2 Creation Date and Target Commit

Created 2026-10-26 / Target commit `d4030c6`

## 2. Purpose and Completion Conditions

The listing currently decodes a source once for the thumbnail it asked for, and the viewer decodes the same source again for the filmstrip and the display image. Changing the thumbnail shape also decodes the source again. This work keeps the number of decodes to one per generation and turns the other variants into ordinary cache entries.

### 2.1 Completion Conditions

1. One generation of an image or video produces the requested variant, the smallest three size steps in both `crop` and `fit`, and, when the requested variant is a thumbnail or a sprite, the other shape at the requested size, all from one decode
2. The companions use the configured default format, the thumbnail quality, the `Lanczos3` resampler, and the center anchor; companion keys are deduplicated, and the primary key is not made again
3. Switching the thumbnail shape between `crop` and `fit` at a size whose companion is cached is answered from the cache without a new conversion or decode
4. A filmstrip request at 128, 256, or 512 with `crop` is answered from the cache after a listing request warmed it
5. A display or sprite request warms the same companions; a request served by the primary cache hit does not decode for companions
6. A companion that fails to validate or encode is logged and dropped; a store that skips the entry because it exceeds the bucket, or fails with an I/O or SQL error, is logged; in every case the primary response still succeeds. Tests assert the response, not the log text
7. `DisplaySet` carries `filmstripSize` from 1 to 5 in both sets, and the filmstrip uses 40, 48, 56, 72, or 96 CSS px for the item's side, defaulting to 3 (56 CSS px)
8. The viewer has a settings panel with the filmstrip size (wide screens only) and the original image swap (images only), and the listing's settings menu no longer has the original image swap
9. `openapi.yaml` marks `filmstripSize` as required, the generated interface and client schema agree with it, and the server validates 1 to 5
10. Existing primary keys and their bytes do not change, and `THUMBNAIL_FORMAT_VERSION` stays at 3
11. `cd daemon && RUSTC_WRAPPER= cargo make lint`, the daemon tests, and `cd ui-web && npm run lint && npm run typecheck && npm test` pass
12. The old preferences JSON makes `GET /preferences` fail with `SerdeError`, the web client runs with the defaults, and the next `PUT` saves the new shape; the plan does not depend on deleting the state directory

### 2.2 Non-Goals

- Generating the display image together with the listing's thumbnail. It roughly doubles the listing's generation time and is wasted for images that are never opened.
- Keeping the decoded source on disk as PNG, PPM, or WebP. Encoding it costs more than the HEIC decode it would save.
- Asking ffmpeg to write WebP directly. With fan-out it needs multiple outputs and a second implementation of the shape and size rules.
- Lowering libwebp's `method` or changing the display's output size.
- Tile-parallel decoding beyond what ffmpeg already does.
- Showing the filmstrip on narrow screens. The narrow set keeps a default filmstrip size but does not use or edit it.

## 3. Current State

[generator.rs#L330](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L330) `Inner::plan` takes the shape, size, format, and quality from the single request, so one generation produces one thumbnail shape at one size.
[generator.rs#L392](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L392) `produce_traced` decodes the source once, but the only extra product is the video's sprite sheet ([generator.rs#L479](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L479)); the video path decodes and resizes each extracted frame in turn and drops it ([generator.rs#L491](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L491)).
The filmstrip requests `crop` at a size derived from `FILMSTRIP_ITEM_WIDTH = 56` ([filmstrip.ts#L6](../../ui-web/src/viewer/filmstrip.ts#L6)), which the server rounds to 128 or 256; a listing request at 512 or 1024 does not warm it.
Changing the thumbnail shape changes `resize` in the cache key ([generator.rs#L330](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L330)), so every visible tile decodes its source again.
The filmstrip's section height and its items are fixed at 64 and 56 CSS px ([Filmstrip.tsx#L93](../../ui-web/src/components/Filmstrip.tsx#L93)); its centering effect does not depend on the item side, and `estimateSize` is not in the virtualizer's measurement dependencies.
The listing's settings menu holds the original image swap ([SettingsMenu.tsx#L118](../../ui-web/src/components/SettingsMenu.tsx#L118)); the viewer has no settings panel, and the viewer's `nestedOpen` only covers the narrow menu and the delete dialog ([Viewer.tsx#L349](../../ui-web/src/components/Viewer.tsx#L349)).
`DisplaySet` has `tileSize`, `thumbnailShape`, `view`, and `originalImage`, and no filmstrip size ([preferences.rs#L52](../../daemon/modules/engine/src/model/preferences.rs#L52)).
`ThumbnailCache::put` returns `Ok(false)` when an entry alone exceeds the bucket ([cache.rs#L140](../../daemon/modules/engine/src/core/thumbnail/cache.rs#L140)), and each `put` runs eviction ([cache.rs#L171](../../daemon/modules/engine/src/core/thumbnail/cache.rs#L171)).
`SingleFlight` is keyed by the primary cache key only ([generator.rs#L271](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L271)).
`PreferencesService::get` returns `SerdeError` when the saved JSON cannot be read with the current `DisplaySet` ([service/preferences.rs#L37](../../daemon/modules/engine/src/service/preferences.rs#L37)).

## 4. Change Policy

The engine builds a set of conversion plans from the source and the primary plan, decodes and normalizes the source once, and resizes and encodes every plan in that one blocking task. The companions are ordinary thumbnail cache entries with their own keys, stored in the thumbnail bucket. The web client sends the filmstrip's size through the existing thumbnail request and stores that size with the other display preferences; the server does not read the preference when generating.

### 4.1 Decisions Assumed

| Decision | Source |
| --- | --- |
| One decode produces the primary, the smallest three steps in both shapes, and the other shape at the requested thumbnail or sprite size | This plan, to be written into Thumbnail Design §3 and §10.1 |
| Companions use the default format, the thumbnail quality, `Lanczos3`, and the center anchor, and are stored in the thumbnail bucket | This plan, to be written into Thumbnail Design §8.3 |
| Companions are generated for image and video `thumbnail`, `display`, and `sprite` requests, only on a primary cache miss, synchronously in the same job, and best-effort | This plan, to be written into Thumbnail Design §10.1 |
| The decoded source is not cached on disk; ffmpeg does not write WebP directly; libwebp keeps `method` 2; tiles are not parallelized by us | This plan, to be written into Thumbnail Design §10.1 as rejected or deferred alternatives |
| Companion keys are deduplicated as a set; the primary and sprite tags differ, so no sprite comparison is needed | This plan, to be written into Thumbnail Design §8.3 |
| Companions are stored before the primary and the sprite, so a small bucket cannot evict the requested image for a companion; an oversized entry is a skip, not an error | This plan, to be written into Thumbnail Design §10.1 and §8.2 |
| Companions are not carried in the shared flight result; they are stored by the job before it returns | This plan |
| A video keeps only the first frame at full size; the remaining frames are resized one at a time and dropped as now | This plan, to be written into Thumbnail Design §6.2 |
| The single-flight window that lets a concurrent request for another size decode twice is accepted and tested | This plan, to be written into Thumbnail Design §7 |
| The filmstrip's size is a display preference from 1 to 5, is edited and used on wide screens only, and maps to 40, 48, 56, 72, and 96 CSS px | This plan, to be written into Client Design §3.4 and §3.9 |
| Viewer-only settings move into a viewer panel, and the listing's settings menu keeps only listing settings | This plan, to be written into Client Design §3.9 and §5.1 |
| The filmstrip's item side change calls `virtualizer.measure()` and re-centers the open Entry | This plan, to be written into Client Design §3.4 |
| The gear is hidden when the panel would have no control: narrow videos have neither setting | This plan, to be written into Client Design §3.4 |
| `filmstripSize` is required in the preferences contract; old saved JSON fails `GET` and the client falls back to defaults until the next `PUT` | This plan, to be written into `openapi.yaml` and the preferences model |
| Tests for companions request WebP, because the configured default format is WebP while the existing helpers use JPEG | This plan |
| `ImageConverter::resize_ref` returns `Cow<'_, DynamicImage>` so the Fit path does not copy the source | This plan |
| The primary's keys and bytes do not change, so `THUMBNAIL_FORMAT_VERSION` stays at 3 | This plan |

### 4.2 Scope of Impact

The engine's size steps, image converter, thumbnail cache, and thumbnail generator change.
The display preferences model and its API conversion change, and `openapi.yaml` and the generated interface change with them.
The web client's generated schema, display preferences defaults, filmstrip sizing, viewer header, viewer settings panel, listing settings menu, and the popover's portal handling inside the modal change.
The existing thumbnail keys and responses for the requested variant do not change.
A `DisplaySet` saved by the previous build has no `filmstripSize`, so `GET /preferences` returns `SerdeError`; the web client runs with the defaults, and the next `PUT` saves the new shape. Deleting only the saved preferences row is enough for this change. Deleting the whole state directory also removes users, sessions, ratings, and tags, so it is not the minimal reset.

## 5. Steps

| No. | Content | Dependencies | State |
| --- | --- | --- | --- |
| [1](#s-1) | Add `filmstripSize` to the preferences contract, models, and client defaults | - | Done |
| [2](#s-2) | Add the viewer settings panel and move the original image swap | 1 | Done |
| [3](#s-3) | Size the filmstrip from `filmstripSize` | 1 | Done |
| [4](#s-4) | Add the decode-once helpers to the size steps, the image converter, and the cache | - | Done |
| [5](#s-5) | Build and deduplicate the companion plans | 4 | Done |
| [6](#s-6) | Fan out images and ffmpeg images from one decode | 5 | Done (ffmpeg path unverified) |
| [7](#s-7) | Fan out videos from the first frame and the frames already extracted | 5 | Done (unverified: needs ffmpeg) |
| [8](#s-8) | Store companions best-effort, in the right order, and skip cache hits | 6, 7 | Done |
| [9](#s-9) | Add engine and web client tests | 2, 3, 8 | Done (ffmpeg tests skipped here) |
| [10](#s-10) | Update the design documents and measure the fan-out cost | 9 | Done |
| [11](#s-11) | Run the full verification | 10 | In progress (the 3 postgres tests fail because the sandbox cannot reach Docker's published ports) |

<a id="s-1"></a>
### Step 1. Add `filmstripSize` to the Preferences Contract, Models, and Client Defaults

**Locations to change**
[preferences.rs#L52](../../daemon/modules/engine/src/model/preferences.rs#L52), [preferences.rs#L85](../../daemon/modules/engine/src/model/preferences.rs#L85), [preferences.rs#L119](../../daemon/modules/engine/src/model/preferences.rs#L119), [preferences.rs#L145](../../daemon/modules/engine/src/model/preferences.rs#L145), [preferences.rs#L113](../../daemon/entrypoints/daemon/src/interface/features/preferences.rs#L113), [preferences.rs#L131](../../daemon/entrypoints/daemon/src/interface/features/preferences.rs#L131), [preferences.rs#L162](../../daemon/entrypoints/daemon/src/interface/features/preferences.rs#L162), [preferences.rs#L182](../../daemon/entrypoints/daemon/src/interface/features/preferences.rs#L182), [openapi.yaml](../../daemon/openapi.yaml), [preferences.ts](../../ui-web/src/library/preferences.ts), [preferences.ts](../../ui-web/src/testing/preferences.ts)

**What to do**
Add `filmstrip_size: u8` to `DisplaySet` with the same camelCase serde name as the other fields, `MIN_FILMSTRIP_SIZE = 1`, `MAX_FILMSTRIP_SIZE = 5`, and a default of 3 in both sets. Validate the range in `DisplaySet::validate` with a message that names the set.
Add `filmstripSize` to `DisplaySet` in `openapi.yaml` as a required integer from 1 to 5, add it to the required list, and put the property last so that the generated model keeps the existing field order. Regenerate the daemon interface with `daemon/entrypoints/gen-interface.sh` and the client schema with `cd ui-web && npm run gen:api`. Update `display_set_to_model` and `display_set_from_model`.
Add `filmstripSize: 3` to both sets in the engine defaults, `DEFAULT_PREFERENCES`, the interface test JSON at lines 162 and 182, and the web client's testing fixtures.
Add a `PreferencesService` test that inserts a preferences JSON without `filmstripSize` directly into the table and checks that `get` returns `SerdeError`.

**Completion condition**
The preferences round-trip test covers 1 and 5 and rejects 0 and 6 with `InvalidInput`. The generated `models::DisplaySet` has the field, and the client's `DisplaySet` type requires it. The old-JSON test returns `SerdeError`. `cd ui-web && npm run typecheck` passes.

<a id="s-2"></a>
### Step 2. Add the Viewer Settings Panel and Move the Original Image Swap

**Locations to change**
[popover.tsx#L22](../../ui-web/src/components/ui/popover.tsx#L22), [Viewer.tsx#L349](../../ui-web/src/components/Viewer.tsx#L349), [Viewer.tsx#L709](../../ui-web/src/components/Viewer.tsx#L709), [SettingsMenu.tsx#L118](../../ui-web/src/components/SettingsMenu.tsx#L118), [Modal.test.tsx](../../ui-web/src/components/Modal.test.tsx), [BrowsePage.tsx](../../ui-web/src/pages/BrowsePage.tsx), [SearchPage.tsx](../../ui-web/src/pages/SearchPage.tsx), [TimelinePage.tsx](../../ui-web/src/pages/TimelinePage.tsx), [radix-nested-layer-events.md](../knowledge/radix-nested-layer-events.md)

**What to do**
Let `PopoverContent` accept `container?: HTMLElement | null` and pass it to `PopoverPrimitive.Portal`, as `DropdownMenuContent` already does.
Pass `container={outer}` from the viewer's panel so that the popover is drawn inside the modal. Add the panel's open state to the viewer's `nestedOpen` so that the viewer's window key handler does not also consume Escape or the arrow keys. Confirm whether the popover handles its own Tab; if it does, add `[role='dialog']` to `Modal`'s `NESTED_LAYER`, otherwise leave the modal's focus trap in charge. Verify Escape, the slider's arrow keys, and Tab, and record the result in `docs/knowledge/radix-nested-layer-events.md`.
Add `filmstripSize: number` and `onDisplaySetChange: (patch: Partial<DisplaySet>) => void` to the viewer's props. Pass `filmstripSize={set.filmstripSize}` and `onDisplaySetChange={updateSet}` from the three pages.
Add a gear button to the viewer header only when the panel would show a control: on a wide screen when the open Entry is an image or a video, and on a narrow screen only when it is an image. The narrow gear sits next to the "その他" menu; the wide gear sits with the other header buttons. The gear opens a popover with `container={outer}`. The popover shows a "フィルムストリップの大きさ" slider from 1 to 5 when the screen is wide, and the "元の画像への差し替え" toggle from auto to on-zoom when the open Entry is an image.
Remove the original image field from `SettingsMenu`.
Keep the existing `originalImage` prop for the viewer's reading side, and do not keep two sources of the value.

**Completion condition**
A viewer test changes the filmstrip size and the original image swap from the panel and observes the written preferences patch. A settings menu test shows that the original image field is gone. Tests show that a narrow video has no gear, a narrow image has a gear with only the original image setting, and a wide image has both settings. Tests show that Escape, the slider's arrow keys, and Tab are handled inside the panel and do not close or move the viewer.

<a id="s-3"></a>
### Step 3. Size the Filmstrip from `filmstripSize`

**Locations to change**
[filmstrip.ts#L6](../../ui-web/src/viewer/filmstrip.ts#L6), [Filmstrip.tsx#L44](../../ui-web/src/components/Filmstrip.tsx#L44), [Filmstrip.tsx#L93](../../ui-web/src/components/Filmstrip.tsx#L93), [Filmstrip.tsx#L126](../../ui-web/src/components/Filmstrip.tsx#L126), [Viewer.tsx#L229](../../ui-web/src/components/Viewer.tsx#L229), [useThumbnailQueue.ts](../../ui-web/src/thumbnails/useThumbnailQueue.ts), [filmstrip.test.ts](../../ui-web/src/viewer/filmstrip.test.ts), [Filmstrip.test.tsx](../../ui-web/src/components/Filmstrip.test.tsx)

**What to do**
Replace `FILMSTRIP_ITEM_WIDTH` with `FILMSTRIP_ITEM_SIDES = [40, 48, 56, 72, 96]` and `filmstripItemSide(level: number): number`, clamping the level to 1 to 5. Level 3 stays 56.
Pass the side to `Filmstrip` as a prop. Use it for the virtualizer's `estimateSize`, the scroll adjustment, the item's width and height, the section's height (`side + 8`), and the inner thumbnail's width and height (`side - 4`). Replace the fixed `h-16` and `h-[52px] w-[52px]` classes with inline sizes.
Call `virtualizer.measure()` when the side changes, and add the side to the centering effect's dependencies so that the open Entry is centered again after a size change. Update the tests and fixtures that refer to `FILMSTRIP_ITEM_WIDTH`.
In the viewer, compute the side from `filmstripSize` and pass it to both `useThumbnailQueue` and `Filmstrip`. The queue's requested size is `Math.ceil(side * devicePixelRatio)` through the existing `thumbnailSize`.

**Completion condition**
A filmstrip test shows the section, item, inner thumbnail, and `getTotalSize()` for levels 1 and 5, and shows that the open Entry is centered after a size change. A `useThumbnailQueue` test shows that changing the level changes the requested size and recreates the queue. Selection, scrolling, and the placeholder and icon paths still work at both ends of the range.

<a id="s-4"></a>
### Step 4. Add the Decode-Once Helpers to the Size Steps, the Image Converter, and the Cache

**Locations to change**
[size_steps.rs](../../daemon/modules/engine/src/core/image/size_steps.rs), [converter.rs#L33](../../daemon/modules/engine/src/core/image/converter.rs#L33), [converter.rs#L53](../../daemon/modules/engine/src/core/image/converter.rs#L53), [cache.rs#L115](../../daemon/modules/engine/src/core/thumbnail/cache.rs#L115)

**What to do**
Add `SizeSteps::smallest(&self, count: usize) -> &[u32]`, returning the first `count` sorted steps or all of them when there are fewer, with a test.
Keep the existing ownership `resize(DynamicImage, ...)` as the implementation for `convert`, and add `resize_ref(&DynamicImage, &ImageConvertOptions) -> Result<Cow<'_, DynamicImage>>` for fan-out. In the `Fit` branch where the image already fits, return `Cow::Borrowed(image)`; in the other branches return `Cow::Owned` from the current `scale`, `crop_imm`, and `pad` behavior. This keeps the ownership path free of the extra Fit clone.
Add `ThumbnailCache::contains(&self, key) -> Result<bool>` that checks the index only, without reading the file or updating its recency.

**Completion condition**
The existing converter tests pass unchanged. A new test calls `resize_ref` twice on one decoded image and gets the same results as `resize` on a clone. `SizeSteps::smallest(3)` on the default steps is `[128, 256, 512]`, and a two-step set returns both. A cache test deletes the cached file and checks that `contains` still answers true from the index.

<a id="s-5"></a>
### Step 5. Build and Deduplicate the Companion Plans

**Locations to change**
[generator.rs#L121](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L121), [generator.rs#L128](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L128), [generator.rs#L330](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L330)

**What to do**
Store the configured default format in `Inner` so that companions do not follow an explicit format in the request.
Add `Inner::companion_plans(&self, plan: &Plan, kind: MediaKind, source: &SourceId) -> Result<Vec<(ThumbnailKey, ImageConvertOptions)>>`:
- For each of `sizes.smallest(3)`, make a `crop` and a `fit` option with the default format, `quality`, `Lanczos3`, center anchor, and that step as both the width and the height.
- When the primary variant is `Thumbnail` or `Sprite` and its resize type is `Crop` or `Fit`, add the same width and height with the other resize type.
- Compute each key with the same `SourceId`, the `Thumbnail` tag, and the media kind. Collect the keys into a `HashSet` and drop the primary key and any duplicate companion keys; do not compare with the sprite key, whose tag differs.
- Return an empty list when the source has no cache key.
Add `companions: Vec<(ThumbnailKey, ImageConvertOptions)>` to `Job`. Do not add companions to `Produced`; the job stores them before it returns, and the shared flight result keeps only the requested image and the sprite.

**Completion condition**
An engine test builds the companion plans for a 1024 `crop` listing request and expects the 128, 256, and 512 keys in both shapes and the 1024 `fit` key, with no duplicate keys. A display request expects only the smallest three steps in both shapes. A test with the primary at 128 `crop` expects the primary key to be removed from the companion set.

<a id="s-6"></a>
### Step 6. Fan Out Images and ffmpeg Images from One Decode

**Locations to change**
[generator.rs#L392](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L392), [generator.rs#L464](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L464), [generator.rs#L527](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L527)

**What to do**
Replace `convert_one` with a `convert_many` that takes the input bytes, the primary plan, and the companion plans. In one `spawn_blocking` call, decode the input once, then for the primary and each companion call `resize_ref` and `encode`. The primary's error fails the request with the existing `decode_error` mapping; a companion's validation or encode error is logged and that companion is dropped.
Use `convert_many` for `MediaKind::Image` and for the bytes `decode_ffmpeg_image` returns. The ffmpeg and heif-dec process counts do not change, because the decode is still one run.
Write the new tests with `ImageFormatType::Webp` requests; the existing `thumbnail_request` and `sprite_request` helpers use JPEG, which never matches the WebP companions.

**Completion condition**
An engine test requests a 512 `crop` WebP thumbnail and then the 512 `fit` thumbnail, and the second request has `conversions() == 1` and returns the same dimensions. A test that requests 128 WebP after a 512 WebP request finds it in the cache without a new conversion. `ffmpeg_runs()` and `heif_dec_runs()` do not increase for companions. A unit test passes a companion option that fails validation (for example `Pad` with JPEG) and observes that the primary is still returned.

<a id="s-7"></a>
### Step 7. Fan Out Videos from the First Frame and the Frames Already Extracted

**Locations to change**
[generator.rs#L479](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L479), [generator.rs#L491](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L491)

**What to do**
Keep only the first decoded frame at full size. Use it for the primary thumbnail and for each companion. Decode, resize, and drop the remaining frames one at a time as the current code does, so that eight full-size frames are not held at once. Keep the sprite sheet's current resize and horizontal tiling.
Do not extract or decode another frame for the companions.

**Completion condition**
A video test requests a WebP thumbnail and then its other shape, and the second request is served from the cache with `ffmpeg_runs() == 1`. A sprite request warms the same companions. The sprite sheet's bytes and frame count are unchanged.

<a id="s-8"></a>
### Step 8. Store Companions Best-Effort, in the Right Order, and Skip Cache Hits

**Locations to change**
[generator.rs#L392](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L392), [generator.rs](../../daemon/modules/engine/src/core/thumbnail/generator.rs), [cache.rs#L115](../../daemon/modules/engine/src/core/thumbnail/cache.rs#L115)

**What to do**
After the primary and sprite cache check misses, use `ThumbnailCache::contains` to remove the companions that are already cached, without reading them.
Store the generated companions first, then the primary and the sprite sheet. This keeps a small bucket from evicting the requested image to make room for a companion. An oversized entry returns `Ok(false)` and is a skip, not an error; an I/O or SQL error is logged. Neither fails the response. The stores are one batch ([thumbnail-store-batching](./thumbnail-store-batching.md)) and the response waits for it by the owner's decision; the measurement in Step 10 records that cost.
On a primary cache hit, return without looking up or generating companions.

**Completion condition**
A test that primes the primary cache shows that a second request does not increase `conversions()`. A test with a companion already in the cache shows that it is not encoded again. A small-capacity test computes the bucket from the measured primary and companion bytes so that the primary fits and all companions together do not, and shows that the primary remains cached. A test that removes the cache's `staging` directory shows that the response still succeeds and returns the primary; the implementation logs a warning, but the test does not assert the log. A unit test for the encode failure path leaves the primary response intact.

<a id="s-9"></a>
### Step 9. Add Engine and Web Client Tests

**Locations to change**
[generator.rs](../../daemon/modules/engine/src/core/thumbnail/generator.rs), [preferences.rs](../../daemon/modules/engine/src/model/preferences.rs), [preferences.rs](../../daemon/entrypoints/daemon/src/interface/features/preferences.rs), [service/preferences.rs](../../daemon/modules/engine/src/service/preferences.rs), [preferences.test.ts](../../ui-web/src/library/preferences.test.ts), [Viewer.test.tsx](../../ui-web/src/components/Viewer.test.tsx), [SettingsMenu.test.tsx](../../ui-web/src/components/SettingsMenu.test.tsx), [Filmstrip.test.tsx](../../ui-web/src/components/Filmstrip.test.tsx), [useThumbnailQueue.test.tsx](../../ui-web/src/thumbnails/useThumbnailQueue.test.tsx)

**What to do**
Cover the completion conditions that are not already covered by the steps above: the companion plan sets, the shape switch, the filmstrip hit, the display and sprite warm-up, the two failure paths, the preference range and round-trip, the old-JSON failure, the viewer panel edits, the removed listing setting, the panel visibility by screen width and kind, the key and focus behavior, and the filmstrip sizing and recentering.
Add a concurrent-request test modeled on `concurrent_requests_convert_once_test` that pins the accepted duplicate-decode window: a 512 WebP request and a 128 WebP request started together may both decode. Assert only that each returns the right image and that `conversions()` is between 1 and 2, because the timing decides whether they share a flight.
Use the existing `conversions`, `ffmpeg_runs`, and `heif_dec_runs` counters to prove that no extra decode or process run happened.

**Completion condition**
The new tests fail on the previous build and pass on this one.

<a id="s-10"></a>
### Step 10. Update the Design Documents and Measure the Fan-Out Cost

**Locations to change**
[thumbnail.md](../design/thumbnail.md), [client.md](../design/client.md), [terms.md](../terms.md), [measurements.md](../measurements.md), [plans.md](../plans.md), [useFlipbook.ts](../../ui-web/src/thumbnails/useFlipbook.ts)

The plan originally named `ui-web/src/thumbnails/api.ts` for the comment about the shared conditions. That file was removed by [image-socket-client](./image-socket-client.md), and the equivalent comment now lives in `useFlipbook.ts`.

**What to do**
Write the fan-out rule, the companion keys and bucket, the store order, and the rejected alternatives (eager display, decoded-source cache, direct ffmpeg WebP, method change, tile parallelism) into Thumbnail Design §3, §6.2, §7, §8.2, §8.3, and §10.1. Update the sentence in §6.2 that says the thumbnail and sprite are both present only under the same conditions, so that it also covers the companions. Record the cache-pressure trade-off in §10.1 as a residual risk.
Write the filmstrip size, the viewer settings panel, the moved original image swap, and the revised "gather display preferences" decision into Client Design §3.2, §3.4, §3.9, §5.1, and §6.1. Update the §3.9 table with `filmstripSize`, the "same width" sentence in §5.1, and the current-state header description in §6.1. Note that the narrow set's filmstrip size is saved but not used.
Add the term "companion" to `terms.md`, and update the comment in `ui-web/src/thumbnails/useFlipbook.ts` that says the thumbnail and the sprite sheet are generated only under the same conditions, so that it also covers the companions. The original location, `ui-web/src/thumbnails/api.ts`, was removed by image-socket-client.
Measure a listing thumbnail generation with and without companions, a shape switch, a filmstrip request that hits a companion, the bytes added per image, and the cache hit-rate change, using the existing fixture method. Add both the `.md` record and its `.json` aggregate to `docs/measurements/` and a row to `docs/measurements.md`.
Remove this plan's row from `plans.md` when the work is done.

**Completion condition**
The design documents describe the behavior that the tests check, and the measurement record states the conditions, the medians, the added bytes, the hit-rate change, and the limits.

<a id="s-11"></a>
### Step 11. Run the Full Verification

**Locations to change**
None.

**What to do**
Run the following and fix what they report.

```sh
cd daemon && RUSTC_WRAPPER= cargo make lint
cd daemon && RUSTC_WRAPPER= cargo make test
cd ui-web && npm run lint && npm run typecheck && npm test
```

After the final `openapi.yaml` edit, run `daemon/entrypoints/gen-interface.sh` and `cd ui-web && npm run gen:api`, and confirm that they produce no diff beyond the intended field. `gen-interface.sh` runs `npx --yes @openapitools/openapi-generator-cli@v2.38.0` and needs network access and a JVM; it recreates `daemon/entrypoints/interface` from scratch.

**Completion condition**
Every command passes, and the generated files have no uncommitted diff beyond the intended field.
