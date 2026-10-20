# Section Measurement of Folder Thumbnails Before the Change

## 1. Subject and Conditions

2026-10-15 (UTC). We used daemon-F, which adds only time and count logs to the code after the background budget was introduced, and measured with a dedicated fixture of 22,433 direct children. The positive fixture is 22,432 folders and an 8×8 PNG, and the negative fixture is the existing group of empty folders and an empty file. We used the 128×128/crop API and waited for the startup background scan to finish. Image cache hits were confirmed in the log. The conditions differ from the original real data with about 165 seconds.

The SHA-256 of the build, each request, and the supporting files are in the [aggregate JSON](./2026-10-15-browse-thumbnails-before.json). This is a before-change record for investigating the contribution of countermeasures, not an alternating comparison for acceptance. From here on, use [comparison procedure §9](./browse-performance-protocol.md#9-comparing-representative-selection-and-folder-thumbnails).

## 2. Results

| Condition | API | Representative selection | Image generation and saving | Enumerated count |
| --- | --- | --- | --- | --- |
| Positive, first request | 0.337107 s | 0.193574 s | 0.136867 s | 22,433 |
| Positive, median of 5 requests with the image cache | 1.030441 s | 0.199043 s | No generation because of a hit | 22,433 every time |
| Negative, first request (404) | 0.208237 s | 0.202756 s | No candidate | 22,433 |
| Negative, median of 5 requests (404) | 0.214511 s | 0.207780 s | No candidate | 22,433 every time |

Even with the image cache, the full enumeration and attribute fetching at selection time are repeated. Of the 5 positive requests, the first took about 0.217 seconds and the later 4 took about 1.03 seconds. The roughly 0.2 seconds of selection alone cannot explain this difference.

### 2.1 Image Cache Update Section

With daemon-F2, which has additional measurement points, there were requests where the SELECT took about 0.5 ms and reading the image file about 0.2 ms, while the UPDATE that updates the usage order took about 0.815 to 0.821 seconds. With daemon-F3, which separates connection acquisition from execution, connection acquisition was about 0.1 to 0.2 ms, and execution varied from about 0.024 to 0.787 seconds. We identified the execution section of the update, including SQLx, but did not isolate the contributions of SQLite locking, persistence, or host state. With Python SQLite in a separate process, commits on the same DB took a few ms, but the driver and connection state differ, so it is not used to settle the cause.

This additional measurement did not change the cache's format, keys, or LRU rules. Later comparisons fix F3, which separates connection acquisition from execution, as the baseline. Acceptance of an improvement is judged on the whole API, and we do not loosen the 25% target because of this variation.

### 2.2 Initial UI Requests

On Chrome 155.0.8059.79 at 1440×900, we displayed 1 folder, 1 ZIP, and 1 PNG at the root. The folder and archive requests had no 200 ms wait. From the DOM MutationObserver notification the requests were about 2 to 3 ms earlier, but this includes the execution order of React effects and the observer, so it is not interpreted as before the DOM itself was created. There are no real video files, and the prefetch retention is confirmed in a test with a fake loader.

## 3. Limits

We used a group of empty ext4 folders and did not control the OS and host caches. We used a separate build for time measurement and the dedicated port 16053, and did not change the running daemon on 6050 or the real data. The API time, representative selection, generation and saving, and DOM observation are different sections, and a simple sum is not treated as the time of screen painting completion. We also distinguish the existing memory and image caches from the representative selection cache to be introduced next.
