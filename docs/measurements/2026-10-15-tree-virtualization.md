# Comparing Staged Display and Virtualization of the Tree

Back to the [measurement list](../measurements.md).

## 1. Subject and Conditions

On 2026-10-15 (UTC), we compared the web client's tree before and after the change with a fixed set of 22,432 folders.
The tree before the change, including the Library root and `twitter`, has 22,434 rows.
The baseline commit is `4987dd4f9d4f04ce6e04d20ede14312c88b37ba1`. For the production web client before and after the change and the common daemon, we used the builds whose SHA-256 is recorded in the [aggregate JSON](./2026-10-15-tree-virtualization.json).

Following the [comparison procedure](./browse-performance-protocol.md), we used the same daemon and data, took before the change as A and after as B, and measured 5 runs each in the order `A B B A A B B A A B`.
We compared with background aggregation finished and a listing snapshot present. We disabled the browser's HTTP cache and recreated the tab for each run.
Chrome was `155.0.8059.79`, the viewport was 1440 × 900, the destination was ext4, and the daemon was a dev build with Rust 1.96.0.
The OS and file system cache layers and the load from other processes were not controlled.

B in the table is the build of the tree's initial implementation. The final source, which includes the later fixes for unsubscribing hidden levels and switching subscriptions, will be measured again separately.

This comparison did not change the daemon's attribute fetching. It does not show the wait of the API for a new snapshot, or the reduction ratio of the total time to open real data on the original VirtioFS.

## 2. Results

Counts without units and times are all medians of 5 runs.
"After response" is the difference from the first response observed to the target level's tree request until the DOM of `folder-00000` is observed.

| Metric | Before | After |
| --- | ---: | ---: |
| From page navigation to DOM observation of the target row | 21.398 s | 0.319 s |
| DOM observation after response | 21.211 s | 0.094 s |
| Maximum DOM rows of the tree observed | 22,434 | 30 |
| Tree requests for the target level | 113 | 2 |
| Distinct pages fetched | 113 | 1 |
| ScriptDuration of the whole browser | 19.810 s | 0.291 s |
| The DOM observation processing itself | 14.7 ms | 0.4 ms |

The range of time after the response was 20.772 to 23.945 seconds before the change and 0.085 to 0.151 seconds after.
Both of the 2 requests after the change are at offset 0, and it no longer waits for all pages to be fetched. One duplicate request at initial display remains.
DOM observation does not guarantee paint completion or that every operation is possible, and ScriptDuration is not the processing time of the tree alone.

The procedure's targets were met: DOM rows of 200 or fewer, initial requests of 2 or fewer, and a median after response of 500 ms or less.

## 3. Functional Checks and Limits

The 33 related tests, including existing operations and the added 22k-item regression checks, the TypeScript check, web lint, and the production build passed.
The added checks confirmed that the first rows can be operated without waiting for slow later pages, and that the End key moves directly to the last page without fetching all the pages in between.
jsdom cannot compute layout, so the regression tests fix only the virtualization library's viewport and scroll.
In a real browser as well, we confirmed with Playwright CLI the move to and focus on the last `folder-22431`. In a different small viewport as well, the DOM row count stayed within the display range.
We observed no console errors during this operation other than the 404 returned by thumbnails of empty folders.

When the whole suite ran in parallel, the unchanged test that reads additional 100,000 timeline items exceeded the default 5-second limit. Running the same test alone passed. When the whole suite was run with parallelism of 4 and a timeout of 30 seconds, all 63 files and 485 tests passed. We could not confirm success of a parallel run with the default 5-second limit in this environment.
We do not omit this constraint from the report that all verification of the tree change is complete.

When an explicit move to a deep path shows an ancestor whose position is not yet known, it searches pages in order until that ancestor is found.
This is distinct from prefetching all siblings in normal expansion or scrolling. An API that gets a listing position from a path was not added this time.
