# Remove the Thumbnail GET Route

## 1. About This Plan

After the web client switches to the image channel, `GET /libraries/{libraryId}/thumbnails` has no caller in the repository. This work removes the route, its contract, and its tests, so that the image channel is the only path for thumbnails, sprite sheets, and display images.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The steps, completion conditions, and scope of this work |
| [Image Channel Design §2.1](../design/image-channel.md#21-what-is-not-built), [§9.1](../design/image-channel.md#carry-thumbnails-and-display-images-over-one-websocket-per-tab), [§10](../design/image-channel.md#10-current-state-and-remaining-work) | Why there is no HTTP path, and the roadmap row IC3 |
| [design.md §3.2](../design.md#32-request-flow), [§12](../design.md#12-current-state-and-remaining-work) | The hand-written routes, and the current state that names the route |
| [daemon/openapi.yaml#L1242](../../daemon/openapi.yaml#L1242) | The contract the path is removed from |

[design.md](../design.md#11-division-of-responsibility-between-documents) holds the map of the document family.

### 1.2 Creation Date and Target Commit

Created 2026-10-26 / Target commit `d4030c6` with the uncommitted work of the same day

## 2. Purpose and Completion Conditions

Thumbnails, sprite sheets, and display images are reachable only through the image channel, and the contract, the generated code, and the documents no longer name the thumbnail route.

### 2.1 Completion Conditions

1. `openapi.yaml` has no `getThumbnail` operation, and none of the parameters and headers only it used (`ThumbnailVariantQuery`, `ThumbnailWidthQuery`, `ThumbnailHeightQuery`, `ThumbnailFormatQuery`, `ResizeQuery`, `ResamplerQuery`, `AnchorQuery`, `PriorityHeader`, `PxnaFrameCount`)
2. The regenerated interface and client schema have no diff after the commit, and the daemon has no thumbnail route and no implementation of the removed trait
3. `GET /api/v1/libraries/{libraryId}/thumbnails` returns the API's 404
4. The behaviors the route's HTTP tests protected that the channel keeps, the default format, the content type, the `ETag` per key, and the error kinds, are covered by channel or engine tests
5. No design document, terms document, or README names the route outside a rejected alternative or the history of a decision
6. `cd daemon && RUSTC_WRAPPER= cargo make lint`, `cd daemon && RUSTC_WRAPPER= cargo make test`, and `cd ui-web && npm run typecheck && npm test` pass

### 2.2 Non-Goals

- Keeping the `format`, `resampler`, and `anchor` choices for other clients. The channel uses the server's configuration ([Image Channel Design §4](../design/image-channel.md#4-wanted-set)).

## 3. Current State

The route is mounted in the outer router ([server.rs#L63](../../daemon/entrypoints/daemon/src/interface/server.rs#L63)) and implemented in [route.rs#L31](../../daemon/entrypoints/daemon/src/interface/features/thumbnails/route.rs#L31), with the uncalled implementation of the generated trait and the HTTP tests in [thumbnails.rs#L19](../../daemon/entrypoints/daemon/src/interface/features/thumbnails.rs#L19).
The contract is the path at [openapi.yaml#L1242](../../daemon/openapi.yaml#L1242) and its parameters and headers from [openapi.yaml#L2130](../../daemon/openapi.yaml#L2130) to [openapi.yaml#L2200](../../daemon/openapi.yaml#L2200) and at [openapi.yaml#L2296](../../daemon/openapi.yaml#L2296), each used only by this path.
design.md §3.2 and §12.1, Thumbnail Design §11.1, and Image Channel Design §10.1 describe the route as current code.
While the route stays, the daemon keeps a second path with its own ordering and caching rules beside the channel.

## 4. Change Policy

Remove the path and its components from the contract, regenerate, delete the route and the uncalled implementation, and move the tests of behaviors the channel keeps to the channel or the engine. Then remove the route from the current-state sections.

### 4.1 Decisions Assumed

| Decision | Source |
| --- | --- |
| There is no HTTP path for thumbnails, sprite sheets, or display images | [Image Channel Design §2.1](../design/image-channel.md#21-what-is-not-built) |
| Items do not carry format, resampler, or anchor | [Image Channel Design §4](../design/image-channel.md#4-wanted-set) |
| `image-channel-client` is done | [image-channel-client](./image-channel-client.md) |

### 4.2 Scope of Impact

`openapi.yaml`, the generated interface, the web client's generated schema, the daemon's outer router, and the thumbnail feature module change.
The published API loses a path, so a client outside this repository that calls it breaks.

## 5. Steps

| No. | Content | Dependencies | State |
| --- | --- | --- | --- |
| [1](#s-1) | Move the route's tests of kept behaviors to the channel and the engine | - | Done |
| [2](#s-2) | Remove the path from the contract and regenerate | 1 | Done |
| [3](#s-3) | Delete the route and the uncalled implementation | 2 | Done |
| [4](#s-4) | Update the documents | 3 | Done |

<a id="s-1"></a>
### Step 1. Move the Route's Tests of Kept Behaviors to the Channel and the Engine

**Locations to change**
[thumbnails.rs#L19](../../daemon/entrypoints/daemon/src/interface/features/thumbnails.rs#L19), `daemon/entrypoints/daemon/src/interface/features/image_channel.rs`

**What to do**
For each HTTP test of the route, keep the behavior through the channel when the channel has it (the default format and its content type, the `ETag` per key, the error kinds of missing and unsupported files, folders, and archives), and drop the tests of `format`, `resampler`, `anchor`, `Cache-Control`, and `Pxna-Priority`, which the channel does not have.

**Completion condition**
Completion condition 4 holds with the route still present.

<a id="s-2"></a>
### Step 2. Remove the Path from the Contract and Regenerate

**Locations to change**
[openapi.yaml#L1242](../../daemon/openapi.yaml#L1242), [openapi.yaml#L2130](../../daemon/openapi.yaml#L2130), [openapi.yaml#L2296](../../daemon/openapi.yaml#L2296), `daemon/entrypoints/interface` (generated), `ui-web/src/api/schema.d.ts` (generated)

**What to do**
Delete the path and the parameters and headers of completion condition 1, and the `Thumbnails` tag if nothing else uses it. Regenerate with `daemon/entrypoints/gen-interface.sh` and `cd ui-web && npm run gen:api`.

**Completion condition**
Completion condition 1 holds, and the generated files have no other change.

<a id="s-3"></a>
### Step 3. Delete the Route and the Uncalled Implementation

**Locations to change**
[server.rs#L63](../../daemon/entrypoints/daemon/src/interface/server.rs#L63), [route.rs#L31](../../daemon/entrypoints/daemon/src/interface/features/thumbnails/route.rs#L31), [thumbnails.rs#L19](../../daemon/entrypoints/daemon/src/interface/features/thumbnails.rs#L19), [features.rs#L12](../../daemon/entrypoints/daemon/src/interface/features.rs#L12)

**What to do**
Remove the route from the outer router and delete the route and the feature module, keeping the error mapping the channel shares. Add a router test that the old path returns the API's 404.

**Completion condition**
Completion conditions 2 and 3 hold.

<a id="s-4"></a>
### Step 4. Update the Documents

**Locations to change**
[design.md](../design.md), [thumbnail.md](../design/thumbnail.md), [image-channel.md](../design/image-channel.md), [README.md](../../README.md), [plans.md](../plans.md)

**What to do**
Remove the route from the current-state sections of design.md §12.1, Thumbnail Design §11.1, and Image Channel Design §10.1, mark IC3 and M23 done, and remove this plan's row from `plans.md`.

**Completion condition**
`rg -n 'libraries/\{libraryId\}/thumbnails|getThumbnail' docs README.md --glob '!docs/plans/**' --glob '!docs/reviews/**' --glob '!docs/measurements/**'` finds only rejected alternatives and the history of decisions.

## 6. Acceptance Verification

| Condition | Command | Expected result |
| --- | --- | --- |
| 1 | `rg -n 'getThumbnail\|ThumbnailVariantQuery\|PriorityHeader\|PxnaFrameCount' daemon/openapi.yaml` | No match |
| 2 | `daemon/entrypoints/gen-interface.sh && (cd ui-web && npm run gen:api) && git status --porcelain -- daemon/entrypoints/interface ui-web/src/api` | No change |
| 3 | `cd daemon && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon router` | The 404 test of step 3 passes |
| 4 | `cd daemon && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon image_channel` | The moved tests pass |
| 5 | The command of step 4's completion condition | Only rejected alternatives and the history of decisions |
| 6 | `cd daemon && RUSTC_WRAPPER= cargo make lint && RUSTC_WRAPPER= cargo make test`, `cd ui-web && npm run typecheck && npm test` | All pass |

The moved tests protect the default format, the content type, the `ETag` per key, and the error kinds through the channel.

## 7. Risks and Rollback

Removing a path from the published API is not undone for a client that has already been changed to stop using it, and a client outside this repository that still calls the path gets 404 from this release on.
`git revert` brings the route back for the next release; a client that broke in between has to wait for that release or switch to the channel.
