# Malformed Archives Are Reported as Internal Server Errors

**Severity: low** (confirmed by reproduction)

Investigated 2026-10-25 / Target commit `4f071e1`

One item of [issues.md](../issues.md). Found while reviewing the changes after the security audit of 2026-10-24.

## Symptom

Opening a folder inside a ZIP whose declared sizes do not match the data returns `500 Internal Server Error` instead of a client error. The same file is skipped by the timeline scan without an error, so the two paths classify the same input differently.

Reproduction: a hand-built ZIP64 whose central directory declares an uncompressed size of 2^63 and whose local header holds 4 bytes. `GET /api/v1/libraries/contents/folders?path=/poison.zip` returns `{"kind":"InternalServerError"}`.

## Locations

- [zip.rs#L149](../../daemon/modules/engine/src/core/nfs/driver/archive/zip.rs#L149) — `map_error`, which handles `ZipError::UpstreamReadError` through `Error::from(io::Error)`
- [zip.rs#L31](../../daemon/modules/engine/src/core/nfs/driver/archive/zip.rs#L31) — `ZipArchive::inspect`, where the error is raised

## Cause

`async_zip` reports the size mismatch as `UpstreamReadError`, which wraps an `io::Error` of kind `UnexpectedEof`. The conversion keeps it as an internal error, so the API reports a server failure for input the client supplied.

## Impact

A client cannot tell a broken file from a fault in the daemon, and the error is counted as a server failure in logs. There is no data loss and no availability effect.

## Handling Policy

Treat `UpstreamReadError` whose kind is `UnexpectedEof` or `InvalidData` as `InvalidFormat` for archives, the same way the archive readers classify other malformed input. Keep genuine I/O failures as internal errors.

The regression test opens the fixture above and asserts a `400` with `UnsupportedType`.
