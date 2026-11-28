import { type QueryClient, queryOptions } from "@tanstack/react-query";
import { t } from "i18next";

import { ApiError, api, unwrap } from "../api/client";
import type { components } from "../api/schema";
import { joinPath, pathSegments, relativePath, scopedKey } from "../location";
import type { SearchCriteria } from "./listing";
import type { SortPreference } from "./preferences";

export type DetailedEntry = components["schemas"]["FileSystemEntry"];
/// 未取得の属性を仮値へ置き換えず、名前だけでも表示・選択できる。
export type Entry = Omit<DetailedEntry, "metadata"> & {
    metadata?: DetailedEntry["metadata"];
    detailError?: string;
};
/// One page of a listing as the screens hold it: Entries cut from one snapshot by `offset`.
export interface FolderListing {
    entries: Entry[];
    totalCount: number;
    offset: number;
    snapshot: string;
    /// Whether a recursive walk was cut off at the entry limit.
    truncated: boolean;
    summary?: boolean;
    /// Whether the name index answered a search, which is false when a start was walked. Only a search tells it.
    indexed?: boolean;
}

type EntryListRequest = components["schemas"]["EntryListRequest"];
type EntryPage = components["schemas"]["EntryPage"];
type EntryKinds = NonNullable<components["schemas"]["EntryFilter"]["kinds"]>;

/// Every kind, which a recursive listing names so that it returns folders too, as the folder listing and the search always did.
const EVERY_KIND: EntryKinds = ["folder", "archive", "image", "video", "other"];
/// The kinds of the tree: folders, and archives that open like folders.
const FOLDER_KINDS: EntryKinds = ["folder", "archive"];

async function listEntries(body: EntryListRequest, signal?: AbortSignal): Promise<EntryPage> {
    return unwrap(await api.POST("/entries", { body, signal }));
}

/// The offset page of a position. A `locate` names its Entry by its path below the start, which the request names from the Library root.
function offsetPage(libraryId: string, start: string, position: PagePosition): EntryListRequest["page"] {
    if (typeof position === "number") return { kind: "offset", offset: position, limit: PAGE_SIZE };
    const path = joinPath([...pathSegments(start), ...pathSegments(position.locate)]);
    return { kind: "offset", locate: { libraryId: position.locateLibraryId ?? libraryId, path }, limit: PAGE_SIZE };
}

function offsetListing(page: EntryPage, entries: Entry[]): FolderListing {
    return {
        entries,
        totalCount: page.page.totalCount ?? 0,
        offset: page.page.offset ?? 0,
        snapshot: page.page.snapshot ?? "",
        truncated: page.warnings.cutOffStarts.length > 0,
    };
}

/// A folder that is gone, or a path that is not a folder, is a missing start of `listEntries`; the screens keep treating it as a missing folder.
function requireStarts(page: EntryPage): EntryPage {
    if (page.warnings.missingStarts.length > 0) throw new ApiError(404, "NotFound", "the folder does not exist");
    return page;
}
export const capabilitiesQuery = queryOptions({
    queryKey: ["capabilities"],
    queryFn: async () => {
        try {
            return unwrap(await api.GET("/capabilities"));
        } catch (error) {
            if (error instanceof ApiError && [404, 405, 501].includes(error.status)) {
                return { summaryListing: false, entryDetails: false, childProbes: false, performanceLogging: false };
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
    get folder() {
        return t("entryKind.folder");
    },
    get archive() {
        return t("entryKind.archive");
    },
    get image() {
        return t("entryKind.image");
    },
    get video() {
        return t("entryKind.video");
    },
    get other() {
        return t("entryKind.other");
    },
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

/// A position that names an Entry with `locate`, sent in place of `offset`.
export interface LocatePosition {
    locate: string;
    locateLibraryId?: string;
}

/// A page position: an `offset`, or a `locate` that names an Entry.
export type PagePosition = number | LocatePosition;

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
    position: PagePosition,
    options: ListOptions,
    signal?: AbortSignal,
): Promise<FolderListing> {
    const query = options.query?.trim() ?? "";
    const kinds = options.foldersOnly ? FOLDER_KINDS : options.recursive ? EVERY_KIND : undefined;
    const page = requireStarts(
        await listEntries(
            {
                scope: { starts: [{ libraryId, path }], recursive: options.recursive ?? false },
                filter: { ...(query === "" ? {} : { query }), ...(kinds === undefined ? {} : { kinds }) },
                options: { foldersFirst: options.foldersFirst ?? false, includeHidden: options.includeHidden ?? false },
                ...(options.sort === undefined ? {} : { sort: options.sort }),
                ...(options.order === undefined ? {} : { order: options.order }),
                page: offsetPage(libraryId, path, position),
            },
            signal,
        ),
    );
    return offsetListing(
        page,
        (page.items ?? []).map(({ entry }) => entry),
    );
}

/// Library を横断する検索のページ。Entry の path は、Library を含む鍵（`scopedKey`）に置き換える。
export async function fetchSearchPage(
    criteria: SearchCriteria,
    position: PagePosition,
    order: { sort: SortPreference["key"]; order: SortPreference["order"] },
    signal: AbortSignal,
): Promise<FolderListing> {
    const query = criteria.query.trim();
    const page = await listEntries(
        {
            scope: {
                starts: criteria.libraryIds.map((libraryId) => ({ libraryId, path: criteria.path })),
                recursive: criteria.recursive,
            },
            filter: {
                ...(query === "" ? {} : { query }),
                ...(criteria.rating === undefined ? {} : { rating: criteria.rating }),
                ...(criteria.tags.length === 0 ? {} : { tags: [...criteria.tags] }),
                ...(criteria.recursive ? { kinds: EVERY_KIND } : {}),
            },
            options: { foldersFirst: true },
            ...order,
            page: offsetPage(criteria.libraryIds[0] ?? "", criteria.path, position),
        },
        signal,
    );
    return {
        ...offsetListing(
            page,
            (page.items ?? []).map(({ libraryId, entry }) => ({ ...entry, path: scopedKey(libraryId, entry.path) })),
        ),
        indexed: page.warnings.walkedStarts.length === 0,
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
    position: PagePosition,
    options: ListOptions,
    signal: AbortSignal,
): Promise<FolderListing> {
    if (options.recursive || (options.sort !== undefined && options.sort !== "name" && options.sort !== "kind")) {
        return fetchFolderPage(libraryId, path, position, options, signal);
    }
    const capabilities = await client.fetchQuery(capabilitiesQuery);
    signal.throwIfAborted();
    if (!capabilities.summaryListing || !capabilities.entryDetails)
        return fetchFolderPage(libraryId, path, position, options, signal);
    const query = options.query?.trim() ?? "";
    const page = requireStarts(
        await listEntries(
            {
                scope: { starts: [{ libraryId, path }] },
                filter: {
                    ...(query === "" ? {} : { query }),
                    ...(options.foldersOnly ? { kinds: FOLDER_KINDS } : {}),
                },
                options: {
                    fields: "summary",
                    foldersFirst: options.foldersFirst ?? false,
                    includeHidden: options.includeHidden ?? false,
                },
                sort: options.sort === "kind" ? "kind" : "name",
                ...(options.order === undefined ? {} : { order: options.order }),
                page: offsetPage(libraryId, path, position),
            },
            signal,
        ),
    );
    return {
        ...offsetListing(
            page,
            (page.summaries ?? []).map(({ summary: { archive, ...entry } }) => ({
                ...entry,
                ...(entry.kind === "file" ? { file: { archive } } : {}),
            })),
        ),
        summary: true,
    };
}

const DETAILS_PER_REQUEST = 200;

/// Whether the folder at `path`, or one of the folders above it, is an archive, which the server tells by the archive hint of the details.
/// A search from inside an archive walks by design, so the web client does not report a missing index for it.
export function startInArchiveQuery(libraryId: string, path: string) {
    return queryOptions({
        queryKey: ["entries", libraryId, path, "in-archive"],
        queryFn: async ({ signal }) => {
            const segments = path.split("/").filter((segment) => segment !== "");
            const paths = segments.map((_, index) => `/${segments.slice(0, index + 1).join("/")}`);
            // The server takes at most 200 paths in one request.
            for (let from = 0; from < paths.length; from += DETAILS_PER_REQUEST) {
                const results = await fetchEntryDetails(
                    libraryId,
                    paths.slice(from, from + DETAILS_PER_REQUEST),
                    signal,
                );
                if (results.some((result) => result.detail?.file?.archive === true)) return true;
            }
            return false;
        },
        staleTime: 60_000,
    });
}

export async function fetchEntryDetails(libraryId: string, paths: string[], signal: AbortSignal) {
    return unwrap(
        await api.POST("/entries/metadata", {
            body: { places: paths.map((path) => ({ libraryId, path })) },
            signal,
        }),
    ).items;
}
