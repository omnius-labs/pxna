# Restore the Last Opened Folder at `/`

## 1. About This Plan

This work makes the web client resume the folder it last showed, instead of an empty screen, when the user visits `/`.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The steps and acceptance checks for restoring the last folder at `/` |
| [Client Design §3.1](../design/client.md#31-web) | The `/` behaviour, the saved location, the switch between folder view and the other modes |
| [design.md §9.2](../design.md#92-where-state-lives) | Which state lives where; this work adds only browser-local state |

### 1.2 Creation Date and Target Commit

Created 2026-10-25 / Target commit `4f071e1`

## 2. Purpose and Completion Conditions

Opening the web client at `/` returns the user to the last folder they were viewing, without asking them to choose a Library again.

### 2.1 Completion Conditions

1. Visiting `/` with a saved folder redirects to `/browse/{libraryId}/{path}` with `replace`
2. With no saved value, `/` redirects to the root of the first Library that `GET /libraries` returns
3. A saved Library id that the Library list does not contain redirects to the root of the first Library
4. A saved path other than `/` that `GET /libraries/{libraryId}/folders` answers 404 for redirects to that Library's root, and any other failure redirects to the saved path
5. With no Libraries, or when the Library list request fails, `/` shows the current message to choose a Library
6. Opening a folder view saves its Library id and path, without `view` or `reveal`, under `pxna.lastFolder`
7. The switch's folder link uses the same saved location, including in a tab opened after the save
8. A malformed, unknown-shaped, or unreadable stored value is treated as no saved value

### 2.2 Non-goals

- Persisting search conditions or the timeline's month and Libraries
- Remembering a separate folder for each Library
- Changing the URLs of folder view, search mode, or the timeline
- Storing the location in the server-side display preferences

## 3. Current

[HomePage.tsx](../../ui-web/src/pages/HomePage.tsx) shows "左のツリーから Library を選んでください。" at `/`, and [router.tsx](../../ui-web/src/router.tsx) has no redirect for that route.
[folderReturn.ts](../../ui-web/src/library/folderReturn.ts) remembers the last folder only in memory for the lifetime of the tab, and [ModeSwitch.tsx](../../ui-web/src/components/ModeSwitch.tsx) falls back to the first Library's root when it has nothing.
A new tab or a reload therefore loses the last folder and shows the empty `/` screen.

## 4. Change Policy

### 4.1 Decisions

| Decision | Source |
| --- | --- |
| `/` redirects with `replace` to the saved folder, or to the first Library's root, and stays on the page only when there is no Library | [Client Design §3.1](../design/client.md#31-web) |
| The last folder is saved per device in `localStorage` under `pxna.lastFolder`, as a JSON object with `libraryId` and `path` | [Client Design §3.1](../design/client.md#31-web) |
| A missing Library or a 404 folder falls back to the Library's root; other failures keep the saved path and leave recovery to the folder view | [Client Design §3.1](../design/client.md#31-web) |

### 4.2 Impact

Only the web client changes. There is no API change and no change to server-side stored formats.
`pxna.lastFolder` is a new browser-local key; older clients ignore it, and clearing it opens the first Library's root.

## 5. Steps

| No | Content | Dependency | State |
| --- | --- | --- | --- |
| [1](#s-1) | Record the `/` behaviour and saved location in Client Design | - | Done |
| [2](#s-2) | Persist and read the last folder in `folderReturn.ts` | 1 | Done |
| [3](#s-3) | Restore the last folder from `/` in the router | 2 | Done |
| [4](#s-4) | Run tests, typecheck, and lint | 3 | Done |

<a id="s-1"></a>
### Step 1. Record the `/` behaviour and saved location in Client Design

**Where to change**
[Client Design §3.1](../design/client.md#31-web) and the switch reference in the same document.

**What to do**
Replace the sentence about `/` showing only the tree with the redirect rule, its fallbacks, and the `localStorage` key.
Update the sentence that says the folder switch returns to a folder remembered only while the tab is open.

**Done when**
Client Design states where the location is saved, when it is updated, and which fallbacks apply.

<a id="s-2"></a>
### Step 2. Persist and read the last folder in `folderReturn.ts`

**Where to change**
[ui-web/src/library/folderReturn.ts](../../ui-web/src/library/folderReturn.ts) and a new `ui-web/src/library/folderReturn.test.ts`.

**What to do**
Replace the module-level variable with a `localStorage` key holding `{ libraryId, path }`.
Treat a missing, malformed, `.`/`..` segment, or unreadable value as no saved value, and drop empty path segments.
Keep `rememberFolder` and `lastFolderLocation` as the interface that folder view and the switch use.

**Done when**
The new tests cover save, read, malformed values, and a storage that throws, and the existing switch tests pass.

<a id="s-3"></a>
### Step 3. Restore the last folder from `/` in the router

**Where to change**
[ui-web/src/router.tsx](../../ui-web/src/router.tsx) and `ui-web/src/router.test.ts`.

**What to do**
In the home route's `beforeLoad`, read the saved folder and the Library list.
If there is no saved folder, use the root of the first Library.
If the saved Library is absent, use the first Library's root.
If the saved path is not `/`, check the folder once with the query client; this adds one request only on `/`. A 404 falls back to the Library's root; any other failure proceeds to the saved path.
When the Library list cannot be read, do not redirect and let the current page render.
Serve the Library list and folder requests from a mocked API in `router.test.ts`, and add tests for the redirect, both fallbacks, the 404 case, the 500 and network-failure cases, the no-Library case, and a malformed stored value.

**Done when**
`src/router.test.ts` covers completion conditions 1 to 5 and 8 at the route level.

<a id="s-4"></a>
### Step 4. Run tests, typecheck, and lint

**Where to change**
No files.

**What to do**
Run the web client's tests, typecheck, and lint.

**Done when**
All three pass.

## 6. Acceptance Verification

| Condition | Command | Expected result |
| --- | --- | --- |
| 1, 2, 3, 4, 5, 8 | `cd ui-web && npx vitest run src/router.test.ts src/library/folderReturn.test.ts` | All tests pass |
| 6, 7 | `cd ui-web && npx vitest run src/components/ModeSwitch.test.tsx src/library/folderReturn.test.ts` | All tests pass |
| 1 to 8 | `cd ui-web && npm run test` | All tests pass except the pre-existing 5 s timeout in `src/library/timeline.test.ts`, which also fails at the target commit and is not touched by this work |
| All | `cd ui-web && npm run typecheck` | No type errors |
| All | `cd ui-web && npm run lint` | Exit 0, with only the existing warnings |

The new tests guard the saved location's shape, the `/` redirect and its fallbacks, and the switch reading the saved location from `localStorage`.
