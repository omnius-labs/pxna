import { describe, expect, it, vi } from "vitest";

import type { Entry } from "../library/entries";
import { type Listing, ListingSnapshotChanged, PAGE_SIZE } from "../library/listing";
import {
    clampPan,
    clampZoom,
    displayRequestSize,
    findEntry,
    fittedWidth,
    loadedNeighbor,
    MAX_ZOOM,
    needsOriginal,
    neighbor,
    originalTiming,
    zoomAfterWheel,
    zoomView,
} from "./viewer";

function entry(name: string, contentType?: string): Entry {
    return {
        path: `/a/${name}`,
        name,
        kind: "file",
        metadata: { size: 1, readonly: false, ...(contentType === undefined ? {} : { contentType }) },
        file: { archive: false },
    };
}

function listing(entries: Entry[], initial: number[] = [0]) {
    const loaded = new Set(initial);
    const loadPages = vi.fn(async (pages: readonly number[], signal: AbortSignal) => {
        signal.throwIfAborted();
        pages.forEach((page) => {
            loaded.add(page);
        });
    });
    const value: Listing = {
        truncated: false,
        totalCount: entries.length,
        snapshot: "one",
        version: 0,
        entryAt: (index) => (loaded.has(Math.floor(index / PAGE_SIZE)) ? entries[index] : undefined),
        pageStatus: (page) => (loaded.has(page) ? "loaded" : "idle"),
        pageError: () => null,
        loadPages,
        retry: () => {},
    };
    return { value, loadPages, loaded };
}
const signal = () => new AbortController().signal;

describe("viewer navigation", () => {
    it("reads the next unread page and skips entries other than images and videos in one operation", async () => {
        const entries = Array.from({ length: 203 }, (_, index) => entry(`${index}.txt`));
        entries[199] = entry("a.png", "image/png");
        entries[201] = entry("b.mp4", "video/mp4");
        entries[202] = entry("c.jpg", "image/jpeg");
        const h = listing(entries);
        expect(loadedNeighbor(h.value, 199, 1)).toBe("unread");
        expect(h.loadPages).not.toHaveBeenCalled();
        expect((await neighbor(h.value, 199, 1, signal()))?.entry.name).toBe("b.mp4");
        expect(h.loadPages.mock.calls.map(([pages]) => pages)).toEqual([[1]]);
        expect(loadedNeighbor(h.value, 199, 1)).toMatchObject({ index: 201 });
        expect((await neighbor(h.value, 201, 1, signal()))?.entry.name).toBe("c.jpg");
    });
    it("reads backward across a page and stops at both ends", async () => {
        const entries = Array.from({ length: 202 }, (_, index) => entry(`${index}.txt`));
        entries[198] = entry("a.png", "image/png");
        entries[201] = entry("b.jpg", "image/jpeg");
        const h = listing(entries, [1]);
        expect((await neighbor(h.value, 201, -1, signal()))?.index).toBe(198);
        expect(h.loadPages.mock.calls.map(([pages]) => pages)).toEqual([[0]]);
        expect(await neighbor(h.value, 198, -1, signal())).toBeUndefined();
        expect(await neighbor(h.value, 201, 1, signal())).toBeUndefined();
        expect(loadedNeighbor(h.value, 198, -1)).toBeUndefined();
    });
    it("reads only as far as the named entry for a direct URL", async () => {
        const entries = Array.from({ length: 601 }, (_, index) => entry(`${index}.txt`));
        entries[401] = entry("deep.png", "image/png");
        const h = listing(entries);
        expect(await findEntry(h.value, "/a", "deep.png", signal())).toMatchObject({ index: 401 });
        expect(h.loadPages.mock.calls.map(([pages]) => pages)).toEqual([[1], [2]]);
        expect(await findEntry(h.value, "/a", "missing.png", signal())).toBeUndefined();
        expect(h.loadPages.mock.calls.map(([pages]) => pages)).toEqual([[1], [2], [3]]);
    });
    it("waits for the page before returning a neighboring image", async () => {
        const entries = Array.from({ length: 201 }, (_, index) => entry(`${index}.png`, "image/png"));
        const h = listing(entries);
        let release: (() => void) | undefined;
        h.loadPages.mockImplementationOnce(async (pages) => {
            await new Promise<void>((resolve) => {
                release = resolve;
            });
            pages.forEach((page) => {
                h.loaded.add(page);
            });
        });
        let result: unknown;
        const moving = neighbor(h.value, 199, 1, signal()).then((entry) => {
            result = entry;
        });
        expect(result).toBeUndefined();
        release!();
        await moving;
        expect(result).toMatchObject({ index: 200 });
    });
    it("rejects aborted and mixed-snapshot exploration", async () => {
        const entries = Array.from({ length: 201 }, (_, index) => entry(`${index}.png`, "image/png"));
        const h = listing(entries);
        const controller = new AbortController();
        controller.abort();
        await expect(findEntry(h.value, "/a", "200.png", controller.signal)).rejects.toMatchObject({
            name: "AbortError",
        });
        h.loadPages.mockImplementationOnce(async () => {
            h.value.snapshot = "two";
        });
        await expect(neighbor(h.value, 199, 1, signal())).rejects.toBeInstanceOf(ListingSnapshotChanged);
    });
});

describe("displayRequestSize", () => {
    it("multiplies the screen size by the pixel ratio", () => {
        expect(displayRequestSize(1280, 800, 2)).toEqual({ width: 2560, height: 1600 });
        expect(displayRequestSize(1280, 800, 1)).toEqual({ width: 1280, height: 800 });
    });

    it("treats a ratio below 1 as 1 and stays inside the API limits", () => {
        expect(displayRequestSize(1000, 500, 0.5)).toEqual({ width: 1000, height: 500 });
        expect(displayRequestSize(3000, 2000, 3)).toEqual({ width: 4096, height: 4096 });
        expect(displayRequestSize(0, 0, 1)).toEqual({ width: 1, height: 1 });
    });
});

describe("originalTiming", () => {
    const sized = (contentType: string | undefined, size: number): Entry => ({
        ...entry("a", contentType),
        metadata: { size, readonly: false, ...(contentType === undefined ? {} : { contentType }) },
    });

    it("fetches the original right away for a browser format under the limit when set to auto", () => {
        for (const type of ["jpeg", "png", "gif", "webp", "avif", "bmp"]) {
            expect(originalTiming(sized(`image/${type}`, 1), "auto")).toBe("immediate");
        }
        expect(originalTiming(sized("image/png", 30_000_000), "auto")).toBe("immediate");
    });

    it("waits for a zoom above the limit, or when set to onZoom", () => {
        expect(originalTiming(sized("image/png", 30_000_001), "auto")).toBe("onZoom");
        expect(originalTiming(sized("image/png", 1), "onZoom")).toBe("onZoom");
        expect(originalTiming(sized("image/png", 30_000_001), "onZoom")).toBe("onZoom");
    });

    it("never swaps a format the browser cannot show, whatever the setting", () => {
        for (const type of ["image/tiff", "image/heic", "image/svg+xml", "video/mp4", undefined]) {
            expect(originalTiming(sized(type, 1), "auto")).toBe("never");
            expect(originalTiming(sized(type, 1), "onZoom")).toBe("never");
        }
    });
});

describe("zoom", () => {
    it("zooms in when the wheel turns up and out when it turns down", () => {
        expect(zoomAfterWheel(1, -100)).toBeGreaterThan(1);
        expect(zoomAfterWheel(4, 100)).toBeLessThan(4);
    });

    it("stays between fit and the maximum", () => {
        expect(clampZoom(0.2)).toBe(1);
        expect(clampZoom(100)).toBe(MAX_ZOOM);
        expect(zoomAfterWheel(1, 500)).toBe(1);
    });

    it("keeps the pan inside the zoomed image", () => {
        const container = { width: 1000, height: 600 };
        expect(clampPan({ x: 9999, y: -9999 }, 3, container)).toEqual({ x: 1000, y: -600 });
        expect(clampPan({ x: 40, y: 10 }, 1, container)).toEqual({ x: 0, y: 0 });
    });
});

describe("original image", () => {
    it("computes the width of the image fitted into the container", () => {
        expect(fittedWidth({ width: 1000, height: 800 }, { width: 3000, height: 2000 })).toBe(1000);
        expect(fittedWidth({ width: 1000, height: 400 }, { width: 3000, height: 2000 })).toBe(600);
        expect(fittedWidth({ width: 1000, height: 400 }, { width: 0, height: 0 })).toBe(0);
    });

    it("is needed only once zoomed beyond the pixels of the display image", () => {
        const base = { fittedWidth: 1000, pixelRatio: 1, naturalWidth: 2048 };
        expect(needsOriginal({ ...base, zoom: 1 })).toBe(false);
        expect(needsOriginal({ ...base, zoom: 2 })).toBe(false);
        expect(needsOriginal({ ...base, zoom: 2.1 })).toBe(true);
        expect(needsOriginal({ ...base, zoom: 1.2, pixelRatio: 2 })).toBe(true);
    });
});

describe("zoomView", () => {
    const container = { width: 1000, height: 600 };

    it("pulls the pan back inside the image when zooming out partway", () => {
        // 拡大率 4 の限界（1500）まで動かした位置は、拡大率 2 では動かせる範囲（500）を超える。
        const zoomedOut = zoomView({ zoom: 4, pan: { x: 1500, y: -900 } }, 800, container);
        expect(zoomedOut.zoom).toBeGreaterThan(1);
        expect(zoomedOut.zoom).toBeLessThan(4);
        const limitX = (container.width * (zoomedOut.zoom - 1)) / 2;
        expect(zoomedOut.pan.x).toBeCloseTo(limitX);
        expect(Math.abs(zoomedOut.pan.y)).toBeLessThanOrEqual((container.height * (zoomedOut.zoom - 1)) / 2 + 1e-9);
    });

    it("resets the pan once the zoom returns to fit", () => {
        expect(zoomView({ zoom: 1.2, pan: { x: 50, y: 20 } }, 2000, container)).toEqual({
            zoom: 1,
            pan: { x: 0, y: 0 },
        });
    });

    it("keeps the pan while zooming in", () => {
        expect(zoomView({ zoom: 2, pan: { x: 100, y: 0 } }, -100, container).pan).toEqual({ x: 100, y: 0 });
    });
});

it("別のフォルダにある同名の画像を相対パスで探す", async () => {
    const h = listing([
        { ...entry("same.png", "image/png"), path: "/a/left/same.png" },
        { ...entry("same.png", "image/png"), path: "/a/right/same.png" },
    ]);
    expect(await findEntry(h.value, "/a", "right/same.png", signal())).toMatchObject({
        index: 1,
        entry: { path: "/a/right/same.png" },
    });
    expect(await findEntry(h.value, "/a", "same.png", signal())).toBeUndefined();
});
