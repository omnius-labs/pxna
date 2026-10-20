import { describe, expect, it, vi } from "vitest";

import { type ThumbnailLoader, ThumbnailQueue } from "./queue";
import { tilePriority, wantedRows } from "./range";

/// 呼ばれた要求を手元で完了させられる、偽の取得。
function fakeLoader() {
    const calls: { key: string; signal: AbortSignal; resolve: (blob: Blob) => void; reject: (error: Error) => void }[] =
        [];
    const load: ThumbnailLoader = (key, signal) =>
        new Promise<Blob>((resolve, reject) => {
            calls.push({ key, signal, resolve, reject });
        });
    return { load, calls, started: () => calls.map((call) => call.key) };
}

function queueWith(load: ThumbnailLoader) {
    const revoked: string[] = [];
    const queue = new ThumbnailQueue(load, {
        createUrl: () => `blob:${Math.random()}`,
        revokeUrl: (url) => revoked.push(url),
    });
    return { queue, revoked };
}

const wanted = (...entries: [string, number][]) => new Map(entries);
const flush = () => new Promise((resolve) => setTimeout(resolve, 0));

describe("ThumbnailQueue", () => {
    it("delays visible folder/archive requests without delaying image/video prefetch or restarting deadlines", async () => {
        vi.useFakeTimers();
        const { load, calls, started } = fakeLoader();
        const { queue } = queueWith(load);
        try {
            const requests = wanted(["folder", 0], ["archive", 1], ["image-prefetch", 8], ["video-prefetch", 9]);
            const delayed = new Set(["folder", "archive"]);
            queue.setWanted(requests, delayed);
            await Promise.resolve();
            expect(started()).toEqual(["image-prefetch", "video-prefetch"]);
            vi.advanceTimersByTime(100);
            queue.setWanted(requests, delayed);
            await Promise.resolve();
            vi.advanceTimersByTime(99);
            expect(started()).toEqual(["image-prefetch", "video-prefetch"]);
            vi.advanceTimersByTime(1);
            expect(started()).toEqual(["image-prefetch", "video-prefetch", "folder", "archive"]);
            queue.setWanted(wanted(["image-prefetch", 0], ["video-prefetch", 1]));
            expect(calls.slice(2).every((call) => call.signal.aborted)).toBe(true);
        } finally {
            queue.dispose();
            vi.useRealTimers();
        }
    });

    it("cancels both pending timers and queued microtasks on leaving or disposal and starts a fresh delay on return", async () => {
        vi.useFakeTimers();
        const { load, started } = fakeLoader();
        const { queue } = queueWith(load);
        try {
            queue.setWanted(wanted(["a", 0]), new Set(["a"]));
            queue.setWanted(wanted());
            await Promise.resolve();
            vi.advanceTimersByTime(200);
            expect(started()).toEqual([]);
            queue.setWanted(wanted(["a", 0]), new Set(["a"]));
            await Promise.resolve();
            vi.advanceTimersByTime(199);
            expect(started()).toEqual([]);
            queue.dispose();
            vi.advanceTimersByTime(1000);
            expect(started()).toEqual([]);
            queue.setWanted(wanted(["a", 0]), new Set(["a"]));
            await Promise.resolve();
            vi.advanceTimersByTime(200);
            expect(started()).toEqual(["a"]);
        } finally {
            queue.dispose();
            vi.useRealTimers();
        }
    });
    it("never sends more than 4 requests at once and starts the next when one finishes", async () => {
        const { load, calls, started } = fakeLoader();
        const { queue } = queueWith(load);
        queue.setWanted(wanted(...["a", "b", "c", "d", "e", "f"].map((key, index): [string, number] => [key, index])));
        expect(started()).toEqual(["a", "b", "c", "d"]);

        calls[0]?.resolve(new Blob());
        await flush();
        expect(started()).toEqual(["a", "b", "c", "d", "e"]);
        expect(queue.getState("a").status).toBe("ready");
        expect(queue.getState("e").status).toBe("loading");
        expect(queue.getState("f").status).toBe("idle");
    });

    it("sends the tiles with the smallest priority first, also after the wanted tiles change", async () => {
        const { load, calls, started } = fakeLoader();
        const { queue } = queueWith(load);
        queue.setWanted(wanted(["far", 9], ["mid", 5], ["near", 1], ["nearer", 0], ["farthest", 10], ["middle2", 6]));
        expect(started()).toEqual(["nearer", "near", "mid", "middle2"]);

        // 中央が移り、まだ送っていない 2 つの優先度が入れ替わる。
        queue.setWanted(wanted(["far", 0], ["mid", 5], ["near", 1], ["nearer", 0], ["farthest", 10], ["middle2", 6]));
        calls[0]?.resolve(new Blob());
        await flush();
        expect(started().slice(4)).toEqual(["far"]);
    });

    it("drops pending tiles that leave the wanted set and aborts the sent ones", async () => {
        const { load, calls, started } = fakeLoader();
        const { queue } = queueWith(load);
        queue.setWanted(wanted(["a", 0], ["b", 1], ["c", 2], ["d", 3], ["e", 4]));
        expect(started()).toEqual(["a", "b", "c", "d"]);

        queue.setWanted(wanted(["c", 2], ["d", 3], ["z", 5]));
        expect(calls.slice(0, 4).map((call) => call.signal.aborted)).toEqual([true, true, false, false]);
        expect(queue.getState("a").status).toBe("idle");
        // 空いた枠は、まだ要求したいタイルに使う。外れた "e" は送らない。
        expect(started()).toEqual(["a", "b", "c", "d", "z"]);
    });

    it("ignores the result of an aborted request", async () => {
        const { load, calls } = fakeLoader();
        const { queue, revoked } = queueWith(load);
        queue.setWanted(wanted(["a", 0]));
        queue.setWanted(wanted());
        calls[0]?.resolve(new Blob());
        await flush();
        expect(queue.getState("a").status).toBe("idle");
        expect(revoked).toEqual([]);
    });

    it("does not request a failed tile again while the queue lives", async () => {
        const { load, calls, started } = fakeLoader();
        const { queue } = queueWith(load);
        queue.setWanted(wanted(["a", 0], ["b", 1]));
        calls[0]?.reject(new Error("504"));
        await flush();
        expect(queue.getState("a").status).toBe("failed");

        queue.setWanted(wanted(["a", 0], ["b", 1], ["c", 2]));
        queue.setWanted(wanted(["a", 0], ["b", 1], ["c", 2]));
        expect(started().filter((key) => key === "a")).toHaveLength(1);
        expect(started()).toContain("c");
    });

    it("keeps the image while the tile is wanted and releases its URL when the tile leaves", async () => {
        const { load, calls } = fakeLoader();
        const { queue, revoked } = queueWith(load);
        queue.setWanted(wanted(["a", 0]));
        calls[0]?.resolve(new Blob());
        await flush();
        const state = queue.getState("a");
        expect(state.status).toBe("ready");

        queue.setWanted(wanted(["a", 3]));
        expect(queue.getState("a")).toBe(state);
        expect(revoked).toEqual([]);

        queue.setWanted(wanted(["b", 0]));
        expect(revoked).toEqual([state.status === "ready" ? state.url : ""]);
        expect(queue.getState("a").status).toBe("idle");
    });

    it("notifies subscribers and can be reused after dispose", async () => {
        const { load, calls } = fakeLoader();
        const { queue, revoked } = queueWith(load);
        let notified = 0;
        queue.subscribe(() => (notified += 1));
        queue.setWanted(wanted(["a", 0], ["b", 1]));
        calls[0]?.resolve(new Blob());
        await flush();
        expect(notified).toBeGreaterThanOrEqual(2);

        queue.dispose();
        expect(calls[1]?.signal.aborted).toBe(true);
        expect(revoked).toHaveLength(1);
        expect(queue.getState("a").status).toBe("idle");

        // 解放した後も、要求したいタイルを渡せば使い直せる。
        queue.setWanted(wanted(["c", 0]));
        expect(calls.map((call) => call.key)).toEqual(["a", "b", "c"]);
    });
});

describe("wantedRows", () => {
    it("looks ahead one screen in the scroll direction", () => {
        const base = { firstRow: 10, lastRow: 14, rowCount: 100 };
        expect(wantedRows({ ...base, direction: "forward" })).toEqual({ first: 10, last: 19, center: 12 });
        expect(wantedRows({ ...base, direction: "backward" })).toEqual({ first: 5, last: 14, center: 12 });
    });

    it("stays inside the rows that exist", () => {
        expect(wantedRows({ firstRow: 0, lastRow: 4, rowCount: 7, direction: "forward" })).toMatchObject({
            first: 0,
            last: 6,
        });
        expect(wantedRows({ firstRow: 2, lastRow: 6, rowCount: 100, direction: "backward" })).toMatchObject({
            first: 0,
            last: 6,
        });
    });
});

describe("tilePriority", () => {
    it("puts rows near the center first and breaks ties from the left", () => {
        expect(tilePriority(12, 3, 12)).toBeLessThan(tilePriority(11, 0, 12) + 1);
        expect(tilePriority(12, 0, 12)).toBeLessThan(tilePriority(12, 1, 12));
        expect(tilePriority(13, 0, 12)).toBeLessThan(tilePriority(15, 0, 12));
        expect(tilePriority(10, 4, 12)).toBeGreaterThan(tilePriority(11, 4, 12));
    });
});
