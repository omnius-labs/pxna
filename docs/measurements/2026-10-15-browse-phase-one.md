# Performance Comparison for Phase One of Listing Display

Back to the [measurement list](../measurements.md).

## 1. Subject and Conditions

On 2026-10-15 (UTC), using the fixed data of the [comparison procedure](./browse-performance-protocol.md), we compared the staged display and virtualization of the tree and the batching of the local driver's attribute fetching.
The baseline commit is `4987dd4f9d4f04ce6e04d20ede14312c88b37ba1`. A is before the change with measurement points added, and B is the final build of phase one.
The SHA-256 of the builds, each run's values, and the hashes of the source files are saved in the [aggregate JSON](./2026-10-15-browse-phase-one.json). The [comparison of the initial tree implementation](./2026-10-15-tree-virtualization.md) is kept as a separate record.

| Condition | Setting |
| --- | --- |
| Fixed data | 22,432 empty folders and 1 empty `_meta.db`. Names, kinds, counts, and modification times checked against the manifest |
| Storage | Dedicated temporary area, ext4. No writes to real data |
| daemon | Rust 1.96.0, dev build, dedicated port and state, read-only Library |
| Browser | Chrome 155.0.8059.79, 1440 × 900, dedicated profile, new tab for each run, HTTP cache disabled |
| Order | UI and API each independently `A B B A A B B A A B`, 5 runs each |
| UI snapshot | Read the listing in advance with the common pre-change daemon, and compare the tree only |
| API snapshot | Restart the daemon each run, 1 first call, then 5 cached calls immediately after |
| Background state | Measured after confirming the dedicated daemon's timeline aggregation had finished. Load from other processes not controlled |
| OS and FS caches | Not deleted, and the layers are unidentified. A condition that repeatedly references the same path. Not treated as cold |

The UI and the API are separate experiments, and adding both is not the total time for the screen to open.
This is not a measurement that reproduces the real data on the original VirtioFS or the state of about 165 seconds for the first call.

## 2. Results and Acceptance

The UI and the first API are medians of 5 runs each, and the cached API is the median of 25 requests each.
The time after the response shows the difference from the first tree response of the target level until the DOM of `folder-00000` is observed.

| Metric | A: before | B: after | Accept/reject |
| --- | ---: | ---: | --- |
| From page navigation to DOM observation of the target row | 19.486 s | 0.269 s | Reference value |
| DOM observation after response | 19.3104 s | 0.0634 s | Met the median of 500 ms or less |
| Maximum DOM rows | 22,434 | 30 | Met 200 or fewer in all runs |
| Tree requests for the target level | 113 | 1 | Met 2 or fewer in all runs |
| ScriptDuration | 17.684 s | 0.243 s | Reference value for the whole browser |
| First listing API | 4.0469 s | 1.0903 s | 26.94% of A. Met 75% or less |
| Cached listing API | 9.937 ms | 9.774 ms | Met the allowed increase within 5 ms |
| Whole listing scan | 3.3215 s | 0.3785 s | A section included in the API |
| Building the order index | 0.6800 s | 0.6975 s | A section included in the API |

The range of DOM observation after the response was 18.8497 to 19.6753 seconds for A and 0.0575 to 0.0682 seconds for B.
The range of the first API call was 3.5284 to 4.2530 seconds for A and 1.0467 to 1.1400 seconds for B.

The predefined performance conditions for phase one were met, so the changes to the tree and attribute fetching are adopted.
The duplicate request at offset 0 that remained in the initial tree implementation was not observed in the final build.
In the first API call after batching, building the order index took about 0.70 seconds. It is a cost that remains even if only attribute fetching is reduced, and it is handled in phase two.

### Values per Run

The UI and API in the same row are independently measured values and do not represent a simultaneous load condition. The cached column is the median of that run's 5 requests.

| Run | Version | UI after response (s) | DOM rows | Tree requests | First API (s) | Cached API (ms) |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| 1 | A | 19.6160 | 22434 | 113 | 4.0469 | 10.316 |
| 2 | B | 0.0682 | 30 | 1 | 1.0587 | 9.424 |
| 3 | B | 0.0575 | 30 | 1 | 1.1400 | 9.129 |
| 4 | A | 18.9406 | 22434 | 113 | 4.0104 | 9.375 |
| 5 | A | 19.3104 | 22434 | 113 | 3.5284 | 9.937 |
| 6 | B | 0.0672 | 30 | 1 | 1.0974 | 9.774 |
| 7 | B | 0.0625 | 30 | 1 | 1.0467 | 11.963 |
| 8 | A | 19.6753 | 22434 | 113 | 4.2530 | 9.595 |
| 9 | A | 18.8497 | 22434 | 113 | 4.0859 | 9.586 |
| 10 | B | 0.0634 | 30 | 1 | 1.0903 | 9.093 |

## 3. Functionality and Required Checks

| Check | Result |
| --- | --- |
| Local driver tests | 16 passed. Includes the attribute count, the shared limit across multiple requests, and stopping processing after cancellation |
| Whole Rust | `cargo make test` passed 409 and skipped 6. The lint it depends on also passed |
| Web related | 33 passed, including the 22k-item DOM, the End key, unsubscribing hidden levels, and existing operations |
| Web types, static checks, and serving build | typecheck, lint, and the production build passed |
| Whole web | 485 passed with parallelism of 4 and a timeout of 30 seconds. A whole run with the default 5 seconds timed out on the unchanged 100,000-item Timeline test. The 16 Timeline tests run alone passed |
| Real browser operation | Confirmed with Playwright CLI the move to the end, focus, and a DOM within the display range. The console error observed during the operation was the thumbnail 404 of empty folders |
| Implementation verification by a different model | Claude Sonnet 5.5 passed the phase-one code and the limit contract. The working tree before and after verification matched |

In the first run of the whole Rust suite, 3 unchanged PostgreSQL integration tests failed to connect.
The cause was that the test support code's connection target is fixed to `127.0.0.1`, and the published port of Docker-in-Docker is on a different network from the agent's container.
When we relayed only the published port of our own PostgreSQL test container during the run, everything passed with the same source. We did not change the product or core-rs sources for this connection.

The attribute-fetching budget is limited to 4 executing batches of the shared local driver, 16 admitted streams, and a prefetch of 64 items × 2 batches per stream.
It is not a combined upper bound on the number of accepted HTTP requests, admission-waiting futures, attribute checks of the root, recursion targets, or other storage operations.

## 4. Limits of Interpretation and Later Measurements

- This fixed data of empty folders does not reproduce image processing or a real permission setup. Do not extrapolate it to a reduction ratio on real data.
- A's `metadata_sec` is the time awaiting Tokio's attribute fetch and includes the cost of waiting and the task. B is the total of synchronous attribute fetches inside workers, does not include the wait to be admitted to a worker, and overlaps across workers. The reduction ratio of the syscall itself is not computed from either value. The whole API time was used for acceptance.
- The API, the whole scan, and the driver stream are nested, so they are not added. B's total attribute-fetch time and the wall-clock section are not added either.
- DOM observation does not guarantee paint completion or completion of all operations. ScriptDuration is also not the processing of the tree alone.
- This time we compared after background aggregation had finished. A paired comparison during background execution has not been done, and foreground priority and guaranteed background progress will be verified and measured separately in phase two.
- The failure of the whole run with the web's default timeout remains. Distinguish it from the all-pass with the extended timeout.
- Summary listing, the persistent browse index, the background budget, whether there are children, the representative image selection cache, and thumbnail delay are not included in the accept/reject of this phase one.
