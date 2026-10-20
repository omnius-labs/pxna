import { rowHeight } from "./grid";

/// 日付の見出しの行の高さ（CSS ピクセル）。
export const DAY_HEADING_HEIGHT = 40;

/// タイムラインの Entry のうち、並べるのに使う値。
export interface DatedItem {
    path: string;
    /// `YYYY-MM-DDTHH:MM:SS`。
    date: string;
}

export interface HeadingRow {
    type: "heading";
    key: string;
    day: string;
    height: number;
}

export interface TilesRow {
    type: "tiles";
    key: string;
    day: string;
    /// この行の先頭のタイルの、`items` の中の位置。
    first: number;
    count: number;
    height: number;
}

export type TimelineRow = HeadingRow | TilesRow;

/// 並びの日時の日付の部分（`YYYY-MM-DD`）。
export function dayOf(date: string): string {
    return date.slice(0, 10);
}

/// 並びの日時の月の部分（`YYYY-MM`）。
export function monthOf(date: string): string {
    return date.slice(0, 7);
}

const WEEKDAYS = ["日", "月", "火", "水", "木", "金", "土"] as const;

/// `2025-05-03` を「2025年5月3日（土）」の形にする。曜日は暦から求め、タイムゾーンに左右されない。
export function formatDay(day: string): string {
    const [year, month, date] = day.split("-").map(Number);
    if (year === undefined || month === undefined || date === undefined) return day;
    const weekday = WEEKDAYS[new Date(Date.UTC(year, month - 1, date)).getUTCDay()];
    return `${year}年${month}月${date}日（${weekday}）`;
}

/// `2025-05` を「2025年5月」の形にする。
export function formatMonth(month: string): string {
    const [year, number] = month.split("-").map(Number);
    return year === undefined || number === undefined ? month : `${year}年${number}月`;
}

/// 新しい順に並んだ Entry を、日ごとの見出しの行と、`columns` 個ずつのタイルの行にする。
/// 行の key は、先頭に足されても変わらない。見出しは日付、タイルの行は先頭の Entry のパスである。
export function buildRows(items: readonly DatedItem[], columns: number, tileSide: number): TimelineRow[] {
    const rows: TimelineRow[] = [];
    const tilesHeight = rowHeight(tileSide);
    let index = 0;
    while (index < items.length) {
        const day = dayOf(items[index]!.date);
        let end = index;
        while (end < items.length && dayOf(items[end]!.date) === day) end += 1;
        rows.push({ type: "heading", key: `h:${day}`, day, height: DAY_HEADING_HEIGHT });
        for (let first = index; first < end; first += columns) {
            rows.push({
                type: "tiles",
                key: `t:${items[first]!.path}`,
                day,
                first,
                count: Math.min(columns, end - first),
                height: tilesHeight,
            });
        }
        index = end;
    }
    return rows;
}

/// 各行の上端の位置と、全体の高さ。
export function rowStarts(rows: readonly TimelineRow[]): { starts: number[]; total: number } {
    const starts: number[] = [];
    let total = 0;
    for (const row of rows) {
        starts.push(total);
        total += row.height;
    }
    return { starts, total };
}

/// 位置 `offset` を含む行の番号。範囲の外なら端の行にする。
export function rowAtOffset(starts: readonly number[], offset: number): number {
    let low = 0;
    let high = starts.length - 1;
    while (low < high) {
        const middle = Math.ceil((low + high) / 2);
        if (starts[middle]! <= offset) low = middle;
        else high = middle - 1;
    }
    return Math.max(0, low);
}

/// `items` の位置を持つタイルの行の番号。
export function rowOfItem(rows: readonly TimelineRow[], position: number): number | undefined {
    let low = 0;
    let high = rows.length - 1;
    while (low <= high) {
        const middle = Math.floor((low + high) / 2);
        const row = rows[middle]!;
        if (row.type === "heading") {
            // 見出しは、直後のタイルの行と同じ側へ寄せて探す。
            const next = rows[middle + 1];
            if (next?.type === "tiles" && position >= next.first) low = middle + 1;
            else high = middle - 1;
        } else if (position < row.first) high = middle - 1;
        else if (position >= row.first + row.count) low = middle + 1;
        else return middle;
    }
    return undefined;
}

/// 上下の矢印キーで移る先の、`items` の中の位置。同じ列の隣の行へ移り、その日の端なら隣の日の同じ列（列が足りなければ最後の列）へ移る。
/// 動けなければ `undefined` にする。
export function verticalTarget(
    rows: readonly TimelineRow[],
    position: number,
    columns: number,
    direction: -1 | 1,
): number | undefined {
    const rowIndex = rowOfItem(rows, position);
    if (rowIndex === undefined) return undefined;
    const row = rows[rowIndex] as TilesRow;
    const column = (position - row.first) % columns;
    for (let next = rowIndex + direction; next >= 0 && next < rows.length; next += direction) {
        const target = rows[next]!;
        if (target.type === "tiles") return target.first + Math.min(column, target.count - 1);
    }
    return undefined;
}

/// スクロールの位置の目印。上端にある行と、その行の先頭の Entry、行の上端からの距離である。
export interface ScrollAnchor {
    key: string;
    /// 行の先頭の Entry のパス。行の区切りが変わって key が消えても、この Entry を含む行を探せる。見出しの行は、その日の最初の Entry である。
    path: string | undefined;
    offset: number;
}

/// 位置 `scrollTop` の目印を作る。
export function anchorAt(
    rows: readonly TimelineRow[],
    starts: readonly number[],
    items: readonly DatedItem[],
    scrollTop: number,
): ScrollAnchor | undefined {
    if (rows.length === 0) return undefined;
    const index = rowAtOffset(starts, scrollTop);
    const row = rows[index]!;
    return { key: row.key, path: items[firstItemOfRow(rows, index)]?.path, offset: scrollTop - starts[index]! };
}

/// 行の先頭の Entry の位置。見出しの行は、その日の最初のタイルの位置である。
export function firstItemOfRow(rows: readonly TimelineRow[], index: number): number {
    for (let next = index; next < rows.length; next += 1) {
        const row = rows[next]!;
        if (row.type === "tiles") return row.first;
    }
    return 0;
}

/// 行を組み直した後に、目印の行を同じ位置へ戻すスクロールの位置。見つけられなければ `undefined` にする。
/// 行の key が残っていればその行を、消えていれば目印の Entry を含む行を使う。
export function restoreScroll(
    rows: readonly TimelineRow[],
    starts: readonly number[],
    items: readonly DatedItem[],
    saved: ScrollAnchor,
): number | undefined {
    let index = rows.findIndex((row) => row.key === saved.key);
    if (index < 0 && saved.path !== undefined) {
        const position = items.findIndex((item) => item.path === saved.path);
        const found = position < 0 ? undefined : rowOfItem(rows, position);
        if (found !== undefined) index = found;
    }
    return index < 0 ? undefined : starts[index]! + saved.offset;
}
