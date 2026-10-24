// @vitest-environment jsdom
import { QueryClient } from "@tanstack/react-query";
import { afterEach, expect, it, vi } from "vitest";
import { ApiError } from "../api/client";
import { type Entry, fetchListingPage } from "./entries";
import { folderListingStore } from "./listing";
import { selectionBytes, selectionItem, selectionValue } from "./selection";

const { get, post } = vi.hoisted(() => ({ get: vi.fn(), post: vi.fn() }));
vi.mock("../api/client", async (original) => ({
    ...(await original<typeof import("../api/client")>()),
    api: { GET: get, POST: post },
}));
const cleanups: (() => void)[] = [];
afterEach(() => {
    for (const stop of cleanups.splice(0)) stop();
    get.mockReset();
    post.mockReset();
});
const response = (data: unknown) => ({ data, response: new Response() });
const caps = { summaryListing: true, entryDetails: true, childProbes: false };
function page(snapshot = "one", offset = 0) {
    return {
        snapshot,
        totalCount: 600,
        truncated: false,
        entries: [{ path: `/f${offset}`, name: `f${offset}`, kind: "file", archive: false }],
    };
}
function full(path = "/f0"): Entry {
    return { path, name: path.slice(1), kind: "file", metadata: { size: 42, readonly: false } };
}
function harness(wanted = [0], tree = false) {
    const client = new QueryClient();
    client.setQueryData(["capabilities"], caps);
    const store = folderListingStore(client, "test", "/", { key: "name", order: "asc" }, tree);
    const owner = {};
    cleanups.push(() => {
        store.release(owner);
        client.clear();
    });
    store.setWanted(owner, new Set(wanted));
    return { client, store, owner };
}

it("名前を先に表示し、詳細の到着で位置とsnapshotを変えない", async () => {
    get.mockResolvedValue(response(page()));
    let resolve!: (value: unknown) => void;
    post.mockImplementation(
        () =>
            new Promise((done) => {
                resolve = done;
            }),
    );
    const { store } = harness();
    await vi.waitFor(() => expect(post).toHaveBeenCalledTimes(1));
    const entry = store.entryAt(0)!;
    expect(entry.name).toBe("f0");
    expect(entry.metadata).toBeUndefined();
    const selected = selectionItem(entry, false);
    expect(selected.readonly).toBe(true);
    expect(selectionBytes({ items: new Map([[entry.path, selectionValue(selected)]]), anchor: 0 })).toBeUndefined();
    resolve(response({ results: [{ path: "/f0", detail: full() }] }));
    await vi.waitFor(() => expect(store.entryAt(0)?.metadata?.size).toBe(42));
    expect(store.snapshot).toBe("one");
    expect(store.locate("/f0")).toBe(0);
    expect(selectionItem(store.entryAt(0)!, false).readonly).toBe(false);
    expect(get.mock.calls.every(([route]) => route.endsWith("/summaries"))).toBe(true);
});

it("詳細の部分失敗を未知の属性として残し、旧一覧へ戻さない", async () => {
    get.mockResolvedValue(response(page()));
    post.mockResolvedValue(response({ results: [{ path: "/f0", error: "NotFound" }] }));
    const { store } = harness();
    await vi.waitFor(() => expect(store.entryAt(0)?.detailError).toBe("NotFound"));
    expect(store.entryAt(0)?.metadata).toBeUndefined();
    expect(get).toHaveBeenCalledTimes(1);
});

it("可視範囲から外れた詳細を取り消し、遅い応答を混ぜない", async () => {
    get.mockImplementation((_, options) => response(page("one", options.params.query.offset)));
    let resolve!: (value: unknown) => void;
    post.mockImplementation(
        () =>
            new Promise((done) => {
                resolve = done;
            }),
    );
    const { store, owner } = harness();
    await vi.waitFor(() => expect(post).toHaveBeenCalledTimes(1));
    const signal = post.mock.calls[0]![1].signal as AbortSignal;
    store.setWanted(owner, new Set([1]));
    expect(signal.aborted).toBe(true);
    resolve(response({ results: [{ path: "/f0", detail: full() }] }));
    await vi.waitFor(() => expect(post).toHaveBeenCalledTimes(2));
    expect(store.entryAt(0)?.metadata).toBeUndefined();
});

it("snapshotを更新した後に古い詳細が到着しても採用しない", async () => {
    get.mockResolvedValue(response(page()));
    let resolve!: (value: unknown) => void;
    post.mockImplementationOnce(
        () =>
            new Promise((done) => {
                resolve = done;
            }),
    ).mockResolvedValue(
        response({ results: [{ path: "/f0", detail: { ...full(), metadata: { size: 99, readonly: true } } }] }),
    );
    const { store } = harness();
    await vi.waitFor(() => expect(post).toHaveBeenCalledTimes(1));
    get.mockResolvedValue(response(page("two")));
    await store.refresh();
    resolve(response({ results: [{ path: "/f0", detail: full() }] }));
    await vi.waitFor(() => expect(store.entryAt(0)?.metadata?.size).toBe(99));
    expect(store.snapshot).toBe("two");
});

it("ツリーは名前だけを読み、メニューなどの対象を指定してから詳細を読む", async () => {
    get.mockResolvedValue(response(page()));
    post.mockResolvedValue(response({ results: [{ path: "/f0", detail: full() }] }));
    const { store } = harness([0], true);
    await vi.waitFor(() => expect(store.pageStatus(0)).toBe("loaded"));
    expect(post).not.toHaveBeenCalled();
    const owner = {};
    store.setDetailPriority(owner, ["/f0"]);
    cleanups.push(() => store.releaseDetailPriority(owner));
    await vi.waitFor(() => expect(store.entryAt(0)?.metadata?.size).toBe(42));
});

it("種別が変わった詳細では並びを直接変えず、一覧を更新する", async () => {
    get.mockResolvedValueOnce(response(page())).mockResolvedValue(
        response({ ...page("two"), entries: [{ path: "/f0", name: "f0", kind: "folder", archive: false }] }),
    );
    post.mockResolvedValue(response({ results: [{ path: "/f0", detail: { ...full(), kind: "folder" } }] }));
    const { store } = harness();
    await vi.waitFor(() => expect(store.snapshot).toBe("two"));
    expect(store.entryAt(0)?.kind).toBe("folder");
    expect(get).toHaveBeenCalledTimes(2);
});

it.each([404, 405, 501])("capabilitiesの%dだけを旧daemonとして扱う", async (status) => {
    const client = new QueryClient();
    cleanups.push(() => client.clear());
    get.mockImplementation((route) =>
        route === "/capabilities"
            ? { response: new Response(null, { status }) }
            : response({ ...page(), entries: [full()] }),
    );
    const result = await fetchListingPage(client, "test", "/", 0, {}, new AbortController().signal);
    expect(result.summary).toBeUndefined();
    expect(result.entries[0]?.metadata?.size).toBe(42);
    expect(get.mock.calls[1]?.[0]).toContain("/folders/entries");
});

it.each([401, 403, 500])("capabilitiesの%dではfallbackせず失敗を伝える", async (status) => {
    const client = new QueryClient();
    cleanups.push(() => client.clear());
    get.mockResolvedValue({ response: new Response(null, { status }) });
    await expect(fetchListingPage(client, "test", "/", 0, {}, new AbortController().signal)).rejects.toBeInstanceOf(
        ApiError,
    );
    expect(get).toHaveBeenCalledTimes(1);
});

it("提供済みの軽量一覧が404でも旧APIへ戻さない", async () => {
    const client = new QueryClient();
    client.setQueryData(["capabilities"], caps);
    cleanups.push(() => client.clear());
    get.mockResolvedValue({ response: new Response(null, { status: 404 }) });
    await expect(fetchListingPage(client, "test", "/", 0, {}, new AbortController().signal)).rejects.toBeInstanceOf(
        ApiError,
    );
    expect(get).toHaveBeenCalledTimes(1);
});

it.each([{ sort: "size" as const }, { sort: "modified_at" as const }, { recursive: true }])(
    "metadataが必要な条件は従来経路を使う %j",
    async (options) => {
        const client = new QueryClient();
        cleanups.push(() => client.clear());
        get.mockResolvedValue(response({ ...page(), entries: [full()] }));
        await fetchListingPage(client, "test", "/", 0, options, new AbortController().signal);
        expect(get).toHaveBeenCalledTimes(1);
        expect(get.mock.calls[0]?.[0]).toContain("/folders/entries");
    },
);

it("詳細だけを再試行し、同じ一覧のsnapshotと位置を維持する", async () => {
    get.mockResolvedValue(response(page()));
    post.mockResolvedValueOnce(response({ results: [{ path: "/f0", error: "NotFound" }] })).mockResolvedValue(
        response({ results: [{ path: "/f0", detail: full() }] }),
    );
    const { store } = harness();
    await vi.waitFor(() => expect(store.entryAt(0)?.detailError).toBe("NotFound"));
    store.retryDetails(["/f0"]);
    await vi.waitFor(() => expect(store.entryAt(0)?.metadata?.size).toBe(42));
    expect(store.snapshot).toBe("one");
    expect(store.locate("/f0")).toBe(0);
    expect(get).toHaveBeenCalledTimes(1);
    expect(post).toHaveBeenCalledTimes(2);
});
