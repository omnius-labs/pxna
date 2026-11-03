# VP8 and VP9 Videos Are Rejected as Unsupported Because ffprobe Reports Their Level as -99

**Severity: high** (confirmed by reproduction)

Investigated 2026-11-04 / Target commit `1649ba2`

One item of [issues.md](../issues.md). Found while running the iOS client against a daemon.

## Symptom

For a VP8 or VP9 video, `GET /api/v1/libraries/{libraryId}/media-info` and `POST /api/v1/libraries/{libraryId}/video/sessions` return `400` with `{"kind":"UnsupportedType"}`, although ffprobe reads the file. A video in H.264, HEVC, AV1, or MPEG-4 is accepted. The response carries no `message`, so a client cannot tell this from a file ffprobe cannot read.

Reproduction against a daemon built from the target commit, with the image's ffmpeg 9.0.2:

```sh
ffmpeg -f lavfi -i testsrc=duration=2:size=320x240:rate=30 -c:v libvpx-vp9 -b:v 200k contents/vp9.webm
ffprobe -v quiet -print_format json -show_streams contents/vp9.webm   # "codec_name": "vp9", "level": -99
curl -G -H "Authorization: Bearer $TOKEN" --data-urlencode 'path=/vp9.webm' \
  http://localhost:6050/api/v1/libraries/contents/media-info          # 400 {"kind":"UnsupportedType"}
```

The same file made with `libvpx` (VP8) is rejected too, and files made with `libx264`, `libx265`, `libsvtav1`, and `mpeg4` return `200` with ffprobe levels of 13, 60, 0, and 1.

## Locations

- [info.rs#L55](../../daemon/modules/engine/src/core/media/info.rs#L55) — `ProbeStream::level`, declared as `Option<u32>`
- [info.rs#L64](../../daemon/modules/engine/src/core/media/info.rs#L64) — inside `VideoInfo::parse`, where any deserialization failure becomes `UnsupportedType`
- [info.rs#L93](../../daemon/modules/engine/src/core/media/info.rs#L93) — the only use of `level`, for the H.264 codec string, which already falls back to 31
- [ffmpeg.rs#L17](../../daemon/modules/engine/src/core/media/ffmpeg.rs#L17) — `probe_video_info`, which runs ffprobe and calls `VideoInfo::parse`
- [video.rs#L72](../../daemon/entrypoints/daemon/src/interface/features/video.rs#L72) — the media info handler
- [video.rs#L259](../../daemon/entrypoints/daemon/src/service/video.rs#L259) — the video session creation
- [generator.rs#L882](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L882) — the thumbnail generator, which reads the duration through the same probe

## Cause

ffprobe writes `"level": -99` for the VP8 and VP9 streams checked here. `serde_json` cannot read a negative number as `u32` (`invalid value: integer -99, expected u32`, checked on its own with `Option<u32>`), so the whole probe output fails to parse, and `VideoInfo::parse` reports it as `UnsupportedType` with the message "invalid media information", which does not reach the response: it carries only `kind`.

## Impact

A VP8 or VP9 video, which is the usual content of a WebM, cannot show media info and cannot be played through a video session, so by reading [the client design](../design/client.md) the viewer, which chooses between direct and transcoded playback from the media info, cannot open it. By reading the code, its thumbnail also cannot be generated, because the generator reads the duration through the same probe; this was not reproduced. There is no data loss.

Other codecs for which ffprobe reports a negative level were not checked.

## Handling Policy

Read `level` as a signed integer and treat a negative value as absent, which keeps the H.264 fallback of 31 and ignores the field for every other codec. Add a test to [info.rs](../../daemon/modules/engine/src/core/media/info.rs#L186) that parses the probe output of a VP9 stream with `"level": -99` and asserts the codec and the duration, so that a probe output the daemon cannot read fails a test instead of reaching a client as `UnsupportedType`.
