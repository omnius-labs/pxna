import { type QueryClient, queryOptions } from "@tanstack/react-query";

import { api, unwrap } from "../api/client";
import type { components } from "../api/schema";
import { entryAddress } from "../location";

/// 評価の値。`none` は評価していない状態である。
export type Rating = components["schemas"]["RatingValue"];

/// 1 つの Entry に付けられるタグの数の上限。daemon の上限と同じ値である。
export const MAX_TAGS_PER_ENTRY = 32;

/// タグの語の長さの上限（文字数）。daemon の上限と同じ値である。
export const MAX_TAG_CHARS = 64;

export const RATING_LABELS: Record<Rating, string> = { good: "良い", bad: "悪い", none: "未評価" };

/// `scope` は Library の id か、タイムラインの範囲である。キーは Entry の所在で作り、フォルダ表示とタイムラインで共有する。
export function ratingQuery(scope: string, scopedPath: string) {
    const { libraryId, path } = entryAddress(scope, scopedPath);
    return queryOptions({
        queryKey: ["rating", libraryId, path],
        queryFn: async ({ signal }): Promise<Rating> =>
            unwrap(
                await api.GET("/libraries/{libraryId}/rating", {
                    params: { path: { libraryId }, query: { path } },
                    signal,
                }),
            ).rating,
        staleTime: 60_000,
        retry: false,
    });
}

/// 評価を保存する。保存の結果を待たずに画面へ反映するため、先にキャッシュを書き換え、失敗したら元へ戻して例外を投げる。
export async function putRating(client: QueryClient, scope: string, scopedPath: string, rating: Rating): Promise<void> {
    const { libraryId, path } = entryAddress(scope, scopedPath);
    const key = ratingQuery(libraryId, path).queryKey;
    const previous = client.getQueryData<Rating>(key);
    client.setQueryData(key, rating);
    try {
        unwrap(
            await api.PUT("/libraries/{libraryId}/rating", { params: { path: { libraryId } }, body: { path, rating } }),
        );
    } catch (error) {
        if (previous === undefined) client.removeQueries({ queryKey: key });
        else client.setQueryData(key, previous);
        throw error;
    }
}

export function entryTagsQuery(scope: string, scopedPath: string) {
    const { libraryId, path } = entryAddress(scope, scopedPath);
    return queryOptions({
        queryKey: ["entry-tags", libraryId, path],
        queryFn: async ({ signal }): Promise<string[]> =>
            unwrap(
                await api.GET("/libraries/{libraryId}/tags", {
                    params: { path: { libraryId }, query: { path } },
                    signal,
                }),
            ).tags,
        staleTime: 60_000,
        retry: false,
    });
}

/// 利用者がこれまでに作った語。タグを入力するときの補完に使う。
export const tagVocabularyQuery = queryOptions({
    queryKey: ["tag-vocabulary"],
    queryFn: async ({ signal }): Promise<string[]> => unwrap(await api.GET("/tags", { signal })).tags,
    staleTime: 60_000,
    retry: false,
});

/// Entry のタグを、与えた語の集合に置き換える。保存された結果をキャッシュへ入れ、新しい語を語彙へ足す。
export async function putEntryTags(
    client: QueryClient,
    scope: string,
    scopedPath: string,
    tags: string[],
): Promise<string[]> {
    const { libraryId, path } = entryAddress(scope, scopedPath);
    const saved = unwrap(
        await api.PUT("/libraries/{libraryId}/tags", { params: { path: { libraryId } }, body: { path, tags } }),
    ).tags;
    client.setQueryData(entryTagsQuery(libraryId, path).queryKey, saved);
    client.setQueryData<string[]>(tagVocabularyQuery.queryKey, (current) =>
        current === undefined ? current : [...new Set([...current, ...saved])].sort(),
    );
    return saved;
}

/// 入力した語を、daemon と同じ規則で整える。使えない語は `undefined` にする。
export function normalizeTag(value: string): string | undefined {
    const tag = value.trim();
    // biome-ignore lint/suspicious/noControlCharactersInRegex: 制御文字を含む語を弾く
    if (tag === "" || [...tag].length > MAX_TAG_CHARS || /[\u0000-\u001f\u007f-\u009f]/.test(tag)) return undefined;
    return tag;
}
