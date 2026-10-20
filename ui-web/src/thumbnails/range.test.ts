import { expect, it } from "vitest";

import { wantedPagesForRows, wantedRows } from "./range";

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
