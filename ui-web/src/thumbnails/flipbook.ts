import type { CSSProperties } from "react";

/// パラパラ再生でフレームを切り替える間隔。
export const FLIPBOOK_INTERVAL_MS = 400;

export function nextFrame(frame: number, frameCount: number): number {
    return frameCount <= 1 ? 0 : (frame + 1) % frameCount;
}

/// 横 1 列に並んだスプライトシートの `frame` 番目のフレームだけをタイルに出す、背景の指定。
/// 各フレームは正方形で、タイルと同じ形なので、シート全体の幅を広げるだけで歪まない（取得は `crop` を指定する）。
/// 背景の幅をフレームの数だけ広げ、位置を 0% から 100% の間で等分する。
export function flipbookStyle(url: string, frame: number, frameCount: number): CSSProperties {
    const position = frameCount > 1 ? (frame / (frameCount - 1)) * 100 : 0;
    return {
        backgroundImage: `url("${url}")`,
        backgroundSize: `${frameCount * 100}% 100%`,
        backgroundPosition: `${position}% 0`,
        backgroundRepeat: "no-repeat",
    };
}

/// 全体表示のスプライトシートの `frame` 番目のフレームだけを出す、背景の指定。
/// 各フレームは縦横比を保って正方形に収まった大きさなので、フレームの縦横比（幅 / 高さ）から、タイルの一辺 `side` に収まる大きさを求める。
/// 隣のフレームがはみ出さないよう、タイルの全体ではなくフレームの大きさの要素に背景を置く。要素の中央寄せは、タイルが行う。
export function fitFlipbookStyle(
    url: string,
    frame: number,
    frameCount: number,
    side: number,
    frameAspect: number,
): CSSProperties {
    const width = frameAspect >= 1 ? side : side * frameAspect;
    const height = frameAspect >= 1 ? side / frameAspect : side;
    return {
        width,
        height,
        backgroundImage: `url("${url}")`,
        backgroundSize: `${width * frameCount}px ${height}px`,
        backgroundPosition: `${-frame * width}px 0`,
        backgroundRepeat: "no-repeat",
    };
}
