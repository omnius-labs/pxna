import { QueryClient } from "@tanstack/react-query";
import { afterEach, expect, it, vi } from "vitest";

import { mediaInfoQuery } from "./queries";

const mocks = vi.hoisted(() => ({ get: vi.fn() }));
vi.mock("../api/client", async (importOriginal) => ({
    ...(await importOriginal<typeof import("../api/client")>()),
    api: { GET: mocks.get },
}));
afterEach(() => mocks.get.mockReset());

it("メディア情報の問い合わせは、Library の id と path で要求し、応答の mediaInfo を返す", async () => {
    const mediaInfo = { kind: "video", video: { videoCodec: "h264" } };
    mocks.get.mockResolvedValue({ data: { mediaInfo }, response: new Response() });
    const client = new QueryClient();
    const result = await client.fetchQuery(mediaInfoQuery("photos", "/trip/clip.mp4"));
    expect(result).toBe(mediaInfo);
    expect(mocks.get).toHaveBeenCalledOnce();
    const [path, options] = mocks.get.mock.calls[0] ?? [];
    expect(path).toBe("/libraries/{libraryId}/media-info");
    expect(options.params).toEqual({ path: { libraryId: "photos" }, query: { path: "/trip/clip.mp4" } });
    expect(options.signal).toBeInstanceOf(AbortSignal);
});

it("キーは、動画の再生と情報パネルで共有する形で、Library の id と path から決まる", () => {
    expect(mediaInfoQuery("photos", "/a.mp4").queryKey).toEqual(["media-info", "photos", "/a.mp4"]);
    expect(mediaInfoQuery("photos", "/a.mp4").queryKey).toEqual(mediaInfoQuery("photos", "/a.mp4").queryKey);
    expect(mediaInfoQuery("photos", "/b.mp4").queryKey).not.toEqual(mediaInfoQuery("photos", "/a.mp4").queryKey);
});
