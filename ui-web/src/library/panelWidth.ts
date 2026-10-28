import { useCallback, useState } from "react";

/// ツリーの幅を保存する `localStorage` のキー。値は CSS px の整数である。
export const TREE_WIDTH_KEY = "pxna.treeWidth";
/// タイムラインの左の領域の幅を保存する `localStorage` のキー。値は CSS px の整数である。
export const TIMELINE_WIDTH_KEY = "pxna.timelineWidth";
/// 情報パネルの幅を保存する `localStorage` のキー。値は CSS px の整数である。
export const INFO_PANEL_WIDTH_KEY = "pxna.infoPanelWidth";

/// 情報パネルのサムネイル枠の高さを保存する `localStorage` のキー。値は CSS px の整数である。
export const INFO_THUMBNAIL_HEIGHT_KEY = "pxna.infoThumbnailHeight";

export interface WidthRange {
    min: number;
    max: number;
    initial: number;
}

export const TREE_WIDTH: WidthRange = { min: 160, max: 480, initial: 224 };
/// タイムラインの左の領域の幅。ツリーと同じ範囲である。
export const TIMELINE_WIDTH: WidthRange = TREE_WIDTH;
export const INFO_PANEL_WIDTH: WidthRange = { min: 240, max: 640, initial: 288 };
/// 情報パネルのサムネイル枠の高さ。
export const INFO_THUMBNAIL_HEIGHT: WidthRange = { min: 96, max: 640, initial: 224 };

/// キー操作で 1 回に動かす幅（CSS px）。
export const WIDTH_KEY_STEP = 16;

export function clampWidth(range: WidthRange, width: number): number {
    return Math.min(range.max, Math.max(range.min, Math.round(width)));
}

/// 保存した幅を読む。保存がないときと、数でないときは、初期値にする。範囲の外の値は範囲に収める。
export function readWidth(key: string, range: WidthRange): number {
    try {
        const stored = window.localStorage.getItem(key);
        if (stored === null) return range.initial;
        const width = Number(stored);
        return Number.isFinite(width) ? clampWidth(range, width) : range.initial;
    } catch {
        return range.initial;
    }
}

/// 幅を保存する。保存できなくても、閲覧は妨げないので、例外を出さない。
export function writeWidth(key: string, width: number): void {
    try {
        window.localStorage.setItem(key, String(width));
    } catch {
        // 保存できなければ、このページを開いている間だけ有効になる。
    }
}

/// ドラッグで変えられるパネルの幅。`resize` は画面の幅だけを変え、`commit` で端末に保存する。
export function usePanelWidth(key: string, range: WidthRange) {
    const [width, setWidth] = useState(() => readWidth(key, range));
    const resize = useCallback((next: number) => setWidth(clampWidth(range, next)), [range]);
    const commit = useCallback((next: number) => writeWidth(key, clampWidth(range, next)), [key, range]);
    return { width, resize, commit };
}
