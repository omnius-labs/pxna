// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import { emptySelection, type Selection } from "../library/selection";
import { TimelineStore } from "../library/timeline";
import { TIMELINE_SCOPE } from "../location";
import { stubResizeObserver } from "../testing/preferences";
import { TimelineGrid } from "./TimelineGrid";

const mock = vi.hoisted(() => ({ get: vi.fn(), entries: vi.fn(), first: 0, last: 3 }));
vi.mock("../api/client", async (original) => ({
    ...(await original<typeof import("../api/client")>()),
    api: { GET: mock.get },
}));
vi.mock("@tanstack/react-virtual", () => ({
    useVirtualizer: (options: { count: number; estimateSize: (index: number) => number }) => ({
        range: { startIndex: mock.first, endIndex: Math.min(mock.last, options.count - 1) },
        scrollDirection: null,
        measure: () => {},
        scrollToIndex: () => {},
        getTotalSize: () => 0,
        getVirtualItems: () =>
            Array.from({ length: Math.min(mock.last, options.count - 1) - mock.first + 1 }, (_, offset) => ({
                index: mock.first + offset,
                key: mock.first + offset,
                start: 0,
                size: options.estimateSize(mock.first + offset),
            })),
    }),
}));
beforeEach(() => {
    // タイムラインの一覧だけを返し、サムネイルなど、ほかの要求は終わらせない。
    mock.get.mockImplementation((path: string, options: unknown) =>
        path.endsWith("/timeline/entries") ? mock.entries(options) : new Promise(() => {}),
    );
});
afterEach(() => {
    mock.get.mockReset();
    mock.entries.mockReset();
});

const item = (name: string, date: string) => ({
    libraryId: "photos",
    entry: {
        path: `/${name}`,
        name,
        kind: "file",
        metadata: { size: 1, readonly: false, contentType: "image/png" },
        file: { archive: false },
    },
    timelineDate: date,
    timelineDateSource: "exif",
});
const reply = (items: ReturnType<typeof item>[], cursors: { previous?: string; next?: string } = {}) => ({
    data: { entries: items, ...cursors },
    response: new Response(),
});

async function harness(store: TimelineStore, onSelectionChange = vi.fn<(selection: Selection) => void>()) {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    const restore = stubResizeObserver();
    const client = new QueryClient();
    const host = document.createElement("div");
    const root = createRoot(host);
    await act(async () =>
        root.render(
            <QueryClientProvider client={client}>
                <TimelineGrid
                    store={store}
                    libraryId={TIMELINE_SCOPE}
                    tileSize={3}
                    shape="crop"
                    narrow={false}
                    selection={emptySelection}
                    onSelectionChange={onSelectionChange}
                    onProgress={() => {}}
                    onVisibleMonth={() => {}}
                    onOpen={() => {}}
                    emptyMessage="画像と動画がありません。"
                />
            </QueryClientProvider>,
        ),
    );
    return {
        host,
        onSelectionChange,
        dispose: async () => {
            await act(async () => root.unmount());
            client.clear();
            restore();
        },
    };
}

it("puts a heading before each day and lists the tiles of a day from newest to oldest", async () => {
    mock.first = 0;
    mock.last = 9;
    mock.entries.mockResolvedValueOnce(
        reply([
            item("a.png", "2025-05-03T10:00:00"),
            item("b.png", "2025-05-03T09:00:00"),
            item("c.png", "2025-04-30T09:00:00"),
        ]),
    );
    const store = new TimelineStore(["photos"], { kind: "newest" });
    await store.ensureInitial();
    const h = await harness(store);
    try {
        expect([...h.host.querySelectorAll("h2")].map((heading) => heading.textContent)).toEqual([
            "2025年5月3日（土）",
            "2025年4月30日（水）",
        ]);
        expect([...h.host.querySelectorAll("[role=option]")].map((tile) => tile.getAttribute("title"))).toEqual([
            "a.png",
            "b.png",
            "c.png",
        ]);
        expect(h.host.querySelector("[role=listbox]")?.getAttribute("aria-label")).toBe("タイムライン");
    } finally {
        await h.dispose();
    }
});

it("selects the clicked tile by its absolute position", async () => {
    mock.first = 0;
    mock.last = 9;
    mock.entries.mockResolvedValueOnce(
        reply([item("a.png", "2025-05-03T10:00:00"), item("b.png", "2025-05-03T09:00:00")]),
    );
    const store = new TimelineStore(["photos"], { kind: "newest" });
    await store.ensureInitial();
    const h = await harness(store);
    try {
        const second = h.host.querySelector<HTMLElement>("[role=option][title='b.png']");
        await act(async () => second?.click());
        const selection = h.onSelectionChange.mock.calls.at(-1)?.[0];
        expect([...(selection?.items.keys() ?? [])]).toEqual(["/photos/b.png"]);
        expect(selection?.anchor).toBe(store.loadedFrom + 1);
    } finally {
        await h.dispose();
    }
});

it("reads older entries when the visible rows reach the end, and newer ones at the start", async () => {
    mock.first = 0;
    mock.last = 9;
    mock.entries.mockResolvedValueOnce(reply([item("a.png", "2025-05-03T10:00:00")], { previous: "p1", next: "n1" }));
    const store = new TimelineStore(["photos"], { kind: "around", path: "/photos/a.png" });
    await store.ensureInitial();
    mock.entries.mockResolvedValue(reply([]));
    const h = await harness(store);
    try {
        await vi.waitFor(() => {
            const queries = mock.entries.mock.calls.map(([options]) => options.params.query);
            expect(queries).toContainEqual(expect.objectContaining({ after: "n1" }));
            expect(queries).toContainEqual(expect.objectContaining({ before: "p1" }));
        });
    } finally {
        await h.dispose();
    }
});

it("shows the empty message when nothing is loaded", async () => {
    mock.entries.mockResolvedValueOnce(reply([]));
    const store = new TimelineStore(["photos"], { kind: "newest" });
    await store.ensureInitial();
    const h = await harness(store);
    try {
        expect(h.host.textContent).toContain("画像と動画がありません。");
        expect(h.host.querySelector("[role=listbox]")).toBeNull();
    } finally {
        await h.dispose();
    }
});

it("keeps the clicked entry's anchor after older entries are read", async () => {
    mock.first = 0;
    mock.last = 9;
    mock.entries.mockResolvedValueOnce(reply([item("a.png", "2025-05-03T10:00:00")], { next: "n1" }));
    const store = new TimelineStore(["photos"], { kind: "newest" });
    await store.ensureInitial();
    mock.entries.mockResolvedValueOnce(reply([item("b.png", "2025-05-02T10:00:00")]));
    const h = await harness(store);
    try {
        await vi.waitFor(() => expect(store.loadedItems).toHaveLength(2));
        expect(store.locate("/photos/a.png")).toBe(store.loadedFrom);
        expect(h.host.querySelectorAll("[role=option]")).toHaveLength(2);
    } finally {
        await h.dispose();
    }
});
