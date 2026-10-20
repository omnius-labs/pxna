# Chromium's Directory Reading Returns 100 Entries at a Time

**Source: Verified** (Chromium 155.0.8059.39 / 2026-10-07)

One item of [knowledges.md](../knowledges.md).

## Fact

In headless Chromium 155.0.8059.39 on Ubuntu, we dropped a directory containing 101 files and called `readEntries()` repeatedly on the same reader of the `FileSystemDirectoryEntry` obtained from `webkitGetAsEntry()`. It returned 100, 1, and 0 entries.
The first call alone cannot fetch the whole contents of the directory. You must keep using the same reader until it returns 0 entries. We did not check other browsers and versions.

## Basis

We created `000.txt` through `100.txt` in a temporary directory and dropped the OS directory from playwright-cli's `run-code` using Chromium's `Input.dispatchDragEvent`. With the entry obtained during the drop, we ran the following and confirmed `[100,1,0]`.

```js
const reader = directoryEntry.createReader();
const counts = [];
for (let i = 0; i < 3; i++) {
    const entries = await new Promise((resolve, reject) => reader.readEntries(resolve, reject));
    counts.push(entries.length);
}
```

The results on the real browser are recorded in [Acceptance Verification of C8b](../reviews/2026-10-07-web-entry-transfer.md#5-what-was-checked-and-found-sound).

## Implication

This fact is the premise for fetching the whole contents when bringing in an OS folder, in [Client Design §3.8](../design/client.md#38-entry-operations).
