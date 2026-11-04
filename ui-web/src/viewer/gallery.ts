import type { Entry } from "../library/entries";
import { isViewable } from "./viewer";

/// The space (CSS px) between items.
export const GALLERY_GAP = 24;
/// The share of the display area's height that the items take. The rest is the margin above and below them.
const ITEM_HEIGHT_SHARE = 0.75;
/// The largest share of the display area's width that one item takes.
const MAX_ITEM_SHARE = 0.9;
/// The aspect ratio of an item whose thumbnail has not loaded.
const DEFAULT_ASPECT = 1;

/// How many positions are laid out on each side of the window's center.
const WINDOW_RADIUS = 250;
/// How close, in positions, the display area may come to the end of the window before it moves.
const WINDOW_MARGIN = 50;

/// The positions to lay out around a center, within a listing that runs from `first` up to `end` (not including it).
/// Only a window is laid out, because the cost of a layout grows with its length and a browser cannot scroll an element
/// beyond a limit that a long listing would pass.
export function galleryWindow(center: number, first: number, end: number): { first: number; count: number } {
    const from = Math.max(first, center - WINDOW_RADIUS);
    return { first: from, count: Math.max(0, Math.min(end, center + WINDOW_RADIUS + 1) - from) };
}

/// The center the window should move to, when the display area nears an end of it that is not the end of the listing.
/// A visible range outside the window is ignored.
export function galleryWindowCenter(
    layout: GalleryLayout,
    visible: { first: number; last: number },
    first: number,
    end: number,
): number | undefined {
    const layoutEnd = layout.first + layout.count;
    // A visible range from before the window moved lies outside it, and says nothing about where to go.
    if (visible.first < layout.first || visible.last >= layoutEnd) return undefined;
    const nearStart = layout.first > first && visible.first - layout.first < WINDOW_MARGIN;
    const nearEnd = layoutEnd < end && layoutEnd - 1 - visible.last < WINDOW_MARGIN;
    return nearStart || nearEnd ? Math.round((visible.first + visible.last) / 2) : undefined;
}

/// The height (CSS px) of the items in a display area of a height.
export function galleryItemHeight(areaHeight: number): number {
    return Math.max(1, Math.round(areaHeight * ITEM_HEIGHT_SHARE));
}

/// Where each item is. Positions are absolute positions in the listing, and the arrays are indexed from `first`.
/// The first and last items can reach the center, because half a display area of space is on both sides.
/// `first` and `count` describe the window, not the listing ([`galleryWindow`]).
export interface GalleryLayout {
    first: number;
    count: number;
    viewport: number;
    starts: Float64Array;
    widths: Float64Array;
    total: number;
}

/// Lays out the positions from `first`. An Entry that is neither an image nor a video takes no space, and an unread position is a square.
export function galleryLayout({
    first,
    count,
    height,
    viewport,
    entryAt,
    aspectOf,
}: {
    first: number;
    count: number;
    height: number;
    viewport: number;
    entryAt: (index: number) => Entry | undefined;
    aspectOf: (path: string) => number | undefined;
}): GalleryLayout {
    const size = Math.max(0, count);
    const starts = new Float64Array(size);
    const widths = new Float64Array(size);
    const lead = viewport / 2;
    const widest = viewport * MAX_ITEM_SHARE;
    let cursor = lead;
    for (let offset = 0; offset < size; offset += 1) {
        const entry = entryAt(first + offset);
        starts[offset] = cursor;
        if (entry !== undefined && !isViewable(entry)) continue;
        const aspect = (entry && aspectOf(entry.path)) ?? DEFAULT_ASPECT;
        const width = Math.min(height * aspect, widest);
        widths[offset] = width;
        cursor += width + GALLERY_GAP;
    }
    return { first, count: size, viewport, starts, widths, total: cursor + lead };
}

/// The offset of the last item starting at or before `x`.
function offsetAt(layout: GalleryLayout, x: number): number {
    let low = 0;
    let high = layout.count - 1;
    while (low < high) {
        const middle = Math.ceil((low + high) / 2);
        if ((layout.starts[middle] as number) <= x) low = middle;
        else high = middle - 1;
    }
    return low;
}

/// The range of positions inside the display area, or `undefined` when there are no positions.
export function galleryVisibleRange(
    layout: GalleryLayout,
    scrollLeft: number,
): { first: number; last: number } | undefined {
    if (layout.count === 0) return undefined;
    return {
        first: layout.first + offsetAt(layout, scrollLeft),
        last: layout.first + offsetAt(layout, scrollLeft + layout.viewport),
    };
}

/// The `scrollLeft` that places a position's center at the center of the display area.
export function centeredScrollLeft(layout: GalleryLayout, index: number): number {
    const offset = Math.min(layout.count - 1, Math.max(0, index - layout.first));
    const center = (layout.starts[offset] ?? 0) + (layout.widths[offset] ?? 0) / 2;
    return Math.max(0, center - layout.viewport / 2);
}

/// A place in the layout that survives a change of widths: a position and how far across it the place is.
export interface GalleryAnchor {
    index: number;
    fraction: number;
}

/// The anchor of the place at the center of the display area.
export function galleryAnchor(layout: GalleryLayout, scrollLeft: number): GalleryAnchor | undefined {
    if (layout.count === 0) return undefined;
    const center = scrollLeft + layout.viewport / 2;
    const offset = offsetAt(layout, center);
    const width = layout.widths[offset] as number;
    return {
        index: layout.first + offset,
        fraction: width === 0 ? 0 : (center - (layout.starts[offset] as number)) / width,
    };
}

/// The `scrollLeft` that returns an anchor to the center of the display area.
export function anchoredScrollLeft(layout: GalleryLayout, anchor: GalleryAnchor): number {
    const offset = Math.min(layout.count - 1, Math.max(0, anchor.index - layout.first));
    const center = (layout.starts[offset] ?? 0) + (layout.widths[offset] ?? 0) * anchor.fraction;
    return Math.max(0, center - layout.viewport / 2);
}

/// The loaded image or video inside the display area nearest its center.
export function nearestViewable(
    layout: GalleryLayout,
    scrollLeft: number,
    entryAt: (index: number) => Entry | undefined,
): { index: number; entry: Entry } | undefined {
    const range = galleryVisibleRange(layout, scrollLeft);
    if (range === undefined) return undefined;
    const center = scrollLeft + layout.viewport / 2;
    let nearest: { index: number; entry: Entry } | undefined;
    let nearestDistance = Number.POSITIVE_INFINITY;
    for (let index = range.first; index <= range.last; index += 1) {
        const entry = entryAt(index);
        if (entry === undefined || !isViewable(entry)) continue;
        const offset = index - layout.first;
        const distance = Math.abs((layout.starts[offset] as number) + (layout.widths[offset] as number) / 2 - center);
        if (distance < nearestDistance) {
            nearest = { index, entry };
            nearestDistance = distance;
        }
    }
    return nearest;
}
