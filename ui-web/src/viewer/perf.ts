/// 計測のログを出す設定。ブラウザの console で `localStorage.setItem("pxna.perf", "1")` として再読み込みすると有効になる。
function perfEnabled(): boolean {
    try {
        return window.localStorage.getItem("pxna.perf") === "1";
    } catch {
        return false;
    }
}

/// 画像の読み込みが終わったときに、Resource Timing から内訳を console へ出す。
/// `queued` はブラウザの接続待ち、`waiting` はサーバの応答待ち（daemon の生成を含む）、`download` は本体の受信、
/// `afterResponse` は受信が終わってから `load` までの、ブラウザのデコードなどである。いずれもミリ秒。
export function logImageTiming(label: string, url: string): void {
    if (!perfEnabled()) {
        return;
    }
    const loadedAt = performance.now();
    const entries = performance.getEntriesByName(
        new URL(url, window.location.href).href,
    ) as PerformanceResourceTiming[];
    const entry = entries[entries.length - 1];
    if (entry === undefined) {
        console.info(`[pxna-perf] ${label}`, { url, note: "no resource timing" });
        return;
    }
    const round = (value: number) => Math.round(value * 10) / 10;
    console.info(`[pxna-perf] ${label}`, {
        url,
        at: new Date().toISOString(),
        cached: entry.transferSize === 0,
        queued: round(entry.requestStart - entry.startTime),
        waiting: round(entry.responseStart - entry.requestStart),
        download: round(entry.responseEnd - entry.responseStart),
        afterResponse: round(loadedAt - entry.responseEnd),
        total: round(loadedAt - entry.startTime),
        bytes: entry.encodedBodySize,
    });
}
