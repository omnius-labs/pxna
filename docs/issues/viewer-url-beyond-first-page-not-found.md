# Opening the Viewer by URL on an Entry Beyond the First Page Shows "Not Found"

**Severity: medium** (visible today)

Investigated 2026-11-04 / Target commit `a7fff39`

One item of [issues.md](../issues.md).

## Symptom

When a URL with `view` is opened directly (reload, a pasted link, or the browser's back and forward operations) and the Entry it names is not among the first 200 Entries of the listing in the current sort order, the viewer opens with "Not found." and shows no image. Closing the viewer and opening the Entry from the grid works.
We reproduced it in Chromium 156 against a daemon built from this tree and against a build of `main`: in a folder of 700 images, `?view=p-0100.png` opens and `?view=p-0250.png` and `?view=p-0300.png` show "Not found.". Opening `p-0446.png` by double-clicking its tile in the scrolled grid worked, and reloading on the resulting URL showed "Not found.". The browser requested only `offset=0` for the viewer's URL, and never `offset=200`. A unit test over the viewer with 300 Entries, whose mock records the requested offsets, showed the same: `[0]` and "Not found.".
Search results are served by the same listing store class, so the same path applies to a `/search` URL; we did not reproduce it in a browser. The timeline has its own `locate` and is not affected.

## Location

- [viewer.ts#L19](../../ui-web/src/viewer/viewer.ts#L19) — `findEntry` takes the `listing.locate` branch for any listing that has `locate`, loads no page but the first, and returns from there
- [viewer.ts#L26](../../ui-web/src/viewer/viewer.ts#L26) — the page-by-page search that reads pages until the Entry is found, which that return makes unreachable for a listing with `locate`
- [listing.ts#L170](../../ui-web/src/library/listing.ts#L170) — the folder and search store defines `locate`, which answers only from `pathPages`, the paths of pages already loaded ([listing.ts#L275](../../ui-web/src/library/listing.ts#L275))
- [timeline.ts#L182](../../ui-web/src/library/timeline.ts#L182) — the timeline's `locate`, for which the branch was written
- [Viewer.tsx#L223](../../ui-web/src/components/Viewer.tsx#L223) — the call, and [Viewer.tsx#L536](../../ui-web/src/components/Viewer.tsx#L536) — where a `done` search with no Entry becomes "Not found."
- [viewer.test.ts#L82](../../ui-web/src/viewer/viewer.test.ts#L82) — the tests of `findEntry`, which use a listing without `locate`

## Cause

The comment on the branch says a listing that searches a loaded range does not read pages from the start, which describes the timeline ([timeline.ts#L182](../../ui-web/src/library/timeline.ts#L182)) (this is a reading of the intent; the code states no other reason).
The folder and search store also defines `locate`, which finds an Entry's position among the pages it holds, and `findEntry` tests only for its presence. So for a folder, `findEntry` can find an Entry only on a page that is already loaded, and returns nothing for any other Entry without asking for its page.
The first page is found because the effect that calls `findEntry` restarts when `listing.snapshot` changes, since `identity` includes it ([Viewer.tsx#L177](../../ui-web/src/components/Viewer.tsx#L177)), which happens once the first page has arrived (this is a reading of the code; we did not trace the restart). An Entry opened from the grid is found because its page is already in the store.
The tests cover only the page-by-page branch, because their listing has no `locate`.

## Impact

There is no data loss. Entries on the first 200 positions, and Entries opened from the grid, work. We did not exercise the arrow keys and the filmstrip against this path; they move to an Entry whose position is known ([Viewer.tsx#L187](../../ui-web/src/components/Viewer.tsx#L187)).
A viewer URL of a later Entry cannot be reopened, so a reload, a shared link, or a back operation into the viewer dead-ends on "Not found." for any folder or search result longer than 200 Entries.
It contradicts [Client Design §3.4](../design/client.md#34-viewer), which says the same image appears when a URL is opened directly with the viewer open, and the description of reading pages in order until the Entry is found in [Client Design §6.1](../design/client.md#61-current-code).

## Remedy

Let `findEntry` use `locate` only for the range a listing has already read, and fall back to the page-by-page search when it finds nothing and the listing does not read a range around a position (for example, when it has no `loadedFrom`, which only the timeline sets).
Add a test with a listing that has both `locate` over loaded pages and unread pages, and a viewer test that opens an Entry on the second page and checks that `offset=200` is requested.
