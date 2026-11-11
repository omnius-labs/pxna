# First Aggregation and Search of the Name Index at One Million Entries

Back to the [measurement list](../measurements.md).

## 1. Subject and Conditions

2026-11-07 (UTC). We measured what the name index adds to the first full aggregation of a Library, how large its file is, and how fast recursive searches from the Library root answer from it.
The plans ask for this on a 10-million-Entry fixture (`name-index-store` condition 6 and `name-index-search` condition 6). **This record is at about 1 million Entries, not 10 million**: the machine was shared and had 48 GiB of disk free, and one first aggregation took five to six minutes at this size. What changes with the size is read in section 4.

- Fixture: made by a generator script that is not kept in the repository, with 1,000,000 Entries asked for and a fixed random seed: 999,998 Entries, 90,908 folders and 909,090 empty files with Japanese and ASCII names, about 1% of the files with image extensions. The folders are at most six deep. The search measurement added 800 files with rare 2-character terms to it, and the final measurements below all ran on the fixture of 1,000,798 Entries.
- A build: the daemon of commit `8558d87` (the commit before the change), `cargo build --release -p omnius-pxna-daemon`, sha256 `41d15f93efc9658f438533d4623e824d9bc1b976e303b48a2831bcec00765764`.
- B build: the worktree of the change before it was committed, with the same command, sha256 `a192395e1a83accdb92b3a8b639b3bbc7b4243339abca81311be45c82feeb457`. It was built after the last review fixes except one: a later edit makes the folders that `NameIndex::rename` creates for a moved Entry carry the generation of the running aggregation, which no run of this record exercises.
- Profile: `release` (`debug = 1`).
- Environment: Debian GNU/Linux 13 (trixie) x86_64 in a container (overlayfs on the host's disk), Intel Core i7-8700K, 9 visible cores, 30 GiB of memory, Rust 1.99.1. The OS file cache was not controlled, and other work on the machine was not controlled.
- Aggregation: a script that is not kept in the repository started a daemon on a fresh state directory, so the run is the Library's first full aggregation, and took the time between the log lines "scan started" and "scan completed". The order was ABBAAB. The ffmpeg tools were replaced by stubs, because no file in the fixture is read as an image or a video.
- Searches: a script that is not kept in the repository ran one aggregation, registered an owner, and timed 20 searches from the root for each class over the HTTP API. Each search has a term of its own. The B build is used.

## 2. Aggregation and File Size

Seconds for the first full aggregation of 1,000,798 Entries.

| Run | Build | Elapsed (s) | `names.db` (bytes) | Peak resident (MiB) |
| ---: | --- | ---: | ---: | ---: |
| 1 | A | 349.2 | 0 | 49.1 |
| 2 | B | 352.0 | 167,006,208 | 60.3 |
| 3 | B | 350.9 | 167,006,208 | 58.4 |
| 4 | A | 329.5 | 0 | 48.0 |
| 5 | A | 329.6 | 0 | 49.2 |
| 6 | B | 325.6 | 167,006,208 | 54.0 |

| | A | B | B / A |
| --- | ---: | ---: | ---: |
| Median elapsed (s) | 329.6 | 350.9 | 1.065 |
| Median peak resident (MiB) | 49.1 | 58.4 | +9.3 MiB |

The name index adds 6.5% to the first aggregation. Its file is 166.9 bytes per Entry (167,006,208 / 1,000,798), after a WAL checkpoint. The timeline index file was 28,262,400 bytes in every run.
Both conditions of the plan hold at this size (at most 1.20 times, at most 200 bytes per Entry). The first run of A was slower than the other two, which fits a cold file cache for the fixture; the medians do not depend on it.

### 2.1 Earlier Implementations

Two earlier builds were measured the same way, in the order ABBAAB, on the fixture of 999,998 Entries.

| Implementation | A median (s) | B median (s) | B / A |
| --- | ---: | ---: | ---: |
| Each folder written in a transaction of its own, one INSERT per Entry, before the walk goes on | 319.0 | 551.5 | 1.729 |
| The folders queued to one writer task that writes several in one transaction with multi-row INSERTs, while the walk goes on | 307.9 | 332.0 | 1.078 |

The first did not meet the plan and was replaced by the second. The second is the build before the fixes of the review (a lock around listing a folder, a restart of the index when the generation goes back, reading in batches of live matches, and the integrity check); the final build above includes them, and its ratio is 1.065. The raw runs are in the JSON file.

## 3. Searches

Searches from the root of the Library with the final B build, after one aggregation of 1,000,798 Entries (326.6 s). All 40 searches were answered from the name index (`indexed` true). Seconds.

| Class | Term | Matches | First page, p95 | First page, median | Following page, p95 |
| --- | --- | ---: | ---: | ---: | ---: |
| 3 or more characters | a run of 4 digits taken from an existing name | 249 to 330 (median 292) | 0.019 | 0.016 | 0.0022 |
| 2 characters | a rare pair of small hiragana | 40 | 2.813 | 0.388 | 0.0018 |

Pages are 100 Entries for the first class and 10 for the second. The first page includes building the snapshot, which reads the attributes of every match.
Both limits of the plan hold at this size (at most 1 second and 100 ms for the long terms, at most 5 seconds for the short ones). The slowest short-term search was the first one (2.85 s) and the others took about 0.39 seconds, which fits the first one reading the index from disk. The same measurement on the build before the review fixes gave the same figures (p95 0.023 s and 2.84 s).

## 4. What This Does Not Show

- It does not show the 10-million-Entry fixture. The aggregation time and the file size grow with the number of Entries, and the file is 167 bytes per Entry in this record, so at 10 million Entries it is about 1.7 GB.
- A search with only terms of 2 characters or fewer reads every Entry under the start ([Listing and Search Design §5.1](../design/listing.md#51-name-index)), so its time grows with the size. Ten times the Entries is about ten times the 0.39 seconds, and a cold start about ten times the 2.8 seconds, so the 5-second limit would not hold for a cold search at 10 million. Searches with a term of 3 or more characters are not expected to grow like this.
- The integrity check that the index passes before it answers (`PRAGMA quick_check`) took 2.6 seconds for the 167 MB file in a separate run with a warm cache. Until it has passed, searches walk, so this time grows with the file at startup.
- The fixture's files are empty. The aggregation time at a real Library, whose files are read for dates, is larger, so the share the name index adds is smaller.
- The searches have no rating or tag condition.
