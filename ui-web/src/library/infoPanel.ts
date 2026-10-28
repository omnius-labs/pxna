import { useState } from "react";

/// 情報パネルの開閉を保存する `localStorage` のキー。値は `true` か `false` である。
export const INFO_PANEL_OPEN_KEY = "pxna.infoPanelOpen";

/// パネルの大きめのサムネイルの一辺（CSS px）の段階。枠の幅と高さの大きい方が収まる最小の段を使う。
export const INFO_PANEL_THUMBNAIL_SIDES = [256, 384, 512, 640] as const;

/// 枠の幅と高さ（CSS px）から、要求するサムネイルの一辺を決める。
export function infoThumbnailSide(width: number, height: number): number {
    const need = Math.max(width, height);
    return INFO_PANEL_THUMBNAIL_SIDES.find((side) => side >= need) ?? INFO_PANEL_THUMBNAIL_SIDES.at(-1)!;
}

/// 保存した開閉を読む。保存がないときと、読めないときは、開いた状態にする。
export function readInfoPanelOpen(): boolean {
    try {
        return window.localStorage.getItem(INFO_PANEL_OPEN_KEY) !== "false";
    } catch {
        return true;
    }
}

/// 開閉を保存する。保存できなくても、閲覧は妨げないので、例外を出さない。
export function writeInfoPanelOpen(open: boolean): void {
    try {
        window.localStorage.setItem(INFO_PANEL_OPEN_KEY, String(open));
    } catch {
        // 保存できなければ、このページを開いている間だけ有効になる。
    }
}

/// 情報パネルの開閉と、切り替え。切り替えると、ブラウザに保存する。
export function useInfoPanel() {
    const [open, setOpen] = useState(readInfoPanelOpen);
    const toggle = () => {
        const next = !open;
        setOpen(next);
        writeInfoPanelOpen(next);
    };
    return { open, toggle };
}
