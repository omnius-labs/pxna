import { queryOptions } from "@tanstack/react-query";

import { api, unwrap } from "../api/client";
import type { components } from "../api/schema";

export type MediaInfo = components["schemas"]["MediaInfo"];

export const librariesQuery = queryOptions({
    queryKey: ["libraries"],
    queryFn: async () => unwrap(await api.GET("/libraries")).libraries,
});

export function folderQuery(libraryId: string, path: string) {
    return queryOptions({
        queryKey: ["folder", libraryId, path],
        queryFn: async ({ signal }) =>
            unwrap(
                await api.GET("/libraries/{libraryId}/folders", {
                    params: { path: { libraryId }, query: { path } },
                    signal,
                }),
            ).folder,
    });
}

/// 画像か動画のメディア情報。動画の再生と情報パネルが同じキーで共有し、同時の要求と結果をまとめる。
export function mediaInfoQuery(libraryId: string, path: string) {
    return queryOptions({
        queryKey: ["media-info", libraryId, path],
        queryFn: async ({ signal }): Promise<MediaInfo> =>
            unwrap(
                await api.GET("/libraries/{libraryId}/media-info", {
                    params: { path: { libraryId }, query: { path } },
                    signal,
                }),
            ).mediaInfo,
        staleTime: 0,
        retry: false,
    });
}
