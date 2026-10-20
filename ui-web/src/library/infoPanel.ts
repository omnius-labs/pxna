import { useState } from "react";

/// 情報パネルの開閉を保存する `localStorage` のキー。値は `true` か `false` である。
export const INFO_PANEL_OPEN_KEY = "pxna.infoPanelOpen";

/// パネルの大きめのサムネイルの一辺（CSS px）。パネルの幅から余白を除いた値である。
export const INFO_PANEL_THUMBNAIL_SIDE = 256;

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
