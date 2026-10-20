# Timeline Design

## 1. About This Document

This document covers the server side of the timeline, which lists a Library's images and videos in the order of their timeline dates, and defines what is listed, how timeline dates are decided, the timeline index, aggregation, reading, and the reasons for adopting them.
The intended readers are developers and AI agents who implement or review this concern.
It assumes the vocabulary of [terms.md](../terms.md).
The definitions of words are not in this document.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | What is listed, timeline dates, the timeline index, aggregation, reading, configuration, reasons for adoption, deferred items |
| [design.md](../design.md) | Pxna's overall scope, where state lives, conventions for mutating APIs |
| [Client Design](./client.md) | The timeline screen and when pages are read |
| [Storage Layer Design](./storage.md) | Block-level extraction of archives, and the extraction path aggregation uses |
| [Listing and Search Design](./listing.md) | Folder listings and name search |
| [Video Playback Design](./video.md) | Media info for images and videos |
| [terms.md](../terms.md) | Definitions of terms |
| [issues.md](../issues.md) | Clear defects confirmed in code and handling policies |
| [daemon/openapi.yaml](../../daemon/openapi.yaml) | The authoritative timeline API paths, parameters, schemas, and status codes |

The entry point of the design document family is [design.md](../design.md).

### 1.2 Tense of This Document

§1 through §9 describe the completed form in the present tense.
The gap between the current code and the completed form, and the remaining work in dependency order, are gathered in §10.
If you want to know what works now, read §10 first.

## 2. Responsibilities and Boundaries

The timeline is a listing for viewing a Library's photos and videos in the order they were taken, without following folder hierarchy.
Folder listings and name search return the result of scanning the target after receiving a request ([Listing and Search Design §4](./listing.md#4-snapshots)).
The timeline is returned from the timeline index that the daemon has aggregated in advance.
If the whole Library were scanned on every request, the wait until the first page would be proportional to the size of the Library, and images and videos would have to be opened one by one to obtain the timeline dates (§9.1).

This document covers the span from scanning a Library, reading timeline dates, and reflecting them in the timeline index, to returning pages and per-month counts from the index.
The boundaries with other concerns are as follows.

| Matter | Authoritative source |
| --- | --- |
| The table of extensions used to determine images and videos | [Thumbnail Design §3](./thumbnail.md#3-determining-the-target) |
| Block-level extraction of archives, and the path that does not use the extraction cache | [Storage Layer Design §6.4](./storage.md#64-block-level-extract) |
| The rule of not following symlinks found during a walk | [Storage Layer Design §4.4](./storage.md#44-symlinks-and-special-entries) |
| How to read the Exif shooting date and time | [Video Playback Design §3](./video.md#3-media-info) |
| The daemon's own mutating operations, and the point at which they finish | [design.md §8](../design.md#8-conventions-for-mutating-apis) |
| Where the index lives, and the impact when it is lost | [design.md §9.2](../design.md#92-where-state-lives) |
| The timeline screen and when pages are read | [Client Design §3.11](./client.md#311-timeline) |

### 2.1 What Is Not Built

- Thumbnails are not made by aggregation.
  Aggregation reads only timeline dates, and thumbnails are made when requested, as before ([Thumbnail Design §2.1](./thumbnail.md#21-what-is-not-built)).
- The timeline index is not used for name search or listings.
  The index holds only images and videos, so it cannot produce search results that include folders and other files.
- A timeline spanning Libraries is not provided.
  By having a Library per Entry, determining changeability does not need to be split per Entry.
- External changes to files are not followed by notifications.
  External changes are reflected by the next aggregation.

## 3. What Is Listed

What the timeline lists is the Library's images and videos.
Whether something is an image or a video is decided with the same table of extensions as thumbnail targets ([Thumbnail Design §3](./thumbnail.md#3-determining-the-target)).
Folders, archives themselves, and other files are not listed.

Images and videos inside archives are also listed.
Nested archives are descended to `max_nested_depth`, as in browsing ([Storage Layer Design §7.2](./storage.md#72-defaults)).
For an archive that cannot be opened because it is password-protected, of unknown format, or exceeds limits, the Entries inside are not listed.

Hidden items (Entries whose names start with `.` and the Entries inside them) are not listed, the same as the listing default.
Symlinks found during the walk are not followed and not listed.

## 4. Timeline Dates

A timeline date is a date and time of the form `YYYY-MM-DDTHH:MM:SS` without a time zone.
For each Entry, the first value obtained in the order of the following table from the top is used.

| Order | Image | Video |
| --- | --- | --- |
| 1 | Exif shooting date and time | The container's `creation_time` that ffprobe returns |
| 2 | The Entry's modification time | The Entry's modification time |
| 3 | The modification time of the archive that contains the Entry | The modification time of the archive that contains the Entry |

The third applies only to Entries inside archives, and for nested archives, they are looked at in order from the inner archive.
Entries inside archives also have their shooting date and time read from their contents, as outside.
For an Entry whose extraction exceeded the limit or deadline, and an Entry whose contents could not be read, the values from the second onward are used.

The Exif shooting date and time is used as is, as a value without a time zone.
The modification time and `creation_time` are instants in UTC, so they are converted to a date and time in the time zone of `[timeline] time_zone` (§8).
When `creation_time` is the origin value that appears in files that did not write a time (exactly 0:00 on January 1, 1904 and January 1, 1970), it is treated as not having a `creation_time`.

The boundaries of dates and months are decided by the date part of the timeline date.
Because the server decides the boundaries, the same Entry is listed on the same day and month from any device.

Which value the timeline date was obtained from is held per Entry as the timeline date source.
For a timeline date obtained from a UTC instant, the original instant is also held.
The index records the time zone into which the timeline dates were converted.
At daemon startup, if the recorded time zone differs from the configured time zone, all timeline dates that have an original instant are converted again in the new time zone and the record is rewritten.
Files do not need to be reread, and after the time zone is changed, all Entries are still listed on the same basis.

## 5. The Timeline Index

The timeline index is placed in the SQLite at `state_dir/timeline/index.db`.
It holds only aggregated results, so even if it is lost, it returns to the same contents with the next aggregation.

The index holds the following.

| Contents | Use |
| --- | --- |
| Rows for Entries to list in the timeline (Library, Library-relative path, kind, timeline date, timeline date source, original instant, size, modification time, the local file it belongs to) | Reading pages |
| Rows per local image, video, and archive file (Library, Library-relative path, size and modification time for images and videos, `ArchiveIdentity` for archives, the last-seen aggregation generation) | Determining whether rereading can be skipped in aggregation, and cleaning up vanished files |
| The number of Entries per month (per Library) | The list of years and months |
| The aggregation state per Library (generation, time the last aggregation finished) | Cleanup and the timing of periodic aggregation |
| The time zone the timeline dates were converted into | Reconversion when the configuration is changed |

The Library-relative path of an Entry inside an archive is held as the archive's path joined with the inner path ([design.md §11.1](../design.md#represent-paths-inside-archives-by-joining-paths)).
The local file it belongs to is the outermost archive that contains that Entry, or the Entry's own local file.

The per-month counts are increased or decreased in the same transaction that adds or removes Entry rows.
Therefore, the per-month counts always match the number obtained by counting Entry rows by month.
This invariant presupposes that all writes to the index go through the timeline component, and breaks if the index file is rewritten from outside.
When it breaks, deleting the index file lets it be rebuilt by the next aggregation.

## 6. Aggregation

### 6.1 Scanning and Skipping Rereads

Aggregation scans the Library root or a specified range.
For local images and videos, it compares the size and modification time with the index's values, and does not reread if they are the same.
For archives, it compares the `ArchiveIdentity` ([Storage Layer Design §7.3](./storage.md#73-cache-identity)) with the index's value, and if the same, does not reread any of the Entries inside.
`ArchiveIdentity` is decided by the outermost archive's path, size, modification time, and the version of the extraction rules, so if any of them changes, it is reread.
If it differs, images and videos have their timeline dates reread.
An archive is reopened, and all the Entry rows belonging to that archive are replaced with the reread images and videos inside.
Images removed from the archive disappear from the rows and the per-month counts through this replacement.
In the second and later aggregations, the costs are only the scan itself and the files that changed.

Rows that have been read are committed a few hundred at a time.
Even in the middle of aggregation, Entries that have been read appear in the timeline and the per-month counts.

**Just before writing rows that have been read, aggregation checks whether the file the rows belong to still exists and has not changed since it was read.**
From the check until the write finishes, it makes the reflection of the daemon's own changes to the same Library (§6.4) wait.
The reflection of a change is done after the file operation finishes, so a result that aggregation read before the change has by then vanished or changed at the time of checking, and is not written.
A result read after the change reflects the post-change contents.
Therefore, even if aggregation and a change overlap, rows deleted or reassigned by the change are not written back with stale contents.

A full aggregation advances the Library's generation by one each time it starts, and records that generation in the rows of files seen in the scan.
The generation recorded on writing is the larger of the generation passed in and the current generation. Even if a range aggregation writes with an old generation, the row's generation does not go back, and the row is not removed by the cleanup of the full aggregation.
When the scan finishes, among the scanned range, the file rows that do not have this generation and the Entry rows belonging to those files are deleted.

### 6.2 Archive Extraction

For an archive, the index is read and the images and videos inside are grouped by block.
Each block is extracted exactly once, and all the timeline dates of the images and videos of that block are read before moving to the next block.
This is because extracting per Entry in a solid 7z would decode the same block again as many times as there are Entries ([Storage Layer Design §6.4](./storage.md#64-block-level-extract)).

Aggregation's extraction is done with a concurrency limit separate from browsing's extraction (`[timeline] max_concurrent_extractions`).
Extracted files are output to temporary files instead of the extraction cache, and deleted after reading.
Video ffprobe runs are also started with a limit separate from browsing's media info (`[timeline] max_concurrent_probes`).
Aggregation does not occupy browsing's concurrency budget, and the archive extraction cache used for browsing is not pushed out by aggregation's extraction.

The limit and deadline for one extraction use the same values of `[storage.archive]` as browsing.

### 6.3 When Aggregation Starts

At daemon startup, a Library whose index has not a single file row starts a full aggregation immediately.
This aggregation starts regardless of the value of `scan_interval_secs`.

If `[timeline] scan_interval_secs` is not 0, for each Library, a full aggregation also starts when `scan_interval_secs` has passed since the last full aggregation finished.
A Library that has never finished a full aggregation is treated as the interval having passed.
If `scan_interval_secs` is 0, only this periodic aggregation is absent.

From the web client, a full aggregation can be started with `POST /libraries/{libraryId}/timeline/refresh`.
Only one full aggregation of the same Library runs at a time, and a request received while one is running returns the current state.
Including range aggregations (§6.4), only one aggregation runs on the same Library, and aggregation yields its turn to the next aggregation each time it finishes reading one folder.

The progress of aggregation is returned by `GET /libraries/{libraryId}/timeline/status`.
It is not made an Operation (§9.1).

### 6.4 The Daemon's Own Changes

The daemon's own mutating operations tell the timeline the changed Entry's original location and destination at the same time as discarding listing snapshots ([Listing and Search Design §4.3](./listing.md#43-freshness)).

If a path told to it no longer exists, the rows of that path and underneath it are deleted before the response.
For folder copies and moves from another Library, child files told as changes are examined one at a time, not hidden under the parent folder.
Even for a file that exists, if the value the index remembers (size and modification time for images and videos, `ArchiveIdentity` for archives) differs from the current value, it is regarded as overwritten and the rows belonging to that file are deleted before the response.
Rows of the new contents are added by range aggregation.
For renames and moves within the same Library (for folder merges and moves of multiple items, per published file), before the response, the rows remaining at the destination and underneath are deleted, and then the paths of the rows at the original location and underneath are reassigned to the destination paths.
When a file is overwritten, the Entry rows and per-month counts belonging to the replaced destination file are also removed by this deletion.
In a folder merge, children of the destination are reassigned individually, so only children with the same name are replaced.
Reassigned rows keep their timeline dates, but their generation is set to a negative value representing the generation N at the time of reassignment, and are treated as unconfirmed rows until range aggregation re-determines whether they are targets.
The cleanup of a full aggregation deletes only the unconfirmed rows reassigned before the aggregation started, and leaves the rows reassigned during aggregation.
The cleanup of a range aggregation deletes the unconfirmed rows reassigned up to the time the aggregation started and not touched by the aggregation.
The `ArchiveIdentity` of a reassigned archive is recomputed from the new path.
For paths that exist, range aggregation is started in the background.
Range aggregation re-determines the extension and hidden-item rules (§3) at the destination path.
When `photo.jpg` is changed to `photo.txt` or `.photo.jpg`, or moved into a hidden folder, the rows that are no longer targets are deleted by the cleanup of range aggregation, and the per-month counts also decrease.
Entries added by uploads and copies appear starting with those whose timeline dates range aggregation has finished reading.
When a conflict results in another name, the actually published path is also told as a change, so the file with the other name is also added by range aggregation.

Range aggregation obtains its turn when the running aggregation finishes reading one folder.
A full aggregation can take hours to days, but waiting is needed only while one folder is read, so the reflection of changes does not stop in the meantime.
Range aggregation also performs the check just before writing (§6.1) and records the generation, so it is not removed by the cleanup of the full aggregation.

## 7. Reading

Timeline pages are ordered by timeline date, newest first, and Entries with the same timeline date are ordered by Library-relative path.
The position of a page is pointed at by a pair of the timeline date and Library-relative path of the Entry at the page's edge (a cursor), and the Entries before or after it are returned.
Instead of a cursor, specifying a month returns from that month's newest Entry, and specifying a Library-relative path returns a range containing that Entry in the center.

The Entries of a page are returned as the same `FileSystemEntry` as listings, with the timeline date and the timeline date source added.
The Entry of a file that vanished externally is also returned until the next aggregation.

The per-month counts are returned as a sequence in newest-month-first order.

## 8. Configuration

The timeline configuration goes in the top-level `[timeline]` and is validated at daemon startup.

| key | Default | Applies to |
| --- | ---: | --- |
| `[timeline] time_zone` | The OS setting | The time zone into which modification time and `creation_time` are converted (an IANA name) |
| `[timeline] scan_interval_secs` | 86400 | Time from the last aggregation's end until the next. 0 means no periodic aggregation |
| `[timeline] max_concurrent_extractions` | 1 | Concurrent archive extractions in aggregation |
| `[timeline] max_concurrent_probes` | 1 | Concurrent ffprobe runs in aggregation |

When `time_zone` is omitted, the time zone is obtained from the environment variable `TZ` or the OS setting, and if it cannot be obtained from either, it is UTC.
A Docker container is UTC by default, so set `time_zone` or `TZ` when listing alongside photos taken in other time zones.
Concurrency allows only positive values.

## 9. Design Decisions

### 9.1 Decided

#### Return from an Index Aggregated in Advance

**Decision**
The timeline is returned from the timeline index that the daemon has aggregated in advance (§5, §6).

**Reason**
The whole Library is the target, so if it were scanned after receiving a request, the wait until the first page would be proportional to the size of the Library.
To decide timeline dates from shooting dates, images and videos must be opened one by one, which cannot be done on every request.
The cost is carrying a discrepancy between the index and the actual files.
External changes are not reflected until the next aggregation, and until then the Entries of vanished files are also returned.

**Rejected alternative**
A method that sorts a recursive listing (`recursive=true`) in modification-date order without terms.
There are few server changes, but it is cut off at `recursive_max_entries`, the first page waits for the end of the scan, and it cannot sort by shooting date, so we rejected it.

#### Sort by Shooting Date and Use the Modification Time If Absent

**Decision**
The timeline date uses the Exif shooting date and time or a video's `creation_time`, and uses the modification time if they cannot be obtained (§4).

**Reason**
A photo whose modification time changed through copying or exporting is also listed on the day it was taken.
Images without a shooting date and time (screenshots, saved images) also enter the same order by modification time.

**Rejected alternative 1**
A method that sorts by modification time alone.
Aggregation would not need to open files, but a photo whose modification time changed through copying or exporting would be listed at a position different from the day it was taken, so we rejected it.

**Rejected alternative 2**
A method that lists only Entries that have a shooting date and time, and either gathers Entries without one at the end or does not list them.
The meaning of the order is clearest, but screenshots and saved images would drop out of the timeline, so we rejected it.

#### List Only Images and Videos

**Decision**
The timeline lists only images and videos (§3).

**Reason**
It suits the use of viewing photos and videos in the order they were taken, and tiles without thumbnails do not get mixed in.

**Rejected alternative 1**
A method that lists only images.
Aggregation would not need to start ffprobe, but videos taken on the same day would not appear in the timeline, so we rejected it.

**Rejected alternative 2**
A method that lists all files regardless of kind.
It could also serve the purpose of following recently changed files, but tiles with only icons would be mixed in and the count would grow, so we rejected it.

#### Also Extract Inside Archives and Read the Shooting Date and Time

**Decision**
Images and videos inside archives are also listed, and, as outside, they are extracted and the timeline date is read from the contents (§3, §4).

**Reason**
Whether the same photo is outside or inside an archive, it is listed on the same day by the same rule.
The cost is that the first aggregation extracts the images and videos of all archives in the Library, taking time proportional to the total volume of archives.

**Rejected alternative 1**
A method that uses the modification time each archive recorded per Entry, without extracting.
Aggregation would only need to read the index, but the rule would differ inside and outside archives, and the same photo would be listed by shooting date outside and by modification time inside, so we rejected it.

**Rejected alternative 2**
A method that uses the archive file's modification time for all Entries inside.
The contents of one archive would be listed together, but the photos inside would not be listed on the day they were taken, so we rejected it.

#### Read creation_time for Videos Inside Archives Too

**Decision**
For videos, both local and inside archives, `creation_time` is read with ffprobe (§4).

**Reason**
It becomes the same single rule as for images: read from the contents, and use the modification time if it cannot be obtained.
The cost is that a video inside an archive is extracted whole to read one tag.
Those that exceed the limit and deadline fall back to the modification time.

**Rejected alternative 1**
A method that reads only local videos and uses the Entry's modification time for videos inside archives.
Extracting large videos could be avoided, but the rule would differ only inside archives, so we rejected it.

**Rejected alternative 2**
A method that sorts videos by modification time and does not use ffprobe.
Aggregation becomes lighter, but photos and videos taken on the same day could be listed far apart, so we rejected it.

#### Hold the Timeline Date as a Date and Time in the Daemon's Time Zone

**Decision**
The timeline date is held as a date and time without a time zone.
The Exif shooting date and time is used as is, and the modification time and `creation_time` are converted to a date and time in the time zone of `[timeline] time_zone` (§4).

**Reason**
The Exif shooting date and time has no time zone, so converting it to a UTC instant would mean guessing the time zone of the place where it was taken.
Pxna is a home server with a single owner, so having the daemon hold one time zone suffices.
The server can decide the date boundaries, and the per-month counts can be aggregated in advance.
A timeline date obtained from a UTC instant also holds the original instant, so when the time zone is changed, it can be converted without rereading the files (§4).

**Rejected alternative 1**
A method that aligns everything to UTC instants.
The Exif shooting date and time is regarded as being in the configured time zone and converted to UTC.
The date boundaries would be decided by the device's time zone, so the per-month counts would have to be recounted in the device's time zone on every request, so we rejected it.

**Rejected alternative 2**
A method with no configuration that regards the Exif shooting date and time as UTC and mixes them.
The configuration does not grow, but Entries without Exif would be listed at positions shifted by the time difference and could cross dates, so we rejected it.

#### Hold a Timeline per Library

**Decision**
The timeline is held per Library, and a timeline spanning Libraries is not provided.

**Reason**
It becomes the same unit as the URL, the API namespace, and the determination of `readonly`.
A Library of photos and a Library gathering other images do not get mixed into one sequence.
The index holds the Library's id, so a display spanning Libraries can be added later (§9.2).

**Rejected alternative 1**
A method that makes one timeline across all Libraries.
It is closest to a photo app, but each Entry would have a Library, determining changeability would be split per Entry, and URLs not containing the Library would also be needed, so we rejected it.

**Rejected alternative 2**
A method that has both per-Library timelines and an all-Library timeline.
They could be used selectively, but the API and the screen would have two entrances, so we rejected it.

#### Start Aggregation Periodically and Manually

**Decision**
Aggregation starts when `[timeline] scan_interval_secs` has passed since the last one finished, and on a request from the web client.
A Library whose index is empty at daemon startup is also aggregated immediately (§6.3).

**Reason**
Without preparing an external cron, we prevent the index from staying stale.
The second and later aggregations read only the files that changed, so even when run periodically, most of the cost is the scan.

**Rejected alternative 1**
A method that aggregates only on a request from the web client.
It never uses disk and CPU without the user's knowledge, but if the request is forgotten the timeline stays stale, so we rejected it.

**Rejected alternative 2**
A method in which the daemon has only an API to start aggregation, and the timing is left to the OS's cron or the like.
The time can be chosen freely, but the user would have to prepare configuration that passes authentication, and with Docker a mechanism outside the container would be needed, so we rejected it.

#### Reflect the Daemon's Own Changes Immediately by Aggregating the Changed Range

**Decision**
The daemon's own changes immediately reflect only the changed Entry's range (§6.4).
Rows of vanished paths are deleted before the response, renames and moves reassign paths before the response and are re-determined by range aggregation, and added Entries are added by range aggregation.

**Reason**
A photo deleted in the timeline disappears immediately, and an uploaded photo appears without waiting for the next aggregation.
Range aggregation is the same processing as full aggregation, so no separate path for reflection is needed.

**Rejected alternative 1**
A method that reflects only deletions and moves immediately and waits for the next aggregation for additions.
Photos that should be gone would not remain, but an uploaded photo would not appear until the next aggregation, so we rejected it.

**Rejected alternative 2**
A method in which only the full aggregation rewrites the index.
The mechanism is the simplest, but a photo deleted from the timeline would remain until the next aggregation, and its tile would fail both in thumbnail and in opening, so we rejected it.

#### Give Aggregation a Concurrency Budget Separate from Browsing

**Decision**
Aggregation's extraction and ffprobe run with concurrency limits separate from browsing, and extracted files are not put in the extraction cache (§6.2).

**Reason**
The first aggregation can take hours to days, so browsing's extraction and media info are not made to wait during that time.
The archive extraction cache used for browsing is not pushed out by aggregation's extraction.
The cost is that aggregation is slower than if it shared the budget with browsing.

**Rejected alternative 1**
A method that uses the same budget and extraction cache as browsing.
No mechanism needs adding and aggregation is faster, but browsing of archives would wait during aggregation and the caches of recently viewed archives would be pushed out, so we rejected it.

**Rejected alternative 2**
A method that stops aggregation while browsing requests are coming and resumes when quiet.
The impact on browsing is the smallest, but a mechanism to watch for requests is needed, and aggregation would not progress under continued use, so we rejected it.

#### Show Results from the Part Where Aggregation Has Finished

**Decision**
Aggregation commits rows that have been read a few hundred at a time, and shows them in the timeline and the per-month counts even midway (§6.1).

**Reason**
Even for the first aggregation, the timeline can be used soon after it starts.
The cost is that during aggregation, Entries get added later among the order already loaded.

**Rejected alternative**
A method that builds a new index separately and swaps it in when aggregation finishes.
The order does not change while viewing, but for the first aggregation nothing would show until it ends, and capacity for two indexes would be needed.
It also cannot coexist with the decision to immediately reflect the daemon's own changes by range aggregation, so we rejected it.

#### Do Not Make the Progress of Aggregation an Operation

**Decision**
The progress of aggregation is returned by the timeline's status API and is not handled as an Operation (§6.3).

**Reason**
Operations exist to list the progress of mutating operations the user started in the activity panel.
Aggregation also runs periodically, so if it were an Operation, the daily aggregation would line up in the activity panel.

**Rejected alternative**
A method that adds aggregation as a kind of Operation and shows progress and cancellation in the activity panel.
Existing screens could be used, but for the reason above the activity panel would be filled with aggregation, so we rejected it.

#### Do Not Make Thumbnails in Aggregation

**Decision**
Aggregation reads only timeline dates and does not make thumbnails.

**Reason**
The size of a thumbnail changes with the device's tile size, shape, and pixel density, so making them in advance would also make combinations that are never used.
The cost is that an image in an archive seen for the first time in the timeline waits for extraction and generation, as in the folder view listing.

**Rejected alternative**
A method that makes thumbnails from the images extracted in aggregation, at the size and shape of the default display preferences.
The timeline would be faster on devices that use the defaults, but aggregation would be slower, the part beyond the thumbnail cache capacity would be discarded by LRU, and it would also overturn the decision not to bulk-generate thumbnails in the background ([Thumbnail Design §10.1](./thumbnail.md#generate-when-a-request-is-received)), so we rejected it.

#### Decide Archive Rereading by ArchiveIdentity

**Decision**
Aggregation decides whether to reread an archive by `ArchiveIdentity`, and when rereading, replaces all the Entry rows belonging to that archive (§6.1).

**Reason**
`ArchiveIdentity` is a value that the extraction cache and media info also use to point to the contents of an archive, and it changes if any of the path, size, modification time, or version of the extraction rules changes.
When the extraction rules are changed, results read under the old rules are not left.
Because rows are replaced, rows of images removed from the archive do not remain either.
The cost is that if an archive is renamed by something other than the daemon, or the Library's location is moved, it is reread even if the contents are the same.
In the daemon's own renames, the `ArchiveIdentity` of reassigned rows is recomputed, so it is not reread (§6.4).

**Rejected alternative**
A method that decides archives by size and modification time alone, like images and videos.
A rename would not need rereading, but old results would remain even when the extraction rules change, and the basis of determination would differ from the other caches, so we rejected it.

#### Check Just Before Writing and Make It Mutually Exclusive with the Reflection of Changes

**Decision**
Just before writing rows that have been read, aggregation checks whether the file they belong to still exists and has not changed since it was read, and from the check until the write finishes, it makes the reflection of the daemon's own changes to the same Library wait (§6.1).
Only one aggregation runs on the same Library, and a full aggregation yields its turn to a waiting range aggregation each time it finishes reading one folder (§6.3, §6.4).

**Reason**
The reflection of a change is done after the file operation finishes, so a result read before the change is removed at the time of checking.
Rows of photos that were deleted or renamed are not written back with aggregation's stale results.
Even with a full aggregation left running for days, a range aggregation starts after waiting for one folder.

**Rejected alternative 1**
A method that makes range aggregation wait until the end of the full aggregation.
No conflict would arise, but uploaded photos would not appear until the full aggregation ends, so we rejected it.

**Rejected alternative 2**
A method that runs the full aggregation and range aggregations at the same time.
There is no wait, but two aggregations would read the same file, and the order of recording generations and cleanup would become tangled, so we rejected it.

**Rejected alternative 3**
A method that does not prevent the conflict and fixes written-back rows in the next aggregation.
The mechanism is the simplest, but a deleted photo would remain in the timeline until the next aggregation, and the promise of deleting before the response could not be kept, so we rejected it.

#### Point at a Page's Position with a Cursor

**Decision**
A timeline page is pointed at by the pair of the timeline date and Library-relative path of the Entry at the page's edge, and the part before or after it is returned (§7).

**Reason**
As long as the timeline date does not change, the position before and after a cursor does not move even when rows increase or decrease through aggregation.
With a fixed index, Entries neither overlap nor go missing between pages.
When aggregation adds an Entry at a position falling within the range of pages already read, that Entry does not appear in the continuation from an existing cursor, and appears when reread.
The screen does not insert it automatically into the already loaded grid, and adds it with a reread operation ([Client Design §3.11](./client.md#311-timeline)).
The index's B-tree in sort order can be followed as is, so a page at any position can be returned at the same cost.
The timeline has no upper limit on count and does not reserve the overall height in advance ([Client Design §5.1](./client.md#the-timeline-does-not-reserve-the-overall-height-and-reads-the-continuation)), so specifying a month is enough for the use of jumping to any position.

**Rejected alternative**
A method that points at positions with `offset`, as in listings.
The client's loading could be shared with listings, but if rows increase midway through aggregation the positions shift and pages overlap or go missing, and SQLite gets slower in proportion to the number of rows it skips, so we rejected it.

#### Aggregate Per-Month Counts in Advance

**Decision**
The per-month counts are held in the index and updated in the same transaction as the increase and decrease of Entry rows (§5).

**Reason**
The list of years and months is read every time the timeline is opened.
Counting on every request would mean reading all the Entry rows.

**Rejected alternative**
A method that groups and counts Entry rows by month on every request.
The index would have one fewer table, but when the number of rows reaches millions, the display of the list of years and months would slow down in proportion, so we rejected it.

<a id="background-scan-budget"></a>
#### Route Aggregation's Heavy Processing Through the Shared Background Slot

**Decision**
The local enumeration, fetching EXIF and creation_time, and background extraction of archives use the background slot of the [storage shared budget](./storage.md#listing-work-budget).
Enumeration and content processing are not nested inside the same slot, and it is returned per batch, file, and block. The shared background slot is acquired first, and the existing probe and extraction slots after, and both are returned before the next unit. A slot is not held across a whole folder or the whole aggregation.
Even while foreground waiting continues, the one slot dedicated to the background remains. The existing per-Library de-duplication, generation checking, retry on failure at the next time, and aggregation state are maintained.
We do not assert from the difference in time alone which of disk waits, VirtioFS, or CPU this control had an effect on.

**Reason**
Even if only metadata is throttled, load remains if aggregation's content reading and extraction run concurrently elsewhere.

### 9.2 Deferred

#### A Timeline Spanning Libraries

**Current state**
The timeline is held only per Library.

**Why not decide now**
When Libraries are used separately for photos and everything else, a combined display would only mix them.
Whether a combined display is needed cannot be known until the per-Library timeline has been used.

**Conditions for deciding**
When situations in which one wants to browse the photos of multiple Libraries in one sequence arise repeatedly in actual use.

#### Whether to Use Exif's Time Zone Fields

**Current state**
The Exif shooting date and time is used as is as a value without a time zone, and Exif's time zone field (`OffsetTimeOriginal`) is not read.

**Why not decide now**
Few devices write the time zone field.
If it were used, how to order photos that have the field and photos that do not would have to be decided again.

**Conditions for deciding**
When photos from places in different time zones get mixed and the order of neighbors swaps, in actual use.

#### Whether to Reflect External Changes Immediately

**Current state**
External file changes are not reflected until the next aggregation.

**Why not decide now**
Reflecting immediately would mean using per-OS file change notifications, and handling the case where notifications do not arrive would also be needed on network file systems.
This is the same issue as the freshness of listings ([Listing and Search Design §8.2](./listing.md#whether-to-reflect-external-changes-in-listings-immediately)), and if it is decided, both would be handled together.

**Conditions for deciding**
When external changes that periodic and manual aggregation cannot keep up with arise repeatedly in actual use.

## 10. Current State and Remaining Work

### 10.1 Current Code

TimelineScanner runs full and changed-range scans in the background, and uses the shared background slot for local enumeration, EXIF, probes of creation_time, and inspecting and extracting archives. Even on paths that consume streams and single-flight in separate tasks, the class is held explicitly. The blocking workers for EXIF and extraction and probes that have started hold the slot until the actual processing ends even after the caller's cancellation.

The timeline index, aggregation, and API are implemented.
Timeline dates are decided by the daemon's `service/timeline`.
`TimelineScanner` scans the Library, and decides whether rereading is needed by size and modification time for local images and videos and by `ArchiveIdentity` for archives.
For archives, `FileStorageArchive::block_groups` and `extract_group` extract each block exactly once without going through the extraction cache.
`TimelineIndex` holds the SQLite index and the triggers that increase and decrease the per-month counts, and `TimelineService` handles periodic aggregation, manual aggregation, state, and the reflection of the daemon's own changes.
`LibraryMutation`, at the same point as discarding listing snapshots, tells `TimelineService` the changed Entry's path via `apply_changes`.
For renames and moves within the same Library, `renamed` reassigns the rows' paths.
The API has 4 endpoints: `GET /libraries/{libraryId}/timeline/entries`, `.../months`, `.../status`, and `POST .../refresh`.
A video's `creation_time` is read by `MediaTools::probe_creation_time`.

### 10.2 Roadmap

There is no remaining work in this milestone.
The web client's screen is in [Client Design §6.1](./client.md#61-current-code).

For confirmed defects, see [issues.md](../issues.md).
