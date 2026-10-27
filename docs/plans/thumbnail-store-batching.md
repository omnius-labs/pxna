# Store a Generation's Thumbnails in One Batch

## 1. About This Plan

The fan-out work ([thumbnail-fanout-and-viewer-settings](./thumbnail-fanout-and-viewer-settings.md)) makes one generation store the requested image and five or six companions one by one while it holds its scheduler slot. This work stores them in one batch, frees the slot first, and relaxes the cache database's durability.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The steps, completion conditions, and scope of this work |
| [Thumbnail Design §8.2](../design/thumbnail.md#82-capacity-budgets-and-lru) | The store order, the batch, the slot release, and `synchronous=NORMAL` |

### 1.2 Creation Date and Target Commit

Created 2026-10-26 / Target commit `d4030c6` with the uncommitted fan-out work

## 2. Purpose and Completion Conditions

The log of a real library showed a median `thumbnail_store` of 45 ms, p90 616 ms, and a maximum of 1.02 s, while the index lookup took 0.3 ms. The serial stores each took the lock, flushed the recency, ran a transaction, and evicted, and a generation kept its slot meanwhile, which lengthened `thumbnail_permit_wait` (median 1.03 s, p90 3.82 s).

1. `ThumbnailCache::put_many` stores several entries with one lock, one recency flush, one transaction, and one eviction per bucket, staging files outside the lock
2. `last_accessed` follows the input order, and the generator passes companions, then the requested image, then the sprite sheet
3. An entry exceeding its bucket is skipped; an I/O or SQL error fails the call, and the generator logs it without failing the response
4. The generation releases its slot before the store, and the response waits for the store
5. The thumbnail cache database uses `synchronous=NORMAL`; `sqlite.db` is unchanged
6. The existing cache and generator tests pass, the lint passes, and the engine and daemon tests pass

### 2.1 Non-Goals

Storing in the background, and changing the companions' kinds, number, keys, or the UI.

## 3. Steps

| No. | Content | State |
| --- | --- | --- |
| 1 | Add `put_many`, evict with one total, set `synchronous=NORMAL` in `cache.rs` | Done |
| 2 | Use `put_many` in `Generator::store` and drop the permit before it | Done |
| 3 | Add the cache tests (`put_many`, order, skip, failure, `PRAGMA synchronous`) | Done |
| 4 | Update Thumbnail Design §8.2 and the fan-out plan's Step 8 | Done |

Unverified: the effect on `thumbnail_store` and `thumbnail_permit_wait` is not measured here, and the ffmpeg paths still skip without ffmpeg.
