import { describe, expect, it, vi } from "vitest";

import type { ClientMessage, FrameHeader, WantedItem } from "../api/imageSocket";
import { MemoryImageCache } from "../images/cache";
import { encodeFrame } from "../images/protocol";
import { type ImageSpec, specKey, type Want } from "../images/spec";
import { ImageSocketClient, type ImageSocketEnvironment, type SettledImage } from "./image";

const ETAG = "a".repeat(64);
const OTHER_ETAG = "b".repeat(64);

class FakeSocket {
    binaryType: BinaryType = "blob";
    onopen: (() => void) | null = null;
    onmessage: ((event: { data: unknown }) => void) | null = null;
    onclose: ((event: { code: number }) => void) | null = null;
    onerror: (() => void) | null = null;
    readonly sent: string[] = [];
    closeCode: number | undefined;

    send(data: string): void {
        this.sent.push(data);
    }

    close(code?: number): void {
        this.closeCode = code;
    }

    /// 送られた message を、順に読み戻す。
    messages(): ClientMessage[] {
        return this.sent.map((text) => JSON.parse(text) as ClientMessage);
    }

    wantedItems(): WantedItem[] {
        return this.messages().flatMap((message) => (message.type === "want" ? message.items : []));
    }

    open(): void {
        this.onopen?.();
    }

    text(message: unknown): void {
        this.onmessage?.({ data: JSON.stringify(message) });
    }

    frame(header: Partial<FrameHeader> & { id: number }, chunk: number[]): void {
        const full: FrameHeader = {
            etag: ETAG,
            contentType: "image/webp",
            frameCount: null,
            offset: 0,
            total: chunk.length,
            ...header,
        };
        this.onmessage?.({ data: encodeFrame(full, new Uint8Array(chunk)) });
    }

    serverClose(code: number): void {
        this.onclose?.({ code });
    }
}

function harness(cache = new MemoryImageCache()) {
    const sockets: FakeSocket[] = [];
    const timers: Array<{ callback: () => void; ms: number; cancelled: boolean }> = [];
    const urls = new Map<string, Blob>();
    const revoked: string[] = [];
    const settled: SettledImage[] = [];
    const sessionEnded = vi.fn();
    let urlCount = 0;
    let clock = 0;
    const env: ImageSocketEnvironment = {
        open: () => {
            const socket = new FakeSocket();
            sockets.push(socket);
            return socket as unknown as WebSocket;
        },
        cache,
        createUrl: (blob) => {
            const url = `blob:${++urlCount}`;
            urls.set(url, blob);
            return url;
        },
        revokeUrl: (url) => revoked.push(url),
        now: () => (clock += 10),
        random: () => 0.5,
        setTimeout: (callback, ms) => {
            const timer = { callback, ms, cancelled: false };
            timers.push(timer);
            return timer;
        },
        clearTimeout: (handle) => {
            if (handle) (handle as { cancelled: boolean }).cancelled = true;
        },
        onSessionEnded: sessionEnded,
        onSettled: (image) => settled.push(image),
    };
    const client = new ImageSocketClient(env);
    return {
        client,
        cache,
        sockets,
        timers,
        urls,
        revoked,
        settled,
        sessionEnded,
        socket: () => {
            const socket = sockets.at(-1);
            if (!socket) throw new Error("no socket was opened");
            return socket;
        },
        /// 待っている再接続を実行する。
        fireReconnect: () => {
            const timer = timers.filter((candidate) => !candidate.cancelled).at(-1);
            if (!timer) throw new Error("no reconnect is waiting");
            timer.cancelled = true;
            timer.callback();
            return timer.ms;
        },
    };
}

const spec = (path: string, kind: ImageSpec["kind"] = "thumbnail"): ImageSpec => ({
    libraryId: "photos",
    path,
    kind,
    width: 256,
    height: 256,
    shape: "crop",
});

const want = (
    path: string,
    wantClass: Want["class"] = "visible",
    order = 0,
    kind: ImageSpec["kind"] = "thumbnail",
): [string, Want] => {
    const s = spec(path, kind);
    return [specKey(s), { spec: s, class: wantClass, order }];
};

const wants = (...entries: [string, Want][]) => new Map(entries);
/// 非同期の置き場の読み出しと、まとめて送る処理が終わるのを待つ。
const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

describe("ImageSocketClient", () => {
    it("opens no socket until something is wanted, and sends everything wanted in one message once it is open", async () => {
        const h = harness();
        expect(h.sockets).toHaveLength(0);
        const owner = {};
        h.client.setWants(owner, wants(want("/a.jpg", "visible", 1), want("/b.jpg", "prefetch", 2)));
        await settle();
        expect(h.sockets).toHaveLength(1);
        expect(h.socket().binaryType).toBe("arraybuffer");
        expect(h.socket().sent).toEqual([]);

        h.socket().open();
        expect(h.socket().messages()).toEqual([
            {
                type: "want",
                items: [
                    {
                        id: 1,
                        libraryId: "photos",
                        path: "/a.jpg",
                        kind: "thumbnail",
                        width: 256,
                        height: 256,
                        shape: "crop",
                        class: "visible",
                        order: 1,
                    },
                    {
                        id: 2,
                        libraryId: "photos",
                        path: "/b.jpg",
                        kind: "thumbnail",
                        width: 256,
                        height: 256,
                        shape: "crop",
                        class: "prefetch",
                        order: 2,
                    },
                ],
            },
        ]);
        expect(h.client.getState(spec("/a.jpg")).status).toBe("loading");
    });

    it("merges what several parts want of one image into one item with the most urgent class and the smallest order", async () => {
        const h = harness();
        const [listing, filmstrip] = [{}, {}];
        h.client.setWants(listing, wants(want("/a.jpg", "prefetch", 5)));
        h.client.setWants(filmstrip, wants(want("/a.jpg", "visible", 9)));
        await settle();
        h.socket().open();
        expect(h.socket().wantedItems()).toHaveLength(1);
        expect(h.socket().wantedItems()[0]).toMatchObject({ class: "visible", order: 9 });

        // A part that wants it more urgently raises the item, and the item is sent again under the same id.
        const viewer = {};
        h.client.setWants(viewer, wants(want("/a.jpg", "visible", 2)));
        await settle();
        expect(h.socket().wantedItems().at(-1)).toMatchObject({ id: 1, class: "visible", order: 2 });

        // When that part lets go, the item falls back to what the others want.
        h.client.release(viewer);
        await settle();
        expect(h.socket().wantedItems().at(-1)).toMatchObject({ id: 1, class: "visible", order: 9 });
        expect(
            h
                .socket()
                .wantedItems()
                .map((item) => item.id),
        ).toEqual([1, 1, 1]);
    });

    it("puts an image together from its chunks, shows it, and keeps it for the next time with its etag", async () => {
        const h = harness();
        h.client.setWants({}, wants(want("/a.jpg")));
        await settle();
        h.socket().open();

        h.socket().frame({ id: 1, offset: 0, total: 5 }, [1, 2, 3]);
        expect(h.client.getState(spec("/a.jpg")).status).toBe("loading");
        h.socket().frame({ id: 1, offset: 3, total: 5 }, [4, 5]);
        const state = h.client.getState(spec("/a.jpg"));
        expect(state).toEqual({ status: "ready", url: "blob:1", frameCount: null });
        const blob = h.urls.get("blob:1");
        expect(blob?.type).toBe("image/webp");
        const bytes = await blob?.arrayBuffer();
        expect(bytes && new Uint8Array(bytes)).toEqual(new Uint8Array([1, 2, 3, 4, 5]));
        expect(h.settled).toMatchObject([{ spec: spec("/a.jpg"), bytes: 5, cached: false }]);

        await settle();
        const cached = await h.cache.get(specKey(spec("/a.jpg")));
        expect(cached?.etag).toBe(ETAG);
        expect(new Uint8Array(cached?.data ?? new ArrayBuffer(0))).toEqual(new Uint8Array([1, 2, 3, 4, 5]));
    });

    it("carries the frame count of a sprite sheet, and ignores frames out of order or for items it does not know", async () => {
        const h = harness();
        h.client.setWants({}, wants(want("/v.mp4", "visible", 0, "sprite")));
        await settle();
        h.socket().open();

        h.socket().frame({ id: 99, offset: 0, total: 1 }, [1]);
        // A chunk that does not continue the image is dropped, and the image is never put together from it.
        h.socket().frame({ id: 1, offset: 4, total: 8, frameCount: 8 }, [1, 2, 3, 4]);
        expect(h.client.getState(spec("/v.mp4", "sprite")).status).toBe("loading");
        h.socket().frame({ id: 1, offset: 0, total: 4, frameCount: 8 }, [1, 2, 3, 4]);
        expect(h.client.getState(spec("/v.mp4", "sprite"))).toMatchObject({ status: "ready", frameCount: 8 });
    });

    it("shows a kept image at once, wants it with its etag, and keeps it when the daemon says it is unchanged", async () => {
        const cache = new MemoryImageCache();
        await cache.put(specKey(spec("/a.jpg")), "thumbnail", {
            data: new Uint8Array([9, 9]).buffer,
            contentType: "image/webp",
            etag: ETAG,
            frameCount: null,
        });
        const h = harness(cache);
        h.client.setWants({}, wants(want("/a.jpg")));
        await settle();
        const shown = h.client.getState(spec("/a.jpg"));
        expect(shown).toMatchObject({ status: "ready", url: "blob:1" });
        h.socket().open();
        expect(h.socket().wantedItems()[0]).toMatchObject({ id: 1, etag: ETAG });

        h.socket().text({ type: "unchanged", id: 1 });
        expect(h.client.getState(spec("/a.jpg"))).toBe(shown);
        expect(h.revoked).toEqual([]);
        expect(h.settled).toMatchObject([{ cached: true }]);
    });

    it("replaces a kept image with the new one when the daemon sends it, and releases the old URL", async () => {
        const cache = new MemoryImageCache();
        await cache.put(specKey(spec("/a.jpg")), "thumbnail", {
            data: new Uint8Array([9]).buffer,
            contentType: "image/webp",
            etag: ETAG,
            frameCount: null,
        });
        const h = harness(cache);
        h.client.setWants({}, wants(want("/a.jpg")));
        await settle();
        h.socket().open();
        h.socket().frame({ id: 1, etag: OTHER_ETAG, total: 2 }, [1, 2]);

        expect(h.client.getState(spec("/a.jpg"))).toMatchObject({ status: "ready", url: "blob:2" });
        expect(h.revoked).toEqual(["blob:1"]);
        await settle();
        expect((await cache.get(specKey(spec("/a.jpg"))))?.etag).toBe(OTHER_ETAG);
    });

    it("does not keep an image that has no etag, because it could not be checked later", async () => {
        const h = harness();
        h.client.setWants({}, wants(want("/a.jpg")));
        await settle();
        h.socket().open();
        h.socket().frame({ id: 1, etag: null, total: 1 }, [1]);
        await settle();
        expect(h.client.getState(spec("/a.jpg")).status).toBe("ready");
        expect(h.cache.size).toBe(0);
    });

    it("fails an item with the kind the daemon gives, and does not ask again while a part still wants it", async () => {
        const h = harness();
        const owner = {};
        h.client.setWants(owner, wants(want("/a.jpg"), want("/b.jpg")));
        await settle();
        h.socket().open();
        h.socket().text({ type: "failed", id: 1, kind: "Timeout" });
        expect(h.client.getState(spec("/a.jpg"))).toEqual({ status: "failed", kind: "Timeout" });

        // The tile leaves and comes back while its part lives: the failure is remembered, and nothing is sent.
        h.client.setWants(owner, wants(want("/b.jpg")));
        h.client.setWants(owner, wants(want("/a.jpg"), want("/b.jpg")));
        await settle();
        expect(
            h
                .socket()
                .wantedItems()
                .filter((item) => item.path === "/a.jpg"),
        ).toHaveLength(1);

        // When the part is gone, the failure is forgotten, so the next part asks again.
        h.client.release(owner);
        expect(h.client.getState(spec("/a.jpg")).status).toBe("idle");
        h.client.setWants({}, wants(want("/a.jpg")));
        await settle();
        expect(
            h
                .socket()
                .wantedItems()
                .filter((item) => item.path === "/a.jpg"),
        ).toHaveLength(2);
    });

    it("forgets a kept image the daemon says is gone, and keeps one it only failed to refresh", async () => {
        const cache = new MemoryImageCache();
        for (const path of ["/gone.jpg", "/slow.jpg"]) {
            await cache.put(specKey(spec(path)), "thumbnail", {
                data: new Uint8Array([1]).buffer,
                contentType: "image/webp",
                etag: ETAG,
                frameCount: null,
            });
        }
        const h = harness(cache);
        h.client.setWants({}, wants(want("/gone.jpg"), want("/slow.jpg")));
        await settle();
        h.socket().open();
        h.socket().text({ type: "failed", id: 1, kind: "NotFound" });
        h.socket().text({ type: "failed", id: 2, kind: "Timeout" });
        await settle();

        expect(h.client.getState(spec("/gone.jpg"))).toEqual({ status: "failed", kind: "NotFound" });
        expect(h.revoked).toEqual(["blob:1"]);
        expect(await cache.get(specKey(spec("/gone.jpg")))).toBeUndefined();
        expect(h.client.getState(spec("/slow.jpg")).status).toBe("ready");
        expect(await cache.get(specKey(spec("/slow.jpg")))).toBeDefined();
    });

    it("drops an item the daemon has not answered when no part wants it, and releases an image nobody wants", async () => {
        const h = harness();
        const owner = {};
        h.client.setWants(owner, wants(want("/a.jpg"), want("/b.jpg")));
        await settle();
        h.socket().open();
        h.socket().frame({ id: 2, total: 1 }, [1]);
        expect(h.client.getState(spec("/b.jpg")).status).toBe("ready");

        h.client.setWants(owner, wants());
        await settle();
        expect(h.socket().messages().at(-1)).toEqual({ type: "drop", ids: [1] });
        expect(h.revoked).toEqual(["blob:1"]);
        expect(h.client.getState(spec("/a.jpg")).status).toBe("idle");
        expect(h.client.getState(spec("/b.jpg")).status).toBe("idle");
        // Frames for the dropped item are ignored.
        h.socket().frame({ id: 1, total: 1 }, [1]);
        expect(h.client.itemCount).toBe(0);
    });

    it("splits a large want into messages the daemon accepts", async () => {
        const h = harness();
        const entries = Array.from({ length: 450 }, (_, index) => want(`/${index}.jpg`, "visible", index));
        h.client.setWants({}, new Map(entries));
        await settle();
        h.socket().open();
        const messages = h.socket().messages();
        expect(messages.map((message) => (message.type === "want" ? message.items.length : 0))).toEqual([200, 200, 50]);
        expect(
            new Set(
                h
                    .socket()
                    .wantedItems()
                    .map((item) => item.id),
            ).size,
        ).toBe(450);
    });

    it("wants again under new names after the connection is lost, but not the images it already has", async () => {
        const h = harness();
        h.client.setWants({}, wants(want("/a.jpg"), want("/b.jpg")));
        await settle();
        h.socket().open();
        h.socket().frame({ id: 1, total: 1 }, [1]);
        const first = h.socket();

        first.serverClose(1006);
        expect(h.sockets).toHaveLength(1);
        // 1 second, with a jitter around it.
        const wait = h.fireReconnect();
        expect(wait).toBeGreaterThanOrEqual(750);
        expect(wait).toBeLessThanOrEqual(1250);
        expect(h.sockets).toHaveLength(2);
        h.socket().open();
        expect(
            h
                .socket()
                .wantedItems()
                .map((item) => [item.id, item.path]),
        ).toEqual([[3, "/b.jpg"]]);
        expect(h.client.getState(spec("/a.jpg")).status).toBe("ready");
    });

    it("waits twice as long for each failed attempt up to 30 seconds, and starts over once it is connected", async () => {
        const h = harness();
        h.client.setWants({}, wants(want("/a.jpg")));
        await settle();
        const waits: number[] = [];
        for (let attempt = 0; attempt < 8; attempt += 1) {
            h.socket().serverClose(1006);
            waits.push(h.fireReconnect());
        }
        expect(waits.map((wait) => Math.round(wait / 100) * 100)).toEqual([
            1000, 2000, 4000, 8000, 16_000, 30_000, 30_000, 30_000,
        ]);

        h.socket().open();
        h.socket().serverClose(1006);
        expect(Math.round(h.fireReconnect() / 100) * 100).toBe(1000);
    });

    it("does not reconnect while nothing waits for the daemon", async () => {
        const h = harness();
        h.client.setWants({}, wants(want("/a.jpg")));
        await settle();
        h.socket().open();
        h.socket().frame({ id: 1, total: 1 }, [1]);
        h.socket().serverClose(1001);
        expect(h.timers.filter((timer) => !timer.cancelled)).toHaveLength(0);
        // A new want connects again at once.
        h.client.setWants({}, wants(want("/b.jpg")));
        await settle();
        expect(h.sockets).toHaveLength(2);
    });

    it("reports a session that ended, and does not reconnect by itself", async () => {
        const h = harness();
        h.client.setWants({}, wants(want("/a.jpg")));
        await settle();
        h.socket().open();
        h.socket().serverClose(4401);
        expect(h.sessionEnded).toHaveBeenCalledTimes(1);
        expect(h.timers.filter((timer) => !timer.cancelled)).toHaveLength(0);
    });

    it("fails what it is waiting for when the daemon says the client broke the contract, and never connects again", async () => {
        const h = harness();
        const owner = {};
        h.client.setWants(owner, wants(want("/a.jpg")));
        await settle();
        h.socket().open();
        h.socket().frame({ id: 1, total: 1 }, [1]);
        h.client.setWants(owner, wants(want("/a.jpg"), want("/b.jpg")));
        await settle();
        h.socket().serverClose(4400);
        expect(h.client.getState(spec("/b.jpg"))).toEqual({ status: "failed", kind: "InvalidRequest" });
        expect(h.client.getState(spec("/a.jpg")).status).toBe("ready");

        h.client.setWants(owner, wants(want("/a.jpg"), want("/b.jpg"), want("/c.jpg")));
        await settle();
        expect(h.sockets).toHaveLength(1);
        expect(h.client.getState(spec("/c.jpg"))).toEqual({ status: "failed", kind: "InvalidRequest" });
    });

    it("forgets everything, closes the socket, and empties the kept images on clear, and connects again for a later want", async () => {
        const h = harness();
        h.client.setWants({}, wants(want("/a.jpg")));
        await settle();
        h.socket().open();
        h.socket().frame({ id: 1, total: 1 }, [1]);
        await settle();
        expect(h.cache.size).toBe(1);

        await h.client.clear();
        expect(h.socket().closeCode).toBe(1000);
        expect(h.revoked).toEqual(["blob:1"]);
        expect(h.cache.size).toBe(0);
        expect(h.client.itemCount).toBe(0);
        expect(h.client.getState(spec("/a.jpg")).status).toBe("idle");

        h.client.setWants({}, wants(want("/b.jpg")));
        await settle();
        expect(h.sockets).toHaveLength(2);
    });

    it("tells its subscribers when a state changes", async () => {
        const h = harness();
        const listener = vi.fn();
        h.client.subscribe(listener);
        h.client.setWants({}, wants(want("/a.jpg")));
        expect(listener).toHaveBeenCalledTimes(1);
        await settle();
        h.socket().open();
        h.socket().frame({ id: 1, total: 1 }, [1]);
        expect(listener.mock.calls.length).toBeGreaterThanOrEqual(2);
    });
});
