import { expect, it } from "vitest";

import { tilePriority, wantedPagesForRows, wantedRows } from "./range";

it("includes a screen ahead in the last scroll direction", () => {
    expect(wantedRows({ firstRow: 10, lastRow: 14, rowCount: 100, direction: "forward" })).toEqual({
        first: 10,
        last: 19,
        center: 12,
    });
    expect(wantedRows({ firstRow: 10, lastRow: 14, rowCount: 100, direction: "backward" })).toEqual({
        first: 5,
        last: 14,
        center: 12,
    });
});
it("maps rows across page boundaries without reading intervening pages", () => {
    expect([...wantedPagesForRows(39, 41, 5, 2500)]).toEqual([0, 1]);
    expect([...wantedPagesForRows(499, 501, 5, 2500)]).toEqual([12]);
    expect([...wantedPagesForRows(0, -1, 5, 0)]).toEqual([]);
});

it("stays inside the rows that exist", () => {
    expect(wantedRows({ firstRow: 0, lastRow: 4, rowCount: 7, direction: "forward" })).toMatchObject({
        first: 0,
        last: 6,
    });
    expect(wantedRows({ firstRow: 2, lastRow: 6, rowCount: 100, direction: "backward" })).toMatchObject({
        first: 0,
        last: 6,
    });
});

it("puts rows near the center first and breaks ties from the left", () => {
    expect(tilePriority(12, 3, 12)).toBeLessThan(tilePriority(11, 0, 12) + 1);
    expect(tilePriority(12, 0, 12)).toBeLessThan(tilePriority(12, 1, 12));
    expect(tilePriority(13, 0, 12)).toBeLessThan(tilePriority(15, 0, 12));
    expect(tilePriority(10, 4, 12)).toBeGreaterThan(tilePriority(11, 4, 12));
});
