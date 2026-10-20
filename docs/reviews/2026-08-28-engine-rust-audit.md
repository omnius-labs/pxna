# Pxna engine Rust Full Audit

## 1. About This Review

This document is a record of auditing the hand-written production Rust code of `daemon/modules/engine` from both the Rust coding conventions and implementation quality.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The audited scope, perspectives, basis, findings, and subsequent handling |
| [design.md](../design.md) | Design rationale, invariants, reasons for adoption, open issues |
| [issues.md](../issues.md) | Unaddressed defects and improvement proposals confirmed in code |

### 1.2 Review Date and Subject

- Review dates: 2026-08-28 to 2026-08-29
- Review type: audit review
- Target commit: `d574e5c43a2d61712d69aa0c0d425f573a508231`
- Target snapshot: a working tree in which uncommitted `free-fn` markers added to 11 files of `daemon/modules/engine` are layered on the target commit
- Reviewer: Codex (GPT-5)

## 2. Scope and Perspectives

### 2.1 What Was Examined

We read through the hand-written production code in `daemon/modules/engine/src`, excluding `generated`.
The boundary is the crate's `base`, `core`, `model`, `service`, error, and the public module layout. To confirm references and design contracts, we also read the related tests, the [Rust Coding Conventions](../coding/rust.md), and the [Storage Layer Design](../design/storage.md).

### 2.2 What Was Not Examined

- `daemon/modules/engine/src/generated` is RocketPack output and outside the scope for judging the hand-written code's conformance to the conventions, so we did not look at it.
- Code under `#[cfg(test)]` and test functions are outside the conventions' scope, so we did not audit their structure, and read only the places needed to confirm the conditions under which findings hold and the existing verification scope.
- `daemon/entrypoints`, `daemon/refs`, the web client, and iOS are separate components and were excluded from the audit. However, to compare the engine's error conversions, we read only the daemon-side conversions of the same kind.
- Runtime behavior on Windows and Linux cannot be run in the current macOS environment, so we limited ourselves to static checks of `cfg` branches.
- We did not run Miri's unsafe execution checks because it is not built into the toolchain.

### 2.3 Perspectives

- Whether types, functions, and modules place responsibilities with their natural owners
- Whether the lifetimes of dependencies, constructors, and public boundaries match the responsibilities
- Whether the classification of errors and the source chain preserve the context of the failure
- Whether unsafe code and path types guarantee the necessary invariants as types
- Whether persistence, file publication, concurrency, and failure paths could corrupt state or be mistaken for success
- Whether the claims of the current design, markers, tests, and lint match the implementation

## 3. Overall Assessment

The engine's existing tests and the workspace lint pass, but the unsafe layout premise of R-1 and the failure paths of R-2 through R-6 remain, so we do not judge the engine as a whole to be in a state where it can be connected to new APIs as is.
The main success paths, resource limits, and staging publication rules of storage and archives are tested, and the findings concentrate on type soundness, an unconnected cache foundation, error boundaries, and responsibility placement.
R-1 takes top priority, followed by fixing the behaviors of R-2 through R-6 together with regression tests.

## 4. Findings

| ID | Summary | Type | Confidence | Handling |
| --- | --- | --- | --- | --- |
| [R-1](#r-1) | The unsafe cast of `NestedPath` has no layout guarantee | Defect | High | issues.md I-10 |
| [R-2](#r-2) | A thumbnail cache write failure is returned as success | Defect | High | issues.md I-11 |
| [R-3](#r-3) | An overwriting rename of the same key deletes the entry | Defect | High | issues.md I-12 |
| [R-4](#r-4) | bulk delete does not delete the keys after a missing key | Defect | High | issues.md I-13 |
| [R-5](#r-5) | `overwrite` does not represent the actual write mode | Improvement proposal | High | issues.md I-14 |
| [R-6](#r-6) | A migration database error is converted to a time error | Defect | High | issues.md I-15 |
| [R-7](#r-7) | `NestedPathBuf` does not own the path contract | Improvement proposal | High | issues.md I-16 |
| [R-8](#r-8) | Some conversions lose the external error's source chain | Improvement proposal | High | issues.md I-17 |
| [R-9](#r-9) | 32 free functions remain with unresolved owners | Improvement proposal | High | issues.md I-18 |

R-1 and R-7 are on the same path type's boundary, and it is safe to fix soundness first and then consolidate the constructor and operation invariants into the same type.
R-2 through R-5 concentrate in the unconnected thumbnail cache foundation, so it is efficient to add regression tests for them together before connecting M4.

<a id="r-1"></a>
### R-1. The unsafe Cast of `NestedPath` Has No Layout Guarantee

**Type: Defect** / **Confidence: High**

**Location**

[`NestedPath` and `NestedPath::new`](../../daemon/modules/engine/src/model/nested_path.rs#L48), [`NestedPath::from_inner_mut`](../../daemon/modules/engine/src/model/nested_path.rs#L67).

**What is wrong**

A `str` fat pointer is dereferenced as a `NestedPath` fat pointer, but `NestedPath` remains in the default Rust representation and does not declare a guarantee that it has the same layout as `str`.

**Basis**

The two unsafe casts need the sameness of representation as a safety condition.
The [type layout in the Rust Reference](https://doc.rust-lang.org/stable/reference/type-layout.html#the-rust-representation) states that the default representation has no other layout guarantees, and only `repr(transparent)` guarantees the same layout and ABI as a single field.

**Impact**

Ordinary path passing, including the deref of `NestedPathBuf`, goes through this cast, so a soundness hole that depends on the compiler's layout choice spreads across the crate's whole storage API.

**Handling**

Transcribed to I-10 of issues.md.

<a id="r-2"></a>
### R-2. A Thumbnail Cache Write Failure Is Returned as Success

**Type: Defect** / **Confidence: High**

**Location**

[`ThumbnailCache::put`](../../daemon/modules/engine/src/core/thumbnail/cache.rs#L36) and the [`allow(unused)`](../../daemon/modules/engine/src/core/thumbnail/cache.rs#L13) on the whole impl.

**What is wrong**

After discarding the `Result` of the RocksDB write, it always returns `Ok(())`, so the caller cannot detect failures of the disk, the database, or the blocking task.

**Basis**

The return value of `put_value_with_meta(...).await` has neither `?` nor explicit handling.
The `allow(unused)` on the whole impl also suppresses `unused_must_use`, so `cargo make lint` does not detect this failure.

**Impact**

When thumbnail generation is connected in M4, a failing cache write looks like a success, and the same thumbnail continues to be regenerated afterward.

**Handling**

Transcribed to I-11 of issues.md.

<a id="r-3"></a>
### R-3. An Overwriting rename of the Same Key Deletes the Entry

**Type: Defect** / **Confidence: High**

**Location**

The existing-destination handling of [`KeyValueRocksdbStorage::rename_key`](../../daemon/modules/engine/src/base/storage/key_value_rocksdb_storage.rs#L48).

**What is wrong**

When `old_key == new_key` and `overwrite == true`, it deletes the block and metadata of the same ID, and deletes right after putting the same name, so the whole entry disappears.

**Basis**

With the same key, `old_id` and `new_id` are identical, and the delete in the existing-destination branch and the name update all act on the same value.
The existing tests cover only renames between different keys and do not verify the same key.

**Impact**

If a caller passes a self-rename as a no-op, a regenerable cache entry is lost.
Current production code has no references to this method, so this is a latent defect before connection.

**Handling**

Transcribed to I-12 of issues.md.

<a id="r-4"></a>
### R-4. bulk delete Does Not Delete the Keys After a Missing Key

**Type: Defect** / **Confidence: High**

**Location**

The missing-key branch of [`KeyValueRocksdbStorage::delete_bulk`](../../daemon/modules/engine/src/base/storage/key_value_rocksdb_storage.rs#L356).

**What is wrong**

If even one key to delete does not exist, it returns `Ok(())` at that point without writing the batch itself, so the existing keys in the same argument are not deleted either.

**Basis**

The `return Ok(())` on a missing key ends both the loop and the method and does not reach `db.write(batch)`.
Single delete and the existing tests treat deleting a missing key as success, which does not match a contract in which bulk also ignores only the missing keys.

**Impact**

If some keys have already disappeared during a bulk reclaim of the cache, all the remaining targets stay, and capacity reclamation does not progress.
Current production code has no references to this method.

**Handling**

Transcribed to I-13 of issues.md.

<a id="r-5"></a>
### R-5. `overwrite` Does Not Represent the Actual Write Mode

**Type: Improvement proposal** / **Confidence: High**

**Location**

[`put_value_with_meta`](../../daemon/modules/engine/src/base/storage/key_value_rocksdb_storage.rs#L126) and [`put_value`](../../daemon/modules/engine/src/base/storage/key_value_rocksdb_storage.rs#L204).

**What is wrong**

Even with `overwrite == false` it overwrites an existing value, and returns `AlreadyExists` when the key does not exist, so the argument name and the error kind do not represent the actual update-only behavior.

**Basis**

The `!overwrite` branch of both methods writes to `Some(id)` and converts `None` to `AlreadyExists`.
The existing tests also pin that an existing value is updated with `false`, so this is not an incidental single line but a mismatch between the API contract and the implementation.

**Impact**

A caller that expected create-only from the argument name replaces an existing cache, and a caller that expected update-only handles a missing key as an existence conflict.
Current production callers pass only `true`.

**Handling**

Transcribed to I-14 of issues.md as a low-priority improvement proposal.

<a id="r-6"></a>
### R-6. A Migration Database Error Is Converted to a Time Error

**Type: Defect** / **Confidence: High**

**Location**

The engine's [`From<omnius_core_migration::Error>`](../../daemon/modules/engine/src/error.rs#L220).

**What is wrong**

It converts `omnius_core_migration::ErrorKind::DatabaseError` to the engine's `ErrorKind::TimeError`, which changes the classification of the cause of failure.

**Basis**

The match arm explicitly maps different enum variants.
The daemon-side conversion used for comparison maps the same source kind to `DatabaseError`.

**Impact**

If migration is connected inside the engine, a database failure propagates to logging, retry, and HTTP conversion as a time conversion failure.
The current engine has no caller of migration.

**Handling**

Transcribed to I-15 of issues.md.

<a id="r-7"></a>
### R-7. `NestedPathBuf` Does Not Own the Path Contract

**Type: Improvement proposal** / **Confidence: High**

**Location**

The lossy [`From<Path>`](../../daemon/modules/engine/src/model/nested_path.rs#L126) and the infallible [`FromStr`](../../daemon/modules/engine/src/model/nested_path.rs#L134).

**What is wrong**

It converts a non-UTF-8 OS path into a different path containing replacement characters, and representations that are invalid by design, such as empty intermediate segments, also succeed, so the types cannot distinguish a checked `NestedPath` from an unchecked string.

**Basis**

`From<Path>` uses `to_string_lossy`, and `FromStr::Err` is `Infallible`.
On the other hand, the local driver and the storage design do not do lossy conversion and specify that an empty segment is allowed only at the end.

**Impact**

When the daemon's Library path resolution is connected in the future, the compiler will not stop a forgotten check at the boundary, and collisions between display paths and actual entities or invalid archive ranks can be passed downstream.

**Handling**

Transcribed to I-16 of issues.md as a low-priority improvement proposal.

<a id="r-8"></a>
### R-8. Some Conversions Lose the External Error's Source Chain

**Type: Improvement proposal** / **Confidence: High**

**Location**

The PKCS#8 [`From` implementation](../../daemon/modules/engine/src/error.rs#L162), the archive's [`source_path_error`](../../daemon/modules/engine/src/core/nfs/driver/archive.rs#L147), ZIP's [`map_zip_error`](../../daemon/modules/engine/src/core/nfs/driver/archive/zip.rs#L151), and 7z's [`map_sevenz_error`](../../daemon/modules/engine/src/core/nfs/driver/archive/sevenz.rs#L151).

**What is wrong**

They convert the original error only to a string or discard the argument, so the cause cannot be traced from `std::error::Error::source`.

**Basis**

Each implementation uses `Error::new` and does not pass external errors it could keep to `Error::from_error`.
[Rust Coding Conventions §3.2](../coding/rust.md#32-split-error-conversion-by-whether-context-matters) specifies that even context-dependent conversions keep the original error as the source wherever possible.

**Impact**

In diagnosing archives, cryptographic keys, and I/O, OS error codes and library-specific variants are lost, and upper layers cannot tell the cause apart from anything but the message.

**Handling**

Transcribed to I-17 of issues.md as a low-priority improvement proposal.

<a id="r-9"></a>
### R-9. 32 Free Functions Remain with Unresolved Owners

**Type: Improvement proposal** / **Confidence: High**

**Location**

The archive's [`extractor.rs`](../../daemon/modules/engine/src/core/nfs/driver/archive/extractor.rs#L7), the local driver's [adapters](../../daemon/modules/engine/src/core/nfs/driver/local.rs#L165), staging's [publish and sweep](../../daemon/modules/engine/src/core/nfs/staging.rs#L163), and the storage service's [path operations](../../daemon/modules/engine/src/service/storage.rs#L218).

**What is wrong**

`debt` markers remain: 18 in archive, 4 in the local driver, 6 in staging, and 4 in the path model and service, so the responsibility boundaries of types and files have not reached the conventions' completed form.
In particular, `extract.rs`, which expresses only a processing stage, is exactly the structure that the conventions explicitly cite as an example to avoid.

**Basis**

`env -u RUSTC_WRAPPER cargo make lint-style` listed all 32 in the module as `debt`.
`ArchiveFormat`, `ArchiveEntryPath`, `ZipArchive`, `SevenZipArchive`, `FileStorageLocal`, the staging concepts, and `NestedPath`, which the markers point to, can also be confirmed in code as natural destinations.

**Impact**

Changes to related conversion and validation are scattered across free functions, and mismatches between the contract and the error context like R-7 and R-8 cannot be stopped at type boundaries.

**Handling**

Consolidated and transcribed to I-18 of issues.md as a low-priority improvement proposal.

## 5. What Was Checked and Found Sound

- The 7 `allow` markers in `rename.rs` correspond to boundaries the conventions allow (OS system calls, FFI, and platform fallbacks), and `lint-style` also accepted the markers.
- Archive entry paths are checked for absolute paths, drive prefixes, NUL, `.`, `..`, empty segments, and length, and the corresponding tests pass both the accepted and rejected cases.
- ZIP materialization verifies the declared size and CRC, and 7z limits the block size in advance and treats the library's checksum error as a failure.
- We confirmed with tests that `ArchiveRuntime` keeps the semaphore permit in the blocking worker even after a timeout, and that the worker's result is dropped on completion.
- We confirmed with tests that `StagingFile` checks the destination's parent directory, cleans up on drop for failure and cancellation, and publishes by rename only on success.
- We confirmed with tests that `MaterializedFile` and `OpenedFile` keep the owned temp until the last lease is gone and delete it on the last drop.
- The old authentication's `todo!` and the unfinished `ThumbnailGenerator` are stated as unconnected in [design.md §12.1](../design.md#121-implementation-status), so we did not file them again as new defects this time.
- `env -u RUSTC_WRAPPER cargo make lint` succeeded.
- `env -u RUSTC_WRAPPER cargo make test` inside the managed sandbox failed only on 3 PostgreSQL testcontainer tests, because the Docker connection returned `Operation not permitted`. Rerunning the same command with permitted privileges passed 131 and skipped 6.

## 6. Afterward

- 2026-08-29: Transcribed R-1 through R-9 to I-10 through I-18 of issues.md.
- 2026-08-29: Fixed R-1 through R-9 in `5466708`, `c3cf956`, and `6031727`, and deleted I-10 through I-18 from issues.md.
