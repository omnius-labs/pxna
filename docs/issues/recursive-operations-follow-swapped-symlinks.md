# Recursive Operations Follow a Symlink Swapped in After the Listing

**Severity: medium** (confirmed by reproduction; reachable without an owner action)

Investigated 2026-10-25 / Target commit `4f071e1`

One item of [issues.md](../issues.md). Found by the security audit of 2026-10-24 and confirmed there by an independent reproduction.

## Symptom

Recursive copy, move and delete decide the type of each child from the listing, then open it again through the path. If a child is replaced by a symbolic link between those two moments, the operation follows the link and acts on the target outside the Library. Deleting a folder then deletes a file outside the Library through the link.

## Locations

- [operation.rs#L483](../../daemon/entrypoints/daemon/src/service/mutation/operation.rs#L483) — `children()` lists a folder and rejects a child whose `symlink_metadata` is not a file or a directory
- [operation.rs#L498](../../daemon/entrypoints/daemon/src/service/mutation/operation.rs#L498) — the `symlink_metadata` check above
- [operation.rs#L525](../../daemon/entrypoints/daemon/src/service/mutation/operation.rs#L525) — `delete_tree` takes the child again with `get_entry()`
- [operation.rs#L536](../../daemon/entrypoints/daemon/src/service/mutation/operation.rs#L536) — the `get_entry()` of `delete_tree`
- [operation.rs#L315](../../daemon/entrypoints/daemon/src/service/mutation/operation.rs#L315) — `transfer_entry` for copy and move, which also takes `get_entry()`
- [operation.rs#L333](../../daemon/entrypoints/daemon/src/service/mutation/operation.rs#L333) — the `get_entry()` of `transfer_entry`
- [scanner.rs#L216](../../daemon/entrypoints/daemon/src/service/timeline/scanner.rs#L216) — a scan reads every archive of a chunk inline before it reads the media of the same chunk, which widens the window for a swap

## Cause

`get_entry()` resolves the path again with `metadata`, which follows symbolic links. The type accepted by `children()` is not carried into the operation. The gap between the two calls is wide enough to be used: media in a chunk is processed after every archive in the same chunk is read, and an archive read may run up to its extraction timeout.

The audit reproduced both the control and the attack:

- Control (symlink present at listing time): `children()` rejects it as `UnsupportedType`, and the file outside the Library survives
- Attack (swap after the listing): the listing accepted the child, `get_entry()` followed the link, and the recursive delete removed the file outside the Library

## Impact

A lower-trust principal that can create symbolic links and rename them inside a Library can make the daemon delete or copy files outside the Library root, with the permissions of the daemon user. The owner does not have to act.

## Handling Policy

Open each child relative to the already-open parent directory and refuse to follow links at that moment. The first candidate is `openat2(RESOLVE_NO_SYMLINKS)`; where it is unavailable, `O_NOFOLLOW` on the opened descriptor followed by `fstat`. A re-`lstat` immediately before use is the minimum fallback; it narrows the window but does not close it.

The regression test creates a child after the listing, replaces it with a symlink to a file outside the Library, and asserts that the operation fails and that the outside file survives.
