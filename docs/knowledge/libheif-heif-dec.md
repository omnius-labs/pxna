# libheif's heif-dec and heif-convert Assemble a HEIC Tile Grid, Apply irot, and Write TIFF from 1.19 When Built with libtiff

**Source: Verified** (libheif 1.15.1, 1.17.6, 1.19.7, 1.21.2, 1.23.6 / 2026-10-26)

One item of [knowledge.md](../knowledge.md).

## Fact

For the tile-grid HEIC of 1280×960 made of 6 tiles of 512×512 and the same file with its `irot` angle set to 90 degrees ([ffmpeg and HEIF tile grids](./ffmpeg-heif-tile-grid.md)), `<decoder> input.heic output.png` wrote PNGs of 1280×960 and 960×1280.
The grid is assembled into one image, and the rotation of `irot` is applied.
In 1.19.7 and 1.23.6, `output.tiff` wrote the same image with the same dimensions.

| libheif | Distribution | Decoder | `--png-compression-level` | TIFF at run time |
| --- | --- | --- | --- | --- |
| 1.15.1 | Debian bookworm (`libheif-examples`) | `heif-convert` | Not in the help | No |
| 1.17.6 | Ubuntu 24.04 (`libheif-examples`, `libheif-plugin-libde265`) | `heif-convert` | In the help | No |
| 1.19.7 | Ubuntu 25.04 (`libheif-examples`, `libheif-plugin-libde265`) | `heif-dec` | In the help | Yes |
| 1.21.2 | Ubuntu 26.04 (`libheif-examples`, `libheif-plugin-libde265`) | `heif-dec` | In the help | No, built without libtiff |
| 1.23.6 | Debian trixie (`libheif-examples`, `libheif-plugin-libde265`), Homebrew | `heif-dec`, with `heif-convert` also installed | In the help | Yes |

Ubuntu 24.04's `libheif-examples` has no `heif-dec`.
The source of 1.18.0 has no TIFF output; 1.19.0 has `heifio/encoder_tiff.cc` and the `tif`/`tiff` branch in `examples/heif_dec.cc`.
Ubuntu 26.04's 1.21.2 lists `tif` and `tiff` in its help but was built without libtiff, and fails with `TIFF support has not been compiled in.` So a version of 1.19 or newer does not guarantee TIFF output; the build must include libtiff.
According to `heif-dec --help` of 1.23.6, auxiliary images such as depth images are also written only with `--with-aux`.
Both 1.17.6 and 1.23.6 print `--help` to stderr and exit with 0. The option takes 0 (fastest) to 9 (best), and -1 for the default.

Most of a decode to PNG is the PNG filtering and writing.
For a 5712×4284 iPhone HEIC (a tile grid of 45 tiles with a depth map and an HDR gain map), heif-dec 1.23.6 took the following on 9 cores (amd64), one run at a time.

| Output | Time | Size |
| --- | ---: | ---: |
| PNG, default compression | 11.6 and 13.5 seconds | 23,545,700 bytes |
| PNG, `--png-compression-level 1` | 3.3 seconds | 25,802,465 bytes |
| PNG, `--png-compression-level 0` | 2.0 seconds | 73,539,326 bytes |
| Y4M | 0.6 seconds | — |

For comparison, ffmpeg 9.0.2's PPM extraction of the same file took 0.7 and 0.9 seconds.

For a synthetic 4032×3024 image with 50% alpha, encoded at quality 90 as untiled and 512×512-tiled HEIC and AVIF, heif-dec 1.23.6 took the following on 9 cores (amd64), median of 9 runs, to tmpfs.
The decoders were libde265 for HEIC and dav1d for AVIF, and the TIFF runs used Debian trixie's 1.23.6, which is built with libtiff.

| Input | PNG, `--png-compression-level 0` | TIFF | TIFF size |
| --- | ---: | ---: | ---: |
| HEIC, untiled | 0.644 seconds | 0.378 seconds | 48,789,402 bytes |
| HEIC, tiled | 0.507 seconds | 0.240 seconds | 48,789,402 bytes |
| AVIF, untiled | 0.509 seconds | 0.227 seconds | 48,789,402 bytes |
| AVIF, tiled | 0.516 seconds | 0.230 seconds | 48,789,402 bytes |

The level-0 PNGs were 48,851,227 bytes, so the TIFF saves the PNG filtering pass rather than I/O.
The Rust `image` crate 0.25.10 in a release build decoded the PNG in 42 ms and the TIFF in 52 ms from memory.
`--tile-threads` defaulted to 4, which was near the best value for tiled images (8 improved it by a few percent); `--codec-threads` from 1 to 8 did not change the untiled time.
The FFmpeg decoder plugin brought untiled HEIC to 0.280 seconds and tiled HEIC to 0.204 seconds, but installing `libheif-plugin-ffmpegdec` on Debian trixie adds 79 packages, so the daemon does not use it.

We did not check the grid assembly and `irot` with HEIC that phones write, the mirroring `imir`, files with several top-level images, or versions other than the five above. The 12MP numbers come from one synthetic image on a shared host and vary by roughly ±20%.

## Basis

The test images are `daemon/modules/engine/tests/fixtures/grid.heic` and `grid-rot90.heic`, made as in [ffmpeg and HEIF tile grids](./ffmpeg-heif-tile-grid.md#basis).
For each distribution, the following was run in a container, and the output files' sizes were read with `stat`.
The `.tiff` conversion fails on 1.15.1 and 1.17.6 with `Unknown file type`, succeeds on 1.19.7 and 1.23.6, and fails on 1.21.2 with `TIFF support has not been compiled in.`

```sh
for image in debian:bookworm-slim ubuntu:24.04 ubuntu:25.04 ubuntu:26.04 debian:trixie-slim; do
  docker run --rm -v "$PWD":/f:ro "$image" sh -c 'apt-get update -qq
    apt-get install -y -qq --no-install-recommends libheif-examples libheif-plugin-libde265 || apt-get install -y -qq --no-install-recommends libheif-examples
    C=$(command -v heif-dec || command -v heif-convert)
    for f in grid grid-rot90; do
      $C /f/$f.heic /tmp/$f.png && stat -c %s /tmp/$f.png
      $C /f/$f.heic /tmp/$f.tiff && stat -c %s /tmp/$f.tiff
    done'
done
```

Bookworm has no `libheif-plugin-libde265` package, and `ldd` shows that its `libheif.so.1` links `libde265.so.0` directly.

The option and suffix columns were read from `heif-convert --help` (`heif-dec --help` on trixie, Ubuntu 25.04, and Ubuntu 26.04) in each container, and the TIFF column was checked by running the conversion.
The source comparison used `examples/heif_dec.cc` at tags v1.18.0 and v1.19.0, and the v1.19.0 tree contains `heifio/encoder_tiff.cc`.

The iPhone timings used the iPhone HEIC, which is a user's photo and is not in the repository, in a `debian:trixie-slim` container with `libheif-examples` and the plugins of the `Dockerfile` and the ffmpeg build it pins.

```sh
time heif-dec "$file" /tmp/o.png
time heif-dec --png-compression-level 0 "$file" /tmp/o.png
time ffmpeg -hide_banner -loglevel error -nostdin -i "$file" -frames:v 1 -an -sn -pix_fmt rgb24 -f image2pipe -vcodec ppm - > /tmp/o.ppm
```

The 12MP fixture was made as follows, and each output was written to tmpfs.

```sh
ffmpeg -f lavfi -i testsrc2=s=4032x3024 -vf "format=rgba,colorchannelmixer=aa=0.5" -frames:v 1 big.png
heif-enc -q 90 big.png -o big.heic
heif-enc -q 90 --cut-tiles 512 big.png -o big-grid.heic
heif-enc -A -q 90 big.png -o big.avif
heif-enc -A -q 90 --cut-tiles 512 big.png -o big-grid.avif
heif-dec --quiet --png-compression-level 0 big.heic /dev/shm/o.png
heif-dec --quiet big.heic /dev/shm/o.tiff
```

The TIFF and the PNG were compared with `psnr` (all planes `inf`) and by the MD5 of the alpha plane, for untiled and tiled HEIC and AVIF.
A 640×480 fixture made with `heif-enc --rotate-cw 90` gave the same match, so the TIFF path keeps `irot`.
The Rust decode times used a release build of the `image` crate 0.25.10 with only the `png` and `tiff` features, reading the file into memory first.

## Implication

This is the premise of using heif-dec for HEIC/HEIF/AVIF images with alpha and as a fallback, while ffmpeg reads the primary image ([Thumbnail Design §10.1](../design/thumbnail.md#read-heif-and-avif-as-ppm-and-use-heif-dec-for-alpha)).
`decode_heif` in [heif.rs](../../daemon/modules/engine/src/core/media/heif.rs#L16) asks for `image.tiff` first and retries with a level-0 PNG when heif-dec reports `TIFF support has not been compiled in.` or `Unknown file type` (the latter is libheif before 1.19).
`require_heif_dec` in [media_check.rs](../../daemon/entrypoints/daemon/src/media_check.rs#L42) reads the help from stdout and stderr at startup and requires libheif 1.17 or newer by checking `--png-compression-level`, which the PNG fallback passes.
The Docker image installs libheif 1.23.6 from Debian trixie, which has libtiff and uses TIFF. Homebrew's libheif depends on `libtiff` and has the same output. Ubuntu 26.04's 1.21.2 is built without libtiff and uses the PNG fallback.
