# Merge Every Request for a Source into One Source Job

## 1. About This Plan

The thumbnail generator merges only concurrent requests for the same cache key, so two sizes of one image, or a thumbnail and a display image, decode the image twice. This work replaces that single-flight with source jobs: every request for a source joins one job, and a source has at most one job decoding it in the whole daemon.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The steps, completion conditions, and scope of this work |
| [Thumbnail Design §7](../design/thumbnail.md#7-order-and-concurrency-of-generation), [§10.1](../design/thumbnail.md#one-source-job-per-source), [§11](../design/thumbnail.md#11-current-state-and-remaining-work) | The placing rules of source jobs, the decision with its reasons, and the roadmap row T2 |
| [Image Channel Design](../design/image-channel.md) | The channel that will pass items to source jobs, which this work does not build |

[design.md](../design.md#11-division-of-responsibility-between-documents) holds the map of the document family.

### 1.2 Creation Date and Target Commit

Created 2026-10-26 / Target commit `d4030c6` with the uncommitted work of [thumbnail-fanout-and-viewer-settings](./thumbnail-fanout-and-viewer-settings.md) and the heif-dec changes of the same day

## 2. Purpose and Completion Conditions

While several requests for different sizes or kinds of one image arrive, from one tab or several, the daemon decodes that image once at a time, and the existing `GET /libraries/{libraryId}/thumbnails` answers as before.

### 2.1 Completion Conditions

1. Two concurrent thumbnail requests for different sizes of one source, both waiting for a slot, return the right images with one conversion
2. A thumbnail request whose key is a companion of a job decoding its source returns the right image with no conversion of its own
3. A request for a size outside the companions, or a display image, that arrives while a job of its source decodes, returns the right image after that job ends, and at no point do two conversions of one source run at the same time
4. An Interactive request that joins a job waiting as Foreground makes the job start before a Foreground job of another source that started waiting earlier in the same pool
5. A job still waiting for its slot starts nothing when every request in it is cut, and a job that has started stores its images when every request in it is cut
6. A local file whose modification time cannot be obtained is generated per request, is not cached, and stops when its request is cut
7. A video's thumbnail and sprite sheet requested together still share one ffmpeg run
8. The existing HTTP tests of the thumbnail route pass unchanged
9. `cd daemon && RUSTC_WRAPPER= cargo make lint` and `cd daemon && RUSTC_WRAPPER= cargo make test` pass

### 2.2 Non-Goals

- The image channel and the switch of the web client to it. They are IC1 and IC2 of [Image Channel Design §10.2](../design/image-channel.md#102-roadmap), planned in [image-channel-server](./image-channel-server.md) and [image-channel-client](./image-channel-client.md).
- Changing the slots or threads of external processes. It is deferred ([Thumbnail Design §10.2](../design/thumbnail.md#threads-and-slots-of-the-external-processes-for-images)).

## 3. Current State

Generation merges requests by `SingleFlight<ThumbnailKey, Produced>` ([generator.rs#L108](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L108)), keyed by the primary key ([generator.rs#L291](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L291), [generator.rs#L303](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L303)).
A flight is created only after its request gets a slot ([single_flight.rs#L86](../../daemon/modules/engine/src/base/util/single_flight.rs#L86)), and a request cut while waiting for the slot starts nothing.
The companions are planned per request before the flight from the primary plan and the `SourceId` ([generator.rs#L152](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L152), [generator.rs#L276](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L276), [generator.rs#L388](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L388), [generator.rs#L453](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L453)), and `produce_traced` decodes, releases the slot, and stores the batch ([generator.rs#L481](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L481)).
The request carries its `WorkClass` ([request.rs#L28](../../daemon/modules/engine/src/core/thumbnail/request.rs#L28)), which the route takes from `Pxna-Priority` ([route.rs#L90](../../daemon/entrypoints/daemon/src/interface/features/thumbnails/route.rs#L90)).
`concurrent_requests_for_different_sizes_may_decode_twice_test` accepts one or two conversions for two sizes ([generator.rs#L2183](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L2183)), and `concurrent_thumbnail_and_sprite_share_one_ffmpeg_run_test` checks the shared video run ([generator.rs#L1437](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L1437)).
`SingleFlight` is also used by the representative selection and the archive index cache, so it stays.
In the log of 2026-10-26, 8 of 90 external decodes ran while another decode of the same source ran, so merging by key is not enough to keep one decode per source.

## 4. Change Policy

The generator keeps, per `SourceId`, at most one job waiting for its slot and at most one job decoding. A request that misses the cache is placed by the four rules of Thumbnail Design §7; a waiting job collects the plans of its requests and computes their companions when it gets its slot. Requests hold a handle to their job, and dropping the last handle of a waiting job discards it. The job answers each request with its own image from the decoded result. A request without a `SourceId` keeps the direct path.

### 4.1 Decisions Assumed

| Decision | Source |
| --- | --- |
| The four placing rules, the fixed plans of a started job, discarding a waiting job with no request, and running a started job to the end | [Thumbnail Design §7](../design/thumbnail.md#7-order-and-concurrency-of-generation) |
| One job decoding a source at a time, a display image included | [Thumbnail Design §10.1](../design/thumbnail.md#one-source-job-per-source) |
| `thumbnail-fanout-and-viewer-settings` is done | [thumbnail-fanout-and-viewer-settings](./thumbnail-fanout-and-viewer-settings.md) |

### 4.2 Scope of Impact

The thumbnail generator and its tests change, and a new module holds the source jobs.
No API, configuration key, cache key, or stored format changes, so compatibility does not break.

## 5. Steps

| No. | Content | Dependencies | State |
| --- | --- | --- | --- |
| [1](#s-1) | Add the source job table with its placing rules | - | Done |
| [2](#s-2) | Route generation through source jobs and remove the thumbnail flights | 1 | Done |
| [3](#s-3) | Replace the decode-twice test with tests of the conditions | 2 | Done |
| [4](#s-4) | Update the documents | 3 | Done |

<a id="s-1"></a>
### Step 1. Add the Source Job Table with Its Placing Rules

**Locations to change**
`daemon/modules/engine/src/core/thumbnail/source_job.rs` (new, the table of waiting and decoding jobs per source), [thumbnail.rs#L1](../../daemon/modules/engine/src/core/thumbnail.rs#L1)

**What to do**
Hold, per source, one waiting job with its plans, the highest `WorkClass` of its requests, and the count of their handles, and one decoding job with its fixed plan keys. Implement the four placing rules, the move of a waiting job to Interactive when an Interactive request joins, the start of the next job when the decoding job ends, and the discard of a waiting job whose handles are all dropped. The table does not decode; it is driven by the generator through a closure, so it can be tested with plain values.

**Completion condition**
Unit tests of the table cover each placing rule, the move to Interactive, the next job, and the discard, with no image involved.

<a id="s-2"></a>
### Step 2. Route Generation Through Source Jobs and Remove the Thumbnail Flights

**Locations to change**
[generator.rs#L108](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L108), [generator.rs#L276](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L276), [generator.rs#L291](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L291), [generator.rs#L481](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L481)

**What to do**
After the primary cache miss, place the request in the source job table instead of the flights. When a job gets its slot, compute the companions of each of its plans, without duplicates and without the plans themselves, decode once, convert every plan and companion, release the slot, store the batch, and answer each request with its plan's image. Keep the video path's shared run of thumbnail and sprite sheet as two plans of one job. Keep the direct path for requests without a `SourceId`. Record the time a request waits for a job of its source as its own performance span, `thumbnail_job_wait`, apart from `thumbnail_permit_wait`, and describe it in the span table of the measurement protocol.

**Completion condition**
Completion conditions 6, 7, and 8 hold, and the generator no longer has the `flights` field.

<a id="s-3"></a>
### Step 3. Replace the Decode-Twice Test with Tests of the Conditions

**Locations to change**
[generator.rs#L2183](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L2183), [generator.rs#L960](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L960)

**What to do**
Replace `concurrent_requests_for_different_sizes_may_decode_twice_test` with tests of completion conditions 1 to 5. Hold the slot or block the first conversion until the later request is known to be placed, so that the result does not depend on timing. Count conversions with `conversions()` and the concurrent conversions of one source with a test counter.

**Completion condition**
The tests of conditions 1, 2, and 3 fail on the build before step 2 and pass after it, and the others pass.

<a id="s-4"></a>
### Step 4. Update the Documents

**Locations to change**
[thumbnail.md](../design/thumbnail.md), [plans.md](../plans.md)

**What to do**
Remove the sentence about the missing source jobs from Thumbnail Design §11.1 and the row T2 from §11.2, remove T2 from the dependencies of IC1 in [Image Channel Design §10.2](../design/image-channel.md#102-roadmap) and from the breakdown of M23 in design.md §12.2, and set the state of this plan to done in `plans.md`. The plan and its row are deleted when the plan is folded, after the review of the change.

**Completion condition**
`rg -n 'T2' docs/design.md docs/design` finds nothing, and Thumbnail Design §11.1 states no gap about source jobs.

## 6. Acceptance Verification

| Condition | Command | Expected result |
| --- | --- | --- |
| 1, 2, 3, 4, 5 | `cd daemon && RUSTC_WRAPPER= cargo test -p omnius-pxna-engine --lib core::thumbnail` | The tests of step 3 and the table's tests pass |
| 6, 7 | `cd daemon && PXNA_REQUIRE_FFMPEG=1 RUSTC_WRAPPER= cargo test -p omnius-pxna-engine --lib core::thumbnail::generator` with ffmpeg 9 on `PATH` | The existing tests of uncached local files and of `concurrent_thumbnail_and_sprite_share_one_ffmpeg_run_test` pass |
| 8 | `cd daemon && RUSTC_WRAPPER= cargo test -p omnius-pxna-daemon thumbnails` | Passes |
| 9 | `cd daemon && RUSTC_WRAPPER= cargo make lint && RUSTC_WRAPPER= cargo make test` | Both pass |

The tests of step 3 protect the following: one decode for two waiting sizes, no decode for a companion of a decoding job, a later job instead of a parallel decode, the move to Interactive, and the discard of a job nobody waits for.
