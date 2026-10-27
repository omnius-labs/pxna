// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";

import { VideoPlayer } from "./VideoPlayer";

const { get, record } = vi.hoisted(() => ({ get: vi.fn(), record: vi.fn() }));
vi.mock("../api/clientTimings", () => ({ clientTimings: { record } }));
vi.mock("../api/client", async (original) => ({
    ...(await original<typeof import("../api/client")>()),
    api: { GET: get },
}));
afterEach(() => {
    get.mockReset();
    record.mockReset();
});

async function render(onDone: () => void) {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    await act(async () =>
        root.render(
            <QueryClientProvider client={new QueryClient()}>
                <VideoPlayer libraryId="big" path="/clip.mp4" onDone={onDone} />
            </QueryClientProvider>,
        ),
    );
    return {
        host,
        dispose: async () => {
            await act(async () => root.unmount());
            host.remove();
        },
    };
}

it("最後まで再生したら onDone を呼ぶ", async () => {
    get.mockReturnValue(new Promise(() => {}));
    const onDone = vi.fn();
    const h = await render(onDone);
    try {
        expect(onDone).not.toHaveBeenCalled();
        await act(async () => h.host.querySelector("video")!.dispatchEvent(new Event("ended")));
        expect(onDone).toHaveBeenCalledOnce();
    } finally {
        await h.dispose();
    }
});

it("動画の情報を読めなかったときも onDone を呼ぶ", async () => {
    get.mockResolvedValue({ error: { kind: "NotFound" }, response: new Response(null, { status: 404 }) });
    const onDone = vi.fn();
    const h = await render(onDone);
    try {
        await vi.waitFor(() => expect(onDone).toHaveBeenCalledOnce());
        expect(h.host.querySelector("[role=alert]")).not.toBeNull();
    } finally {
        await h.dispose();
    }
});

it("最初のフレームが出たら video_start を 1 回だけ記録する", async () => {
    get.mockReturnValue(new Promise(() => {}));
    const h = await render(vi.fn());
    try {
        const video = h.host.querySelector("video")!;
        await act(async () => video.dispatchEvent(new Event("playing")));
        await act(async () => video.dispatchEvent(new Event("playing")));
        expect(record).toHaveBeenCalledOnce();
        expect(record.mock.calls[0]?.[0]).toMatchObject({
            operation: "video_start",
            libraryId: "big",
            path: "/clip.mp4",
        });
    } finally {
        await h.dispose();
    }
});

it("再生が始まらないまま閉じたときは video_start を記録しない", async () => {
    get.mockReturnValue(new Promise(() => {}));
    const h = await render(vi.fn());
    await h.dispose();
    expect(record).not.toHaveBeenCalled();
});
