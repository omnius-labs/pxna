# Listing and Search Design

## 1. About This Document

This document covers the mechanism that sorts and filters the contents of folders and archives and returns them in pages, and name search, and defines snapshots, how freshness is kept, search matching, the name index, recursion limits, and the reasons for adopting them.
The intended readers are developers and AI agents who implement or review this concern.
It assumes the vocabulary of [terms.md](../terms.md).
The definitions of words are not in this document.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | Sorting and filtering of listings, snapshots, name search, the name index, recursion limits, reasons for adoption, deferred items |
| [design.md](../design.md) | Pxna's overall scope, overall structure, how name order is compared, conventions for mutating APIs |
| [Storage Layer Design](./storage.md) | The driver operation that enumerates the Entries of folders and archives, archive identity and fingerprint |
| [Client Design](./client.md) | In what range and when listings are read, the search screen |
| [Timeline Design](./timeline.md) | The aggregation whose walk updates the name index, and when it starts |
| [Rating and Tag Design](./rating-tags.md) | Conditions on ratings and tags, and the identifier of a filtered listing |
| [terms.md](../terms.md) | Definitions of terms |
| [issues.md](../issues.md) | Clear defects confirmed in code and handling policies |
| [daemon/openapi.yaml](../../daemon/openapi.yaml) | The authoritative listing API paths, parameters, schemas, and status codes |

The entry point of the design document family is [design.md](../design.md).

### 1.2 Tense of This Document

§1 through §8 describe the completed form in the present tense.
The gap between the current code and the completed form, and the remaining work in dependency order, are gathered in §9.
If you want to know what works now, read §9 first.

## 2. Responsibilities and Boundaries

This document covers the span from receiving the inside of a folder or archive that a checked `NestedPath` points to, to returning the Entries of the requested range as a response.
`listFolderEntries` returns detail listings and recursive name search. The advance display of direct children's names and kinds uses the summary listing.
Search is handled as a listing with a name condition (§5).
A recursive listing with conditions is answered from the name index without walking the file system (§5.1, §5.2).

The client jumps to any position by the scrollbar even for a large folder, and reads only the page at that position ([Client Design §3.5](./client.md#35-loading-listings)).
For that reason, the server does not rescan every time a page is requested, and has a form in which a page at any position can be returned at the same cost.

The boundaries with other concerns are as follows.

| Matter | Authoritative source |
| --- | --- |
| The procedure for building a `NestedPath` from a Library-relative path and checking the boundary | [design.md §4.3](../design.md#43-resolving-library-and-archive-boundaries) |
| The operation that enumerates Entries inside folders and archives, and the rule of not following symlinks | [Storage Layer Design §5.1](./storage.md#51-read-operations), [§4.4](./storage.md#44-symlinks-and-special-entries) |
| How name order is compared | [design.md §11.1](../design.md#compare-name-order-in-natural-order) |
| Mutating operations the daemon itself performs | [design.md §8](../design.md#8-conventions-for-mutating-apis) |
| In what range and when listings are read, the search screen | [Client Design §3.5](./client.md#35-loading-listings), [§3.6](./client.md#36-search) |
| Returning a Library's images and videos in timeline-date order from an index built by aggregation | [Timeline Design](./timeline.md) |

### 2.1 What Is Not Built

- The name index has no scan of its own and does not watch file system notifications.
  It is updated by the walk of aggregation and by the daemon's own changes, so external changes appear in search at the next aggregation (§5.1).
- Neither a recursive walk nor the name index descends into archives.
  This avoids building the indexes of all archives in the range of the walk or the aggregation.
  Only when the path of an archive is specified as the starting point does a listing walk the inside of that archive.
- Search by file contents and metadata (such as EXIF) is not done.
  Matching is by name alone, so the walk does not need to read Entry contents.
- The continuation of a cut-off recursive walk is not returned.
  This avoids holding the mid-walk state across requests (§6).

## 3. Sorting and Filtering

A listing is a sequence of Entries determined by the target (Library and `path`) and the following specifications.

| Specification | Meaning |
| --- | --- |
| `recursive` | If false, only direct children; if true, all Entries underneath are targets |
| `includeHidden` | If false, excludes Entries whose names start with `.` and the Entries inside them |
| `sort`, `order` | The sort key and direction. Names are compared in natural order, and ties are decided by path |
| `foldersFirst` | Places folder-like Entries first, each following `sort` and `order` within itself |
| `foldersOnly` | Returns only folder-like Entries |
| `query` | A name condition (§5) |

`totalCount` is the count after applying `foldersOnly`, `includeHidden`, and `query`.
With this value the client decides the overall height of the listing from the first page.

## 4. Snapshots

### 4.1 The Unit That Is Built

At the first request for a listing, the server scans the target and keeps the resulting sequence of Entries in memory as a snapshot.
For later page requests, it responds from the snapshot without rescanning.

A snapshot is built for each combination of target, `recursive`, and `includeHidden`.
The target's version is held to check the freshness of the snapshot (§4.3).
The order by `sort`, `order`, and `foldersFirst` is computed only once per order inside the snapshot and kept.
`foldersOnly` and `query` are applied to the order on every request.
This way, the scan is needed only once even when the sort order is changed or the search terms are retyped.

A recursive listing answered from the name index is the exception.
Its snapshot holds only the Entries that match the conditions, and the conditions are part of the combination (§5.2).
Retyping the search terms queries the index again instead of rescanning.

Concurrent requests for the snapshot of the same combination are merged into one scan.
If all the requests waiting for a scan disconnect, the scan is stopped and the snapshot is discarded.

### 4.2 Extracting a Page

A page is specified by `offset` and `limit`, and cut out from the sequence after sorting and filtering are applied.
If `offset` exceeds the count, an empty page is returned.

The response has the page's Entries, `totalCount`, the snapshot's id, and whether the recursion was cut off (`truncated`, §6).
The snapshot's id is a value newly assigned each time a snapshot is built, and it changes if rebuilt even when the version is the same.

The client learns that the listing has changed by comparing the id of each page.
When the id changes, it discards the other pages read so far and reads again.
As long as only pages of the same id are joined together, Entries are neither duplicated nor missing between pages.

### 4.3 Freshness

On every request, the server checks whether the snapshot has gone stale.
If stale, it rescans and responds from a snapshot with a new id.
The client is not told that it has gone stale by the request, and learns of it from the change in id (§4.2).

A snapshot holds the target's version at the time of scanning, and is considered stale if it differs from the version at the time of the request.
The target's version is decided according to the kind of target as follows.

| Target | Version |
| --- | --- |
| Direct children of a local folder | The folder's modification time |
| Inside an archive | The archive's identity ([Storage Layer Design §7.3](./storage.md#73-cache-identity)) |
| A recursive listing | None |

Separately from this, any snapshot is considered stale once `snapshot_max_age_secs` has passed since it was built.
A recursive listing has no means of checking changes underneath with one stat, so freshness is checked by this elapsed time alone.

On POSIX-compliant file systems, a local folder's modification time changes when an Entry is created, deleted, or renamed among its direct children ([Directory modification time](../knowledge/posix-directory-mtime.md) in [knowledge.md](../knowledge.md)).
A change to the contents of an existing file, and a change inside a child folder, do not change the modification time.
These are not reflected in the listing's size and modification date until `snapshot_max_age_secs` has passed since the snapshot was built.

On file systems with coarse modification-time precision, a change right after a scan gets the same time as before the scan and may not show up as a version difference.
Therefore, if the difference between the time the scan started and the folder's modification time is less than 2 seconds, that snapshot is rebuilt on the next request.
If a Library is placed on an NFS mount, a change on another device may be delayed before showing up in the modification time because of the client's attribute cache ([NFS attribute cache](../knowledge/nfs-attribute-cache.md) in [knowledge.md](../knowledge.md)).
During that time, the old listing is returned.

The daemon's own mutating operations discard snapshots without waiting for the modification time.
Synchronous operations do so before returning the response, and Operations when they finish, discarding the snapshots whose targets are the ancestors (up to the Library root) of the changed Entry, regardless of whether they are recursive.
Ancestors are discarded and not only the parent because, in a change that writes while creating intermediate folders, the listing of the parent of the created folder also changes.

### 4.4 Capacity and Expiry

A snapshot is discarded once `snapshot_idle_secs` has passed since it was last used.
The total number of Entries held by all snapshots has an upper limit of `snapshot_capacity_entries`, and when exceeded, the ones with the oldest last-used time are discarded first.
A snapshot that alone exceeds the limit is not retained and is used only for that request's response.
That target is rescanned at every page request.

Detail listing snapshots are placed only in the process's memory. Summary listings are also reused from the browse index ([Design decisions](#persistent-summary-index)).
They disappear when the daemon is restarted and are rebuilt at the next request.

## 5. Name Search

When `query` is specified, only Entries whose names match the condition are returned.
An Entry matches if its name contains all of the terms obtained by splitting `query` on whitespace.
Before matching, both the name and the terms are normalized with NFKC, and lowercased with Unicode's lowercase mapping.

We normalize because a Library may contain both decomposed and composed names.
macOS's HFS Plus stores filenames in decomposed form, and APFS stores them in the normalization they had at creation ([macOS filename normalization](../knowledge/macos-filename-normalization.md) in [knowledge.md](../knowledge.md)).
If the "が" in a name is decomposed (か of U+304B and the voiced mark of U+3099) and the "が" in the term is composed (U+304C), comparing without normalization does not match.
NFKC also unifies full-width and half-width alphanumerics into the same characters.

Only the Entry's name is matched, and the parent's path is not included.
When searching underneath, `recursive=true` and `query` are combined.
When a subtree search walks the file system, Entries that do not match the condition are also walked, so the time it takes is decided by the number of Entries underneath regardless of the condition.
The walked snapshot is built in the combination that does not include `query` (§4.1), so retyping search terms does not rescan.

### 5.1 Name Index

The name index holds, per Library, the Library-relative path, the parent's path, the kind, and the normalized name of every Entry outside archives.
Hidden Entries whose names start with `.`, and the Entries inside them, are not held, by the same rule as aggregation ([Timeline Design §3](./timeline.md#3-what-is-listed)).
An archive is held as one Entry, and its contents are not.
It is derived state in SQLite under the listing state directory, separate from the authentication DB.

The index is updated at the same points as the timeline index.
Each time aggregation enumerates a folder, the rows of that folder's direct children are replaced with the enumerated names ([Timeline Design §6.1](./timeline.md#61-scanning-and-skipping-rereads)).
A full aggregation records its generation in the rows it saw, and when the scan finishes, removes the rows in the scanned range that do not have it.
The daemon's own changes delete the rows of a path that no longer exists and everything under it, and reassign the rows under a renamed or moved path to the destination, before the response ([Timeline Design §6.4](./timeline.md#64-the-daemons-own-changes)).
Added Entries appear when range aggregation enumerates their folder.
External changes appear at the next full aggregation or manual aggregation.
A result that has changed externally since then is still returned, and opening it gives `NotFound` through the existing revalidation of the path.

An index of a Library becomes usable when a full aggregation of that Library has finished with it once, and this is recorded in the index file per Library.
Later aggregations update the rows in place, so the index stays usable while they run.
If reading, writing, or verifying the index fails, the process stops using it and returns to walking.
At the next startup the file is verified again, and a Library whose record of usability is missing or whose file was discarded waits for a full aggregation ([Timeline Design §6.3](./timeline.md#63-when-aggregation-starts)).
A snapshot answered from the index is discarded by the daemon's own changes and after `snapshot_max_age_secs`, as other recursive listings are (§4.3).
A missing, corrupt, or unknown-format index is discarded and rebuilt by the next full aggregation, and does not affect the authentication DB.

The index holds the whole range above and has no capacity limit or LRU.
An index that has dropped part of a Library cannot tell a missing Entry from an Entry that does not match, so it could not answer a search.
Its size is proportional to the number of Entries.

A term of 3 or more characters is matched against a trigram index of the normalized names, and a shorter term is matched by comparing the normalized names of the candidates directly.
SQLite's trigram tokenizer matches no rows for a substring shorter than 3 characters ([SQLite FTS5 trigram](../knowledge/sqlite-fts5-trigram.md) in [knowledge.md](../knowledge.md)).
When all terms are shorter than 3 characters, the candidates are all rows under the start.
Names are stored already normalized by the rules of §5, so the trigram index compares them as they are and does not fold case or remove diacritics itself.
The meaning of a match is the same as in a walk.

### 5.2 Answering Recursive Listings with Conditions

A listing is answered from the name index when all of the following hold.

- `recursive` is true and `includeHidden` is false
- At least one of a name condition, a rating condition, or a tag condition is given
- The start is a local folder, not an archive or a path inside one
- The index of that Library is usable (§5.1)

Otherwise the listing walks the file system as before.
The response tells whether the name index answered it.
When only the last condition fails, the walked result may be cut off by the walk's count, and the client shows that the index is being built ([Client Design §3.6](./client.md#36-search)).
A walk taken for one of the other reasons is the intended answer and is not marked.

The snapshot holds only the matching Entries, sorted by the same rules as other snapshots.
The index holds names and kinds only, so the attributes that a detail listing returns are read from the file system for the matching Entries, with the same workers and limits as the detail batch.
A match whose Entry no longer exists is dropped at this point, so the results do not include Entries deleted since the last aggregation.
Beyond the index query, the cost of a search is therefore proportional to the number of matches, not to the size of the Library.
With a good, bad, or tag condition, the candidates are the paths under the start in the user's ratings and tags, kept only if the name index holds them, and then the name terms are applied.
Ratings and tags are a small part of a Library, so starting from them is cheaper than starting from the names.
With the unrated condition, the candidates are the image and video rows of the index under the start, minus the rated paths, so without name terms the number of matches can reach the limit of §6.
A rated path whose Entry has been deleted externally is not in the index and does not appear in the results.
The version of ratings and tags is part of the combination, so a snapshot with a rating or tag condition is rebuilt from the index when they are written ([Rating and Tag Design §7.1](./rating-tags.md#71-filtering-listings)).

## 6. Recursion Limit

A recursive walk is cut off when the Entries put in the snapshot reach `recursive_max_entries`.
A cut-off snapshot lists only the Entries already walked and sets `truncated` in the response.
Which Entries become already walked is decided by the storage layer's walk order and has nothing to do with the sort order.

The limit is there to hold down memory and the wait until the first page.
In a subtree search from the Library root, the snapshot would hold the Entries of the whole Library.
Without the limit, one search would use memory and wait time proportional to the size of the Library.

For a listing answered from the name index, the same `recursive_max_entries` limits the number of matching Entries put in the snapshot (§5.2).
Matching Entries are a part of the Entries underneath, so a search from the Library root is not cut off unless the conditions themselves match that many.
There is no walk to bound, and the query is stopped when all requests waiting for it disconnect, as with a scan (§4.1).

When the client receives `truncated`, it indicates that the result is partial and urges moving the starting point down ([Client Design §3.6](./client.md#36-search)).

## 7. Configuration

The listing configuration goes in the top-level `[listing]`.
Counts and times allow only positive values and are validated at daemon startup.

| key | Default | Applies to |
| --- | ---: | --- |
| `[listing] snapshot_idle_secs` | 600 | Time until an unused snapshot is discarded |
| `[listing] snapshot_max_age_secs` | 300 | Time from building a snapshot until it is considered stale |
| `[listing] snapshot_capacity_entries` | 500000 | Upper limit on the total number of Entries held by all snapshots |
| `[listing] recursive_max_entries` | 200000 | The count at which a recursive walk is cut off |
| `[listing] summary_index_capacity_bytes` | 268435456 | Upper limit on the total persistent payload of the browse index |

`recursive_max_entries` must be smaller than `snapshot_capacity_entries`.
If it is the same or larger, a cut-off recursive snapshot alone would exceed the capacity, and would not be retained and would be rescanned at every page.

## 8. Design Decisions

### 8.1 Decided

#### Place Folder-Like Entries First in Folder Listings and Allow Filtering

**Decision**
`listFolderEntries` places folder-like Entries first when `foldersFirst` is specified, and sorts the folder-like Entries and the rest each by `sort` and `order`.
When `foldersOnly` is specified, it returns only folder-like Entries.
Both default to false.
Whether something is an archive is determined by the `archive` hint in the file detail, without reading the file's contents.

**Reason**
The client's default order ([Client Design §3.2](./client.md#32-tiles)) places folder-like Entries first, and the web client's tree needs only folder-like Entries.
Listings are returned in pages, so if the client separated them after receiving, the order at the top would not be decided until all pages had been read.
We made the default false so that the order of calls that do not specify it does not change.

**Rejected alternative 1**
A method in which the client fetches all pages and then separates them.
The API would not change, but for a large folder the first display would wait for fetching all pages, so we rejected it.

**Rejected alternative 2**
A method in which `listFolderEntries` always returns folder-like Entries first.
There would be fewer specifications, but even when `sort=size` or the like is specified, folder-like Entries would come first and the specified order could not be obtained, so we rejected it.

**Rejected alternative 3**
A method that makes separating and filtering a single enumeration specification.
The contradiction of specifying both would no longer arise, but when another axis is added to ordering, the enumeration's values would multiply by the number of combinations, so we rejected it.

#### Cut Listings from a Snapshot of a Single Scan

**Decision**
At the first request for a listing, the target is scanned to build a snapshot, and the following pages are cut from the snapshot by `offset` (§4).
The response carries the snapshot's id, and there is no cursor.

**Reason**
The per-page scan goes away, and even for a folder of 100,000 items, the second and later pages do not pay the cost of a scan.
With `offset`, a page at any position can be returned at the same cost, so the client can read only the page it jumped to with the scrollbar.
Pages of the same snapshot are cut from the same sequence, so even if the folder changes externally between pages, neither duplicates nor omissions occur.
The change is conveyed to the client by the change in id.

**Rejected alternative 1**
A method that reads and sorts the whole target for each page and cuts out one page.
The previous implementation took this form and the server holds no state, but reading a folder of 100,000 items to the end with pages of 200 items would be 500 scans, so we rejected it.
If the position to cut is pointed at by a count from the top with this method, duplicates or omissions occur when Entries increase or decrease between pages.
The `offset` cut from a snapshot points to a position within the same sequence, so this problem does not occur.

**Rejected alternative 2**
A method in which the cursor holds the sort key and path of the last Entry of the page, and the next page is returned from after that Entry.
The previous implementation took this form, and neither duplicates nor omissions occur even if Entries increase or decrease between pages, but it can point only to the continuation, and jumping to the end means reading all pages in order, so we rejected it.
The per-page scan also remains.

**Rejected alternative 3**
A method that holds a snapshot and reads the continuation with a cursor.
The per-page scan goes away, but as with rejected alternative 2, one cannot jump to an arbitrary position, so we rejected it.

#### Check Snapshot Freshness by the Target's Version and Our Own Changes

**Decision**
On every request, we check the target's version (a local folder's modification time, an archive's identity) and rebuild if it changed.
Any snapshot is rebuilt once `snapshot_max_age_secs` has passed since it was built, and a recursive listing is checked by this elapsed time alone.
The daemon's own mutating operations discard the related snapshots (§4.3).

**Reason**
A folder's modification time changes with additions, deletions, and renames of direct children, so with one stat we can check whether the listing's rows have changed.
The daemon's own changes are reflected in the listing right after the change, regardless of the precision of the modification time.
Changes that the modification time does not catch (changes to file contents, changes inside child folders) also do not remain forever in a snapshot that keeps being used, because there is an upper bound on elapsed time.
Only a recursive listing has no means of checking changes underneath with one stat, so it is delimited by elapsed time alone.

**Rejected alternative 1**
A method that only expires after a fixed time from building.
The mechanism is the simplest, but during that time neither external changes nor the daemon's own changes are reflected in the listing, so we rejected it.

**Rejected alternative 2**
A method that rebuilds only when the client instructs an update and on the daemon's own mutating operations.
It also needs no stat on every request, but external changes would not be visible until the user updates, so we rejected it.

#### Treat Name Search as a Listing Condition

**Decision**
Name search is done with `query` of `listFolderEntries`, and search underneath is done in combination with `recursive=true`.
There is no dedicated search API.

**Reason**
Search results can also use snapshots, sorting, `totalCount`, and extracting a page at any position as they are.
The client can display search results in the same grid and list as ordinary listings.
A walked snapshot is built in the combination that does not include `query`, so retyping search terms does not rescan.
A snapshot answered from the name index includes the conditions, so retyping costs one index query instead.
The cost is that the first page does not return until the walk or the index query finishes.

**Rejected alternative 1**
A streaming search API that sends found Entries in order.
The first results appear immediately, but the order changes as they arrive, so virtual scrolling and sorting would have to be built separately from ordinary listings, so we rejected it.

**Rejected alternative 2**
Keeping a search API (`searchEntries`) that waits until the scan finishes or `limit` items are reached and returns in scan order.
The previous implementation took this form, and no server change is needed, but there is no way to fetch the continuation and sorting is left to the client, so we rejected it.

<a id="name-index"></a>
#### Answer Recursive Searches from a Library-Wide Name Index Updated by Aggregation

**Decision**
A recursive listing with name, rating, or tag conditions is answered from the name index, which aggregation's walk and the daemon's own changes update (§5.1, §5.2).
External changes appear in search at the next aggregation.
The browse index keeps holding only already-browsed summary listings of direct children, and the timeline index is not substituted for name listings.

**Reason**
The design assumes Libraries of 10 million Entries or more.
A walk costs time in proportion to the Entries underneath, and a search from the Library root also has to be cut off by count.
Answering without touching the file system is the only way to keep a search from the root within a second at that scale.
Aggregation already walks every non-hidden folder once a day by default, so the index needs no scan of its own.
Reflecting external changes at the next aggregation is the same promise the timeline makes ([Timeline Design §2.1](./timeline.md#21-what-is-not-built)).
The timeline index holds only images and videos, so it cannot answer searches that include folders and other files.

**Rejected alternative 1**
A method that reuses the browse index during a recursive walk, checking each folder's modification time with one stat and reusing its saved names.
Freshness would stay as exact as now and nothing new would follow external changes, but a search from the root still needs one stat and one lookup per folder.
At an average of 10 Entries per folder, that is about a million stats per search at the assumed scale, which takes minutes, so we rejected it.

**Rejected alternative 2**
A method that walks when the range under the start is small and uses the index when it is large.
Small searches would always be current, but the same terms would give different results depending only on the start, and both paths would have to be tested for agreement, so we rejected it.

**Rejected alternative 3**
The previous method of scanning at request time for every recursive search.
It always returns the current state, but at the assumed scale a search from the root waits for the whole Library and is cut off by count, so it is replaced.

**Rejected alternative 4**
A method in which the name index has its own periodic walk.
It would not depend on aggregation's interval, but it would double the daily full walk of the NAS, so we rejected it.

#### Read the Attributes of Matching Entries from the File System

**Decision**
The name index holds no sizes or times, and a listing answered from it reads the attributes of the matching Entries from the file system before building the snapshot (§5.2).

**Reason**
A detail listing returns size, modification time, and readonly as required fields, and sorting by size or date needs them.
Reading them for the matches only keeps them current and drops Entries deleted since the last aggregation.

**Rejected alternative 1**
A method that stores sizes and times in the name index.
No file system access would be needed, but they would be as old as the last aggregation and the index would grow, so we rejected it.

**Rejected alternative 2**
A method that returns results from the index without those attributes.
It would change the required fields of the existing listing contract, which the summary listing decision kept for existing clients ([summary-listing](#summary-listing)), so we rejected it.

#### Match Names in the Index with Trigrams and Compare Short Terms Directly

**Decision**
Terms of 3 or more characters are matched through a trigram index of the normalized names, and shorter terms by comparing the normalized names of the candidates (§5.1).

**Reason**
Terms of 3 or more characters, which narrow results well, return from the index without reading every row.
Japanese terms are often 1 or 2 characters, and the trigram index cannot match those, so they are compared directly.
A search with only such terms costs as much as rejected alternative 1 and no more.

**Rejected alternative 1**
A method that compares the normalized names of all rows under the start for every term.
It needs the least disk, but every search reads all the rows, so we rejected it.

**Rejected alternative 2**
A method that keeps all normalized names in memory and compares them in parallel.
It would be fast for every term, but at the assumed scale it keeps 400 MB or more resident on the NAS, so we rejected it.

#### Walk While the Name Index Is Not Usable

**Decision**
While the index of a Library is not usable, recursive listings with conditions walk the file system as before, and the response tells that the index did not answer (§5.2).

**Reason**
It follows the same rule as the browse index: when derived state cannot be used, the listing returns to enumeration.
The first full aggregation of a large Library can take hours, and search keeps working during that time.

**Rejected alternative 1**
A method that refuses recursive search until the index is usable.
The results would always come from one source, but search would be unavailable for hours after adding a Library, so we rejected it.

**Rejected alternative 2**
A method that answers from the part of the index built so far.
It would be fast, but the user could not tell which part of the Library the results cover, so we rejected it.

#### Normalize Names with NFKC and Match in Lowercase

**Decision**
Before search matching, both the name and the terms are normalized with NFKC and lowercased with Unicode's lowercase mapping (§5).

**Reason**
Decomposed filenames created on macOS can be found with terms typed in composed form.
Differences between full-width and half-width alphanumerics also do not cause misses.
The lowercase mapping is in Rust's standard library, and NFKC normalization can be done with a crate already included in the dependencies, so no dependency needs to be added for matching.

**Rejected alternative 1**
A method that lowercases the name and terms without normalizing and then compares.
The previous `searchEntries` took this form, and it does not distinguish upper and lower case of letters, but it misses decomposed names and full-width alphanumerics, so we rejected it.

**Rejected alternative 2**
A method that uses Unicode case folding instead of the lowercase mapping.
Pairs that do not become the same under lowercase mapping, like German ß and ss, could be regarded as the same, but a crate for folding would be added to the dependencies, so we rejected it.

#### Cut Off Recursive Walks by Count

**Decision**
A recursive walk is cut off when the Entries put in the snapshot reach `recursive_max_entries`, and responds with only the Entries already walked and sets `truncated` (§6).
For a listing answered from the name index, the same limit counts matching Entries.

**Reason**
Even in a subtree search from the Library root, memory and the wait until the first page stay within the limit.
Even when cut off, the results for the range already walked are returned, so the user can move the starting point down and search again.
A snapshot from the name index holds only the matching Entries, so counting matches bounds the same memory without cutting off a search merely because the Library is large.

**Rejected alternative 1**
A method that sets a deadline for the walk and fails with a 504 `Timeout` once it passes.
It would match the rule for extraction and generation deadline expiry, but the results for the range already walked would not be returned either, so we rejected it.

**Rejected alternative 2**
A method with no limit that walks until the client disconnects.
It is expected to fit at the scale of a personal NAS, but the snapshot's memory would have no upper limit, so we rejected it.

<a id="summary-listing"></a>
#### Make the Summary Listing of Direct Children a Separate Response from the Existing Detail Listing

**Decision**
The summary listing is chosen explicitly with a new API. Each item has only the Library-relative path, the name, the kind, and the archive hint by extension, and size, time, and readonly are not filled in with placeholder values.
The target is limited to direct children, and name order and kind order, ascending and descending, foldersFirst, foldersOnly, includeHidden, name conditions, and offset and limit can be used. Ordering and filtering follow the existing rules.
Recursion, size order, and modification-date order use the existing detail listing. The items and required-ness of the existing API are not changed, and existing clients continue to use the detail listing.

Local enumerates from names and file_type, and in environments where the kind cannot be obtained, falls back to the necessary attribute query. We do not unconditionally guarantee that all stat calls can be omitted on VirtioFS.
A symlink is returned as a link, and the kind and path of the target are read in the detail. Non-UTF-8 and special Entries are not silently excluded.
The archive's summary enumeration uses the existing archive index, and does not extract each file merely to get names.

Detail fetching is a read-only batch of 1 to 200 paths, and the path syntax of the whole request and duplicates after normalization are checked before I/O. Across all Libraries and all requests, at most 4 items are resolved at the same time. Separately from this item admission slot, the leaf's local attribute fetch uses the same worker as an ordinary listing, and after the introduction of the background budget it is included in the 3 foreground slots ([shared budget](./storage.md#listing-work-budget)).
Each result returns the normalized path and either `detail` (the existing detail Entry) or `error` (a failure code), in input order. A leaf symlink is not followed and the same link detail as in listings is returned. A failed item is also not removed from the initial listing on its own, and can be retried.
The path goes through the same boundary resolution as the existing ones. Aliased libraries and host paths are not mixed into the response, and authentication and authorization are the same as the existing APIs.

The summary listing's snapshot identifies the sequence of names and kinds. Even when details arrive, the order, total, and snapshot do not change.
When names and kinds disagree with the details, it is re-read as an update, and pages or details of different snapshots are not mixed. The name snapshot does not guarantee the freshness of metadata.

**Reason**
It separates the first page from the detail fetching of all items, and avoids breaking old clients by turning the existing required metadata into missing values.
To keep pages at any position, even on the summary path the enumeration and sorting of direct children complete before the first page is returned.

**Rejected alternative**
Putting 0 and false into the required metadata of the existing FileSystemEntry would misrepresent sizes and changeability, so we do not adopt it.
A partial listing in first-come order cannot maintain the same snapshot sequence as offset, so we do not adopt it.

<a id="persistent-summary-index"></a>
#### Persist the Browse Index as Rebuildable Derived State

**Decision**
Only summary listings of direct children are saved in SQLite under the dedicated state's listing directory. The key includes the Library id, the configured root, the normalized path, and includeHidden, and is separate from recursive and detail listings.
What is saved is the sequence of names and kinds, the normalized names for matching, the indexes for name order and kind order, the target's version, the creation time, the format version, and a verification value of the contents. Authorization information and Entry readonly are not saved.
At request time, the current boundary is resolved, and the local folder's modification time or the archive identity and snapshot_max_age_secs are checked. Entries with an unknown version, a future time, a local update within the last 2 seconds, or an expiry are not reused.
External changes that this check cannot detect have the same limits as the existing freshness rules, and we do not promise immediate detection of changes with spoofed modification times or of the file system's attribute cache.

The in-memory summary listings and the persistent side each have a count limit of snapshot_capacity_entries. The total persistent payload is up to summary_index_capacity_bytes (the default is 256 MiB) and 1 item up to 64 MiB, discarded in order of oldest use.
An empty listing also consumes one retention slot, so that the number retained is not unbounded with empty folders alone. On restart, capacity is rechecked with the current settings.
This is a budget for payload, and does not guarantee that the physical file including SQLite's pages, free space, and journal has the same number of bytes. A listing that cannot be retained is still returned for that request.
Persistent writes are limited to 1 running and 1 waiting, and if full, persisting that result is omitted. It does not make the first response wait. Concurrent scans of the same target are merged, and if all waiters leave, the unfinished enumeration is canceled.

When reflecting our own changes, the in-memory and persistent names are invalidated for the same range of ancestors. Generations are checked so that a stale scan or write does not put its result back.
If reading, writing, invalidating, or verifying the persistent side fails, persistent use is stopped for that process and it returns to enumeration. At startup as well, writability and format are checked, and an unusable index is not trusted merely by reading.
A missing, corrupt, or unknown-format index does not affect the authentication DB, and only the dedicated derived state is discarded and rebuilt. Old builds can ignore this storage location.

Listings and details are for reference display, and for operations that open or change, the existence and kind of the entity, authorization, and readonly are revalidated through the existing paths.

**Reason**
It skips enumeration and natural-order sorting after a restart, and avoids being unable to browse because of an index failure.
It keeps the checks of maximum age and version and does not become a cache that returns stale displays forever. The cost is that the first enumeration and sorting after the expiry remain.

### 8.2 Deferred

#### Whether to Reflect External Changes in Listings Immediately

**Current state**
Changes to the direct children of a local folder are reflected from the next request by modification time.
Changes to file contents, changes inside child folders, and changes delayed by the NFS attribute cache are not reflected until `snapshot_max_age_secs` has passed since the snapshot was built (§4.3).
The name index reflects external changes at the next aggregation (§5.1).

**Why not decide now**
Reflecting immediately would mean using per-OS file change notifications, and handling the case where notifications do not arrive would also be needed on network file systems.
Whether the modification time and the elapsed-time limit are enough cannot be known until it is used with a real operation Library.

**Conditions for deciding**
When situations where a stale listing is displayed because of changes the modification time and the elapsed-time limit cannot catch arise repeatedly in real operation.

## 9. Current State and Remaining Work

### 9.1 Current Code

The summary listing, the detail-fetching batch, and the persistent browse index are implemented in [summary.rs](../../daemon/entrypoints/daemon/src/service/listing/summary.rs) and below. The existing detail listing and required metadata are kept, and the summary listing holds names and kinds in separate columns. If the persistent side cannot be used, it returns to enumeration.

`listFolderEntries` cuts a page by `offset` and `limit` from the `ListingSnapshot` held by `ListingSnapshots` in [snapshot.rs](../../daemon/entrypoints/daemon/src/service/listing/snapshot.rs).
Sorting and name matching, the per-level walk that does not descend into hidden folders, and cutting off recursion are handled by [listing.rs](../../daemon/entrypoints/daemon/src/service/listing.rs).
A listing with `query` uses the same snapshots.

The `[listing]` configuration, checking freshness by the target's version and elapsed time, sharing the scan among concurrent requests, stopping the scan when all requests have disconnected, and discarding by capacity and expiry work according to the rules of §4 through §7.
`LibraryBrowser` reads snapshots, and `LibraryMutation` and `OperationService` discard ancestors' snapshots after the daemon's own changes.

The name index of §5.1 and §5.2 is not implemented. Recursive listings with conditions always walk the file system and are cut off by the number of walked Entries.

### 9.2 Roadmap

The order is provisional.
Only the dependencies must be respected.

| No. | Content | Dependencies assumed | State |
| --- | --- | --- | --- |
| L1 | Implement the name index, its update by aggregation and by the daemon's own changes, and answering recursive listings with name, rating, and tag conditions from it (§5.1, §5.2) | None | Not started |

L1 corresponds to M19 of [design.md §12.2](../design.md#122-roadmap).

For confirmed defects, see [issues.md](../issues.md).
