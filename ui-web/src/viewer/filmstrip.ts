import type { Entry } from "../library/entries";
import { wantedPagesForRows } from "../thumbnails/range";
import { isViewable } from "./viewer";

/// フィルムストリップの 1 つの位置の幅（CSS px）。一覧のすべての Entry に同じ幅を割り当てる。
export const FILMSTRIP_ITEM_WIDTH = 56;

/// 見えている範囲の前後に足す、先読みの位置の数。
export const FILMSTRIP_LOOKAHEAD = 10;

export interface FilmstripWanted {
    /// 読むページ。
    pages: ReadonlySet<number>;
    /// サムネイルを要求する画像と動画の相対パスと優先度。小さいほど先で、見えている範囲の中央に近い位置から決める。
    thumbnails: ReadonlyMap<string, number>;
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
    for (let index = from; index <= to; index += 1) {
        const entry = entryAt(index);
        if (entry && isViewable(entry)) {
            thumbnails.set(entry.path, Math.abs(index - center));
        }
    }
    return { pages: wantedPagesForRows(from, to, 1, count), thumbnails };
}
