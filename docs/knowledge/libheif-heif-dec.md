# libheif's heif-dec and heif-convert Assemble a HEIC Tile Grid and Apply irot

**Source: Verified** (libheif 1.15.1, 1.17.6, 1.23.6 / 2026-10-25)

One item of [knowledge.md](../knowledge.md).

## Fact

For the tile-grid HEIC of 1280×960 made of 6 tiles of 512×512 and the same file with its `irot` angle set to 90 degrees ([ffmpeg and HEIF tile grids](./ffmpeg-heif-tile-grid.md)), `<decoder> input.heic output.png` wrote PNGs of 1280×960 and 960×1280.
The grid is assembled into one image, and the rotation of `irot` is applied.

| libheif | Distribution | Decoder |
| --- | --- | --- |
| 1.15.1 | Debian bookworm (`libheif-examples`) | `heif-convert` |
| 1.17.6 | Ubuntu 24.04 (`libheif-examples`, `libheif-plugin-libde265`) | `heif-convert` |
| 1.23.6 | Debian trixie (`libheif-examples`, `libheif-plugin-libde265`), Homebrew | `heif-dec`, with `heif-convert` also installed |

Ubuntu 24.04's `libheif-examples` has no `heif-dec`.
According to `heif-dec --help` of 1.23.6, auxiliary images such as depth images are also written only with `--with-aux`.

We did not check HEIC that phones write, the mirroring `imir`, files with several top-level images, or versions other than the three above.

## Basis

The test images are `daemon/modules/engine/tests/fixtures/grid.heic` and `grid-rot90.heic`, made as in [ffmpeg and HEIF tile grids](./ffmpeg-heif-tile-grid.md#basis).
For each distribution, the following was run in a container, and the size was read from the PNG's IHDR.

```sh
for image in debian:bookworm-slim ubuntu:24.04 debian:trixie-slim; do
  docker run --rm -v "$PWD":/f:ro "$image" sh -c 'apt-get update -qq
    apt-get install -y -qq --no-install-recommends libheif-examples libheif-plugin-libde265 || apt-get install -y -qq --no-install-recommends libheif-examples
    C=$(command -v heif-dec || command -v heif-convert)
    for f in grid grid-rot90; do $C /f/$f.heic /tmp/$f.png && od -An -tu1 -j16 -N8 /tmp/$f.png; done'
done
```

Bookworm has no `libheif-plugin-libde265` package, and `ldd` shows that its `libheif.so.1` links `libde265.so.0` directly.

## Implication

This is the premise of the decision to decode HEIC with heif-dec ([Thumbnail Design §10.1](../design/thumbnail.md#read-heif-with-libheifs-heif-dec-and-fall-back-to-ffmpeg)).
`decode_heif` in [ffmpeg.rs](../../daemon/modules/engine/src/core/media/ffmpeg.rs#L112) passes only the input and output paths, so `heif-convert` of 1.15.1 and 1.17.6 can be configured as `[media] heif_dec_path`, and the tests use whichever of the two is on `PATH`.
The Docker image installs `libheif-examples` and `libheif-plugin-libde265` of Debian trixie.
