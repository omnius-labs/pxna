import { useEffect, useState } from "react";

/// 値が変わらないまま、この時間（ミリ秒）がたったときに、その値を返す。
export const SETTLE_MS = 200;

/// `value` が `delayMs` の間変わらなかったときだけ、その値を返す。変わった描画からは、`undefined` に戻る。
/// 値の比較は参照で行うので、同じ Entry を選んでいる間は、同じ値が続く。
/// 以前に落ち着いた値へ短い間に戻っても、待ち直す。
export function useSettled<T>(value: T, delayMs = SETTLE_MS): T | undefined {
    const [state, setState] = useState<{ current: T; settled: boolean }>({ current: value, settled: false });
    if (state.current !== value) setState({ current: value, settled: false });
    useEffect(() => {
        const timer = setTimeout(() => setState({ current: value, settled: true }), delayMs);
        return () => clearTimeout(timer);
    }, [value, delayMs]);
    return state.settled && state.current === value ? value : undefined;
}
