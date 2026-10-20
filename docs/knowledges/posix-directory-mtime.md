# A Directory's Modification Time Is Updated by Creating, Deleting, or Renaming Its Direct Children

**Source: Literature only** (POSIX.1-2024 (IEEE Std 1003.1-2024) / 2026-10-03)

One item of [knowledges.md](../knowledges.md).

## Fact

POSIX specifies that when the following operations succeed, the parent directory's last data modification time (mtime) and last file status change time (ctime) are marked for update.

- When `open()` with `O_CREAT` creates a file that did not exist
- When `mkdir()` creates a directory
- When `unlink()` removes an item
- When `rename()` changes a name (the parents of both the source and the destination)

On the other hand, `write()` marks for update only the times of the file written, and not the parent directory's times.
Therefore, a change to the contents of an existing file and a change inside a child directory do not change the directory's modification time.
This holds on file systems that follow POSIX.
The precision with which times are recorded differs by file system.

## Basis

On 2026-10-03 we consulted the POSIX.1-2024 pages for `rename()`, `unlink()`, `open()`, `mkdir()`, and `write()`.

- `rename()`: "Upon successful completion, rename() shall mark for update the last data modification and last file status change timestamps of the parent directory of each file."
- `unlink()`: "Upon successful completion, unlink() shall mark for update the last data modification and last file status change timestamps of the parent directory."
- `open()`: "If O_CREAT is set and the file did not previously exist, upon successful completion, open() shall mark for update ... the last data modification and last file status change timestamps of the parent directory."
- `mkdir()`: "Also, the last data modification and last file status change timestamps of the directory that contains the new entry shall be marked for update."
- `write()`: "... write() shall mark for update the last data modification and last file status change timestamps of the file ..."

## Implication

A listing snapshot checks the version of a local folder's direct children by the folder's modification time ([Listing and Search Design §4.3](../design/listing.md#43-freshness)).
This fact is why this version cannot pick up changes to file contents and changes to grandchild Entries.
