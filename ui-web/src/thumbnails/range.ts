import { PAGE_SIZE } from "../library/listing";

export type ScrollDirection = "forward" | "backward";

export interface VisibleRows {
    /// 画面に見えている最初と最後の行。
    firstRow: number;
    lastRow: number;
    rowCount: number;
    /// 最後にスクロールした方向。まだスクロールしていなければ `forward` とする。
    direction: ScrollDirection;
}

/// 要求の対象にする行の範囲と、優先度の基準にする画面の中央の行。
/// 見えている行に加え、スクロールの方向へ画面 1 枚分を先読みする。
export function wantedRows({ firstRow, lastRow, rowCount, direction }: VisibleRows): {
    first: number;
    last: number;
    center: number;
} {
    const screenRows = lastRow - firstRow + 1;
    const first = direction === "backward" ? firstRow - screenRows : firstRow;
    const last = direction === "forward" ? lastRow + screenRows : lastRow;
    return { first: Math.max(0, first), last: Math.min(rowCount - 1, last), center: (firstRow + lastRow) / 2 };
}

/// 画面の中央に近い行ほど小さく、同じ行では左のタイルを先にする。
export function tilePriority(row: number, column: number, center: number): number {
    return Math.abs(row - center) * 1000 + column;
}

/// 行とページの境界が一致しなくても、範囲のすべての位置を含むページを求める。
export function wantedPagesForRows(
    first: number,
    last: number,
    columns: number,
    totalCount: number,
): ReadonlySet<number> {
    const pages = new Set<number>();
    if (totalCount <= 0 || last < first) return pages;
    const from = Math.floor((first * columns) / PAGE_SIZE);
    const to = Math.floor(Math.min(totalCount - 1, (last + 1) * columns - 1) / PAGE_SIZE);
    for (let page = from; page <= to; page += 1) pages.add(page);
    return pages;
}
