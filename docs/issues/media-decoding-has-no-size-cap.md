# Media Decoding Has No Size or Dimension Cap

**Severity: high** (confirmed by reproduction; one request can take about 1 GiB)

Investigated 2026-10-25 / Target commit `4f071e1`

One item of [issues.md](../issues.md). Found by the security audit of 2026-10-24 and measured in the second pass. Covers the EXIF read of the timeline, the image thumbnail path, the frame extraction through ffmpeg, and the doubled concurrency introduced by display permits.

## Symptom

A media file placed in a Library makes the daemon allocate memory in proportion to its file size or pixel count, with no upper bound. Measured on the shipped image with ffmpeg 9.0.2:

| Path | Input | RSS increase |
| --- | --- | --- |
| Timeline EXIF read | a 1 GiB TIFF (valid header, zero padding) | +761 MB |
| Image thumbnail | a 1 GiB PNG (valid header, zero padding) | +934 MB |
| AVIF thumbnail through PPM | a 79,979 B 12000x12000 AVIF | +941 MB |

All three requests returned `200`. The daemon did not reject the input or bound the work.

## Locations

- [image.rs#L57](../../daemon/modules/engine/src/core/media/image.rs#L57) — `read_exif` opens the file and passes the whole reader to kamadak-exif, with no length check
- [generator.rs#L405](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L405) — the image path reads the whole file with `tokio::fs::read`
- [ffmpeg.rs#L62](../../daemon/modules/engine/src/core/media/ffmpeg.rs#L62) — `extract_image_pixels` writes a PPM to stdout; the compressed input can be tiny while the output is 3 bytes per pixel
- [process.rs#L9](../../daemon/modules/engine/src/core/media/process.rs#L9) — `run` collects all of stdout into one `Vec<u8>`
- [converter.rs#L40](../../daemon/modules/engine/src/core/image/converter.rs#L40) — `decode` builds a `DynamicImage` without `image::Limits`
- [generator.rs#L186](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L186) — image and video permits plus their display counterparts, each sized by the same option
- [generator.rs#L377](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L377) — `semaphore_for` keeps display work in a second set of permits

## Cause

The engine trusts the declared size of a media file and the dimensions of a decoded image. The EXIF reader, the whole-file read and the `image` decoder all work on the complete input, and the ffmpeg path hands back the raw pixels through stdout. The concurrency limits are per variant, so grid thumbnails and display requests run at the same time and their memory adds up.

## Impact

A Library that accepts content from a lower-trust principal, or the automatic timeline scan over such a Library, can exhaust the daemon's memory. With the defaults the image path allows 4 grid and 4 display decodes at once, so a few requests reach several GiB and the process is killed by the OOM killer. The scan repeats the EXIF read on every aggregation, so a single large file keeps the pressure up.

## Handling Policy

- Check the file length against a media size limit before reading for EXIF or for thumbnails, and before starting ffmpeg
- Give `image::Limits` an explicit pixel and allocation cap, and set the same cap on the ffmpeg path from the probed dimensions before decoding
- Cap the bytes `process::run` will accept from stdout for the PPM path, and kill the child when it is exceeded
- Make the concurrency limit cover grid and display together, so that the option bounds the total rather than each variant
- On exceeding a cap, return `413` or `UnsupportedType` and record a negative result so that the timeline does not retry the file on every aggregation

The limits belong in the engine and their defaults in [Storage Layer Design §7.2](../design/storage.md#72-defaults). The regression tests use a large file with a valid header and a small file with a large declared size, and assert both the error and a bounded RSS.
