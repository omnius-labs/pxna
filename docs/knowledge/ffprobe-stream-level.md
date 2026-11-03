# ffprobe 9.0.2 Reports -99 as the Level of VP8 and VP9 Streams

**Source: Verified** (ffmpeg 9.0.2 / 2026-11-04)

One item of [knowledge.md](../knowledge.md).

## Fact

For a 2-second 320x240 `testsrc` clip at 30 fps, ffprobe 9.0.2 reported the following `level` values for the first video stream.

| File       | Encoder     | `codec_name` | `level` |
| ---------- | ----------- | ------------ | ------: |
| `vp9.webm` | `libvpx-vp9` | `vp9`        |     -99 |
| `vp8.webm` | `libvpx`     | `vp8`        |     -99 |
| `h264.mp4` | `libx264`    | `h264`       |      13 |
| `hevc.mp4` | `libx265`    | `hevc`       |      60 |
| `av1.mp4`  | `libsvtav1`  | `av1`        |       0 |
| `mpeg4.mp4` | `mpeg4`     | `mpeg4`      |       1 |

The checked VP8 and VP9 streams had -99. Every other checked codec had a non-negative level. We did not check other codecs, other container formats, or other ffmpeg versions.

## Basis

The files were made and inspected with the static ffmpeg 9.0.2 build that the daemon image pins ([Dockerfile](../../Dockerfile#L28)).

```sh
ffmpeg -v error -y -f lavfi -i testsrc=duration=2:size=320x240:rate=30 -c:v libvpx-vp9 -b:v 200k vp9.webm
ffmpeg -v error -y -f lavfi -i testsrc=duration=2:size=320x240:rate=30 -c:v libvpx -b:v 200k vp8.webm
ffmpeg -v error -y -f lavfi -i testsrc=duration=2:size=320x240:rate=30 -c:v libx264 -pix_fmt yuv420p h264.mp4
ffmpeg -v error -y -f lavfi -i testsrc=duration=2:size=320x240:rate=30 -c:v libx265 -pix_fmt yuv420p hevc.mp4
ffmpeg -v error -y -f lavfi -i testsrc=duration=2:size=320x240:rate=30 -c:v libsvtav1 -pix_fmt yuv420p av1.mp4
ffmpeg -v error -y -f lavfi -i testsrc=duration=2:size=320x240:rate=30 -c:v mpeg4 -pix_fmt yuv420p mpeg4.mp4
for file in vp9.webm vp8.webm h264.mp4 hevc.mp4 av1.mp4 mpeg4.mp4; do
  printf '%s ' "$file"
  ffprobe -v error -select_streams v:0 -show_entries stream=codec_name,level -of csv=p=0 "$file"
done
```

The `ffprobe` lines printed `vp9,-99`, `vp8,-99`, `h264,13`, `hevc,60`, `av1,0`, and `mpeg4,1`.

## Implication

This is the premise of the decision in [Video Playback Design §7.1](../design/video.md#treat-a-negative-ffprobe-level-as-absent).
The daemon reads the field as a signed integer at [info.rs](../../daemon/modules/engine/src/core/media/info.rs#L55) and uses it only for the H.264 codec string at [info.rs](../../daemon/modules/engine/src/core/media/info.rs#L93).
Treating a negative value as absent keeps the fallback of 31 and lets a VP8 or VP9 probe output parse.
