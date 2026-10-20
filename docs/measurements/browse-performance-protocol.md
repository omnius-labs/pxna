# Listing Performance Comparison Procedure

Back to the [measurement list](../measurements.md).

## 1. Subject and Acceptance Conditions

Using fixed data that contains 22,432 empty folders and an empty `_meta.db`, compare before and after a change under the same build conditions.
The targets below are fixed before measuring after the change. This is a separate experiment from the real-data numbers of 2026-10-14 to 15, and the same reduction ratio is not extrapolated to real data.

| Subject | Acceptance condition |
| --- | --- |
| Tree generation volume | With a 1440 × 900 viewport, the DOM rows for the visible range plus margin are 200 or fewer. They do not grow in proportion to the 22,432 items |
| Initial tree fetch | The target level requests 2 pages or fewer before the first display. The first rows can be displayed and operated even if later pages do not return |
| Tree time | With a snapshot present, the median from the response of the first required page to the DOM observation of the target row is 500 ms or less |
| Attribute fetching | On the fixed data with an empty in-process snapshot, the median of the first API call is 75% or less of before the change. Each run and the range are also reported |
| Revisit retention | The increase in the median of the cached listing API is within the larger of 20% of before the change and 5 ms |
| Features and limits | Regression checks pass for the attribute and error conventions of all Entries, page consistency, operations, cancellation, the number of concurrent attribute-fetch batches of the shared local driver (4), the number of admitted streams (16), and the prefetch count of each stream (64 items × 2 batches) |

If a target is not reached, record it as not met, fix the approach, and measure again. When variation is large and a difference between approaches cannot be judged, it is not used as a basis for adoption either.
The phase-two targets are fixed before measurement in each implementation plan.

## 2. Fixed Data

Data the tools create is confined to `AGENT_TEMP_DIR`. Existing directories are not overwritten.

```sh
python3 tools/performance/fixture.py "$AGENT_TEMP_DIR/pxna-browse-fixture"
python3 tools/performance/fixture.py "$AGENT_TEMP_DIR/pxna-browse-fixture" --verify
```

The Library path is the generated `library`, and the display target is `/twitter`. The manifest is outside the Library and does not get mixed into listings.
Check the hashes of the generated names, kinds, and counts, and the fixed modification times. Record the mount type of the generation destination with `findmnt -T "$AGENT_TEMP_DIR/pxna-browse-fixture"`.
This data is for measuring the cost of file system attribute fetching, and it does not reproduce image processing or a real permission setup.
Boundaries such as symlinks, special Entries, and permission errors are verified separately in the driver's regression tests.

## 3. Fixing the daemon and Conditions

Start the daemon with a dedicated port, dedicated configuration, and dedicated state, and make the Library read-only. Do not reuse a production daemon or state.
The web client serves a production build of the same source, and record the Rust toolchain, build profile, commit and diff identifiers, browser version, and viewport.
Use Rust 1.96.0 as specified by the repository, and do not change the profile or instrumentation during the comparison.

Measure the first API call only once, immediately after restarting the daemon, with the in-process snapshot empty.
Do not delete the OS and VirtioFS caches. Do not call a state cold merely because of a restart or a new path; record the state as unknown.
Before UI measurement, read the target listing once to build the snapshot.

`scan_interval_secs = 0` alone cannot stop the startup scan of the empty timeline index. For the no-background condition, confirm the completion of the dedicated daemon's scan with the status API and the log before measuring.
Do not assume the load from other processes is the same, and record the background state and free capacity during measurement.

With before the change as A and after as B, measure 5 runs each in the order `A B B A A B B A A B`. Align the start, wait, and measurement steps of each run.
Save the first API call, the cached API, and the UI as separate values. When adding a comparison with and without background, run the same procedure alternating per condition.
Even without background, the previous measurement changes the file system cache and this cannot be avoided, so it is not treated as complete cache isolation.
Checks on real data are kept apart as reference values unless the manifest can be fixed across updates.

## 4. API and Section Measurement

Pass authentication with the environment variable `PXNA_PERF_TOKEN`. Do not put tokens or authentication responses in record files or command arguments.
The URLs below are an example in which the dedicated daemon's Library id is `perf`.

```sh
python3 tools/performance/api.py --url 'http://127.0.0.1:16051/api/v1/libraries/perf/folders/entries?path=%2Ftwitter&limit=200' --label A-1 --snapshot-state empty --output "$AGENT_TEMP_DIR/api-A-1.json"
python3 tools/performance/api.py --url 'http://127.0.0.1:16051/api/v1/libraries/perf/folders/entries?path=%2Ftwitter&limit=200' --label A-1-cache --snapshot-state reuse --runs 5 --output "$AGENT_TEMP_DIR/api-A-1-cache.json"
```

Expect `total_count = 22433`, a returned count of 200, and `truncated = false` for both the first and the cached responses.
`empty` is a condition the caller guarantees by restarting, and the tool does not clear the server's cache.
The API tool saves the elapsed time, the response count, the total count, the snapshot, and the HTTP status, and does not save actual Entry names. It does not overwrite existing result files.

Enable JSON output with `pxna::performance=debug` in the logging filter. The measurement points kept in the implementation are as follows.

| operation | Metric and boundary |
| --- | --- |
| `local_listing` | Enumeration, the count and wait time of `symlink_metadata`, and the elapsed time of the whole stream. The elapsed time includes the consumer's processing |
| `listing_scan` | The returned count, the conversion and name normalization time, the whole scan, and truncation. Async spans do not use manual enter guards across an await |
| `listing_sort` | The count and the elapsed time of sorting |

Section events are emitted on success after the stream is read to the end. The absence of a success event on failure or cancellation is not interpreted as zero time.
The API, the whole scan, and the whole driver stream are nested, so they are not added together. Attribute fetching means the query of the symlink itself, and does not include the query of the target's kind or reading the link.
Even when the total attribute-fetch time is reported after parallelization, distinguish the total that overlaps across workers from the wall-clock elapsed time.

## 5. Browser

Browser operation checks use Playwright CLI, and the measurement tool uses a local CDP connection.
Start a dedicated Chromium with an empty profile and a remote debugging port. Put the profile in `AGENT_TEMP_DIR` and do not connect to a production browser.
Limit CDP to the loopback.

```sh
node tools/performance/browse.mjs --cdp http://127.0.0.1:9222 --url http://127.0.0.1:16051/browse/perf/twitter --label A-1 --output "$AGENT_TEMP_DIR/ui-A-1.json"
```

Do not use additional npm dependencies other than the browser and Node 24. The tool creates a dedicated tab and closes it on exit. It does not save authentication tokens.
The viewport is fixed at 1440 × 900 and the target row at `folder-00000`.
What is saved is the browser version, the viewport, the DOM observation of the target row, the number of DOM rows, the offsets and response times of listing requests, long tasks, CDP metrics, the time of the observation processing, and the round-trip time to the event loop.
DOM observation does not guarantee paint completion or that every operation is possible. Key operations, menus, editing, and dragging are checked separately with functional tests and Playwright CLI.
Also check that the time of the observation processing is not the main part of the difference.

## 6. Regression Checks and Saving Results

```sh
npm --prefix ui-web run test -- src/components/Tree.test.tsx src/library/tree.test.ts src/library/entries.test.ts
npm --prefix ui-web run typecheck
npm --prefix ui-web run lint
cd daemon && RUSTC_WRAPPER= cargo make test
```

Beyond the time comparison, pass the checks of the DOM row count at 22k items, the number of initial requests, page generation changes, key operations on off-screen rows, the number of attributes fetched, the number of batches executed by the shared local driver, the number of admitted streams, the upper bound on each stream's prefetch count, and failure and cancellation.
These limits do not restrict the number of accepted HTTP requests, admission-waiting futures, attribute checks of the root, recursion targets, or other storage operations. A budget that sums all requests is handled in S-4 / S-8 of the plan.
Save each run's raw time, median, minimum and maximum, counts, build and environment, background state, accept/reject, and unverified scope in a new dated record and JSON.
A re-measurement does not overwrite earlier records. Credentials and real-data Entry names are not saved.
This procedure does not stop a production daemon. If additional measurement requires stopping one, prepare a watchdog and resumption on exit or exception before stopping it.


## 7. Comparing Summary Listing and the Index

In phase two, `daemon-B` of phase one is fixed as the old version and the new implementation as `daemon-C`.
The first comparison alternates `/folders/entries?path=%2Ftwitter&limit=200` and `/folders/summaries?path=%2Ftwitter&limit=200`, on the same fixed data, toolchain, and after background scanning has ended, 5 runs each in the order `ABBAABBAAB`. Both use the default ascending name order and `foldersFirst=false`. The old version responds with details and the new version with names and kinds, so the amount of information already fetched differs.
For the new version's runs without the index, while stopped, set aside only the dedicated `state/listing/summary.db` and its WAL and SHM, and start from a state where both memory and the persistent index are empty. Do not set aside or delete the authentication DB or the timeline DB. In each run, measure 1 first request and 5 cached requests for the same listing.
The comparison of the persistent index alternates, 5 runs each, the same `daemon-C` without the index and the reuse of a fresh index after a restart. The index is created in advance with the new build, and the process is ended after `summary_persist` completes. For each reuse run, copy this original while stopped and run within the TTL. When the TTL has passed, create a new original. The OS and VirtioFS caches are not controlled and are not treated as a reproduction of cold.
Acceptance is: the median of the summary listing's first call is 90% or less of the old version, index reuse is 50% or less of the same new version without the index, and the increase in the cached median is `max(20% of the old version, 5 ms)` or less relative to the old version.
For the browser, alternate `web-B2` and the new version on the same `daemon-C`, and measure both paths. The measurement tool aggregates both the detail and summary listing requests. Read each API's snapshot once in advance so that the index and the initial listing scan do not mix into UI processing. Keep DOM at 200 rows or fewer, the target tree's initial pages at 2 or fewer, and the median DOM after the response at 500 ms or less. Record each run's browser version, screen width, build SHA-256, request path, and amount of information.

## 8. Comparing the Background Budget and Child Probes

Before the comparison, fix the conditions as follows. The subject is the same dedicated fixture of 22,432 empty folders and 1 file. Real-data scans and the running daemon 6050 are not used.

- The foreground comparison uses the phase-one daemon-B (4 workers) and the build after the background budget was introduced. Request the existing detail listing for direct children, name/asc, foldersFirst=true, limit=200. Restart a dedicated daemon for each run and empty the in-process snapshot.
- Separate the condition after the startup timeline has finished from the condition where running=true was observed on a manual refresh, and measure each 5 runs in the order ABBAABBAAB. Do not run the browser or other measurements during a foreground request.
- It passes when, for each condition, the new median minus the old median is `max(old median × 20%, 50ms)` or less. Do not loosen the target after seeing the values.
- A run during execution requires running=true before sending, and also at least 1 successful EOF event (`local_listing`, `entry_count=0`) within the time from the start of the request to the end of the response. processedFiles=0 of the empty fixture is not evidence of progress. A run that does not meet the condition is kept with its reason and not mixed into the comparison.
- The reuse of the index after a restart compares the summary listing's final build daemon-D and the new build, 5 runs each in ABBAABBAAB. Copy the same fresh index DB generated just before into each run, and confirm that the same snapshot id as the seed is returned and that there is no summary_scan of 22,433 items. It requires the new median minus the old median to be `max(old median × 20%, 50ms)` or less.
- The OS, host, and VirtioFS cache layers are not controlled. Alternating the order is not treated as having removed the cache's contribution. Do not extrapolate this fixture's values to the wait when the foreground piggybacks on the background single-flight, or to heavy-content videos and whole archives.

For each run, after recording the start of the dedicated daemon and the end of the startup scan, use the following command. For quiet, change to `--condition quiet`. The token is read from the environment variable and not saved to the output.

```sh
python3 tools/performance/background.py \
  --origin http://127.0.0.1:16051 --library perf --path /twitter \
  --log-file "$AGENT_TEMP_DIR/pxna-browse-run/background-run.log" \
  --fixture-dir "$AGENT_TEMP_DIR/pxna-browse-fixture" \
  --condition active --snapshot-state empty --label budget-new \
  --output "$AGENT_TEMP_DIR/pxna-browse-run/background-run.json"
```

Distinguish a name-only load from content-reading behavior. With a separate fixture of 4 small fixed PNGs and 1 ZIP containing 2 PNGs, also confirm that the dedicated daemon's timeline finishes and registers 6 items in the index. This is a functional check and is not used to measure the reduction ratio of heavy content processing. Record the counts, the PNG/ZIP hashes, and the build. Probing of visible rows is a separate browser check, confirming cancellation of off-screen requests and normal expansion.

## 9. Comparing Representative Selection and Folder Thumbnails

Before the change, fix a build in which only measurement points were added to the daemon after the background budget was introduced. Record the times of representative selection, image cache hits, and actual generation and cache saving, and keep the nesting between selection and generation. The generation section does not include the wait for a generation slot, and for the API, the time is measured until the body is read to the end. Do not infer the presence of the image cache from API time differences alone.

The positive fixture uses 22,433 direct children of 22,432 empty folders and 1 fixed 8×8 PNG, and the negative fixture uses the previous 22,432 empty folders and 1 empty file. Fix the names, contents, and mtimes, and do not change the existing 22k fixture. The positive Library root for UI checks holds the same PNG, a ZIP containing it, and the target folder, 3 items in total. No new external media tools are introduced, and video prefetching is verified in the web client's deterministic tests.

Separate the first request, the request with only the image cache, the request that also uses the representative selection cache, and the no-candidate case. Each request requires exactly 1 representative selection event in the JSON log. The hash and ETag of the HTTP 200 image must match before and after. The no-candidate case is 404 and is not mixed with the image cache measurement.

Acceptance is decided by alternating before and after the change, 5 runs each, in ABBAABBAAB. In each run, restart a dedicated daemon and wait for the startup background scan to finish. After recording the first request with the selection cache empty separately, use the next request in the same process for acceptance. Create the image cache in advance with the same 128×128/crop request, and confirm hit=true for the positive acceptance target. It requires that the API median is 25% or less of before the change and that entry_count=0 on a selection cache hit in the new version. Do not loosen the target after measuring. Save each run's values for the first request and the negative request too. The OS and host caches are not controlled, and no claim is made about cold or a ratio on real data.

```sh
python3 tools/performance/thumbnail.py \
  --origin http://127.0.0.1:16053 --path /twitter \
  --log-file "$AGENT_TEMP_DIR/pxna-browse-run/thumbnail-run.log" \
  --label thumbnail-new --expected-status 200 \
  --output "$AGENT_TEMP_DIR/pxna-browse-run/thumbnail-run.json"
```

For the browser, use `browse.mjs --thumbnails --marker twitter` with the fixed 3-item root, and record the first observed tile DOM and the start of the thumbnail request separately. Do not leave paths or tokens in the output. Confirm that folder/archive requests do not go out less than 200 ms after each tile's DOM, that image requests are immediate, and that off-screen folders/archives are not sent. A request's ResourceTiming appears after the response, so the dedicated observation waits 2 seconds after the DOM arrives and also confirms that all 3 fixed requests were recorded.
Separately, confirm DOM of 200 rows or fewer and DOM after response of 500 ms or less on the 22k listing. The DOM is not equated with paint completion. The timer at 199/200 ms, no postponement on redraw, cancellation off-screen and on dispose, and image/video prefetching are verified with the web client's fake timers.

## 10. Measuring Display Image Latency

This isolates the sections in which the viewer's display image is slow. It is not a comparison, but a measurement of per-section values of the daemon before the change. It does not include countermeasures.

Create the fixed data with `python3 tools/performance/display_fixture.py "$AGENT_TEMP_DIR/pxna-display-fixture"` (check with `--verify`). It duplicates a synthetic 6000 × 4000 JPEG (about 13.5 MB) and places the copies in local `/local` (10 images) and `/flip` (20 images), and inside the uncompressed ZIP `/arch.zip` (30 images). ffmpeg is used to generate the images, so record the ffmpeg version in the manifest. The Library id is `perf`, exposed read-only.

The dedicated daemon includes `pxna::performance=debug` in the log and uses JSON output. Measure after confirming the end of the startup timeline aggregation with `running=false` of `GET /libraries/perf/timeline/status`. Restart the daemon for each unit of measurement and empty the display image cache.

The sections are defined as follows. For each `--case`, the tool saves the HTTP response time and the log's measurement points.

| Section | `--case` | What is measured | Discrimination |
| --- | --- | --- | --- |
| First generation | `initial` | The first display image request for each of 5 images in local and in the archive (from a cache miss until the body is read to the end) | `thumbnail_image_cache` is a miss. Generation (`thumbnail_generation`) is split into extraction (`thumbnail_materialize`), reading and conversion (`thumbnail_convert`), and saving (`thumbnail_store`) |
| Cache hit | `cached` | Re-request the same 5 images 5 times each | `thumbnail_image_cache` is a hit. Confirm that all are hits |
| Swapping to the original image | `cached` | Fetch `files/content` of the same 5 images 5 times each | The time until the body is read to the end. Browser decoding and painting are not included |
| Consecutive flipping | `flip` | Advance through 10 images in order. Opening an image requests the display images of that image and the one before and after it (the same as the viewer's prefetch). The operation intervals of 1000 ms and 300 ms are measured on different sets of 10 images | The wait at each step (from the step's time until the display image arrives) and the request's duration. The wait for a slot for generation is seen from the difference between `thumbnail_generation` and `thumbnail_production` |

The request size defaults to a 1440 × 900 screen with a pixel density of 2, and is sent with a width of 2880, a height of 1800, and `resize=fit`, the same as the viewer's `displayRequestSize`. The next operation is performed after the current image arrives and at or after the scheduled time.

On each daemon start, run `initial`, `cached`, and `flip` in this order, and repeat by restarting 3 times. There is no other version to compare, so they are not alternated.

```sh
python3 tools/performance/display.py --origin http://127.0.0.1:16061 \
  --case initial --log-file "$AGENT_TEMP_DIR/pxna-display-run/r1/daemon.log" \
  --label D1-r1-initial --output "$AGENT_TEMP_DIR/pxna-display-run/r1/initial.json"
```

Run `--case cached` and `--case flip` in the same form. Pass authentication with the environment variable `PXNA_PERF_TOKEN`. Browser decoding and painting and real network lines are not measured. When browser-side time is needed, read the display image and original image requests separately with Resource Timing.
