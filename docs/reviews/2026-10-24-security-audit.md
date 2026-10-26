# Security Audit of the Daemon and the Web Client

## 1. About This Review

This document is a record of a security audit of the daemon, the engine and the web client, run as a source-first review with limited execution in an isolated sandbox, and of the second pass that decided the items the first pass left open.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The audited scope, perspectives, basis, findings, and subsequent handling |
| [design.md](../design.md) | Design rationale, invariants, deferred items. Destination for findings of type Design issue |
| [issues.md](../issues.md) | Unaddressed defects and improvement proposals. Destination for findings of type Defect and Improvement proposal |
| [reviews.md](../reviews.md) | The review list and the transcription status |

### 1.2 Review Date and Subject

- Review dates: 2026-10-24 (first pass) and 2026-10-25 (evaluation and second pass)
- Review type: security audit
- Target commit: `47b81b0` for the first pass, `4f071e1` for the second pass (24 commits later)
- Submodule core-rs: `831a476`
- Reviewer: AI agents (first pass and second pass), with an independent verifier of a different model family for the second pass
- Method: source-first review of entry point, propagation and sink, plus execution only against an instance built from the same commit in an isolated sandbox. No running instance of another owner was touched.

## 2. Scope and Perspectives

### 2.1 What Was Examined

The daemon's interface layer (routing, session, middleware, the feature handlers), the engine's storage, archive, media, thumbnail and timeline paths, the web client's entry transfer and location handling, the configuration and compose setup, the Dockerfile and the CI workflows.

The second pass additionally executed the built daemon and measured resident memory and crash behaviour for the media, EXIF, hashing and recursive-operation paths, and ran ffmpeg 9.0.2 on manifests that point at a local HTTP server.

### 2.2 What Was Not Examined

- Windows and macOS specific branches were read but not executed.
- The browser-side end-to-end behaviour of framing and of a cross-origin drop was not executed; only the preconditions were confirmed.
- Nested archive staging (N4 in the audit deliverable), the declared ZIP64 and 7z header sizes (N6), overlapping ZIP data ranges (N7) and the FIFO swap of a listed file (N9) were not reproduced.
- Supply chain work (pinning Actions to commit SHAs, Docker base digests, `cargo audit`) was not performed.
- Coverage is not claimed to be complete. The audit stopped after the decided items and the residual candidates were listed.

### 2.3 Perspectives

- Whether third-party content placed in a Library can reach a sink without an owner action
- Whether the Library boundary, `readonly`, and the session checks hold on every path
- Whether input size, pixel count and declared sizes are bounded before resources are spent
- Whether external processes receive attacker-controlled arguments or inputs
- Whether the deployed defaults (bind address, cookie attributes, Host handling) match the stated threat model

## 3. Overall Assessment

The boundary model holds up well: paths are resolved through the Library abstraction, the API never accepts absolute host paths, the archive index enforces its entry limit, and the new search and cross-library timeline APIs validate their Library ids and bind their SQL parameters.

The defects concentrate in resources and in the handling of third-party content. A deep folder tree aborts the daemon, media files allocate memory in proportion to their size or pixel count with no cap, and unauthenticated requests can allocate memory through argon2. These are reachable from content in a Library or from an unauthenticated device on the same network, and they are the items that should be fixed first.

Two items that the first pass left open are refuted: the ffmpeg SSRF candidate and the `safeRedirect` open redirect. The first-pass archive finding (C1) is fixed and covered by tests.

## 4. Findings

| ID | Summary | Type | Confidence | Handling |
| --- | --- | --- | --- | --- |
| [S-1](#s-1) | A deep folder tree overflows the worker stack and aborts the daemon | Defect | High | issues.md |
| [S-2](#s-2) | Media decoding has no size or dimension cap | Defect | High | issues.md |
| [S-3](#s-3) | Pre-auth password hashing has no concurrency limit | Defect | High | issues.md |
| [S-4](#s-4) | Recursive operations follow a symlink swapped in after the listing | Defect | High | issues.md |
| [S-5](#s-5) | The owner SPA can be framed by a same-site origin | Improvement proposal | Medium | issues.md |
| [S-6](#s-6) | Malformed archives are reported as internal server errors | Defect | High | issues.md |
| [S-7](#s-7) | Setup and login accept any Host | Design issue | High | design.md §11.2 |
| [S-8](#s-8) | ffmpeg and ffprobe fetch external URLs from a local file | Refuted | High | none |
| [S-9](#s-9) | `safeRedirect` allows an open redirect through control characters | Refuted | Medium | none |
| [S-10](#s-10) | A declared ZIP64 size above `i64::MAX` stops the timeline scan | Refuted | Medium | none |
| [S-11](#s-11) | External drag payload, nested archive staging, header-declared sizes, overlapping ranges, FIFO swap | Unverified | - | none |

S-1, S-2 and S-3 form one group: each spends memory or availability on input that does not need an owner action. S-4 and S-7 are boundary and deployment issues with their own fixes.

<a id="s-1"></a>
### S-1. A Deep Folder Tree Overflows the Worker Stack and Aborts the Daemon

**Type: Defect** / **Confidence: High**

**Location**

[`delete_tree`](../../daemon/entrypoints/daemon/src/service/mutation/operation.rs#L525), [`transfer_entry`](../../daemon/entrypoints/daemon/src/service/mutation/operation.rs#L315).

**What is wrong**

Both recurse once per level of the tree. The recursion consumes the 2 MiB stack of a tokio worker, and the runtime aborts the process.

**Basis**

A directory chain of depth 1400 (relative path length 2799) was deleted with `recursive=true`. The request returned `202`, and a few seconds later the log held `thread 'tokio-rt-worker' has overflowed its stack` and `fatal runtime error: stack overflow, aborting`. `/api/v1/health` stopped answering.

**Impact**

One request stops every Library until the daemon is restarted. The same tree can be created through the API or carried inside an archive.

**Handling**

[issues.md: deep folder trees overflow the worker stack](../issues/deep-folder-trees-overflow-the-worker-stack.md).

<a id="s-2"></a>
### S-2. Media Decoding Has No Size or Dimension Cap

**Type: Defect** / **Confidence: High**

**Location**

[`read_exif`](../../daemon/modules/engine/src/core/media/image.rs#L57), [`generator.rs`](../../daemon/modules/engine/src/core/thumbnail/generator.rs#L405), [`extract_image_pixels`](../../daemon/modules/engine/src/core/media/ffmpeg.rs#L62), [`process::run`](../../daemon/modules/engine/src/core/media/process.rs#L9), [`ImageConverter::decode`](../../daemon/modules/engine/src/core/image/converter.rs#L40).

**What is wrong**

The EXIF reader receives the whole file, the image path reads the whole file, the PPM path collects all of stdout, and the decoder builds a `DynamicImage` with no `image::Limits`. Display and grid permits are separate, so the concurrency limit applies twice.

**Basis**

Measured with the shipped ffmpeg 9.0.2 and the release daemon: a 1 GiB TIFF raised RSS by 761 MB during a timeline aggregation; a 1 GiB PNG raised it by 934 MB for one thumbnail; an 80 KB 12000x12000 AVIF raised it by 941 MB through PPM. All requests returned `200`.

**Impact**

A few requests reach several GiB. The timeline repeats the EXIF read on every aggregation, so one large file keeps the pressure up.

**Handling**

[issues.md: media decoding has no size or dimension cap](../issues/media-decoding-has-no-size-cap.md).

<a id="s-3"></a>
### S-3. Pre-Auth Password Hashing Has No Concurrency Limit

**Type: Defect** / **Confidence: High**

**Location**

[`PasswordDigest::hash`](../../daemon/modules/engine/src/service/auth/password.rs#L9), [`PasswordDigest::verify`](../../daemon/modules/engine/src/service/auth/password.rs#L26), [`login`](../../daemon/entrypoints/daemon/src/interface/features/auth.rs#L45), [`createInitialSetup`](../../daemon/entrypoints/daemon/src/interface/features/setup.rs#L20).

**What is wrong**

`Argon2::default()` runs inside `spawn_blocking` with no permit and no rate limit. Each run holds about 16 MB.

**Basis**

300 concurrent logins with a wrong password raised RSS from 39 MB to 4.09 GB on a host with 4 GB available. 100 concurrent took 1.5 GB. No credentials were needed.

**Impact**

An unauthenticated device on the same network can push the process into the OOM killer. The memory bound applies even on a trusted network.

**Handling**

[issues.md: pre-auth password hashing has no concurrency limit](../issues/pre-auth-hashing-has-no-concurrency-limit.md). The rate policy itself stays deferred in [design.md §11.2](../design.md#limiting-login-attempts).

<a id="s-4"></a>
### S-4. Recursive Operations Follow a Symlink Swapped in After the Listing

**Type: Defect** / **Confidence: High**

**Location**

[`children`](../../daemon/entrypoints/daemon/src/service/mutation/operation.rs#L483), [`delete_tree`](../../daemon/entrypoints/daemon/src/service/mutation/operation.rs#L525), [`transfer_entry`](../../daemon/entrypoints/daemon/src/service/mutation/operation.rs#L315).

**What is wrong**

`children()` rejects a symlink with `symlink_metadata`, but the operation resolves the same child again with `get_entry()`, which follows links.

**Basis**

The first pass reproduced the control and the attack: with the symlink in place at listing time the file outside the Library survived; with the swap after the listing the recursive delete removed it. The second pass confirmed that the code is unchanged at `4f071e1`.

**Impact**

A lower-trust principal that can create and rename symlinks inside a Library can delete or copy files outside it. The owner does not have to act.

**Handling**

[issues.md: recursive operations follow a swapped symlink](../issues/recursive-operations-follow-swapped-symlinks.md).

<a id="s-5"></a>
### S-5. The Owner SPA Can Be Framed by a Same-Site Origin

**Type: Improvement proposal** / **Confidence: Medium**

**Location**

[`ApiServer::router`](../../daemon/entrypoints/daemon/src/interface/server.rs#L58), [session cookie attributes](../../daemon/entrypoints/daemon/src/interface/session.rs#L62).

**What is wrong**

No layer sets `X-Frame-Options`, `frame-ancestors` or `Cross-Origin-Resource-Policy`. `SameSite=Strict` does not stop a same-site frame from carrying the cookie.

**Basis**

The SPA entry and the API responses carried no framing headers. The click-through was not executed in a browser, so the two-click delete is inferred from the framing precondition.

**Impact**

A page on another port of the same host can overlay the client and make the owner trigger a destructive action. The attacker cannot read the result.

**Handling**

[issues.md: the owner SPA can be framed](../issues/owner-spa-can-be-framed.md). The headers are cheap and can be added before the reproduction.

<a id="s-6"></a>
### S-6. Malformed Archives Are Reported as Internal Server Errors

**Type: Defect** / **Confidence: High**

**Location**

[`ZipArchive::map_error`](../../daemon/modules/engine/src/core/nfs/driver/archive/zip.rs#L149).

**What is wrong**

`ZipError::UpstreamReadError` becomes `Error::from(io::Error)`, so a size mismatch in client input is reported as a server failure.

**Basis**

A hand-built ZIP64 declaring an uncompressed size of 2^63 made the folder listing return `500 InternalServerError`.

**Impact**

A client cannot tell a broken file from a daemon fault, and the failure is counted as a server error in logs.

**Handling**

[issues.md: malformed archives are reported as internal server errors](../issues/malformed-archives-are-reported-as-internal-errors.md).

<a id="s-7"></a>
### S-7. Setup and Login Accept Any Host

**Type: Design issue** / **Confidence: High**

**Location**

[`createInitialSetup`](../../daemon/entrypoints/daemon/src/interface/features/setup.rs#L20), [`login`](../../daemon/entrypoints/daemon/src/interface/features/auth.rs#L45), [`OriginCheck`](../../daemon/entrypoints/daemon/src/interface/middleware/origin.rs#L17).

**What is wrong**

Neither handler uses the `Host` it receives. `OriginCheck` compares `Origin` with `Host`, which both belong to the attacker under DNS rebinding, so it does not cover these two endpoints.

**Basis**

On an uninitialized daemon, `POST /api/v1/setup` with `Host: rebind.attacker.example` returned `201` with a session cookie, and that cookie worked for later API calls. After setup the endpoint returns `409`, so the window is limited to first-run setup.

**Impact**

A page that passes DNS rebinding to a local address can claim ownership while the daemon is uninitialized. How much this matters depends on how the daemon is reached.

**Handling**

Deferred in [design.md §11.2](../design.md#validating-the-host-header-for-setup-and-login). The choice between a Host allowlist, a one-time setup token, and requiring Origin is a deployment decision, so no issue is filed for it.

<a id="s-8"></a>
### S-8. ffmpeg and ffprobe Fetch External URLs from a Local File (Refuted)

**Type: Refuted** / **Confidence: High**

The image and video paths pass the file straight to ffmpeg and ffprobe with no `-protocol_whitelist` and no fixed input format. A DASH manifest or an HLS playlist inside a `.mp4` would make the tool fetch a remote URL if the default allowed it.

It does not. With ffmpeg 9.0.2 and ffprobe 9.0.2, both report `Protocol 'http' not on whitelist 'file,crypto,data'` and do not open the URL; the local HTTP server received no request. libavformat sets the nested protocol whitelist from the input URL, and a local file yields `file,crypto,data`.

The daemon requires ffmpeg 9 or newer at startup ([`media_check.rs`](../../daemon/entrypoints/daemon/src/media_check.rs#L9)), so the older behaviour that the first pass measured is not reachable in the shipped configuration. No fix is required. Pinning the whitelist explicitly remains reasonable as a guard against future default changes, but it is not a defect.

<a id="s-9"></a>
### S-9. `safeRedirect` Allows an Open Redirect Through Control Characters (Refuted)

**Type: Refuted** / **Confidence: Medium**

[`safeRedirect`](../../ui-web/src/location.ts#L179) still accepts `/\t/evil.example/` because it only checks three lexical prefixes. The consequence does not follow: `navigate({ href })` reaches `documentNavigation` only when the href has a scheme or `reloadDocument` is set, and `getUrlScheme` returns nothing here, so the router calls `history.pushState` with the raw string. `pushState` rejects a cross-origin URL with `SecurityError` and the page does not move.

The function's documented contract is still wider than its implementation, so tightening it is a reasonable defence in depth, but there is no open redirect.

<a id="s-10"></a>
### S-10. A Declared ZIP64 Size Above `i64::MAX` Stops the Timeline Scan (Refuted)

**Type: Refuted** / **Confidence: Medium**

`async_zip 0.0.18` rejects the hand-built fixture (uncompressed size 2^63, local header of 4 bytes) with `unexpected end of file`, so the value never reaches `i64::try_from`. The timeline aggregation over the same file completed and set `completedAt`.

The 7z leg of the original claim was not reproduced, so a 7z header that declares a size above `i64::MAX` is still unverified.

<a id="s-11"></a>
### S-11. Items Left Unverified

**Type: Unverified**

These were not reproduced in the second pass. They are recorded so that a later pass does not treat them as settled.

- **Nested archive staging.** Whether a scan holds every sibling nested archive of one tree in staging at the same time, and how the peak compares to the configured limits.
- **Header-declared sizes.** Whether a ZIP64 with an absurd entry count or central-directory size, or a 7z with an absurd header size, makes inspect allocate before validating the data. The ZIP leg of S-10 suggests the size check is strict, but the count and header paths are separate.
- **Overlapping data ranges.** Whether ZIP entries that share one data range are decoded once per entry, and how that multiplies during extraction.
- **FIFO swap.** Whether replacing a listed file with a FIFO stops the single background permit and with it every Library's background work. The structure is unchanged from the first pass, which reasoned about it without executing.
- **Cross-origin drag.** [`readEntryDrag`](../../ui-web/src/library/dnd.ts#L18) and [`accepts`](../../ui-web/src/library/dnd.ts#L139) require only the MIME type, so a payload from another origin is indistinguishable from an internal drag. The browser drop itself was not executed.

For each item the audit deliverable holds the fixture and command needed; this repository does not carry them.

## 5. What Was Checked and Found Sound

- The archive index limit is enforced on the raw count, on every synthesized parent, and after synthesis; a shared deep parent is synthesized once, and the three regression tests in `index.rs` cover it.
- The cross-library search and timeline APIs reject unknown and duplicate Library ids, require an empty `path` when several Libraries are named, bind SQL parameters, and cap `limit` at 1000.
- Listing snapshots are keyed by Library, path, `recursive` and `include_hidden` only, so varying the search conditions does not create new snapshots.
- The timeline write transactions start with `BEGIN IMMEDIATE` and wait through a 30 second busy timeout; no deadlock was observed.
- `heif-dec` and ffmpeg receive their arguments through `Command::arg`, never a shell, and are run under a timeout with `kill_on_drop`.
- The Dockerfile fetches ffmpeg 9.0.2 from a fixed release with a sha256 check.
- The cookie is `HttpOnly` and `SameSite=Strict`; `OriginCheck` rejects cookie-bearing mutations whose `Origin` does not match `Host`.
- The timeline `limit` is capped by the OpenAPI schema and the folder listing enforces the same bound.

## 6. Afterward

- 2026-10-25: Transcribed S-1 through S-6 to issues.md, and S-7 to the deferred items of design.md. S-8 through S-10 need no entry; the refutation is recorded here. S-11 stays unverified and is listed in the audit deliverable with the reproduction steps.
