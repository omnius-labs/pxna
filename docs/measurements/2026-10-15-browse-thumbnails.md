# Comparing the Folder Thumbnail Representative Selection Cache and Delay

Back to the [measurement list](../measurements.md).

## 1. Subject and Conditions

2026-10-15 (UTC). Following [comparison procedure §9](./browse-performance-protocol.md#9-comparing-representative-selection-and-folder-thumbnails), we measured before the change A (daemon-F3, with only the measurement points for representative selection and the image cache) and after the change B (daemon-H), 5 runs each in the order `ABBAABBAAB`. The SHA-256 of the builds and each run's values are in the [aggregate JSON](./2026-10-15-browse-thumbnails.json).

- A dedicated fixture of 22,433 direct children (positive is 22,432 folders and an 8×8 PNG, negative is 1 empty file). The API is 128×128/crop.
- In each run, we restarted the daemon, and after the startup background scan finished, measured the first request with the selection cache empty and the following 5 requests in the same process. The first of the following 5 requests is used for acceptance.
- The image cache was created in advance, and all positive acceptance targets were hits.
- B includes two changes that add caching: (1) the representative selection cache, and (2) a method that takes the `last_accessed` update of the image cache out of the response and writes it together in one transaction after a 500 ms delay. We did not measure them separately.
- The OS, host, and VirtioFS caches were not controlled. No factor is claimed for real data or the original state of about 165 seconds.

## 2. Results

| Condition | A median of acceptance targets | B median of acceptance targets | B/A | B's enumerated count on a selection cache hit |
| --- | --- | --- | --- | --- |
| Positive (with image cache, 200) | 1.0261 s | 0.0065 s | 0.63% | 0 (all 5 runs) |
| Negative (no candidate, 404) | 0.2056 s | 0.0055 s | 2.7% | 0 (all 5 runs) |

The positive targets were 25% or less of before the change and no rescan on a selection cache hit, and both were met.

Each run's acceptance-target values (seconds) are as follows.

| Condition | Each run (alternating order) |
| --- | --- |
| Positive A | 1.0392, 0.2196, 1.0261, 1.0581, 1.0261 |
| Positive B | 0.0065, 0.0065, 0.0065, 0.0062, 0.0068 |
| Negative A | 0.2024, 0.2132, 0.2062, 0.2056, 0.2026 |
| Negative B | 0.0056, 0.0067, 0.0055, 0.0054, 0.0055 |

The medians of the first request (selection cache empty) are 0.2269 seconds for A and 0.1282 seconds for B in the positive case, and 0.2050 seconds for A and 0.1272 seconds for B in the negative case. The first request performs a full enumeration, so even B takes about 0.1 seconds. This is the lower bound of the scan cost on the first call on the negative side, which the cache does not reduce.

For A's positive case, 4 of 5 runs were about 1.03 to 1.06 seconds, and 1 run was 0.22 seconds. The increment of around 0.8 seconds agrees with the variation in the execution time of the image cache's usage-order update. We identified the cause as SQLite writing, but did not isolate the breakdown such as locking and persistence. B does not put the update on the response path, so this variation does not appear in the response.

## 3. UI Check

On Chrome 155.0.8059.79 at 1440×900, with `browse.mjs --thumbnails`, we checked 1 folder, 1 ZIP, and 1 PNG at the root 5 times.

- The folder and archive requests started 200.7 to 202 ms after each tile's DOM (none under 200 ms). The image request was almost simultaneous with the DOM (about −1 ms).
- For the 22k listing (`/twitter`), the DOM was 30 rows (200 or fewer), and from the first response including the target row to the DOM observation of the target row it was 81 to 260 ms (median 85.3 ms, 500 ms or less). The 260 ms was the first run, which includes the daemon's first listing request.

DOM observation does not mean paint completion. Off-screen cancellation, not sending under 200 ms, and image and video prefetching are verified by the [Grid fake-timer tests](../../ui-web/src/components/Grid.test.tsx) and the queue tests. There are no real video files, and prefetching was confirmed with a fake loader.

## 4. Limits

- A and B measured the same fixture alternately, but the influence of order and host variation cannot be removed.
- The effect of taking the image cache update out of the response and the effect of the selection cache are not isolated.
- If the reflection of the usage order is delayed, some updates are lost before a restart. This only changes the LRU eviction order and does not affect image content.
- We did not measure real data, the running daemon, or generation time for videos and heavy archives.
- On macOS (APFS), non-UTF-8 filenames cannot be created, and `summary_rejects_non_utf8_and_special_entries_test` fails with `os error 92`. This is an environment constraint, not treated as an implementation defect. Passing on Linux is unconfirmed.
