# webp Crate 0.3.1 Depends on the 0.9 Series of libwebp-sys

**Source: Literature only** (`Cargo.toml` of webp 0.3.1 / 2026-10-05)

One item of [knowledge.md](../knowledge.md).

## Fact

Version 0.3.1 of the `webp` crate, which can write lossy WebP through a safe API, depends on `libwebp-sys` in the range `^0.9.3`.
Its default feature `img` also pulls in the `image` crate (`^0.25.0`).
This is a different version series from libwebp-sys 0.14.4.

## Basis

On 2026-10-05 we read `[dependencies.libwebp-sys]` (`version = "0.9.3"`) and `[dependencies.image]` (`optional = true`) in the `Cargo.toml` of webp 0.3.1 fetched from crates.io.
`cargo info webp` showed 0.3.1 as the latest version.

## Implication

The properties of 0.14.4 confirmed in [Building libwebp-sys](./libwebp-sys-build.md) do not apply to the 0.9 series reached through the `webp` crate.
Therefore, thumbnail WebP output does not use the `webp` crate and calls libwebp-sys 0.14.4 directly ([Thumbnail Design §10.1](../design/thumbnail.md#choose-the-output-format-and-quality-by-configuration-defaulting-to-webp)).
