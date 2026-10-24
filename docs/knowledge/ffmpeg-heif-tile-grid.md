# ffmpeg 8.1 and 9.0 Assemble a HEIC Tile Grid Only Under the Default Stream Selection, and Only ffprobe 9.0 Reports Its Rotation

**Source: Verified** (ffmpeg 5.1.9, 6.1.1, 7.1.5, 8.1.3, 9.0.2 / 2026-10-25)

One item of [knowledge.md](../knowledge.md).

## Fact

The HEIC that macOS 26.6.2's `sips` writes from a 1280×960 PNG is a tile grid: a `grid` item that bundles 6 HEVC tiles of 512×512 (3×2).
ffmpeg shows each tile as a stream and the grid as a stream group of type `Tile Grid`.
For this file and for the same file with its `irot` angle set to 90 degrees, ffmpeg behaved as follows.

| ffmpeg | Opening the file | Decoding with the default stream selection | Decoding with `-map 0:v:0` | `-show_stream_groups` of ffprobe |
| --- | --- | --- | --- | --- |
| 5.1.9 (Debian bookworm), 6.1.1 (Ubuntu 24.04) | Fails with `moov atom not found` | — | — | The option is unknown and ffprobe fails |
| 7.1.5 (Debian trixie) | Opens | One 512×512 tile | One 512×512 tile | `Tile Grid` of 1280×960, no rotation |
| 8.1.3 (BtbN static build) | Opens | 1280×960, and 960×1280 with `irot` 90 | One 512×512 tile | `Tile Grid` of 1280×960, no rotation even with `irot` 90 |
| 9.0.2 (BtbN static build, Homebrew) | Opens | 1280×960, and 960×1280 with `irot` 90 | One 512×512 tile | `Tile Grid` of 1280×960, with a Display Matrix `rotation` of 90 when `irot` is 90 |

In 9.0.2, the rotation is in the `side_data_list` of the `Tile Grid` component, and the width and height of the component stay as encoded.
The failure of `-show_stream_groups` on 5.1.9 and 6.1.1 was also seen with an AVIF without tiles (`daemon/modules/engine/tests/fixtures/blue.avif`).

We did not check HEIC that phones write, the mirroring `imir`, grids whose size is not a multiple of the tile size in both directions, or ffmpeg versions other than the five above.

## Basis

The test images were made on macOS 26.6.2 as follows, and are the same as `daemon/modules/engine/tests/fixtures/grid.heic` and `grid-rot90.heic`.
`sips` writes an `irot` box with an angle of 0, so only that byte was changed.

```sh
ffmpeg -f lavfi -i testsrc=s=1280x960 -frames:v 1 base.png
sips -s format heic base.png --out grid.heic
python3 -c '
d = bytearray(open("grid.heic", "rb").read())
i = d.index(b"irot")
assert d.count(b"irot") == 1 and d[i - 4:i] == b"\x00\x00\x00\x09" and d[i + 4] == 0
d[i + 4] = 1   # irot is not a FullBox, so the angle is the byte right after the type; 1 means 90 degrees counterclockwise
open("grid-rot90.heic", "wb").write(d)
'
```

For each ffmpeg, the following was run on both files.
5.1.9, 6.1.1, and 7.1.5 were the distribution packages in `debian:bookworm-slim`, `ubuntu:24.04`, and `debian:trixie-slim` containers.
8.1.3 and 9.0.2 were `ffmpeg-n8.1.3-9-g29e619e767-linuxarm64-gpl-8.1.tar.xz` and `ffmpeg-n9.0.2-17-g2a571b6068-linuxarm64-gpl-9.0.tar.xz` of the BtbN/FFmpeg-Builds release `autobuild-2026-09-30-13-08`, run in `debian:trixie-slim`, and 9.0.2 was also Homebrew's on macOS.

```sh
ffmpeg -v error -i grid-rot90.heic -frames:v 1 -y a.png && ffprobe -v error -show_entries stream=width,height -of csv=p=0 a.png
ffmpeg -v error -i grid-rot90.heic -map 0:v:0 -frames:v 1 -y b.png && ffprobe -v error -show_entries stream=width,height -of csv=p=0 b.png
ffprobe -v error -show_streams -show_stream_groups -of json grid-rot90.heic
```

## Implication

This is the premise of why images read with ffmpeg are extracted under the default stream selection ([Thumbnail Design §3](../design/thumbnail.md#3-determining-the-target)), and of why the Docker image bundles ffmpeg 9.0 instead of Debian's package ([design.md](../design.md#bundle-a-pinned-static-build-of-ffmpeg)).
`extract_image` in [ffmpeg.rs](../../daemon/modules/engine/src/core/media/ffmpeg.rs#L97) passes no `-map`.
`probe_image_size` in [ffmpeg.rs](../../daemon/modules/engine/src/core/media/ffmpeg.rs#L63) adds `-show_stream_groups` and runs ffprobe again without it when ffprobe fails on that option, and `tile_grid_size` in [image.rs](../../daemon/modules/engine/src/core/media/image.rs#L161) prefers the `Tile Grid` size and rotation ([Video Playback Design §3](../design/video.md#3-media-info)).
With ffprobe 8.1.3, the media info of a rotated tile grid would have no `rotation` while its thumbnails and display images are rotated.
