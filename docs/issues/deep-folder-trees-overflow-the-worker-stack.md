# Deep Folder Trees Overflow the Worker Stack and Abort the Daemon

**Severity: high** (confirmed by reproduction; one request stops the whole daemon)

Investigated 2026-10-25 / Target commit `4f071e1`

One item of [issues.md](../issues.md). Found by the security audit of 2026-10-24 and confirmed in the second pass.

## Symptom

Deleting a folder tree of depth 1400 with `recursive=true` makes a tokio runtime worker overflow its stack. The runtime aborts the process, so every Library stops until the daemon is restarted.

The reproduction used a directory chain of depth 1400 (relative path length 2799) and `POST /api/v1/libraries/contents/folders/delete` with `{"srcPaths":["/d"],"recursive":true}`, which returned `202`. A few seconds later the log held:

```
thread 'tokio-rt-worker' (28157) has overflowed its stack
fatal runtime error: stack overflow, aborting
```

`GET /api/v1/health` stopped answering.

## Locations

- [operation.rs#L525](../../daemon/entrypoints/daemon/src/service/mutation/operation.rs#L525) — `delete_tree`, which recurses through its boxed future
- [operation.rs#L536](../../daemon/entrypoints/daemon/src/service/mutation/operation.rs#L536) — the recursive call into `delete_tree` for each child
- [operation.rs#L315](../../daemon/entrypoints/daemon/src/service/mutation/operation.rs#L315) — `transfer_entry`, which recurses in the same way for copy and move
- [operation.rs#L455](../../daemon/entrypoints/daemon/src/service/mutation/operation.rs#L455) — `remove_destination`, which removes the destination of an overwriting move through `delete_tree`

## Cause

The recursion depth follows the depth of the tree, and each level costs more stack than the 2 MiB a tokio worker allows by default. Nothing checks the depth of a source tree, and nothing bounds the path depth of the Library.

A folder tree of this depth is easy to create through the API (`createFolder` plus uploads), and an archive can also contain one. The audit reached it with a local directory chain; no owner action is needed beyond having the content in a Library.

## Impact

The daemon aborts, not just the operation. All Libraries become unavailable, in-flight uploads and scans are lost, and the operation is left unfinished. The same request can be repeated after every restart.

## Handling Policy

Replace the recursion with an explicit stack or a work queue, so that the depth of the tree no longer consumes the call stack. Add an upper bound on the depth walked by a recursive operation and return `400` above it.

The regression test builds a tree of the reproduced depth, deletes it through the API, and asserts that the daemon answers `/health` afterwards and that the tree is gone. A second test covers copy and move over the same fixture.
