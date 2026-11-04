import { expect, it } from "vitest";
import type { Entry } from "../library/entries";
import {
    anchoredScrollLeft,
    centeredScrollLeft,
    GALLERY_GAP,
    galleryAnchor,
    galleryLayout,
    galleryVisibleRange,
    galleryWindow,
    galleryWindowCenter,
    nearestViewable,
} from "./gallery";

const image = (name: string): Entry => ({
    name,
    path: `/${name}`,
    kind: "file",
    metadata: { size: 1, readonly: false, contentType: "image/png" },
    file: { archive: false },
});
const folder: Entry = { name: "sub", path: "/sub", kind: "folder", metadata: { size: 0, readonly: false } };

const VIEWPORT = 1000;
const HEIGHT = 500;

function layoutOf(entries: (Entry | undefined)[], aspects: Record<string, number> = {}, first = 0) {
    return galleryLayout({
        first,
        count: entries.length,
        height: HEIGHT,
        viewport: VIEWPORT,
        entryAt: (index) => entries[index - first],
        aspectOf: (path) => aspects[path],
    });
}

it("gives each item the width of its aspect ratio, and an unread one a square", () => {
    const layout = layoutOf([image("a"), undefined], { "/a": 0.5 });
    expect([...layout.widths]).toEqual([250, 500]);
    expect(layout.starts[1]).toBe(VIEWPORT / 2 + 250 + GALLERY_GAP);
});

it("treats a loaded image without an aspect ratio as a square", () => {
    expect(layoutOf([image("a")]).widths[0]).toBe(HEIGHT);
});

it("gives no space to an Entry that is neither an image nor a video", () => {
    const layout = layoutOf([image("a"), folder, image("b")]);
    expect(layout.widths[1]).toBe(0);
    expect(layout.starts[2]).toBe(layout.starts[1]);
});

it("limits an extremely wide item to the share of the display area", () => {
    expect(layoutOf([image("a")], { "/a": 8 }).widths[0]).toBe(VIEWPORT * 0.9);
});

it("can center the first and the last item", () => {
    const layout = layoutOf([image("a"), image("b")]);
    expect(centeredScrollLeft(layout, 0)).toBe(HEIGHT / 2);
    const last = centeredScrollLeft(layout, 1);
    expect(last + VIEWPORT).toBeLessThanOrEqual(layout.total);
    expect(last + VIEWPORT / 2).toBe((layout.starts[1] as number) + HEIGHT / 2);
});

it("finds the visible range in absolute positions", () => {
    const layout = layoutOf([image("a"), image("b"), image("c"), image("d"), image("e")], {}, 100);
    expect(galleryVisibleRange(layout, 0)).toEqual({ first: 100, last: 100 });
    const atSecond = centeredScrollLeft(layout, 101);
    expect(galleryVisibleRange(layout, atSecond)).toEqual({ first: 100, last: 102 });
});

it("has no visible range without positions", () => {
    expect(galleryVisibleRange(layoutOf([]), 0)).toBeUndefined();
    expect(galleryAnchor(layoutOf([]), 0)).toBeUndefined();
});

it("keeps the anchored place at the center when an earlier item changes width", () => {
    const entries = [image("a"), image("b"), image("c")];
    const before = layoutOf(entries);
    const scrollLeft = centeredScrollLeft(before, 2) + 37;
    const anchor = galleryAnchor(before, scrollLeft);
    expect(anchor?.index).toBe(2);
    const after = layoutOf(entries, { "/a": 0.25, "/b": 2 });
    const restored = anchoredScrollLeft(after, anchor as NonNullable<typeof anchor>);
    expect(restored - centeredScrollLeft(after, 2)).toBeCloseTo(37);
});

it("keeps the anchor when positions are added before it", () => {
    const entries = [image("a"), image("b")];
    const before = layoutOf(entries, {}, 10);
    const anchor = galleryAnchor(before, centeredScrollLeft(before, 11));
    const after = layoutOf([image("x"), ...entries], {}, 9);
    expect(anchoredScrollLeft(after, anchor as NonNullable<typeof anchor>)).toBeCloseTo(centeredScrollLeft(after, 11));
});

it("picks the image or video nearest the center, skipping other kinds", () => {
    const entries = [image("a"), folder, image("b")];
    const layout = layoutOf(entries);
    const at = (index: number) => entries[index];
    expect(nearestViewable(layout, centeredScrollLeft(layout, 2), at)?.entry.name).toBe("b");
    expect(nearestViewable(layout, centeredScrollLeft(layout, 0), at)?.index).toBe(0);
});

it("does not pick an unread position", () => {
    const entries = [undefined, undefined];
    const layout = layoutOf(entries);
    expect(nearestViewable(layout, 0, (index) => entries[index])).toBeUndefined();
});

it("lays out a window of positions, however long the listing is", () => {
    expect(galleryWindow(5_000_000, 0, 10_000_000)).toEqual({ first: 4_999_750, count: 501 });
});

it("cuts the window at both ends of the listing", () => {
    expect(galleryWindow(10, 0, 1000)).toEqual({ first: 0, count: 261 });
    expect(galleryWindow(990, 0, 1000)).toEqual({ first: 740, count: 260 });
    expect(galleryWindow(0, 0, 0)).toEqual({ first: 0, count: 0 });
});

it("moves the window only when the display area nears an end that is not the listing's", () => {
    const span = galleryWindow(1000, 0, 2000);
    const layout = galleryLayout({
        ...span,
        height: HEIGHT,
        viewport: VIEWPORT,
        entryAt: () => undefined,
        aspectOf: () => undefined,
    });
    expect(galleryWindowCenter(layout, { first: 999, last: 1001 }, 0, 2000)).toBeUndefined();
    expect(galleryWindowCenter(layout, { first: 760, last: 762 }, 0, 2000)).toBe(761);
    expect(galleryWindowCenter(layout, { first: 1240, last: 1242 }, 0, 2000)).toBe(1241);
    // A range from before the window moved is outside it.
    expect(galleryWindowCenter(layout, { first: 10, last: 12 }, 0, 2000)).toBeUndefined();
    // The window already reaches this end of the listing, so there is nothing to bring in.
    expect(galleryWindowCenter(layout, { first: 760, last: 762 }, 750, 2000)).toBeUndefined();
});
