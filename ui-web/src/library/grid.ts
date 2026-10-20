/// タイルの間と、一覧の左端と上端の余白（CSS ピクセル）。
export const TILE_GAP = 12;
const NAME_HEIGHT = 24;
export const PLACE_HEIGHT = 20;

/// タイルサイズの段階（1 が最小、5 が最大）。
export const MIN_TILE_SIZE = 1;
export const MAX_TILE_SIZE = 5;

/// 広い画面で、段階ごとのタイルの一辺の最小値（CSS ピクセル）。添字は段階 - 1。
export const TILE_MIN_SIDES = [96, 128, 176, 240, 320] as const;

/// 狭い画面で、段階ごとの列の数。添字は段階 - 1。
export const NARROW_COLUMNS = [6, 5, 4, 3, 2] as const;

export interface GridLayout {
    columns: number;
    /// タイルの一辺（CSS ピクセル）。列を一覧の幅いっぱいに広げた値である。
    tileSide: number;
}

function stepIndex(tileSize: number): number {
    return Math.min(Math.max(Math.round(tileSize), MIN_TILE_SIZE), MAX_TILE_SIZE) - 1;
}

/// 一覧の幅とタイルサイズの段階から、列の数とタイルの一辺を求める。
/// 広い画面では、段階の値を一辺の最小値として入る列の数を求める。狭い画面では、段階を列の数として扱う。
/// どちらも、列を一覧の幅いっぱいに広げる。タイルの間と左端の余白は `TILE_GAP` で、右端に余白は残らない。
export function gridLayout({
    width,
    tileSize,
    narrow,
}: {
    width: number;
    tileSize: number;
    narrow: boolean;
}): GridLayout {
    const index = stepIndex(tileSize);
    const columns = narrow
        ? NARROW_COLUMNS[index]!
        : Math.max(1, Math.floor(width / (TILE_MIN_SIDES[index]! + TILE_GAP)));
    return { columns, tileSide: Math.max(1, Math.floor(width / columns) - TILE_GAP) };
}

/// タイルの一辺から求める、行の高さ。名前、場所、行の間隔を含む。
export function rowHeight(tileSide: number, placeHeight = 0): number {
    return tileSide + NAME_HEIGHT + placeHeight + TILE_GAP;
}

/// サムネイルを要求するときの、タイルの表示の大きさ（CSS ピクセル）。
/// 広い画面では段階の一辺の最小値、狭い画面では画面の幅を列の数で割った値とする。
/// 一覧の幅だけが変わっても変わらないので、幅を変えている間にサムネイルの要求が続けて出ない。
export function requestTileSide({
    tileSize,
    narrow,
    screenWidth,
}: {
    tileSize: number;
    narrow: boolean;
    screenWidth: number;
}): number {
    const index = stepIndex(tileSize);
    return narrow ? Math.max(1, Math.floor(screenWidth / NARROW_COLUMNS[index]!) - TILE_GAP) : TILE_MIN_SIDES[index]!;
}
