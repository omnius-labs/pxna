import { describe, expect, it } from "vitest";

import {
    anchorAt,
    buildRows,
    DAY_HEADING_HEIGHT,
    formatDay,
    formatMonth,
    restoreScroll,
    rowAtOffset,
    rowOfItem,
    rowStarts,
    verticalTarget,
} from "./timelineLayout";

const items = [
    { path: "/a", date: "2025-05-03T10:00:00" },
    { path: "/b", date: "2025-05-03T09:00:00" },
    { path: "/c", date: "2025-05-03T08:00:00" },
    { path: "/d", date: "2025-05-02T23:00:00" },
    { path: "/e", date: "2025-04-30T00:00:00" },
    { path: "/f", date: "2025-04-30T00:00:00" },
];

describe("timeline layout", () => {
    it("starts a heading row at every day and wraps the tiles of a day by column", () => {
        const rows = buildRows(items, 2, 100);
        expect(rows.map((row) => (row.type === "heading" ? `H${row.day}` : `T${row.first}+${row.count}`))).toEqual([
            "H2025-05-03",
            "T0+2",
            "T2+1",
            "H2025-05-02",
            "T3+1",
            "H2025-04-30",
            "T4+2",
        ]);
        expect(rows[0]?.height).toBe(DAY_HEADING_HEIGHT);
    });

    it("keeps row keys when older entries are appended or newer entries are prepended", () => {
        const before = buildRows(items.slice(0, 4), 2, 100).map((row) => row.key);
        const appended = buildRows(items, 2, 100).map((row) => row.key);
        expect(appended.slice(0, before.length)).toEqual(before);
        const prepended = buildRows([{ path: "/new", date: "2025-05-04T00:00:00" }, ...items.slice(0, 4)], 2, 100).map(
            (row) => row.key,
        );
        expect(prepended.slice(-before.length)).toEqual(before);
    });

    it("locates rows by offset and tiles by position", () => {
        const rows = buildRows(items, 2, 100);
        const { starts, total } = rowStarts(rows);
        expect(total).toBe(rows.reduce((sum, row) => sum + row.height, 0));
        expect(rowAtOffset(starts, 0)).toBe(0);
        expect(rowAtOffset(starts, starts[2]! + 1)).toBe(2);
        expect(rowAtOffset(starts, total + 1000)).toBe(rows.length - 1);
        expect([0, 1, 2, 3, 4, 5].map((position) => rowOfItem(rows, position))).toEqual([1, 1, 2, 4, 6, 6]);
        expect(rowOfItem(rows, 99)).toBeUndefined();
    });

    it("moves up and down by column and across days", () => {
        const rows = buildRows(items, 2, 100);
        expect(verticalTarget(rows, 0, 2, 1)).toBe(2);
        expect(verticalTarget(rows, 1, 2, 1)).toBe(2);
        expect(verticalTarget(rows, 2, 2, 1)).toBe(3);
        expect(verticalTarget(rows, 3, 2, 1)).toBe(4);
        expect(verticalTarget(rows, 5, 2, 1)).toBeUndefined();
        expect(verticalTarget(rows, 4, 2, -1)).toBe(3);
        expect(verticalTarget(rows, 3, 2, -1)).toBe(2);
        expect(verticalTarget(rows, 1, 2, -1)).toBeUndefined();
    });

    it("formats days and months from the calendar", () => {
        expect(formatDay("2025-05-03")).toBe("May 3, 2025 (Sat)");
        expect(formatDay("2024-02-29")).toBe("Feb 29, 2024 (Thu)");
        expect(formatMonth("2025-05")).toBe("May 2025");
    });
});

describe("scroll anchors", () => {
    const day = (count: number, prefix: string) =>
        Array.from({ length: count }, (_, i) => ({
            path: `/${prefix}${String(i).padStart(3, "0")}`,
            date: "2025-05-03T10:00:00",
        }));

    it("restores the row after newer entries are prepended and the row boundaries shift", () => {
        const before = day(10, "b");
        const beforeRows = buildRows(before, 3, 100);
        const beforeStarts = rowStarts(beforeRows).starts;
        // 先頭から 3 行目（2 件目の行の途中）を見ている。
        const scrollTop = beforeStarts[3]! + 17;
        const anchor = anchorAt(beforeRows, beforeStarts, before, scrollTop)!;
        expect(anchor.key).toBe("t:/b006");

        // 同じ日の新しい側へ 200 件足すと、行の区切りは列の数で割り切れない分だけずれ、元の行の key が消える。
        const after = [...day(200, "a"), ...before];
        const afterRows = buildRows(after, 3, 100);
        expect(afterRows.some((row) => row.key === anchor.key)).toBe(false);
        const afterStarts = rowStarts(afterRows).starts;
        const restored = restoreScroll(afterRows, afterStarts, after, anchor)!;
        const row = afterRows[rowAtOffset(afterStarts, restored)] as { first: number; count: number };
        // 見ていた行の先頭の Entry は、復元した行に含まれる。
        expect(row.first <= 200 + 6 && 200 + 6 < row.first + row.count).toBe(true);
        expect(restored - afterStarts[rowAtOffset(afterStarts, restored)]!).toBe(17);
    });

    it("keeps the position when the key survives and gives up when the entry is gone", () => {
        const items = day(6, "c");
        const rows = buildRows(items, 3, 100);
        const { starts } = rowStarts(rows);
        const anchor = anchorAt(rows, starts, items, starts[2]! + 5)!;
        expect(restoreScroll(rows, starts, items, anchor)).toBe(starts[2]! + 5);
        const other = day(2, "z");
        const otherRows = buildRows(other, 3, 100);
        expect(restoreScroll(otherRows, rowStarts(otherRows).starts, other, anchor)).toBeUndefined();
        expect(anchorAt([], [], [], 0)).toBeUndefined();
    });
});
