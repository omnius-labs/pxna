import { describe, expect, it, vi } from "vitest";

import { FakeImageSource } from "../testing/images";
import { type ImageSpec, specKey } from "./spec";
import { ImageWants, shapeFor, thumbnailSize, thumbnailSpec } from "./wants";

const spec = (path: string): ImageSpec => ({
    libraryId: "photos",
    path,
    kind: "thumbnail",
    width: 352,
    height: 352,
    shape: "crop",
});

function wantsWith(source = new FakeImageSource()) {
    return { source, wants: new ImageWants(source, spec) };
}

const wanted = (...entries: [string, number][]) => new Map(entries);
/// 欲しいと言われた画像を、パスの並びで返す。順は急ぎ度の順にする。
const paths = (source: FakeImageSource) =>
    source
        .wanted()
        .sort((a, b) => (a.class === b.class ? a.order - b.order : a.class < b.class ? 1 : -1))
        .map((want) => want.spec.path);

describe("ImageWants", () => {
    it("wants the paths it is given, as visible images ordered by the given numbers", () => {
        const { source, wants } = wantsWith();
        wants.setWanted(wanted(["b", 5], ["a", 1]));
        expect(source.wanted().map((want) => [want.spec.path, want.class, want.order])).toEqual([
            ["b", "visible", 5],
            ["a", "visible", 1],
        ]);
        expect(source.wanted()[0]?.spec).toEqual(spec("b"));
    });

    it("wants the paths it is told are expected soon as prefetch", () => {
        const { source, wants } = wantsWith();
        wants.setWanted(wanted(["on-screen", 0], ["margin", 1000]), new Set(), new Set(["margin"]));
        expect(Object.fromEntries(source.wanted().map((want) => [want.spec.path, want.class]))).toEqual({
            "on-screen": "visible",
            margin: "prefetch",
        });
    });

    it("stops wanting a path that leaves the wanted set", () => {
        const { source, wants } = wantsWith();
        wants.setWanted(wanted(["a", 0], ["b", 1], ["c", 2]));
        wants.setWanted(wanted(["c", 2], ["d", 3]));
        expect(paths(source)).toEqual(["c", "d"]);
    });

    it("reads the state of a path from the source, with the spec of that path", () => {
        const { source, wants } = wantsWith();
        expect(wants.getState("a").status).toBe("idle");
        source.ready(spec("a"), "blob:a");
        expect(wants.getState("a")).toEqual({ status: "ready", url: "blob:a", frameCount: null });
        let notified = 0;
        wants.subscribe(() => (notified += 1));
        source.ready(spec("b"), "blob:b");
        expect(notified).toBe(1);
    });

    it("delays visible folder/archive requests without delaying image/video prefetch or restarting deadlines", async () => {
        vi.useFakeTimers();
        const { source, wants } = wantsWith();
        try {
            const requests = wanted(["folder", 0], ["archive", 1], ["image-prefetch", 8], ["video-prefetch", 9]);
            const delayed = new Set(["folder", "archive"]);
            wants.setWanted(requests, delayed);
            await Promise.resolve();
            expect(paths(source)).toEqual(["image-prefetch", "video-prefetch"]);
            vi.advanceTimersByTime(100);
            wants.setWanted(requests, delayed);
            await Promise.resolve();
            vi.advanceTimersByTime(99);
            expect(paths(source)).toEqual(["image-prefetch", "video-prefetch"]);
            vi.advanceTimersByTime(1);
            expect(paths(source)).toEqual(["folder", "archive", "image-prefetch", "video-prefetch"]);
            wants.setWanted(wanted(["image-prefetch", 0], ["video-prefetch", 1]));
            expect(paths(source)).toEqual(["image-prefetch", "video-prefetch"]);
        } finally {
            wants.dispose();
            vi.useRealTimers();
        }
    });

    it("cancels both pending timers and queued microtasks on leaving or disposal and starts a fresh delay on return", async () => {
        vi.useFakeTimers();
        const { source, wants } = wantsWith();
        try {
            wants.setWanted(wanted(["a", 0]), new Set(["a"]));
            wants.setWanted(wanted());
            await Promise.resolve();
            vi.advanceTimersByTime(200);
            expect(paths(source)).toEqual([]);
            wants.setWanted(wanted(["a", 0]), new Set(["a"]));
            await Promise.resolve();
            vi.advanceTimersByTime(199);
            expect(paths(source)).toEqual([]);
            wants.dispose();
            vi.advanceTimersByTime(1000);
            expect(paths(source)).toEqual([]);
            wants.setWanted(wanted(["a", 0]), new Set(["a"]));
            await Promise.resolve();
            vi.advanceTimersByTime(200);
            expect(paths(source)).toEqual(["a"]);
        } finally {
            wants.dispose();
            vi.useRealTimers();
        }
    });

    it("gives everything back on dispose and can be used again", () => {
        const { source, wants } = wantsWith();
        wants.setWanted(wanted(["a", 0], ["b", 1]));
        wants.dispose();
        expect(paths(source)).toEqual([]);
        wants.setWanted(wanted(["c", 0]));
        expect(paths(source)).toEqual(["c"]);
    });

    it("merges what two parts want of the same image in the source, by the key of the spec", () => {
        const { source, wants } = wantsWith();
        wants.setWanted(wanted(["a", 3]));
        const key = specKey(spec("a"));
        expect(source.calls.at(-1)?.wants.get(key)).toMatchObject({ class: "visible", order: 3 });
    });
});

describe("thumbnail specs", () => {
    it("maps the thumbnail shape to the shape of the spec", () => {
        expect(shapeFor("crop")).toBe("crop");
        expect(shapeFor("fit")).toBe("fit");
    });

    it("multiplies the tile size by the pixel ratio, and never below one", () => {
        expect(thumbnailSize(176, 2)).toBe(352);
        expect(thumbnailSize(176, 0.5)).toBe(176);
    });

    it.each([
        ["crop", "crop"],
        ["fit", "fit"],
    ] as const)("asks for a %s thumbnail with the same size for the width and the height", (shape, expected) => {
        expect(thumbnailSpec("photos", "/a.jpg", 352, shape)).toEqual({
            libraryId: "photos",
            path: "/a.jpg",
            kind: "thumbnail",
            width: 352,
            height: 352,
            shape: expected,
        });
    });

    it("splits the Library from the path in the scope of a timeline", () => {
        expect(thumbnailSpec("*", "/photos/a/b.jpg", 256, "crop")).toMatchObject({
            libraryId: "photos",
            path: "/a/b.jpg",
        });
    });
});
