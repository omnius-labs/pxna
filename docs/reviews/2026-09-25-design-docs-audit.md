# Audit of the Design Documents and the Read Path

## 1. About This Review

This document is a record of an audit that checked the design documents and the terms document against the code of the target commit and `openapi.yaml`.
At the same time, we looked for gaps in the design that should be decided before starting M2 through M7.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The audited scope, perspectives, basis, findings, and subsequent handling |
| [design.md](../design.md) | Design rationale, invariants, deferred items. Destination for transcribing findings of type Design issue |
| [Storage Layer Design](../design/storage.md) | Design details of storage |
| [terms.md](../terms.md) | Vocabulary. Destination for transcribing findings of type Term |
| [issues.md](../issues.md) | Unaddressed defects and improvement proposals. Destination for transcribing findings of types Defect and Improvement proposal |

[design.md](../design.md#11-division-of-responsibility-between-documents) holds the map of the document family.

### 1.2 Review Date and Subject

- Review date: 2026-09-25
- Review type: audit review
- Target commit: `370036a289e629117ff1302954364b547ded35eb`
- Reviewer: Claude Code (AI agent)

## 2. Scope and Perspectives

### 2.1 What Was Examined

For documents, we read through `docs/design.md`, `docs/design/storage.md`, `docs/terms.md`, `docs/terms/storage.md`, `docs/issues.md`, `docs/plans.md`, and `docs/reviews.md`.
For code, we cut the scope to what was needed to confirm the facts these documents state.
The criterion is "the code that a sentence in a design document points to", and the quality of implementation that does not appear in the documents is not included in the scope.

The code we read is as follows.

- The daemon's `config.rs`, `executor.rs`, `state.rs`, `path.rs`, `library.rs`, `error.rs`, and the read-side handlers and startup processing of `server.rs`
- The engine's `service/storage.rs`, `core/nfs/driver.rs`, the symlink and rename handling in `driver/local.rs`, the archive's `format.rs` and `identity.rs`, the timeout handling in `options.rs`, and the initialization and lookup in `cache.rs`
- The engine's `core/thumbnail/` cache and generator, and the open in `base/storage/key_value_rocksdb_storage.rs`
- The security of `openapi.yaml`, `FileSystemEntryLinkDetail`, `RelativePath`, and `ReadLimitQuery`
- The authentication branch of the generated interface and the type of the content response, and CI's `test-daemon-slim.yml`

### 2.2 What Was Not Examined

- The internals of the archive readers (`zip.rs`, `sevenz.rs`), `extractor.rs`, `index.rs`, `index_cache.rs`, `single_flight.rs`, `staging.rs`, `rename.rs`, and `materialized.rs` were read through in the audit of 2026-08-28, so we read only a few places to check against the documents' descriptions.
- The web client, iOS, and `daemon/refs` were out of scope, because the design documents merely state that they are unimplemented.
- We did not read the engine's authentication and old challenge-method code, because it is planned to be removed in M2.
- `docs/coding/rust.md` was excluded because its canonical copy is in core-rs.
- We did not run tests or builds. The purpose was to check the documents against the code, and runtime behavior is not among the perspectives, so it is out of scope. All observations are the result of reading the code and documents.
- External behaviors such as the OS's no-replace rename, the ZIP and 7z specifications, and the browser's handling of cookies were not checked against primary sources for the documents' descriptions. This is for lack of time.

### 2.3 Perspectives

- Whether the sentences of the design documents match the current code and `openapi.yaml` (obsolescence, wrong references)
- Whether the same fact is duplicated across documents or sections, and whether the canonical place follows the division of responsibility
- Whether the read path (boundary resolution, listing, content, link) satisfies the API contract and the design's rules
- Whether there are gaps in the design that should be decided before starting M2 through M7
- Whether the identifiers and external representations in the terms document disagree with the names in the implementation

## 3. Overall Assessment

The skeleton of the design documents, and the descriptions of boundary resolution, error mapping, and storage identity and cache, match the code well.
After correcting the obsolete descriptions and duplications (R-15 through R-17) in the same change, the design documents can be used as is as the premise for work from M2 onward.
However, the 7 items R-4 through R-10 are open matters that should be decided when starting M2 through M7. 6 were registered as new deferred items, and for R-9 the conditions for deciding an existing deferred item were corrected.
The code defects are R-1 through R-3. R-1 is best fixed before connecting authentication in M2, and R-3 before building the Operation layer in M3. R-2, confined to configuration validation, has no particular timing.

## 4. Findings

| ID | Summary | Type | Confidence | Handling |
| --- | --- | --- | --- | --- |
| [R-1](#r-1) | Returns the targetPath of a broken symbolic link | Defect | High | issues.md broken-link-target-exposed |
| [R-2](#r-2) | Library id validation lets `.`, `..`, and duplicates through | Defect | High | issues.md library-id-validation-gaps |
| [R-3](#r-3) | The driver's rename does not accept ConflictPolicy | Defect | High | issues.md driver-rename-ignores-conflict |
| [R-4](#r-4) | The content response reads the whole requested range into memory | Design issue | High | design.md deferred item "Streaming file content responses" |
| [R-5](#r-5) | The folder listing cursor causes duplicates and omissions in a changing folder | Design issue | High | design.md deferred item "Paging folder listings" |
| [R-6](#r-6) | The thumbnail RocksDB makes the whole `state_dir` the DB directory | Design issue | High | design.md deferred item "Storage method for the thumbnail cache" |
| [R-7](#r-7) | Cannot build a thumbnail key for an archive entry without a modification time | Design issue | Medium | design.md deferred item "Cache key for thumbnails of entries in archives" |
| [R-8](#r-8) | Anyone on the LAN can take the initial setup first | Design issue | High | design.md deferred item "First-come registration in initial setup" |
| [R-9](#r-9) | The decision conditions of the TLS deferred item disagree with the actual trigger | Design issue | Medium | design.md deferred item "TLS and exposure outside the LAN" |
| [R-10](#r-10) | Extraction timeout and cancellation fall to 500 | Design issue | High | design.md deferred item "HTTP representation of archive extraction timeout and cancellation" |
| [R-11](#r-11) | A single request repeats NestedPath resolution many times | Improvement proposal | High | issues.md storage-resolve-repeated |
| [R-12](#r-12) | The identifier and per-layer names of Library in terms.md disagree with the code | Term | High | terms.md Library |
| [R-13](#r-13) | The public name `LocalStorageDriver` of `FileStorageLocal` is not in the terms document | Term | High | terms/storage.md FileStorageDriver |
| [R-14](#r-14) | Words the design documents use are not in the term list | Term | High | terms.md term list |
| [R-15](#r-15) | The design documents retain descriptions that disagree with the current code | Defect | High | Fixed within the same review |
| [R-16](#r-16) | Wrong section references and words from a rejected isolation proposal remain | Defect | High | Fixed within the same review |
| [R-17](#r-17) | The same fact is duplicated across sections and documents | Improvement proposal | High | Fixed within the same review |

R-1 and R-2 are confined to the daemon's Library layer and can be fixed in the same change.
R-3 and R-11 both change the shape of `StorageService`'s public API, so handling them together before M3 requires only one API change.
R-6 and R-7 are matters to be decided at the same M4 connection time, and if R-6 chooses the same method as the extraction cache, R-7's `EntryCacheKey` rides on the same mechanism.
R-4, R-5, and R-10 will surface at the same time, when the web client and iOS start handling large files and large folders.

<a id="r-1"></a>
### R-1. Returns the targetPath of a Broken Symbolic Link

**Type: Defect** / **Confidence: High**

**Location**

The [link branch of `EntryConverter::convert`](../../daemon/entrypoints/daemon/src/library.rs#L213) and [`EntryConverter::link_target`](../../daemon/entrypoints/daemon/src/library.rs#L247).
The contract is [`FileSystemEntryLinkDetail` in `openapi.yaml`](../../daemon/openapi.yaml#L1852).

**What is wrong**

Even for a link that points inside the Library but whose target does not exist, it puts a Library-relative path in `targetPath` and makes only `targetKind` null.
`openapi.yaml` and design.md §11.1 specify that the `targetPath` of a broken link is null.

**Basis**

`targetPath` is the result of `link_target` when the rank is 0 and the target is `Some`, and `link_target` does only lexical normalization and `strip_prefix`.
The [local driver](../../daemon/modules/engine/src/core/nfs/driver/local.rs#L119) returns the target as `Some` and `target_kind` as `None` even for a broken link, so the condition holds with this combination.
There are no tests that check the conversion of link entries.

**Impact**

A client shows a nonexistent path as the link target, and receives a 404 when following it.
With authentication always failing at present, it cannot be reached from HTTP, and it becomes apparent in M2.

**Handling**

Transcribed to broken-link-target-exposed in issues.md.

<a id="r-2"></a>
### R-2. Library id Validation Lets `.`, `..`, and Duplicates Through

**Type: Defect** / **Confidence: High**

**Location**

[`LibraryConfig::validate_id`](../../daemon/entrypoints/daemon/src/config.rs#L133) and [`LibraryBrowser::new`](../../daemon/entrypoints/daemon/src/library.rs#L66).

**What is wrong**

It does not reject ids of `.` and `..`, and duplicate ids are resolved last-wins when collected into a `HashMap`, so the earlier Library silently becomes invisible.

**Basis**

The conditions of `validate_id` are non-empty, 64 characters or fewer, and only `[A-Za-z0-9._-]`, all of which `..` satisfies.
Neither `DaemonConfig::load` nor `LibraryBrowser::new` has any processing that checks for duplicates.
design.md §11.1's "Validate Library ids at startup" gives failing on configuration mistakes before startup as the reason for the decision.

**Impact**

An id that is a dot segment cannot be routed through clients and relays that remove URL dot segments, and a duplicate does not expose the Library that was configured.
Startup succeeds in both cases, so the cause becomes apparent only when the API is called.

**Handling**

Transcribed to library-id-validation-gaps in issues.md.

<a id="r-3"></a>
### R-3. The Driver's rename Does Not Accept ConflictPolicy

**Type: Defect** / **Confidence: High**

**Location**

[`FileStorageDriver::rename`](../../daemon/modules/engine/src/core/nfs/driver.rs#L93), [`FileStorageLocal::rename`](../../daemon/modules/engine/src/core/nfs/driver/local.rs#L200), [`StorageService::rename`](../../daemon/modules/engine/src/service/storage.rs#L118), and [`StorageService::move_file`](../../daemon/modules/engine/src/service/storage.rs#L174).

**What is wrong**

The public `StorageService::rename` silently replaces an existing destination.
`move_file` bypasses the driver to respect the conflict and calls `StagingPublisher::apply` directly.
storage.md §8.4 had said "the driver returns only the success or failure of the rename", but move was not using the driver's rename.

**Basis**

The local driver's rename calls `tokio::fs::rename` as is, and has no `ConflictPolicy` in its arguments.
storage.md §8.3 requires no-replace publication for `Fail`, `Skip`, and `Rename`, and only `StagingFile::publish` and its internal `StagingPublisher` implement that.
No production code calls `StorageService::rename` yet.

**Impact**

When the Operation layer uses `StorageService::rename` for move in M3, it overwrites an existing entry under `conflict=fail`.
When a writable driver other than local is added, the move rules do not reach that driver.

**Handling**

Transcribed to driver-rename-ignores-conflict in issues.md.
storage.md §8.4's "the driver returns only the success or failure of the rename" was removed within the same review.

<a id="r-4"></a>
### R-4. The Content Response Reads the Whole Requested Range into Memory

**Type: Design issue** / **Confidence: High**

**Location**

[The buffer allocation of `LibraryBrowser::read_file_content`](../../daemon/entrypoints/daemon/src/library.rs#L140), [the response with `ByteArray`](../../daemon/entrypoints/daemon/src/server.rs#L157), and [the upper bound of `ReadLimitQuery`](../../daemon/openapi.yaml#L1222).

**What is wrong**

The daemon reads the requested range (up to 128 MiB) into a `Vec<u8>` before responding.
storage.md §5.2 had said "the API-side limit upper bound can be satisfied without putting it in the parent's memory", but neither the implementation nor the generated interface has that shape.

**Basis**

The body of the generated `ReadFileContentResponse` is `ByteArray`, and there is no path to pass a stream.
`ByteRange::parse` merely truncates an open-ended `bytes=N-` to `N + 128 MiB - 1`, so it allocates that whole range.

**Impact**

For a client that sends an open-ended `Range`, each request allocates up to 128 MiB and cannot return the first byte until it has read all of it.
Memory use grows by the number of concurrent requests.

**Handling**

How to fix it is a choice between sacrificing the spec-first generation path and the number of round trips, so we transcribed it to the design.md deferred item "Streaming file content responses".
The unmet description in storage.md §5.2 was replaced within the same review with a pointer to the deferred item.

<a id="r-5"></a>
### R-5. The Folder Listing Cursor Causes Duplicates and Omissions in a Changing Folder

**Type: Design issue** / **Confidence: High**

**Location**

The [collect-all](../../daemon/entrypoints/daemon/src/library.rs#L90) and [sort](../../daemon/entrypoints/daemon/src/library.rs#L98) in [`LibraryBrowser::list_folder_entries`](../../daemon/entrypoints/daemon/src/library.rs#L79), and [`ListCursor::encode`](../../daemon/entrypoints/daemon/src/library.rs#L283).

**What is wrong**

Each listing reads and sorts all entries, then cuts at the position the cursor points to.
If entries are added or removed between pages, the position shifts and duplicates or omissions occur.
With `recursive=true`, the whole tree is scanned for every page.

**Basis**

The cursor's content is `offset:<position>` and does not hold a reference value for sorting.
design.md §8.9 rejects a search cursor on the grounds that "reproducing the cutoff position requires a stable scan order", but the rules for the listing cursor are written in no design document.
The sort is a total order with the path as the second key, so using the pair of sort key and path as the cursor would prevent position shifts.

**Impact**

When paging a folder that keeps changing, such as during photo import, the display fluctuates.
A `recursive=true` on a large tree repeats the full scan in proportion to the number of pages.

**Handling**

Transcribed to the design.md deferred item "Paging folder listings".

<a id="r-6"></a>
### R-6. The Thumbnail RocksDB Makes the Whole `state_dir` the DB Directory

**Type: Design issue** / **Confidence: High**

**Location**

[`ThumbnailCache::new`](../../daemon/modules/engine/src/core/thumbnail/cache.rs#L17) and [the open in `KeyValueRocksdbStorage::new`](../../daemon/modules/engine/src/base/storage/key_value_rocksdb_storage.rs#L42).

**What is wrong**

`KeyValueRocksdbStorage` opens the directory it receives as RocksDB's directory as is.
design.md §9.2 makes that directory `state_dir`, so the `sqlite.db` to be preserved and the regenerable cache files sit side by side in the same directory.
This does not fit the reason for separating it from SQLite (separating the unit of backup).

**Basis**

It passes the `state_dir` argument as is to `open_cf_descriptors`.
On the other hand, the archive's extraction cache is placed in a dedicated directory `state_dir/storage/archive/`, and capacity is managed by LRU with a SQLite index and real files.
Only the thumbnail cache uses RocksDB.

**Impact**

Deleting the directory to discard the cache also deletes user information.
Capacity management of the thumbnail cache remains deferred, and the same problem as the extraction cache would be solved by two methods.

**Handling**

As an issue to choose before the M4 connection, we transcribed it to the design.md deferred item "Storage method for the thumbnail cache".

<a id="r-7"></a>
### R-7. Cannot Build a Thumbnail Key for an Archive Entry Without a Modification Time

**Type: Design issue** / **Confidence: Medium**

**Location**

The commented-out [`gen_cache_meta`](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L57).

**What is wrong**

The key in design.md §6 includes the modification time, and `gen_cache_meta` turns an entry whose modification time is `None` into `UnsupportedType`.
storage.md §4.5 treats an archive entry's modification time as a best-effort `Option`, and a thumbnail cannot be made for such an entry.

**Basis**

`modified` in `gen_cache_meta` is `ok_or_else(|| Error::new(ErrorKind::UnsupportedType))`.
We have not confirmed how often it becomes `None` in real archives.
The storage layer already has `EntryCacheKey`, a value derived from the outermost file's fingerprint that uniquely identifies an entry in an archive.

**Impact**

In an archive containing entries without a modification time, no thumbnail appears in the listing.
`gen_cache_meta` is not yet connected, so it does not occur at present.

**Handling**

Transcribed to the design.md deferred item "Cache key for thumbnails of entries in archives".

<a id="r-8"></a>
### R-8. Anyone on the LAN Can Take the Initial Setup First

**Type: Design issue** / **Confidence: High**

**Location**

"The first registrant becomes the only owner" in design.md §7.1 and §11.1, and [`listen_addr` in the development configuration](../../daemon/config/pxna.toml#L2).

**What is wrong**

While there are no users, `POST /setup` creates the owner with the first request regardless of the source.
The example configuration's `listen_addr` is `0.0.0.0:6050`, so another device on the LAN can register before the owner.
The reason for the decision had said "the window anyone could take over can be closed", but what is closed is only the window after registration.

**Basis**

We read the descriptions in design.md §7.1 and §11.1.
`POST /setup` is not yet implemented, so we have not confirmed the code's behavior.

**Impact**

In an environment where the operating premise of §2 (a trusted network) breaks down, the daemon can be taken over between first startup and owner registration.

**Handling**

Transcribed to the design.md deferred item "First-come registration in initial setup", and the overstatement in the reason for the decision was corrected within the same review.

<a id="r-9"></a>
### R-9. The Decision Conditions of the TLS Deferred Item Disagree with the Actual Trigger

**Type: Design issue** / **Confidence: Medium**

**Location**

design.md §11.2 "TLS and exposure outside the LAN".

**What is wrong**

The decision condition was "when access from outside the LAN is actually needed", but the same section said "the cookie is rejected as soon as another device on the LAN accesses it".
The "why not decide now" statement that "if limited to LAN use, it can be operated in plaintext" does not hold for the web client.

**Basis**

We confirmed the disagreement between the condition and the note by reading the section's body texts against each other.
That the session cookie has the `__Host-` prefix and the `Secure` attribute is specified by design.md §7.2 and the example in `openapi.yaml`.
The note itself, that the browser rejects this cookie on a plaintext LAN address, has not been checked against primary sources.

**Impact**

If the browser does reject the cookie as the note says, then judging from the deferred item's condition that "TLS can come later on a LAN" would leave the web client not working in M6.

**Handling**

Rewrote the deferred item "TLS and exposure outside the LAN" condition to list M6 first.

<a id="r-10"></a>
### R-10. Extraction Timeout and Cancellation Fall to 500

**Type: Design issue** / **Confidence: High**

**Location**

[The default branch of the conversion to `ApiError`](../../daemon/entrypoints/daemon/src/error.rs#L299) and [`ArchiveRuntime::run_blocking`](../../daemon/modules/engine/src/core/nfs/driver/archive/options.rs#L158).

**What is wrong**

The engine's `Timeout` and `Cancelled` are not in the table of design.md §4.4, and become `InternalServerError`, a 500.
A 500 after waiting 300 seconds by default cannot be told apart from a defect by the client.

**Basis**

The conversion in `error.rs` drops `Timeout` and `Cancelled` to the default branch, and the tests also pin that mapping.
`run_blocking` returns to the caller on expiry and the worker's result is discarded on completion, so retrying does not yield a cache hit.

**Impact**

When opening an archive that contains a large solid block, the user repeatedly sees a failure with an unclear reason.

**Handling**

Transcribed to the design.md deferred item "HTTP representation of archive extraction timeout and cancellation", and added to §4.4 a pointer to how kinds outside the table are handled.

<a id="r-11"></a>
### R-11. A Single Request Repeats NestedPath Resolution Many Times

**Type: Improvement proposal** / **Confidence: High**

**Location**

[`StorageService::resolve`](../../daemon/modules/engine/src/service/storage.rs#L198), [`LibraryPathResolver::resolve_inside`](../../daemon/entrypoints/daemon/src/path.rs#L117), and [`LibraryBrowser::read_file_content`](../../daemon/entrypoints/daemon/src/library.rs#L116).

**What is wrong**

A single `readFileContent` for `/photos/a.7z/dir/x.png` performs resolution from the outermost archive 3 times.
With nested archives, each resolution re-fetches the intermediate archive from the extraction cache.

**Basis**

`StorageService`'s public API receives a `NestedPath` every time, and `resolve` redoes the outermost file's `stat`, the fingerprint, the index cache lookup, and the materialization of the intermediate archive.
The boundary search, `get_entry`, and `open_file` each call `resolve`.
[`ExtractionCache::lookup`](../../daemon/modules/engine/src/core/nfs/driver/archive/cache.rs#L63) searches under the lock of the whole cache.

**Impact**

The result is correct and the cost of one call is small.
When thumbnails of archives with rank 2 or higher are requested concurrently in M4, duplicated lookups pile up as lock contention.

**Handling**

Transcribed to storage-resolve-repeated in issues.md as an improvement proposal of low severity.

<a id="r-12"></a>
### R-12. The Identifier and Per-Layer Names of Library in terms.md Disagree with the Code

**Type: Term** / **Confidence: High**

**Location**

The Library row of the term list in terms.md, and the Library body in §4.
On the code side, [`LibraryConfig`](../../daemon/entrypoints/daemon/src/config.rs#L122).

**What is wrong**

The identifier was `Library`, and the per-layer names were "`Library` in the configuration and the engine".
There is no type called `Library` in the hand-written code, and the daemon uses `LibraryConfig`.
design.md §4.1 says "the engine does not even know `libraryId`", so Library does not appear in the engine.

**Basis**

`rg 'struct Library\b'` returned only `models::Library`, the API schema in the generated interface.

**Impact**

An agent that looks up the identifier from the terms document searches for a nonexistent type or for Library on the engine side.

**Handling**

Corrected the Library row and body in terms.md to the correspondence of the configuration `[[libraries]]`, the daemon `LibraryConfig`, and the API `libraryId`.

<a id="r-13"></a>
### R-13. The Public Name `LocalStorageDriver` of `FileStorageLocal` Is Not in the Terms Document

**Type: Term** / **Confidence: High**

**Location**

[The re-export in the service layer](../../daemon/modules/engine/src/service/storage.rs#L15) and the daemon's [`state.rs`](../../daemon/entrypoints/daemon/src/state.rs#L4).

**What is wrong**

The engine's service layer re-exports `FileStorageLocal` under the name `LocalStorageDriver`, and the daemon uses only this name.
The same type has two names depending on the layer, but the terms document does not have this correspondence.

**Basis**

We read `pub use crate::core::nfs::{ ..., FileStorageLocal as LocalStorageDriver, ... }`.

**Impact**

A person reading the daemon cannot look up `LocalStorageDriver` in the term list and cannot connect it to `FileStorageLocal` in storage.md.

**Handling**

Added the correspondence with the public name to the per-layer names of FileStorageDriver in terms/storage.md.
Whether to eliminate the alias itself can be decided the next time the service layer's public surface is touched.

<a id="r-14"></a>
### R-14. Words the Design Documents Use Are Not in the Term List

**Type: Term** / **Confidence: High**

**Location**

The term list in terms.md.
Words used in design.md §4.3 and §4 through §7 of storage.md.

**What is wrong**

Library-relative path (`LibraryRelativePath`), ConflictPolicy, block, lease, rank, ArchiveOptions, EntryCacheKey, extraction cache (`ExtractionCache`), and fingerprint (`SourceFingerprint`), which the design documents use without definition, were not in the term list.
Library-relative path and NestedPath are a pair that, if confused, either skips the boundary check or leaks the host's path, but there was no description of the boundary.

**Basis**

We matched the identifier column of the term list against the words in the body of the design documents.
We confirmed with `rg` that each identifier exists in the code.

**Impact**

Readers of the design documents have to infer the definitions of words from the context of the design documents.

**Handling**

Added 9 rows to the term list in terms.md, and added the body for Library-relative path and the boundary between Library-relative path and NestedPath.
NestedPath itself does not appear in the API, so its external representation was set to `-`.
Added `NestedFsEntry` to the body of Entry, and the conversion in the daemon layer to NestedPath in terms/storage.md.

<a id="r-15"></a>
### R-15. The Design Documents Retain Descriptions That Disagree with the Current Code

**Type: Defect** / **Confidence: High**

**Location**

The following descriptions at the target commit.

- "This proposal remains in the current code" in the rejected alternative of storage.md §9.1 "Limit NestedPath resolution to the service layer".
- The current state of design.md §11.2 "Separating the admin port". [pxna.toml](../../daemon/config/pxna.toml#L1) does not have `listen_console_addr`.
- "Once M1 is reached, the contents can be seen from the browser" in design.md §12.2.
- "Read responses return an ETag" in design.md §11.2 "Optimistic locking with ETag / If-Match".
- The way to find unimplemented places in design.md §1.2, `rg 'todo!' daemon`.
- "The follow-up plan takes the implementation commits of N2/N3 as its target baseline" in storage.md §10.2.

**What is wrong**

All of them disagree with the current state of the code or the document family, and give readers wrong premises.

**Basis**

- The `FileStorage` dispatcher is not in the code, and §10.1 of the same document also says it has been deleted.
- `listen_console_addr` disappeared from the configuration in 7370a2f.
- The read endpoints are under `openapi.yaml`'s [default security](../../daemon/openapi.yaml#L65), and since the [claims extraction](../../daemon/entrypoints/daemon/src/server.rs#L38) always returns `None`, they give 401. There is also no serving of the web client.
- The [content response](../../daemon/entrypoints/daemon/src/server.rs#L161) returns `e_tag: None`.
- Unconnected handlers return [`not_implemented()`](../../daemon/entrypoints/daemon/src/server.rs#L455) rather than `todo!`, so `rg 'todo!' daemon` cannot find the 26 places.
- The referenced plan document was wrapped up in 407beab.

**Impact**

Overreading the state reached by M1, someone would try to check from the web client without waiting for M2 and M6, and be stopped by a 401.
The others make people search for code or documents that do not exist.

**Handling**

Fixed within the same review.
We added a state column to design.md §12.2 and rewrote it to say that the contents can be viewed once M1, M2, and M6 are in place.

<a id="r-16"></a>
### R-16. Wrong Section References and Words from a Rejected Isolation Proposal Remain

**Type: Defect** / **Confidence: High**

**Location**

The following descriptions at the target commit.

- "The staging area is as in §5" in design.md §8.7. §5 does not have that content, which has moved to storage.md §5.3.
- "(§4.2)" in storage.md §4.4. §4.2 is the decoding of archive entry names, and the daemon layer's interpretation is in design.md §11.1.
- "(§2.1)" in storage.md §7.3. The policy of not taking a content hash is in §2.2.
- "parent" in §4.3, §5.2, §6.5, §7.6, and §9.2 of storage.md.

**What is wrong**

The last item, "parent", refers to the parent process in the rejected process-isolation proposal.
The same document also uses "parent" to mean the parent archive, so one cannot tell which is meant.

**Basis**

storage.md §9.1 decides to "run the archive library in the same process as the daemon", so no parent process exists.
What does the verification in §6.5 and §7.6 is the outermost archive driver and the processing of publishing to the cache.

**Impact**

Readers search for the boundary of a child process that does not exist.

**Handling**

Fixed within the same review.
The heading of the deferred item in §9.2 was changed to "Strict daemon-wide memory admission".
There are no links to the old heading within the repository.

<a id="r-17"></a>
### R-17. The Same Fact Is Duplicated Across Sections and Documents

**Type: Improvement proposal** / **Confidence: High**

**Location**

§4.4, §6, §8.2, §8.3, §8.4, §8.7, and §8.9 of design.md and the corresponding decisions in §11.1.
The paragraph on case-insensitive file systems in design.md §8.3 and storage.md §8.3.
The daemon-side rules in storage.md §4.4, the single-flight sentence in §6.2, and the fingerprint matching in §7.3 and §6.5.

**What is wrong**

The detailed sections and the decided matters held the same reasons in nearly the same sentences, and design.md and storage.md held the same paragraph.
Fixing only one side causes a disagreement.

**Basis**

We matched the sentences of each pair.

**Impact**

Updating a reason requires two places, and one of them rots.

**Handling**

Fixed within the same review.
No unaddressed part remains, so no entry is filed in issues.md.
We left the reasons in the decisions of §11.1 and left only pointers from the detailed sections.
For the case-insensitivity paragraph, as a publication rule for one file, storage.md §8.3 was made the canonical source.

## 5. What Was Checked and Found Sound

- The boundary resolution of design.md §4.3 matches `LibraryPathResolver`, and `fast_path_uses_one_stat_test` pins that a local path is settled with one `stat`. `max_depth_test` also confirms that exceeding the limit on the number of boundaries gives `LimitExceeded`.
- The table of design.md §4.4 matched the conversions in `error.rs`. `SourceChanged` becomes 409, `InvalidFormat` becomes a 400 of `UnsupportedType`, and `PasswordRequired` becomes 403.
- The staging name `.pxna-tmp-` and the 24-hour reclaim threshold of storage.md §5.3 matched the constants in `staging.rs`.
- The tests in `format.rs` pin storage.md §6.1's "a mismatch between a known extension and the signature is `InvalidFormat`, and an unknown signature is `UnsupportedType`".
- The identity of storage.md §7.3 matches `identity.rs` and serializes each field with its length prefixed. `CACHE_FORMAT_VERSION` is 1.
- The "if initialization fails, delete everything and recreate once" of storage.md §7.4 is as in `ExtractionCache::open`.
- `PXNA_DAEMON_CONFIG_DIR` and `--config-dir`, the process lock of design.md §9.1, and SQLite's WAL of §9.2 are as in `executor.rs` and `db.rs`.
- The "CI checks that there is no diff against the regenerated interface" of design.md §12.1 is as in the steps of `test-daemon-slim.yml`.

## 6. Afterward

- 2026-09-25: Transcribed R-1 through R-3 and R-11 to issues.md, R-4 through R-10 to the deferred items of design.md, and R-12 through R-14 to the terms documents. For R-15 through R-17, the design documents were fixed in the same change.
