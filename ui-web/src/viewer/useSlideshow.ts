import { useCallback, useEffect, useRef, useState } from "react";

/// 画像を出してから次の画像へ進むまでの時間（ミリ秒）。
export const SLIDESHOW_INTERVAL_MS = 5000;

/// 1 回の前後の移動の結果。`moved` は移った、`end` は進む先がない（または読めなかった）、`busy` は別の移動の最中か中断である。
export type SlideshowStep = "moved" | "end" | "busy";

/// スライドショーの状態とタイマー。
/// `settled` は表示用画像が出た（か、読めないと分かった）画像の識別子で、これが変わるたびに数え直す。`undefined` の間は数えない。
/// 数え終えると `advance` で次の画像へ進み、進む先がなければ止まる。
export function useSlideshow({
    settled,
    advance,
}: {
    settled: string | undefined;
    advance: () => Promise<SlideshowStep>;
}) {
    const [running, setRunning] = useState(false);
    // 別の移動の最中に時間が来たときは、数え直す。
    const [retry, setRetry] = useState(0);
    const latest = useRef(advance);
    useEffect(() => {
        latest.current = advance;
    });
    // biome-ignore lint/correctness/useExhaustiveDependencies: retry は別の移動の最中に時間が来たとき、タイマーを掛け直させる gate で、本体からは参照しない意図的な依存
    useEffect(() => {
        if (!running || settled === undefined) {
            return;
        }
        const timer = setTimeout(() => {
            void latest.current().then((result) => {
                if (result === "end") setRunning(false);
                else if (result === "busy") setRetry((count) => count + 1);
            });
        }, SLIDESHOW_INTERVAL_MS);
        return () => clearTimeout(timer);
    }, [running, settled, retry]);
    const start = useCallback(() => setRunning(true), []);
    const stop = useCallback(() => setRunning(false), []);
    return { running, start, stop };
}
