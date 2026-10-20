import { describe, expect, it } from "vitest";

import { testEntry } from "../testing/entries";
import { clickSelection, emptySelection, extendSelection, selectAll, selectionItem } from "./selection";

const order = ["a", "b", "c", "d", "e"];
const entries = order.map((path, index) =>
    testEntry(path, { folder: index === 0, metadata: { size: index + 1, readonly: index === 1 } }),
);
const itemAt = (index: number) => {
    const entry = entries[index];
    return entry === undefined ? undefined : selectionItem(entry, false);
};
const plain = { toggle: false, range: false };
const toggle = { toggle: true, range: false };
const range = { toggle: false, range: true };

describe("selection", () => {
    it("selects only the clicked entry on a plain click", () => {
        const first = clickSelection(emptySelection, 1, plain, itemAt);
        const second = clickSelection(first, 3, plain, itemAt);
        expect([...second.items.keys()]).toEqual(["d"]);
        expect(second.anchor).toBe(3);
    });

    it("adds and removes entries with the toggle modifier", () => {
        let selection = clickSelection(emptySelection, 0, plain, itemAt);
        selection = clickSelection(selection, 2, toggle, itemAt);
        expect([...selection.items.keys()].sort()).toEqual(["a", "c"]);
        selection = clickSelection(selection, 0, toggle, itemAt);
        expect([...selection.items.keys()]).toEqual(["c"]);
    });

    it("selects a range from the anchor in either direction", () => {
        const anchored = clickSelection(emptySelection, 3, plain, itemAt);
        expect([...clickSelection(anchored, 1, range, itemAt).items.keys()].sort()).toEqual(["b", "c", "d"]);
        expect([...clickSelection(anchored, 4, range, itemAt).items.keys()].sort()).toEqual(["d", "e"]);
    });

    it("keeps the anchor across consecutive range clicks", () => {
        const anchored = clickSelection(emptySelection, 2, plain, itemAt);
        const wide = clickSelection(anchored, 4, range, itemAt);
        const narrow = clickSelection(wide, 3, range, itemAt);
        expect([...narrow.items.keys()].sort()).toEqual(["c", "d"]);
    });

    it("adds a range to the selection with both modifiers", () => {
        let selection = clickSelection(emptySelection, 0, plain, itemAt);
        selection = clickSelection(selection, 3, toggle, itemAt);
        selection = clickSelection(selection, 4, { toggle: true, range: true }, itemAt);
        expect([...selection.items.keys()].sort()).toEqual(["a", "d", "e"]);
    });

    it("ignores a click outside the loaded entries", () => {
        expect(clickSelection(emptySelection, 9, plain, itemAt)).toBe(emptySelection);
    });

    it("extends from the previous focus when no anchor exists yet", () => {
        const first = extendSelection(emptySelection, 1, 2, itemAt);
        expect([...first.items.keys()].sort()).toEqual(["b", "c"]);
        const second = extendSelection(first, 2, 3, itemAt);
        expect([...second.items.keys()].sort()).toEqual(["b", "c", "d"]);
        const back = extendSelection(second, 3, 2, itemAt);
        expect([...back.items.keys()].sort()).toEqual(["b", "c"]);
    });
});

it("counts sizes for the entire selection independently of listing pages", () => {
    const selection = selectAll(order.length, itemAt);
    expect(selection.items.size).toBe(5);
    expect([...selection.items.values()].reduce((sum, size) => sum + size.size!, 0)).toBe(15);
});

it("範囲と全件の選択で種類と readonly を保つ", () => {
    const selected = selectAll(5, itemAt);
    expect(selected.items.get("a")).toMatchObject({ size: 1, folder: true, readonly: false });
    expect(clickSelection(emptySelection, 1, plain, itemAt).items.get("b")).toMatchObject({
        size: 2,
        folder: false,
        readonly: true,
    });
});

it("クリック、範囲、全選択、トグルが作る値は、選んだ Entry と同じ参照を持つ", () => {
    const entryOf = (selection: { items: ReadonlyMap<string, { entry: unknown }> }, path: string) =>
        selection.items.get(path)?.entry;
    const expected = (index: number) => entries[index];
    const clicked = clickSelection(emptySelection, 1, plain, itemAt);
    expect(entryOf(clicked, "b")).toBe(expected(1));
    const ranged = clickSelection(clicked, 3, range, itemAt);
    expect(["b", "c", "d"].map((path) => entryOf(ranged, path))).toEqual([expected(1), expected(2), expected(3)]);
    const toggled = clickSelection(clicked, 4, toggle, itemAt);
    expect(entryOf(toggled, "e")).toBe(expected(4));
    const all = selectAll(order.length, itemAt);
    expect(order.map((path) => entryOf(all, path))).toEqual(order.map((_, index) => expected(index)));
});
