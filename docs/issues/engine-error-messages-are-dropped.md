# Engine Error Messages Are Dropped When the Daemon Converts Them for API Responses

**Severity: low** (visible today)

Investigated 2026-11-04 / Target commit `74a485b`

One item of [issues.md](../issues.md).

## Symptom

A 4xx response for an error that originated in the engine carries only `kind`, even though the engine attached a message and `ApiError` sets `body.message` for non-500 responses.
For example, a media probe that finds no video stream or an archive path that is not a directory returns no message, so a client can show only the kind.

## Locations

- [error.rs#L253](../../daemon/entrypoints/daemon/src/error.rs#L253) — the `From<omnius_pxna_engine::Error>` conversion calls `Error::from_error(e, kind)`, which sets `message` to `None` and keeps the engine error only as `source`
- [error.rs#L320](../../daemon/entrypoints/daemon/src/error.rs#L320) — `ApiError::from_error` sets `body.message` from `error.message()` for non-500 responses
- [error.rs#L317](../../daemon/entrypoints/daemon/src/error.rs#L317) — a 500 response logs the error and omits the message by design

## Cause

`Error::from_error` does not copy the source's message.
The engine error's message survives only in the source chain, which `ApiError` does not read.
The conversion therefore loses the message for every engine error, not only the media ones.

## Impact

A client that shows `message` receives only the kind for 4xx errors that originate in the engine.
The API still distinguishes unsupported, not found, conflict, and the other kinds, so branching on `kind` is unaffected; only the detail is missing.
There is no data loss.

## Handling Policy

Copy the engine error's message into the daemon error in the conversion, and keep the current rule that `ApiError` omits the message for 500 responses.
Add a test that an engine error with a message produces a 4xx `ErrorResponse` with that message, and a test that a 500 response still omits it.
