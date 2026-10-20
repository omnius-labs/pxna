# Diff Review and Acceptance Verification of the Web Client's Entry Transfer and Bring-In

## 1. About This Review

A record of checking C8b's move, copy, and bring-in against the design and the acceptance conditions.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The diff review and the results of verification against the real API and real screens |
| [Client Design](../design/client.md#38-entry-operations) | The contracts for operations, conflicts, bringing in folders, and notifications, and the reasons for adopting them |
| [design.md](../design.md#82-determining-sync-and-async) | The API contracts for sync and async, conflict, readonly, and uploads |
| [knowledges.md](../knowledges.md) | Measurements of Chromium's reading and leave confirmation |
| [issues.md](../issues.md) | Unaddressed defects |

For the map of the document family, see [design.md](../design.md#11-division-of-responsibility-between-documents).

### 1.2 Review Date and Subject

A diff review on 2026-10-07. The base is `1311b51` and the implementation head is `af54dd7`. The cleanup of documents that reflect the current state and verification results was also covered in the same working tree as this review. The reviewers were Codex (GPT-6) and a verifier (GPT-5.6-Terra).

## 2. Scope and Perspectives

### 2.1 What Was Examined

We checked the implementation and tests in `ui-web/src` and the design and plan changes in the target diff. At each step the verifier inspected read-only, and we committed after fixing the findings and receiving `pass`.
We started the daemon built in this worktree on 6056 and Vite on 5179, and prepared a normal Library, a readonly Library, another Library, and an archive in `/tmp/pxna-verify-transfer`. We operated real screens in Chromium 155.0.8059.39 with playwright-cli. It was a headless run inside Docker, with screen widths of 1440 and 500 CSS px and a height of 1000 CSS px.
For the API we used curl with a cookie, and for the screens we checked the DOM, requests and responses, and the results on real files. For cancellation and the concurrency count, we held chunks with a route, and for the vanishing destination we resumed sending after the actual folder deletion had completed.

### 2.2 What Was Not Examined

- The implementation as a whole outside the diff and iOS are out of scope, so we did not inspect them.
- Real devices of Safari, Firefox, and touch devices are outside the plan. Narrow screens were checked with Chromium's width.
- Resuming uploads after a reload, internal drags across Libraries, and automatic reflection of changes by external clients are outside the plan and not implemented.
- We did not check physically closing a tab. The leave confirmation was checked with a reload that fires the same native `beforeunload`.

### 2.3 Perspectives

- Whether the API's response shapes, sync and Operation, the 4 values of conflict, and size boundaries match the contract.
- Whether the clipboard, the destination's readonly, re-input during conflict confirmation, and partial success of mixed requests are preserved.
- Whether internal and OS drags are distinguished, and whether auto-scroll and expansion work.
- Whether the upload queue, chunk offsets, cancellation and session discarding, and updates after completion are consistent.
- Whether the panel's state, failure reasons, narrow screens, and the leave confirmation, as well as existing checks, have no regressions.

## 3. Overall Assessment

Acceptance conditions 1 to 10 were met with the real API, real screens, and intended route interruptions. R-1 and R-2 were fixed before committing the implementation, and we confirmed the regression tests and the real screens. The target diff may proceed as is. The limits of the scope checked are as in §2.2.

## 4. Findings

| ID | Summary | Type | Confidence | Handling |
| --- | --- | --- | --- | --- |
| [R-1](#r-1) | No success notification for sync when sync and async are mixed | Defect | High | Fixed in s-4 |
| [R-2](#r-2) | An OS drag that allows only copy cannot be accepted | Defect | High | Fixed in s-6 |

R-1 came from a branch that decided on a single kind of response, and R-2 from a branch that returned the same drag effect for internal and external drags. Each was fixed by distinguishing the results and the inputs.

<a id="r-1"></a>
### R-1. No Success Notification for Sync When Sync and Async Are Mixed

**Type: Defect** / **Confidence: High**

**Location**
The notification of transfer results in [TransferProvider.tsx](../../ui-web/src/components/TransferProvider.tsx#L38).

**What is wrong**
The initial implementation only opened the panel if there was an Operation, and did not notify the sync success that returned at the same time.

**Basis**
After the verifier's finding, we verified both notifications with a response mixing a 202 for multiple files and a 200 for a single folder. The [regression test](../../ui-web/src/pages/EntryOperations.test.tsx#L277) checks both the sync toast and the async panel.

**Impact**
With a selection that mixes files and folders, the user cannot confirm the part that completed synchronously.

**Handling**
Fixed in `cd103f3`. The decision to keep each request's result is recorded in [Client Design §5.1](../design/client.md#keep-each-requests-result-in-mixed-moves-and-copies), and since the defect is resolved, it is not transcribed to issues.

<a id="r-2"></a>
### R-2. An OS Drag That Allows Only Copy Cannot Be Accepted

**Type: Defect** / **Confidence: High**

**Location**
`dropEffect` in [dnd.ts](../../ui-web/src/library/dnd.ts#L67).

**What is wrong**
The initial implementation returned `move` even for an OS drag without Alt. With input that allows only copy, the browser did not accept the drop.

**Basis**
We checked with Chromium's native directory drag, and after fixing it to return `copy` for the OS's Files, the folder conflict confirmation, creating an alternate name, and transferring the contents completed. The [regression test](../../ui-web/src/library/dnd.test.ts#L42) also checks copy and the rejection for readonly.

**Impact**
Although the file-selection entrance worked, there was input from which bringing in from the OS could not be started.

**Handling**
Fixed in `327f66e`. This is a fix that follows the existing design of treating an OS bring-in as an upload, and since the defect is resolved, it is not transcribed to issues.

## 5. What Was Checked and Found Sound

### 5.1 The Real API's Contract

Before implementation, we confirmed the following with the daemon at `1311b51`, and fixed the design's prose about the sync move response.

| Subject | Measured |
| --- | --- |
| move and copy | A single-file or folder move within the same Library is 200 `{entries}`. A multiple or cross-Library move and a copy are 202 `{operation}`, with the identifier `operation.id`. `dstFolderPath` places items directly under it, keeping the original names. |
| conflict and Operation | Sync fail is 409, and skip is 200 with empty entries. Async fail gives failedEntries, and skip gives skippedCount. The published destinations of overwrite and rename go in resultEntries. |
| One-shot boundary | Exactly 32 MiB is 201, and 32 MiB + 1 byte is 413 `PayloadTooLarge`. |
| Session | Creation is 201 `{upload}`. A 16 MiB PUT is 204 with no body, and GET is 200 `{upload}` with committedSize advanced. commit is 201 `{file}`. |
| DELETE during execution | 204. The following GET is 200 and returns aborted and the committedSize before discarding. Later PUTs are 409, and the destination is unpublished. |

### 5.2 Acceptance Results on Real Screens

| Condition | Execution and result |
| --- | --- |
| 1. Cut and copy | Checked Ctrl/⌘ X and C and the menu. Cut shows opacity 0.4 and is released by replacing it with copy. No re-operation during input and confirmation. |
| 2. Paste | A single move within the same Library is 200, and a cross-Library move and a 3-item copy are 202. We checked the listing and tree updates after completion, the panel, and disabling for readonly and archives. |
| 3. Conflict confirmation | Displays the count and names. For paste and upload, we checked the alternate name of rename, keeping the existing file of skip, and 0 sends on abort. No confirmation without a same-name item. The requests and imports of the 4 policies were also checked in integration tests. |
| 4. Conflict after matching | We replaced the matching response with no conflict; a sync 409 shows a toast, and an async copy shows the reason from failedEntries. |
| 5. Internal drag | A move to the tree and tiles is 200, and a copy of a tile with Alt is 202. For readonly and archives, a rejection is displayed and there are 0 requests. |
| 6. Scroll and expansion | Scrolled 160px at the bottom edge of a 400-item listing. A closed tree folder expands in 1 second. Esc ends without sending. |
| 7. Bring-in | A 33 MiB file uses a session from the start with 16+16+1 MiB, and a PNG of about 3 MiB is a one-shot 201. Folder selection and a native OS folder drop send the contents after creating the parent. A real drop of 3 files used 2 sessions and 2 PUTs, and the third waited and was sent after the earlier ones completed. The entrances and drops for readonly and archives are rejected, with 0 writes. |
| 8. Progress, cancel, failure | Shows 16,777,216 / 34,603,008 bytes and sending, and cancel gives DELETE 204, aborted, committedSize=16,777,216, and no commit. Shows "取り消した" (canceled). After the destination folder was actually deleted, it shows 404 with the file, path, and `parent folder not found`. Successful files appear in the listing, and one update per file was also checked in the Provider's tests. |
| 9. Leave confirmation | On a reload during sending, the native beforeunload appeared, and dismiss returned. A reload after cancellation and after normal completion has no confirmation. Registering and removing the listener only during execution was also checked in tests. |
| 10. Notification and re-input | A sync 409 is a toast, and Operation and upload share the same panel. Ctrl/⌘+V during confirmation issues no new request, and at 500px neither operations nor the panel appear. |

Observation equivalent to playwright-cli's `network` was done in this version with `requests` and the request and response listeners of `run-code`. `drop --path` rejects directories, so we used Chromium's `Input.dispatchDragEvent` inside `run-code` and passed a real directory. Without changing the meaning of the input, we verified using `webkitGetAsEntry()` of a real file.
The browser verification scripts and raw logs were kept in the working directory `pxna-transfer`. The permanent results are kept in this document.

### 5.3 Machine Checks and External Behavior

`npm run typecheck`, `npm run lint`, and `npm test` of `ui-web` succeeded. Lint had 0 errors and the same 9 existing warnings. 48 files and 314 tests passed. `RUSTC_WRAPPER= cargo make lint` of `daemon` also succeeded. There are no Rust changes.
The 101 files of a real Chromium directory were fetched from the same reader as `[100,1,0]`. We recorded the checks of [directory reading](../knowledges/chromium-directory-reader.md) and [beforeunload](../knowledges/chromium-beforeunload.md) in external knowledge.
The 404 for folders without a representative thumbnail, and the 409, 404, and aborted requests triggered by interruptions, remained in the console/network. The operations under check proceeded to the expected end states.

## 6. Afterward

- 2026-10-07: Reflected the partial success of mixed transfers and the conflict policy for bringing in folders in §5.1 of client.md, and the current features in §6.1. Removed C8b from the remaining work and marked M13 complete.
- 2026-10-07: Moved the measurements of Chromium's directory reading and leave confirmation to knowledges. In the review of the new documents, we fixed the division of responsibility and the format of this section.
- 2026-10-07: Confirmed acceptance conditions 1 to 10 and where things moved to, and deleted the plan document and the plans.md row in the same change as the document updates. No unaddressed findings remain.
