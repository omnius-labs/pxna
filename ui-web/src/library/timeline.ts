import { type QueryClient, queryOptions, useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo, useSyncExternalStore } from "react";

import { api, unwrap } from "../api/client";
import type { components } from "../api/schema";
import { entryAddress, TIMELINE_SCOPE, timelineKey } from "../location";
import type { Entry } from "./entries";
import { type Listing, ListingSnapshotChanged, type PageStatus } from "./listing";

export type TimelineMonth = components["schemas"]["TimelineMonth"];
export type TimelineStatus = components["schemas"]["TimelineStatus"];

/// タイムラインに並べる Library の id。保存した選択のうち今ある Library を、Library の一覧の順に並べる。
/// 保存した選択がないか、今ある Library が 1 つも残らないときは、すべての Library を並べる。
export function timelineLibraryIds(
    libraries: readonly { id: string }[],
    saved: readonly string[] | undefined,
): string[] {
    const chosen = saved === undefined ? undefined : new Set(saved);
    const ids = libraries.map((library) => library.id).filter((id) => chosen?.has(id) ?? true);
    return ids.length > 0 ? ids : libraries.map((library) => library.id);
}

/// 1 回の要求で読む件数。
export const TIMELINE_PAGE_SIZE = 200;

/// 最初に読んだ範囲の先頭の位置。前へ読み足すたびに小さくなり、読み込み済みの Entry の位置は変わらない。
/// ビューアの `neighbor` が 0 以上の位置だけを探すため、0 より十分大きい値から始める。
export const TIMELINE_BASE = 100_000;

/// 前へ読み足して、先頭の位置がこの値より小さくなるときは、位置を振り直す。
const REBASE_THRESHOLD = 1_000;

/// 見えている位置から遠い Entry を、これだけたまってから捨てる。捨てるたびに読み込みと行の組み直しが続かないようにする。
const TRIM_STEP = 200;

/// 見えている位置の前後に残す Entry の数。これより遠い Entry は捨てる。
export const KEEP_ITEMS = 1000;

/// ビューアが、開いている Entry から前後へ読み進めてよい範囲（件数）。フィルムストリップをこれより遠くへ送っても、その先は読まない。
/// 読み込み済みの範囲は連続した 1 つなので、開いている Entry と遠い位置の間をすべて持つことになるのを防ぐ。
export const VIEWER_REACH_ITEMS = 3000;

/// フィルムストリップが読もうとするページのうち、開いている Entry から `reach` 件以内にあるもの。
export function reachablePages(
    current: number | undefined,
    pages: ReadonlySet<number>,
    reach: number = VIEWER_REACH_ITEMS,
): ReadonlySet<number> {
    if (current === undefined) return pages;
    return new Set(
        [...pages].filter(
            (page) =>
                page * TIMELINE_PAGE_SIZE <= current + reach && (page + 1) * TIMELINE_PAGE_SIZE - 1 >= current - reach,
        ),
    );
}

/// 開いている Entry と、フィルムストリップが読む範囲（ページ）を、前後に `margin` 件の余白をつけて残す位置の範囲。
export function keepRange(
    current: number | undefined,
    pages: ReadonlySet<number>,
    margin: number = KEEP_ITEMS,
): { first: number; last: number } | undefined {
    const points: number[] = current === undefined ? [] : [current];
    if (pages.size > 0) {
        points.push(Math.min(...pages) * TIMELINE_PAGE_SIZE, (Math.max(...pages) + 1) * TIMELINE_PAGE_SIZE - 1);
    }
    return points.length === 0
        ? undefined
        : { first: Math.min(...points) - margin, last: Math.max(...points) + margin };
}

/// サーバの cursor と同じ形（並びの日時の 19 文字に Library の id と Library 相対パスを続けた文字列）にする。
function cursorOf(item: TimelineItem): string {
    return `${item.date}${item.libraryId}${item.path}`;
}

/// 読み込みの起点。
export type TimelineAnchor =
    | { kind: "newest" }
    /// `YYYY-MM` の月の、最も新しい Entry から。`nonce` は、同じ月を選び直したときに、読み直した別の範囲として扱うための番号である。
    | { kind: "month"; month: string; nonce?: number }
    /// 鍵（`timelineKey`）の Entry を中央に含む範囲。
    | { kind: "around"; path: string };

/// タイムラインに並べる Entry。`entry.path` は Library の id を含む鍵で、選択、ビューア、範囲の中の位置はこの鍵で Entry を区別する。
export interface TimelineItem {
    entry: Entry;
    libraryId: string;
    /// Library 相対パス。
    path: string;
    /// `YYYY-MM-DDTHH:MM:SS`。
    date: string;
    source: string;
}

type Direction = "previous" | "next";

function anchorKey(anchor: TimelineAnchor): string {
    return anchor.kind === "newest"
        ? "newest"
        : anchor.kind === "month"
          ? `month:${anchor.month}:${anchor.nonce ?? 0}`
          : `around:${anchor.path}`;
}

/// タイムラインの読み込み済みの範囲。新しい順に並べた連続した範囲を持ち、前（新しい側）と後ろ（古い側）へ読み足す。
///
/// 位置は、最初に読んだ範囲の先頭を `TIMELINE_BASE` とする絶対の値で表す。前へ読み足しても読み込み済みの Entry の位置は変わらないので、
/// 選択、フォーカス、ビューアが持つ位置を直さずに済む。読み込み済みの範囲の外は未読として扱い、続きがあれば範囲の外の 1 つ先まで `totalCount` に含める。
export class TimelineStore implements Listing {
    truncated = false;
    snapshot: string | undefined;
    private items: TimelineItem[] = [];
    private base = TIMELINE_BASE;
    private previousCursor: string | undefined;
    private nextCursor: string | undefined;
    private loaded = false;
    private failure: Error | null = null;
    private epoch = 0;
    private revision = 0;
    private generation = 0;
    private listeners = new Set<() => void>();
    private initial: Promise<void> | undefined;
    private pending: Partial<Record<Direction, Promise<void>>> = {};
    private focus: string | undefined;
    holders = 0;

    constructor(
        readonly libraryIds: readonly string[],
        readonly anchor: TimelineAnchor,
    ) {}

    subscribe = (listener: () => void) => {
        this.listeners.add(listener);
        return () => {
            this.listeners.delete(listener);
        };
    };
    getRevision = () => this.revision;
    get version() {
        return this.revision;
    }
    private emit() {
        this.revision += 1;
        for (const listener of this.listeners) listener();
    }

    get totalCount(): number | undefined {
        return this.loaded ? this.base + this.items.length + (this.nextCursor === undefined ? 0 : 1) : undefined;
    }
    get firstIndex(): number {
        return this.previousCursor === undefined ? this.base : 0;
    }
    get loadedFrom(): number {
        return this.base;
    }
    /// 読み込み済みの範囲の終わりの次の位置。
    get loadedTo(): number {
        return this.base + this.items.length;
    }
    get hasPrevious(): boolean {
        return this.previousCursor !== undefined;
    }
    get hasNext(): boolean {
        return this.nextCursor !== undefined;
    }
    get loadedItems(): readonly TimelineItem[] {
        return this.items;
    }
    get error(): Error | null {
        return this.failure;
    }
    get isLoaded(): boolean {
        return this.loaded;
    }

    entryAt = (index: number) => this.items[index - this.base]?.entry;
    itemAt = (index: number) => this.items[index - this.base];

    locate = (path: string): number | undefined => {
        const position = this.items.findIndex((item) => item.entry.path === path);
        return position < 0 ? undefined : this.base + position;
    };

    /// 見えている位置から遠い Entry を捨てる。`first` から `last`（含む）の外側を、`TRIM_STEP` 件以上たまったときに捨てる。
    /// 読み込み済みの Entry の位置は変わらず、捨てた側は、捨てた境の Entry の cursor から読み直せる。
    trim(first: number, last: number) {
        if (!this.loaded) return;
        const from = Math.max(this.base, first);
        const to = Math.min(this.loadedTo - 1, last);
        if (to < from) return;
        const dropFront = this.pending.previous ? 0 : from - this.base;
        const dropBack = this.pending.next ? 0 : this.loadedTo - 1 - to;
        if (dropFront < TRIM_STEP && dropBack < TRIM_STEP) return;
        const items = this.items;
        const front = dropFront >= TRIM_STEP ? dropFront : 0;
        const back = dropBack >= TRIM_STEP ? dropBack : 0;
        if (front > 0) {
            const edge = items[front];
            if (edge === undefined) return;
            this.previousCursor = cursorOf(edge);
            this.base += front;
        }
        if (back > 0) {
            const edge = items[items.length - back - 1];
            if (edge === undefined) return;
            this.nextCursor = cursorOf(edge);
        }
        this.items = items.slice(front, items.length - back);
        this.emit();
    }

    /// 再読み込みで、見えている位置の近くから読み直すための目印。鍵（`timelineKey`）で渡す。
    setFocus(path: string | undefined) {
        this.focus = path;
    }

    pageError = () => this.failure;

    pageStatus = (page: number): PageStatus => {
        if (!this.loaded) return this.failure ? "error" : this.initial ? "loading" : "idle";
        const need = this.need(page);
        if (!need) return "loaded";
        if (this.failure) return "error";
        return this.pending[need] ? "loading" : "idle";
    };

    /// 位置のページに、まだ読んでいない前か後ろの範囲があるか。
    private need(page: number): Direction | undefined {
        const low = Math.max(page * TIMELINE_PAGE_SIZE, this.firstIndex);
        const high = Math.min((page + 1) * TIMELINE_PAGE_SIZE, this.totalCount ?? 0);
        if (low >= high) return undefined;
        if (low < this.base) return "previous";
        if (high > this.loadedTo) return "next";
        return undefined;
    }

    retry = () => {
        this.failure = null;
        this.emit();
        if (!this.loaded) void this.ensureInitial().catch(() => {});
    };

    /// 起点の範囲を読む。読み終えるまで同じ要求を共有する。
    ensureInitial(): Promise<void> {
        if (this.loaded) return Promise.resolve();
        this.initial ??= this.fetchInitial(this.anchor).then(
            (result) => {
                this.initial = undefined;
                this.apply(result);
            },
            (error: unknown) => {
                this.initial = undefined;
                this.failure = error instanceof Error ? error : new Error(String(error));
                this.emit();
                throw error;
            },
        );
        return this.initial;
    }

    loadPages = async (pages: readonly number[], signal: AbortSignal): Promise<void> => {
        signal.throwIfAborted();
        await this.ensureInitial();
        const snapshot = this.snapshot;
        for (;;) {
            signal.throwIfAborted();
            if (this.snapshot !== snapshot) throw new ListingSnapshotChanged();
            const need = pages.map((page) => this.need(page)).find((direction) => direction !== undefined);
            if (need === undefined) return;
            await this.loadMore(need);
        }
    };

    ensureIndex = async (index: number, signal: AbortSignal): Promise<void> => {
        signal.throwIfAborted();
        await this.ensureInitial();
        const snapshot = this.snapshot;
        for (;;) {
            signal.throwIfAborted();
            if (this.snapshot !== snapshot) throw new ListingSnapshotChanged();
            if (index >= this.base && index < this.loadedTo) return;
            const direction: Direction = index < this.base ? "previous" : "next";
            if ((direction === "previous" ? this.previousCursor : this.nextCursor) === undefined) return;
            await this.loadMore(direction);
        }
    };

    loadMore(direction: Direction): Promise<void> {
        const cursor = direction === "previous" ? this.previousCursor : this.nextCursor;
        if (!this.loaded || cursor === undefined) return Promise.resolve();
        const running = this.pending[direction];
        if (running) return running;
        const generation = this.generation;
        let request: Promise<void> | undefined;
        request = (async () => {
            try {
                const page = await fetchPage(
                    this.libraryIds,
                    direction === "previous" ? { before: cursor } : { after: cursor },
                );
                if (generation !== this.generation) return;
                this.append(direction, page);
            } catch (error) {
                if (generation === this.generation) {
                    this.failure = error instanceof Error ? error : new Error(String(error));
                    this.emit();
                }
                throw error;
            } finally {
                if (this.pending[direction] === request) delete this.pending[direction];
            }
        })();
        this.pending[direction] = request;
        this.emit();
        return request;
    }

    /// 前（新しい側）と後ろ（古い側）へ、見えている範囲の近くまで読み足す。画面の側が、見えている端の近さから呼ぶ。
    /// 読み足しに失敗した後は、読み足し続けず、`retry` で再開するまで止める。通信が続けて失敗しても、要求が増え続けないようにする。
    wantMore(direction: Direction) {
        if (this.failure) return;
        void this.loadMore(direction).catch(() => {});
    }

    /// 指定した鍵の Entry を中央に含む範囲へ読み直す。
    recenter(path: string) {
        this.focus = path;
        void this.reload();
    }

    /// 古い内容を見せたまま起点から読み直し、読み終えたら範囲を差し替える。見えている位置があれば、その近くから読む。
    async reload(): Promise<void> {
        if (!this.loaded) return;
        const generation = ++this.generation;
        this.pending = {};
        const focus = this.focus;
        let result: Awaited<ReturnType<TimelineStore["fetchInitial"]>>;
        try {
            result = await this.fetchInitial(focus === undefined ? this.anchor : { kind: "around", path: focus }).catch(
                () => this.fetchInitial(this.anchor.kind === "around" ? { kind: "newest" } : this.anchor),
            );
        } catch (error) {
            if (generation === this.generation) {
                this.failure = error instanceof Error ? error : new Error(String(error));
                this.emit();
            }
            return;
        }
        if (generation !== this.generation) return;
        this.apply(result);
    }

    private async fetchInitial(anchor: TimelineAnchor) {
        if (anchor.kind !== "around")
            return fetchPage(this.libraryIds, anchor.kind === "month" ? { month: anchor.month } : {});
        const { libraryId, path } = entryAddress(TIMELINE_SCOPE, anchor.path);
        return fetchPage(this.libraryIds, { around: path, aroundLibraryId: libraryId });
    }

    /// 範囲を差し替える。版が変わるので、選択の起点とビューアの位置が捨てられる。
    private apply(page: TimelineChunk) {
        this.items = page.items;
        this.base = TIMELINE_BASE;
        this.previousCursor = page.previous;
        this.nextCursor = page.next;
        this.loaded = true;
        this.failure = null;
        this.epoch += 1;
        this.snapshot = String(this.epoch);
        this.emit();
    }

    private append(direction: Direction, page: TimelineChunk) {
        const known = new Set(this.items.map((item) => item.entry.path));
        const fresh = page.items.filter((item) => !known.has(item.entry.path));
        if (direction === "previous") {
            this.items = [...fresh, ...this.items];
            this.base -= fresh.length;
            this.previousCursor = page.previous;
            if (this.base < REBASE_THRESHOLD) {
                // 位置を振り直す。位置を持つ側は、版の変化として扱う。
                this.base += TIMELINE_BASE;
                this.epoch += 1;
                this.snapshot = String(this.epoch);
            }
        } else {
            this.items = [...this.items, ...fresh];
            this.nextCursor = page.next;
        }
        this.failure = null;
        this.emit();
    }
}

interface TimelineChunk {
    items: TimelineItem[];
    previous: string | undefined;
    next: string | undefined;
}

async function fetchPage(
    libraryIds: readonly string[],
    query: { before?: string; after?: string; month?: string; around?: string; aroundLibraryId?: string },
    signal?: AbortSignal,
): Promise<TimelineChunk> {
    const listing = unwrap(
        await api.GET("/timeline/entries", {
            params: { query: { libraryIds: [...libraryIds], ...query, limit: TIMELINE_PAGE_SIZE } },
            signal,
        }),
    );
    return {
        items: listing.entries.map((item) => ({
            entry: { ...item.entry, path: timelineKey(item.libraryId, item.entry.path) },
            libraryId: item.libraryId,
            path: item.entry.path,
            date: item.timelineDate,
            source: item.timelineDateSource,
        })),
        previous: listing.previous,
        next: listing.next,
    };
}

const stores = new WeakMap<QueryClient, Map<string, TimelineStore>>();

function storeKey(libraryIds: readonly string[], anchor: TimelineAnchor): string {
    return JSON.stringify([libraryIds, anchorKey(anchor)]);
}

function storeFor(client: QueryClient, libraryIds: readonly string[], anchor: TimelineAnchor): TimelineStore {
    let byKey = stores.get(client);
    if (!byKey) {
        byKey = new Map();
        stores.set(client, byKey);
    }
    const key = storeKey(libraryIds, anchor);
    let store = byKey.get(key);
    if (!store) {
        store = new TimelineStore(libraryIds, anchor);
        byKey.set(key, store);
    }
    return store;
}

/// 画面が使っている間だけ範囲を持つ。使われなくなったら捨て、次に開くときは読み直す。
export function useTimelineStore(libraryIds: readonly string[], anchor: TimelineAnchor): TimelineStore {
    const client = useQueryClient();
    const key = storeKey(libraryIds, anchor);
    // biome-ignore lint/correctness/useExhaustiveDependencies: libraryIds と anchor は key から作り直せる値で、key の変化だけを gate にする
    const store = useMemo(() => storeFor(client, libraryIds, anchor), [client, key]);
    useEffect(() => {
        store.holders += 1;
        void store.ensureInitial().catch(() => {});
        return () => {
            store.holders -= 1;
            // StrictMode の仮の取り外しで捨てないよう、同じ描画の中で付け直される間は待つ。
            queueMicrotask(() => {
                if (store.holders > 0) return;
                const byKey = stores.get(client);
                const registered = byKey?.get(storeKey(store.libraryIds, store.anchor));
                if (registered === store) byKey?.delete(storeKey(store.libraryIds, store.anchor));
            });
        };
    }, [client, store]);
    useSyncExternalStore(store.subscribe, store.getRevision);
    return store;
}

/// Library の変更や集計の後に、それらの Library を並べている範囲のうち、使われている範囲を読み直し、使われていない範囲を捨てる。月ごとの件数と状態も読み直す。
export async function refreshTimeline(client: QueryClient, libraryIds: readonly string[]): Promise<void> {
    const touches = (ids: readonly string[] | undefined) => ids?.some((id) => libraryIds.includes(id)) ?? false;
    const byKey = stores.get(client);
    const reloads: Promise<void>[] = [];
    for (const [key, store] of byKey ?? []) {
        if (!touches(store.libraryIds)) continue;
        if (store.holders > 0) reloads.push(store.reload());
        else byKey?.delete(key);
    }
    await Promise.all([
        ...reloads,
        client.invalidateQueries({
            queryKey: ["timeline"],
            predicate: (query) => touches(query.queryKey[2] as readonly string[] | undefined),
        }),
    ]);
}

export function timelineMonthsQuery(libraryIds: readonly string[]) {
    return queryOptions({
        queryKey: ["timeline", "months", libraryIds],
        queryFn: async ({ signal }): Promise<TimelineMonth[]> =>
            unwrap(
                await api.GET("/timeline/months", {
                    params: { query: { libraryIds: [...libraryIds] } },
                    signal,
                }),
            ).months,
    });
}

export function timelineStatusQuery(libraryIds: readonly string[]) {
    return queryOptions({
        queryKey: ["timeline", "status", libraryIds],
        queryFn: async ({ signal }): Promise<TimelineStatus[]> =>
            unwrap(
                await api.GET("/timeline/status", {
                    params: { query: { libraryIds: [...libraryIds] } },
                    signal,
                }),
            ).libraries,
    });
}

/// 並べている Library の全体の集計を始め、今の状態を返す。
export async function startTimelineScan(libraryIds: readonly string[]): Promise<TimelineStatus[]> {
    return unwrap(await api.POST("/timeline/refresh", { body: { libraryIds: [...libraryIds] } })).libraries;
}
