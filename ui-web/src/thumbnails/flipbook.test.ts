import { describe, expect, it } from "vitest";

import { fitFlipbookStyle, flipbookStyle, nextFrame } from "./flipbook";

describe("flipbook", () => {
    it("cycles through the frames and returns to the first", () => {
        expect([0, 1, 2, 3].map((frame) => nextFrame(frame, 4))).toEqual([1, 2, 3, 0]);
        expect(nextFrame(0, 1)).toBe(0);
    });

    it("shows one frame of a horizontal sprite sheet", () => {
        expect(flipbookStyle("blob:x", 0, 8)).toMatchObject({
            backgroundSize: "800% 100%",
            backgroundPosition: "0% 0",
        });
        expect(flipbookStyle("blob:x", 7, 8)).toMatchObject({ backgroundPosition: "100% 0" });
        expect(flipbookStyle("blob:x", 1, 5)).toMatchObject({ backgroundPosition: "25% 0" });
    });

    it("fits a wide frame into the side of the tile and keeps the other frames out of view", () => {
        // 16:9 のフレーム 8 枚。一辺 160 に収めると 160 x 90 で、シート全体は 1280 x 90 になる。
        expect(fitFlipbookStyle("blob:x", 0, 8, 160, 16 / 9)).toMatchObject({
            width: 160,
            height: 90,
            backgroundSize: "1280px 90px",
            backgroundPosition: "0px 0",
        });
        expect(fitFlipbookStyle("blob:x", 3, 8, 160, 16 / 9)).toMatchObject({ backgroundPosition: "-480px 0" });
    });

    it("fits a tall frame by its height", () => {
        expect(fitFlipbookStyle("blob:x", 1, 4, 160, 0.5)).toMatchObject({
            width: 80,
            height: 160,
            backgroundSize: "320px 160px",
            backgroundPosition: "-80px 0",
        });
    });
});
