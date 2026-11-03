# Accept Negative ffprobe Levels and Classify Probe Parse Failures

## 1. About This Plan

This work makes VP8 and VP9 WebM files usable through media info, video sessions, and thumbnails, and reports a probe output that the daemon cannot parse as a server failure instead of an unsupported file.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The steps, completion conditions, and scope of this work |
| [design.md §4.4](../design.md#44-error-model-and-http-mapping) | The HTTP status for a probe output that cannot be parsed |
| [Video Playback Design §7.1](../design/video.md#treat-a-negative-ffprobe-level-as-absent) | The decision to treat a negative probe level as absent |
| [knowledge.md](../knowledge.md), [ffprobe stream levels](../knowledge/ffprobe-stream-level.md) | The measured level values of the checked codecs |
| [issues.md](../issues.md), [VP8 and VP9 videos are rejected as unsupported](../issues/vp8-vp9-videos-are-rejected-as-unsupported.md) | The defect this work fixes |
| [daemon/openapi.yaml](../../daemon/openapi.yaml) | The error responses of the media info and video session endpoints; unchanged by this work |

[design.md §1.1](../design.md#11-division-of-responsibility-between-documents) holds the map of the document family.

### 1.2 Creation Date and Target Commit

Created 2026-11-04 / Target commit `74a485b`

## 2. Purpose and Completion Conditions

A VP8 or VP9 WebM, the usual content of a WebM, cannot show media info, cannot start a video session, and cannot generate a thumbnail, although ffprobe reads it. A client also cannot tell this from a file that ffprobe cannot read, because both answer `400 UnsupportedType`. This work removes the first failure and separates the second.

### 2.1 Completion Conditions

1. `GET /api/v1/libraries/{libraryId}/media-info` for a VP9 WebM generated with ffmpeg 9.0.2 returns 200 with `kind` video, `videoCodec` vp9, `contentType` beginning `video/webm`, and `durationSecs` 2.0
2. The same request for a VP8 WebM returns 200 with `videoCodec` vp8
3. `POST /api/v1/libraries/{libraryId}/video/sessions` for the VP9 WebM returns 201, and its playlist and segment 0 return 200
4. A thumbnail generation for the VP9 WebM succeeds and returns an image with the requested dimensions
5. `VideoInfo::parse` accepts a probe output with `"level": -99` and returns the VP9 codec and the duration, and an H.264 stream with a negative level keeps the codec string with level 31
6. `VideoInfo::parse` and `ImageInfo::parse_probe_size` return `UnexpectedError` for a probe output that is not valid JSON, and `ApiError` maps `UnexpectedError` to 500 `InternalServerError`
7. `docs/design.md §4.4` and `docs/design/video.md §7.1` state the two decisions, and `docs/knowledge.md` lists the new ffprobe stream level item
8. `cd daemon && RUSTC_WRAPPER= cargo make lint` passes, and the unit and end-to-end tests in §6 pass with ffmpeg 9.0.2 on `PATH`
9. When the work is wrapped up, the issue child and its row are deleted, the plan child and its row are deleted, and the summary sentence in `docs/issues.md` no longer names the VP8 and VP9 item

### 2.2 Non-Goals

- Preserving engine error messages through the daemon's error conversion for 4xx responses. The 500 path logs the source chain; the other 4xx messages stay as they are, and this is recorded in [the engine error message item](../issues/engine-error-messages-are-dropped.md).
- Enumerating every codec, container, or ffmpeg version for which ffprobe reports a negative level.
- Changing the iOS or web client. The server returns media info, and the client chooses between direct and transcoded playback.
- Changing `openapi.yaml` or adding an API `ErrorKind`.
- Changing the H.264 fallback level from 31.
- Verifying playback in a browser or on a phone. The end-to-end tests stop at the HTTP and thumbnail boundaries.

## 3. Current State

[info.rs#L55](../../daemon/modules/engine/src/core/media/info.rs#L55) declares `ProbeStream::level` as `Option<u32>`.
[info.rs#L64](../../daemon/modules/engine/src/core/media/info.rs#L64) turns any deserialization failure into `UnsupportedType`.
[info.rs#L93](../../daemon/modules/engine/src/core/media/info.rs#L93) is the only use of `level`, in the H.264 codec string, and it already falls back to 31 when the value is absent.
[image.rs#L256](../../daemon/modules/engine/src/core/media/image.rs#L256) turns a deserialization failure into `UnsupportedType` in the same way.
[ffmpeg.rs#L17](../../daemon/modules/engine/src/core/media/ffmpeg.rs#L17) runs ffprobe and calls `VideoInfo::parse`.
The media info handler at [video.rs#L72](../../daemon/entrypoints/daemon/src/interface/features/video.rs#L72), the video session creation at [service/video.rs#L259](../../daemon/entrypoints/daemon/src/service/video.rs#L259), and the thumbnail generator at [generator.rs#L882](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L882) all obtain the media information through that one probe.
[error.rs#L303](../../daemon/entrypoints/daemon/src/error.rs#L303) maps `UnsupportedType` and `InvalidFormat` to 400 `UnsupportedType`, and [error.rs#L313](../../daemon/entrypoints/daemon/src/error.rs#L313) maps every other kind to 500 `InternalServerError`.
[error.rs#L253](../../daemon/entrypoints/daemon/src/error.rs#L253) drops the engine error's message when it converts the error for the API.

ffprobe 9.0.2 reports -99 for the level of VP8 and VP9 streams, and non-negative levels for the other checked codecs ([ffprobe stream levels](../knowledge/ffprobe-stream-level.md)).
A VP8 or VP9 file therefore fails at the first deserialization, and all three paths answer 400 `UnsupportedType`.
A probe output that the daemon cannot parse is also reported as a fact about the file, although ffprobe succeeded.

## 4. Change Policy

The engine reads a level as a signed integer and treats a negative value as absent, keeping the existing H.264 fallback of 31. It changes the two deserialization failures to `UnexpectedError`, which the API maps to 500. Tests cover the probe parse, the three HTTP and thumbnail paths, and the API mapping.

### 4.1 Decisions Assumed

| Decision | Source |
| --- | --- |
| A negative probe level means that the stream has no level, and the H.264 fallback of 31 stays | [Video Playback Design §7.1](../design/video.md#treat-a-negative-ffprobe-level-as-absent) |
| A successful ffprobe whose output cannot be deserialized is `InternalServerError`, not a fact about the file | [design.md §4.4](../design.md#44-error-model-and-http-mapping) |
| The end-to-end tests generate VP8 and VP9 files with ffmpeg at runtime and skip only when ffmpeg or the encoder is missing | This plan |
| The issue and the plan are deleted when the work is complete | [issues.md §1.1](../issues.md#11-division-of-responsibility-between-documents), [plans.md §1.1](../plans.md#11-division-of-responsibility-between-documents) |

### 4.2 Impact

The engine's video and image probe parsing, the daemon's video feature tests, the thumbnail generator tests, and the error mapping test change.
`GET /media-info` and `POST /video/sessions` answer 200 for VP8 and VP9 WebM files that previously answered 400, and a probe output that cannot be parsed answers 500 instead of 400 `UnsupportedType`.
`openapi.yaml` does not change, because both endpoints already declare 400 and 500.
No persisted data or configuration changes, and a file that ffprobe can read keeps working as before.

## 5. Steps

| No. | Content | Depends on | State |
| --- | --- | --- | --- |
| [1](#s-1) | Record the decisions and the external fact | - | Not started |
| [2](#s-2) | Add unit tests that reproduce the failure | 1 | Not started |
| [3](#s-3) | Fix the level parsing and the parse error kinds | 2 | Not started |
| [4](#s-4) | Add end-to-end tests with real ffmpeg | 3 | Not started |
| [5](#s-5) | Cover the API mapping for `UnexpectedError` | 3 | Not started |
| [6](#s-6) | Run lint and the test suite | 4, 5 | Not started |
| [7](#s-7) | Wrap up the issue and plan documents | 6 | Not started |

<a id="s-1"></a>
### Step 1. Record the decisions and the external fact

**Change Locations**
[design.md §4.4](../design.md#44-error-model-and-http-mapping), [Video Playback Design §7.1](../design/video.md#71-decided), `docs/knowledge/ffprobe-stream-level.md`, [knowledge.md](../knowledge.md), `docs/plans/media-probe-level-and-errors.md`, [plans.md](../plans.md)

**What to Do**
Add the negative-level decision to Video Playback Design §7.1, the 500 rule for an unparseable probe output to design.md §4.4, the measured ffprobe values to a new knowledge child and the knowledge list, and this plan to the plan list.

**Completion Condition**
Every relative link in the changed documents resolves, `docs/knowledge.md` lists the new knowledge child, and `docs/plans.md` lists this plan with a predecessor of `-`.

<a id="s-2"></a>
### Step 2. Add unit tests that reproduce the failure

**Change Locations**
[info.rs](../../daemon/modules/engine/src/core/media/info.rs#L186), [image.rs](../../daemon/modules/engine/src/core/media/image.rs#L459)

**What to Do**
Add a test that parses a VP9 probe output with `"level": -99` and asserts the codec and duration, a test that an H.264 output with a negative level keeps the `avc1` string ending in `1F`, and tests that invalid JSON returns `UnexpectedError` from both parse functions.

**Completion Condition**
The new level test fails on the current code because `VideoInfo::parse` returns `UnsupportedType`, and the invalid-JSON tests fail because both functions return `UnsupportedType` instead of `UnexpectedError`.

<a id="s-3"></a>
### Step 3. Fix the level parsing and the parse error kinds

**Change Locations**
[info.rs#L55](../../daemon/modules/engine/src/core/media/info.rs#L55), [info.rs#L64](../../daemon/modules/engine/src/core/media/info.rs#L64), [info.rs#L93](../../daemon/modules/engine/src/core/media/info.rs#L93), [image.rs#L256](../../daemon/modules/engine/src/core/media/image.rs#L256)

**What to Do**
Declare the level as a signed integer, convert it to `u32` only when it is non-negative, and keep the existing fallback of 31. Change the two `serde_json` failures from `UnsupportedType` to `UnexpectedError` and keep the messages for the log.

**Completion Condition**
The tests from Step 2 pass, and the existing media info and image probe tests pass unchanged.

<a id="s-4"></a>
### Step 4. Add end-to-end tests with real ffmpeg

**Change Locations**
[interface/features/video.rs](../../daemon/entrypoints/daemon/src/interface/features/video.rs#L250), [core/thumbnail/generator.rs](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L1610)

**What to Do**
Generate a VP8 and a VP9 WebM at runtime with `libvpx` and `libvpx-vp9`, and check media info, a VP9 video session with its playlist and segment 0, and a VP9 thumbnail. Follow the existing `skip_without_ffmpeg` pattern and skip when the named encoder is absent, while `PXNA_REQUIRE_FFMPEG=1` keeps ffmpeg mandatory in CI.

**Completion Condition**
The new tests pass with ffmpeg 9.0.2 on `PATH`, and the same tests skip without ffmpeg or without the encoder.

<a id="s-5"></a>
### Step 5. Cover the API mapping for `UnexpectedError`

**Change Locations**
[daemon/entrypoints/daemon/src/error.rs](../../daemon/entrypoints/daemon/src/error.rs#L386)

**What to Do**
Add `UnexpectedError` to the existing mapping cases with 500 `InternalServerError`.

**Completion Condition**
The mapping test passes and includes the new case.

<a id="s-6"></a>
### Step 6. Run lint and the test suite

**Change Locations**
None.

**What to Do**
Run the commands in §6: lint, the targeted unit and end-to-end tests with ffmpeg 9.0.2 on `PATH`, and the full daemon test suite.

**Completion Condition**
Every command in §6 returns the expected result.

<a id="s-7"></a>
### Step 7. Wrap up the issue and plan documents

**Change Locations**
[docs/issues.md](../issues.md), `docs/issues/vp8-vp9-videos-are-rejected-as-unsupported.md`, [docs/plans.md](../plans.md), `docs/plans/media-probe-level-and-errors.md`

**What to Do**
Delete the issue child and its row, update the sentence in `docs/issues.md` that names the VP8 and VP9 item, and delete the plan child and its row after §6 passes.

**Completion Condition**
`rg -n 'vp8-vp9-videos-are-rejected-as-unsupported|media-probe-level-and-errors' docs` prints nothing, and both child files are gone.

## 6. Acceptance Verification

Run the commands from the repository root unless the command changes directory. For the end-to-end tests, put ffmpeg 9.0.2 and ffprobe on `PATH` first; the checked build is the one the Dockerfile pins, and the local `pxna-probe` image has the same binaries.

| Condition | Command | Expected Result |
| --- | --- | --- |
| 1, 2 | `cd daemon && PXNA_REQUIRE_FFMPEG=1 RUSTC_WRAPPER= cargo nextest run --all-features -p omnius-pxna-daemon interface::features::video` | pass, including the VP8 and VP9 media info cases |
| 3 | `cd daemon && PXNA_REQUIRE_FFMPEG=1 RUSTC_WRAPPER= cargo nextest run --all-features -p omnius-pxna-daemon interface::features::video` | pass, including the VP9 session, playlist, and segment cases |
| 4 | `cd daemon && PXNA_REQUIRE_FFMPEG=1 RUSTC_WRAPPER= cargo nextest run --all-features -p omnius-pxna-engine core::thumbnail::generator` | pass, including the VP9 thumbnail case |
| 5 | `cd daemon && RUSTC_WRAPPER= cargo nextest run --all-features -p omnius-pxna-engine core::media::info` | pass, including the negative-level and H.264 fallback tests |
| 6 | `cd daemon && RUSTC_WRAPPER= cargo nextest run --all-features -p omnius-pxna-engine core::media::image` and `cd daemon && RUSTC_WRAPPER= cargo nextest run --all-features -p omnius-pxna-daemon error::tests` | pass, including `UnexpectedError` from invalid JSON and its 500 mapping |
| 7 | `rg -n 'A successful ffprobe whose output the daemon cannot deserialize' docs/design.md`, `rg -n 'Treat a Negative ffprobe Level as Absent' docs/design/video.md`, `rg -n 'ffprobe-stream-level' docs/knowledge.md docs/design/video.md`, and `test -f docs/knowledge/ffprobe-stream-level.md` | every command succeeds and the new decision, rule, and knowledge item are present |
| 8 | `cd daemon && RUSTC_WRAPPER= cargo make lint` and `cd daemon && PXNA_REQUIRE_FFMPEG=1 RUSTC_WRAPPER= cargo nextest run --all-features -j 4` | both commands succeed |
| 9 | `test ! -e docs/issues/vp8-vp9-videos-are-rejected-as-unsupported.md`, `test ! -e docs/plans/media-probe-level-and-errors.md`, `rg -nF 'the representative-thumbnail item and the engine-message item are unrelated to them and to each other' docs/issues.md`, `! rg -n 'VP8 and VP9 item' docs/issues.md`, and `! rg -n 'vp8-vp9-videos-are-rejected-as-unsupported|media-probe-level-and-errors' docs` | every command succeeds; the `rg` for the remaining sentence prints one match and the other `rg` commands print nothing |
