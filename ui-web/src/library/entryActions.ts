import type { QueryClient } from "@tanstack/react-query";
import { t } from "i18next";
import { ApiError, api, unwrap } from "../api/client";
import type { components } from "../api/schema";
import { describeError } from "../i18n/errors";
import { entryAddress, groupByLibrary } from "../location";
import type { Entry } from "./entries";
import { refreshLibrary } from "./listing";
import { isRunning } from "./operations";

export type Operation = components["schemas"]["Operation"];
export type EntryRef = components["schemas"]["EntryRef"];

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
        await api.POST("/libraries/{libraryId}/entries/folders", {
            params: { path: { libraryId } },
            body: { path, createParents: false },
        }),
    );
    await refreshLibrary(client, libraryId);
    return data.entry;
}
/// Entry を 1 つ指す値。フォルダは `view=folder` で指し、それ以外は archive のファイルも含めてファイルとして指す。
export function entryRef(libraryId: string, path: string, folder: boolean): EntryRef {
    return { libraryId, path, view: folder ? "folder" : "file" };
}
/// 終わった Operation の最初の失敗を、要求の失敗と同じ形のエラーにする。
export function operationFailure(operation: Operation): ApiError | undefined {
    const failure = operation.failedEntries[0];
    if (!failure) return undefined;
    return new ApiError(0, failure.kind, failure.message ?? undefined);
}
/// `scope` は Library の id か、タイムラインの範囲である。同じ Library の中の 1 件の移動は応答の前に終わり、終わった Operation が返る。
export async function renameEntry(
    client: QueryClient,
    scope: string,
    entry: { path: string; folder: boolean },
    name: string,
): Promise<Entry> {
    const { libraryId, path } = entryAddress(scope, entry.path);
    const dstPath = childPath(path.slice(0, path.lastIndexOf("/")), name);
    const { operation } = unwrap(
        await api.POST("/entries/move", {
            body: { sources: [entryRef(libraryId, path, entry.folder)], destination: { libraryId, path: dstPath } },
        }),
    );
    const failure = operationFailure(operation);
    if (failure) throw failure;
    const renamed = isRunning(operation) ? undefined : operation.resultEntries?.[0];
    if (!renamed) throw new Error(t("errors.renameNotSync"));
    await refreshLibrary(client, libraryId);
    return renamed;
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
            describeError(first, t("errors.deleteFailed")),
        );
    }
}
/// `scope` は Library の id か、タイムラインの範囲である。Library ごとに要求を分け、一部の Library だけが失敗しても、ほかの Library の削除は進める。
/// 応答の前に終わった削除は、その場で一覧を読み直す。失敗を含むときは、その Operation も返して失敗を見せる。
export async function deleteEntries(
    client: QueryClient,
    scope: string,
    items: ReadonlyMap<string, { folder: boolean }>,
): Promise<Operation[]> {
    const requests = [...groupByLibrary(scope, items)].map(([libraryId, group]) => ({
        libraryId,
        request: api.POST("/entries/delete", {
            body: {
                targets: [...group].map(([path, item]) => entryRef(libraryId, path, item.folder)),
                recursive: true,
            },
        }),
    }));
    const operations: Operation[] = [];
    const failures: unknown[] = [];
    const finished = new Set<string>();
    const results = await Promise.allSettled(
        requests.map(async ({ libraryId, request }) => ({ libraryId, response: await request })),
    );
    for (const result of results) {
        if (result.status === "rejected") {
            failures.push(result.reason);
            continue;
        }
        try {
            const { operation } = unwrap(result.value.response);
            if (isRunning(operation)) {
                operations.push(operation);
                continue;
            }
            finished.add(result.value.libraryId);
            const failure = operationFailure(operation);
            if (failure) {
                failures.push(failure);
                operations.push(operation);
            }
        } catch (error) {
            failures.push(error);
        }
    }
    await Promise.all([...finished].map((libraryId) => refreshLibrary(client, libraryId)));
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
/// 選んだファイルとフォルダを 1 つの要求で送る。応答の前に終わった Operation は、その結果を返して一覧を読み直し、実行中の Operation は呼び出し側に見せてもらう。
export async function transferSelection(
    client: QueryClient,
    source: { mode: "cut" | "copy"; libraryId: string; entries: readonly { path: string; folder: boolean }[] },
    destination: { libraryId: string; path: string },
    conflict: ConflictPolicy,
    onAccepted?: (result: AcceptedTransfer) => void,
): Promise<TransferResult> {
    const result: TransferResult = { entries: [], operations: [], failures: [], skippedCount: 0, acceptedPaths: [] };
    if (!source.entries.length) return result;
    const srcPaths = source.entries.map((entry) => entry.path);
    try {
        const { operation } = unwrap(
            await api.POST(source.mode === "cut" ? "/entries/move" : "/entries/copy", {
                body: {
                    sources: source.entries.map((entry) => entryRef(source.libraryId, entry.path, entry.folder)),
                    destination: { libraryId: destination.libraryId, folderPath: destination.path },
                    conflict,
                },
            }),
        );
        const running = isRunning(operation);
        if (running) result.operations.push(operation);
        else {
            result.entries = operation.resultEntries ?? [];
            result.skippedCount = operation.skippedCount;
            const failure = operationFailure(operation);
            if (failure) result.failures.push(failure);
        }
        result.acceptedPaths = srcPaths;
        onAccepted?.({
            entries: result.entries,
            operations: result.operations,
            skippedCount: result.skippedCount,
            acceptedPaths: srcPaths,
        });
        if (!running) {
            const refreshed = await Promise.allSettled(
                [...new Set([source.libraryId, destination.libraryId])].map((id) => refreshLibrary(client, id)),
            );
            for (const item of refreshed) if (item.status === "rejected") result.failures.push(item.reason);
        }
    } catch (error) {
        result.failures.push(error);
    }
    return result;
}
