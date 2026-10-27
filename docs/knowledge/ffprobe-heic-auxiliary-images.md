# ffprobe 9.0 Shows the Alpha and the Other Auxiliary Images of a HEIC as Gray Video Streams, Told Apart Only by Grouping and Size

**Source: Verified** (ffmpeg 9.0.2, libheif 1.23.6 / 2026-10-26)

One item of [knowledge.md](../knowledge.md).

## Fact

`ffprobe -show_streams -show_stream_groups` of ffmpeg 9.0.2 shows every coded image of a HEIC or AVIF as a video stream, auxiliary images included.
Alpha, depth maps, HDR gain maps, and segmentation mattes all come out with a `gray` pixel format, and no stream field names the kind of auxiliary image.
What differs is the disposition, the membership of a stream group, and the size.

An iPhone HEIC of 5712×4284 (compatible brands `mif1, MiHB, MiHE, MiPr, miaf, heic, tmap`; `heif-info` reports `alpha channel: no` and `depth channel: yes`) gave the following 67 streams and 2 stream groups.

| Streams | Size | Pixel format | Disposition | Stream group |
| --- | --- | --- | --- | --- |
| 0–44, the primary image's tiles | 640×896 | `yuvj420p` | `dependent` | Group 0, `Tile Grid` of 5712×4284, `default` |
| 45, the thumbnail | 416×312 | `yuvj420p` | none | none |
| 46–60, the HDR gain map's tiles | 640×896 | `gray` | `dependent` | Group 1, `Tile Grid` of 2856×2142, not `default` |
| 61, the depth map | 768×576 | `gray` | none | none |
| 62–66, the segmentation mattes | 2016×1512 | `gray` | none | none |

The alpha fixtures in the repository gave the following.

| Fixture | Color streams | Alpha streams | Stream group |
| --- | --- | --- | --- |
| `alpha.heic`, `alpha.avif` | One 512×512, `default` | One 512×512 `gray`, no disposition | none |
| `alpha-grid.heic`, `alpha-grid.avif` | Four 512×512 tiles, `dependent` | Four 512×512 `gray` tiles, no disposition, each listed after its color tile | One `Tile Grid` of 1024×1024 with the color tiles only, `default` |

So in these files, an alpha stream is neither `dependent` nor grouped and has the size of the primary image or of a color tile, while the gain map's tiles are grouped and `dependent`, and the depth map and the mattes have sizes of their own.

We did not check HEIC from other phones or cameras, HEIC whose alpha is tiled differently from its color, AVIF with a gain map, or versions other than ffmpeg 9.0.2.

## Basis

The iPhone HEIC is a user's photo and is not in the repository.
The fixtures are under `daemon/modules/engine/tests/fixtures`.
In a `debian:trixie-slim` container with the BtbN static build `ffmpeg-n9.0.2-17-g2a571b6068` (the one in `Dockerfile`) and `libheif-examples` 1.23.6, the following was run for each file, and the `index`, `width`, `height`, `pix_fmt`, and `disposition` of each stream and the `type`, `disposition`, `components`, and member `index` of each stream group were read from the JSON.

```sh
ffprobe -v error -show_streams -show_stream_groups -of json -i "$file"
heif-info "$file"
```

## Implication

This is the premise of how the daemon tells an image with alpha from an image with other auxiliary images, and so of which images go to heif-dec ([Thumbnail Design §10.1](../design/thumbnail.md#read-heif-and-avif-as-ppm-and-use-heif-dec-for-alpha)).
The rule is `has_alpha` in [image.rs](../../daemon/modules/engine/src/core/media/image.rs#L223), and its tests reproduce the shapes above in [image.rs](../../daemon/modules/engine/src/core/media/image.rs#L513) and [image.rs](../../daemon/modules/engine/src/core/media/image.rs#L539).
