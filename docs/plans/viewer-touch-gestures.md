# Operate the Image Viewer with Touch Gestures

## 1. About This Plan

This work lets the image viewer be operated on a phone with left and right flicks, pinch, double tap, and a downward swipe.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The steps and acceptance checks of this work |
| [Client Design §3.4](../design/client.md#34-viewer) | The behavior of the viewer's gestures |
| [Client Design §5.1](../design/client.md#add-double-tap-and-a-downward-swipe-to-the-viewers-touch-operations) | The decision on the set of gestures and its reasons |

[design.md](../design.md#11-division-of-responsibility-between-documents) holds the map of the document family.

### 1.2 Creation Date and Target Commit

Created 2026-10-23 / Target commit `47b81b0`

## 2. Purpose and Completion Conditions

On a phone, the owner can move between images, enlarge an image to the original, and close the viewer with one hand.

### 2.1 Completion Conditions

1. At the whole view, a horizontal flick shorter than 300 ms and longer than 50 CSS px moves to the next image (leftward) or the previous image (rightward), skipping non-images as the keys do
2. A two-finger pinch changes the zoom around the midpoint of the fingers, within the limits of `clampZoom`
3. A double tap at the whole view enlarges to 2 times around the tapped point, and a double tap while zoomed returns to the whole view. With the narrow-screen timing "only on zoom", the enlargement requests the original image
4. At the whole view, a downward swipe longer than 80 CSS px closes the viewer the same way as the close button
5. While zoomed, a one-finger drag moves the display position and does not move or close the viewer
6. In the video viewer, horizontal and downward swipes neither move nor close the viewer
7. In Chromium's mobile emulation, a downward swipe on the image does not start pull-to-refresh or scroll the page
8. `cd ui-web && npm run lint && npm run typecheck && npm test` pass

### 2.2 Non-Goals

- Momentum and animation of following the finger during a flick. The image changes when the flick ends.
- Touch gestures outside the viewer, such as in the grid. Client Design §5.1 keeps Entry operations to wide screens.

## 3. Current State

[Viewer.tsx#L735](../../ui-web/src/components/Viewer.tsx#L735) handles only one pointer, and only to drag while `zoom > 1`.
Zoom changes only through the wheel at [Viewer.tsx#L712](../../ui-web/src/components/Viewer.tsx#L712) via [zoomView at viewer.ts#L181](../../ui-web/src/viewer/viewer.ts#L181).
Moving is done by `stepForward` and `stepBackward` at [Viewer.tsx#L263](../../ui-web/src/components/Viewer.tsx#L263), and closing by `requestClose` at [Viewer.tsx#L321](../../ui-web/src/components/Viewer.tsx#L321). Neither is reachable from `ViewerImage` at [Viewer.tsx#L683](../../ui-web/src/components/Viewer.tsx#L683).
The viewer's stage sets neither `touch-action` nor `overscroll-behavior`.
On a narrow screen the original image is fetched only on zoom by default, so without these gestures it cannot be shown by touch alone.

## 4. Change Policy

Recognize the gestures in a pure function over pointer samples, so that thresholds can be tested without a browser, and connect its results to the existing zoom state and to callbacks that `ViewerImage` receives from the viewer.

### 4.1 Decisions Assumed

| Decision | Source |
| --- | --- |
| The set of gestures, the whole-view limit for flicks and the swipe, no gestures on videos, Pointer Events without a library |
| The thresholds of each gesture and the CSS of the image area | [Client Design §3.4](../design/client.md#34-viewer) | [Client Design §5.1](../design/client.md#add-double-tap-and-a-downward-swipe-to-the-viewers-touch-operations) |
| An enlarged view counts as zooming for fetching the original image | [Client Design §3.4](../design/client.md#34-viewer) |

### 4.2 Scope of Impact

`ui-web/src/viewer/viewer.ts`, a new `ui-web/src/viewer/gestures.ts`, and `ui-web/src/components/Viewer.tsx` change.
No API, configuration, or stored format changes.

## 5. Steps

| No. | Content | Dependencies | State |
| --- | --- | --- | --- |
| [1](#s-1) | Recognize gestures from pointer samples | - | Not started |
| [2](#s-2) | Zoom around a point | - | Not started |
| [3](#s-3) | Connect the gestures to the image viewer | 1, 2 | Not started |
| [4](#s-4) | Confirm in mobile emulation and update the current state | 3 | Not started |

<a id="s-1"></a>
### Step 1. Recognize Gestures from Pointer Samples

**Locations to change**
New `ui-web/src/viewer/gestures.ts` and `ui-web/src/viewer/gestures.test.ts`.

**What to do**
Export a recognizer that takes pointer down, move, and up samples with pointer ids, times, and positions, and the current zoom, and returns one of `flickLeft`, `flickRight`, `swipeDown`, `doubleTap` with its point, `pinch` with its scale and midpoint, `pan`, or nothing. Use the thresholds of §2.1, and 300 ms and 30 CSS px between taps for a double tap.

**Completion condition**
Unit tests cover each gesture at and around its thresholds, and that flicks and the swipe are not returned when the zoom is above 1.

<a id="s-2"></a>
### Step 2. Zoom Around a Point

**Locations to change**
[viewer.ts#L181](../../ui-web/src/viewer/viewer.ts#L181), [viewer.test.ts](../../ui-web/src/viewer/viewer.test.ts#L1)

**What to do**
Add a function that scales a `ViewTransform` by a factor around a point and clamps it with `clampZoom` and `clampPan`, and use it for both pinch and double tap.

**Completion condition**
Unit tests show that the point under the fingers stays in place and that the result stays within the zoom and pan limits.

<a id="s-3"></a>
### Step 3. Connect the Gestures to the Image Viewer

**Locations to change**
[Viewer.tsx#L683](../../ui-web/src/components/Viewer.tsx#L683), [Viewer.tsx#L735](../../ui-web/src/components/Viewer.tsx#L735), [Viewer.tsx#L448](../../ui-web/src/components/Viewer.tsx#L448)

**What to do**
Pass `onFlick(direction)` and `onSwipeDown()` from the viewer into `ViewerImage`, mapping them to the same functions as the arrow keys and the close button, and feed the stage's pointer events to the recognizer. Give the stage `touch-action: none` and `overscroll-behavior: contain`. Do not pass the callbacks to the video viewer.

**Completion condition**
Component tests with synthesized pointer events show a flick calling the step function, a swipe calling close, and a double tap changing the zoom, and the video viewer receiving no callbacks.

<a id="s-4"></a>
### Step 4. Confirm in Mobile Emulation and Update the Current State

**Locations to change**
[client.md §6](../design/client.md#6-current-state-and-remaining-work), [design.md §12.2](../design.md#122-roadmap)

**What to do**
Run the checks of conditions 1 to 7 in Chromium's mobile emulation with playwright-cli, then remove the sentence that the viewer lacks these gestures and the sentence about touch-only original images, and mark C10 and M15 as done.

**Completion condition**
All observations of §6 for conditions 1 to 7 match, and `rg -n 'until C10' docs` returns nothing.

## 6. Acceptance Checks

| Condition | Command | Expected result |
| --- | --- | --- |
| 1, 4, 5 | `cd ui-web && npx vitest run src/viewer/gestures.test.ts src/components` | Threshold and component tests for flicks, the swipe, and dragging while zoomed pass |
| 2, 3 | `cd ui-web && npx vitest run src/viewer/viewer.test.ts src/viewer/gestures.test.ts` | Tests for zooming around a point and for double tap pass |
| 1 to 7 | playwright-cli with a mobile device profile on the web client served by the daemon: flick, pinch, double tap, swipe down on an image, the same on a video, and a swipe down at the top of the page | Each gesture behaves as its condition states, the video viewer does not move or close, and the page does not reload |
| 8 | `cd ui-web && npm run lint && npm run typecheck && npm test` | All exit with 0 |
