# Video Playback Design

## 1. About This Document

This document covers the mechanism for playing videos in the viewer, and defines responsibility boundaries, invariants, and the reasons for adoption for obtaining media info for images and videos, direct playback, and HLS transcoded delivery.
The intended readers are developers and AI agents who implement or review this concern.
It assumes the vocabulary of [terms.md](../terms.md).
The definitions of words are not in this document.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | Media info for images and videos, the premises of direct playback, transcode sessions, transcoding output, reasons for adoption, deferred items |
| [design.md](../design.md) | Pxna's overall scope, overall structure, boundaries between concerns, and cross-cutting design decisions including returning file content responses as streams |
| [Thumbnail Design](./thumbnail.md) | The frame sequence of videos shown on listing tiles |
| [Client Design](./client.md) | Choosing between direct and transcoded playback, and handling of the player |
| [Storage Layer Design](./storage.md) | How to turn a video in an archive into a physical file |
| [Timeline Design](./timeline.md) | Deciding the timeline date from a video's `creation_time` |
| [terms.md](../terms.md) | Definitions of terms |
| [issues.md](../issues.md) | Clear defects confirmed in code and handling policies |
| [daemon/openapi.yaml](../../daemon/openapi.yaml) | The authoritative API paths, schemas, and status codes |

The entry point of the design document family is [design.md](../design.md).

### 1.2 Tense of This Document

§1 through §7 describe the completed form in the present tense.
The gap between the current code and the completed form, and the remaining work in dependency order, are gathered in §8.
If you want to know what works now, read §8 first.

## 2. Responsibilities and Boundaries

This document covers the information and delivery needed until the client starts playback of a video opened in the viewer.
The server returns the video's media info, and the client compares it with its own playback capability and chooses between direct playback and transcoded playback.
In direct playback the original file is sent as is, and in transcoded playback the server sends while converting to HLS with ffmpeg.

The Library also contains videos in formats that browsers cannot play as they are.
If the server can convert them, the videos in the Library can be watched in the viewer regardless of the client's playback capability.
In exchange, conversion is processing that keeps using the CPU, so the number running at the same time is limited (§5.4).

Media info is also returned for images.
This is so that the web client's info panel can show the dimensions and shooting info of the selected image ([Client Design §3.8](./client.md#38-info-panel)).
Images and videos use the same API and cache rules, so this document covers both.

The boundaries with other concerns are as follows.

| Matter | Authoritative source |
| --- | --- |
| Returning file content responses as streams from a route outside the generated router | [design.md §11.1](../design.md#return-file-content-responses-as-streams) |
| The frame sequence shown on listing tiles | [Thumbnail Design §6](./thumbnail.md#6-video-frame-sequence) |
| The rule for choosing between direct and transcoded playback | [Client Design §3.4](./client.md#34-viewer) |
| How to turn an entry in an archive into a physical file, and its limits | [Storage Layer Design §5.2](./storage.md#52-default-implementations-centered-on-materialize) |
| Calling ffmpeg as an external process | [Thumbnail Design §10.1](./thumbnail.md#call-ffmpeg-as-an-external-process) |

### 2.1 What Is Not Built

- Delivery that prepares multiple outputs of differing quality and switches according to the connection (adaptive bitrate) is not provided.
  By limiting output to one kind, only one ffmpeg needs to run per playback.
  Under the operating premise of a LAN or a trusted network ([design.md §2](../design.md#2-what-pxna-is)), the need to follow fluctuations in the connection is small.
- Conversion results are not kept beyond playback.
  This avoids having a cache of converted videos and managing its capacity.
- Selecting audio tracks and subtitles is not provided.
  Only the default audio track is converted, and subtitles are not converted.
  This avoids having a UI for listing and selecting tracks.

## 3. Media Info

The server returns, as media info, the container format, the video and audio codecs, the duration, and the resolution examined with ffprobe for videos, and the format, dimensions, and shooting info for images.
The response is returned with a `kind` indicating image or video, and exactly one of `video` or `image` corresponding to it (§7.1).
Whether something is an image or a video is decided from the extension, using the same table as the determination of thumbnail targets ([Thumbnail Design §3](./thumbnail.md#3-determining-the-target)).
An Entry in an archive is made a physical file with `materialize_file` and then read.

For the dimensions of an image, whether the orientation is applied differs by format.
For formats read with the `image` crate, they are the width and height after applying the EXIF orientation, obtained from the file header.
For formats decided to be read with ffmpeg (HEIC, AVIF, and so on), they are the width and height as encoded that ffprobe returns, and are not swapped.
For an image encoded as a tile grid, they are the width and height of the assembled image in the Tile Grid stream group, not those of one tile ([ffmpeg and HEIF tile grids](../knowledge/ffmpeg-heif-tile-grid.md)).
An ffprobe that does not know the option that outputs stream groups fails (5.1 and 6.1 among the versions checked), so the server runs it again without the option and uses the first video stream.

For the rotation of these formats, the rotation ffprobe returns (the `rotation` of the Display Matrix in `side_data_list` of the Tile Grid or of the stream) is returned separately as `rotation`.
The value is the counterclockwise angle by which the stored image is rotated for display, and the server internally normalizes it to one of 0, 90, 180, or 270 degrees. A value that is not a multiple of 90 is treated as 0.
In the response, `rotation` is omitted when it is 0.
Formats read with the `image` crate already have the orientation reflected in the width and height, so `rotation` is not returned (§7.1).

Shooting info is read from Exif, and returns the shooting date and time, the camera maker and model, the lens, the focal length, the aperture, the exposure time, and the ISO sensitivity.
kamadak-exif is used to read Exif, and location information is not returned (§7.1).
Images without Exif, and images of formats from which kamadak-exif does not read Exif (GIF, BMP, and so on), are returned with empty shooting info.

Media info is held in an in-process LRU.
As with the thumbnail cache, the key is built from the path, size, and modification time for local files, and from the `EntryCacheKey` for entries in archives ([Thumbnail Design §8.3](./thumbnail.md#83-keys)).
Frame sequence generation also obtains the video's duration from this information to decide the positions to extract.

A video container's `creation_time` is not included in the media info, and only timeline aggregation reads it for the timeline date ([Timeline Design §4](./timeline.md#4-timeline-dates)).

A video ffprobe cannot read and an image whose dimensions cannot be read are `UnsupportedType`.
The LRU holds 256 items, merges concurrent requests for the same key, and limits concurrent ffprobe runs to 2.

## 4. Direct Playback

In direct playback, the client fetches the original file from the content API with `Range`.
The content API returns the requested range as a stream without accumulating it in memory ([design.md §11.1](../design.md#return-file-content-responses-as-streams)).
The player re-requests with a new `Range` at every seek, so the server holds neither conversion nor state.

A video in an archive is read from the materialized physical file.
The upper limit on the size that can be materialized is decided by `max_materialized_file_size`, and a video that exceeds it gives `LimitExceeded` ([Storage Layer Design §7.2](./storage.md#72-defaults)).

## 5. Transcoded Playback

### 5.1 Transcode Sessions

Transcoded playback starts when the client specifies a video's path and creates a transcode session.
A transcode session holds together the original video's physical file and its lease, the ffmpeg process, and the directory where the output segments are placed.
The client gives the session's playlist URL to the player, and the player fetches the playlist and segments by the HLS procedure.

A session is limited to 100,000 segments, and an empty video or one beyond that is `LimitExceeded`.
This is to bound the size of the playlist and the state held per request.

The playlist lists, at the time the session is created, all the segments obtained by dividing the video's duration by `segment_secs`.
Segments not yet output are also listed from the start so that the player can show the video's full length and the seek bar from the beginning.

A session ends with any of the following.

- The client requests termination when closing the viewer
- Neither the playlist nor segments are requested for `session_idle_timeout_secs`
- A new session is created while the concurrency limit has been reached (§5.4)

The segment directory is created per session under `state_dir/video/`.
When a session ends, ffmpeg is killed.
After segments being delivered and pending I/O are gone and the file handles are closed, the directory is removed.
If deletion fails, it is retried for 5 seconds, and if it still fails, it is left in the log.
At daemon startup, all the segment directories left from the previous run are removed.

### 5.2 Generating Segments and Seeking

**Segment boundaries must come at the same times regardless of where conversion is started.**
ffmpeg forces a keyframe at every time that is the segment number times `segment_secs` and outputs.
Therefore, even if ffmpeg is restarted from a time midway, the output segments have the same divisions as if converted from the start, and do not disagree with the playlist's enumeration.
However, a keyframe can be placed only at a time where a frame exists.
For a video without a frame exactly at a boundary time, the boundary shifts to the next frame, and each segment's actual length differs slightly from the length written in the playlist.

For a segment request, the server responds as follows.

| Requested segment | Behavior |
| --- | --- |
| Already output | Return it as is |
| Ahead of the position being converted, within `restart_threshold_segments` | Wait for it to be output and return it |
| Further ahead, or not yet output and before the position being converted | Kill ffmpeg and restart from that segment's start time |

A segment not yet output is waited for up to 60 seconds, and when the deadline passes `Timeout` is returned.
If a request that arrived at the same time is replaced by a different seek, it is `Conflict`, and conversion is not moved back to the earlier position.

With this method, playback starts from the position sought to and does not wait for the conversion of the whole video.
The cost is that audio and video synchronization may be slightly off in the segment right after a restart.
This is because, at the position of restart, the audio frame boundary and the video keyframe do not necessarily coincide.

### 5.3 Output

The output is one kind: H.264 video and AAC audio.
The video preserves the aspect ratio and is shrunk only when its height exceeds `max_height`.
It is never made larger than the original.

The default encoder is the software `libx264`.
The encoder's name and arguments can be replaced by configuration, and whether to use a hardware encoder is up to the operator.
Using a hardware encoder in Docker also needs operator-side configuration to pass the device to the container.

### 5.4 Concurrency

The number of transcode sessions that can exist at the same time is limited by `max_concurrent_transcodes`.
When a new session is created while the limit has been reached, the oldest session is ended to make way.

We end the old one rather than rejecting the new session because the user is a single owner.
Opening a new video can be taken to mean the previous video was finished or abandoned.
However, if two devices transcode-play different videos at the same time, with the limit at 1 they would end each other's sessions.

## 6. Configuration

The transcoding configuration goes in the top-level `[video]`.
The locations of the ffmpeg and ffprobe executables are in `[media]`, shared with thumbnails ([Thumbnail Design §9](./thumbnail.md#9-configuration)).

| key | Default | Applies to |
| --- | ---: | --- |
| `max_concurrent_transcodes` | 1 | Number of transcode sessions that can exist at the same time |
| `segment_secs` | 6 | Length of one segment |
| `restart_threshold_segments` | 3 | The distance from the position being converted that separates waiting from restarting |
| `max_height` | 1080 | Upper limit on the height of the converted video |
| `encoder` | `libx264` | Video encoder passed to ffmpeg |
| `encoder_args` | `["-preset", "veryfast", "-crf", "23"]` | Arguments passed to the encoder |
| `session_idle_timeout_secs` | 300 | Time until ending a session whose requests have stopped |

`session_idle_timeout_secs` is set to a few minutes so that the session is kept while paused.
While paused, the player issues no requests, so if it were short, the session would be gone when resuming.

## 7. Design Decisions

### 7.1 Decided

#### The Server Examines and the Client Chooses the Playback Method

**Decision**
The server only returns media info, and the client compares it with its own playback capability and chooses between direct playback and transcoded playback.

**Reason**
The codecs that can be played differ with the browser and OS version even for the same format.
Only the client has the information needed for that judgment.
If chosen beforehand, there is neither waiting for a failure on a video that cannot be played, nor needlessly converting a video that can be played.

**Rejected alternative 1**
A method that first tries direct playback and switches to transcoded playback on failure.
No API would be added, but one would be made to wait until the failure is detected, and partial failures, such as audio playing but no video, are hard to detect, so we rejected it.

**Rejected alternative 2**
A method that always converts.
No judgment is needed, but it uses the CPU even for videos that can be played, and image quality also drops, so we rejected it.

#### Return Image Media Info from the Same API and Exclude Location Information

**Decision**
The media info API also answers for images, and returns the format, dimensions, and shooting info (§3).
Shooting info is read from Exif with kamadak-exif, and location information is not included.

**Reason**
The info panel can request the same API for both images and videos, and the cache key and the merging of concurrent requests can be shared with videos.
kamadak-exif can extract Exif in the same way from JPEG, PNG, WebP, TIFF, and from HEIC and AVIF read with ffmpeg ([Formats supported by kamadak-exif](../knowledge/kamadak-exif-containers.md) in [knowledge.md](../knowledge.md)).
We exclude location information because there is no screen that shows a map, and showing coordinates does not help with browsing.

**Rejected alternative 1**
A method that puts dimensions and shooting info in the listing response.
The info panel could be shown from the listing alone, but each file would have to be opened and its Exif read at every listing, so we rejected it.

**Rejected alternative 2**
A method that adds a separate API returning an image's shooting info.
It avoids splitting the shape from the video response, but the info panel would change which API it calls by Entry kind, and there would be two sets of cache rules, so we rejected it.

#### Return the Rotation of Images Read with ffmpeg as an Attribute Without Swapping the Dimensions

**Decision**
For images of formats decided to be read with ffmpeg, the width and height are returned as encoded, and the rotation is returned separately as `rotation` (§3).
When `rotation` is 90 or 270 degrees, the client swaps the width and height for display.

**Reason**
ffprobe of ffmpeg 8.1.1 returns AVIF's `irot` as the `rotation` of the Display Matrix in `side_data_list`, separately from the width and height, and the width and height stay as encoded.
ffmpeg 8.1.1's decoding applies the rotation, and ffmpeg 5.1 to 7.1 neither report nor apply it ([Measurements of ffprobe](../knowledge/ffprobe-heif-rotation.md)).
For a HEIC tile grid, ffprobe of ffmpeg 9.0.2 returns `irot` on the Tile Grid, and ffprobe of 8.1.3 does not ([ffmpeg and HEIF tile grids](../knowledge/ffmpeg-heif-tile-grid.md)).
The Docker image bundles ffmpeg 9.0 ([design.md](../design.md#bundle-a-pinned-static-build-of-ffmpeg)), where the rotation ffprobe returns and the rotation that decoding by ffmpeg applies to thumbnails and display images agree for both AVIF and HEIC (heif-dec applies the same rotation for images with alpha).
Dimensions and rotation are separate facts, so if they are returned as is, the server does not need a rule for interpreting an image's orientation, and the client can handle it to suit the display.

**Rejected alternative 1**
A method in which the server applies the rotation, swaps the dimensions, and returns them.
The client would not need to do anything, but the stored dimensions and whether there is a rotation would become invisible to the client, so we rejected it.

**Rejected alternative 2**
A method in which the client indicates to the user that these are the dimensions as encoded.
The server would not change, but the client would hold a list of formats that do not apply the rotation, which violates the rule that the set of supported formats is a property of the server ([Thumbnail Design §3](./thumbnail.md#3-determining-the-target)), so we rejected it.

**Rejected alternative 3**
A method that does not return the dimensions of images read with ffmpeg.
It avoids showing wrong values, but the dimensions of HEIC and AVIF would not be visible in the info panel, so we rejected it.

#### Return the Media Info Response with `kind` and One of Image or Video

**Decision**
The media info response has a `kind` indicating image or video, and exactly one of `video` or `image` corresponding to `kind`.
The video items (container, codec, duration, frame rate, and the MIME type used for playback) and the image items (format, dimensions, rotation, shooting info) are placed under each.

**Reason**
The required items for images and videos do not overlap.
If the video items were reused for images while remaining required, the image response would be filled with meaningless values.
With a form that makes `kind` the discriminator and carries only the corresponding side, the client can branch on `kind` and then read the items.
We do not use `oneOf` because `openapi.yaml` does not use it at present, and we have not confirmed whether the generated server types would come out as intended.

**Rejected alternative 1**
Listing the video type and the image type with `oneOf` using `kind` as the discriminator.
It is straightforward as a contract, but we have not confirmed the generated types and there is a risk of rework, so we rejected it.

**Rejected alternative 2**
A method that leaves the video items in their current position and adds `image`.
It would not break video clients, but the video items would lose meaning for images while remaining required, which goes against the policy of splitting the shape of the response, so we rejected it.

#### Deliver Transcoding with HLS and Restart ffmpeg on Seeks

**Decision**
Transcoded playback is delivered with HLS, and the playlist lists all segments in advance.
When a seek goes to a position away from the one being converted, ffmpeg is restarted from that position (§5.2).

**Reason**
The web client can play it using hls.js.
The player's standard seek bar can be used as is, and playback starts a few seconds after the sought position.

**Rejected alternative 1**
A method that streams a single fragmented MP4 while converting.
The implementation is simple, but the player's standard seek bar cannot be used, and a proprietary UI that re-requests with a start time would be needed, so we rejected it.

**Rejected alternative 2**
A method that finishes converting the whole thing and then delivers it by the same path as direct playback.
It is the simplest, but for long videos there would be a wait of several minutes before playback starts, so we rejected it.

#### Make the Default Encoder Software

**Decision**
The default encoder is `libx264`, and its name and arguments can be replaced by configuration.

**Reason**
Software encoders work the same in any environment.
Whether a hardware encoder is usable changes with the server's model, drivers, and how the device is passed to the container.

**Rejected alternative**
A method that looks for a usable hardware encoder at startup and uses it automatically.
No configuration would be needed, but investigating the cause when the detection is wrong becomes difficult, so we rejected it.

### 7.2 Deferred

#### Number of Devices That Play Transcoded Video at the Same Time

**Current state**
The default of `max_concurrent_transcodes` is 1, and when the limit is exceeded, the oldest session is ended (§5.4).

**Why not decide now**
How many streams a software encoder can convert at once cannot be known without measuring on the server's CPU.

**Conditions for deciding**
When a usage in which two or more devices transcode-play at the same time actually arises.

## 8. Current State and Remaining Work

### 8.1 Current Code

The content API's HTTP response is returned as a stream by `FileContentRoute`, which holds the file and lease, and carries the ETag and the requested range.
Media info is returned from the cache shared with thumbnail generation, and transcode sessions and HLS delivery are also connected to the API.
Seeking far ahead and backward, the concurrency limit, expiry, the body's lease, and resource reclamation are confirmed with real ffmpeg and HTTP tests.
The media info response is split into images and videos by `kind`.
For images, it returns the MIME type decided from the extension, the dimensions with the EXIF orientation applied, and the shooting info read by kamadak-exif, and does not return location information.
For formats read with ffmpeg such as HEIC, the dimensions are obtained from ffprobe as the width and height as encoded, preferring the Tile Grid of a tiled image, and the rotation is returned separately as `rotation` (§7.1).
Images and videos share the same LRU and the upper limit on concurrent ffprobe runs.

### 8.2 Roadmap

The order is provisional.
Only the dependencies must be respected.

| No. | Content | Dependencies assumed | State |
| --- | --- | --- | --- |
| V1 | Move the content API to a route that returns streams, also return `ETag`, and enable direct playback | None | Done |
| V2 | Add an API that returns media info with ffprobe | None | Done |
| V3 | Implement transcode sessions and the delivery of HLS playlists and segments | V1, V2 | Done |
| V4 | Split the media info response into images and videos, and return the dimensions and shooting info of images | V2 | Done |
| V5 | Return the rotation of images read with ffmpeg as the media info's `rotation` | V4 | Done |

V1 through V3 together correspond to M8 of [design.md §12.2](../design.md#122-roadmap), V4 to M11, and V5 to M14.

For confirmed defects, see [issues.md](../issues.md).
