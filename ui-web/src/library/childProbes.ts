import { api, unwrap } from "../api/client";

export interface ChildProbeTarget {
    key: string;
    libraryId: string;
    path: string;
    snapshot: string;
}
interface ProbeResponse {
    hasChildren: boolean | null;
    sourceVersion?: string;
    maxAgeMs: number;
}
interface ProbeState {
    snapshot: string;
    value?: boolean;
    expires: number;
    retryAt: number;
    failures: number;
}

/// 可視行だけを1要求ずつ確認する。通常の子一覧の購読とは独立している。
export class ChildProbeQueue {
    private wanted = new Map<string, ChildProbeTarget>();
    private states = new Map<string, ProbeState>();
    private active?: { target: ChildProbeTarget; controller: AbortController };
    private timer?: ReturnType<typeof setTimeout>;
    private listeners = new Set<() => void>();
    private disposed = false;
    private results: ReadonlyMap<string, boolean> = new Map();
    constructor(
        private readonly request: (target: ChildProbeTarget, signal: AbortSignal) => Promise<ProbeResponse> = async (
            target,
            signal,
        ) =>
            unwrap(
                await api.GET("/libraries/{libraryId}/folders/children", {
                    params: {
                        path: { libraryId: target.libraryId },
                        query: { path: target.path, includeHidden: false },
                    },
                    signal,
                }),
            ),
    ) {}
    subscribe = (listener: () => void) => {
        this.listeners.add(listener);
        return () => {
            this.listeners.delete(listener);
        };
    };
    snapshot = () => this.results;
    setWanted(targets: readonly ChildProbeTarget[]) {
        if (this.disposed) return;
        this.wanted = new Map(targets.map((target) => [target.key, target]));
        for (const [key, state] of this.states) {
            if (this.wanted.get(key)?.snapshot !== state.snapshot) this.states.delete(key);
        }
        if (this.active && this.wanted.get(this.active.target.key)?.snapshot !== this.active.target.snapshot) {
            this.active.controller.abort();
        }
        this.publish();
        this.pump();
    }
    dispose() {
        this.disposed = true;
        clearTimeout(this.timer);
        this.active?.controller.abort();
        this.wanted.clear();
        this.states.clear();
        this.listeners.clear();
    }
    private publish() {
        const now = Date.now();
        this.results = new Map(
            [...this.states]
                .filter(([, state]) => state.value !== undefined && state.expires > now)
                .map(([key, state]) => [key, state.value!]),
        );
        for (const listener of this.listeners) listener();
    }
    private pump() {
        clearTimeout(this.timer);
        if (this.disposed) return;
        this.publish();
        const now = Date.now();
        let wake = Infinity;
        let next: ChildProbeTarget | undefined;
        for (const target of this.wanted.values()) {
            const state = this.states.get(target.key);
            const ready = state ? Math.max(state.expires, state.retryAt) : 0;
            if (ready <= now) next ??= target;
            else wake = Math.min(wake, ready);
        }
        if (!this.active && next) {
            const target = next;
            const controller = new AbortController();
            this.active = { target, controller };
            void this.request(target, controller.signal)
                .then(
                    (result) => this.accept(target, controller, result),
                    () => this.accept(target, controller, { hasChildren: null, maxAgeMs: 0 }),
                )
                .finally(() => {
                    this.active = undefined;
                    this.pump();
                });
        }
        // Expire displayed values even if a different request is still pending.
        for (const state of this.states.values()) if (state.expires > now) wake = Math.min(wake, state.expires);
        if (Number.isFinite(wake) && wake > now) this.timer = setTimeout(() => this.pump(), wake - now);
    }
    private accept(target: ChildProbeTarget, controller: AbortController, result: ProbeResponse) {
        if (this.disposed || controller.signal.aborted || this.wanted.get(target.key)?.snapshot !== target.snapshot)
            return;
        const now = Date.now();
        const known = result.hasChildren !== null && !!result.sourceVersion && result.maxAgeMs > 0;
        const failures = known ? 0 : (this.states.get(target.key)?.failures ?? 0) + 1;
        this.states.set(target.key, {
            snapshot: target.snapshot,
            value: known ? result.hasChildren! : undefined,
            expires: known ? now + Math.min(result.maxAgeMs, 30000) : 0,
            retryAt: known ? 0 : now + Math.min(500 * 2 ** Math.min(failures - 1, 6), 30000),
            failures,
        });
    }
}
