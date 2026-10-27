import type { Entry } from "../library/entries";
import { wantedPagesForRows } from "../thumbnails/range";
import { isViewable } from "./viewer";

/// The side (CSS px) of one filmstrip item for each size level from 1 to 5. Every Entry in the listing gets the same side.
export const FILMSTRIP_ITEM_SIDES = [40, 48, 56, 72, 96] as const;

export const MIN_FILMSTRIP_SIZE = 1;
export const MAX_FILMSTRIP_SIZE = FILMSTRIP_ITEM_SIDES.length;

export const DEFAULT_FILMSTRIP_SIZE = 3;

/// The side (CSS px) of one filmstrip item for a size level. A level outside 1 to 5 is clamped, and a non-finite level is the default.
export function filmstripItemSide(level: number): number {
    const rounded = Number.isFinite(level) ? Math.round(level) : DEFAULT_FILMSTRIP_SIZE;
    const clamped = Math.min(Math.max(rounded, MIN_FILMSTRIP_SIZE), MAX_FILMSTRIP_SIZE);
    return FILMSTRIP_ITEM_SIDES[clamped - 1] as number;
}

/// 見えている範囲の前後に足す、先読みの位置の数。
export const FILMSTRIP_LOOKAHEAD = 10;

export interface FilmstripWanted {
    /// 読むページ。
    pages: ReadonlySet<number>;
    /// サムネイルを要求する画像と動画の相対パスと優先度。小さいほど先で、見えている範囲の中央に近い位置から決める。
    thumbnails: ReadonlyMap<string, number>;
    /// `thumbnails` のうち、見えている範囲の外にあり、先読みとして要求するもの。
    prefetch: ReadonlySet<string>;
}

/// 見えている位置の範囲と先読みの余白から、読むページと、サムネイルを要求する画像と動画を決める。
/// 画像でも動画でもない Entry は、種類のアイコンを出すので、サムネイルを要求しない。
export function filmstripWanted({
    first,
    last,
    count,
    entryAt,
}: {
    first: number;
    last: number;
    count: number;
    entryAt: (index: number) => Entry | undefined;
}): FilmstripWanted {
    const from = Math.max(0, first - FILMSTRIP_LOOKAHEAD);
    const to = Math.min(count - 1, last + FILMSTRIP_LOOKAHEAD);
    const center = (first + last) / 2;
    const thumbnails = new Map<string, number>();
    const prefetch = new Set<string>();
    for (let index = from; index <= to; index += 1) {
        const entry = entryAt(index);
        if (entry && isViewable(entry)) {
            thumbnails.set(entry.path, Math.abs(index - center));
            if (index < first || index > last) prefetch.add(entry.path);
        }
    }
    return { pages: wantedPagesForRows(from, to, 1, count), thumbnails, prefetch };
}
