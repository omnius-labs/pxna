# kamadak-exif Reads Exif from TIFF, JPEG, HEIF, PNG, and WebP

**Source: Literature only** (kamadak-exif 0.6.1 API reference and source code / 2026-10-03)

One item of [knowledge.md](../knowledge.md).

## Fact

kamadak-exif's `Reader::read_from_container` detects the format from the first 4096 bytes and reads the Exif inside TIFF (including some RAW formats based on TIFF), JPEG, HEIF, PNG, and WebP.
HEIF is detected by the compatible brand `mif1` or `msf1`, so HEIC and AVIF are also covered.
A format that matches none of these returns `InvalidFormat`.
kamadak-exif is written in pure Rust and does not depend on a C library.

## Basis

On 2026-10-03 we consulted the kamadak-exif 0.6.1 page on docs.rs, `read_from_container` in `exif::reader`, and the source code of the brand detection in `exif::isobmff`.

## Implication

An image's media info can extract the shooting info in the same way from JPEG, PNG, WebP, and TIFF, and from HEIC and AVIF read with ffmpeg ([Video Playback Design §3](../design/video.md#3-media-info)).
Formats that match none of these, such as GIF and BMP, do not yield shooting info.
