# Release A/B of the Thumbnail Fan-Out

Back to the [measurement list](../measurements.md).

## 1. Subject and Conditions

2026-10-27 (UTC). We compared the thumbnail fan-out on and off in release builds. The development-profile record of the same change is [2026-10-27-thumbnail-fanout](./2026-10-27-thumbnail-fanout.md); this record replaces its derived "primary only" value with a measured B side.

- A build: the current worktree with fan-out (`cd daemon && RUSTC_WRAPPER= cargo build --release -p omnius-pxna-daemon`).
- B build: the same command after a temporary early `return Vec::new();` in `ThumbnailGenerator::companion_plans`. The patch was removed after the B run. The file hashes were:
  - before: `80485487e4a8ca52220ad3ce03f70dfe5889015709eee166b6a4fd8326647290`
  - patched: `d4576b2d34ea7ef99196b0ad9aedb14bbf21a3ff44ec39712bfca6d3e576ced1`
  - restored: `80485487e4a8ca52220ad3ce03f70dfe5889015709eee166b6a4fd8326647290`
- Profile: `release` (`debug = 1`, default optimized settings; no LTO change).
- Environment: Debian GNU/Linux 13 (trixie) x86_64, 10 cores, 8 GiB. Rust 1.96.0. The tools were ffmpeg `n9.0.2-17-g2a571b6068` and heif-dec 1.23.6.
- Fixture: five copies of one iPhone 15 Plus HEIC (3,552,847 bytes, 5712 × 4284). Each copy has its own `SourceKey`.
- Cache: a fresh daemon state and empty thumbnail cache for each side. The OS file cache was not controlled.
- Requests: one image channel sends, one at a time, a 512 × 512 `crop` thumbnail (the listing's request), a 512 × 512 `fit` thumbnail (a shape switch), and a 128 × 128 `crop` thumbnail (the filmstrip's request), at device pixel ratio 1. The same sequence and fixture are used for A and B. Five sources give five runs per side.
- The output format is the default WebP with quality 80.

## 2. Results

Medians of five runs. Milliseconds unless noted.

| Value | A (fan-out on) | B (fan-out off) | A − B |
| --- | ---: | ---: | ---: |
| Client-observed primary request | 703.5 | 542.7 | +160.8 |
| Server `thumbnail_request`, primary | 702.4 | 541.4 | +161.0 |
| Server `thumbnail_production`, primary | 701.6 | 540.6 | +161.0 |
| Companion resize and encode | 194.0 | 0 | +194.0 |
| Shape switch, server request | 0.9 | 467.8 | −466.9 |
| Shape switch, client-observed | 2.2 | 469.0 | −466.8 |
| Filmstrip 128 `crop`, server request | 0.9 | 478.2 | −477.3 |
| Filmstrip, client-observed | 1.6 | 479.0 | −477.4 |
| `thumbnail_store` | 3.9 | 1.5 | +2.4 |

The fan-out adds about 161 ms to the listing's first generation and makes the two later requests cache hits. In B the shape switch and the filmstrip request decode the source again, each taking about 0.47 seconds.

### 2.1 Decodes and Hit Rate

| | A | B |
| --- | ---: | ---: |
| External decodes | 5 | 15 |
| Conversions | 5 | 15 |
| Generations | 5 | 15 |
| Requests | 15 | 15 |
| Hits | 10 | 0 |
| Hit rate | 66.7% | 0% |

Each source is decoded once in A and three times in B. The change is −10 external decodes and +66.7 percentage points of hit rate for this sequence.

### 2.2 Bytes

| Value | A | B |
| --- | ---: | ---: |
| Primary 512 `crop` response | 39,428 | 39,428 |
| Companions added per source | 62,030 | 0 |
| Stored per source | 101,458 | 39,428 |
| Companions per generation | 5 | 0 |

The primary's bytes and `ETag` are the same on both sides: the A and B responses for the primary carry the same key and 39,428 bytes. The companions add 62,030 bytes per source, or about 1.57 times the primary's bytes. The thumbnail budget therefore holds about 2.57 times the bytes for the same number of sources.

The per-run values are in the [aggregate JSON](./2026-10-27-thumbnail-fanout-release-ab.json).

## 3. Limits

The fixture repeats one file, so the medians do not cover other HEIC sizes or pixel content. Five runs per side, one process, and an empty cache; the OS file cache is not controlled, and the first runs of each side may have paid more of the read cost. B was built from a temporary local patch, restored byte-for-byte; the A binary was built from the current worktree before that patch and rebuilt after the restore to confirm the restore. The client-observed latencies include the browser, the channel, and transfer on the same host; the server spans do not.
