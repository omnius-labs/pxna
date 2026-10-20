import { beforeEach, expect, it, vi } from "vitest";

import { fetchThumbnail, resizeFor, thumbnailSize } from "./api";

const mocks = vi.hoisted(() => ({ get: vi.fn() }));

vi.mock("../api/client", async (importOriginal) => ({
    ...(await importOriginal<typeof import("../api/client")>()),
    api: { GET: mocks.get },
}));

beforeEach(() => {
    mocks.get.mockReset();
    mocks.get.mockResolvedValue({
        data: new Blob(["x"]),
        response: new Response(null, { status: 200, headers: { "Pxna-Frame-Count": "8" } }),
    });
});

it("maps the thumbnail shape to the resize parameter", () => {
    expect(resizeFor("crop")).toBe("crop");
    expect(resizeFor("fit")).toBe("fit");
});

it.each([
    ["crop", "crop"],
    ["fit", "fit"],
] as const)(
    "requests a %s thumbnail with resize=%s and the same size for the width and the height",
    async (shape, resize) => {
        await fetchThumbnail("photos", "/a.jpg", 352, "thumbnail", shape, new AbortController().signal);
        expect(mocks.get.mock.calls[0]?.[1]?.params?.query).toMatchObject({
            path: "/a.jpg",
            variant: "thumbnail",
            width: 352,
            height: 352,
            resize,
        });
    },
);

it("asks for the sprite sheet with the same shape, and reads the frame count", async () => {
    const result = await fetchThumbnail("photos", "/a.mp4", 352, "sprite", "fit", new AbortController().signal);
    expect(mocks.get.mock.calls[0]?.[1]?.params?.query).toMatchObject({ variant: "sprite", resize: "fit" });
    expect(result.frameCount).toBe(8);
});

it("multiplies the tile size by the pixel ratio, and never below one", () => {
    expect(thumbnailSize(176, 2)).toBe(352);
    expect(thumbnailSize(176, 0.5)).toBe(176);
});
