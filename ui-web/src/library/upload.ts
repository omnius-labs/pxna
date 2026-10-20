import { createContext, useContext, useSyncExternalStore } from "react";
import { ApiError, api, unwrap } from "../api/client";
import type { ConflictPolicy, TransferDestination } from "./collision";
import { fetchFolderPage } from "./entries";
import { childPath, validateName } from "./entryActions";

export const ONE_SHOT_LIMIT = 32 * 1024 * 1024;
export const UPLOAD_CHUNK_SIZE = 16 * 1024 * 1024;
export interface UploadInput {
    path: string;
    file?: File;
}
export interface UploadItem {
    id: string;
    libraryId: string;
    path: string;
    name: string;
    size: number;
    sent: number;
    chunked: boolean;
    status: "pending" | "running" | "succeeded" | "failed" | "cancelled" | "skipped";
    error?: string;
}
export function isUploadRunning(item: UploadItem) {
    return item.status === "pending" || item.status === "running";
}
async function writeBytes(url: string, body: Blob, signal: AbortSignal): Promise<Response> {
    const response = await fetch(url, {
        method: "PUT",
        body,
        signal,
        headers: { "Content-Type": "application/octet-stream" },
    });
    if (!response.ok) {
        const error = (await response.json().catch(() => undefined)) as
            | { kind?: ApiError["kind"]; message?: string }
            | undefined;
        throw new ApiError(response.status, error?.kind, error?.message);
    }
    return response;
}
export async function uploadFile(
    file: File,
    destination: TransferDestination,
    conflict: ConflictPolicy,
    signal: AbortSignal,
    progress: (sent: number) => void,
): Promise<void> {
    signal.throwIfAborted();
    const base = `/api/v1/libraries/${encodeURIComponent(destination.libraryId)}`;
    if (file.size <= ONE_SHOT_LIMIT) {
        await writeBytes(
            `${base}/files/content?${new URLSearchParams({ path: destination.path, conflict })}`,
            file,
            signal,
        );
        progress(file.size);
        return;
    }
    // 作成は中断せず識別子を受け取る。取り消しと作成が重なっても、そのセッションを破棄できる。
    const upload = unwrap(
        await api.POST("/libraries/{libraryId}/uploads", {
            params: { path: { libraryId: destination.libraryId } },
            body: { path: destination.path, size: file.size, conflict },
        }),
    ).upload;
    const params = { path: { libraryId: destination.libraryId, uploadId: upload.id } };
    let committed = false;
    try {
        signal.throwIfAborted();
        let offset = upload.committedSize;
        if (offset !== 0) throw new Error("新しいアップロードの開始位置が不正です。");
        while (offset < file.size) {
            const chunk = file.slice(offset, Math.min(offset + UPLOAD_CHUNK_SIZE, file.size));
            await writeBytes(
                `${base}/uploads/${encodeURIComponent(upload.id)}/content?offset=${offset}`,
                chunk,
                signal,
            );
            const state = unwrap(await api.GET("/libraries/{libraryId}/uploads/{uploadId}", { params, signal })).upload;
            if (state.status !== "active" || state.committedSize !== offset + chunk.size)
                throw new Error("アップロードの送信位置を確認できませんでした。");
            offset = state.committedSize;
            progress(offset);
        }
        signal.throwIfAborted();
        // commit の応答は待ち、公開と取り消しが重なっても公開済みの結果を失わない。
        unwrap(await api.POST("/libraries/{libraryId}/uploads/{uploadId}/commit", { params }));
        committed = true;
    } finally {
        if (!committed) unwrap(await api.DELETE("/libraries/{libraryId}/uploads/{uploadId}", { params }));
    }
}
async function existingEntry(libraryId: string, parent: string, name: string) {
    let offset = 0;
    while (true) {
        const page = await fetchFolderPage(libraryId, parent, offset, { query: name, recursive: false });
        const found = page.entries.find((entry) => entry.name === name);
        if (found) return found;
        offset += page.entries.length;
        if (offset >= page.totalCount) return;
        if (!page.entries.length) throw new Error("宛先の種類を確認できませんでした。");
    }
}
async function makeUploadFolder(
    libraryId: string,
    parent: string,
    name: string,
    policy: ConflictPolicy,
    signal: AbortSignal,
): Promise<string | undefined> {
    for (let suffix = 0; suffix < 1000; suffix++) {
        signal.throwIfAborted();
        const path = childPath(parent, suffix ? `${name} (${suffix})` : name);
        try {
            unwrap(
                await api.POST("/libraries/{libraryId}/folders", {
                    params: { path: { libraryId } },
                    body: { path, createParents: false },
                }),
            );
            return path;
        } catch (error) {
            if (!(error instanceof ApiError) || error.status !== 409 || policy === "fail") throw error;
            if (policy === "skip") return;
            if (policy === "rename") continue;
            const existing = await existingEntry(libraryId, parent, name);
            if (!existing) throw error;
            if (existing.metadata?.readonly !== false)
                throw new Error("このフォルダへは送れません。", { cause: error });
            if (existing.kind === "folder") return path;
            signal.throwIfAborted();
            unwrap(
                await api.POST("/libraries/{libraryId}/files/delete", {
                    params: { path: { libraryId } },
                    body: { srcPaths: [path] },
                }),
            );
            signal.throwIfAborted();
            unwrap(
                await api.POST("/libraries/{libraryId}/folders", {
                    params: { path: { libraryId } },
                    body: { path, createParents: false },
                }),
            );
            return path;
        }
    }
    throw new Error("空いているフォルダ名を見つけられませんでした。");
}
interface UploadJob {
    item: UploadItem;
    input: UploadInput;
    prepare(signal: AbortSignal): Promise<string | undefined>;
    policy: ConflictPolicy;
    controller: AbortController;
}
export class UploadStore {
    private items: UploadItem[] = [];
    private jobs = new Map<string, UploadJob>();
    private listeners = new Set<() => void>();
    private running = 0;
    private sequence = 0;
    constructor(private completed: (libraryId: string) => Promise<void> = async () => {}) {}
    subscribe = (listener: () => void) => {
        this.listeners.add(listener);
        return () => {
            this.listeners.delete(listener);
        };
    };
    getSnapshot = () => this.items;
    private update(id: string, changes: Partial<UploadItem>) {
        this.items = this.items.map((item) => (item.id === id ? { ...item, ...changes } : item));
        for (const listener of this.listeners) listener();
    }
    enqueue(inputs: readonly UploadInput[], destination: TransferDestination, policy: ConflictPolicy) {
        const folders = new Map<string, Promise<string | undefined>>();
        const prepareFolder = (relative: string, signal: AbortSignal): Promise<string | undefined> => {
            if (!relative) return Promise.resolve(destination.path);
            const cached = folders.get(relative);
            if (cached) return cached;
            const split = relative.lastIndexOf("/");
            const parent = split < 0 ? "" : relative.slice(0, split);
            const name = relative.slice(split + 1);
            // 作成は同じバッチで共有する。他のファイルの取り消しで共有の作成を中断しない。
            const created = prepareFolder(parent, signal).then((path) =>
                path === undefined
                    ? undefined
                    : makeUploadFolder(destination.libraryId, path, name, policy, new AbortController().signal),
            );
            folders.set(relative, created);
            return created;
        };
        for (const input of inputs) {
            if (!input.path || input.path.split("/").some((part) => validateName(part)))
                throw new Error("持ち込む名前が不正です。");
        }
        for (const input of inputs) {
            const id = `web-upload-${++this.sequence}`;
            const item: UploadItem = {
                id,
                libraryId: destination.libraryId,
                path: childPath(destination.path, input.path),
                name: input.path,
                size: input.file?.size ?? 0,
                sent: 0,
                chunked: (input.file?.size ?? 0) > ONE_SHOT_LIMIT,
                status: "pending",
            };
            const parent = input.file ? input.path.slice(0, Math.max(0, input.path.lastIndexOf("/"))) : input.path;
            const prepare = async (signal: AbortSignal) => {
                const folder = await prepareFolder(parent, signal);
                signal.throwIfAborted();
                if (folder === undefined) return;
                const name = input.path.split("/").at(-1)!;
                if (input.file && policy === "skip" && (await existingEntry(destination.libraryId, folder, name)))
                    return;
                return input.file ? childPath(folder, name) : folder;
            };
            this.jobs.set(id, { item, input, prepare, policy, controller: new AbortController() });
            this.items = [...this.items, item];
        }
        for (const listener of this.listeners) listener();
        this.pump();
    }
    cancel = (id: string) => {
        const job = this.jobs.get(id);
        const item = this.items.find((item) => item.id === id);
        if (!job || !item || !isUploadRunning(item)) return;
        if (item.status === "pending") {
            this.jobs.delete(id);
            this.update(id, { status: "cancelled" });
        } else job.controller.abort();
    };
    cancelAll = () => {
        for (const id of this.jobs.keys()) this.cancel(id);
    };
    private pump() {
        for (const job of this.jobs.values()) {
            if (this.running >= 2) break;
            if (this.items.find((item) => item.id === job.item.id)?.status !== "pending") continue;
            this.running++;
            this.update(job.item.id, { status: "running" });
            void this.run(job);
        }
    }
    private async run(job: UploadJob) {
        const { signal } = job.controller;
        let published = false;
        try {
            const path = await job.prepare(signal);
            signal.throwIfAborted();
            if (path === undefined) {
                this.update(job.item.id, { status: "skipped" });
                return;
            }
            this.update(job.item.id, { path });
            if (job.input.file)
                await uploadFile(job.input.file, { libraryId: job.item.libraryId, path }, job.policy, signal, (sent) =>
                    this.update(job.item.id, { sent }),
                );
            published = true;
            await this.completed(job.item.libraryId);
            this.update(job.item.id, { status: "succeeded" });
        } catch (error) {
            this.update(job.item.id, {
                status:
                    signal.aborted &&
                    !published &&
                    (error === signal.reason || (error instanceof Error && error.name === "AbortError"))
                        ? "cancelled"
                        : "failed",
                error: error instanceof Error ? error.message : String(error),
            });
        } finally {
            this.jobs.delete(job.item.id);
            this.running--;
            this.pump();
        }
    }
}
export const UploadContext = createContext<UploadStore | undefined>(undefined);
const emptyStore = new UploadStore();
export function useUploads() {
    const store = useContext(UploadContext);
    const items = useSyncExternalStore((store ?? emptyStore).subscribe, (store ?? emptyStore).getSnapshot);
    return { store, items };
}
export function filesToInputs(files: readonly File[]): UploadInput[] {
    return files.map((file) => ({ file, path: file.webkitRelativePath || file.name }));
}
export async function droppedInputs(data: DataTransfer): Promise<UploadInput[]> {
    const files = Array.from(data.files);
    const entries = Array.from(data.items)
        .map((item) => item.webkitGetAsEntry?.())
        .filter((entry): entry is FileSystemEntry => !!entry);
    if (!entries.length) return filesToInputs(files);
    const result: UploadInput[] = [];
    const visit = async (entry: FileSystemEntry, parent: string) => {
        const path = parent ? `${parent}/${entry.name}` : entry.name;
        if (entry.isFile) {
            const file = await new Promise<File>((resolve, reject) =>
                (entry as FileSystemFileEntry).file(resolve, reject),
            );
            result.push({ path, file });
        } else {
            result.push({ path });
            const reader = (entry as FileSystemDirectoryEntry).createReader();
            while (true) {
                const children = await new Promise<FileSystemEntry[]>((resolve, reject) =>
                    reader.readEntries(resolve, reject),
                );
                if (!children.length) break;
                for (const child of children) await visit(child, path);
            }
        }
    };
    for (const entry of entries) await visit(entry, "");
    return result;
}
export function watchUploadsBeforeUnload(store: UploadStore, target: Window = window) {
    let attached = false;
    const beforeUnload = (event: BeforeUnloadEvent) => {
        event.preventDefault();
        event.returnValue = "";
    };
    const update = () => {
        const running = store.getSnapshot().some(isUploadRunning);
        if (running === attached) return;
        attached = running;
        if (running) target.addEventListener("beforeunload", beforeUnload);
        else target.removeEventListener("beforeunload", beforeUnload);
    };
    const unsubscribe = store.subscribe(update);
    update();
    return () => {
        unsubscribe();
        if (attached) target.removeEventListener("beforeunload", beforeUnload);
    };
}
