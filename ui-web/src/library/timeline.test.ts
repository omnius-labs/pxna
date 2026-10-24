// @vitest-environment jsdom
import { QueryClient } from "@tanstack/react-query";
import { afterEach, describe, expect, it, vi } from "vitest";

import { ListingSnapshotChanged, PAGE_SIZE } from "./listing";
import {
    keepRange,
    reachablePages,
    refreshTimeline,
    TIMELINE_BASE,
    TIMELINE_PAGE_SIZE,
    TimelineStore,
    timelineLibraryIds,
} from "./timeline";

const { get } = vi.hoisted(() => ({ get: vi.fn() }));
vi.mock("../api/client", async (original) => ({
    ...(await original<typeof import("../api/client")>()),
    api: { GET: get },
}));
afterEach(() => get.mockReset());

function item(name: string, date = "2025-05-01T00:00:00", libraryId = "photos") {
    return {
        libraryId,
        entry: {
            path: `/${name}`,
            name,
            kind: "file",
            metadata: { size: 1, readonly: false },
            file: { archive: false },
        },
        timelineDate: date,
        timelineDateSource: "exif",
    };
}
const reply = (names: string[], cursors: { previous?: string; next?: string } = {}) => ({
    data: { entries: names.map((name) => item(name)), ...cursors },
    response: new Response(),
});
const names = (store: TimelineStore) => store.loadedItems.map((entry) => entry.entry.name);
const query = (call: number) => get.mock.calls[call]?.[1].params.query;

describe("TimelineStore", () => {
    it("keeps the page size in step with the listing page size", () => {
        expect(TIMELINE_PAGE_SIZE).toBe(PAGE_SIZE);
        expect(TIMELINE_BASE % PAGE_SIZE).toBe(0);
    });

    it("reads the anchor and exposes absolute positions with an unread slot after the loaded range", async () => {
        get.mockResolvedValueOnce(reply(["a", "b"], { next: "c1" }));
        const store = new TimelineStore(["photos"], { kind: "month", month: "2024-05" });
        expect(store.totalCount).toBeUndefined();
        await store.ensureInitial();
        expect(query(0)).toMatchObject({ month: "2024-05", limit: 200 });
        expect(store.firstIndex).toBe(TIMELINE_BASE);
        expect(store.loadedFrom).toBe(TIMELINE_BASE);
        expect(store.totalCount).toBe(TIMELINE_BASE + 2 + 1);
        expect(store.entryAt(TIMELINE_BASE)?.name).toBe("a");
        expect(store.entryAt(TIMELINE_BASE + 2)).toBeUndefined();
        expect(store.locate("/photos/b")).toBe(TIMELINE_BASE + 1);
        expect(store.locate("/photos/missing")).toBeUndefined();
        expect(store.snapshot).toBe("1");
    });

    it("prepends newer entries without moving the positions of the loaded ones", async () => {
        get.mockResolvedValueOnce(reply(["c", "d"], { previous: "p1", next: "n1" }));
        const store = new TimelineStore(["photos"], { kind: "around", path: "/photos/c" });
        await store.ensureInitial();
        expect(query(0)).toMatchObject({ libraryIds: ["photos"], around: "/c", aroundLibraryId: "photos" });
        expect(store.firstIndex).toBe(0);
        const before = store.locate("/photos/d");

        get.mockResolvedValueOnce(reply(["a", "b", "c"], { previous: "p0" }));
        await store.loadMore("previous");
        expect(query(1)).toMatchObject({ before: "p1" });
        expect(names(store)).toEqual(["a", "b", "c", "d"]);
        expect(store.locate("/photos/d")).toBe(before);
        expect(store.loadedFrom).toBe(TIMELINE_BASE - 2);
        expect(store.snapshot).toBe("1");

        get.mockResolvedValueOnce(reply(["z"]));
        await store.loadMore("previous");
        expect(store.hasPrevious).toBe(false);
        expect(store.firstIndex).toBe(store.loadedFrom);
    });

    it("appends older entries and stops asking when there is nothing further", async () => {
        get.mockResolvedValueOnce(reply(["a"], { next: "n1" }));
        const store = new TimelineStore(["photos"], { kind: "newest" });
        await store.ensureInitial();
        get.mockResolvedValueOnce(reply(["b"]));
        await store.loadMore("next");
        expect(query(1)).toMatchObject({ after: "n1" });
        expect(names(store)).toEqual(["a", "b"]);
        expect(store.hasNext).toBe(false);
        expect(store.totalCount).toBe(TIMELINE_BASE + 2);
        await store.loadMore("next");
        expect(get).toHaveBeenCalledTimes(2);
    });

    it("shares one request per direction", async () => {
        get.mockResolvedValueOnce(reply(["a"], { next: "n1" }));
        const store = new TimelineStore(["photos"], { kind: "newest" });
        await store.ensureInitial();
        get.mockResolvedValueOnce(reply(["b"]));
        await Promise.all([store.loadMore("next"), store.loadMore("next")]);
        expect(get).toHaveBeenCalledTimes(2);
    });

    it("reports a page as loaded only when its loaded part covers it", async () => {
        get.mockResolvedValueOnce(reply(["a", "b"], { previous: "p1", next: "n1" }));
        const store = new TimelineStore(["photos"], { kind: "around", path: "/photos/a" });
        await store.ensureInitial();
        const page = Math.floor(TIMELINE_BASE / PAGE_SIZE);
        expect(store.pageStatus(page)).toBe("idle");
        get.mockResolvedValueOnce(reply(["c"]));
        await store.loadPages([page], new AbortController().signal);
        expect(store.hasNext).toBe(false);
        expect(store.pageStatus(page + 1)).toBe("loaded");
        expect(store.pageStatus(0)).toBe("idle");
    });

    it("loads further until the requested page is covered", async () => {
        const full = (prefix: string) => Array.from({ length: PAGE_SIZE }, (_, i) => `${prefix}${i}`);
        get.mockResolvedValueOnce(reply(full("a"), { next: "n1" }));
        const store = new TimelineStore(["photos"], { kind: "newest" });
        await store.ensureInitial();
        const first = Math.floor(TIMELINE_BASE / PAGE_SIZE);
        // 2 ページ目の先頭は、読み込み済みの範囲の次の位置（未読の 1 つ）である。
        expect(store.pageStatus(first + 1)).toBe("idle");
        get.mockResolvedValueOnce(reply(full("b"), { next: "n2" }));
        await store.loadPages([first + 1], new AbortController().signal);
        expect(store.loadedItems).toHaveLength(PAGE_SIZE * 2);
        expect(store.pageStatus(first + 1)).toBe("loaded");
        expect(store.pageStatus(first + 2)).toBe("idle");
        expect(get).toHaveBeenCalledTimes(2);
    });

    it("rejects a pending load when the snapshot changes and keeps the old range until the reload arrives", async () => {
        get.mockResolvedValueOnce(reply(["a"], { next: "n1" }));
        const store = new TimelineStore(["photos"], { kind: "newest" });
        await store.ensureInitial();
        let release: (value: unknown) => void = () => {};
        get.mockReturnValueOnce(new Promise((resolve) => (release = resolve)));
        const outcome = store.loadPages([Math.floor(TIMELINE_BASE / PAGE_SIZE)], new AbortController().signal).then(
            () => "loaded",
            (error: unknown) => error,
        );
        // 続きの要求を出させてから、読み直す。
        await new Promise((resolve) => setTimeout(resolve, 0));
        expect(get).toHaveBeenCalledTimes(2);

        get.mockResolvedValueOnce(reply(["x"]));
        await store.reload();
        expect(names(store)).toEqual(["x"]);
        release(reply(["late"]));
        expect(await outcome).toBeInstanceOf(ListingSnapshotChanged);
        expect(names(store)).toEqual(["x"]);
        expect(store.snapshot).toBe("2");
    });

    it("reloads around the focused entry and falls back to the first page when it is gone", async () => {
        get.mockResolvedValueOnce(reply(["a", "b"], { next: "n1" }));
        const store = new TimelineStore(["photos"], { kind: "newest" });
        await store.ensureInitial();
        store.setFocus("/photos/b");
        get.mockResolvedValueOnce(reply(["a", "b"]));
        await store.reload();
        expect(query(1)).toMatchObject({ around: "/b", aroundLibraryId: "photos" });

        store.setFocus("/photos/gone");
        get.mockRejectedValueOnce(new Error("not found"));
        get.mockResolvedValueOnce(reply(["a"]));
        await store.reload();
        expect(query(3)).toEqual({ libraryIds: ["photos"], limit: 200 });
        expect(names(store)).toEqual(["a"]);
    });

    it("renumbers positions when older loads would take the start below zero", async () => {
        get.mockResolvedValueOnce(reply(["start"], { previous: "p0" }));
        const store = new TimelineStore(["photos"], { kind: "around", path: "/photos/start" });
        await store.ensureInitial();
        let page = 0;
        get.mockImplementation(async () => {
            page += 1;
            return reply(
                Array.from({ length: 200 }, (_, i) => `n${page}-${i}`),
                { previous: `p${page}` },
            );
        });
        const snapshots = new Set<string | undefined>();
        for (let i = 0; i < 500; i += 1) {
            await store.loadMore("previous");
            snapshots.add(store.snapshot);
        }
        expect(store.loadedFrom).toBeGreaterThan(0);
        expect(snapshots.size).toBeGreaterThan(1);
        expect(store.locate("/photos/start")).toBeDefined();
    });

    it("drops the reload of unused stores and reloads the ones in use", async () => {
        const client = new QueryClient();
        const store = new TimelineStore(["photos"], { kind: "newest" });
        get.mockResolvedValueOnce(reply(["a"]));
        await store.ensureInitial();
        await refreshTimeline(client, ["photos"]);
        expect(get).toHaveBeenCalledTimes(1);
    });

    describe("trim", () => {
        const many = (prefix: string, count: number) =>
            Array.from({ length: count }, (_, i) => ({
                ...item(`${prefix}${String(i).padStart(4, "0")}`, "2025-05-01T00:00:00"),
            }));
        const loadedStore = async () => {
            get.mockResolvedValueOnce({
                data: { entries: many("e", 1000), previous: "p", next: "n" },
                response: new Response(),
            });
            const store = new TimelineStore(["photos"], { kind: "around", path: "/photos/e0500" });
            await store.ensureInitial();
            return store;
        };

        it("drops entries far from the kept range and keeps the positions of the rest", async () => {
            const store = await loadedStore();
            const keep = store.locate("/photos/e0600") ?? 0;
            store.trim(keep - 100, keep + 100);
            expect(store.loadedItems).toHaveLength(201);
            expect(store.loadedFrom).toBe(TIMELINE_BASE + 500);
            expect(store.locate("/photos/e0600")).toBe(keep);
            expect(store.entryAt(TIMELINE_BASE)).toBeUndefined();
            // 捨てた側は、捨てた境の Entry の cursor から読み直す。
            get.mockResolvedValueOnce(reply(["x"], { previous: "p2" }));
            await store.loadMore("previous");
            expect(query(1)).toEqual({ libraryIds: ["photos"], before: "2025-05-01T00:00:00photos/e0500", limit: 200 });
            get.mockResolvedValueOnce(reply(["y"]));
            await store.loadMore("next");
            expect(query(2)).toEqual({ libraryIds: ["photos"], after: "2025-05-01T00:00:00photos/e0700", limit: 200 });
        });

        it("does nothing for a small excess and does not trim a side that is loading", async () => {
            const store = await loadedStore();
            store.trim(TIMELINE_BASE + 100, TIMELINE_BASE + 950);
            expect(store.loadedItems).toHaveLength(1000);
            let release: (value: unknown) => void = () => {};
            get.mockReturnValueOnce(new Promise((resolve) => (release = resolve)));
            const loading = store.loadMore("next");
            store.trim(TIMELINE_BASE + 400, TIMELINE_BASE + 450);
            expect(store.loadedItems).toHaveLength(600);
            expect(store.loadedTo).toBe(TIMELINE_BASE + 1000);
            release(reply(["z"]));
            await loading;
        });
    });

    it("keeps the opened entry and the filmstrip pages with a margin", () => {
        expect(keepRange(undefined, new Set())).toBeUndefined();
        expect(keepRange(5000, new Set(), 100)).toEqual({ first: 4900, last: 5100 });
        expect(keepRange(5000, new Set([30, 31]), 100)).toEqual({ first: 4900, last: 6499 });
        expect(keepRange(8000, new Set([30, 31]), 100)).toEqual({ first: 5900, last: 8100 });
    });

    it("does not read pages farther than the reach from the opened entry, so the kept range stays bounded", () => {
        const far = new Set([5, 30, 600]);
        expect(reachablePages(6000, far, 1000)).toEqual(new Set([30]));
        const reachable = reachablePages(6000, new Set([1000]), 1000);
        expect(reachable.size).toBe(0);
        const range = keepRange(6000, reachablePages(6000, new Set([0, 100, 100000]), 3000), 1000);
        expect(range && range.last - range.first).toBeLessThan(2 * (3000 + 200 + 1000));
        expect(reachablePages(undefined, far)).toBe(far);
    });

    it("stops reading further by itself after a failure and starts again when retried", async () => {
        get.mockResolvedValueOnce(reply(["a"], { next: "n1" }));
        const store = new TimelineStore(["photos"], { kind: "newest" });
        await store.ensureInitial();
        get.mockRejectedValue(new Error("down"));
        store.wantMore("next");
        await vi.waitFor(() => expect(store.error).not.toBeNull());
        store.wantMore("next");
        store.wantMore("next");
        await new Promise((resolve) => setTimeout(resolve, 10));
        expect(get).toHaveBeenCalledTimes(2);

        get.mockReset();
        get.mockResolvedValueOnce(reply(["b"]));
        store.retry();
        store.wantMore("next");
        await vi.waitFor(() => expect(names(store)).toEqual(["a", "b"]));
        expect(store.error).toBeNull();
    });

    it("keys entries of different Libraries apart and asks for every Library", async () => {
        get.mockResolvedValueOnce({
            data: {
                entries: [item("same.jpg", "2025-05-01T00:00:00", "a"), item("same.jpg", "2025-05-01T00:00:00", "b")],
            },
            response: new Response(),
        });
        const store = new TimelineStore(["a", "b"], { kind: "newest" });
        await store.ensureInitial();
        expect(query(0)).toMatchObject({ libraryIds: ["a", "b"] });
        expect(store.locate("/a/same.jpg")).toBe(TIMELINE_BASE);
        expect(store.locate("/b/same.jpg")).toBe(TIMELINE_BASE + 1);
        expect(store.itemAt(TIMELINE_BASE + 1)).toMatchObject({ libraryId: "b", path: "/same.jpg" });
    });
});

describe("timelineLibraryIds", () => {
    const libraries = [{ id: "a" }, { id: "b" }, { id: "c" }];
    it("shows every Library until a choice is saved", () => {
        expect(timelineLibraryIds(libraries, undefined)).toEqual(["a", "b", "c"]);
    });
    it("keeps the saved Libraries that still exist in the order of the Library list", () => {
        expect(timelineLibraryIds(libraries, ["c", "gone", "a"])).toEqual(["a", "c"]);
    });
    it("falls back to every Library when none of the saved ones exists", () => {
        expect(timelineLibraryIds(libraries, ["gone"])).toEqual(["a", "b", "c"]);
    });
});
