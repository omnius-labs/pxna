import { type QueryClient, queryOptions } from "@tanstack/react-query";

import { ApiError, api, unwrap } from "../api/client";
import type { components } from "../api/schema";
import { relativePath, scopedKey } from "../location";
import type { SearchCriteria } from "./listing";
import type { SortPreference } from "./preferences";

export type DetailedEntry = components["schemas"]["FileSystemEntry"];
/// 未取得の属性を仮値へ置き換えず、名前だけでも表示・選択できる。
export type Entry = Omit<DetailedEntry, "metadata"> & {
    metadata?: DetailedEntry["metadata"];
    detailError?: string;
};
export type FolderListing = Omit<components["schemas"]["FolderListing"], "entries"> & {
    entries: Entry[];
    summary?: boolean;
};
export const capabilitiesQuery = queryOptions({
    queryKey: ["capabilities"],
    queryFn: async () => {
        try {
            return unwrap(await api.GET("/capabilities"));
        } catch (error) {
            if (error instanceof ApiError && [404, 405, 501].includes(error.status)) {
                return { summaryListing: false, entryDetails: false, childProbes: false };
            }
            throw error;
        }
    },
    staleTime: Infinity,
    retry: false,
});

/// タイルの見た目とダブルクリックの動作を決める、Entry の種類。
export type EntryKind = "folder" | "archive" | "image" | "video" | "other";

const PAGE_SIZE = 200;

/// 検索の起点から見た親フォルダ。直下の Entry は「.」とする。
export function entryPlace(base: string, entry: Entry): string {
    const path = relativePath(base, entry.path);
    const last = path.lastIndexOf("/");
    return last < 0 ? "." : path.slice(0, last);
}

/// フォルダ扱いの Entry かどうか。archive はファイル詳細の `archive` のヒントで判定する。
export function isFolderLike(entry: Entry): boolean {
    return entry.kind === "folder" || entry.file?.archive === true;
}

/// 種類の表示名。
export const ENTRY_KIND_LABELS: Record<EntryKind, string> = {
    folder: "フォルダ",
    archive: "アーカイブ",
    image: "画像",
    video: "動画",
    other: "ファイル",
};

export function entryKind(entry: Entry): EntryKind {
    if (entry.kind === "folder") {
        return "folder";
    }
    if (entry.file?.archive === true) {
        return "archive";
    }
    // 詳細が未取得の間だけ名前を表示用のヒントにする。archive の判定は常にサーバのヒントを使う。
    if (!entry.metadata) {
        if (/\.(jpe?g|png|gif|webp|avif|bmp|tiff?|heic|heif)$/i.test(entry.name)) return "image";
        if (/\.(mp4|m4v|mov|webm|mkv|avi|mpeg|mpg|m2ts|ts)$/i.test(entry.name)) return "video";
    }
    const contentType = entry.metadata?.contentType ?? "";
    if (contentType.startsWith("image/")) {
        return "image";
    }
    if (contentType.startsWith("video/")) {
        return "video";
    }
    return "other";
}

/// サムネイルを要求する Entry。フォルダと archive は代表サムネイルで、`NotFound` ならアイコンのままにする。
export function isThumbnailable(entry: Entry): boolean {
    return entryKind(entry) !== "other";
}

interface ListOptions {
    includeHidden?: boolean;
    query?: string;
    recursive?: boolean;
    foldersFirst?: boolean;
    foldersOnly?: boolean;
    sort?: SortPreference["key"];
    order?: SortPreference["order"];
}

export async function fetchFolderPage(
    libraryId: string,
    path: string,
    offset: number,
    options: ListOptions,
    signal?: AbortSignal,
): Promise<FolderListing> {
    const query = { path, limit: PAGE_SIZE, offset, ...options };
    return unwrap(
        await api.GET("/libraries/{libraryId}/folders/entries", { params: { path: { libraryId }, query }, signal }),
    );
}

/// Library を横断する検索のページ。Entry の path は、Library を含む鍵（`scopedKey`）に置き換える。
export async function fetchSearchPage(
    criteria: SearchCriteria,
    offset: number,
    order: { sort: SortPreference["key"]; order: SortPreference["order"] },
    signal: AbortSignal,
): Promise<FolderListing> {
    const query = criteria.query.trim();
    const page = unwrap(
        await api.GET("/search/entries", {
            params: {
                query: {
                    libraryIds: [...criteria.libraryIds],
                    ...(criteria.path === "/" ? {} : { path: criteria.path }),
                    recursive: criteria.recursive,
                    ...(query === "" ? {} : { query }),
                    ...(criteria.rating === undefined ? {} : { rating: criteria.rating }),
                    ...(criteria.tags.length === 0 ? {} : { tag: [...criteria.tags] }),
                    ...order,
                    foldersFirst: true,
                    offset,
                    limit: PAGE_SIZE,
                },
            },
            signal,
        }),
    );
    return {
        entries: page.entries.map(({ libraryId, entry }) => ({ ...entry, path: scopedKey(libraryId, entry.path) })),
        totalCount: page.totalCount,
        snapshot: page.snapshot,
        truncated: page.truncatedLibraryIds.length > 0,
    };
}

/// ツリーの子。フォルダ扱いの Entry だけで、件数が少ないので、すべてのページを読んでから返す。
export function childFoldersQuery(libraryId: string, path: string) {
    return queryOptions({
        queryKey: ["entries", libraryId, path, "tree"],
        queryFn: async ({ signal }) => {
            const entries: Entry[] = [];
            let offset = 0;
            let totalCount = Infinity;
            let snapshot: string | undefined;
            while (offset < totalCount) {
                const page = await fetchFolderPage(libraryId, path, offset, { foldersOnly: true }, signal);
                if (snapshot !== undefined && snapshot !== page.snapshot) {
                    entries.length = 0;
                    offset = 0;
                    totalCount = Infinity;
                    snapshot = undefined;
                    continue;
                }
                snapshot = page.snapshot;
                entries.push(...page.entries);
                offset += page.entries.length;
                totalCount = page.totalCount;
                if (page.entries.length === 0) break;
            }
            return entries;
        },
    });
}

/// サイズ順・日時順・再帰は従来の応答を使う。提供済み機能の失敗は隠さない。
export async function fetchListingPage(
    client: QueryClient,
    libraryId: string,
    path: string,
    offset: number,
    options: ListOptions,
    signal: AbortSignal,
): Promise<FolderListing> {
    if (options.recursive || (options.sort !== undefined && options.sort !== "name" && options.sort !== "kind")) {
        return fetchFolderPage(libraryId, path, offset, options, signal);
    }
    const capabilities = await client.fetchQuery(capabilitiesQuery);
    signal.throwIfAborted();
    if (!capabilities.summaryListing || !capabilities.entryDetails)
        return fetchFolderPage(libraryId, path, offset, options, signal);
    const summaryOptions = { ...options };
    delete summaryOptions.recursive;
    const page = unwrap(
        await api.GET("/libraries/{libraryId}/folders/summaries", {
            params: {
                path: { libraryId },
                query: {
                    path,
                    offset,
                    limit: PAGE_SIZE,
                    ...summaryOptions,
                    sort: options.sort === "kind" ? "kind" : "name",
                },
            },
            signal,
        }),
    );
    return {
        ...page,
        summary: true,
        entries: page.entries.map(({ archive, ...entry }) => ({
            ...entry,
            ...(entry.kind === "file" ? { file: { archive } } : {}),
        })),
    };
}

export async function fetchEntryDetails(libraryId: string, paths: string[], signal: AbortSignal) {
    return unwrap(
        await api.POST("/libraries/{libraryId}/entries/details", {
            params: { path: { libraryId } },
            body: { paths },
            signal,
        }),
    ).results;
}
