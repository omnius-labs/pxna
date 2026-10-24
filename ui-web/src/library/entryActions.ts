import type { QueryClient } from "@tanstack/react-query";

import { ApiError, api, unwrap } from "../api/client";
import type { components } from "../api/schema";
import { entryAddress, groupByLibrary } from "../location";
import type { Entry } from "./entries";
import { refreshLibrary } from "./listing";

export type Operation = components["schemas"]["Operation"];

export function canMutate(
    library: { readonly: boolean },
    entry: { metadata?: { readonly: boolean } } | undefined,
): boolean {
    return !library.readonly && entry?.metadata?.readonly === false;
}
export function validateName(name: string): "empty" | "slash" | "dot" | undefined {
    if (name === "") return "empty";
    if (name.includes("/")) return "slash";
    if (name === "." || name === "..") return "dot";
}
export function childPath(parent: string, name: string): string {
    return `${parent === "/" ? "" : parent}/${name}`;
}
export async function createFolder(client: QueryClient, libraryId: string, path: string): Promise<Entry> {
    const data = unwrap(
        await api.POST("/libraries/{libraryId}/folders", {
            params: { path: { libraryId } },
            body: { path, createParents: false },
        }),
    );
    await refreshLibrary(client, libraryId);
    return data.folder;
}
/// `scope` は Library の id か、タイムラインの範囲である。
export async function renameEntry(
    client: QueryClient,
    scope: string,
    entry: { path: string; folder: boolean },
    name: string,
): Promise<Entry> {
    const { libraryId, path } = entryAddress(scope, entry.path);
    const dstPath = childPath(path.slice(0, path.lastIndexOf("/")), name);
    const route = entry.folder ? "/libraries/{libraryId}/folders/move" : "/libraries/{libraryId}/files/move";
    const data = unwrap(
        await api.POST(route, { params: { path: { libraryId } }, body: { srcPaths: [path], dstPath } }),
    );
    if (!("entries" in data)) throw new Error("名前の変更が同期で完了しませんでした。");
    await refreshLibrary(client, libraryId);
    return data.entries[0]!;
}
/// 部分的な成功を捨てず、受理された Operation と元の失敗を呼び出し側へ返す。
export class DeleteEntriesError extends ApiError {
    constructor(
        readonly failures: unknown[],
        readonly operations: Operation[],
        readonly acceptedCount = 0,
    ) {
        const first = failures[0];
        super(
            first instanceof ApiError ? first.status : 0,
            first instanceof ApiError ? first.kind : undefined,
            first instanceof Error ? first.message : "削除できませんでした。",
        );
    }
}
/// `scope` は Library の id か、タイムラインの範囲である。Library ごとに要求を分け、一部の Library だけが失敗しても、ほかの Library の削除は進める。
export async function deleteEntries(
    client: QueryClient,
    scope: string,
    items: ReadonlyMap<string, { folder: boolean }>,
): Promise<Operation[]> {
    const requests = [];
    for (const [libraryId, group] of groupByLibrary(scope, items)) {
        const files = [...group].filter(([, item]) => !item.folder).map(([path]) => path);
        const folders = [...group].filter(([, item]) => item.folder).map(([path]) => path);
        if (files.length)
            requests.push({
                libraryId,
                request: api.POST("/libraries/{libraryId}/files/delete", {
                    params: { path: { libraryId } },
                    body: { srcPaths: files },
                }),
            });
        if (folders.length)
            requests.push({
                libraryId,
                request: api.POST("/libraries/{libraryId}/folders/delete", {
                    params: { path: { libraryId } },
                    body: { srcPaths: folders, recursive: true },
                }),
            });
    }
    const operations: Operation[] = [];
    const failures: unknown[] = [];
    const synchronous = new Set<string>();
    const results = await Promise.allSettled(
        requests.map(async ({ libraryId, request }) => ({ libraryId, response: await request })),
    );
    for (const result of results) {
        if (result.status === "rejected") {
            failures.push(result.reason);
            continue;
        }
        try {
            const data = unwrap(result.value.response);
            if (data && "operation" in data) operations.push(data.operation);
            else synchronous.add(result.value.libraryId);
        } catch (error) {
            failures.push(error);
        }
    }
    await Promise.all([...synchronous].map((libraryId) => refreshLibrary(client, libraryId)));
    if (failures.length) throw new DeleteEntriesError(failures, operations, requests.length - failures.length);
    return operations;
}

export type ConflictPolicy = components["schemas"]["ConflictPolicy"];
export interface TransferResult {
    entries: Entry[];
    operations: Operation[];
    failures: unknown[];
    skippedCount: number;
    acceptedPaths: string[];
}
export type AcceptedTransfer = Omit<TransferResult, "failures">;
async function transferEntries(
    client: QueryClient,
    mode: "move" | "copy",
    srcLibraryId: string,
    srcPaths: string[],
    dstLibraryId: string,
    dstFolderPath: string,
    conflict: ConflictPolicy,
    folder: boolean,
    onAccepted?: (result: AcceptedTransfer) => void,
): Promise<TransferResult> {
    const result: TransferResult = { entries: [], operations: [], failures: [], skippedCount: 0, acceptedPaths: [] };
    if (!srcPaths.length) return result;
    const route = folder
        ? mode === "move"
            ? "/libraries/{libraryId}/folders/move"
            : "/libraries/{libraryId}/folders/copy"
        : mode === "move"
          ? "/libraries/{libraryId}/files/move"
          : "/libraries/{libraryId}/files/copy";
    try {
        const data = unwrap<{ entries: Entry[] } | { operation: Operation }>(
            await api.POST(route, {
                params: { path: { libraryId: srcLibraryId } },
                body: { srcPaths, dstLibraryId, dstFolderPath, conflict },
            }),
        );
        if ("operation" in data) result.operations.push(data.operation);
        else {
            result.entries = data.entries;
            if (conflict === "skip") result.skippedCount = srcPaths.length - data.entries.length;
        }
        result.acceptedPaths = srcPaths;
        onAccepted?.({
            entries: result.entries,
            operations: result.operations,
            skippedCount: result.skippedCount,
            acceptedPaths: srcPaths,
        });
        if ("entries" in data) {
            const refreshed = await Promise.allSettled(
                [...new Set([srcLibraryId, dstLibraryId])].map((id) => refreshLibrary(client, id)),
            );
            for (const item of refreshed) if (item.status === "rejected") result.failures.push(item.reason);
        }
    } catch (error) {
        result.failures.push(error);
    }
    return result;
}
export function moveEntries(
    client: QueryClient,
    srcLibraryId: string,
    srcPaths: string[],
    dstLibraryId: string,
    dstFolderPath: string,
    conflict: ConflictPolicy,
    folder = false,
    onAccepted?: (result: AcceptedTransfer) => void,
): Promise<TransferResult> {
    return transferEntries(
        client,
        "move",
        srcLibraryId,
        srcPaths,
        dstLibraryId,
        dstFolderPath,
        conflict,
        folder,
        onAccepted,
    );
}
export function copyEntries(
    client: QueryClient,
    srcLibraryId: string,
    srcPaths: string[],
    dstLibraryId: string,
    dstFolderPath: string,
    conflict: ConflictPolicy,
    folder = false,
    onAccepted?: (result: AcceptedTransfer) => void,
): Promise<TransferResult> {
    return transferEntries(
        client,
        "copy",
        srcLibraryId,
        srcPaths,
        dstLibraryId,
        dstFolderPath,
        conflict,
        folder,
        onAccepted,
    );
}
export async function transferSelection(
    client: QueryClient,
    source: { mode: "cut" | "copy"; libraryId: string; entries: readonly { path: string; folder: boolean }[] },
    destination: { libraryId: string; path: string },
    conflict: ConflictPolicy,
    onAccepted?: (result: AcceptedTransfer) => void,
): Promise<TransferResult> {
    const send = source.mode === "cut" ? moveEntries : copyEntries;
    const results = await Promise.all(
        [false, true].map((folder) =>
            send(
                client,
                source.libraryId,
                source.entries.filter((entry) => entry.folder === folder).map((entry) => entry.path),
                destination.libraryId,
                destination.path,
                conflict,
                folder,
                onAccepted,
            ),
        ),
    );
    return {
        entries: results.flatMap((result) => result.entries),
        operations: results.flatMap((result) => result.operations),
        failures: results.flatMap((result) => result.failures),
        skippedCount: results.reduce((sum, result) => sum + result.skippedCount, 0),
        acceptedPaths: results.flatMap((result) => result.acceptedPaths),
    };
}
