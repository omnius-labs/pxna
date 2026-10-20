import { describe, expect, it } from "vitest";

import { gridLayout, NARROW_COLUMNS, PLACE_HEIGHT, requestTileSide, rowHeight, TILE_GAP, TILE_MIN_SIDES } from "./grid";

const steps = [1, 2, 3, 4, 5];

describe("gridLayout on a wide screen", () => {
    it.each(steps)("never makes a tile smaller than the minimum of step %d", (step) => {
        for (const width of [300, 640, 900, 1280, 1921]) {
            const { tileSide } = gridLayout({ width, tileSize: step, narrow: false });
            expect(tileSide).toBeGreaterThanOrEqual(Math.min(TILE_MIN_SIDES[step - 1]!, width - TILE_GAP));
        }
    });

    it("fits the most columns that keep the minimum side, and one column at least", () => {
        expect(gridLayout({ width: 0, tileSize: 5, narrow: false }).columns).toBe(1);
        // 最小値 176 と間隔 12 の 1 列分が 188。1000 には 5 列が入る。
        expect(gridLayout({ width: 1000, tileSize: 3, narrow: false }).columns).toBe(5);
        expect(gridLayout({ width: 187, tileSize: 3, narrow: false }).columns).toBe(1);
        expect(gridLayout({ width: 188, tileSize: 3, narrow: false }).columns).toBe(1);
        expect(gridLayout({ width: 376, tileSize: 3, narrow: false }).columns).toBe(2);
    });

    it("spreads the columns over the whole width without leaving a margin on the right", () => {
        for (const step of steps) {
            for (const width of [640, 777, 1280, 1500]) {
                const { columns, tileSide } = gridLayout({ width, tileSize: step, narrow: false });
                const used = columns * (tileSide + TILE_GAP);
                expect(used).toBeLessThanOrEqual(width);
                expect(width - used).toBeLessThan(columns);
            }
        }
    });

    it("uses more columns as the steps get smaller", () => {
        const columns = steps.map((tileSize) => gridLayout({ width: 1280, tileSize, narrow: false }).columns);
        expect(columns).toEqual([...columns].sort((a, b) => b - a));
    });
});

describe("gridLayout on a narrow screen", () => {
    it.each(steps)("uses the number of columns of step %d whatever the width", (step) => {
        for (const width of [320, 390, 500, 767]) {
            const { columns, tileSide } = gridLayout({ width, tileSize: step, narrow: true });
            expect(columns).toBe(NARROW_COLUMNS[step - 1]);
            expect(columns * (tileSide + TILE_GAP)).toBeLessThanOrEqual(width);
        }
    });

    it("maps steps 1 to 5 to 6, 5, 4, 3 and 2 columns", () => {
        expect(steps.map((tileSize) => gridLayout({ width: 500, tileSize, narrow: true }).columns)).toEqual([
            6, 5, 4, 3, 2,
        ]);
    });
});

it("clamps a step outside the range", () => {
    expect(gridLayout({ width: 1000, tileSize: 0, narrow: true }).columns).toBe(6);
    expect(gridLayout({ width: 1000, tileSize: 9, narrow: true }).columns).toBe(2);
});

describe("requestTileSide", () => {
    it("is the minimum side of the step on a wide screen, and does not depend on the list width", () => {
        expect(steps.map((tileSize) => requestTileSide({ tileSize, narrow: false, screenWidth: 1280 }))).toEqual([
            96, 128, 176, 240, 320,
        ]);
        expect(requestTileSide({ tileSize: 3, narrow: false, screenWidth: 1920 })).toBe(176);
    });

    it("is the screen width divided by the columns on a narrow screen", () => {
        expect(requestTileSide({ tileSize: 3, narrow: true, screenWidth: 400 })).toBe(100 - TILE_GAP);
        expect(requestTileSide({ tileSize: 5, narrow: true, screenWidth: 400 })).toBe(200 - TILE_GAP);
    });
});

it("adds the name and the gap to the tile side for a row", () => {
    expect(rowHeight(100)).toBe(100 + 24 + TILE_GAP);
});

it("場所の行を出す高さだけ、グリッドの行を広げる", () => {
    expect(rowHeight(100, PLACE_HEIGHT)).toBe(rowHeight(100) + 20);
});
