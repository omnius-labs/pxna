import { createContext, useContext, useSyncExternalStore } from "react";

export interface ClipboardEntry {
    path: string;
    folder: boolean;
    readonly?: boolean;
}
export interface ClipboardContents {
    mode: "cut" | "copy";
    libraryId: string;
    entries: readonly ClipboardEntry[];
}
export class EntryClipboard {
    private contents: ClipboardContents | undefined;
    private revisions = new WeakMap<ClipboardContents, object>();
    private listeners = new Set<() => void>();
    subscribe = (listener: () => void) => {
        this.listeners.add(listener);
        return () => {
            this.listeners.delete(listener);
        };
    };
    getSnapshot = () => this.contents;
    write(
        mode: ClipboardContents["mode"],
        libraryId: string,
        items: ReadonlyMap<string, { folder: boolean; readonly?: boolean }>,
    ) {
        if (!items.size || (mode === "cut" && [...items.values()].some((item) => item.readonly))) return;
        this.contents = { mode, libraryId, entries: [...items].map(([path, { folder }]) => ({ path, folder })) };
        this.revisions.set(this.contents, {});
        this.emit();
    }
    removeAccepted(expected: ClipboardContents, paths: readonly string[]) {
        const revision = this.revisions.get(expected);
        if (!revision || this.contents?.mode !== "cut" || revision !== this.revisions.get(this.contents)) return;
        const accepted = new Set(paths);
        const entries = this.contents.entries.filter((entry) => !accepted.has(entry.path));
        this.contents = entries.length ? { ...this.contents, entries } : undefined;
        if (this.contents) this.revisions.set(this.contents, revision);
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
export function isCut(contents: ClipboardContents | undefined, libraryId: string, path: string) {
    return (
        contents?.mode === "cut" &&
        contents.libraryId === libraryId &&
        contents.entries.some((entry) => entry.path === path)
    );
}
