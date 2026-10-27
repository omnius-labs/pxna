import { type QueryClient, QueryObserver, type QueryObserverResult, useQueryClient } from "@tanstack/react-query";
import { t } from "i18next";
import { useEffect, useMemo, useState, useSyncExternalStore } from "react";
import { describeError } from "../i18n/errors";
import { CROSS_LIBRARY_SCOPE } from "../location";
import { type Entry, type FolderListing, fetchEntryDetails, fetchListingPage, fetchSearchPage } from "./entries";
import type { SortPreference } from "./preferences";
import type { Rating } from "./ratingTags";
import { refreshTimeline } from "./timeline";

export const PAGE_SIZE = 200;
export type PageStatus = "idle" | "loading" | "loaded" | "error";

/// 検索の条件。`libraryIds` の順は、同じ並びの値の Entry を置く順になる。
export interface SearchCriteria {
    /// 探す Library の id。空ではない。
    libraryIds: readonly string[];
    /// 起点の Library 相対パス。Library が 1 つのときだけ `/` 以外になる。
    path: string;
    recursive: boolean;
    query: string;
    /// 評価での絞り込み。`none` は未評価の画像と動画を選ぶ。
    rating?: Rating;
    /// すべてを持つ Entry を選ぶタグ。
    tags: readonly string[];
}

/// 一覧の出どころ。フォルダの直下か、Library を横断する検索である。
/// 検索の Entry の path は、`scopedKey` が作る Library を含む鍵である。
export type ListingSource =
    | { kind: "folder"; libraryId: string; path: string }
    | { kind: "search"; criteria: SearchCriteria };

/// 検索の条件が 1 つでもあるか。名前の語だけでなく、評価とタグも数える。
export function hasSearchConditions(criteria: SearchCriteria): boolean {
    return criteria.query.trim() !== "" || criteria.rating !== undefined || criteria.tags.length > 0;
}

/// 一覧の出どころの範囲。フォルダは Library の id、検索は `CROSS_LIBRARY_SCOPE` である。
export function sourceScope(source: ListingSource): string {
    return source.kind === "folder" ? source.libraryId : CROSS_LIBRARY_SCOPE;
}

export interface Listing {
    truncated: boolean;
    totalCount: number | undefined;
    readonly version: number;
    snapshot: string | undefined;
    entryAt(index: number): Entry | undefined;
    pageStatus(pageIndex: number): PageStatus;
    pageError(pageIndex: number): Error | null;
    loadPages(pageIndexes: readonly number[], signal: AbortSignal): Promise<void>;
    retry(pageIndex: number): void;
    /// 選択や情報表示の対象を、画面外でも先に詳細取得する。
    setDetailPriority?(owner: object, paths: readonly string[]): void;
    releaseDetailPriority?(owner: object): void;
    /// 存在しうる最小の位置。省略すると 0 である。タイムラインは、前に読む続きがなければ読み込み済みの先頭になる。
    readonly firstIndex?: number;
    /// 読み込み済みの最小の位置。省略すると 0 である。フィルムストリップは、この位置から並べる。
    readonly loadedFrom?: number;
    /// Library 相対パスの Entry の位置を、読み込み済みの範囲から探す。持つ一覧は、ページを順に探さずにこれを使う。
    locate?(path: string): number | undefined;
    /// 位置の Entry が読み込み済みになるまで読む。ページ全体を読まず、その位置に届くまでだけ読み足す。
    ensureIndex?(index: number, signal: AbortSignal): Promise<void>;
}

/// 最後に届いたページと異なる版のページだけを捨てるために使う。
export function staleSnapshotPages(pages: ReadonlyMap<number, FolderListing>, snapshot: string): number[] {
    return [...pages].filter(([, page]) => page.snapshot !== snapshot).map(([index]) => index);
}

export function folderPageKey(
    libraryId: string,
    path: string,
    sort: SortPreference,
    pageIndex: number,
    foldersOnly = false,
) {
    return [
        "entries",
        libraryId,
        path,
        foldersOnly ? "tree" : "page",
        sort.key,
        sort.order,
        sort.key === "name" || sort.key === "kind" ? "summary-or-legacy" : "detailed",
        pageIndex,
    ] as const;
}

export function searchPageKey(criteria: SearchCriteria, sort: SortPreference, pageIndex: number) {
    return [
        "search",
        criteria.libraryIds.join("\n"),
        criteria.path,
        criteria.recursive,
        criteria.query.trim(),
        criteria.rating,
        criteria.tags.join("\n"),
        sort.key,
        sort.order,
        pageIndex,
    ] as const;
}

export class ListingSnapshotChanged extends Error {
    constructor() {
        super(t("errors.listingCancelled"));
    }
}

/// 観測は表示範囲と明示的な読み込みの和に限る。観測を外すと Query が渡した signal で要求も止まる。
class FolderListingStore implements Listing {
    truncated = false;
    totalCount: number | undefined;
    snapshot: string | undefined;
    private pages = new Map<number, FolderListing>();
    private results = new Map<number, QueryObserverResult<FolderListing, Error>>();
    private observers = new Map<number, { observer: QueryObserver<FolderListing, Error>; unsubscribe: () => void }>();
    private owners = new Map<object, ReadonlySet<number>>();
    private listeners = new Set<() => void>();
    private revision = 0;
    private generation = 0;
    private details = new Map<string, Entry>();
    private detailPages = new Set<number>();
    private priorities = new Map<object, readonly string[]>();
    private pathPages = new Map<string, number>();
    private detailJob: { page: number; controller: AbortController } | undefined;
    private detailScheduled = false;
    private mismatches = new Set<string>();

    /// 詳細を補う要求の Library。検索は軽量一覧を使わないので、補う詳細はない。
    private libraryId: string;

    constructor(
        private client: QueryClient,
        private source: ListingSource,
        private sort: SortPreference,
        private foldersOnly = false,
    ) {
        this.libraryId = sourceScope(source);
    }

    /// この一覧が、その Library の Entry を含みうるか。
    covers(libraryId: string): boolean {
        return this.source.kind === "folder"
            ? this.source.libraryId === libraryId
            : this.source.criteria.libraryIds.includes(libraryId);
    }

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
    entryAt = (index: number) => {
        if (index < 0 || index >= (this.totalCount ?? 0)) return undefined;
        const entry = this.pages.get(Math.floor(index / PAGE_SIZE))?.entries[index % PAGE_SIZE];
        return entry === undefined ? undefined : (this.details.get(entry.path) ?? entry);
    };
    locate = (path: string) => {
        const pageIndex = this.pathPages.get(path);
        if (pageIndex === undefined) return undefined;
        const index = this.pages.get(pageIndex)?.entries.findIndex((entry) => entry.path === path) ?? -1;
        return index < 0 ? undefined : pageIndex * PAGE_SIZE + index;
    };
    pageStatus = (index: number): PageStatus => {
        if (this.pages.has(index)) return "loaded";
        const result = this.results.get(index);
        return result?.isError ? "error" : result?.isFetching ? "loading" : "idle";
    };
    pageError = (index: number) => this.results.get(index)?.error ?? null;

    setWanted(owner: object, pages: ReadonlySet<number>) {
        this.owners.set(owner, pages);
        this.syncObservers();
    }
    release(owner: object) {
        this.owners.delete(owner);
        this.syncObservers();
    }
    private key(index: number): readonly unknown[] {
        return this.source.kind === "folder"
            ? folderPageKey(this.source.libraryId, this.source.path, this.sort, index, this.foldersOnly)
            : searchPageKey(this.source.criteria, this.sort, index);
    }
    private fetchPage(index: number, signal: AbortSignal): Promise<FolderListing> {
        const order = { sort: this.sort.key, order: this.sort.order };
        if (this.source.kind === "search")
            return fetchSearchPage(this.source.criteria, index * PAGE_SIZE, order, signal);
        return fetchListingPage(
            this.client,
            this.source.libraryId,
            this.source.path,
            index * PAGE_SIZE,
            { foldersFirst: true, ...(this.foldersOnly ? { foldersOnly: true } : {}), ...order },
            signal,
        );
    }
    private syncObservers() {
        const wanted = new Set<number>();
        for (const pages of this.owners.values()) for (const index of pages) wanted.add(index);
        if (this.owners.size > 0) wanted.add(0);
        for (const [index, observed] of this.observers) {
            if (!wanted.has(index)) {
                observed.unsubscribe();
                observed.observer.destroy();
                this.observers.delete(index);
                if (!this.pages.has(index) && !this.results.get(index)?.isError) this.results.delete(index);
            }
        }
        for (const index of wanted) {
            if (index < 0 || !Number.isInteger(index) || this.observers.has(index)) continue;
            const observer = new QueryObserver<FolderListing, Error>(this.client, {
                queryKey: this.key(index),
                queryFn: ({ signal }) => this.fetchPage(index, signal),
                staleTime: Infinity,
                retry: false,
                retryOnMount: false,
                refetchOnMount: false,
                refetchOnWindowFocus: false,
                refetchOnReconnect: false,
            });
            /// 登録を先に済ませ、同期通知で別の版が届いても二重の観測を作らない。
            const observed = { observer, unsubscribe: () => {} };
            this.observers.set(index, observed);
            observed.unsubscribe = observer.subscribe((result) => this.receive(index, result));
            this.receive(index, observer.getCurrentResult());
        }
        this.syncDetails();
        this.emit();
    }
    private receive(index: number, result: QueryObserverResult<FolderListing, Error>) {
        this.results.set(index, result);
        if (result.data && this.pages.get(index) !== result.data) {
            const page = result.data;
            this.detailPages.delete(index);
            for (const entry of page.entries) this.details.delete(entry.path);
            if (this.snapshot !== undefined && this.snapshot !== page.snapshot) {
                this.generation += 1;
                this.detailJob?.controller.abort();
                this.details.clear();
                this.detailPages.clear();
                this.pathPages.clear();
                this.mismatches.clear();
                for (const stale of staleSnapshotPages(this.pages, page.snapshot)) this.pages.delete(stale);
                /// 古い版の応答が後から届いて版を戻さないよう、未完了の要求も止めて観測を作り直す。
                for (const [other, observed] of this.observers) {
                    if (other === index) continue;
                    observed.unsubscribe();
                    observed.observer.destroy();
                    this.observers.delete(other);
                }
                for (const query of this.client.getQueryCache().findAll({ queryKey: this.key(0).slice(0, -1) })) {
                    const other = query.queryKey.at(-1) as number;
                    if (other !== index) {
                        this.client.removeQueries({ queryKey: query.queryKey, exact: true });
                        this.results.delete(other);
                    }
                }
            }
            this.snapshot = page.snapshot;
            this.totalCount = page.totalCount;
            this.truncated = page.truncated;
            this.pages.set(index, page);
            for (const entry of page.entries) this.pathPages.set(entry.path, index);
            this.emit();
            this.syncObservers();
        } else {
            this.emit();
        }
    }
    setDetailPriority = (owner: object, paths: readonly string[]) => {
        this.priorities.set(owner, paths);
        this.syncDetails();
    };
    releaseDetailPriority = (owner: object) => {
        this.priorities.delete(owner);
        this.syncDetails();
    };
    private desiredDetailPages(): number[] {
        const wanted = new Set<number>();
        for (const paths of this.priorities.values())
            for (const path of paths) {
                const page = this.pathPages.get(path);
                if (page !== undefined) wanted.add(page);
            }
        if (!this.foldersOnly) for (const pages of this.owners.values()) for (const page of pages) wanted.add(page);
        return [...wanted];
    }
    private syncDetails() {
        const wanted = this.desiredDetailPages();
        if (this.detailJob && !wanted.includes(this.detailJob.page)) this.detailJob.controller.abort();
        if (this.detailScheduled) return;
        this.detailScheduled = true;
        queueMicrotask(() => {
            this.detailScheduled = false;
            this.startDetails();
        });
    }
    private startDetails() {
        if (this.detailJob) return;
        const index = this.desiredDetailPages().find(
            (page) => this.pages.get(page)?.summary && !this.detailPages.has(page),
        );
        if (index === undefined) return;
        const page = this.pages.get(index)!;
        if (page.entries.length === 0) {
            this.detailPages.add(index);
            return;
        }
        const snapshot = this.snapshot;
        const generation = this.generation;
        const job = { page: index, controller: new AbortController() };
        this.detailJob = job;
        void fetchEntryDetails(
            this.libraryId,
            page.entries.map((entry) => entry.path),
            job.controller.signal,
        )
            .then((results) => {
                if (
                    job.controller.signal.aborted ||
                    generation !== this.generation ||
                    snapshot !== this.snapshot ||
                    this.pages.get(index) !== page
                )
                    return;
                const byPath = new Map(results.map((result) => [result.path, result]));
                let changed = false;
                for (const summary of page.entries) {
                    const result = byPath.get(summary.path);
                    if (result?.detail) {
                        if (
                            result.detail.kind !== summary.kind ||
                            !!result.detail.file?.archive !== !!summary.file?.archive
                        ) {
                            if (!this.mismatches.has(summary.path)) {
                                this.mismatches.add(summary.path);
                                changed = true;
                            }
                            continue;
                        }
                        this.details.set(summary.path, result.detail);
                    } else
                        this.details.set(summary.path, {
                            ...summary,
                            detailError: result?.error ?? t("errors.detailFailed"),
                        });
                }
                this.detailPages.add(index);
                this.emit();
                if (changed) void this.refresh();
            })
            .catch((error: unknown) => {
                if (
                    job.controller.signal.aborted ||
                    generation !== this.generation ||
                    snapshot !== this.snapshot ||
                    this.pages.get(index) !== page
                )
                    return;
                for (const entry of page.entries)
                    this.details.set(entry.path, {
                        ...entry,
                        detailError: describeError(error, t("errors.detailFailed")),
                    });
                this.detailPages.add(index);
                this.emit();
            })
            .finally(() => {
                if (this.detailJob === job) this.detailJob = undefined;
                this.syncDetails();
            });
    }
    retryDetails = (paths: readonly string[]) => {
        for (const path of paths) {
            const page = this.pathPages.get(path);
            if (page !== undefined) this.detailPages.delete(page);
            this.details.delete(path);
        }
        this.emit();
        this.syncDetails();
    };
    get active() {
        return this.owners.size > 0;
    }
    refresh = async () => {
        await this.observers.get(0)?.observer.refetch();
    };

    retry = (index: number) => {
        void this.observers.get(index)?.observer.refetch();
    };

    loadPages = async (indexes: readonly number[], signal: AbortSignal): Promise<void> => {
        signal.throwIfAborted();
        const generation = this.generation;
        const owner = {};
        const wanted = new Set(indexes);
        await new Promise<void>((resolve, reject) => {
            let unsubscribe = () => {};
            const cleanup = () => {
                unsubscribe();
                signal.removeEventListener("abort", aborted);
                this.release(owner);
            };
            const aborted = () => {
                cleanup();
                reject(signal.reason);
            };
            const check = () => {
                if (generation !== this.generation) {
                    cleanup();
                    reject(new ListingSnapshotChanged());
                    return;
                }
                const failed = indexes.find((index) => this.pageStatus(index) === "error");
                if (failed !== undefined) {
                    cleanup();
                    reject(this.pageError(failed));
                    return;
                }
                if (indexes.every((index) => this.pageStatus(index) === "loaded")) {
                    cleanup();
                    resolve();
                }
            };
            unsubscribe = this.subscribe(check);
            signal.addEventListener("abort", aborted, { once: true });
            this.setWanted(owner, wanted);
            check();
        });
    };
}

const stores = new WeakMap<QueryClient, Map<string, FolderListingStore>>();

function sourceKey(source: ListingSource, sort: SortPreference, foldersOnly: boolean): string {
    return source.kind === "folder"
        ? JSON.stringify([source.libraryId, source.path, sort.key, sort.order, foldersOnly])
        : JSON.stringify([CROSS_LIBRARY_SCOPE, ...searchPageKey(source.criteria, sort, 0).slice(0, -1)]);
}

/// 同じ出どころと並べ方の購読を共有する。ツリーは foldersOnly で一覧から分離する。
export function listingStore(client: QueryClient, source: ListingSource, sort: SortPreference, foldersOnly = false) {
    let listings = stores.get(client);
    if (!listings) {
        listings = new Map();
        stores.set(client, listings);
    }
    const key = sourceKey(source, sort, foldersOnly);
    let listing = listings.get(key);
    if (!listing) {
        listing = new FolderListingStore(client, source, sort, foldersOnly);
        listings.set(key, listing);
    }
    return listing;
}

export function folderListingStore(
    client: QueryClient,
    libraryId: string,
    path: string,
    sort: SortPreference,
    foldersOnly = false,
) {
    return listingStore(client, { kind: "folder", libraryId, path }, sort, foldersOnly);
}

/// 詳細だけの失敗を再試行し、名前のsnapshotと並びを維持する。
export function retryEntryDetails(client: QueryClient, libraryId: string, paths: readonly string[]) {
    for (const [key, store] of stores.get(client) ?? []) {
        if (JSON.parse(key)[0] === libraryId) store.retryDetails(paths);
    }
}

/// 表示中の一覧は古いページを残して先頭から再取得し、未表示のキャッシュは捨てる。その Library を探す検索も同じく読み直す。
export async function refreshLibrary(client: QueryClient, libraryId: string): Promise<void> {
    await Promise.all([
        client.cancelQueries({ queryKey: ["entries", libraryId] }),
        client.cancelQueries({ queryKey: ["search"] }),
    ]);
    const timeline = refreshTimeline(client, [libraryId]);
    const active: FolderListingStore[] = [];
    for (const [key, store] of stores.get(client) ?? []) {
        if (!store.covers(libraryId)) continue;
        if (store.active) active.push(store);
        else stores.get(client)?.delete(key);
    }
    client.removeQueries({ queryKey: ["entries", libraryId], type: "inactive" });
    client.removeQueries({ queryKey: ["search"], type: "inactive" });
    await Promise.all([
        ...active.map((store) => store.refresh()),
        client.invalidateQueries({
            queryKey: ["entries", libraryId],
            predicate: (query) => query.queryKey[3] === "tree",
        }),
        client.invalidateQueries({ queryKey: ["folder", libraryId] }),
        timeline,
    ]);
}

/// 出どころの一覧を購読し、`wantedPages` のページを読む。
export function useListing(source: ListingSource, sort: SortPreference, wantedPages: ReadonlySet<number>): Listing {
    const client = useQueryClient();
    const key = sourceKey(source, sort, false);
    // biome-ignore lint/correctness/useExhaustiveDependencies: 出どころと並べ方は key に畳み、内容が同じなら同じ購読を使う
    const store = useMemo(() => listingStore(client, source, { key: sort.key, order: sort.order }), [client, key]);
    const [owner] = useState(() => ({}));
    const pagesKey = [...wantedPages].sort((a, b) => a - b).join(",");
    useEffect(() => {
        /// StrictMode の仮の設置では送らず、実際に残る観測だけが要求を始める。
        let active = true;
        queueMicrotask(() => {
            if (active) store.setWanted(owner, new Set(pagesKey === "" ? [] : pagesKey.split(",").map(Number)));
        });
        return () => {
            active = false;
        };
    }, [store, owner, pagesKey]);
    useEffect(() => () => store.release(owner), [store, owner]);
    useSyncExternalStore(store.subscribe, store.getRevision);
    return store;
}

export function useFolderListing(
    libraryId: string,
    path: string,
    sort: SortPreference,
    wantedPages: ReadonlySet<number>,
): Listing {
    return useListing({ kind: "folder", libraryId, path }, sort, wantedPages);
}
