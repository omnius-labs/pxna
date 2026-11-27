# A Copy That Replaces an Entry Keeps the Folder Settings at Its Path

**Severity: Low** (a copied folder can open with values the owner set for what it replaced; no data is lost)

Investigated 2026-11-26 / Target commit `b7edbaf`

One item of [issues.md](../issues.md).

## Symptom

After a copy with the conflict policy that overwrites replaces a file, an archive, or a folder at the destination, the folder settings saved at or under that path stay.
A folder the copy creates there, or a folder under it with the same name as one inside the replaced archive, then opens with those settings, and nothing is logged.
A move that replaces the same Entry removes them.

## Location

- [operation.rs#L344](../../daemon/entrypoints/daemon/src/service/mutation/operation.rs#L344): a folder the entry-by-entry transfer creates clears the folder settings at or under its path only when the transfer moves.
- [operation.rs#L378](../../daemon/entrypoints/daemon/src/service/mutation/operation.rs#L378) and [operation.rs#L392](../../daemon/entrypoints/daemon/src/service/mutation/operation.rs#L392): a copied file that replaces a folder or an archive reassigns and removes nothing.
- [operation.rs#L435-L442](../../daemon/entrypoints/daemon/src/service/mutation/operation.rs#L435-L442): `directory` removes a file or an archive in the way of a new folder without telling `LibraryMutation`.
- [mutation.rs#L73-L92](../../daemon/entrypoints/daemon/src/service/mutation.rs#L73-L92): `forget_missing` removes folder settings only for a changed path that no longer exists, and the replaced path exists again after the copy.

## Cause

The reassignment of folder settings follows renames, moves, and deletes ([design.md §6.5](../design.md#65-shelves-and-folder-settings)).
A move that replaces its destination removes the settings there through `ShelfService::moved` or `folder_created`, but a copy calls neither, and the end of the operation looks only for paths that have gone.

## Impact

Only folder settings at the replaced path are affected; Shelf starts keep naming the path, as they do for a move.
It shows only when the owner had saved folder settings for the replaced folder, or for a folder inside the replaced archive, and then copies over it with overwrite.
No client writes folder settings yet (M30 of [design.md §12.2](../design.md#122-roadmap)), so it has not been observed.

## Remedy

Clear the folder settings at or under the destination wherever a copy replaces or creates an Entry, as a move does: call `LibraryMutation::folder_created` for every folder the transfer creates, not only when it moves, and `ShelfService::forget` for the destination of a copied file that replaced a folder or an archive.
A route test that copies with overwrite over a folder and over an archive, both with folder settings, and then reads `GET /settings/folder`, keeps it fixed.
