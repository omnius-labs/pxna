// @vitest-environment jsdom
import { QueryClient, QueryObserver } from "@tanstack/react-query";
import { afterEach, describe, expect, it, vi } from "vitest";
import { childFoldersQuery, type Entry, entryPlace, type FolderListing } from "./entries";
import { refreshLibrary } from "./listing";

const { get } = vi.hoisted(() => ({ get: vi.fn() }));
vi.mock("../api/client", async (importOriginal) => {
    const actual = await importOriginal<typeof import("../api/client")>();
    return { ...actual, api: { GET: get } };
});

function entry(name: string): Entry {
    return { path: `/${name}`, name, kind: "folder", metadata: { size: 0, readonly: false } };
}

function mockPages(pages: FolderListing[]) {
    for (const data of pages) get.mockResolvedValueOnce({ data, response: new Response() });
    return get;
}

afterEach(() => get.mockReset());

describe("tree listing pages", () => {
    it("reads every tree page with the accumulated offset", async () => {
        const get = mockPages([
            { entries: [entry("a")], totalCount: 3, snapshot: "one", truncated: false },
            { entries: [entry("b"), entry("c")], totalCount: 3, snapshot: "one", truncated: false },
        ]);
        const client = new QueryClient();
        const result = await client.fetchQuery(childFoldersQuery("photos", "/"));
        expect(result.map((entry) => entry.name)).toEqual(["a", "b", "c"]);
        expect(get.mock.calls.map(([, options]) => options?.params?.query?.offset)).toEqual([0, 1]);
        client.clear();
    });

    it("restarts tree traversal when a later page has a different snapshot", async () => {
        const get = mockPages([
            { entries: [entry("old-a")], totalCount: 2, snapshot: "old", truncated: false },
            { entries: [entry("new-b")], totalCount: 2, snapshot: "new", truncated: false },
            { entries: [entry("new-a")], totalCount: 2, snapshot: "new", truncated: false },
            { entries: [entry("new-b")], totalCount: 2, snapshot: "new", truncated: false },
        ]);
        const client = new QueryClient();
        const result = await client.fetchQuery(childFoldersQuery("photos", "/"));
        expect(result.map((entry) => entry.name)).toEqual(["new-a", "new-b"]);
        expect(get.mock.calls.map(([, options]) => options?.params?.query?.offset)).toEqual([0, 1, 0, 1]);
        client.clear();
    });
});

it.each([
    ["/a", "/a/f.png", "."],
    ["/a", "/a/b/c/f.png", "b/c"],
    ["/", "/a/f.png", "a"],
    ["/", "/f.png", "."],
])("起点から親フォルダの場所を求める %s %s", (base, path, expected) => {
    expect(entryPlace(base, { ...entry("f.png"), path })).toBe(expected);
});

it("変更後は表示中のツリーだけを再取得する", async () => {
    const client = new QueryClient();
    const options = childFoldersQuery("photos", "/");
    get.mockResolvedValue({
        data: { entries: [entry("old")], totalCount: 1, snapshot: "one", truncated: false },
        response: new Response(),
    });
    const observer = new QueryObserver(client, options);
    const stop = observer.subscribe(() => {});
    try {
        await observer.refetch();
        client.setQueryData(childFoldersQuery("other", "/").queryKey, [entry("other")]);
        get.mockResolvedValue({
            data: { entries: [entry("new")], totalCount: 1, snapshot: "two", truncated: false },
            response: new Response(),
        });
        await refreshLibrary(client, "photos");
        expect(client.getQueryData(options.queryKey)?.[0]?.name).toBe("new");
        expect(client.getQueryData(childFoldersQuery("other", "/").queryKey)).toEqual([entry("other")]);
    } finally {
        stop();
        observer.destroy();
        client.clear();
    }
});
