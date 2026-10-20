# Measuring the Listing of a Large Number of Folders

## 1. Subject of the Record

On 2026-10-14 to 15 (UTC), we investigated the delay before `/browse/web/twitter` opens.
The target commit is `4987dd4f9d4f04ce6e04d20ede14312c88b37ba1`. All measurements are from before any performance improvement was applied.
The subject is a daemon running in Docker on Ubuntu and headless Chromium, with the data mounted over VirtioFS.

| Document | What it covers |
| --- | --- |
| This document | Measurement conditions, observed values, the current state confirmed in code, limits of the judgment |
| [Aggregate data](./2026-10-15-browse-performance.json) | Aggregate values, per-run values of attribute fetching, hashes of the sources |
| [Measurement record list](../measurements.md) | Entry to the other measurement records |
| [plans.md](../plans.md) | Work and progress that makes the response candidates concrete |
| [Listing and Search Design](../design/listing.md#4-snapshots) | The current contract for pages and freshness |

## 2. Method and Conditions

The listing API was measured in a measurement process separate from the existing daemon. The configuration was a read-only Library referring to the same real data, and the measurement process's own timeline scan was stopped.
The daemon for section measurement was a dev build with Rust 1.96, a copy of the original source with only time measurement added.
"First" means the state in which the measurement process was restarted and the listing snapshot was empty. It is not a state with the OS or VirtioFS caches emptied.

In the background-processing comparison, we paused the whole existing daemon that had started a timeline scan, and resumed it afterward. In the re-measurement of 2026-10-15, the pause lasted 44 seconds.
We checked the index state, for which the scan start log and completion time were not recorded, but we did not separately measure the CPU, disk, and file system waits of the scan alone.
We did not confirm whether external batches, such as fetching Twitter data, were running.

The UI was measured with a listing snapshot present. We recorded the last response time of Resource Timing, the DOM observation time of MutationObserver, Long Tasks, and ScriptDuration of the Chrome DevTools Protocol.
The observation thresholds were more than 1000 elements directly under the tree for the full display and more than 100 for the control, and the final node counts were 22,436 and 204 respectively.
The time of the observation processing itself was 1 to 1.4 ms for the full display.
The control limited only the browser's tree responses to 200 items, and did not change the backend's scan processing or the source data.
In the re-measurement, both the full display and the control were run while the existing daemon was paused. In the earlier UI measurement, the control's background load was not the same.

The attribute-fetching approaches were compared inside a separate measurement program with Rust 1.99 / Tokio 1.52.3.
We chose a fixed 1,000 items from 22,433 items sorted by name, and measured the same set 3 times each, changing the order of approaches.
The comparison covers sequential `tokio::fs::symlink_metadata`, blocking tasks in units of 128 items, a single task processing all 1,000 items, and 2, 4, and 8 blocking workers.
This is not performance with a countermeasure applied to the real daemon.

## 3. Listing API

The first target was 22,364 items, and afterward 22,433 items.
"With background" is the condition with the existing daemon running, and "Paused" is the condition with that whole daemon stopped.

| Condition | First measurement | Re-measurement | Attribute fetching in re-measurement |
| --- | ---: | ---: | ---: |
| With background, no snapshot | 164.98 s | 86.10 s | 84.04 s (97.6%) |
| Paused, no snapshot | 17.70 s | 9.32 s | 7.99 s |
| With background, with snapshot | 0.044 s | 0.023 s | No refetch of all items |
| Paused, with snapshot | 0.010 s | 0.0095 s | No refetch of all items |

In a separate auxiliary measurement without background processing, the first API call took 9.20 seconds.
The first "180 seconds" is not a completion time but the value at which the client-side measurement was cut off.

The exclusive breakdown of the first 164.98 seconds is as follows. The wait time of the whole stream includes attribute fetching and so on, so it is not added on top.

| Section | Time |
| --- | ---: |
| Attribute fetching | 161.439 s |
| Directory enumeration | 1.275 s |
| Data conversion and name normalization | 0.973 s |
| Sorting | 0.514 s |
| Other and communication | 0.777 s |

In an auxiliary Python measurement as well, attribute fetching for the same 22,364 items took 60.48 seconds the first time and 4.75 seconds immediately after. This is not a measurement of the API, and the original values were transcribed from the process output.
In a separate sample of 2,000 items, of 15.79 seconds, 15.57 seconds was inside the `std::fs::symlink_metadata` calls, and the waiting for and returning from the blocking task totaled about 0.22 seconds.
These show the variation of wait times by environment, but are not evidence that identifies which cache layer is effective.

## 4. Left Tree

The numbers are the time elapsed since opening the page, and only the after-communication row is the difference from the last response.
Each UI measurement targeted 22,433 items, of which 22,432 were folders. There were 113 pages of tree requests, and 22,436 nodes were generated including the upper nodes.

| Observation point | Earlier measurement | Re-measurement |
| --- | ---: | ---: |
| DOM generation of the first item in the center | 0.637 s | 0.475 s |
| Completion of communication for all tree pages | 2.148 s | 1.839 s |
| DOM observation of the tree | 25.221 s | 20.882 s |
| From communication completion to DOM observation | 23.073 s | 19.043 s |
| End of the last observed long task | 26.915 s | 22.211 s |
| CDP ScriptDuration | 23.359 s | 19.244 s |
| DOM observation after communication of the control limited to 200 items | 0.124 s | 0.067 s |

In the re-measurement, there was a 19.041-second long task immediately after the last response, followed by another 1.329-second long task.
The DOM observation time does not guarantee the time at which painting finished or the page became operable.
We could confirm that the processing of a large number of nodes continues after communication completes, and the difference from the control with a limited display count.
However, we do not equate the whole time of the long tasks with pure React rendering time.

## 5. Attribute-Fetching Approaches

Medians of 1,000 items per run, 3 runs per approach. The unit is seconds.
The earlier "Paused" measurement was a condition without the background daemon, and the re-measurement was a condition with the existing daemon paused.

| Approach | Paused, earlier | Paused, re-measurement | With background, earlier | With background, re-measurement |
| --- | ---: | ---: | ---: | ---: |
| Await one at a time | 0.333 | 0.333 | 0.338 | 4.974 |
| Batch in units of 128 | 0.195 | 0.193 | 0.202 | 0.219 |
| 1,000 items in a single task | 0.192 | 0.197 | 0.211 | 0.654 |
| 2 in parallel | 0.107 | 0.101 | 0.140 | 0.226 |
| 4 in parallel | 0.073 | 0.071 | 0.085 | 0.757 |
| 8 in parallel | 0.063 | 0.066 | 0.070 | 0.940 |

In the paused comparison, we could reconfirm the difference between batching and a small amount of parallelism.
In the re-measurement with background, individual values fluctuated widely from 0.071 to 7.585 seconds, and merely swapping the execution order could not control the temporal variation of the load.
The ranking of approaches or the optimal degree of parallelism cannot be decided from the medians alone, and we also cannot estimate that the 165-second API would shrink by the same factor.
The effect of parallelization can include concurrent processing on the VirtioFS side.

## 6. Current State Confirmed in Code

| Subject | Current state and basis |
| --- | --- |
| First listing | [LibraryBrowser::list_folder_entries](../../daemon/entrypoints/daemon/src/service/library.rs#L59) returns a page after the scan is complete. [ListingEntries::scan](../../daemon/entrypoints/daemon/src/service/listing.rs#L128) processes all direct children even when non-recursive |
| Attribute fetching | [FileStorageLocal::find_entries](../../daemon/modules/engine/src/core/nfs/driver/local.rs#L97) awaits each Entry's `symlink_metadata` sequentially |
| Listing cache | [The default of ListingConfig](../../daemon/entrypoints/daemon/src/config.rs#L241) has a maximum age of 300 seconds. It is not a persistent index, and keeps completed listings in memory |
| Tree | [childFoldersQuery](../../ui-web/src/library/entries.ts#L83) collects all pages before returning, and [Tree](../../ui-web/src/components/Tree.tsx#L115) generates all nodes without virtualization |
| Whether there are children | The [local driver](../../daemon/modules/engine/src/core/nfs/driver/local.rs#L138) returns `has_children: None`. [isExpandable](../../ui-web/src/library/tree.ts#L37) treats a node as non-expandable only when it is false. The check for grandchildren is already omitted |
| Background scan | [TimelineService::start](../../daemon/entrypoints/daemon/src/service/timeline.rs#L115) starts aggregation at startup and periodically. We confirmed the start log at measurement time and the incomplete index state |
| Representative thumbnail | [Representative image selection](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L284) scans all direct children of the folder. [Request handling](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L205) checks the image cache after selecting the representative. The time it contributes to the delay is unmeasured |

## 7. Judgment and Limits

The confirmed sources of delay are the all-item attribute queries of the listing API and the processing of a large number of nodes after the left tree fetches all items.
We observed short values with the background daemon stopped, but the measurement order was fixed from with-background to paused.
Order effects such as the OS metadata cache are confounded, and the contribution of background load could not be isolated.
Which of disk waits, VirtioFS round trips, CPU, or the cache layers dominates is also undetermined.
A process's `read_bytes` is a metric of physical reads and does not directly represent waiting for attribute queries to the file system.

The fixed values for screen width and browser version were not kept in the original measurement record.
The source data was updated and the counts differ between the first and later runs, so the numerical differences are not treated as implementation improvements.
Free capacity had dropped to about 432 MiB in the earlier measurement and about 387 MiB in the re-measurement. We did not measure its effect.

The response candidates and the matters to settle before implementation are referenced from [plans.md](../plans.md).
