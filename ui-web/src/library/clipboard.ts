import { createContext, useContext, useSyncExternalStore } from "react";

import { entryAddress, groupByLibrary } from "../location";

export interface ClipboardEntry {
    path: string;
    folder: boolean;
    readonly?: boolean;
}
/// 1 つの Library から送る Entry の組。貼り付けとドラッグは、この組ごとに送る。
export interface TransferSource {
    mode: "cut" | "copy";
    libraryId: string;
    entries: readonly ClipboardEntry[];
}
/// 切り取りかコピーをした Entry。タイムラインでは複数の Library にまたがるので、Library ごとの組に分けて持つ。
export interface ClipboardContents {
    mode: "cut" | "copy";
    sources: readonly TransferSource[];
}
export class EntryClipboard {
    private contents: ClipboardContents | undefined;
    /// 組ごとの版。送っている間に同じ組を書き直していなければ、送れた Entry を組から除く。
    private revisions = new WeakMap<TransferSource, object>();
    private listeners = new Set<() => void>();
    subscribe = (listener: () => void) => {
        this.listeners.add(listener);
        return () => {
            this.listeners.delete(listener);
        };
    };
    getSnapshot = () => this.contents;
    /// `scope` は Library の id か、タイムラインの範囲である。選んだ Entry を Library ごとの組に分けて持つ。
    write(
        mode: ClipboardContents["mode"],
        scope: string,
        items: ReadonlyMap<string, { folder: boolean; readonly?: boolean }>,
    ) {
        if (!items.size || (mode === "cut" && [...items.values()].some((item) => item.readonly))) return;
        const sources = [...groupByLibrary(scope, items)].map(
            ([libraryId, group]): TransferSource => ({
                mode,
                libraryId,
                entries: [...group].map(([path, { folder }]) => ({ path, folder })),
            }),
        );
        for (const source of sources) this.revisions.set(source, {});
        this.contents = { mode, sources };
        this.emit();
    }
    removeAccepted(expected: TransferSource, paths: readonly string[]) {
        const revision = this.revisions.get(expected);
        if (!revision || this.contents?.mode !== "cut") return;
        const index = this.contents.sources.findIndex((source) => this.revisions.get(source) === revision);
        const current = this.contents.sources[index];
        if (current === undefined) return;
        const accepted = new Set(paths);
        const entries = current.entries.filter((entry) => !accepted.has(entry.path));
        const sources = [...this.contents.sources];
        if (entries.length) {
            const remaining = { ...current, entries };
            this.revisions.set(remaining, revision);
            sources[index] = remaining;
        } else sources.splice(index, 1);
        this.contents = sources.length ? { ...this.contents, sources } : undefined;
        this.emit();
    }
    clear(expected = this.contents) {
        if (expected !== this.contents) return;
        this.contents = undefined;
        this.emit();
    }
    private emit() {
        for (const listener of this.listeners) listener();
    }
}
export const ClipboardContext = createContext<EntryClipboard | undefined>(undefined);
const emptyClipboard = new EntryClipboard();
export function useClipboard() {
    const store = useContext(ClipboardContext);
    const contents = useSyncExternalStore((store ?? emptyClipboard).subscribe, (store ?? emptyClipboard).getSnapshot);
    return { store, contents };
}
/// `scope` は Library の id か、タイムラインの範囲である。
export function isCut(contents: ClipboardContents | undefined, scope: string, scopedPath: string) {
    if (contents?.mode !== "cut") return false;
    const { libraryId, path } = entryAddress(scope, scopedPath);
    return contents.sources.some(
        (source) => source.libraryId === libraryId && source.entries.some((entry) => entry.path === path),
    );
}
