// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, createElement, StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import type { FolderListing } from "./entries";
import {
    folderPageKey,
    type Listing,
    ListingSnapshotChanged,
    type ListingSource,
    refreshLibrary,
    type SearchCriteria,
    searchPageKey,
    staleSnapshotPages,
    useListing,
} from "./listing";

const { get } = vi.hoisted(() => ({ get: vi.fn() }));
vi.mock("../api/client", async (original) => ({
    ...(await original<typeof import("../api/client")>()),
    api: {
        GET: (route: string, options?: unknown) =>
            route === "/capabilities"
                ? Promise.resolve({
                      data: { summaryListing: false, entryDetails: false, childProbes: false },
                      response: new Response(),
                  })
                : get(route, options),
    },
}));
afterEach(() => get.mockReset());
const page = (snapshot = "one", count = 1000): FolderListing => ({
    entries: [{ name: "a", path: "/a", kind: "folder", metadata: { size: 1, readonly: false } }],
    totalCount: count,
    snapshot,
    truncated: false,
});
const response = (data: FolderListing) => ({ data, response: new Response() });

const folderSource: ListingSource = { kind: "folder", libraryId: "big", path: "/" };
async function harness(wanted: number[] = [], strict = false, source: ListingSource = folderSource) {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    const client = new QueryClient();
    const root = createRoot(document.createElement("div"));
    const current: { listing?: Listing } = {};
    function Harness({ wanted, source }: { wanted: number[]; source: ListingSource }) {
        current.listing = useListing(source, { key: "name", order: "asc" }, new Set(wanted));
        return null;
    }
    const render = async (wanted: number[], next: ListingSource = source) => {
        await act(async () =>
            root.render(
                createElement(
                    QueryClientProvider,
                    { client },
                    strict
                        ? createElement(StrictMode, null, createElement(Harness, { wanted, source: next }))
                        : createElement(Harness, { wanted, source: next }),
                ),
            ),
        );
    };
    await render(wanted);
    return {
        client,
        current,
        render,
        dispose: async () => {
            await act(async () => root.unmount());
            client.clear();
        },
    };
}
async function wait(assertion: () => void) {
    await vi.waitFor(async () => {
        await act(async () => {
            await Promise.resolve();
        });
        assertion();
    });
}

it("requests only zero and wanted pages, with offsets from page indexes", async () => {
    get.mockResolvedValue(response(page()));
    const h = await harness([4]);
    try {
        await wait(() => expect(h.current.listing?.pageStatus(4)).toBe("loaded"));
        expect(get.mock.calls.map(([, options]) => options.params.query.offset)).toEqual([800, 0]);
        expect(h.current.listing?.entryAt(800)?.path).toBe("/a");
        expect(h.current.listing?.entryAt(200)).toBeUndefined();
        expect(h.current.listing?.entryAt(1000)).toBeUndefined();
    } finally {
        await h.dispose();
    }
});
const searchSource = (patch: Partial<SearchCriteria> = {}): ListingSource => ({
    kind: "search",
    criteria: { libraryIds: ["a", "b"], path: "/", recursive: true, query: "", tags: [], ...patch },
});
const searchResponse = (snapshot = "one") => ({
    data: {
        entries: [{ libraryId: "b", entry: { name: "x.png", path: "/dir/x.png", kind: "file" } }],
        totalCount: 1,
        snapshot,
        truncatedLibraryIds: ["a"],
    },
    response: new Response(),
});
it("sends a search to the cross-Library API with every condition, and keys paths by Library", async () => {
    get.mockResolvedValue(searchResponse());
    const h = await harness([0], false, searchSource({ query: " cat ", rating: "good", tags: ["trip", "sea"] }));
    try {
        await wait(() => expect(h.current.listing?.pageStatus(0)).toBe("loaded"));
        const [route, options] = get.mock.calls[0]!;
        expect(route).toBe("/search/entries");
        expect(options.params.query).toMatchObject({
            libraryIds: ["a", "b"],
            recursive: true,
            query: "cat",
            rating: "good",
            tag: ["trip", "sea"],
            offset: 0,
        });
        expect(options.params.query).not.toHaveProperty("path");
        expect(h.current.listing?.entryAt(0)?.path).toBe("/b/dir/x.png");
        expect(h.current.listing?.truncated).toBe(true);
        expect(h.current.listing?.totalCount).toBe(1);
    } finally {
        await h.dispose();
    }
});
it("sends the start only when it is not the Library root", async () => {
    get.mockResolvedValue(searchResponse());
    const h = await harness([0], false, searchSource({ libraryIds: ["a"], path: "/photos", query: "x" }));
    try {
        await wait(() => expect(h.current.listing?.pageStatus(0)).toBe("loaded"));
        expect(get.mock.calls[0]![1].params.query).toMatchObject({ libraryIds: ["a"], path: "/photos" });
    } finally {
        await h.dispose();
    }
});
it("keys a page by every search condition so different searches never mix", () => {
    const sort = { key: "name", order: "asc" } as const;
    const base = { libraryIds: ["a"], path: "/", recursive: true, query: "", tags: [] as string[] };
    const keys = [
        searchPageKey(base, sort, 0),
        searchPageKey({ ...base, libraryIds: ["a", "b"] }, sort, 0),
        searchPageKey({ ...base, path: "/x" }, sort, 0),
        searchPageKey({ ...base, recursive: false }, sort, 0),
        searchPageKey({ ...base, query: "q" }, sort, 0),
        searchPageKey({ ...base, rating: "good" }, sort, 0),
        searchPageKey({ ...base, rating: "bad" }, sort, 0),
        searchPageKey({ ...base, tags: ["x"] }, sort, 0),
        searchPageKey({ ...base, tags: ["x", "y"] }, sort, 0),
        folderPageKey("a", "/", sort, 0),
    ].map((key) => JSON.stringify(key));
    expect(new Set(keys).size).toBe(keys.length);
});
it("rereads a search that covers a refreshed Library", async () => {
    get.mockResolvedValue(searchResponse("old"));
    const search = await harness([0], false, searchSource());
    try {
        await wait(() => expect(search.current.listing?.snapshot).toBe("old"));
        get.mockResolvedValue(searchResponse("new"));
        await act(async () => refreshLibrary(search.client, "b"));
        await wait(() => expect(search.current.listing?.snapshot).toBe("new"));
    } finally {
        await search.dispose();
    }
});
it("cancels an unobserved request", async () => {
    get.mockImplementation((_, options) =>
        options.params.query.offset === 0 ? Promise.resolve(response(page())) : new Promise(() => {}),
    );
    const h = await harness([3]);
    try {
        const signal = get.mock.calls.find(([, options]) => options.params.query.offset === 600)?.[1]
            .signal as AbortSignal;
        await h.render([]);
        expect(signal.aborted).toBe(true);
    } finally {
        await h.dispose();
    }
});
it("finds pages of another snapshot", () => {
    expect(
        staleSnapshotPages(
            new Map([
                [0, page("old")],
                [4, page("new")],
            ]),
            "new",
        ),
    ).toEqual([0]);
});
it("discards old pages, updates the count and rereads wanted pages", async () => {
    get.mockResolvedValue(response(page("old")));
    const h = await harness();
    try {
        await wait(() => expect(h.current.listing?.snapshot).toBe("old"));
        get.mockResolvedValue(response(page("new", 1001)));
        await h.render([4]);
        await wait(() => expect(h.current.listing?.pageStatus(0)).toBe("loaded"));
        await wait(() => expect(h.current.listing?.snapshot).toBe("new"));
        expect(h.current.listing?.totalCount).toBe(1001);
        expect(get.mock.calls.map(([, options]) => options.params.query.offset)).toEqual([0, 800, 0]);
    } finally {
        await h.dispose();
    }
});
it("does not automatically retry a failed page on remount, and retries explicitly", async () => {
    get.mockRejectedValue(new Error("failed"));
    const h = await harness();
    try {
        await wait(() => expect(h.current.listing?.pageStatus(0)).toBe("error"));
        await h.render([4]);
        await wait(() => expect(h.current.listing?.pageStatus(4)).toBe("error"));
        expect(get.mock.calls.filter(([, options]) => options.params.query.offset === 0)).toHaveLength(1);
        get.mockResolvedValue(response(page()));
        await act(async () => h.current.listing?.retry(0));
        await wait(() => expect(h.current.listing?.pageStatus(0)).toBe("loaded"));
    } finally {
        await h.dispose();
    }
});
it("resolves explicit loads after reading the page", async () => {
    get.mockResolvedValue(response(page()));
    const h = await harness();
    try {
        await act(async () => {
            await h.current.listing!.loadPages([2], new AbortController().signal);
        });
        expect(h.current.listing?.pageStatus(2)).toBe("loaded");
    } finally {
        await h.dispose();
    }
});
it("rejects explicit loads when the snapshot changes", async () => {
    get.mockResolvedValue(response(page("old")));
    const h = await harness();
    try {
        await wait(() => expect(h.current.listing?.snapshot).toBe("old"));
        get.mockResolvedValue(response(page("new")));
        await act(async () => {
            await expect(h.current.listing!.loadPages([2], new AbortController().signal)).rejects.toBeInstanceOf(
                ListingSnapshotChanged,
            );
        });
    } finally {
        await h.dispose();
    }
});
it("aborts explicit loads and cancels their request", async () => {
    get.mockImplementation((_, options) =>
        options.params.query.offset === 0 ? Promise.resolve(response(page())) : new Promise(() => {}),
    );
    const h = await harness();
    try {
        const controller = new AbortController();
        await act(async () => {
            const loading = h.current.listing!.loadPages([2], controller.signal);
            const rejected = expect(loading).rejects.toMatchObject({ name: "AbortError" });
            controller.abort();
            await rejected;
        });
        // capabilities の共有結果を待つ前に取消済みなら、ページ自体を送らない。
        const requests = get.mock.calls.filter(([, options]) => options.params.query.offset === 400);
        expect(requests.every(([, options]) => options.signal.aborted)).toBe(true);
    } finally {
        await h.dispose();
    }
});

it("starts only one request during StrictMode setup", async () => {
    get.mockResolvedValue(response(page()));
    const h = await harness([], true);
    try {
        await wait(() => expect(h.current.listing?.pageStatus(0)).toBe("loaded"));
        expect(get).toHaveBeenCalledTimes(1);
    } finally {
        await h.dispose();
    }
});

it("フォルダの一覧には検索の条件を付けない", async () => {
    get.mockResolvedValue(response(page()));
    const h = await harness();
    try {
        await wait(() => expect(h.current.listing?.pageStatus(0)).toBe("loaded"));
        expect(get.mock.calls[0]![1].params.query).toEqual({
            path: "/",
            limit: 200,
            offset: 0,
            foldersFirst: true,
            sort: "name",
            order: "asc",
        });
    } finally {
        await h.dispose();
    }
});
it("検索語を整え、打ち切りを Library の一覧から知る", async () => {
    get.mockResolvedValue(searchResponse());
    const h = await harness([], false, searchSource({ query: " alpha " }));
    try {
        await wait(() => expect(h.current.listing?.pageStatus(0)).toBe("loaded"));
        expect(get.mock.calls[0]![1].params.query).toMatchObject({ query: "alpha", recursive: true });
        expect(h.current.listing?.truncated).toBe(true);
        get.mockResolvedValue({
            ...searchResponse(),
            data: { ...searchResponse().data, truncatedLibraryIds: [] },
        });
        await h.render([2], searchSource({ query: "alpha" }));
        await wait(() => expect(h.current.listing?.pageStatus(2)).toBe("loaded"));
        expect(h.current.listing?.truncated).toBe(false);
    } finally {
        await h.dispose();
    }
});
it("語を変えると未完了の要求を止め、前の語のページを混ぜない", async () => {
    let finish: ((value: ReturnType<typeof searchResponse>) => void) | undefined;
    const resultFor = (query: string) => {
        const answer = searchResponse();
        return {
            ...answer,
            data: {
                ...answer.data,
                entries: [{ libraryId: "a", entry: { ...answer.data.entries[0]!.entry, path: `/${query}` } }],
            },
        };
    };
    get.mockImplementation((_, options) =>
        options.params.query.query === "old" && options.params.query.offset === 200
            ? new Promise((done) => {
                  finish = done;
              })
            : Promise.resolve(resultFor(options.params.query.query)),
    );
    const h = await harness([1], false, searchSource({ query: "old", recursive: false }));
    try {
        await wait(() => expect(h.current.listing?.entryAt(0)?.path).toBe("/a/old"));
        const previous = h.current.listing;
        const signal = get.mock.calls.find(([, options]) => options.params.query.offset === 200)![1]
            .signal as AbortSignal;
        await h.render([], searchSource({ query: "new", recursive: false }));
        await wait(() => expect(h.current.listing?.entryAt(0)?.path).toBe("/a/new"));
        expect(signal.aborted).toBe(true);
        expect(h.current.listing).not.toBe(previous);
        await act(async () => finish!(resultFor("old")));
        expect(h.current.listing?.entryAt(200)).toBeUndefined();
        expect(h.current.listing?.entryAt(0)?.path).toBe("/a/new");
    } finally {
        await h.dispose();
    }
});
it("再帰の条件でもストアとページを分ける", async () => {
    get.mockResolvedValue(searchResponse());
    const h = await harness([], false, searchSource({ query: "alpha", recursive: false }));
    try {
        await wait(() => expect(h.current.listing?.pageStatus(0)).toBe("loaded"));
        const previous = h.current.listing;
        await h.render([], searchSource({ query: "alpha", recursive: true }));
        await wait(() => expect(h.current.listing?.pageStatus(0)).toBe("loaded"));
        expect(h.current.listing).not.toBe(previous);
        expect(get.mock.calls.map(([, options]) => options.params.query.recursive)).toEqual([false, true]);
    } finally {
        await h.dispose();
    }
});

it("変更した Library だけを、古い内容を残して読み直し、未表示のページを捨てる", async () => {
    get.mockResolvedValue(response(page("old")));
    const h = await harness();
    try {
        await wait(() => expect(h.current.listing?.snapshot).toBe("old"));
        const other = folderPageKey("other", "/", { key: "name", order: "asc" }, 0);
        const inactive = folderPageKey("big", "/away", { key: "name", order: "asc" }, 0);
        h.client.setQueryData(other, page());
        h.client.setQueryData(inactive, page());
        let finish!: (value: ReturnType<typeof response>) => void;
        get.mockImplementation(
            () =>
                new Promise((resolve) => {
                    finish = resolve;
                }),
        );
        let refreshing!: Promise<void>;
        await act(async () => {
            refreshing = refreshLibrary(h.client, "big");
        });
        expect(h.current.listing?.entryAt(0)?.path).toBe("/a");
        expect(h.current.listing?.snapshot).toBe("old");
        expect(h.client.getQueryData(inactive)).toBeUndefined();
        expect(h.client.getQueryData(other)).toEqual(page());
        await act(async () => {
            finish(response({ ...page("new"), entries: [{ ...page().entries[0]!, path: "/changed" }] }));
            await refreshing;
        });
        await wait(() => expect(h.current.listing?.entryAt(0)?.path).toBe("/changed"));
    } finally {
        await h.dispose();
    }
});
