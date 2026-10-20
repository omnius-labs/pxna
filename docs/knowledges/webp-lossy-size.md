# Google States That Lossy WebP Is 25% to 34% Smaller Than JPEG at the Same SSIM

**Source: Literature only** (Google's WebP explanation page / 2026-10-03)

One item of [knowledges.md](../knowledges.md).

## Fact

Google, the provider of WebP, states that lossy WebP images are 25% to 34% smaller than JPEG images of the same quality by SSIM.
This value is the result of Google's comparison, not a value measured with images from a Pxna Library and the encoder settings.

## Basis

On 2026-10-03 we consulted the statement "WebP lossy images are 25-34% smaller than comparable JPEG images at equivalent SSIM quality index." at developers.google.com/speed/webp.

## Implication

This is the reason the default format of thumbnails, sprite sheets, and display images is lossy WebP ([Thumbnail Design §10.1](../design/thumbnail.md#choose-the-output-format-and-quality-by-configuration-defaulting-to-webp)).
