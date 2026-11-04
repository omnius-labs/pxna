# Add Gallery and Focus Modes to the Wide-Screen Viewer

## 1. About This Plan

This work adds two display modes to the wide-screen viewer beside its current presentation: a gallery that lays out works horizontally with their own aspect ratios, and a focus mode with wide margins.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The steps and acceptance checks of this work |
| [Client Design §3.4](../design/client.md#34-viewer) | The behavior of the display modes and the position meter |
| [Client Design §5.1](../design/client.md#give-the-wide-screen-viewer-three-display-modes) | The decisions on the modes and their reasons |
| [terms.md](../terms.md#2-term-list) | The names of the modes and the position meter |

[design.md](../design.md#11-division-of-responsibility-between-documents) holds the map of the document family.

### 1.2 Creation Date and Target Commit

Created 2026-11-04 / Target commit `a7fff39`

## 2. Purpose and Completion Conditions

On a wide screen, the owner can look through the works of a listing with their own aspect ratios and with wide margins, without leaving the viewer, from folder view, the timeline, and search mode.

### 2.1 Completion Conditions

1. On a wide screen, the viewer's header has a switch of three choices, "Standard", "Gallery", and "Focus", and on a narrow screen it has none and the viewer keeps its current presentation
2. The chosen mode is written to `localStorage` under `pxna.viewerMode` and the next viewer opening uses it. An absent or unknown value opens standard mode
3. Standard mode shows the filmstrip, and gallery and focus modes show a position meter instead. Clicking a bar moves to the image or video nearest that position, reading pages outward from it
4. Gallery mode lays out images and videos with the aspect ratios of their loaded `fit` thumbnails and the same height of 75% of the display area, gives other Entries no space, and requests no display images
5. In gallery mode the wheel scrolls horizontally, arrow keys move to the previous or next item and center it, and after scrolling stops the item nearest the center becomes `view` without pushing history, once the page of an unread center has arrived
6. When an earlier thumbnail loads in gallery mode, the open Entry stays at the center, or after a user scroll the position at the center stays in place. A listing of 1,000,000 Entries lays out at most 501 positions, and a far move still reaches its position
7. Gallery mode shows no zoom, slideshow button, or rate button, stops a running slideshow and rate mode on entry, opens focus mode on a double click, and on `r` moves to focus mode with rate mode on
8. A video in gallery mode shows a thumbnail with a mark and starts no transcode session
9. Focus mode keeps zoom, the swap to the original image, slideshow, rate mode, and video playback
10. `cd ui-web && npm run lint && npm run typecheck && npm test && npm run build` pass

### 2.2 Non-Goals

- The display modes on narrow screens. They follow [viewer-touch-gestures](./viewer-touch-gestures.md), because a touch layout is a separate design; Client Design §5.1 records why they wait.
- Changing the listing's square tiles or returning dimensions from the listing API, which [Client Design §5.1](../design/client.md#make-tiles-square-and-switch-between-crop-and-whole-display) rejects for its cost.
- Saving the mode on the server. Per-device saving is the decision in Client Design §5.1.

## 3. Current State

[ViewerView at Viewer.tsx#L164](../../ui-web/src/components/Viewer.tsx#L164) renders one image or video and, on a wide screen, [Filmstrip at Viewer.tsx#L616](../../ui-web/src/components/Viewer.tsx#L616). Its header is [HeaderActions at Viewer.tsx#L737](../../ui-web/src/components/Viewer.tsx#L737).
Display images for the current Entry and its neighbors are wanted in [the effect at Viewer.tsx#L263](../../ui-web/src/components/Viewer.tsx#L263) regardless of what is on screen.
[filmstripWanted at filmstrip.ts#L34](../../ui-web/src/viewer/filmstrip.ts#L34) turns a visible range into pages and thumbnail wants, and [useImageWants at wants.ts#L108](../../ui-web/src/images/wants.ts#L108) accepts an explicit side and shape.
[neighbor at viewer.ts#L46](../../ui-web/src/viewer/viewer.ts#L46) reads pages and skips other kinds of Entry, but nothing moves to an arbitrary position that may be unread.
The presentation is therefore fixed, and a work whose edges a crop would cut can be seen whole only one at a time.

## 4. Change Policy

Keep the layout and position calculations in pure functions, so that they can be tested without a browser, and let the viewer choose between the current stage, the gallery, and the filmstrip or meter from one mode value. The gallery positions its own items within a window around the viewed place, because a single layout has to answer both virtualization and keeping the center in place, and its cost must not grow with the listing.

### 4.1 Decisions Assumed

| Decision | Source |
| --- | --- |
| Three modes, the header switch, wide screens only, saving per device | [Client Design §5.1](../design/client.md#give-the-wide-screen-viewer-three-display-modes) |
| Thumbnails as the gallery's images and the source of its aspect ratios | [Client Design §5.1](../design/client.md#lay-out-gallery-mode-from-loaded-thumbnails) |
| Gallery mode keeps out zoom, slideshow, rate mode, and video playback | [Client Design §5.1](../design/client.md#keep-gallery-mode-for-viewing-and-keep-video-playback-out-of-it) |
| The numbers and behavior of the modes and the meter | [Client Design §3.4](../design/client.md#34-viewer) |

### 4.2 Scope of Impact

`ui-web/src/components/Viewer.tsx` and the English and Japanese strings change, and `ui-web/src/viewer/{displayMode,gallery,positionMeter}.ts`, `ui-web/src/components/{GalleryStrip,PositionMeter}.tsx`, and their tests are new.
The new `localStorage` key `pxna.viewerMode` and the names in [terms.md](../terms.md#2-term-list) are visible from outside the code. An absent key means standard mode, so compatibility does not break. The daemon, the API, and the stored display preferences do not change.

## 5. Steps

| No. | Content | Dependencies | State |
| --- | --- | --- | --- |
| [1](#s-1) | Save the display mode | - | Done |
| [2](#s-2) | Calculate the position meter and the gallery layout | - | Done |
| [3](#s-3) | Build the position meter and the gallery | 2 | Done |
| [4](#s-4) | Connect the modes to the viewer | 1, 3 | Done |
| [5](#s-5) | Confirm in a browser and update the current state | 4 | Done |

<a id="s-1"></a>
### Step 1. Save the Display Mode

**Locations to change**
New `ui-web/src/viewer/displayMode.ts` and its test, modeled on [infoPanel.ts#L16](../../ui-web/src/library/infoPanel.ts#L16).

**What to do**
Export the mode type, the key, reading that treats unknown values as standard, writing that ignores storage failures, and a hook returning the mode and its setter.

**Completion condition**
Tests show a saved value returning, unknown and absent values reading as standard, and a throwing storage not throwing out.

<a id="s-2"></a>
### Step 2. Calculate the Position Meter and the Gallery Layout

**Locations to change**
New `ui-web/src/viewer/positionMeter.ts`, `ui-web/src/viewer/gallery.ts`, and their tests. [viewer.ts#L46](../../ui-web/src/viewer/viewer.ts#L46) gains a function that finds the viewable Entry at or nearest a position.

**What to do**
The meter maps positions to bars and bars to positions, and gives bar heights by distance. The gallery computes each item's start and width within a window of positions around a center from the height, the viewport, and the aspect ratios, gives other Entries no width, finds the visible range, the viewable item nearest the center, converts between a center anchor and a scroll position, and decides when the window moves. The nearest-viewable search reads pages outward on both sides.

**Completion condition**
Tests cover bar mapping at both ends and with fewer Entries than bars, widths for loaded, unread, other, and extremely wide Entries, the visible range, the nearest item skipping other kinds, an anchor keeping its place across a width change before it, the window at both ends of a listing and when it moves, and a search that takes the nearer item on each side.

<a id="s-3"></a>
### Step 3. Build the Position Meter and the Gallery

**Locations to change**
New `ui-web/src/components/PositionMeter.tsx`, `ui-web/src/components/GalleryStrip.tsx`, and their tests. Strings in [en.ts#L280](../../ui-web/src/i18n/locales/en.ts#L280) and `ja.ts`.

**What to do**
Render only the items near the visible range, request their thumbnails and pages through `filmstripWanted`, record each aspect ratio on load, keep the open Entry or the center in place when the layout changes, map the wheel to horizontal scrolling, and report the nearest item after scrolling stops.

**Completion condition**
Component tests show aspect-ratio widths after a load, a wheel moving `scrollLeft`, the idle report selecting the nearest item and waiting for the page of an unread center, a layout of at most 501 positions in a listing of 1,000,000 that follows a far move and scrolling, a click and double click reporting their targets, and a video rendered without a player.

<a id="s-4"></a>
### Step 4. Connect the Modes to the Viewer

**Locations to change**
[Viewer.tsx#L164](../../ui-web/src/components/Viewer.tsx#L164), [Viewer.tsx#L263](../../ui-web/src/components/Viewer.tsx#L263), [Viewer.tsx#L366](../../ui-web/src/components/Viewer.tsx#L366), [Viewer.tsx#L473](../../ui-web/src/components/Viewer.tsx#L473), [Viewer.tsx#L533](../../ui-web/src/components/Viewer.tsx#L533), [Viewer.tsx#L615](../../ui-web/src/components/Viewer.tsx#L615), [Viewer.tsx#L737](../../ui-web/src/components/Viewer.tsx#L737), and [Viewer.test.tsx](../../ui-web/src/components/Viewer.test.tsx#L1).

**What to do**
Add the header switch, choose the stage and bottom part from the mode, stop wanting display images in gallery mode, hide the slideshow and rate buttons and the settings that have no effect, and move to focus mode for `r` and a double click. Narrow screens always use standard mode.

**Completion condition**
Viewer tests cover the switch's presence by screen width, the restored mode, the absence of display image wants in gallery mode, and `r` entering focus and rate mode, and all existing viewer tests still pass.

<a id="s-5"></a>
### Step 5. Confirm in a Browser and Update the Current State

**Locations to change**
[client.md §6.1](../design/client.md#61-current-code), [plans.md](../plans.md#2-plan-list)

**What to do**
Run the browser rows of §6 on the web client served by the daemon, keep the current-state description of the modes accurate to what was built, and mark this plan done.

**Completion condition**
Every observation of §6 matches, and the state of this plan in `docs/plans.md` is Done.

## 6. Acceptance Checks

| Condition | Command | Expected result |
| --- | --- | --- |
| 2 | `cd ui-web && npx vitest run src/viewer/displayMode.test.ts` | The saved, absent, and unknown value cases pass |
| 3, 4, 5, 6 | `cd ui-web && npx vitest run src/viewer/positionMeter.test.ts src/viewer/gallery.test.ts src/viewer/viewer.test.ts src/components/PositionMeter.test.tsx src/components/GalleryStrip.test.tsx` | The layout, window, anchor, meter, search, and interaction tests pass |
| 1, 4, 7, 8, 9 | `cd ui-web && npx vitest run src/components/Viewer.test.tsx` | The tests for the switch, wanted images, `r`, and video in each mode pass |
| 1 to 9 | playwright-cli at 1440x900 and 390x844 on the web client served by the daemon, opening folders with portrait, landscape, and video files, one of them with 700 images: switch modes, reload, wheel, press arrows and `r`, double-click, click meter bars, and read `localStorage`, the rendered item count, and the transcode requests | Each behaves as its condition states, the 390x844 viewer has no switch, and no `video/sessions` request appears in gallery mode |
| 10 | `cd ui-web && npm run lint && npm run typecheck && npm test && npm run build` | All exit with 0 |
