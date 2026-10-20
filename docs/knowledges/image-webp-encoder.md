# The image Crate's WebP Encoder Has Only Lossless Compression

**Source: Literature only** (image 0.25.9 source code and the docs.rs API reference / 2026-10-03)

One item of [knowledges.md](../knowledges.md).

## Fact

The only constructor that the image crate's `codecs::webp::WebPEncoder` exposes is `new_lossless`, and it has no way to write lossy WebP.
The image format support table also lists WebP encoding as lossless only.
WebP decoding supports both lossy and lossless, so this constraint applies only to encoding.

## Basis

On 2026-10-03 we consulted `src/codecs/webp/encoder.rs` in the image 0.25.9 source code (that the `WebPEncoder` constructor is only `new_lossless`), the format support table in `src/lib.rs` (the statement that WebP encoding is "Only lossless encoding"), and the docs.rs page for `codecs::webp::WebPEncoder` of image 0.25.9.

## Implication

Outputting thumbnails, sprite sheets, and display images as lossy WebP requires an encoder other than image ([Thumbnail Design §4.2](../design/thumbnail.md#42-shape-and-output-format)).
