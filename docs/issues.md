# Pxna Known Defects

This document covers unaddressed defects and improvement proposals confirmed by reading the code.
Design issues for which an implementation approach has not yet been chosen are covered by the design decisions chapter of the relevant design document.

## 1. About This Document

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | Confirmed defects, severity, investigation date, handling policy |
| [design.md](./design.md#11-division-of-responsibility-between-documents) | Design decisions and open issues |
| [terms.md](./terms.md#11-division-of-responsibility-between-documents) | Definitions and spelling of terms |
| [reviews.md](./reviews.md) | History of reviews and transcription status |

Append the number of a filed Issue to the Issue column of the table.
Delete the child file and the table row for a fixed item.
Line numbers change, so recheck the relevant location when filing and when fixing.

## 2. Defect List

| Item | Severity | Investigated | Issue |
| --- | --- | --- | --- |
| [Deep folder trees overflow the worker stack and abort the daemon](./issues/deep-folder-trees-overflow-the-worker-stack.md) | High | 2026-10-25 | - |
| [Media decoding has no size or dimension cap](./issues/media-decoding-has-no-size-cap.md) | High | 2026-10-25 | - |
| [Recursive operations follow a symlink swapped in after the listing](./issues/recursive-operations-follow-swapped-symlinks.md) | Medium | 2026-10-25 | - |
| [Pre-auth password hashing has no concurrency limit](./issues/pre-auth-hashing-has-no-concurrency-limit.md) | Medium | 2026-10-25 | - |
| [The owner SPA can be framed by a same-site origin](./issues/owner-spa-can-be-framed.md) | Medium | 2026-10-25 | - |
| [Malformed archives are reported as internal server errors](./issues/malformed-archives-are-reported-as-internal-errors.md) | Low | 2026-10-25 | - |
| [Representative thumbnails are not reflected on screen after their contents are deleted](./issues/representative-thumbnail-stale.md) | Medium | 2026-10-23 | - |

The six items at the top come from the security audit of 2026-10-24; the representative-thumbnail item is unrelated to them.
The stack overflow, the media caps and the pre-auth hashing all spend the daemon's memory or availability, so they are best fixed as one group.
The TOCTOU and the framing header are boundary and defence issues with their own fixes, and the archive error classification stands alone.

## 3. What Is Not Included Here

The following are design issues whose fix has not been decided, so they are not included in this document.

| Issue | Reference |
| --- | --- |
| Limiting login attempts | [design.md §11.2](./design.md#limiting-login-attempts) |
| Validating the Host header for setup and login | [design.md §11.2](./design.md#validating-the-host-header-for-setup-and-login) |
| Multi-user and permission model | [design.md §11.2](./design.md#multi-user-and-permission-model) |
| Persisting Operations | [design.md §11.2](./design.md#persisting-operations) |
| Optimistic locking with ETag and If-Match | [design.md §11.2](./design.md#optimistic-locking-with-etag--if-match) |
| Separating the admin port | [design.md §11.2](./design.md#separating-the-admin-port) |
| Whether to reflect external changes in listings immediately | [Listing and Search Design §8.2](./design/listing.md#whether-to-reflect-external-changes-in-listings-immediately) |
| Process isolation for archive handling | [Storage Layer Design §9.2](./design/storage.md#process-isolation-for-archive-handling) |
| Password-protected archives and decrypted caches | [Storage Layer Design §9.2](./design/storage.md#password-protected-archives-and-decrypted-caches) |
| Legacy encodings other than CP932 | [Storage Layer Design §9.2](./design/storage.md#legacy-encodings-other-than-cp932) |
| Archives that hit the block bulk-extraction limit | [Storage Layer Design §9.2](./design/storage.md#archives-that-hit-the-block-bulk-extraction-limit) |
| Strict daemon-wide memory admission | [Storage Layer Design §9.2](./design/storage.md#strict-daemon-wide-memory-admission) |
| Settings for the number and positions of frames | [Thumbnail Design §10.2](./design/thumbnail.md#settings-for-the-number-and-positions-of-frames) |
| Flipbook playback for animated images | [Thumbnail Design §10.2](./design/thumbnail.md#flipbook-playback-for-animated-images) |
| Number of devices that play transcoded video at the same time | [Video Playback Design §7.2](./design/video.md#number-of-devices-that-play-transcoded-video-at-the-same-time) |
| Whether to show counts on folder tiles | [Client Design §5.2](./design/client.md#whether-to-show-counts-on-folder-tiles) |
| Whether to download multiple Entries at once | [Client Design §5.2](./design/client.md#whether-to-download-multiple-entries-at-once) |
| Whether to filter listings by kind | [Client Design §5.2](./design/client.md#whether-to-filter-listings-by-kind) |
| Whether to show an Entry's details from the viewer | [Client Design §5.2](./design/client.md#whether-to-show-an-entrys-details-from-the-viewer) |
