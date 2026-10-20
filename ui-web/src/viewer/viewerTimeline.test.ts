// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";

import { TIMELINE_BASE, TimelineStore } from "../library/timeline";
import { findEntry, loadedNeighbor, neighbor } from "./viewer";

const { get } = vi.hoisted(() => ({ get: vi.fn() }));
vi.mock("../api/client", async (original) => ({
    ...(await original<typeof import("../api/client")>()),
    api: { GET: get },
}));
afterEach(() => get.mockReset());

const item = (name: string, contentType = "image/png") => ({
    entry: {
        path: `/${name}`,
        name,
        kind: "file",
        metadata: { size: 1, readonly: false, contentType },
        file: { archive: false },
    },
    timelineDate: "2025-05-01T00:00:00",
    timelineDateSource: "exif",
});
const reply = (names: string[], cursors: { previous?: string; next?: string } = {}) => ({
    data: { entries: names.map((name) => item(name, name.endsWith(".mp4") ? "video/mp4" : "image/png")), ...cursors },
    response: new Response(),
});
const signal = () => new AbortController().signal;

it("finds the opened entry among the loaded entries without paging from the start", async () => {
    get.mockResolvedValueOnce(reply(["b.png", "c.png"], { previous: "p", next: "n" }));
    const store = new TimelineStore("photos", { kind: "around", path: "/c.png" });
    const found = await findEntry(store, "/", "c.png", signal());
    expect(found?.index).toBe(TIMELINE_BASE + 1);
    expect(found?.entry.name).toBe("c.png");
    expect(get).toHaveBeenCalledTimes(1);
    expect(await findEntry(store, "/", "missing.png", signal())).toBeUndefined();
    expect(get).toHaveBeenCalledTimes(1);
});

it("moves to newer and older images by reading further, skipping videos", async () => {
    get.mockResolvedValueOnce(reply(["c.png"], { previous: "p", next: "n" }));
    const store = new TimelineStore("photos", { kind: "around", path: "/c.png" });
    const opened = await findEntry(store, "/", "c.png", signal());
    const index = opened?.index ?? -1;
    expect(loadedNeighbor(store, index, 1)).toBe("unread");
    expect(loadedNeighbor(store, index, -1)).toBe("unread");

    get.mockResolvedValueOnce(reply(["clip.mp4", "d.png"]));
    expect((await neighbor(store, index, 1, signal()))?.entry.name).toBe("d.png");
    expect(get.mock.calls.at(-1)?.[1].params.query).toMatchObject({ after: "n" });

    get.mockResolvedValueOnce(reply(["a.png", "movie.mp4"], { previous: "p0" }));
    const before = await neighbor(store, index, -1, signal());
    expect(before?.entry.name).toBe("a.png");
    expect(before?.index).toBe(TIMELINE_BASE - 2);
    // 前へ読み足しても、開いている画像の位置は変わらない。
    expect(store.entryAt(index)?.name).toBe("c.png");
});

it("ends at the newest and oldest entries once nothing further remains", async () => {
    get.mockResolvedValueOnce(reply(["only.png"]));
    const store = new TimelineStore("photos", { kind: "around", path: "/only.png" });
    const opened = await findEntry(store, "/", "only.png", signal());
    const index = opened?.index ?? -1;
    expect(loadedNeighbor(store, index, 1)).toBeUndefined();
    expect(loadedNeighbor(store, index, -1)).toBeUndefined();
    expect(await neighbor(store, index, 1, signal())).toBeUndefined();
    expect(await neighbor(store, index, -1, signal())).toBeUndefined();
    expect(get).toHaveBeenCalledTimes(1);
});
