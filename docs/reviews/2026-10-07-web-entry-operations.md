# Diff Review and Acceptance Verification of the Web Client's Basic Entry Operations

## 1. About This Review

A record of checking the web client's basic Entry operations and the activity panel against the design and the acceptance conditions.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The scope of the diff review, judgments, and the results of acceptance verification |
| [Client Design](../design/client.md) | The contracts for operations, keys, confirmation, notifications, and re-reading listings, and the reasons for adopting them |
| [design.md](../design.md#86-archives-and-readonly) | The API contracts for readonly and archives |
| [issues.md](../issues.md) | Unaddressed defects |

For the map of the family, see [design.md](../design.md#11-division-of-responsibility-between-documents).

### 1.2 Review Date and Subject

A diff review on 2026-10-07. The base is `1de0cde`, the initial implementation head is `178eaa9`, and the head for confirming the fixes is `4397b11`. The cleanup of documents that reflect the current state and verification results was also covered in the same day's working tree. The reviewers were Codex and a verifier (GPT-5.6-Terra).

## 2. Scope and Perspectives

### 2.1 What Was Examined

We checked the implementation and tests in `ui-web/src` and the design and plan changes in the target diff.
We started the daemon built in this worktree on 6056 and Vite on 5179, and prepared a normal Library, a readonly Library, and an archive in `/tmp/pxna-verify-ops`. We operated real screens in Chromium with playwright-cli, and checked the DOM, request bodies, responses, and file deletion and downloads.
Screen widths were 1280 and 500 CSS px, with a height of 800 CSS px. Because this runs inside Docker, the browser was started headless. The state storage location was an absolute path inside the temporary library.

### 2.2 What Was Not Examined

- The implementation as a whole outside the diff, iOS, and the operations of C8b are out of scope this time.
- Real devices of Safari, Firefox, and touch devices are out of scope, and narrow screens were checked by changing Chromium's width.
- We did not check canceling an actual long-running Operation, because the verification deletion finishes in a short time. Cancellation, slow progress, failure, and skipping were checked on real screens with the API responses replaced.

### 2.3 Perspectives

- Whether the operation's target, the API kind, the name rules, and the readonly determination match the design.
- Whether focus for input and confirmation, key operations, and the narrow-screen restrictions do not interfere.
- Whether the selection, the listing, the tree, and Operations are not left behind on success and on partial failure.
- Whether restoring Operations, the update interval, terminal states, and cancellation are consistent with the display.
- Whether type checking, lint, and existing tests have no regressions.

## 3. Overall Assessment

The acceptance conditions for the basic operations were met with the real API, real screens, and checks with the responses of slow Operations replaced.
We confirmed the fixes for R-1 and R-2 within the same review, and the target diff may proceed as is. The scope for canceling long-running Operations and for other browsers is as in §2.2.

## 4. Findings

| ID | Summary | Type | Confidence | Handling |
| --- | --- | --- | --- | --- |
| [R-1](#r-1) | An operation can be committed after switching to a narrow screen | Defect | High | Fixed within the same review |
| [R-2](#r-2) | A selected off-screen item cannot be edited with F2 | Defect | High | Fixed within the same review |

Both dealt only with the screen's state at the time the operation started, and did not deal with later changes in width or in the visible range.

<a id="r-1"></a>
### R-1. An Operation Can Be Committed After Switching to a Narrow Screen

**Type: Defect** / **Confidence: High**

**Location**
The edit and delete state in [BrowsePage.tsx](../../ui-web/src/pages/BrowsePage.tsx#L37), and the inputs and confirmations of the listing and tree.

**What is wrong**
In the initial target, an input and a delete confirmation started on a wide screen remained on a narrow screen, and the change could be committed.

**Basis**
The verifier changed the width to 500px on a real screen and confirmed that the name input and the "削除する" (Delete) button remained. After the fix, the inputs of the grid, list, and tree disappeared, and the listing's confirmation also disappeared. They did not come back when returning to a wide screen, and the change POSTs were 0. A regression test also confirmed that the tree's delete confirmation is canceled.

**Impact**
The design's restriction of not showing Entry operations on narrow screens cannot be kept.

**Handling**
Fixed in `4397b11` within the same review. Canceling unsent operations is recorded in §3.8 and §5.1 of client.md.

<a id="r-2"></a>
### R-2. A Selected Off-Screen Item Cannot Be Edited with F2

**Type: Defect** / **Confidence: High**

**Location**
[Grid.tsx](../../ui-web/src/components/Grid.tsx#L58), [List.tsx](../../ui-web/src/components/List.tsx#L50), and [useRevealRename.ts](../../ui-web/src/library/useRevealRename.ts#L7).

**What is wrong**
In the initial target, F2 only set the editing state, and an input could not be shown for items outside the rendered range.

**Basis**
Only items rendered by virtualization draw an input, and there was no processing to scroll to the target. After the fix, we prepared 400 files, selected the first item, scrolled to the end, and pressed F2 with the target not in the DOM. In both the grid and the list, it returned to the top and focus was placed on the target name's input. The 400-item integration regression test also passed.

**Impact**
If you scroll the listing while keeping the selection, you cannot start renaming from the keyboard or the toolbar.

**Handling**
Fixed in `4397b11` within the same review. Reading the target's position, showing it, and editing is recorded in §3.8 and §5.1 of client.md.

## 5. What Was Checked and Found Sound

| Condition or suspicion | Check and result |
| --- | --- |
| Is a prohibition not overlooked with the readonly attribute alone | Entries of the normal and readonly Libraries and `GET /folders` of a normal folder have `metadata.readonly=false`. The Libraries themselves are false and true respectively. Entries inside an archive and `GET /folders` of the archive's root and inside are true. A rejected creation is 403 `Forbidden`. We confirmed the determination that uses the logical OR of the Library and the metadata. |
| 1. Whether the menu and toolbar allow operations | Checked operations on tiles, the list, empty space, and the toolbar. Changes are disabled for readonly and inside archives. At a width of 500px, both the menu and the operation toolbar have 0 items. |
| 2. Creating a folder from a placeholder item | Creating `/created` is 201, and the body is `createParents=false`. Focus stays on the input, and input worked in an empty grid and list too. No request on Esc or an invalid name. A duplicate name is 409 and keeps the toast and the input. Creating on blur is also 201. |
| 3. Renaming in place | The initial selection of `a.txt` is 0–1, and of the folder `docs` is 0–4. `files/move` with `dstPath=/renamed.txt` is 200. No request for empty, `/`, `.`, `..`, or the same name. On 409 the input remains, and closing the menu keeps focus. `folders/move` on blur is also 200. |
| 4. Delete confirmation and mixed selections | Shows 2 items, that it cannot be undone, and deletion with contents. Before confirming, 0 requests. Confirming sends `files/delete` and `folders/delete` (`recursive=true`), the items disappear from the listing, and the selection is 0. No request on cancel after `Meta+Backspace`. Retaining multiple Operations and partial failure was also checked in tests. |
| 5. Start, completion, and cancel of Operations | The actual deletion of `/tree`, which includes 400 files and more, is 202, disappears from the listing and tree on completion, and the panel shows 403 processed. The slow state, with the response replaced, shows `40 / 400` and running, and canceling sent cancel once and switched to "取り消した" (canceled). |
| 6. Reload, update interval, failure, collapse | With replaced responses, we confirmed restoring after reload, an interval of about 1 second at steady state, and 3 updates in 3.1 seconds. No extra requests during the 3.1 seconds after the terminal state. Shows `/x: denied` and 2 skipped, and collapsing leaves 1 status bar button. |
| 7. Downloading a single file | `href` is the content API's `path=%2Fpack.zip`, and `download` is `pack.zip`. The actual saved name is also `pack.zip`. Disabled for folders, multiple, and empty selections. |
| 8. Cache after changes | Without reloading the whole page, we went back and forth with `docs` and the breadcrumbs and confirmed that creations and renames remain in the listing and tree. The store's tests confirmed that it does not touch other Libraries and that it keeps the stale displayed page while switching to the new snapshot. |
| 9. Tree targets and API | We expanded the closed `tree`, created `/tree/made`, and confirmed renaming to `made2` and deleting. The right-click target is independent of the listing's selection. The root allows only creation. Changing the archive itself is 200 with `files/move`, and creating inside it is disabled. |
| 10. Keys during input and dialogs | Pressing F2, Delete, and Alt+Shift+N in the search box produced 0 operation requests and edit fields. Editing does not start during a delete confirmation either. We confirmed that creation starts with `code=KeyN` even when `key` is not N. |
| Switching screen width and an off-screen selection | In a 400-file listing, pressing F2 with the target not in the DOM, we confirmed input and focus in both grid and list. Narrowing the width removes the input and the delete confirmation, and they do not come back when restored, with 0 POSTs. |
| Regression in machine checks | `npm run typecheck`, `npm run lint`, and `npm test` of `ui-web` succeeded. Lint had 0 errors and the same 9 existing warnings. 41 files and 270 tests passed. `RUSTC_WRAPPER= cargo make lint` of `daemon` also succeeded. |

The 404 for empty folders and archives without a representative thumbnail, the 400 for the invalid-as-an-image verification `b.png`, and the 409 for conflict confirmation remained in the console/network. The operations under check proceeded to the expected state, and we also confirmed on screen that the fallback icon does not break the layout.
A check equivalent to Playwright's `requests` was done by observing requests and responses with `network` and `run-code`, and physical keys were checked with a KeyboardEvent of `code: "KeyN"`.
