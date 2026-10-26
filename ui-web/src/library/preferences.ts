import { type QueryClient, queryOptions, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useSyncExternalStore } from "react";
import { toast } from "sonner";

import { api, unwrap } from "../api/client";
import type { components } from "../api/schema";

export type Preferences = components["schemas"]["Preferences"];
export type DisplaySet = components["schemas"]["DisplaySet"];
export type SortPreference = components["schemas"]["SortPreference"];

/// 広い画面と狭い画面を分ける画面の幅（CSS px）。この幅以上が広い画面である。
export const NARROW_BREAKPOINT = 768;

/// 操作が止まってから、表示設定をサーバへ書き戻すまでの時間。続けた操作を 1 回にまとめる。
export const WRITE_DELAY_MS = 500;

/// 保存したことがないときにサーバが返す値と同じ既定値。取得に失敗したときも、この値で動かす。
export const DEFAULT_PREFERENCES: Preferences = {
    wide: { tileSize: 3, thumbnailShape: "crop", view: "grid", originalImage: "auto" },
    narrow: { tileSize: 3, thumbnailShape: "crop", view: "grid", originalImage: "onZoom" },
    sort: { key: "name", order: "asc" },
};

/// 取得の問い合わせ。書き戻しの保留中の値を再取得で上書きしないよう、いつまでも新鮮として扱い、フォーカスでも取り直さない。
export const preferencesQuery = queryOptions({
    queryKey: ["preferences"],
    queryFn: async () => unwrap(await api.GET("/preferences")),
    staleTime: Infinity,
    refetchOnWindowFocus: false,
});

/// 画面の幅の区分に合う組を選ぶ。
export function selectSet(preferences: Preferences, narrow: boolean): DisplaySet {
    return narrow ? preferences.narrow : preferences.wide;
}

/// 画面の幅の区分に合う組の一部を置き換える。もう一方の組と並べ替えは変えない。
export function replaceSet(preferences: Preferences, narrow: boolean, patch: Partial<DisplaySet>): Preferences {
    return narrow
        ? { ...preferences, narrow: { ...preferences.narrow, ...patch } }
        : { ...preferences, wide: { ...preferences.wide, ...patch } };
}

/// 組の外にある並べ替えの一部を置き換える。
export function replaceSort(preferences: Preferences, patch: Partial<SortPreference>): Preferences {
    return { ...preferences, sort: { ...preferences.sort, ...patch } };
}

/// タイムラインに並べる Library を置き換える。空の選択は保存せず、今の値のままにする。
export function replaceTimelineLibraries(preferences: Preferences, libraryIds: readonly string[]): Preferences {
    return libraryIds.length === 0 ? preferences : { ...preferences, timelineLibraries: [...libraryIds] };
}

interface WriterOptions {
    delayMs?: number;
    /// サーバへ全体を書き戻す。失敗したら例外を投げる。
    put?: (preferences: Preferences) => Promise<Preferences>;
    /// 書き戻しの失敗を利用者に知らせる。
    notify?: (message: string) => void;
}

async function putPreferences(preferences: Preferences): Promise<Preferences> {
    // 画面を閉じる間際の書き戻しも届くよう、keepalive を付ける。
    return unwrap(await api.PUT("/preferences", { body: preferences, keepalive: true }));
}

/// 表示設定の変更を受け取り、キャッシュを先に書き換えて、操作が止まった時点で全体を 1 回だけ書き戻す。
/// 書き戻しに失敗したら、最後に保存できた値へキャッシュを戻して知らせる。
export class PreferencesWriter {
    private timer: ReturnType<typeof setTimeout> | undefined;
    private saving = false;
    /// 最後に保存できた値。最初の変更を受けたときのキャッシュの値から始める。
    private saved: Preferences | undefined;

    constructor(
        private readonly client: QueryClient,
        private readonly options: WriterOptions = {},
    ) {}

    /// 今の値を `change` で変えた値をキャッシュへ入れ、書き戻しを予約する。
    update(change: (current: Preferences) => Preferences): void {
        const current = this.client.getQueryData(preferencesQuery.queryKey) ?? DEFAULT_PREFERENCES;
        this.saved ??= current;
        this.client.setQueryData(preferencesQuery.queryKey, change(current));
        this.schedule();
    }

    /// 保留中の書き戻しを待たずに送る。画面を閉じるときに使う。
    flush(): void {
        if (this.timer !== undefined) {
            clearTimeout(this.timer);
            this.timer = undefined;
            void this.write();
        }
    }

    /// 保留中の書き戻しを捨てる。
    dispose(): void {
        clearTimeout(this.timer);
        this.timer = undefined;
    }

    private schedule(): void {
        clearTimeout(this.timer);
        this.timer = setTimeout(() => {
            this.timer = undefined;
            void this.write();
        }, this.options.delayMs ?? WRITE_DELAY_MS);
    }

    private async write(): Promise<void> {
        // 送信中に次の操作が止まったときは、送信が終わってから送り直す。
        if (this.saving) {
            this.schedule();
            return;
        }
        const next = this.client.getQueryData(preferencesQuery.queryKey);
        if (next === undefined) {
            return;
        }
        this.saving = true;
        try {
            this.saved = await (this.options.put ?? putPreferences)(next);
        } catch {
            clearTimeout(this.timer);
            this.timer = undefined;
            if (this.saved !== undefined) {
                this.client.setQueryData(preferencesQuery.queryKey, this.saved);
            }
            (this.options.notify ?? ((message) => void toast.error(message)))("表示設定を保存できませんでした。");
        } finally {
            this.saving = false;
        }
    }
}

const writers = new WeakMap<QueryClient, PreferencesWriter>();

/// `QueryClient` ごとに 1 つの書き戻し。設定のメニューとタイムラインの Library の選択など、複数の場所から変えても、まとめと巻き戻しの基準を 1 つにするために共有する。
/// 画面を閉じるときに、保留中の書き戻しを送る。
export function preferencesWriterFor(client: QueryClient): PreferencesWriter {
    let writer = writers.get(client);
    if (writer === undefined) {
        const created = new PreferencesWriter(client);
        window.addEventListener("pagehide", () => created.flush());
        writers.set(client, created);
        writer = created;
    }
    return writer;
}

const NARROW_QUERY = `(max-width: ${NARROW_BREAKPOINT - 0.02}px)`;

function subscribeNarrow(listener: () => void): () => void {
    const media = window.matchMedia(NARROW_QUERY);
    media.addEventListener("change", listener);
    return () => media.removeEventListener("change", listener);
}

/// 画面の幅が狭い画面の区分かどうか。幅が境界をまたぐと再描画する。
export function useNarrowScreen(): boolean {
    return useSyncExternalStore(subscribeNarrow, () => window.matchMedia(NARROW_QUERY).matches);
}

/// 今使っている組と並べ替えを返し、変更を書き戻す。取得が終わっている前提で、なければ既定値を使う。
export function usePreferences() {
    const client = useQueryClient();
    const { data } = useQuery(preferencesQuery);
    const narrow = useNarrowScreen();
    const preferences = data ?? DEFAULT_PREFERENCES;

    const writer = preferencesWriterFor(client);
    useEffect(() => () => writer.flush(), [writer]);

    return {
        preferences,
        narrow,
        set: selectSet(preferences, narrow),
        sort: preferences.sort,
        /// 今使っている組の一部を変える。
        updateSet: (patch: Partial<DisplaySet>) => writer.update((current) => replaceSet(current, narrow, patch)),
        updateSort: (patch: Partial<SortPreference>) => writer.update((current) => replaceSort(current, patch)),
        /// タイムラインに並べる Library を保存する。空にはしない。
        updateTimelineLibraries: (libraryIds: readonly string[]) =>
            writer.update((current) => replaceTimelineLibraries(current, libraryIds)),
    };
}
