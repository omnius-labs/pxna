# Measuring HEIF Display Image Latency

Back to the [measurement list](../measurements.md).

## 1. Subject and Conditions

2026-10-24 (UTC). To check the report that HEIF display feels slow, we measured the time to decode a HEIC and to generate its display image and thumbnail. We also measured the same-size JPEG to separate the cost of HEIF from the cost of the display pipeline. This is a measurement record, not a comparison of countermeasures; no optimization is included.

We changed only the organization of the media tools (`heif-dec` moved out of `ffmpeg.rs`) and measured before and after it. The temporary test used for the engine-level measurement was not committed.

- Build: `1773c4a` of `main` with the working-tree change that splits the media tools, release profile. The pre-change values use the same tree before that split.
- Environment: Debian GNU/Linux 13 (trixie) arm64 on Linux 6.12.76-linuxkit, 10 cores, 8 GiB. Rust 1.96.0. The OS file cache was not controlled.
- Tools: ffmpeg `n9.0.2-17-g2a571b6068` (BtbN static), libheif 1.23.6, libde265 1.0.15.
- Fixture: one synthetic 4032×3024 image (ffmpeg's `mandelbrot`, JPEG `-q:v 2`, 1.49 MB), the same image as an untiled HEIC and as a HEIC with 512×512 tiles (x265 quality 90, 1.62 MB and 1.60 MB). The JPEG and both HEICs carry the same pixels. Hashes are in the [aggregate JSON](./2026-10-24-heif-display.json).
- Display request: a 1440×900 screen at pixel density 2, `variant=display`, width 2880, height 1800, `resize=fit`, WebP quality 90. The server rounds the long edge up to 4096, and since the image is 4032×3024, resizing is a no-op.
- Thumbnail request: width 512, height 512, `resize=fit`, WebP quality 80.
- Settings: `ThumbnailOptions::default()` (display steps 2048 and 4096, `display_quality` 90, `max_concurrent_images` 4, `max_concurrent_videos` 2).
- Procedure: each of 5 runs uses a fresh copy of the file, so the cache misses. The display image is generated, then the same file is requested again to measure a cache hit. The same 5 copies per format are used for the thumbnail measurement. Engine-level values come from `ThumbnailGenerator::generate`; the breakdown decodes or extracts the source again outside the generator.
- Tool-level values come from running the decoders directly and writing a PNG. Each is 5 runs. We also ran ffmpeg 9 with WebP output in one process (§2.4).

## 2. Results

### 2.1 Decoding to PNG

Seconds are wall time. CPU is user + system. RSS is the decoder's peak. Each cell is median (minimum–maximum) of 5 runs.

| Decoder | HEIC | Wall | CPU | RSS |
| --- | --- | ---: | ---: | ---: |
| heif-dec 1.23.6 | tiled | 0.61 (0.59–0.64) | 0.78 | 62 MiB |
| heif-dec 1.23.6 | untiled | 0.74 (0.73–0.75) | 0.74 | 65 MiB |
| ffmpeg 9.0.2 | tiled | 0.54 (0.52–0.74) | 0.73 | 185 MiB |
| ffmpeg 9.0.2 | untiled | 0.58 (0.57–0.58) | 0.57 | 147 MiB |

On a 1280×960 fixture, both decoders took 0.02 seconds, so process startup is not the cause.

ffmpeg 9 is about 10% faster on the tiled HEIC and about 20% faster on the untiled one, but it uses about three times the peak memory. Replacing the decoder alone therefore does not remove the gap to JPEG.

### 2.2 Display Image Generation

Seconds. Median (minimum–maximum) of 5 cache misses. `read` is `heif-dec`, or reading the JPEG bytes. `png` is decoding the intermediate PNG. Resizing was a no-op for this fixture.

| Source | Total | read | png | resize | WebP | Bytes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| HEIC, tiled | 0.935 (0.904–1.080) | 0.636 | 0.063 | 0.000 | 0.233 | 596,960 |
| HEIC, untiled | 1.064 (1.055–1.070) | 0.791 | 0.060 | 0.000 | 0.250 | 597,598 |
| JPEG | 0.259 (0.240–0.310) | 0.000 | 0.030 | 0.000 | 0.225 | 583,396 |

A first display image takes 0.94 to 1.06 seconds for HEIC and 0.26 seconds for JPEG, so HEIF is 3.6 to 4.1 times slower here. Decoding the HEIC is 0.64 to 0.79 seconds, about 70% of the total. The intermediate PNG costs only 0.06 seconds.

Before the media-tool split, the medians were 0.944 seconds (tiled), 1.079 seconds (untiled), and 0.260 seconds (JPEG). The difference is within run-to-run variation, so the split does not change the time.

### 2.3 Thumbnail Generation and Cache Hits

Seconds. Median (minimum–maximum) of 5 cache misses and of 5 cache hits for the same display request.

| Source | Thumbnail 512 | Cache hit |
| --- | ---: | ---: |
| HEIC, tiled | 0.765 (0.757–0.870) | 0.00046 (0.00038–0.00070) |
| HEIC, untiled | 0.908 (0.885–0.977) | 0.00054 (0.00036–0.00070) |
| JPEG | 0.102 (0.102–0.103) | 0.00043 (0.00035–0.00048) |

Even for a 512-pixel thumbnail, the whole HEIC is decoded, so HEIC is 7.5 to 8.9 times slower than JPEG. A cache hit is under 1 millisecond for every format.

### 2.4 Direct WebP Output by ffmpeg 9

Seconds. Median (minimum–maximum) of 5 runs. The HEIC is decoded and encoded to WebP in one process, without the intermediate PNG and without the Rust WebP encoder.

| libwebp setting | HEIC | Wall | WebP bytes |
| --- | --- | ---: | ---: |
| default | tiled | 0.50 (0.49–0.61) | 570,388 |
| default | untiled | 0.55 (0.55–0.59) | 570,506 |
| `-compression_level 2` | tiled | 0.27 (0.26–0.38) | 588,082 |
| `-compression_level 2` | untiled | 0.32 (0.32–0.33) | 588,184 |
| `-compression_level 0` | tiled | 0.23 (0.23–0.24) | 656,294 |
| `-compression_level 0` | untiled | 0.29 (0.29–0.31) | 657,208 |

`-compression_level` is libwebp's `method`. Level 2 is the same setting the Rust encoder uses, and its output size is close to the current path (588 KB against 597 KB). The default setting is slower (0.50 / 0.55 seconds), so comparing only the default would understate the gain. Level 0 is faster still but about 10% larger.

This is 3.3 to 3.5 times faster than the current display path and nearly the same as the JPEG baseline (0.259 seconds). The tile grid is assembled and `irot` is applied; `grid-rot90.heic` produced a 960×1280 WebP.

The tile grid cannot be combined with a simple `-vf scale`: ffmpeg builds an internal complex filtergraph for the grid, so `-vf` either conflicts or reads a single tile. For a thumbnail, scaling the full-size WebP in a second step took 0.09 seconds (512×384), and decoding to raw pixels first took 0.09 to 0.15 seconds. A thumbnail can therefore be produced by decoding to raw pixels and resizing in Rust, or by writing the full-size WebP and shrinking it afterwards.

## 3. Conclusion

The report is reproducible at the engine level. HEIF display is slower than JPEG mainly because the whole HEIC is decoded for every request, including thumbnails, and HEVC decoding takes 0.64 to 0.79 seconds for this 12-megapixel image. The PNG roundtrip and WebP encoding are not the main cause. Cache hits and process startup are not the cause.

The most effective external route measured here is ffmpeg 9 writing WebP directly with `-compression_level 2`: 0.27 to 0.32 seconds, 3.3 to 3.5 times faster than the current path and close to JPEG. It avoids the PNG roundtrip without linking libheif into the daemon, so the crash isolation of the external-process design is kept.

This route depends on ffmpeg 9, which assembles tile grids. Older ffmpeg reads only one tile without failing, so choosing it needs an explicit version or stream-group check and a fallback to heif-dec. We did not implement or verify that guard here. Reducing the decoded size and the thumbnail path are still open.

## 4. Limits

- The source is one synthetic image. Its detail, noise, and HEVC bitstream are not representative of phone photos, and the absolute values should not be extrapolated to real libraries.
- We measured one size (12 megapixels) and one bit depth (8-bit). We checked `irot` only with `grid-rot90.heic`, and did not check `imir`, HDR, or files with multiple top-level images.
- The engine-level measurement is `ThumbnailGenerator::generate` without HTTP, authentication, a real network, browser decoding, or painting. Past measurements found the API overhead to be about 2 milliseconds on loopback.
- All requests were sequential. We did not measure several HEICs arriving at once, the shared limit of 2 for ffmpeg and heif-dec runs, or flipping through a list.
- The OS file cache was not controlled, and the pre-change and post-change runs were not alternated. The refactor comparison is only a sanity check that the values did not move.
- The tiled HEIC was faster than the untiled one here. That ordering depends on the encoder and content and is not a general property of tiled HEICs.
