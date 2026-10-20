# Pxna External Knowledge

This document lists verified facts about how the libraries and external components Pxna depends on actually behave.
The design documents cover the structure of our own code and design decisions, and [issues.md](./issues.md) covers defects in our own code.

## 1. About This Document

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | List of verified facts about the outside, the subject verified, the source, the last confirmation date |
| [design.md](./design.md#11-division-of-responsibility-between-documents) | Design decisions, invariants, deferred items |
| [terms.md](./terms.md#11-division-of-responsibility-between-documents) | Vocabulary |
| [issues.md](./issues.md) | Confirmed defects |

[design.md](./design.md#11-division-of-responsibility-between-documents) holds the map of the document family.

When the outside changes, rewrite the child, and do not keep descriptions of old versions.
For items whose source is `Literature only`, reread the primary sources every 3 months from the reference date.

## 2. List of External Knowledge

| Item | Subject verified | Source | Last confirmed |
| --- | --- | --- | --- |
| [The image crate's WebP encoder has only lossless compression](./knowledges/image-webp-encoder.md) | image 0.25.9 | Literature only | 2026-10-03 |
| [libwebp-sys builds the bundled libwebp with cc](./knowledges/libwebp-sys-build.md) | libwebp-sys 0.14.4 | Literature only | 2026-10-03 |
| [webp crate 0.3.1 depends on the 0.9 series of libwebp-sys](./knowledges/webp-crate-libwebp-sys-version.md) | webp 0.3.1 | Literature only | 2026-10-05 |
| [kamadak-exif reads Exif from TIFF, JPEG, HEIF, PNG, and WebP](./knowledges/kamadak-exif-containers.md) | kamadak-exif 0.6.1 | Literature only | 2026-10-03 |
| [Google states that lossy WebP is 25% to 34% smaller than JPEG at the same SSIM](./knowledges/webp-lossy-size.md) | Google's WebP explanation page (as of 2026-10-03) | Literature only | 2026-10-03 |
| [A directory's modification time is updated by creating, deleting, or renaming its direct children](./knowledges/posix-directory-mtime.md) | POSIX.1-2024 | Literature only | 2026-10-03 |
| [The Linux NFS client caches directory attributes for 30 to 60 seconds by default](./knowledges/nfs-attribute-cache.md) | Linux man-pages nfs(5) (as of 2026-10-03) | Literature only | 2026-10-03 |
| [HFS Plus stores filenames in decomposed form, and APFS preserves the normalization used at creation](./knowledges/macos-filename-normalization.md) | TN1150, Apple File System Guide | Literature only | 2026-10-03 |
| [Chromium's directory reading returns 100 entries at a time](./knowledges/chromium-directory-reader.md) | Chromium 155.0.8059.39 | Verified | 2026-10-07 |
| [Chromium shows a leave confirmation when a beforeunload is canceled after an operation](./knowledges/chromium-beforeunload.md) | Chromium 155.0.8059.39 | Verified | 2026-10-07 |
| [Chromium's modal dialog cannot be made full screen, and elements drawn outside the dialog cannot be operated](./knowledges/chromium-modal-dialog-fullscreen.md) | Chromium 154.0.8037.98 | Verified | 2026-10-10 |
| [Radix's confirmation dialog and menu propagate the Esc keydown, and a Tab in a Portal child reaches React ancestors](./knowledges/radix-nested-layer-events.md) | radix-ui 1.6.7 | Verified | 2026-10-10 |
| [ffprobe reports the AVIF irot as the Display Matrix rotation, and the width and height stay as encoded](./knowledges/ffprobe-heif-rotation.md) | ffmpeg 8.1.1 | Verified | 2026-10-11 |

The first 2 items, the WebP size item, and the webp crate item are the premises of the decision to output thumbnails as lossy WebP.
The first 2 items hold as a pair, and if image gains a lossy WebP encoder, there is no longer a reason to depend on libwebp-sys.
The kamadak-exif item is independent and is the premise for reading shooting info in an image's media info.
The POSIX modification-time item and the NFS item are the premises of the freshness of listing snapshots, and the NFS item describes the delay before the modification time, for which the POSIX item holds, reaches the client.
The macOS normalization item is the reason names are normalized before matching in name search.

The Chromium directory-reading and beforeunload items are the premises of bringing in folders and of the leave confirmation during uploads. They do not depend on the other external component items.
The ffprobe rotation item is independent and is the premise of the decision to return the rotation of images read with ffmpeg as an attribute without swapping the dimensions.
The Chromium modal item and the Radix item are the premises of the viewer's full-screen display and of the confirmation dialogs and menus shown inside it, and they hold as a pair. The former describes the reason for drawing into the outer frame element, and the latter describes the processing the key operations inside the outer frame need.
