import { describe, expect, it, vi } from "vitest";

const { post, remove } = vi.hoisted(() => ({ post: vi.fn(), remove: vi.fn() }));
vi.mock("../api/client", () => ({
    api: { POST: post, DELETE: remove },
    unwrap: (result: { data: unknown }) => result.data,
}));

import { acquireVideoSession, playbackMode, type VideoSession } from "./video";

const session = (id: string): VideoSession => ({
    id,
    libraryId: "test",
    path: "/clip.mkv",
    playlistUrl: `/video/${id}`,
    durationSecs: 10,
});
const turn = async () => {
    await new Promise<void>((resolve) => setTimeout(resolve, 0));
};

describe("playback selection", () => {
    it("uses codec capability, then native HLS, then MSE", () => {
        const mime = 'video/mp4; codecs="avc1.640028,mp4a.40.2"';
        expect(playbackMode(mime, (type) => (type === mime ? "probably" : ""), true)).toBe("direct");
        expect(playbackMode(mime, (type) => (type.includes("mpegurl") ? "maybe" : ""), true)).toBe("native-hls");
        expect(playbackMode(mime, () => "", true)).toBe("hls-js");
        expect(playbackMode(mime, () => "", false)).toBe("unsupported");
    });
});

describe("session lifetime", () => {
    it("reuses StrictMode reattachment and deletes once after the last release", async () => {
        post.mockResolvedValueOnce({ data: { session: session("strict") } });
        remove.mockResolvedValue({});
        const first = acquireVideoSession("test", "/strict.mkv");
        first.release();
        const second = acquireVideoSession("test", "/strict.mkv");
        expect(second.session).toBe(first.session);
        await second.session;
        await turn();
        expect(remove).not.toHaveBeenCalled();
        second.release();
        second.release();
        await turn();
        expect(post).toHaveBeenCalledTimes(1);
        expect(remove).toHaveBeenCalledTimes(1);
        post.mockClear();
        remove.mockClear();
    });

    it("ends a closed pending creation before creating the newer video", async () => {
        let resolve!: (value: { data: { session: VideoSession } }) => void;
        post.mockImplementationOnce(
            () =>
                new Promise((done) => {
                    resolve = done;
                }),
        );
        post.mockResolvedValueOnce({ data: { session: session("new") } });
        const old = acquireVideoSession("test", "/old.mkv");
        await turn();
        old.release();
        const next = acquireVideoSession("test", "/new.mkv");
        await turn();
        expect(post).toHaveBeenCalledTimes(1);
        resolve({ data: { session: session("old") } });
        expect((await next.session).id).toBe("new");
        await turn();
        expect(remove).toHaveBeenCalledWith(
            expect.any(String),
            expect.objectContaining({ params: { path: { libraryId: "test", sessionId: "old" } } }),
        );
        next.release();
        await turn();
        expect(remove).toHaveBeenCalledTimes(2);
        post.mockClear();
        remove.mockClear();
    });

    it("avoids a queued creation when already closed", async () => {
        const resource = acquireVideoSession("test", "/closed.mkv");
        resource.release();
        await expect(resource.session).rejects.toMatchObject({ name: "AbortError" });
        await turn();
        expect(post).not.toHaveBeenCalled();
        expect(remove).not.toHaveBeenCalled();
    });
});
