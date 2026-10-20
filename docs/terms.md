# Pxna Terms

This document is the single source of truth for Pxna-specific vocabulary.
Design rationale, implementation status, defects, and external behavior are each covered by their own documents.

## 1. About This Document

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | Definitions of terms, spelling, aliases, boundaries with neighboring terms, reasons for names |
| [design.md](./design.md#11-division-of-responsibility-between-documents) | Design rationale, invariants, implementation boundaries |
| [issues.md](./issues.md) | Clear defects confirmed in the code |

[design.md](./design.md#11-division-of-responsibility-between-documents) holds the map of the document family.

## 2. Term List

| Term | Scope | Identifier | External representation | Refers to | Aliases not used |
| --- | --- | --- | --- | --- | --- |
| ConflictPolicy | all | `ConflictPolicy` | `conflict` | How to handle an existing entry at the destination (fail, skip, overwrite, rename) | - |
| [Entry](#t-entry) | all | `FsEntry` | `FileSystemEntry` | Collective name for a file, folder, or symbolic link in a Library | file |
| [Library](#t-library) | all | `LibraryConfig` | `libraryId` | One directory the daemon exposes | root |
| [Library-relative path](#t-library-relative-path) | all | `LibraryRelativePath` | `RelativePath` | A path with which a client points at a location, treating the Library root as `/` | - |
| Operation | all | `Operation` | `operation` | The progress state of a mutating operation whose duration is proportional to the content size | job |
| Rotation | all | `rotation` | `rotation` | For images in formats read with ffmpeg, the counterclockwise angle by which the stored image is rotated for display. Internally one of 0, 90, 180, or 270 degrees; returned as an image field of the media info, separately from width and height, only when it is not 0 | - |
| Session | all | `Session` | `AuthSession` | The server-side authentication state created by login or initial setup and referenced by a cookie or a bearer token | login state |
| Session token | all | `SessionToken` | `accessToken`, `__Host-pxna_session` | The unguessable random string a client sends to refer to its session | access token |
| Thumbnail | all | `ThumbnailGenerator` | `thumbnail`, `variant=thumbnail` | A reduced image generated from an Entry for a tile in a listing. For a video, it is made from the first frame | preview |
| [Timeline](#t-timeline) | all | `TimelineService` | `/timeline/{libraryId}`, `timeline`, "Timeline" | A listing of a Library's images and videos in descending order of timeline date without folder hierarchy, and the screen showing it | flat view, chronological view |
| Upload | all | `Upload` | `upload` | A server-side session for sending a large file in parts | transfer |
| Owner | all | `UserRole::Owner` | `owner` | Pxna's only user, registered once by initial setup | administrator |
| Natural order | all | - | - | A name ordering that compares runs of digits as numbers and everything else without distinguishing case | - |
| Folder-like Entry | all | - | `foldersFirst`, `foldersOnly` | A collective name for an Entry whose `kind` is `folder` and an Entry whose file detail `archive` is true | - |
| block | storage | `ArchiveBlockId` | `-` | The unit extracted in one decode. One entry for ZIP and non-solid 7z | - |
| ResolvedStoragePath | storage | `ResolvedStoragePath` | - | A value that holds the resolved driver, the path within the namespace, and the archive identity for the duration of a request | - |
| ArchiveIdentity | storage | `ArchiveIdentity` | `-` | A SHA3-256 value that identifies one archive | archive key |
| ArchiveIndex | storage | `ArchiveIndex` | `-` | An index that holds the entries in an archive in path order | directory tree |
| ArchiveOptions | storage | `ArchiveOptions` | `[storage.archive]` | Settings for archive processing limits and cache capacity | - |
| ArchiveRuntime | storage | `ArchiveRuntime` | `-` | The archive reader's limits, the process-wide semaphore, the index cache, and the extraction cache | archive context |
| EntryCacheKey | storage | `EntryCacheKey` | `-` | A SHA3-256 value that identifies one entry in an archive in the extraction cache | - |
| extraction cache | storage | `ExtractionCache` | `-` | A persistent LRU cache that keeps extracted entries across restarts | - |
| [Archive](./terms/storage.md#t-archive) | storage | `FileStorageArchive` | `archive` | A ZIP or 7z archive file and its read namespace | compressed folder |
| [FileStorageDriver](./terms/storage.md#t-file-storage-driver) | storage | `FileStorageDriver` | `-` | An operation contract bound to one filesystem namespace | storage backend |
| lease | storage | `Lease` | `-` | A hold that prevents the physical file of a `MaterializedFile` from being deleted while in use | - |
| MaterializedFile | storage | `MaterializedFile` | `-` | A cloneable file handle that holds a physical path and a lease together | temporary file |
| [NestedPath](./terms/storage.md#t-nested-path) | storage | `NestedPath` | `-` | The engine-internal path that represents archive nesting | archive path |
| rank | storage | `NestedPath::rank` | `-` | The number of archive boundaries a `NestedPath` contains. 0 is a local path | - |
| OpenedFile | storage | `OpenedFile` | `-` | A value that holds an opened file handle and its lease at the same time | file handle |
| fingerprint | storage | `SourceFingerprint` | `-` | The pair of the outermost local file's size and its modification time in seconds and nanoseconds | - |
| StagingFile | storage | `StagingFile` | `-` | An unpublished write target placed in the same directory as the destination | temporary file |
| StorageService | storage | `StorageService` | `-` | The engine's storage component that resolves a `NestedPath` and selects a driver | storage manager |
| Listing snapshot | listing | `ListingSnapshot` | `snapshot` | The sequence of Entries obtained by scanning a listing target once, kept in memory so that pages can be cut from it | listing cache |
| Capabilities query | listing | `Capabilities` | `getCapabilities` | A query that returns whether the daemon provides an optional browsing API, separately from the success or failure of a target path | connection status |
| Summary listing | listing | `FolderSummaryListing` | `listFolderSummaries` | A listing that returns, page by page, the names, kinds, and archive hints of direct children without waiting for detailed attributes | detail listing |
| Detail batch | listing | `EntryDetailResult` | `getEntryDetails` | A read operation that returns the current attributes of up to 200 explicitly given paths, with per-item success or failure | attribute index |
| Browse index | listing | `SummaryIndex` | `[listing] summary_index_capacity_bytes` | Derived state that reuses summary listings of already browsed direct children across restarts after checking their version and expiry | search index |
| Child probe | listing | `ChildProbe` | `probeFolderChildren` | An operation that checks the expansion hint of a visible row with a shallow enumeration that stops at the first folder-like child | recursive check |
| Subtree search | listing | - | `recursive=true` with `query`, the default scope of search mode | A listing that recursively scans under a starting folder and returns Entries whose names match the condition | global search |
| Search mode | client | - | `/search/{libraryId}`, `q`, `path`, `flat=1`, `rating`, `tag` | The web screen where you choose, within the screen, a start (the whole Library or under a folder) and a scope (include descendants or only direct children), and search by name, rating, and tag. Both folder view and the timeline enter the same screen | search box, search page |
| Activity panel | client | - | - | A list, overlaid at the bottom right of the web client, of the progress and results of Operations and uploads | transfer panel |
| Info panel | client | - | `pxna.infoPanelOpen` (a `localStorage` key) | The web display, placed to the right of a listing, of the details and media info of the selected Entry | inspector |
| Narrow screen | client | - | - | A web screen whose width is under 768 CSS px | mobile layout |
| Wide screen | client | - | - | A web screen whose width is 768 CSS px or more | desktop layout |
| Original image | client | - | - | The image file itself in a Library, which the content API returns | full-size image |
| Tile | client | - | - | The display unit of one Entry arranged in a grid | cell |
| Tile size | client | - | - | The specification of tile size in a listing, chosen from five levels in the display preferences | thumbnail size |
| Flipbook playback | client | - | - | A display that cycles through the frames of a sprite sheet on a tile | preview playback |
| Slideshow | client | - | - | A display, inside the viewer, that automatically advances the open Entry at a fixed interval | autoplay |
| Rate mode | client | - | the viewer's "Rate mode" button, `r` | A state inside the viewer where ← records bad and moves to the next Entry, → records good and moves to the next Entry, and ↑ moves to the next Entry without rating | triage mode |
| [Viewer](./terms/client.md#t-viewer) | client | `Viewer` | `view` | The web screen that displays and plays images and videos in full screen. It has a filmstrip, a slideshow, and full-screen display | image display mode, slideshow mode |
| Filmstrip | client | - | - | A display, placed at the bottom edge of the image viewer on a wide screen, that lays out the Entries of the current listing horizontally in the same order as the listing | - |
| Folder view | client | - | `/browse/{libraryId}/…`, "Folder" | The web screen that lists the direct children of the folder selected in the tree | dir mode |
| Display preferences | client | `Preferences` | `/preferences` | The set of values for the web client's appearance and behavior that the owner chooses and saves on the server | settings |
| Size steps | thumbnail | - | `[thumbnail] sizes`, `display_sizes` | The fixed set of values to which the server rounds up the width and height of thumbnails and display images | - |
| Sprite sheet | thumbnail | `ThumbnailVariant::Sprite` | `variant=sprite`, `Pxna-Frame-Count` | A single image that lays out a video's frame sequence in one row | - |
| Representative thumbnail | thumbnail | - | - | The thumbnail of the first image or video found by looking at the direct children of a folder or archive in natural order | - |
| Display image | thumbnail | `ThumbnailVariant::Display` | `variant=display` | The image the viewer shows first, which the server shrinks for the screen while preserving the aspect ratio | preview image |
| Frame sequence | thumbnail | - | - | Multiple frames taken from positions decided as ratios of a video's duration | - |
| Direct playback | video | - | - | Playing a video by fetching the original video file from the content API | - |
| Transcoded playback | video | - | - | Playing a video that the server serves while converting it to HLS | transcode playback |
| Transcode session | video | - | - | The server-side state that holds the ffmpeg process and segments for the transcoded playback of one video | - |
| Shooting info | video | `ImageExif` | `shooting` | The shooting date and time, camera, lens, focal length, aperture, exposure time, and ISO sensitivity read from an image's Exif | EXIF info |
| Media info | video | - | `media-info` | The container, codec, duration, and resolution of a video, and the format, dimensions, and shooting info of an image, which the server examines and returns | - |
| Timeline date | timeline | `TimelineDate` | `timelineDate` | The date and time, without a time zone, by which the timeline orders Entries, decided from the shooting date and time or `creation_time` (or else the modification time) | - |
| Timeline date source | timeline | `TimelineDateSource` | `timelineDateSource` | Which of the shooting date and time, `creation_time`, the Entry's modification time, or the archive's modification time the timeline date came from | - |
| Timeline index | timeline | `TimelineIndex` | `state_dir/timeline/index.db` | A SQLite index, built by aggregation, that holds per Library the Entries to list in the timeline and their timeline dates | timeline cache |
| Aggregation | timeline | `TimelineScanner` | `/timeline/refresh`, `/timeline/status` | The process that scans a Library, reads the timeline dates of images and videos, and reflects them in the timeline index | indexing, scan |
| [Rating](./terms/rating-tags.md#t-rating) | rating-tags | `Rating` | `rating`, `RatingValue` | The good or bad value a user gives to an image or video. The unrated state is represented as none | star, score |
| Vocabulary | rating-tags | `RatingTagService::tags` | `GET /tags` | The list of tag words the user has created so far. Words removed from Entries remain and serve as completion candidates while typing | tag list |
| Tag | rating-tags | `TagName` | `tags` | A word the user freely attaches to an image or video. Up to 32 per Entry, each word up to 64 characters, case-sensitive | label |

## 3. Boundaries Between Neighboring Terms

<a id="t-library-vs-entry"></a>
#### Library and Entry

**Boundary**
A Library is the unit of configuration that decides the exposed scope, and an Entry is a resource inside it.

**What goes wrong if they are confused**
If a Library is treated as an Entry, the exposed scope of host paths and the unit to which `readonly` applies leak into operations on individual files.

<a id="t-entry-vs-archive"></a>
#### Entry and Archive

**Boundary**
An Archive is a subset of the Entries that appear as files, and is not a kind of Entry.

**What goes wrong if they are confused**
If Archive becomes an independent kind, download, copy, delete, and filtering by `kind`, which target files, miss archives.

<a id="t-library-relative-path-vs-nested-path"></a>
#### Library-relative path and NestedPath

**Boundary**
A Library-relative path is the path relative to the Library root that a client sends and receives. A NestedPath is the sequence of the host's absolute path and paths inside archives after the daemon layer has resolved the Library boundary and archive boundaries.

**What goes wrong if they are confused**
If a NestedPath is built directly from client input, the checks of the Library boundary and archive boundaries are skipped. If a NestedPath is put in a response as is, the host's absolute path leaks.

<a id="t-materialized-file-vs-opened-file"></a>
#### MaterializedFile and OpenedFile

**Boundary**
A MaterializedFile represents a physical file and a lease, and an OpenedFile adds a `tokio::fs::File` opened from it.

**What goes wrong if they are confused**
If an OpenedFile does not hold the lease, the open file breaks in environments, including Windows, where a cache file in use cannot be deleted.

<a id="t-archive-index-vs-archive-runtime"></a>
#### ArchiveIndex and ArchiveRuntime

**Boundary**
An ArchiveIndex represents the contents of one archive, and an ArchiveRuntime represents the limits, semaphore, and caches that apply to the readers as a whole.

**What goes wrong if they are confused**
If the limits are included in the per-archive index, the process cannot limit extraction concurrency consistently.

<a id="t-thumbnail-vs-display-image"></a>
#### Thumbnail and Display image

**Boundary**
A thumbnail is the image shown on a tile in a listing. It is made from the thumbnail size steps and fits in the thumbnail capacity budget.
A display image is the image shown in the viewer. It is shrunk to the display size steps while preserving the aspect ratio and fits in the display capacity budget.
Both come from the same thumbnail API and conversion path, and the request specifies the kind.

**What goes wrong if they are confused**
If a display image is used for a tile, the transfer volume of a listing grows by an order of magnitude.
If a thumbnail is requested with the display budget, flipping through photos in the viewer evicts the listing's thumbnails from the cache.

<a id="t-display-image-vs-original-image"></a>
#### Display image and Original image

**Boundary**
A display image is an image the server shrinks and compresses for the screen, and the thumbnail API returns it.
An original image is the image file itself in a Library, and the content API returns it.
The viewer shows the display image first and swaps in the original image when conditions allow.

**What goes wrong if they are confused**
If a display image is treated as the original image, detail lost to shrinking and compression does not come back when zoomed in, and downloads are also the shrunk image.
If original images are prefetched in place of display images, every flip of a photo adds a transfer of several MB for each neighboring image.

<a id="t-tile-size-vs-size-step"></a>
#### Tile size and Size steps

**Boundary**
A tile size is a five-level value with which the user chooses, in the display preferences, the reference size at which the web client draws tiles. On a wide screen it is the minimum length of a tile's side, and on a narrow screen it is the number of columns.
Size steps are the values to which the server rounds up the width and height of thumbnails and display images, and they are decided by the server's configuration.
The web client calculates the requested width and height from the tile size, and the server rounds them up to size steps.

**What goes wrong if they are confused**
If the web client decides the tile size on the premise of size step values, changing the steps in the server's configuration makes the resolution of the web tiles and the thumbnails disagree.

<a id="t-session-vs-upload"></a>
#### Session and Upload

**Boundary**
A session is the authentication state that refers to the logged-in user, and it expires 30 days after creation.
An Upload is the in-progress state of sending one file in parts, and the design documents call it an "upload session".
It has nothing to do with authentication and expires 24 hours after creation.

**What goes wrong if they are confused**
If the expiry or discarding of an Upload is treated as session expiry, merely interrupting a split upload logs the user out.

<a id="t-timeline-date-vs-shooting-info"></a>
#### Timeline date and Shooting info

**Boundary**
The shooting date and time in the shooting info is a value read from an image's Exif, and some images do not have it.
Every Entry listed in the timeline has a timeline date, and for an Entry without a shooting date and time it is decided from `creation_time` or the modification time.

**What goes wrong if they are confused**
If the timeline date is shown on screen as the shooting date and time, an image without a shooting date and time is presented with its modification time as the shooting date and time.

<a id="t-viewer-vs-slideshow"></a>
#### Viewer and Slideshow

**Boundary**
The viewer is the whole screen that opens images and videos in full screen, and the slideshow is one feature inside it (automatic advance at a fixed interval).
Even when the slideshow is stopped, the viewer stays open.

**What goes wrong if they are confused**
If the screen is named "slideshow mode", even the use of looking at one image at a time by hand reads as part of automatic advance, and it becomes impossible to decide whether a new feature belongs to the viewer or to automatic advance.

## 4. Individual Terms

<a id="t-entry"></a>
#### Entry

**Names by layer**
The engine's driver uses `FsEntry`, `StorageService` uses `NestedFsEntry`, which includes the outer path, and the API uses `FileSystemEntry`.

<a id="t-library"></a>
#### Library

**Names by layer**
The configuration uses `[[libraries]]`, the daemon uses `LibraryConfig`, and the API uses the schema `Library`. A location is pointed at by the pair of `libraryId` and a Library-relative path.
The engine does not know about Libraries, and this word does not appear in engine identifiers.

<a id="t-library-relative-path"></a>
#### Library-relative path

**Names by layer**
The API uses `RelativePath`, and the daemon uses `LibraryRelativePath`, which is checked and split into segments.

<a id="t-timeline"></a>
#### Timeline

**Names by layer**
The daemon uses `TimelineService`, the API uses paths under `/libraries/{libraryId}/timeline/`, and the web client uses the URL `/timeline/{libraryId}` and the screen "Timeline".

**Why this name**
"Flat view" was rejected because it expresses only that folder hierarchy was removed, and the name does not show that Entries are ordered by timeline date.
"Chronological view" reads naturally in Japanese, but it would make the Japanese term and the identifier word (`timeline`) differ, so it was rejected.
