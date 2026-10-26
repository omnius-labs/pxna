# The Linux NFS Client Caches Directory Attributes for 30 to 60 Seconds by Default

**Source: Literature only** (Linux man-pages nfs(5) / 2026-10-03)

One item of [knowledge.md](../knowledge.md).

## Fact

The Linux NFS client caches directory attributes (including the modification time) and returns the cached value without asking the server between `acdirmin` and `acdirmax`.
If these are not specified at mount time, `acdirmin` is 30 seconds and `acdirmax` is 60 seconds.
As a result, a directory change made on another client or on the server may not show up in the modification time during this period.
Mounting with `noac` stops attribute caching.

## Basis

On 2026-10-03 we consulted man7.org's nfs(5) descriptions of `acdirmin`, `acdirmax`, and `noac`, and its description of attribute caching.

- `acdirmin`: "If this option is not specified, the NFS client uses a 30-second minimum."
- `acdirmax`: "If this option is not specified, the NFS client uses a 60-second maximum."

## Implication

If a Library is placed on an NFS mount, a folder change on another device may be delayed before the snapshot version check finds it ([Listing and Search Design §4.3](../design/listing.md#43-freshness)).
