# libwebp-sys Builds the Bundled libwebp with cc

**Source: Literature only** (libwebp-sys 0.14.4 README, feature list, and `build.rs` / 2026-10-03)

One item of [knowledges.md](../knowledges.md).

## Fact

With the default features, libwebp-sys builds the libwebp C source bundled in the crate with the `cc` crate and links it.
In this case the build needs a C compiler, but it does not need cmake or a system-installed libwebp.
If you enable the `system-dylib` feature, it does not build the bundled source and instead links the system libwebp found with pkg-config.

## Basis

On 2026-10-03 we consulted the libwebp-sys 0.14.4 README statement "libwebp is built with the `cc` crate. It needs a C compiler, but `cmake` is not used.", the feature list of the same version on docs.rs (the defaults are `parallel` and `std`), and the `system-dylib` branch in `build.rs` (when enabled, it looks for the system libwebp with pkg-config; when disabled, it compiles the C source in `vendor/src` with `cc`).

## Implication

If the default features are used as is, adding lossy WebP encoding needs no additional system packages for the 3-OS CI and for the amd64 and arm64 container images ([Thumbnail Design §10.1](../design/thumbnail.md#choose-the-output-format-and-quality-by-configuration-defaulting-to-webp), [design.md §9.3](../design.md#93-distribution)).
It is premised on a C compiler being present in each environment.
