import { IDBFactory, IDBKeyRange } from "fake-indexeddb";
import { describe, expect, it } from "vitest";

import {
    type CachedImage,
    type CacheGroup,
    cacheGroupOf,
    type ImageCache,
    IndexedDbImageCache,
    MemoryImageCache,
} from "./cache";

// IndexedDB の範囲指定は、ブラウザではグローバルにある。
Object.assign(globalThis, { IDBKeyRange });

const image = (bytes: number, etag = "e"): CachedImage => ({
    data: new Uint8Array(bytes).fill(7).buffer,
    contentType: "image/webp",
    etag,
    frameCount: null,
});

const budgets: Record<CacheGroup, number> = { thumbnail: 100, display: 300 };

/// どちらの置き場も、同じ約束を守る。
const caches: Array<[string, () => ImageCache]> = [
    ["memory", () => new MemoryImageCache(budgets)],
    ["IndexedDB", () => new IndexedDbImageCache(new IDBFactory(), budgets)],
];

describe.each(caches)("%s image cache", (_name, create) => {
    it("returns what was put, with its etag and frame count, and nothing for an unknown key", async () => {
        const cache = create();
        await cache.put("a", "thumbnail", { ...image(10, "etag-a"), frameCount: 8 });
        const found = await cache.get("a");
        expect(found?.etag).toBe("etag-a");
        expect(found?.frameCount).toBe(8);
        expect(found?.contentType).toBe("image/webp");
        expect(new Uint8Array(found?.data ?? new ArrayBuffer(0))).toEqual(new Uint8Array(10).fill(7));
        expect(await cache.get("missing")).toBeUndefined();
    });

    it("replaces an image put again under the same key, without counting both against the budget", async () => {
        const cache = create();
        await cache.put("a", "thumbnail", image(60, "old"));
        await cache.put("a", "thumbnail", image(60, "new"));
        await cache.put("b", "thumbnail", image(30));
        expect((await cache.get("a"))?.etag).toBe("new");
        expect(await cache.get("b")).toBeDefined();
    });

    it("drops the image used least recently when a group is over its budget, and never the one just put", async () => {
        const cache = create();
        await cache.put("a", "thumbnail", image(40));
        await cache.put("b", "thumbnail", image(40));
        await cache.get("a");
        await cache.put("c", "thumbnail", image(40));

        expect(await cache.get("b")).toBeUndefined();
        expect(await cache.get("a")).toBeDefined();
        expect(await cache.get("c")).toBeDefined();
    });

    it("keeps each group within its own budget, so a display image does not push out thumbnails", async () => {
        const cache = create();
        await cache.put("thumb", "thumbnail", image(90));
        await cache.put("display-1", "display", image(200));
        await cache.put("display-2", "display", image(200));

        expect(await cache.get("thumb")).toBeDefined();
        expect(await cache.get("display-1")).toBeUndefined();
        expect(await cache.get("display-2")).toBeDefined();
    });

    it("keeps an image bigger than its budget just long enough to be the newest, and removes it on the next put", async () => {
        const cache = create();
        await cache.put("big", "thumbnail", image(150));
        expect(await cache.get("big")).toBeDefined();
        await cache.put("small", "thumbnail", image(10));
        expect(await cache.get("big")).toBeUndefined();
        expect(await cache.get("small")).toBeDefined();
    });

    it("forgets a deleted image, and everything on clear", async () => {
        const cache = create();
        await cache.put("a", "thumbnail", image(10));
        await cache.put("b", "display", image(10));
        await cache.delete("a");
        expect(await cache.get("a")).toBeUndefined();
        expect(await cache.get("b")).toBeDefined();

        await cache.clear();
        expect(await cache.get("b")).toBeUndefined();
        // The budget counts from zero again.
        await cache.put("c", "thumbnail", image(90));
        expect(await cache.get("c")).toBeDefined();
    });
});

describe("IndexedDbImageCache", () => {
    it("keeps images for another cache that opens the same database, as after a reload", async () => {
        const factory = new IDBFactory();
        await new IndexedDbImageCache(factory, budgets).put("a", "thumbnail", image(10, "kept"));

        const reopened = new IndexedDbImageCache(factory, budgets);
        expect((await reopened.get("a"))?.etag).toBe("kept");
    });

    it("counts what is already stored against the budget after a reload", async () => {
        const factory = new IDBFactory();
        const first = new IndexedDbImageCache(factory, budgets);
        await first.put("old", "thumbnail", image(80));

        const reopened = new IndexedDbImageCache(factory, budgets);
        await reopened.put("new", "thumbnail", image(80));
        expect(await reopened.get("old")).toBeUndefined();
        expect(await reopened.get("new")).toBeDefined();
    });

    it("runs operations that overlap one after another", async () => {
        const cache = new IndexedDbImageCache(new IDBFactory(), budgets);
        await Promise.all(Array.from({ length: 20 }, (_, index) => cache.put(`k${index}`, "thumbnail", image(10))));
        const kept = (await Promise.all(Array.from({ length: 20 }, (_, index) => cache.get(`k${index}`)))).filter(
            Boolean,
        );
        // 10 images of 10 bytes fit in the 100 byte budget.
        expect(kept).toHaveLength(10);
    });
});

describe("cacheGroupOf", () => {
    it("keeps display images apart from thumbnails and sprite sheets", () => {
        expect(cacheGroupOf("display")).toBe("display");
        expect(cacheGroupOf("thumbnail")).toBe("thumbnail");
        expect(cacheGroupOf("sprite")).toBe("thumbnail");
    });
});
