# Comparing the Background Budget and Child Probes of Visible Rows

## 1. Subject and Conditions

On 2026-10-15 (UTC), we measured a working tree in which phase one, the summary listing and index, and the background budget and child probes were added to commit `4987dd4f9d4f04ce6e04d20ede14312c88b37ba1`.
Following [comparison procedure §8](./browse-performance-protocol.md#8-comparing-the-background-budget-and-child-probes), we used the fixed 22,432 empty folders and 1 empty file, ext4, and a dev build with Rust 1.96.0. Each condition has 5 runs, in the order `ABBAABBAAB`.

The foreground was the existing detail listing for both versions (direct children/name/asc/foldersFirst=true/limit200), and for each run we restarted a dedicated daemon and emptied the in-process snapshot. We separated after background finished and while a manual refresh was running. A run during execution required running=true before sending and a successful empty-folder EOF within the time from the start of the request to the end of the response. The browser, builds, tests, and other intended measurements were not run at the same time. The OS and host caches and external load were not controlled, and this is not a cold comparison.

The foreground baseline is the phase-one `daemon-B`, and the new version is `daemon-E`. The reuse of the index compares the summary listing's final `daemon-D` and `daemon-E`, copying into each run the same fresh index DB created just before. The SHA-256 of the builds, each run's values, and the SHA-256 of the source data and the tools as saved are in the [aggregate JSON](./2026-10-15-browse-background-probes.json).

## 2. Results and Acceptance

| Condition | A median | B median | Increase | Allowed increase | Accept/reject |
| --- | --- | --- | --- | --- | --- |
| Detail listing, after background finished | 1.156171 s | 1.068356 s | -87.815 ms | 231.234 ms | Accepted |
| Detail listing, during background scan | 1.162783 s | 1.268066 s | +105.283 ms | 232.557 ms | Accepted |
| Reuse of a fresh index after restart | 0.258232 s | 0.270695 s | +12.463 ms | 51.646 ms | Accepted |

The allowed increase is `max(20% of A's median, 50ms)`, decided before measurement. During the background scan the new version was about 105 ms slower than the old version, within the allowed range. This result is not treated as contention having disappeared or as a reduction rate of the background load itself.
All 30 runs had HTTP 200 and totalCount=22433. In all 10 runs of index reuse, the seed's snapshot id matched and there was no summary_scan of 22,433 items. In all 10 runs during the background scan, at least 1 empty-folder EOF progressed during the request. The empty fixture's processedFiles=0 was not used as evidence of progress.

### 2.1 Values per Run

In seconds. For A/B, quiet and active are daemon-B/E, and restart is daemon-D/E. EOF is the number of successes within the request time, and is not a comparison of the background's completion time or processing speed.

| Condition | Order | Version | Time | Empty-folder EOF |
| --- | --- | --- | --- | --- |
| quiet | 01 | A | 1.259565 | 0 |
| quiet | 02 | B | 1.136441 | 0 |
| quiet | 03 | B | 1.059305 | 0 |
| quiet | 04 | A | 1.100830 | 0 |
| quiet | 05 | A | 1.124013 | 0 |
| quiet | 06 | B | 1.068356 | 0 |
| quiet | 07 | B | 1.079409 | 0 |
| quiet | 08 | A | 1.351327 | 0 |
| quiet | 09 | A | 1.156171 | 0 |
| quiet | 10 | B | 1.019641 | 0 |
| active | 01 | A | 1.156288 | 2151 |
| active | 02 | B | 1.186910 | 1634 |
| active | 03 | B | 1.399799 | 1912 |
| active | 04 | A | 1.162783 | 1786 |
| active | 05 | A | 1.187773 | 2014 |
| active | 06 | B | 1.163800 | 1519 |
| active | 07 | B | 1.268066 | 1993 |
| active | 08 | A | 1.139407 | 2414 |
| active | 09 | A | 1.210428 | 2534 |
| active | 10 | B | 1.383814 | 2213 |
| restart | 01 | A | 0.254025 | - |
| restart | 02 | B | 0.260267 | - |
| restart | 03 | B | 0.315188 | - |
| restart | 04 | A | 0.271218 | - |
| restart | 05 | A | 0.255101 | - |
| restart | 06 | B | 0.263481 | - |
| restart | 07 | B | 0.272173 | - |
| restart | 08 | A | 0.258232 | - |
| restart | 09 | A | 0.281161 | - |
| restart | 10 | B | 0.270695 | - |

## 3. Functionality and Verification

`cargo make test` passed 438 and skipped 6, and the fmt, clippy, and lint-style it depends on also passed. Only the published destination of DIND's test PostgreSQL was relayed through an investigation relay, and we did not change the product or core-rs sources. The web client passed 66 files and 513 tests on the final tree after formatting, and typecheck, lint, and build also passed. Lint had no errors and 245 warnings, up from the previous 234. The whole test run used maxWorkers=4 and testTimeout=30000 as planned, and the constraint of the existing 100,000-item Timeline test at the default 5 seconds is carried over.

We confirmed with deterministic tests: foreground 3 / background 1, stream foreground 12 / background 4, the real paths of enumeration, EXIF, probes, archives, and index saving, the acquisition order of budgets, the actual worker lifetime after cancellation, non-nesting, and sharing with details. For child probes, we confirmed 128 items / 50 ms, version, TTL, and self-changes, empty only on EOF, and use of only existing archive indexes. For the web client, we confirmed visible rows only, 1 request, cancellation on generation change and off-screen, unknown backoff, remaining TTL, and not waiting on the normal child listing.

With 18 items over real HTTP, we confirmed authentication, boundaries, counts, partial failure, the new probe's true/false/unknown, the remaining TTL, and capabilities' childProbes=true. OpenAPI and the generated artifacts are unchanged from the final summary listing, and the already generated stub of the probe adapter was replaced with an implementation.

With a separate fixed fixture (4 PNGs and 1 ZIP containing 2 PNGs), the dedicated daemon's background scan finished and 6 images were registered in the timeline. This is a functional check that content reading and archive extraction complete using the shared budget, not a performance comparison of heavy content processing.

The manual check in a real browser was at 780×437 and DPR 1, where the initial probes were limited to 11 targets and, after moving to End, to the last 12 targets. The DOM was 16 rows, the page at offset 22400 was fetched, and folder-22431 was focused. A folder confirmed to be empty was also opened by click, the normal listing request completed, and "このフォルダは空です。" (This folder is empty.) was displayed.
In a separate single check at 1440×900, the DOM maximum was 30 rows, there was 1 tree page request for the target, and the target row DOM arrived 44.8 ms after the response. This is a DOM observation and not paint completion, and not a paired UI performance comparison. The thumbnail 404 of empty folders remains and is handled by a later thumbnail countermeasure.

Claude Sonnet 5.5 checked the implementation of S-1 to S-3 and the final verification log in read-only mode and judged pass. The trees before and after matched at `849d133f9c6426767715620d2e8f6b8e9c5108cc`. This judgment does not include this document's S-4 measurement. The measurement record was also separately re-aggregated in read-only mode and passed. The trees before and after verification matched at `5bf582c39030c76e6954b4bda17bb5a114dc4752`.

## 4. Limits

This is ext4 centered on empty folders, and does not reproduce real-data VirtioFS, the load of heavy video probes or solid archives, or the original state of about 165 seconds. The cache layers and external load are not isolated, and alternating the order is not treated as having removed their contribution. We do not assert the contribution of CPU, disk waits, or file system round trips, or whether external batches were running, from this comparison.

The budget limits the specified processing units, and it does not limit HTTP acceptance, waiting futures, root stat, or all I/O of changes and delivery to 4. If the foreground piggybacks on the background's archive single-flight, the possibility of waiting for existing background processing remains. The comparison with the empty fixture does not isolate that effect. The delay of folder thumbnails and the representative selection cache are outside this document.

The first attempt of the measurement tool failed on a missing import, and the first attempt of the auxiliary content-check script failed on a wrong reference to TimelineEntry, each stopping before JSON output. Only the reruns after fixes without changing the product code were adopted, and the failure logs were also saved. We did not touch the running daemon 6050 or write to real data.
