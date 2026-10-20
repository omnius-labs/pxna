# Comparing Summary Listing, Detail Completion, and the Persistent Index

## 1. Subject and Conditions

On 2026-10-15 (UTC), we measured a working tree in which the phase-one and the summary listing and index implementations were added to commit `4987dd4f9d4f04ce6e04d20ede14312c88b37ba1`.
We take `daemon-B` of the phase-one [final record](./2026-10-15-browse-phase-one.md#1-subject-and-conditions) as the previous version and this time's final build `daemon-D` as the new version. Their SHA-256 values are `3016f90c520bc3c9480f12c5f6e759cf53b776492e16e9fc82f6866d349c09cf` and `9d1a587ea61d66bd7e95fbb980cd19202126af6ecb47a208f89388c360b757a9` respectively.
For the web client we compared `web-B2` and `web-C3`. The SHA-256 of all files, each run's values, the SHA-256 of the source data, and copies of the measurement tools and procedure as of when they were saved are in the [aggregate JSON](./2026-10-15-browse-summary-index.json).

Following [comparison procedure §7](./browse-performance-protocol.md#7-comparing-summary-listing-and-the-index), with the fixed 22,432 empty folders and 1 empty file, ext4, and a dev build with Rust 1.96.0, we alternated 5 runs per condition in the order `ABBAABBAAB`.
Each API run was restarted independently and measured after its own timeline scan had finished. External load and the OS cache were not controlled, and this is not a cold measurement. The fixture's manifest and its names, contents, and modification times still matched after the comparison.

In the first-call API comparison, A is the previous detail listing, and B is the new version's summary listing with only names and kinds. Both are direct children, ascending name order, `foldersFirst=false`, `limit=200`, with an empty snapshot and a count of 22,433 items. The new version's persistent index was also emptied. The time to fetch details is not included in the summary listing's first response, so this is not a comparison of the same amount of information.
The comparison after restart used the same `daemon-D` for both A and B, where A is without the index and B is reuse of a fresh index. The seed was stopped after saving completed, its WAL was checkpointed, and then it was copied. In all 5 runs of B, we confirmed that the seed's snapshot id matched and that no summary scan or sort events were emitted.

The browser was `Chrome/155.0.8059.79` at 1440 × 900, compared on the same `daemon-D`. We read the snapshots of both listings in advance and used a new tab with the HTTP cache disabled. The Chrome process and other caches are shared.

## 2. Results and Acceptance

| Metric | A | B | Predefined condition | Accept/reject |
| --- | --- | --- | --- | --- |
| First API median | 1.057552 s | 0.910825 s | 90% or less of previous | Accepted (86.13%) |
| New version's API median after restart | 0.895577 s | 0.258630 s | 50% or less of without the index | Accepted (28.88%) |
| Cached API median (25 requests each) | 9.704 ms | 5.290 ms | Increase within 5.000 ms | Maintained |
| Target row DOM arrival after the tree response, median | 73.600 ms | 55.200 ms | 500 ms or less | Maintained |
| Target row DOM arrival, median | 306.100 ms | 279.600 ms | Reference value | - |
| Maximum DOM rows of the tree | 30 | 30 | 200 or fewer | Maintained |
| Initial page requests of the target tree | 1 | 1 | 2 or fewer | Maintained |

The time after the response is a DOM observation, not paint completion. Do not equate the shortened first API with detail fetching becoming unnecessary.
The API, the listing scan, and the driver sections are nested, so they are not added. For each event, we took the last one with a count of 22,433 items, so as not to confuse it with the startup scan.

### 2.1 Values per Run of the First API

The unit is seconds. A is the old detail API and B is the new summary API.

| Order | Condition | Time |
| --- | --- | --- |
| 01 | A | 1.063891 |
| 02 | B | 0.914431 |
| 03 | B | 0.910825 |
| 04 | A | 1.071487 |
| 05 | A | 1.037148 |
| 06 | B | 0.934206 |
| 07 | B | 0.898326 |
| 08 | A | 1.032138 |
| 09 | A | 1.057552 |
| 10 | B | 0.877733 |

### 2.2 Values per Run of the New Version After Restart

The unit is seconds. A is without the index and B reuses a fresh index.

| Order | Condition | Time |
| --- | --- | --- |
| 01 | A | 0.893018 |
| 02 | B | 0.262789 |
| 03 | B | 0.260598 |
| 04 | A | 0.895577 |
| 05 | A | 0.896064 |
| 06 | B | 0.258630 |
| 07 | B | 0.254658 |
| 08 | A | 0.917473 |
| 09 | A | 0.861713 |
| 10 | B | 0.257953 |

### 2.3 Values per Run of the UI

The unit of time is ms. A is `web-B2` and B is `web-C3`. The target requests count both the detail and summary paths.

| Order | Condition | DOM after response | Target row DOM | Max rows | Initial pages |
| --- | --- | --- | --- | --- | --- |
| 01 | A | 73.600 | 302.300 | 30 | 1 |
| 02 | B | 118.500 | 375.700 | 30 | 1 |
| 03 | B | 46.800 | 249.700 | 30 | 1 |
| 04 | A | 75.900 | 306.100 | 30 | 1 |
| 05 | A | 67.100 | 351.300 | 30 | 1 |
| 06 | B | 51.800 | 247.500 | 30 | 1 |
| 07 | B | 55.200 | 288.300 | 30 | 1 |
| 08 | A | 56.600 | 1058.600 | 30 | 1 |
| 09 | A | 74.800 | 274.200 | 30 | 1 |
| 10 | B | 65.700 | 279.600 | 30 | 1 |

## 3. Functionality and Verification

`cargo make test` passed 426 and skipped 6, and the fmt, clippy, and lint-style it depends on also passed. The difference in the published destination of DIND's PostgreSQL was handled by an existing investigation script that relays only our own test port, and we did not change the product or core-rs sources.
The web client passed 65 files and 505 tests, and typecheck, lint, and build also passed. 234 existing lint warnings remain. The whole test run was executed as planned with `maxWorkers=4` and `testTimeout=30000`. The timeout of the existing 100,000-item Timeline test at the default 5 seconds is kept as the [phase-one constraint](./2026-10-15-browse-phase-one.md#3-functionality-and-required-checks).

We confirmed listing sorting, filtering, and page consistency, archives and broken symlinks, non-UTF-8 and special Entries, sharing of budgets, cancellation, and the index's version, TTL, capacity, queue, expiry of stale work, and fallback on corruption. We also confirmed that unfetched details are not treated as 0 or writable, updates of the selection's attributes, partial failure, compatibility with the old daemon, and retry of details.
The old 36 paths and 76 schemas are unchanged, and 4 new paths and 7 new schemas were added. The success item name of the new detail result is `detail` and the failure item name is `error`. Child probes return 501 and `childProbes=false` until a later implementation.

With 13 items over real HTTP, we confirmed authentication, boundaries, rejection of counts and duplicates, partial failure, and capabilities. With Playwright CLI, we confirmed that the End key focuses `folder-22431`, that fetching the summary page at offset 22400 fits in 16 DOM rows, switching to the list, selection and info display, and suppression of changes when readonly. The 27 console messages were only thumbnail 404s of empty folders.

A different model, Claude Sonnet 5.5, verified the implementation and functional checks of S-1 to S-3 in read-only mode and judged `pass`. The working tree before and after matched at `812f2853e17b510588db207af0b2f8356524a665`. This code verification does not include this document's final performance measurement. The final performance measurement was separately re-aggregated in read-only mode and judged `pass`. The working tree before and after verification matched at `4a33380a1ff457860608c586d778f483100ddff3`.

## 4. Reference Check on the Real Data Side

In a run that checked the VirtioFS of `/mnt/work02/web/twitter` by reading only, `getdents64` provided the kind for all 22,433 items, and the number of unknown kinds was 0. With 22,432 directories and 1 file, 213 calls of `getdents64`, and reading and aggregation in Python, it took about 53 ms. Names were not saved.
This is a reference check that the kind hint can be used for the current target. It is not a value showing a comparison with individual stat calls, the time of sorting, the API, or the screen, a cold state, or an improvement factor. The real data is updated, and other load and caches are not controlled.

## 5. Limits

This is fixed data on ext4 centered on empty folders, and differs from the load of real data on VirtioFS or of content processing of images and videos. The OS and host cache layers are not isolated. No extrapolation is made to the state of about 165 seconds or to performance during background scanning.
The representative selection cache, the delay of folder thumbnails, reserved foreground and background slots, and child probes of visible rows only are outside this document, and will be implemented and verified in later plans.
The external data of the pre-fix `daemon-C` remains, but only the values of the final `daemon-D` and `web-C3` above were used for acceptance. The first failure of the UI measurement tool was a syntax error in an expression evaluated inside the browser, before JSON output. We fixed the tool, ran all 10 runs, and also kept the failure log.
