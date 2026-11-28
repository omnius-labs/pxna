// The requests that the listings send to `listEntries`, checked against the contract without the bridge the screens' tests answer through.
import { QueryClient } from "@tanstack/react-query";
import { afterEach, expect, it, vi } from "vitest";
import { fetchEntryDetails, fetchFolderPage, fetchListingPage, fetchSearchPage } from "./entries";
import { timelineMonthsQuery } from "./timeline";

const { get, post } = vi.hoisted(() => ({ get: vi.fn(), post: vi.fn() }));
vi.mock("../api/client", async (original) => ({
    ...(await original<typeof import("../api/client")>()),
    api: { GET: get, POST: post },
}));

const ok = (data: unknown) => ({ data, response: new Response(null, { status: 200 }) });
const page = (items: unknown[], extra: Record<string, unknown> = {}) =>
    ok({
        items,
        page: { kind: "offset", offset: 0, totalCount: items.length, snapshot: "s" },
        warnings: { cutOffStarts: [], walkedStarts: [], missingStarts: [] },
        ...extra,
    });
const file = { path: "/photos/a.png", name: "a.png", kind: "file", metadata: { size: 1, readonly: false } };

afterEach(() => {
    get.mockReset();
    post.mockReset();
});

it("lists one folder as one start, naming every kind when it reads descendants", async () => {
    post.mockResolvedValue(page([{ libraryId: "lib", entry: file }]));
    const listing = await fetchFolderPage("lib", "/photos", 200, { recursive: true, query: " a ", foldersFirst: true });
    expect(post).toHaveBeenCalledWith("/entries", expect.anything());
    expect(post.mock.calls[0]?.[1].body).toEqual({
        scope: { starts: [{ libraryId: "lib", path: "/photos" }], recursive: true },
        filter: { query: "a", kinds: ["folder", "archive", "image", "video", "other"] },
        options: { foldersFirst: true, includeHidden: false },
        page: { kind: "offset", offset: 200, limit: 200 },
    });
    expect(listing).toMatchObject({ entries: [file], totalCount: 1, snapshot: "s", truncated: false });
});

it("locates an Entry from the Library root and treats a missing start as a missing folder", async () => {
    post.mockResolvedValue(
        page([], {
            warnings: { cutOffStarts: [], walkedStarts: [], missingStarts: [{ libraryId: "lib", path: "/x" }] },
        }),
    );
    await expect(fetchFolderPage("lib", "/x", { locate: "sub/a.png" }, { foldersOnly: true })).rejects.toMatchObject({
        status: 404,
    });
    expect(post.mock.calls[0]?.[1].body).toMatchObject({
        filter: { kinds: ["folder", "archive"] },
        page: { kind: "offset", locate: { libraryId: "lib", path: "/x/sub/a.png" } },
    });
});

it("searches the starts of the chosen Libraries and tells a walked start from an indexed answer", async () => {
    post.mockResolvedValue(
        page([{ libraryId: "b", entry: file }], {
            warnings: { cutOffStarts: [], walkedStarts: [{ libraryId: "b", path: "/" }], missingStarts: [] },
        }),
    );
    const listing = await fetchSearchPage(
        { libraryIds: ["a", "b"], path: "/", recursive: true, query: "x", rating: "good", tags: ["t"] },
        { locate: "photos/a.png", locateLibraryId: "b" },
        { sort: "name", order: "asc" },
        new AbortController().signal,
    );
    expect(post.mock.calls[0]?.[1].body).toEqual({
        scope: {
            starts: [
                { libraryId: "a", path: "/" },
                { libraryId: "b", path: "/" },
            ],
            recursive: true,
        },
        filter: { query: "x", rating: "good", tags: ["t"], kinds: ["folder", "archive", "image", "video", "other"] },
        options: { foldersFirst: true },
        sort: "name",
        order: "asc",
        page: { kind: "offset", locate: { libraryId: "b", path: "/photos/a.png" }, limit: 200 },
    });
    expect(listing.entries[0]?.path).toBe("/b/photos/a.png");
    expect(listing.indexed).toBe(false);
});

it("reads the summary of direct children as summaries of one start", async () => {
    get.mockResolvedValue(
        ok({ summaryListing: true, entryDetails: true, childProbes: false, performanceLogging: false }),
    );
    post.mockResolvedValue(
        ok({
            summaries: [{ libraryId: "lib", summary: { path: "/z.zip", name: "z.zip", kind: "file", archive: true } }],
            page: { kind: "offset", offset: 0, totalCount: 1, snapshot: "s" },
            warnings: { cutOffStarts: [], walkedStarts: [], missingStarts: [] },
        }),
    );
    const listing = await fetchListingPage(
        new QueryClient(),
        "lib",
        "/",
        0,
        { sort: "kind", order: "desc" },
        new AbortController().signal,
    );
    expect(post.mock.calls[0]?.[1].body).toEqual({
        scope: { starts: [{ libraryId: "lib", path: "/" }] },
        filter: {},
        options: { fields: "summary", foldersFirst: false, includeHidden: false },
        sort: "kind",
        order: "desc",
        page: { kind: "offset", offset: 0, limit: 200 },
    });
    expect(listing).toMatchObject({ summary: true, entries: [{ path: "/z.zip", file: { archive: true } }] });
});

it("reads the metadata of several paths as places of their Library", async () => {
    post.mockResolvedValue(ok({ items: [{ libraryId: "lib", path: "/a", error: "NotFound" }] }));
    const results = await fetchEntryDetails("lib", ["/a"], new AbortController().signal);
    expect(post).toHaveBeenCalledWith(
        "/entries/metadata",
        expect.objectContaining({ body: { places: [{ libraryId: "lib", path: "/a" }] } }),
    );
    expect(results).toEqual([{ libraryId: "lib", path: "/a", error: "NotFound" }]);
});

it("counts the months of the timeline over the roots of its Libraries with the insides of archives", async () => {
    post.mockResolvedValue(ok({ months: [{ month: "2026-01", count: 2 }] }));
    const months = await new QueryClient().fetchQuery(timelineMonthsQuery(["a", "b"]));
    expect(post.mock.calls[0]?.[0]).toBe("/entries/months");
    expect(post.mock.calls[0]?.[1].body).toEqual({
        scope: {
            starts: [
                { libraryId: "a", path: "/" },
                { libraryId: "b", path: "/" },
            ],
            recursive: true,
            insideArchives: true,
        },
        filter: { kinds: ["image", "video"] },
    });
    expect(months).toEqual([{ month: "2026-01", count: 2 }]);
});
