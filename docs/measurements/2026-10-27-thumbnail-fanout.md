# Measuring the Thumbnail Fan-Out Cost

Back to the [measurement list](../measurements.md).

## 1. Subject and Conditions

2026-10-27 (UTC). One generation of a thumbnail now also makes the companions ([Thumbnail Design §3](../design/thumbnail.md#3-determining-the-target)). We measured the generation cost with the companions, the cost of the primary alone within the same generation, the cache hits of a shape switch and a filmstrip request, the bytes the companions add, and the change in the thumbnail hit rate of a short browsing sequence.

- Build: the daemon was built with Rust 1.96.0 in the development profile. The tools were ffmpeg `n9.0.2-17-g2a571b6068` and heif-dec 1.23.6.
- Environment: Debian GNU/Linux 13 (trixie) x86_64, 10 cores, 8 GiB. The OS file cache was not controlled.
- Fixture: five copies of one iPhone 15 Plus HEIC (3,552,847 bytes, 5712 × 4284) under distinct names. Each copy has its own `SourceKey`, so each is a cache miss on its first request.
- State: a fresh daemon state with an empty thumbnail cache.
- Requests: one image channel sends, in order, a 512 × 512 `crop` thumbnail (the listing's request), a 512 × 512 `fit` thumbnail (a shape switch), and a 128 × 128 `crop` thumbnail (the filmstrip's request), at device pixel ratio 1. All are `visible` and are sent one at a time, waiting for each image to finish.
- The configured output format is the default WebP with quality 80.

The fan-out is always on, so the "without companions" value is derived from the same generation rather than from a second build: `thumbnail_production` minus the elapsed time of the `image_resize` and `image_encode` spans of the companions. The decode is shared by both and is counted in both. The store is reported separately.

## 2. Results

Medians of five runs. Seconds unless noted.

| Value | Median |
| --- | ---: |
| `thumbnail_production` with companions | 28.451 |
| Companion resize and encode | 19.961 |
| Derived primary-only production | 5.973 |
| Shape switch, cached companion (`thumbnail_request`) | 0.0025 |
| Filmstrip 128 `crop`, cached companion (`thumbnail_request`) | 0.0023 |
| `thumbnail_store` with companions | 0.0055 |

The first request of each source is the only generation: 5 generations, 5 `thumbnail_convert` spans, and 5 external decodes for 5 sources. The shape switch and the filmstrip request add no decode and no conversion. In this fixture the companions are most of the generation time: the derived primary-only production is about 21% of the generation with companions. The per-run values are in the [aggregate JSON](./2026-10-27-thumbnail-fanout.json).

### 2.1 Bytes

| Value | Median |
| --- | ---: |
| Primary 512 `crop` response | 39,428 bytes |
| Companions added per source | 62,030 bytes |
| Stored per source (primary + companions) | 101,458 bytes |
| Companions per generation | 5 |

The five companions are 128, 256, and 512 in both shapes, minus the primary 512 `crop`. They add about 1.57 times the primary's bytes per source, so the thumbnail budget holds about 2.57 times as many bytes for the same number of sources.

### 2.2 Cache Hit Rate

The sequence makes three thumbnail requests per source.

| | Requests | Hits | Misses | Hit rate |
| --- | ---: | ---: | ---: | ---: |
| With companions | 15 | 10 | 5 | 66.7% |
| Without companions (the same sequence) | 15 | 0 | 15 | 0% |

Within one source, the listing's 512 `crop` request misses, and the 512 `fit` shape switch and the 128 `crop` filmstrip request hit. Without the companions both later requests would decode the source again. The change in this sequence is +66.7 percentage points.

## 3. Limits

The fixture repeats one file, so the medians do not cover other HEIC sizes or pixel content, and the absolute times are not comparable with the records that use a different machine or a warm cache. The "without companions" production is a subtraction of the measured companion span times from the same generation, not a run of the previous build; it excludes the store, which was measured only with companions. The sequence measures three requests per source, not a full listing, a scroll, or a viewer session. The cache was empty and large enough that no companion was evicted, so the record does not include eviction or a bucket that cannot hold the companions.
