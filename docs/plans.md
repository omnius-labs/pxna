# Implementation Plans in Progress

This document covers work in progress and the dependencies between pieces of work.
Child documents cover the content of each piece of work, and for completed work, the code and git history are authoritative.

## 1. About This Document

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | Plans in progress and dependencies |
| [design.md](./design.md#11-division-of-responsibility-between-documents) | Design decisions, invariants, deferred items |
| [terms.md](./terms.md#11-division-of-responsibility-between-documents) | Vocabulary |
| [issues.md](./issues.md) | Confirmed defects |

Create a child and add a row to the list before starting.
During implementation, advance the state of the child and of this document, and when a plan is wrapped up, delete the child and the row.

## 2. Plan List

| Plan | Purpose | State | Predecessor |
| --- | --- | --- | --- |
| [thumbnail-fanout-and-viewer-settings](./plans/thumbnail-fanout-and-viewer-settings.md) | One decode warms the listing's, filmstrip's, and both-shape thumbnails, and the viewer gets its own settings panel | Done | - |
| [thumbnail-store-batching](./plans/thumbnail-store-batching.md) | A generation stores its thumbnails in one batch and frees its slot first | Done | thumbnail-fanout-and-viewer-settings |
| [source-jobs](./plans/source-jobs.md) | While several requests for different sizes or kinds of one image arrive, the daemon decodes it once at a time | Done | thumbnail-fanout-and-viewer-settings |
| [image-socket-server](./plans/image-socket-server.md) | A client can want images over one WebSocket and receive them in priority order, with held images answered by `unchanged` | Done | source-jobs |
| [split-video-and-image-pools](./plans/split-video-and-image-pools.md) | Video ffmpeg runs have their own slots, so a slow video does not block HEIC decodes | Done | - |
| [viewer-touch-gestures](./plans/viewer-touch-gestures.md) | The image viewer can be operated with one hand on a phone | Done | - |
| [restore-last-folder](./plans/restore-last-folder.md) | Opening `/` returns to the last folder instead of asking for a Library | Done | - |
| [search-condition-panel](./plans/search-condition-panel.md) | Every Library, or chosen ones, can be searched by name, rating, and several tags with the conditions in the left area | Done | - |
| [viewer-display-modes](./plans/viewer-display-modes.md) | A work can be viewed in the wide-screen viewer's gallery and focus modes, and the mode returns on the next opening | Done | - |

The thumbnail and image socket plans form a chain: `thumbnail-store-batching` and `source-jobs` follow `thumbnail-fanout-and-viewer-settings`, and `image-socket-server` follows `source-jobs`.
The `viewer-touch-gestures`, `restore-last-folder`, and `viewer-display-modes` plans are independent of each other and of them.
