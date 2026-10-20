/// 1 枚のサムネイルの状態。`failed` は、そのキューが生きている間は要求し直さない。
export type ThumbnailState =
    | { status: "idle" }
    | { status: "loading" }
    | { status: "ready"; url: string }
    | { status: "failed" };

const IDLE: ThumbnailState = { status: "idle" };
const LOADING: ThumbnailState = { status: "loading" };
const FAILED: ThumbnailState = { status: "failed" };

export type ThumbnailLoader = (key: string, signal: AbortSignal) => Promise<Blob>;

export interface ThumbnailQueueOptions {
    /// 同時に送る要求の数。
    limit?: number;
    createUrl?: (blob: Blob) => string;
    revokeUrl?: (url: string) => void;
}

/// サムネイルの要求のキュー（[クライアントの設計 §4](../../../docs/design/client.md)）。
///
/// 呼び出し側が、いま要求したいタイルと優先度（小さいほど先）を渡し直す。
/// 同時に送る数を絞り、残りは送らずに待たせるので、画面から外れたタイルの要求を、送信前ならキューから外し、
/// 送信後なら取り消せる。サーバの生成は到着順なので、すべてを一度に送ると、外れたタイルの生成が先に並んだまま残る。
export class ThumbnailQueue {
    readonly #load: ThumbnailLoader;
    readonly #limit: number;
    readonly #createUrl: (blob: Blob) => string;
    readonly #revokeUrl: (url: string) => void;
    readonly #states = new Map<string, ThumbnailState>();
    readonly #inFlight = new Map<string, AbortController>();
    readonly #listeners = new Set<() => void>();
    #wanted: ReadonlyMap<string, number> = new Map();
    readonly #delays = new Map<string, { ready: boolean; timer?: ReturnType<typeof setTimeout> }>();

    constructor(load: ThumbnailLoader, options: ThumbnailQueueOptions = {}) {
        this.#load = load;
        this.#limit = options.limit ?? 4;
        this.#createUrl = options.createUrl ?? ((blob) => URL.createObjectURL(blob));
        this.#revokeUrl = options.revokeUrl ?? ((url) => URL.revokeObjectURL(url));
    }

    /// `useSyncExternalStore` に渡す。状態が変わらない間は同じ値を返す。
    getState = (key: string): ThumbnailState => this.#states.get(key) ?? IDLE;

    subscribe = (listener: () => void): (() => void) => {
        this.#listeners.add(listener);
        return () => this.#listeners.delete(listener);
    };

    /// 要求したいタイルを渡し直す。
    /// 外れたタイルの送信済みの要求は取り消し、読み終えた画像の URL は解放する。
    setWanted(wanted: ReadonlyMap<string, number>, delayed: ReadonlySet<string> = new Set()): void {
        this.#wanted = wanted;
        for (const [key, delay] of this.#delays) {
            if (!wanted.has(key) || !delayed.has(key)) {
                clearTimeout(delay.timer);
                this.#delays.delete(key);
            }
        }
        for (const key of delayed) {
            if (!wanted.has(key) || this.#delays.has(key)) continue;
            const delay: { ready: boolean; timer?: ReturnType<typeof setTimeout> } = { ready: false };
            this.#delays.set(key, delay);
            // DOM の commit に伴う observer 通知の後から遅延を数える。
            queueMicrotask(() => {
                if (this.#delays.get(key) !== delay) return;
                delay.timer = setTimeout(() => {
                    if (this.#delays.get(key) !== delay) return;
                    delay.ready = true;
                    this.#pump();
                    this.#emit();
                }, 200);
            });
        }
        for (const [key, controller] of this.#inFlight) {
            if (!wanted.has(key)) {
                this.#inFlight.delete(key);
                this.#states.delete(key);
                controller.abort();
            }
        }
        for (const [key, state] of this.#states) {
            if (!wanted.has(key) && state.status === "ready") {
                this.#revokeUrl(state.url);
                this.#states.delete(key);
            }
        }
        this.#pump();
        this.#emit();
    }

    /// 送信済みの要求をすべて取り消し、読み終えた画像の URL を解放する。
    /// 画面の開発時の `StrictMode` は後片付けの後で副作用を再実行するので、解放した後も `setWanted` で使い直せる。
    dispose(): void {
        this.#wanted = new Map();
        for (const delay of this.#delays.values()) clearTimeout(delay.timer);
        this.#delays.clear();
        for (const controller of this.#inFlight.values()) {
            controller.abort();
        }
        this.#inFlight.clear();
        for (const state of this.#states.values()) {
            if (state.status === "ready") {
                this.#revokeUrl(state.url);
            }
        }
        this.#states.clear();
    }

    /// 空いている枠の数だけ、優先度の小さいタイルから送る。
    #pump(): void {
        while (this.#inFlight.size < this.#limit) {
            const next = this.#next();
            if (next === undefined) {
                return;
            }
            this.#start(next);
        }
    }

    #next(): string | undefined {
        let best: string | undefined;
        let bestPriority = Infinity;
        for (const [key, priority] of this.#wanted) {
            if (this.#states.has(key) || priority >= bestPriority || this.#delays.get(key)?.ready === false) {
                continue;
            }
            best = key;
            bestPriority = priority;
        }
        return best;
    }

    #start(key: string): void {
        const controller = new AbortController();
        this.#inFlight.set(key, controller);
        this.#states.set(key, LOADING);
        this.#load(key, controller.signal).then(
            (blob) => {
                // 取り消した要求の結果は捨てる。
                if (this.#inFlight.get(key) !== controller) {
                    return;
                }
                this.#inFlight.delete(key);
                this.#states.set(key, { status: "ready", url: this.#createUrl(blob) });
                this.#pump();
                this.#emit();
            },
            () => {
                if (this.#inFlight.get(key) !== controller) {
                    return;
                }
                this.#inFlight.delete(key);
                this.#states.set(key, FAILED);
                this.#pump();
                this.#emit();
            },
        );
    }

    #emit(): void {
        for (const listener of this.#listeners) {
            listener();
        }
    }
}
