# Measurement Records and Comparison Procedures

This document lists records of performance observed at specific commits and environments, and procedures for comparing before and after a change.
Each record has the measurement method, the numbers, and the limits, and values after an implementation change are not appended over it. A re-measurement is kept as a record with a different date.

## 1. Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | List of measurement records and comparison procedures |
| [design.md](./design.md#11-division-of-responsibility-between-documents) | Map of the document family, design decisions and invariants |
| [plans.md](./plans.md) | Order and progress of the work that responds |
| [reviews.md](./reviews.md) | Review judgments and transcription status of findings |

## 2. Record List

| Record | Measured (UTC) | Subject |
| --- | --- | --- |
| [2026-10-15-browse-performance](./measurements/2026-10-15-browse-performance.md) | 2026-10-14 to 15 | Listing API for large folders, the left tree, attribute fetching, with and without background processing |
| [2026-10-15-tree-virtualization](./measurements/2026-10-15-tree-virtualization.md) | 2026-10-15 | Comparison of staged display and virtualization of the tree with a fixed set of about 22k folders |
| [2026-10-15-browse-phase-one](./measurements/2026-10-15-browse-phase-one.md) | 2026-10-15 | Five runs each comparing the tree and attribute fetching on the final build, and the accept/reject decision for phase one |
| [2026-10-15-browse-summary-index](./measurements/2026-10-15-browse-summary-index.md) | 2026-10-15 | Five runs each comparing the final build of summary listing, detail completion, and the persistent index |
| [2026-10-15-browse-background-probes](./measurements/2026-10-15-browse-background-probes.md) | 2026-10-15 | Reserved foreground and background slots, responsiveness during background scanning, index reuse, and probes of visible rows |
| [2026-10-15-browse-thumbnails-before](./measurements/2026-10-15-browse-thumbnails-before.md) | 2026-10-15 | Before-change measurement of representative selection, image cache updates, and initial thumbnail requests |
| [2026-10-15-browse-thumbnails](./measurements/2026-10-15-browse-thumbnails.md) | 2026-10-15 | Comparison of the representative selection cache, delayed usage-order updates, and the 200 ms delay of folder requests |
| [2026-10-16-display-image-latency](./measurements/2026-10-16-display-image-latency.md) | 2026-10-16 | The viewer's first generation of a display image, cache hits, fetching the original image, and waits during consecutive flipping |
| [2026-10-16-webp-method](./measurements/2026-10-16-webp-method.md) | 2026-10-16 | A sweep of WebP compression effort (method), and first generation and consecutive flipping before and after lowering it from 4 to 2 |
| [2026-10-24-heif-display](./measurements/2026-10-24-heif-display.md) | 2026-10-24 | HEIC decoding, display image and thumbnail generation, and cache hits compared with the same-size JPEG |
| [2026-10-24-heif-ppm](./measurements/2026-10-24-heif-ppm.md) | 2026-10-24 | Display and thumbnail generation after switching ffmpeg-read images to PPM output |

## 3. Comparison Procedures

| Procedure | Use |
| --- | --- |
| [browse-performance-protocol](./measurements/browse-performance-protocol.md) | Compares the listing API, the tree, attribute fetching, responsiveness during background scanning, and index reuse with a fixed set of about 22k Entries. It also includes section-by-section measurement of display image latency |
