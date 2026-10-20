// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";

import type { Entry, FolderListing } from "../library/entries";
import { emptySelection } from "../library/selection";
import type { BrowseLocation } from "../location";
import { stubResizeObserver } from "../testing/preferences";
import { ThumbnailQueue } from "../thumbnails/queue";
import { Grid } from "./Grid";

const mock = vi.hoisted(() => ({ get: vi.fn(), first: 0, last: 2 }));
vi.mock("../api/client", async (original) => ({
    ...(await original<typeof import("../api/client")>()),
    api: {
        GET: (route: string, options?: unknown) =>
            route === "/capabilities"
                ? Promise.resolve({
                      data: { summaryListing: false, entryDetails: false, childProbes: false },
                      response: new Response(),
                  })
                : mock.get(route, options),
    },
}));
vi.mock("@tanstack/react-virtual", () => ({
    useVirtualizer: (options: { count: number; estimateSize: () => number }) => ({
        range: { startIndex: mock.first, endIndex: mock.last },
        scrollDirection: null,
        measure: () => {},
        scrollToIndex: () => {},
        getTotalSize: () => options.count * options.estimateSize(),
        getVirtualItems: () =>
            Array.from({ length: mock.last - mock.first + 1 }, (_, offset) => ({
                index: mock.first + offset,
                key: mock.first + offset,
                start: (mock.first + offset) * options.estimateSize(),
                size: options.estimateSize(),
            })),
    }),
}));

it("reserves the full height and shows unread frames while requesting only visible pages", async () => {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    mock.first = 0;
    mock.last = 2;
    mock.get.mockImplementation((_, options) =>
        options.params.query.offset === 0
            ? Promise.resolve({
                  data: {
                      entries: Array.from({ length: 200 }, (_, i) => ({
                          name: `f${i}`,
                          path: `/f${i}`,
                          kind: "file",
                          metadata: { size: 0, readonly: false },
                      })),
                      totalCount: 2500,
                      snapshot: "one",
                      truncated: false,
                  },
                  response: new Response(),
              })
            : new Promise(() => {}),
    );
    const restore = stubResizeObserver();
    const client = new QueryClient();
    const host = document.createElement("div");
    const root = createRoot(host);
    const render = async () => {
        await act(async () =>
            root.render(
                <QueryClientProvider client={client}>
                    <Grid
                        location={{ libraryId: "big", path: "/" }}
                        tileSize={3}
                        shape="crop"
                        narrow={false}
                        sort={{ key: "name", order: "asc" }}
                        selection={emptySelection}
                        onSelectionChange={() => {}}
                        onTotal={() => {}}
                        onProgress={() => {}}
                        onOpen={() => {}}
                    />
                </QueryClientProvider>,
            ),
        );
    };
    try {
        await render();
        await vi.waitFor(async () => {
            await act(async () => {
                await Promise.resolve();
            });
            expect(host.querySelector("[role=listbox]")).not.toBeNull();
        });
        expect((host.querySelector("[role=listbox] > div") as HTMLElement).style.height).toBe("92500px");
        expect(mock.get.mock.calls.map(([, options]) => options.params.query.offset)).toEqual([0]);
        mock.first = 2497;
        mock.last = 2499;
        await render();
        expect(host.querySelectorAll("[data-placeholder]")).toHaveLength(3);
        expect(mock.get.mock.calls.map(([, options]) => options.params.query.offset)).toEqual([0, 2400]);
    } finally {
        await act(async () => root.unmount());
        client.clear();
        restore();
        mock.get.mockReset();
    }
});

const searchEntries: Entry[] = ["/base/gamma.txt", "/base/deep/delta.txt"].map((path) => ({
    path,
    name: path.split("/").at(-1)!,
    kind: "file",
    metadata: { size: 0, readonly: false },
}));
async function searchHarness(location: BrowseLocation, data: FolderListing | null) {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    mock.first = 0;
    mock.last = 2;
    mock.get.mockImplementation(() =>
        data === null ? new Promise(() => {}) : Promise.resolve({ data, response: new Response() }),
    );
    const restore = stubResizeObserver();
    const client = new QueryClient();
    const host = document.createElement("div");
    const root = createRoot(host);
    await act(async () =>
        root.render(
            <QueryClientProvider client={client}>
                <Grid
                    location={location}
                    tileSize={3}
                    shape="crop"
                    narrow={false}
                    sort={{ key: "name", order: "asc" }}
                    selection={emptySelection}
                    onSelectionChange={() => {}}
                    onTotal={() => {}}
                    onProgress={() => {}}
                    onOpen={() => {}}
                />
            </QueryClientProvider>,
        ),
    );
    if (data)
        await vi.waitFor(async () => {
            await act(async () => {
                await Promise.resolve();
            });
            expect(host.textContent).not.toContain("読み込み中");
            expect(host.textContent).not.toContain("検索中");
        });
    return {
        host,
        dispose: async () => {
            await act(async () => root.unmount());
            client.clear();
            restore();
            mock.get.mockReset();
        },
    };
}
it.each([true, false])("グリッドは配下の検索のときだけ名前の下に場所を出す %s", async (recursive) => {
    const h = await searchHarness(
        { libraryId: "search", path: "/base", query: "txt", recursive },
        { entries: searchEntries, totalCount: 2, snapshot: "search", truncated: false },
    );
    try {
        const tiles = [...h.host.querySelectorAll("[role=option]")];
        expect(tiles[0]!.querySelector('[title="."]')?.textContent).toBe(recursive ? "." : undefined);
        expect(tiles[1]!.querySelector('[title="deep"]')?.textContent).toBe(recursive ? "deep" : undefined);
        expect((h.host.querySelector("[role=listbox] > div") as HTMLElement).style.height).toBe(
            recursive ? "114px" : "74px",
        );
        expect(h.host.querySelector('[role="status"]')).toBeNull();
    } finally {
        await h.dispose();
    }
});
it.each([true, false])("グリッドは配下の最初のページだけ検索中と示す %s", async (recursive) => {
    const h = await searchHarness({ libraryId: "search", path: "/base", query: "txt", recursive }, null);
    try {
        expect(h.host.textContent).toBe(recursive ? "検索中…" : "読み込み中…");
    } finally {
        await h.dispose();
    }
});
it.each([false, true])("グリッドは検索の 0 件と打ち切りを示す %s", async (truncated) => {
    const h = await searchHarness(
        { libraryId: "search", path: "/base", query: "missing", recursive: true },
        { entries: [], totalCount: 0, snapshot: "empty", truncated },
    );
    try {
        expect(h.host.textContent).toContain("一致する項目がありません。");
        expect(h.host.querySelector('[role="status"]')?.textContent).toBe(
            truncated ? "結果の一部を表示しています。起点のフォルダを下げて探し直してください。" : undefined,
        );
    } finally {
        await h.dispose();
    }
});
it("グリッドは 0 件の通常の一覧を空のフォルダと示す", async () => {
    const h = await searchHarness(
        { libraryId: "search", path: "/base" },
        { entries: [], totalCount: 0, snapshot: "empty", truncated: false },
    );
    try {
        expect(h.host.textContent).toBe("このフォルダは空です。");
    } finally {
        await h.dispose();
    }
});
it("グリッドは結果があるときも打ち切りを示す", async () => {
    const h = await searchHarness(
        { libraryId: "search", path: "/base", query: "txt", recursive: true },
        { entries: searchEntries, totalCount: 2, snapshot: "search", truncated: true },
    );
    try {
        expect(h.host.querySelector('[role="status"]')?.textContent).toContain("起点のフォルダを下げて");
    } finally {
        await h.dispose();
    }
});

it("only wants visible folder/archive thumbnails while retaining image/video prefetch", async () => {
    const spy = vi.spyOn(ThumbnailQueue.prototype, "setWanted").mockImplementation(() => {});
    const data: FolderListing = {
        entries: Array.from({ length: 60 }, (_, i) => ({
            name: i % 4 === 0 ? `d${i}` : i % 4 === 1 ? `a${i}.zip` : i % 4 === 2 ? `i${i}.png` : `v${i}.mp4`,
            path: `/item${i}`,
            kind: i % 4 === 0 ? ("folder" as const) : ("file" as const),
            file: i % 4 === 1 ? { archive: true } : undefined,
        })),
        totalCount: 60,
        snapshot: "visible-thumbnails",
        truncated: false,
    };
    const h = await searchHarness({ libraryId: "visible-thumbnails", path: "/" }, data);
    try {
        const last = spy.mock.calls.at(-1);
        if (!last || !last[1]) throw new Error("thumbnail demand missing");
        const wanted = last[0];
        const delayed = last[1];
        const visible = new Set(
            [...h.host.querySelectorAll("[role=option][data-index]")].map((tile) =>
                Number((tile as HTMLElement).dataset.index),
            ),
        );
        expect(delayed.size).toBeGreaterThan(0);
        for (const path of delayed) expect(visible.has(Number(path.replace("/item", "")))).toBe(true);
        expect([...wanted.keys()].some((path) => !visible.has(Number(path.replace("/item", ""))))).toBe(true);
        for (const path of wanted.keys()) {
            const i = Number(path.replace("/item", ""));
            if (i % 4 <= 1) expect(delayed.has(path)).toBe(true);
        }
    } finally {
        await h.dispose();
        spy.mockRestore();
    }
});
