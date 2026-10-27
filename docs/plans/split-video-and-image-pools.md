# Split the Video and Image External Process Pools

## 1. About This Plan

`FfmpegImage`, `HeifImage`, and `Video` shared the `processes` pool. A 22 s frame extraction of an action-camera video took a slot that 10 to 16 s HEIC decodes needed, and `thumbnail_permit_wait` reached 38 s. This work gives videos their own pool.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The steps and completion conditions |
| [Thumbnail Design §7, §9.1, §10.1](../design/thumbnail.md#7-order-and-concurrency-of-generation) | The pools, the settings, and the decision |

### 1.2 Creation Date and Target Commit

Created 2026-10-26 / Target commit `d4030c6` with the uncommitted work

## 2. Purpose and Completion Conditions

1. `ScheduleOptions` has `video_slots` and `video_interactive_slots`, defaulting to 1 and 1; `process_slots` (2) and `process_interactive_slots` (1) are for images; `video_slots` must be positive
2. `MediaPools::videos` is a separate pool; a held video permit does not take a slot of `processes` or `images`
3. `Video` generation and `VideoService` use `videos`; `FfmpegImage` and `HeifImage` use `processes`; `Image` uses `images`
4. `[scheduler]` accepts the two new keys, and the example configurations list them
5. Lint, the engine tests, and the daemon tests pass

## 3. Steps

| No. | Content | State |
| --- | --- | --- |
| 1 | Add the options and the pool in `schedule.rs` | Done |
| 2 | Route `pool_for` and `VideoService` | Done |
| 3 | Parse the keys in `config.rs`, update the examples | Done |
| 4 | Add tests, update the design documents | Done |

Unverified: the effect on `thumbnail_permit_wait` is not measured, and the ffmpeg paths skip without ffmpeg.
