# Thumbnail Design

## 1. About This Document

This document covers the mechanism that generates and caches thumbnails, sprite sheets, and display images from Entries, and defines responsibility boundaries, the order of generation, cache rules, and the reasons for adoption.
The intended readers are developers and AI agents who implement or review this concern.
It assumes the vocabulary of [terms.md](../terms.md).
The definitions of words are not in this document.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | Generating thumbnails, sprite sheets, and display images, determining targets, the order and concurrency of generation, caches, reasons for adoption, deferred items |
| [design.md](../design.md) | Pxna's overall scope, overall structure, boundaries between concerns, and cross-cutting design decisions |
| [Image Socket Design](./image-socket.md) | How the client says which images it wants, request classes and the order of items, delivery, revalidation |
| [Client Design](./client.md) | Which images each part of the web client wants and when, and the client's image cache |
| [Video Playback Design](./video.md) | Media info and conversion for playing videos in the viewer |
| [Storage Layer Design](./storage.md) | Resolving `NestedPath`, materializing entries in archives, the definition of `EntryCacheKey` |
| [terms.md](../terms.md) | Definitions of terms |
| [issues.md](../issues.md) | Clear defects confirmed in code and handling policies |
| [daemon/sockets/image.schema.json](../../daemon/sockets/image.schema.json) | The authoritative fields and values of an item and of the images returned for it |

The entry point of the design document family is [design.md](../design.md).

### 1.2 Tense of This Document

§1 through §10 describe the completed form in the present tense.
The gap between the current code and the completed form, and the remaining work in dependency order, are gathered in §11.
If you want to know what works now, read §11 first.

## 2. Responsibilities and Boundaries

This document covers the span from receiving an item with a checked `NestedPath` to handing the converted image to the image socket.
Images, videos, folders, and archives are all wanted by the client through the image socket ([Image Socket Design](./image-socket.md)), and `ThumbnailGenerator` chooses the generation method according to the kind of target.

Pxna's purpose is to quickly browse files on a home server from a phone or a browser ([design.md §2](../design.md#2-what-pxna-is)).
If the server shrinks before sending, the client can draw a listing without reading the original files.
The design of this document is to satisfy this one point, and assumes requests arrive at the same time for as many items as are in the listing.

The boundaries with other concerns are as follows.

| Matter | Authoritative source |
| --- | --- |
| The procedure for building a `NestedPath` from a Library-relative path and checking the boundary | [design.md §4.3](../design.md#43-resolving-library-and-archive-boundaries) |
| How to turn an entry in an archive into a readable file, and its cache | [Storage Layer Design §5.2](./storage.md#52-default-implementations-centered-on-materialize) |
| Which images the client wants, and when it drops them | [Client Design §4](./client.md#4-thumbnail-requests) |
| The order in which wanted items reach generation, and how images are sent and revalidated | [Image Socket Design §5](./image-socket.md#5-request-classes-and-order), [§6](./image-socket.md#6-delivery), [§7](./image-socket.md#7-revalidation) |
| How name order is compared | [design.md §11.1](../design.md#compare-name-order-in-natural-order) |
| The mapping from the engine's error kinds to HTTP status | [design.md §4.4](../design.md#44-error-model-and-http-mapping) |

### 2.1 What Is Not Built

- The whole Library is not scanned in the background to generate thumbnails in advance.
  This avoids having to follow changes and generating files nobody looks at.
- Compatibility with Susie plugins (loading Windows DLLs) is not provided.
  This avoids having a per-OS plugin mechanism.
  Formats the `image` crate cannot read are left to ffmpeg, with libheif only for images with alpha (§3).
- For animated GIFs and animated WebPs, only the first still frame becomes the thumbnail.
  This avoids having a path that extracts a frame sequence per image format.
- Delivering the original image for the viewer is not covered.
  The original image the viewer swaps in from the display image is returned by the content API.

## 3. Determining the Target

The kind of target is decided by extension.
For the same reason that archives are determined by extension alone, each entry does not need to be opened every time the inside of an archive is listed ([design.md §11.1](../design.md#represent-is-an-archive-in-file-detail-not-in-the-kind)).
The set of supported extensions is a property of the server, and the client is not made to hold the same set.

| Kind | Determination | Generation method |
| --- | --- | --- |
| Image | An extension the `image` crate can read | Shrunk with `ImageConverter` |
| HEIF images | The extensions `heic` and `heif` | Decode with ffmpeg 9 to a PPM and then shrink. If it has alpha, decode it with libheif's heif-dec instead |
| Other images | Extensions such as AVIF that are not read by the `image` crate but are decided to be read with ffmpeg | AVIF is decoded to a PPM and shrunk; AVIF with alpha uses heif-dec. JXL and JP2 keep the PNG path |
| Video | Video extensions decided to be read with ffmpeg | Extract the frame sequence with ffmpeg (§6) |
| Folders and archives | The Entry's kind and the archive flag | Return the representative thumbnail (§5) |

A file that matches none of these is `UnsupportedType`.
A file whose extension matches but whose contents cannot be read is also `UnsupportedType`.

The listing response puts the MIME type decided from the extension in the `metadata.contentType` of files that match this determination.
The client decides image and video tiles and the viewer from the `image/` and `video/` prefixes (the reason is in §10.1).
It is not put on files that match none of them.

An image encoded as a tile grid has one stream per tile, and the image is assembled from the stream group that bundles them.
Pointing at the first stream gives only one tile, so ffmpeg is not given a stream for images and is left to its default selection, which assembles the grid in ffmpeg 9.0 ([ffmpeg and HEIF tile grids](../knowledge/ffmpeg-heif-tile-grid.md)).
ffmpeg 9 also applies the rotation of `irot`.
heif-dec assembles the grid and applies the rotation and alpha for HEIC/HEIF/AVIF with alpha ([libheif's heif-dec](../knowledge/libheif-heif-dec.md)).

One decode of an image or a video also produces the **companions**: the thumbnails of the smallest three size steps in both `crop` and `fit`, and, when the request is a thumbnail or a sprite sheet, the other shape at the requested size ([§8.3](#83-keys), [§10.1](#decode-once-and-warm-the-companions)).
They are produced for the requests of `thumbnail`, `display`, and `sprite`, only on a primary cache miss.
The filmstrip of the viewer asks for `crop` at one of these steps, and a switch between `crop` and `fit` in the listing asks for the other shape, so both are answered from the cache without decoding the source again.

ffmpeg, ffprobe, and heif-dec are started by the daemon as external processes.
The path of a physical file is passed to them.
For an entry in an archive, it is made a physical file with `materialize_file` and then passed, and the lease is held until generation finishes.
If the lease were released, a file being read could be deleted by the extraction cache's LRU.

## 4. Size and Shape

### 4.1 Size Steps

The server rounds the requested `width` and `height` up, each to the smallest value in the size steps that is at least the request, before generating.
A request beyond the largest step is rounded to the largest step.
The default size steps are 128, 256, 512, and 1024 for thumbnails and 2048 and 4096 for display images.
The 4096 step of display images is used when the viewer shows the display image filling a high-pixel-density screen without swapping to the original image ([Client Design §3.4](./client.md#34-viewer)).
Whether a thumbnail or a display image is wanted is specified by the request, and the server rounds up within the respective steps.

We round up to keep the kinds of cache entries down to the number of steps.
If values computed by the client from the screen width or the device's pixel density were accepted as they are, nearly the same sized conversion results would line up in the cache as many times as there are values.
We round up rather than down because if the client shrinks when displaying, image quality is not lost.

The step values can be changed by configuration (§9).
The step values are included in the cache key, so requests after a change are regenerated with the new steps.

### 4.2 Shape and Output Format

The shape is specified by the API's `resize` and `anchor`.

| `resize` | Shape |
| --- | --- |
| `crop` | Scale until it covers the frame, and cut out to the frame's size at the position of `anchor` |
| `pad` | Scale until it fits in the frame, and fill the frame with a transparent margin |
| `manual` | Ignore the aspect ratio and make it the frame's size |
| `fit` | Fit in the frame preserving the aspect ratio, with no margin added. Never made larger than the original |

`fit` is used for display images and for thumbnails that fit the whole in a tile ([Client Design §3.2](./client.md#32-tiles)).
The other three return an image of the frame's size, so they have no means of obtaining an image of the same aspect ratio as the original.

The output format is chosen from WebP, JPEG, and PNG.
The configuration `[thumbnail] format` decides it, and its default is WebP.
WebP and JPEG are compressed lossily, and for quality, `quality` is used for thumbnails and sprite sheets and `display_quality` for display images.
Display images are shown filling the screen in the viewer, so the coarseness of compression is more noticeable than for thumbnails seen at tile size.
For that reason the quality values are separated.
PNG is compressed losslessly and does not use a quality value.

libwebp is used for lossy WebP encoding.
This is because the image crate's WebP encoder has only lossless compression ([The image crate's WebP encoder](../knowledge/image-webp-encoder.md) in [knowledge.md](../knowledge.md)).
JPEG cannot represent transparency, so the combination of `pad` and JPEG is `InvalidRequest`.
WebP and PNG can represent transparency, so they can be combined with `pad`.

Input images are shrunk after applying the EXIF orientation.
Without applying the orientation, a photo taken vertically on a phone would line up sideways on the tile.

A display image is an image shrunk to the display steps with `fit`.
It goes through the same conversion path as thumbnails, but the cache capacity budget is separate (§8.2).

## 5. Representative Thumbnails

When the path of a folder or archive is requested, the server compares its direct children Entries in natural order of names and returns the thumbnail of the first image or video that appears.
No distinction is made between images and videos.
Hidden Entries whose names start with `.` and symbolic links are not chosen.
This is because ZIPs created on macOS contain working entries starting with `._`, which come first in natural order and cannot be read as images.
If there are no images or videos among the direct children, it is `NotFound` and the client shows an icon.

It does not descend into subfolders.
Descending would make the number of Entries traversed for one tile grow without limit.
With direct children only, one listing per tile suffices.
It does not need to sort all the direct children, and the smallest name can be chosen in one scan.

The selection of the representative is reused after checking the target's version and a short expiry ([design decision](#representative-selection-cache)).
The chosen Entry's attributes are checked on every request, and the cache key of the image itself and the cache key of the representative selection are kept separate.
The direct children of an archive can be looked up from the index cache, so each file is not extracted to make the selection.

An item for a representative thumbnail names the folder or archive, so it stays the same when the chosen Entry changes.
Its `ETag` is the chosen Entry's key, and the client revalidates every want with the `ETag` of the image it holds, as for any image ([Image Socket Design §7](./image-socket.md#7-revalidation)).
The server compares it with the key of the Entry chosen now, so a new representative is sent at once, and while it is the same the answer is `unchanged` and the image is not transferred again ([design decision](#representative-revalidation)).

## 6. Video Frame Sequence

### 6.1 Positions to Extract

From a video, frames at multiple positions decided as ratios of the duration are extracted.
The duration is read from the media info obtained with ffprobe ([Video Playback Design §3](./video.md#3-media-info)).
By default, 8 points are taken at even intervals from 10% to 90% of the duration.
We avoid the 0% at the start because the beginning of a video can be a black screen or a logo, and the contents cannot be told apart.

The first of the extracted frames is the video's thumbnail.
All the frames are lined up in one row to make one sprite sheet.
The client switches the display position to do flipbook playback ([Client Design §3.3](./client.md#33-flipbook-playback)).

The number of frames, the start point, and the end point are held as arguments to the generation process and passed from defaults in code.
Whether to make them specifiable from the configuration file is recorded as a deferred item in §10.2.
The three values are included in the cache key (§8.3).

### 6.2 Generate Together

Whether a thumbnail or a sprite sheet is requested, one run of ffmpeg extracts all the frames and puts both in the cache.
Generating them separately would pay twice for starting ffmpeg and for materializing a video in an archive.
Done once, these costs are paid only once.
If both are in place at the moment a tile becomes visible, flipbook playback can start at the instant of mouse-over or long press.

Both are in place only when the thumbnail and the sprite sheet are requested with the same width, height, and shape.
The key includes the conversion conditions (§8.3), so a request with different conditions does not hit the other request's cache and becomes a separate run.
The companions (§3) are the exception: they use the default format and the thumbnail quality, so they are in place for every thumbnail, sprite sheet, and display request of an image or a video.

A video keeps only the first frame at full size. The primary thumbnail and every companion are resized from it, and the remaining frames are decoded, resized, and dropped one at a time, as before, so eight full-size frames are never held at once.
No other frame is extracted or decoded for the companions.

Each position is reached at keyframe granularity, allowing a deviation from the specified time.
Frames for browsing in a listing do not need an exact time, and the less is decoded, the faster it is.

The response returning a sprite sheet includes the number of frames.
The client divides the sprite sheet equally by this value.

A video's display image is made from just one frame at the first position of the frame sequence, and a sprite sheet is not made.
A display image is a single still image preserving the aspect ratio, and does not need a frame sequence.
The position depends on the ratio of the start point, so the key includes the number of frames and the position even for the display image.

### 6.3 Failure and Timeout

If ffmpeg could not read it as a video, it is `UnsupportedType`.
Each run has a deadline of `video_timeout_secs`, and when it passes the process is killed and it is `Timeout`.
`Timeout` is returned as 504 in the API ([design.md §11.1](../design.md#return-extraction-and-generation-deadline-expiry-as-504)).

## 7. Order and Concurrency of Generation

Generation is done when an item of the image socket needs it.
Which item reaches generation when is decided by the wanted sets of the sockets: each connection passes its items in the order of request class and then order, a few at a time, so items that left the screen do not take the slots ([Image Socket Design §5](./image-socket.md#5-request-classes-and-order)).

**The display image a viewer shows now is Interactive.**
An `interactive` item is Interactive work, and `visible` and `prefetch` items are Foreground work.
That class is what the generation asks for its conversion slot with; the storage reads and the external processes inside the generation run as Foreground, so an Interactive item does not hold the listing enumeration behind it.
Image conversion, the ffmpeg and heif-dec runs for images, and the ffmpeg runs for videos are each one resource of the scheduler ([Storage Layer Design §9.1](./storage.md#listing-work-budget)).
While an Interactive generation runs or waits in a resource, no new Foreground generation starts there, and no Background work starts anywhere.
Interactive also has slots of its own besides the shared slots, so a display image does not wait for tile generations that already started.
Each resource runs the work of one class in arrival order up to its limit on concurrency.

**A source has at most one source job decoding it, in the whole daemon.**
An item whose image is already in the cache is answered from it and needs no job.
Every other item that reaches generation, from any connection, belongs to the source job of its source: the `SourceKey` of the file, or of the Entry chosen as the representative of a folder or an archive (§8.3).
An item is placed by the first of the following that applies.

1. The source has a job decoding it, and the job's plans include the item's key, as a requested plan or as a companion. The item waits for that job and is answered from it.
2. The source has a job waiting for its slot. The item joins it, and its plan is added to the job's plans. If the item is Interactive and the job waits as Foreground, the job waits again as Interactive.
3. The source has a job decoding it, and its plans do not include the item's key. The item starts the source's next job, which starts waiting for its slot when the decoding job ends. A display image waits in this way too.
4. The source has no job. The item starts one.

When a job gets its slot, its plans are fixed: the plans of its items and the companions of each, without duplicates (§3, [design decision](#decode-once-and-warm-the-companions)).
The images that the cache already has are left out, and the items that want them read the cache after the job.
The job decodes the source once and makes all the rest, releases its slot, stores them in one batch, and answers each of its items with its own image.
If a job fails to decode or read the source, every item of the job gets that error, including an item that waited for it as a companion.
If a job leaves no image for an item, because the store failed or the entry exceeded the bucket and was evicted, the item places itself again, at most 3 times.
A video's thumbnail and sprite sheet are plans of the same job, so one ffmpeg run makes both (§6.2).
A job still waiting for its slot is discarded when all its items are dropped or their connections close. A job that has started runs to the end and stores its images even if no item is left.

A local file whose modification time cannot be obtained has no `SourceKey`, so it is neither cached nor merged into a source job.
Each of its items is generated on its own, and dropping the item stops the generation.

We do not stop a generation once started because, in scrolling back and forth through a listing, a tile that once left the screen may come back.
If the conversion spent so far were discarded, the same cost would be paid again on return.
Image conversion runs on a blocking worker and there is no means of stopping it midway either.

## 8. Cache

### 8.1 Placement

The cache is held as a SQLite index plus real files.
The placement matches the archive's extraction cache ([Storage Layer Design §7.4](./storage.md#74-placement-of-the-extraction-cache)).

```text
state_dir/thumbnail/
├── cache.sqlite3
├── files/
│   └── ab/
│       └── ab...<key>
└── staging/
```

The directory is separate from `state_dir/sqlite.db`, which holds users and sessions, so what is to be preserved and the regenerable cache can be excluded from backups by directory.

Initialization failures, corruption while running, and publishing and reconciling generated results are handled by the same rules as the extraction cache ([Storage Layer Design §7.4](./storage.md#74-placement-of-the-extraction-cache), [§7.6](./storage.md#76-publishing-and-reconciliation)).
The contents are regenerable, so if initialization fails, `thumbnail/` is recreated and retried once.

### 8.2 Capacity Budgets and LRU

The cache is an LRU that deletes starting from the oldest `last_accessed`, and has no TTL.
The `last_accessed` of reads is accumulated in memory and reflected together after 500 ms or before the next write's eviction. There was a case where a SQLite write stretched to 0.8 seconds (diagnosis F3), and this is so that responses do not wait for its completion. If restarted before it is reflected, that usage order is lost.
The capacity limit is split into a budget for thumbnails and sprite sheets and a budget for display images.

We split the budgets because the sizes differ by an order of magnitude.
If put in the same budget, merely flipping through a few dozen photos in the viewer would evict the listing's thumbnails, and they would have to be regenerated on returning to the listing.

The companions are ordinary entries of the thumbnail budget. A generation stores all its entries in one batch, in the order companions, requested image, sprite sheet, so that a small budget evicts the oldest companions and not the image that was just requested.
The batch takes the lock once, reflects the pending recency once, writes the index in one transaction with `last_accessed` following that order, and evicts once per budget. The files are staged outside the lock.
The generation releases its scheduler slot when the conversion ends, before the batch, so the store does not hold the slot of the next generation. The response still waits for the batch.
The cache's SQLite uses `synchronous=NORMAL`. The cache is regenerable and a power loss may lose the latest entries, which the reconciliation at open drops; `sqlite.db` keeps its own setting.
An entry that alone exceeds the budget is skipped and is not an error. A failed store is logged. Neither fails the response.

### 8.3 Keys

The key is a SHA3-256 value combining a value that points to the original Entry and the conversion conditions.

```text
SourceKey(local)   = H(THUMBNAIL_FORMAT_VERSION, "local", abs_path, size, mtime_s, mtime_ns)
SourceKey(archive) = H(THUMBNAIL_FORMAT_VERSION, "archive", EntryCacheKey)
ThumbnailKey       = H(THUMBNAIL_FORMAT_VERSION, SourceKey, kind, resize, anchor, resampler, format, quality, width step, height step, frame count, start, end)
```

`kind` is the distinction among thumbnail, sprite sheet, and display image.
`quality` is the quality value used for that kind and format, and is not included for PNG.
The three frame values are included only for videos.
If the format or quality is changed by configuration, requests after the change are regenerated with new keys, and old conversion results are no longer referenced and disappear by LRU.

For a local file, if the size or modification time changes it becomes a different key, so the old thumbnail of a replaced file is no longer referenced and eventually disappears by LRU.
A local file whose modification time cannot be obtained is not cached.

An entry in an archive may not have a modification time depending on the format ([Storage Layer Design §4.5](./storage.md#45-fsentry-and-metadata)).
For that reason the storage layer's `EntryCacheKey` is used as the value for the original Entry.
`EntryCacheKey` is derived from the outermost file's fingerprint, so if the outer archive changes, all the inner thumbnails miss ([Storage Layer Design §7.3](./storage.md#73-cache-identity)).

A representative thumbnail uses the chosen Entry's `ThumbnailKey` as it is.
The cache is one whether the same image is requested as a file or as a folder's representative.

A companion is a thumbnail whose key is computed with the same `SourceKey`, the `thumbnail` kind, the center anchor, `Lanczos3`, the configured default format, the thumbnail `quality`, and a width and height step.
The keys are collected as a set, and the primary key and duplicates are dropped. The sprite sheet's `kind` differs, so it never needs comparing.
Because the keys are the same as those of a direct request, a companion and a request for it are one cache entry. The primary's keys and bytes do not change.

If the conversion implementation, the encoder's quality, or the way frames are extracted is changed, raise `THUMBNAIL_FORMAT_VERSION`.
Adding companions changes neither the primary's key nor its bytes, so the version stays.

## 9. Configuration

### 9.1 Where the Configuration Belongs

The thumbnail configuration goes in the top-level `[thumbnail]`, and the locations of the ffmpeg, ffprobe, and heif-dec executables go in `[media]`, shared with video playback.
Byte sizes, counts, times, and parallelism allow only positive values and are validated at daemon startup.

| key | Default | Applies to |
| --- | ---: | --- |
| `[thumbnail] sizes` | `[128, 256, 512, 1024]` | Size steps of thumbnails and sprite sheets |
| `[thumbnail] display_sizes` | `[2048, 4096]` | Size steps of display images |
| `[thumbnail] format` | `"webp"` | Output format of thumbnails, sprite sheets, and display images (`webp`, `jpeg`, `png`) |
| `[thumbnail] quality` | 80 | Quality of lossy compression for thumbnails and sprite sheets (1 to 100) |
| `[thumbnail] display_quality` | 90 | Quality of lossy compression for display images (1 to 100) |
| `[thumbnail] cache_capacity_bytes` | 2 GiB | Capacity budget for thumbnails and sprite sheets |
| `[thumbnail] display_cache_capacity_bytes` | 2 GiB | Capacity budget for display images |
| `[scheduler] image_slots` | 4 | Number of image conversions running at the same time for Interactive and Foreground |
| `[scheduler] image_interactive_slots` | 1 | Additional image conversions that only Interactive uses |
| `[scheduler] process_slots` | 2 | Number of ffmpeg and heif-dec runs for images (HEIF, AVIF, JXL, JP2) at the same time for Interactive and Foreground (heif-dec is used for alpha and as a fallback) |
| `[scheduler] process_interactive_slots` | 1 | Additional ffmpeg and heif-dec runs for images that only Interactive uses |
| `[scheduler] video_slots` | 1 | Number of ffmpeg runs for videos at the same time for Interactive and Foreground, apart from `process_slots` |
| `[scheduler] video_interactive_slots` | 1 | Additional video ffmpeg runs that only Interactive uses |
| `[thumbnail] video_timeout_secs` | 60 | Deadline for one frame extraction |
| `[media] ffmpeg_path` | `ffmpeg` | The ffmpeg executable |
| `[media] ffprobe_path` | `ffprobe` | The ffprobe executable |
| `[media] heif_dec_path` | `heif-dec` | libheif's heif-dec executable, from libheif 1.17 or newer, used for HEIC/HEIF/AVIF with alpha and as a fallback. TIFF is used when the build supports it (1.19 or newer with libtiff); otherwise the decode uses a level-0 PNG |

The defaults of `[media]` are names searched from `PATH`.
The Docker image for distribution bundles all three ([design.md §9.3](../design.md#93-distribution)).

## 10. Design Decisions

### 10.1 Decided

#### Generate When a Request Is Received

**Decision**
Thumbnails are generated when a client wants them through the image socket, and there is no bulk generation in the background.

**Reason**
The folders being browsed are a small part of the Library, and converting files nobody sees only costs.
With generation on request, there is also no need for a mechanism to track files changing externally.
If the key includes size and modification time, the file will be generated as a new Entry the next time it is requested.

**Rejected alternative**
A method that scans the whole Library in the background and generates thumbnails in advance.
Even a folder opened for the first time would not need to be waited on, but it would generate even files nobody looks at, and need a mechanism to keep tracking additions and changes of files, so we rejected it.

#### Generate Video Frames Together When the Tile Becomes Visible

**Decision**
For a video's thumbnail and sprite sheet, whichever is requested, both are made in one run of ffmpeg (§6.2).

**Reason**
Starting ffmpeg and materializing a video in an archive are done only once.
If the sprite sheet is already in the cache at the time of the operation that starts flipbook playback, playback starts without waiting.

**Rejected alternative**
A method that makes only one frame when the tile becomes visible and makes the rest at the time of mouse-over or long press.
It reduces the conversion volume for videos that are only looked at, but every time flipbook playback starts for the first time there would be a wait to reopen the video, so we rejected it.

<a id="decode-once-and-warm-the-companions"></a>
#### Decode Once and Warm the Companions

**Decision**
One generation of an image or a video decodes the source once, and from that decode produces the requested image, the smallest three size steps in both `crop` and `fit`, and the other shape at the requested size of a thumbnail or a sprite sheet (§3).
The companions use the default format, the thumbnail quality, `Lanczos3`, and the center anchor, and are stored in the thumbnail budget before the requested image (§8.2).
They are made only on a primary cache miss, synchronously in the same job, and best effort: a companion that fails to validate or encode is logged and dropped, and the response does not fail.
The companions already in the cache are found by the index alone and are not made again.

**Reason**
The decode is the expensive part: a HEIC takes far longer to decode than to shrink and encode. The listing decodes a source for the size it asked for, and the viewer decoded the same source again for the filmstrip, and a switch of the thumbnail shape decoded every visible source again.
Shrinking an already decoded image to a few more sizes costs little next to that.

**Rejected alternative 1**
Producing the display image together with the listing's thumbnail. It roughly doubles the listing's generation time and is wasted for images that are never opened.

**Rejected alternative 2**
Keeping the decoded source on disk as PNG, PPM, or WebP. Encoding it costs more than the HEIC decode it would save.

**Rejected alternative 3**
Asking ffmpeg to write WebP directly. With fan-out it needs several outputs and a second implementation of the shape and size rules.

**Rejected alternative 4**
Lowering libwebp's `method` below 2, or changing the display image's output size, to make a generation cheaper.

**Rejected alternative 5**
Decoding tiles of a tile grid in parallel ourselves. ffmpeg already does what it can.

**Residual risk**
The companions use thumbnail budget that the requested images would otherwise use, so a small budget loses older thumbnails sooner. The measurement ([record](../measurements.md#2-record-list)) records the bytes added per image.
The batch store keeps the items waiting for the store; moving it to the background is a separate change if it matters.

<a id="one-source-job-per-source"></a>
#### Merge Every Item for a Source into One Source Job

**Decision**
All the items for one source, from every connection, are merged into one source job, and a source has at most one job decoding it at a time (§7).
An item that a decoding job makes, as a requested plan or a companion, waits for it; any other item, a display image included, waits for the next job of the source.

**Reason**
The decode is the expensive part, so the number of decodes is what the merging has to bound. In the 2026-10-26 log of browsing a library of iPhone HEIC with the viewer, 8 of 90 external decodes ran while another decode of the same source ran. In 7 of them the request was for a companion of the running generation, such as the filmstrip's 128 `crop` next to the listing's 512 `fit`, or a thumbnail next to the display image; the other one was a display image.
Merging by source rather than by key lets one decode answer every size that is wanted before the job starts, and the companions of a running job answer most of what is wanted after.
Keeping one decoding job per source also keeps one image from taking two slots and competing with itself for the CPU. A HEIC decodes in about 0.7 seconds ([HEIF decision](#read-heif-and-avif-as-ppm-and-use-heif-dec-for-alpha)), which bounds what a display image waits behind a thumbnail job of the same image.

**Rejected alternative 1**
Merging only requests for the same cache key by single-flight. This was the form before source jobs, and a request for another size decoded the source again while a generation of it ran.

**Rejected alternative 2**
Keeping single-flight by key and registering the companion keys of a running generation, so that a request for one of them waits. It removes most of the duplicates seen in the log, but a size outside the companions and a display image still decode a second time, and it adds a second lookup beside the flights.

**Rejected alternative 3**
Merging only the items of one connection. It removes the duplicates between the parts of one tab, but two tabs or two devices showing the same folder would still decode each source twice.

**Rejected alternative 4**
Letting a display image start a second decode while a job of its source runs. The viewer would not wait for the thumbnail job, but the image would be decoded twice at the same time, and the two decodes would slow each other.

**Residual risk**
A display image waits behind a decoding job of its own source. For a video, that job extracts every frame of the sprite sheet, which took 22 s for an action-camera video (§10.1 "Give Videos Their Own External Process Slots").

#### Give Videos Their Own External Process Slots

**Decision**
The ffmpeg runs for videos (frame extraction and the video service's runs) have their own pool, `video_slots` (default 1) and `video_interactive_slots` (default 1). `process_slots` (default 2) is only for the ffmpeg and heif-dec runs that decode images.

**Reason**
In a real log, extracting frames from an action-camera video took 22 s and decoding a HEIC took 10 to 16 s. With one shared pool, a video held a slot that HEIC tiles needed, and `thumbnail_permit_wait` reached 38 s.

**Rejected alternative**
Keeping one pool and raising its size. It would let several slow videos run at once and compete for the CPU with the image decodes.

#### Return the Frame Sequence as a Sprite Sheet

**Decision**
A video's frame sequence is returned in one response as a single image laid out horizontally.

**Reason**
With many videos in a listing, per-frame requests would be the product of the number of tiles and the number of frames.
Combined into one, a frame sequence needs only one request.

**Rejected alternative**
A method that fetches one at a time by specifying the `frame` number.
The cache and the API would be simpler, but the number of requests would grow by the number of frames, so we rejected it.

#### Call ffmpeg as an External Process

**Decision**
Videos, and images the `image` crate cannot read, are read by starting ffmpeg as an external process.

**Reason**
ffmpeg is not linked into the daemon, so the build does not need ffmpeg's development libraries.
If ffmpeg terminates abnormally on a broken video, the daemon is not affected, and a process past its deadline can be stopped by killing it.
The cost of needing ffmpeg in the execution environment is paid by making the Docker image bundling ffmpeg the basis of distribution ([design.md §11.1](../design.md#make-the-docker-image-the-basis-of-distribution)).

**Rejected alternative**
A method that links ffmpeg as a library with bindings such as `ffmpeg-next`.
The dependency on the execution environment goes away, but the 3-OS CI would have to prepare ffmpeg's development libraries, and decoder anomalies would spread to the daemon's process, so we rejected it.

#### Read HEIF and AVIF as PPM, and Use heif-dec for Alpha

**Decision**
HEIC/HEIF and AVIF are decoded by ffmpeg 9 to a PPM (P6, 8-bit RGB) on stdout, then resized and encoded in Rust.
JXL and JP2 keep the PNG path, because ffmpeg's JXL decode carries the alpha channel in the primary stream and the current PNG path preserves it.
HEIC/HEIF/AVIF images with alpha are decoded with libheif's heif-dec instead, so the alpha channel is kept.
If ffmpeg fails for a HEIC/HEIF/AVIF image, the cause is logged and heif-dec reads it.
If alpha was detected and heif-dec fails, the request fails instead of silently dropping the transparency.
If the alpha probe fails, HEIC/HEIF/AVIF are decoded with heif-dec rather than risk dropping alpha.
An image has alpha when ffprobe returns a gray video stream that stands on its own and has the size of the primary image or of one of its tiles. A stream stands on its own when it is not the primary stream, not `dependent`, and not a member of any stream group. The primary image is the default `Tile Grid`, or the first one when none is the default; without a grid, it is the default video stream, or the first video stream.
heif-dec writes an uncompressed TIFF when the build supports it, and a level-0 PNG otherwise. The TIFF holds the same 8-bit pixels as the PNG, with alpha when the image has it, but skips the PNG filtering pass.
ffmpeg and ffprobe must be version 9 or newer at startup. heif-dec must be libheif 1.17 or newer, which is checked at startup by `--png-compression-level` appearing in its help. The decode asks for a TIFF first and retries with a level-0 PNG when the output suffix or libtiff is missing; TIFF output needs libheif 1.19 and a build with libtiff. heif-dec is required at startup on Linux and macOS; on Windows it is only needed when an image with alpha is requested.

**Reason**
heif-dec writes the decoded image as an uncompressed TIFF when it can. On a 12MP alpha HEIC, TIFF took 0.38 seconds (0.24 tiled) against 0.64 (0.51 tiled) for a level-0 PNG; both intermediates are 49 MB, so the PNG filtering pass is what the TIFF avoids, and the Rust image crate decodes the TIFF in 52 ms against 42 ms for the PNG ([libheif's heif-dec](../knowledge/libheif-heif-dec.md)).
The fallback to a level-0 PNG keeps builds without libtiff working. Ubuntu 26.04's libheif 1.21.2 lists `tif` in its help but is built without libtiff and fails with `TIFF support has not been compiled in.` ([libheif's heif-dec](../knowledge/libheif-heif-dec.md)).
The PPM path replaced heif-dec for images without alpha: in the measurement that introduced it, PPM took 0.13 to 0.20 seconds against 0.61 to 0.74 for heif-dec's PNG, making a first display image about 2.3 times faster and a thumbnail about 3.5 times faster ([measurement](../measurements/2026-10-24-heif-ppm.md)). TIFF narrows that gap but does not remove it.
ffmpeg 9 assembles tile grids, applies `irot`, and writes PPM with the dimensions after rotation ([ffmpeg and HEIF tile grids](../knowledge/ffmpeg-heif-tile-grid.md)). PPM is uncompressed pixels with a small header, so it is as fast as raw output and does not need a separate size probe.
JXL keeps the PNG path because its alpha is in the primary stream and already preserved there; moving it to PPM would drop the transparency for no benefit to the formats this change is about.
ffmpeg's default selection does not read the auxiliary alpha image of HEIC/AVIF, while libheif does, including tiled and rotated images. That is why alpha is the one case that stays with heif-dec ([libheif's heif-dec](../knowledge/libheif-heif-dec.md)).
ffprobe shows alpha and the other auxiliary images alike as gray video streams, and only how the streams are grouped and sized tells them apart ([ffprobe and HEIC auxiliary images](../knowledge/ffprobe-heic-auxiliary-images.md)). An iPhone HEIC carries a depth map, an HDR gain map, and segmentation mattes as gray streams. Under the earlier rule, which took any gray stream next to another video stream for alpha, every such photo went to heif-dec. In the logs of a library of iPhone photos, 78 of 90 HEIC decodes took that path at 0.32 seconds per megapixel, against 0.043 for the PPM path, and one 24-megapixel image took 0.7 seconds with ffmpeg and 12 seconds with heif-dec.
The rule keeps the alpha of untiled and tiled HEIC and AVIF: their alpha streams are not grouped and have the size of the image or of a color tile. A tiled grayscale image is made of grouped `dependent` tiles, so it now stays on the PPM path. A depth map or a matte that happens to have the size of the image or of a tile would still be taken for alpha; that keeps the image correct at the cost of speed.
The PNG writer spent most of its run on filtering and writing even at level 0: the 24-megapixel image took 12 seconds with the default compression and 2.0 seconds with none ([libheif's heif-dec](../knowledge/libheif-heif-dec.md)). TIFF stores the same uncompressed pixels without that pass, which is why the decode asks for a TIFF first. libheif 1.15.1 and 1.17 have only PNG; 1.19 added TIFF, and the decode uses it when the build has libtiff. The level-0 PNG fallback keeps 1.17 working.
Requiring ffmpeg 9 at startup keeps an older ffmpeg from silently reading only one tile of a tile grid.

**Rejected alternative 1**
Keeping heif-dec as the primary decoder.
It works on old libheif and ffmpeg versions, but the intermediate file dominates the time and makes HEIF about 3 to 4 times slower than JPEG, so we rejected it.

**Rejected alternative 2**
A method that links libheif as a library through bindings.
It removes the intermediate file and the process boundary, but a decoder anomaly would spread to the daemon's process, and the 3-OS CI would need libheif's development files, so we rejected it (same reason as §10.1 "Call ffmpeg as an External Process").

**Rejected alternative 3**
Having ffmpeg write WebP directly instead of PPM.
It is faster only when the output is WebP and no resizing is needed (0.27 to 0.32 seconds). For JPEG/PNG output, images larger than the display step, and thumbnails, it needs a second decode and encode and loses quality twice. PPM keeps one path for all sizes.

**Rejected alternative 4**
Merging the alpha stream with ffmpeg's `alphamerge`.
It preserves alpha for an untiled image, but for a tile grid it produces only one tile. libheif already handles the grid and the alpha together, so we rejected it.

**Rejected alternative 5**
Keeping AVIF on the old PNG path.
The same PPM path is faster and the rotation behavior is unchanged. We unified it with HEIC/HEIF.

**Rejected alternative 6**
Moving JXL and JP2 to PPM for a single ffmpeg path.
ffmpeg's JXL decode carries alpha in the primary stream, and the current PNG path preserves it. PPM would drop that transparency. JXL is not the target of this change, so it keeps the PNG path.

**Rejected alternative 7**
Asking libheif's `heif-info` whether the image has alpha.
Its answer is exact, but it adds one more process per image, and its output is meant for people and may change between libheif versions, so we rejected it.

**Rejected alternative 8**
Decoding every HEIC/HEIF/AVIF with ffmpeg and using heif-dec only when ffmpeg fails.
It is the simplest, but the transparency of HEIC and AVIF with alpha would be lost, so we rejected it.

**Rejected alternative 9**
Requiring libheif 1.19 and libtiff, with no PNG fallback.
Version 1.19 does not guarantee libtiff: Ubuntu 26.04's 1.21.2 is built without it, and alpha images would fail there. The slower PNG path is better than failing, so we keep the fallback.

**Rejected alternative 10**
Keeping level-0 PNG for every build.
It is the simplest, but gives up the measured 1.7 to 2.2 times gain on builds with libtiff, so we use TIFF when it is available.

#### Announce Images and Videos with the Listing's contentType

**Decision**
The listing's `metadata.contentType` carries the MIME type of files that match the determination in §3.
The client decides the kind of tile from the `image/` and `video/` prefixes and does not hold a set of extensions.

**Reason**
For the client to decide the kind of tile and viewer, whether something is an image or a video must be known from the listing response alone.
The determination is by extension alone, so files do not need to be opened on every listing.
The set of supported extensions is drawn from the same table as the thumbnail generation determination, so files that can be generated and files announced as images or videos do not disagree.
Because it is decided by extension alone, extensions shared with other uses, such as `ts`, are also treated as videos.

**Rejected alternative 1**
A method in which the client holds the set of extensions.
Every time the server's support grows, all clients would need updating, and the determination of whether generation is possible would disagree, so we rejected it.

**Rejected alternative 2**
A method that requests thumbnails for all files and returns files that failed with `UnsupportedType` to icons.
The kind of video sprite sheet and viewer would also be decided from the result of the request, and there would be as many failing requests as files that are neither images nor videos, so we rejected it.

#### Round Sizes Up to Steps

**Decision**
`width` and `height` are accepted without changing the API's shape, and the server rounds them up to size steps (§4.1).

**Reason**
The client can send values matched to the display as they are, and the kinds of cache entries stay within the number of steps.
Even if the step values are changed by configuration, the API and the clients do not change.

**Rejected alternative 1**
A method that makes values other than the steps `InvalidRequest`.
Every change of the steps would need changes to the API and clients, so we rejected it.

**Rejected alternative 2**
A method that drops `width` and `height` and has the size specified by an enumeration `size`.
It could no longer express frames other than squares, so we rejected it.

#### Let the Server Choose the Representative Thumbnail

**Decision**
When an item names a folder or archive, the server chooses a representative from the direct children and returns it (§5).

**Reason**
One request per tile suffices, and requesting and canceling can follow the same rules as file tiles.
The selection rules are in one place only, on the server, so clients do not disagree.

**Rejected alternative**
A method in which the client lists the folder and then requests the thumbnail of the first file.
The API would not change, but there would be two round trips per tile, and the same rule would be implemented in two clients, so we rejected it.

#### Choose the Output Format and Quality by Configuration, Defaulting to WebP

**Decision**
The output format of thumbnails, sprite sheets, and display images can be chosen from WebP, JPEG, and PNG, and the format and the quality of lossy compression are decided by the server's configuration (§4.2).
The default format is lossy WebP.
Encoding calls libwebp-sys directly and fixes the compression effort (`method`) at 2.

**Reason**
By Google's comparison, lossy WebP is 25% to 34% smaller than JPEG at the same SSIM ([The size of lossy WebP](../knowledge/webp-lossy-size.md) in [knowledge.md](../knowledge.md)).
The main subjects are photos and video frames, so the transfer volume of listings and the viewer is expected to decrease.
The libwebp used for lossy encoding is built by libwebp-sys from its bundled C source, so no system libraries need to be added to the 3-OS CI and the container images ([Building libwebp-sys](../knowledge/libwebp-sys-build.md) in [knowledge.md](../knowledge.md)).
The cost is that the build needs a C compiler and there is one more dependency on a C library.
libwebp-sys calls C functions, so the calls need unsafe.
We close creating the configuration, importing pixels, encoding, and freeing the allocated output into one function, and confine unsafe to inside that function.
We lower the compression effort from the default 4 to 2 because WebP encoding accounted for about half of the conversion in the first generation of a display image. For an image shrunk to a long side of 4096 at quality 90, 2 took 40% to 60% of the time of 4, with a PSNR difference within 0.2 dB and a size difference within ±4% ([measurement record](../measurements/2026-10-16-webp-method.md)). 1 and below are larger at the same quality, and 3 and above were slower than 2 without becoming smaller.
If the format and quality are put in the server's configuration, every client requests with the same values, so conversion results of the same image do not line up in the cache by the number of format and quality pairs.

**Rejected alternative 1**
A method with JPEG as the default, encoding with the `image` crate alone.
The previous conversion took this form, and it does not depend on a C library, but at the same quality the transfer volume is larger than WebP, so we removed it from the default.

**Rejected alternative 2**
A method that outputs only PNG.
The conversion before the change took this form, but the transfer volume is large for photo thumbnails, so we removed it from the default.

**Rejected alternative 3**
A method that adds lossless WebP.
It can be encoded with the `image` crate alone, but lossless compression does not discard photo detail, so it does not become as small as lossy compression, and we rejected it.

**Rejected alternative 4**
A method that adds AVIF.
A pure Rust encoder is available as a feature of the `image` crate, but we have not confirmed the encoding speed on a NAS CPU, and it could slow the display of listings of folders opened for the first time, so we rejected it.

**Rejected alternative 5**
A method in which the format and quality are chosen on the web client's settings screen and carried in the request.
It makes it easy to try and compare formats, but if each browser requested with different values, conversion results of the same image would line up in the cache by the number of value pairs, so we rejected it.

**Rejected alternative 6**
A method that uses the `webp` crate, which has a safe API.
It avoids writing unsafe, but the libwebp-sys that `webp` crate 0.3.1 depends on is the 0.9 series, different from the version whose build method we confirmed (0.14.4), so we rejected it ([The webp crate's dependency](../knowledge/webp-crate-libwebp-sys-version.md) in [knowledge.md](../knowledge.md)).

#### Keep the Thumbnail Cache as an LRU of a SQLite Index and Real Files

**Decision**
The thumbnail cache is held as a SQLite index plus real files, and the capacity limit is managed by an LRU divided into two budgets (§8).

**Reason**
The capacity limit and eviction rules can be held by the same mechanism as the extraction cache.
Because the key includes the modification time, old entries merely become unreferenced and remain, but with an LRU they disappear within the capacity.
By separating the placement into `state_dir/thumbnail/`, the units of backup for user information and the cache are also separated.

**Rejected alternative**
A method that stores values and metadata serialized with RocketPack in RocksDB.
The `ThumbnailCache` before the change took this form.
Capacity management would have to be made separately, and it would sit directly under `state_dir` next to user information, so we rejected it.

#### Use EntryCacheKey for the Keys of Entries in Archives

**Decision**
The thumbnail of an entry in an archive builds its key from the `EntryCacheKey` and the conversion conditions (§8.3).

**Reason**
Browsing inside archives is a main use, and entries of formats without a modification time also need thumbnails.
`EntryCacheKey` always changes when the outer archive changes, so a stale thumbnail is never returned.

**Rejected alternative**
A method that does not make thumbnails of entries without a modification time.
There would be one key rule, but the contents of such archives would be only icons in listings, so we rejected it.

<a id="representative-selection-cache"></a>
#### Reuse the Representative Selection with a Version and Expiry

**Decision**
The selection of representative candidates is held per folder namespace and path, and per local folder modification time or archive identity. At most 4096 targets, an expiry of 30 seconds, and discarded in order of oldest use.
The selected path and the no-candidate case are held distinctly. Cases with an unknown version and local updates within the last 2 seconds are not reused, and concurrent selections for the same target and version are merged into one.
Only the representative's position is held, and the target's metadata and image are checked on every request by the existing rules. If fetching the candidate gives NotFound, the selection is discarded and selected again once.
When reflecting our own changes, the representative selection is also invalidated. External changes are detected by version and expiry, and the limits of the existing file system attribute cache carry over.
The rule of the first image or video in natural order is maintained, and neither recursive probing of subfolders nor following symlinks is added.

**Reason**
It omits the full scan of direct children that was repeated even for requests that hit the image cache. The no-candidate case is also reused briefly, so the same full scan is not done for every 404 of an empty folder.

**Rejected alternative**
The previous method of selecting again every time is simple, but repeats enumeration and attribute fetching even after caching, so it is replaced. Holding forever hides external changes, so we do not adopt it.

<a id="representative-revalidation"></a>
#### Revalidate Representative Thumbnails and Request Them Again After Our Own Changes

**Decision**
The image of a folder or archive is revalidated against the representative chosen now on every want, like any image (§5).
After the daemon's own change, when the web client rereads the listing, it forgets the folder and archive tiles it has shown or failed, and wants them again with the `ETag` of the image it holds ([Client Design §4](./client.md#4-thumbnail-requests)).
External changes appear when the tile leaves the request targets and returns, within the 30-second expiry of the representative selection ([representative-selection-cache](#representative-selection-cache)).

**Reason**
An item for a folder names the folder, so it stays the same when the representative changes, and only a comparison with the current selection shows the new one. It costs an `unchanged` while the representative is the same.
Wanting again only right after our own change keeps scrolling back and forth in the same listing from adding wants or generation.
The tiles wanted again are at most those in the request targets, and the server's selection has already been invalidated by the same change.

**Rejected alternative 1**
Revalidating only when a tile returns to the request targets.
A tile that stays in the request targets would keep showing the old image until it leaves them, so we rejected it.

**Rejected alternative 2**
Showing a held image without revalidating it.
There is no data loss, but a tile could show a stale representative until the client's cache drops the image, so we rejected it.

<a id="folder-thumbnail-delay"></a>
#### Request Folder Thumbnails After the Listing Is Displayed

**Decision**
The web client delays thumbnails of folders and archives by at least 200 ms after the listing's row appears, and requests only the visible range. The existing prefetch of images and videos is maintained.
The delay for the same visible Entry is not restarted on every redraw, and unsent timers and requests of Entries that moved away are canceled. An Entry that returns is treated as a new display.
The delay and priority are held by the client, and the server does not guess the visible range. The existing limits of generation, the merging of §7, and the image cache are maintained.

**Reason**
The time at which the listing becomes operable is not delayed by the scan and conversion of folder representative selection. Contributions are measured separately for listing display, representative selection, and generation.

### 10.2 Deferred

#### Threads and Slots of the External Processes for Images

**Current state**
`process_slots` (default 2) and `process_interactive_slots` (default 1) limit how many ffmpeg and heif-dec runs decode images at once, and each run uses as many threads as ffmpeg or libheif choose. In the 2026-10-26 log with `process_slots = 4`, an Interactive display image of a 24-megapixel HEIC got its slot at once but took 16 seconds to decode while four thumbnail decodes of the same size ran; at that time those images went to heif-dec with a compressed PNG (§10.1).

**Why not decide now**
The images of that log now take the PPM path, which decoded one of them in 0.7 seconds instead of 12, so the decodes the display image competes with are far shorter. How much they still slow it down has not been measured.

**Conditions for deciding**
When a log after that change shows an Interactive display image of a HEIC or AVIF whose `thumbnail_external_decode` takes more than twice its time when it runs alone, while Foreground external decodes run.

#### Settings for the Number and Positions of Frames

**Current state**
The number of frames, the start point, and the end point are arguments to the generation process, but cannot be specified from the configuration file and are fixed to the defaults in code (8 frames, 10% to 90%).

**Why not decide now**
Whether the defaults are easy to browse with on real videos cannot be known until flipbook playback has been run.
Even if the shape of the configuration keys is decided after knowing which value to change, no migration cost arises because the values are included in the cache key.

**Conditions for deciding**
When flipbook playback has been used with real operation videos and it turns out the number of frames or the positions do not fit the defaults.

#### Flipbook Playback for Animated Images

**Current state**
For animated GIFs and animated WebPs, only the first still frame becomes the thumbnail (§2.1).

**Why not decide now**
Making sprite sheets as with videos would need a path that extracts frames per image format.
We do not know how much demand there is for browsing animated images.

**Conditions for deciding**
When, while browsing a folder with many animated images, it turns out that the first frame alone cannot tell the contents apart.

## 11. Current State and Remaining Work

### 11.1 Current Code

Representative selection uses lightweight enumeration of names and kinds, and holds positive and negative results for 4096 targets and 30 seconds. Selections for the same target, version, and generation are shared by single-flight, and an unknown version and local updates within the last 2 seconds are not held. The daemon's reflection of changes advances `StorageService`'s generation, preventing the publication and reuse of stale selections. The candidate's attributes are fetched each time without following symlinks, and if it has disappeared or changed to something other than a file, it is selected again once.

Grid and List target only the folders and archives of visible rows, and wait 200 ms from the microtask after the DOM notification. Images and videos keep the existing prefetch range. The queue holds the deadline of the same visible Entry, and cancels timers and requests when off-screen and on dispose.

Representative selection, candidate attributes, the lookup, read, and LRU update of the image cache, and generation are recorded at separate measurement points. Connection acquisition and execution of the LRU update are also distinguished. The format and keys of the image cache are not changed, and LRU updates are taken out of the response and written together as in [§8.2](#82-capacity-budgets-and-lru). The comparison is in the [measurement record](../measurements/2026-10-15-browse-thumbnails.md).

`ThumbnailGenerator` generates according to the rules of §3 through §8.
It handles images, HEIF images read with ffmpeg (or heif-dec for alpha), images read with ffmpeg, videos, folders, and archives, and returns thumbnails, sprite sheets, and display images.
The cache is a SQLite index and real files in `state_dir/thumbnail/`, with a budget for thumbnails and sprite sheets and a budget for display images.
Thumbnails, sprite sheets, and display images are returned over the image socket ([Image Socket Design §10](./image-socket.md#10-current-state-and-remaining-work)), and there is no HTTP route for them. Each item that misses the cache is placed in the source job of its source (§7), in `ThumbnailGenerator`, so one source is decoded by one job at a time, whichever requests are for it. The order in which items reach the slots is their class and order on the socket.
The client revalidates a held image by sending its `ETag` (§5), and the web client does not yet request folder and archive tiles again after the daemon's own changes.

All the keys in §9.1 can be specified from the configuration file.
The output format is `[thumbnail] format` (the default is WebP), chosen from WebP, JPEG, and PNG.
Lossy WebP is encoded with libwebp-sys with the compression effort fixed at 2, and for quality, `quality` is used for thumbnails and sprite sheets and `display_quality` for display images.
The key includes quality only for lossy formats, and the default of `display_sizes` is `[2048, 4096]`.
The number and positions of frames cannot be configured and are fixed to the defaults in code (8 frames, 10% to 90%).
A generation also makes the companions of §3, and the tests of them request WebP, because the existing test helpers request JPEG, which never matches a WebP companion.
A video's duration is obtained from the media info service shared with video playback. The LRU and the merging of concurrent requests are common with the video's media info ([Video Playback Design §3](./video.md#3-media-info)).

In environments where ffmpeg and ffprobe are not on `PATH`, the tests for videos and images read with ffmpeg are skipped with the reason printed.
The tests that assemble tile grids with ffmpeg are also skipped when ffmpeg is older than 8, and the test that reads the rotation of a tile grid with ffprobe when it is older than 9.
The HEIF tests use ffmpeg 9 and heif-dec or heif-convert 1.17 or newer from `PATH`, and are skipped without them.
Linux CI installs a pinned ffmpeg 9 and libheif on `ubuntu-latest`, and makes these tests mandatory with `PXNA_REQUIRE_FFMPEG=1` and `PXNA_REQUIRE_HEIF_DEC=1`. The Linux job exercises the PNG fallback and the macOS job the TIFF output.

### 11.2 Roadmap

M4 and M11 of [design.md §12.2](../design.md#122-roadmap) have no remaining work.
The API that returns media info and transcoded playback are covered by [Video Playback Design §8.2](./video.md#82-roadmap), and the issues held as deferred by §10.2.

The order is provisional.
Only the dependencies must be respected.

| No. | Content | Dependencies assumed | State |
| --- | --- | --- | --- |
| T1 | Have the web client want folder and archive tiles again after the daemon's own changes ([design decision](#representative-revalidation)) | IC2 of [Image Socket Design §10.2](./image-socket.md#102-roadmap) | Not started |

T1 corresponds to M20 of [design.md §12.2](../design.md#122-roadmap). The revalidation of §5 itself comes with the image socket (IC1).

For confirmed defects, see [issues.md](../issues.md).
