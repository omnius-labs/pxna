import { MAX_TILE_SIZE, MIN_TILE_SIZE } from "./grid";

/// タイルサイズを増減する入力の向き。1 で大きく、-1 で小さくする。
export type StepDirection = 1 | -1;

/// 段階を範囲の中に収める。
export function clampTileSize(tileSize: number): number {
    return Math.min(Math.max(Math.round(tileSize), MIN_TILE_SIZE), MAX_TILE_SIZE);
}

/// 段階を `direction` へ 1 つ動かす。範囲の端では動かさない。
export function stepTileSize(tileSize: number, direction: StepDirection): number {
    return clampTileSize(tileSize + direction);
}

interface KeyInput {
    key: string;
    ctrlKey: boolean;
    metaKey: boolean;
    altKey: boolean;
}

/// `+` と `-` のキーによる増減。修飾キーとの組はブラウザの拡大縮小に使われるので、拾わない。
export function directionFromKey(input: KeyInput): StepDirection | undefined {
    if (input.ctrlKey || input.metaKey || input.altKey) {
        return undefined;
    }
    if (input.key === "+" || input.key === "=") {
        return 1;
    }
    if (input.key === "-" || input.key === "_") {
        return -1;
    }
    return undefined;
}

interface WheelInput {
    deltaY: number;
    ctrlKey: boolean;
    metaKey: boolean;
}

/// Ctrl か ⌘ を押しながらのホイールによる増減。奥へ回すと小さく、手前へ回すと大きくする。
export function directionFromWheel(input: WheelInput): StepDirection | undefined {
    if (!(input.ctrlKey || input.metaKey) || input.deltaY === 0) {
        return undefined;
    }
    return input.deltaY < 0 ? 1 : -1;
}

/// キー入力を受け取る要素か。入力欄、選択欄、編集できる要素と、ダイアログの中では、増減の入力を拾わない。
export function isTypingTarget(target: EventTarget | null): boolean {
    if (!(target instanceof Element)) {
        return false;
    }
    return (
        target.closest(
            "input, textarea, select, [contenteditable=''], [contenteditable='true'], dialog, [role='dialog'], [role='alertdialog']",
        ) !== null
    );
}

export function clipboardCommand(input: KeyInput): "cut" | "copy" | "paste" | undefined {
    if (!(input.metaKey || input.ctrlKey) || input.altKey) return;
    return ({ x: "cut", c: "copy", v: "paste" } as const)[input.key.toLowerCase() as "x" | "c" | "v"];
}
