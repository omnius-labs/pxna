import type { QueryClient } from "@tanstack/react-query";

import { capabilitiesQuery } from "../library/entries";
import { entryAddress } from "../location";
import { api } from "./client";
import type { components } from "./schema";

type ClientTiming = components["schemas"]["ClientTiming"];
type ClientTimingOperation = components["schemas"]["ClientTimingOperation"];

/// 1 回に送る件数の上限。API の上限と同じ。
const MAX_BATCH = 200;
/// 送る前に溜める件数の上限。超えた分は古いものから捨てる。
const MAX_PENDING = 1000;
/// 記録してから送るまでの待ち。まとめて送り、要求の数を減らす。
const FLUSH_DELAY_MS = 2000;

const round = (value: number) => Math.round(value * 10) / 10;
const phase = (from: number, to: number) => (from > 0 && to >= from ? round(to - from) : undefined);

/// クライアントで測った時間を daemon の計測のログへ送る（`/performance/client-timings`）。
///
/// daemon が計測を有効にしているか（`/capabilities` の `performanceLogging`）を最初の記録で確かめ、
/// 無効なら記録を捨てる。送信の失敗は、表示を妨げないよう無視する。
export class ClientTimings {
    #client: QueryClient | undefined;
    #enabled: boolean | undefined;
    #checking = false;
    #pending: ClientTiming[] = [];
    #timer: ReturnType<typeof setTimeout> | undefined;

    /// 設定を確かめるための QueryClient を渡す。画面を離れるときに、溜めた記録を送る。
    attach(client: QueryClient): void {
        this.#client = client;
        globalThis.addEventListener?.("pagehide", () => this.flush());
    }

    record(timing: ClientTiming): void {
        if (this.#enabled === false) {
            return;
        }
        this.#pending.push(timing);
        if (this.#pending.length > MAX_PENDING) {
            this.#pending.splice(0, this.#pending.length - MAX_PENDING);
        }
        if (this.#enabled === undefined) {
            this.#check();
        } else {
            this.#schedule();
        }
    }

    /// 読み終えた要求の時間を、Resource Timing の内訳とともに記録する。`startedAt` は要求を始めた `performance.now()` の値。
    /// `scope` は Library の id か、タイムラインの範囲である。
    recordResource(
        operation: ClientTimingOperation,
        url: string,
        scope: string,
        scopedPath: string,
        startedAt: number,
    ): void {
        if (this.#enabled === false) {
            return;
        }
        const loadedAt = performance.now();
        const name = new URL(url, globalThis.location?.href ?? "http://localhost/").href;
        const entries = performance.getEntriesByName(name) as PerformanceResourceTiming[];
        const entry = entries[entries.length - 1];
        const { libraryId, path } = entryAddress(scope, scopedPath);
        this.record({
            operation,
            libraryId,
            path,
            durationMs: round(Math.max(0, loadedAt - startedAt)),
            ...(entry && {
                queuedMs: phase(entry.startTime, entry.requestStart),
                waitingMs: phase(entry.requestStart, entry.responseStart),
                downloadMs: phase(entry.responseStart, entry.responseEnd),
                afterResponseMs: phase(entry.responseEnd, loadedAt),
                bytes: entry.encodedBodySize,
                cached: entry.transferSize === 0,
            }),
        });
    }

    /// 溜めた記録をすべて送る。画面を離れた後も届くよう `keepalive` で送る。
    flush(): void {
        clearTimeout(this.#timer);
        this.#timer = undefined;
        if (this.#enabled !== true) {
            return;
        }
        while (this.#pending.length > 0) {
            const timings = this.#pending.splice(0, MAX_BATCH);
            void api.POST("/performance/client-timings", { body: { timings }, keepalive: true }).catch(() => undefined);
        }
    }

    #check(): void {
        if (this.#checking || this.#client === undefined) {
            return;
        }
        this.#checking = true;
        this.#client
            .fetchQuery(capabilitiesQuery)
            .then((capabilities) => capabilities.performanceLogging)
            .catch(() => false)
            .then((enabled) => {
                this.#checking = false;
                this.#enabled = enabled;
                if (enabled) {
                    this.#schedule();
                } else {
                    this.#pending = [];
                }
            });
    }

    #schedule(): void {
        if (this.#timer === undefined) {
            this.#timer = setTimeout(() => this.flush(), FLUSH_DELAY_MS);
        }
    }
}

export const clientTimings = new ClientTimings();
