import { type DragEvent, useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import type { TransferSource } from "./clipboard";
import type { TransferDestination } from "./collision";
import { useTransfer } from "./transfer";
import { droppedInputs } from "./upload";

export const ENTRY_MIME = "application/x-pxna-entries";
let dragged: TransferSource | undefined;
export function beginEntryDrag(data: DataTransfer, source: TransferSource) {
    dragged = source;
    data.effectAllowed = "copyMove";
    data.setData(ENTRY_MIME, JSON.stringify(source));
}
export function endEntryDrag() {
    dragged = undefined;
}
export function readEntryDrag(data: DataTransfer): TransferSource | undefined {
    try {
        const value: unknown = JSON.parse(data.getData(ENTRY_MIME));
        if (
            !value ||
            typeof value !== "object" ||
            !("libraryId" in value) ||
            typeof value.libraryId !== "string" ||
            !("entries" in value) ||
            !Array.isArray(value.entries)
        )
            return;
        if (
            !value.entries.length ||
            !value.entries.every((item) => item && typeof item.path === "string" && typeof item.folder === "boolean")
        )
            return;
        return {
            mode: "cut",
            libraryId: value.libraryId,
            entries: value.entries.map((item) => ({
                path: item.path,
                folder: item.folder,
                ...(item.readonly === true ? { readonly: true } : {}),
            })),
        };
    } catch {
        return;
    }
}
export function canDropEntry(
    destination: TransferDestination | undefined,
    readonly: boolean,
    source = dragged,
): boolean {
    return !!destination && !readonly && (!source || source.libraryId === destination.libraryId);
}
export function createDragScroll(element: HTMLElement) {
    let timer: ReturnType<typeof setInterval> | undefined;
    let speed = 0;
    const stop = () => {
        if (timer !== undefined) clearInterval(timer);
        timer = undefined;
        speed = 0;
    };
    return {
        move(y: number) {
            const rect = element.getBoundingClientRect();
            speed = y < rect.top + 40 ? -16 : y > rect.bottom - 40 ? 16 : 0;
            if (!speed) {
                stop();
                return;
            }
            timer ??= setInterval(() => {
                element.scrollTop += speed;
            }, 33);
        },
        stop,
    };
}
export function useDragScroll(element: HTMLElement | null) {
    useEffect(() => {
        if (!element) return;
        const scroll = createDragScroll(element);
        const move = (event: globalThis.DragEvent) => scroll.move(event.clientY);
        const leave = (event: globalThis.DragEvent) => {
            if (!element.contains(event.relatedTarget as Node | null)) scroll.stop();
        };
        const key = (event: globalThis.KeyboardEvent) => {
            if (event.key === "Escape") scroll.stop();
        };
        element.addEventListener("dragover", move, true);
        element.addEventListener("dragleave", leave);
        window.addEventListener("dragend", scroll.stop);
        window.addEventListener("drop", scroll.stop);
        window.addEventListener("keydown", key);
        return () => {
            scroll.stop();
            element.removeEventListener("dragover", move, true);
            element.removeEventListener("dragleave", leave);
            window.removeEventListener("dragend", scroll.stop);
            window.removeEventListener("drop", scroll.stop);
            window.removeEventListener("keydown", key);
        };
    }, [element]);
}
export function useEntryDrop(
    destination: TransferDestination | undefined,
    readonly: boolean,
    expand?: () => void,
    uploadDestination?: TransferDestination,
) {
    const transfer = useTransfer();
    const [hover, setHover] = useState<"allowed" | "blocked">();
    const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
    const reset = useCallback(() => {
        if (timer.current !== undefined) clearTimeout(timer.current);
        timer.current = undefined;
        setHover(undefined);
    }, []);
    useEffect(() => {
        const key = (event: globalThis.KeyboardEvent) => {
            if (event.key === "Escape") {
                endEntryDrag();
                reset();
            }
        };
        const end = () => {
            endEntryDrag();
            reset();
        };
        window.addEventListener("dragend", end);
        window.addEventListener("drop", reset);
        window.addEventListener("keydown", key);
        return () => {
            if (timer.current !== undefined) clearTimeout(timer.current);
            window.removeEventListener("dragend", end);
            window.removeEventListener("drop", reset);
            window.removeEventListener("keydown", key);
        };
    }, [reset]);
    const accepts = (event: DragEvent) => event.dataTransfer.types.includes(ENTRY_MIME);
    const files = (event: DragEvent) => event.dataTransfer.types.includes("Files");
    return {
        "data-drop-state": hover,
        onDragOver(event: DragEvent) {
            if (!accepts(event) && !files(event)) return;
            if (files(event) && !uploadDestination) return;
            event.preventDefault();
            event.stopPropagation();
            const allowed = files(event)
                ? !!transfer?.upload && !!uploadDestination && !readonly && !transfer.busy
                : !!transfer &&
                  !transfer.busy &&
                  canDropEntry(destination, readonly) &&
                  (event.altKey || !dragged?.entries.some((entry) => entry.readonly));
            event.dataTransfer.dropEffect = allowed ? (files(event) || event.altKey ? "copy" : "move") : "none";
            setHover(allowed ? "allowed" : "blocked");
            if (expand && timer.current === undefined) timer.current = setTimeout(expand, 1000);
        },
        onDragLeave(event: DragEvent) {
            if (!event.currentTarget.contains(event.relatedTarget as Node | null)) reset();
        },
        onDrop(event: DragEvent) {
            if (files(event)) {
                if (!uploadDestination) return;
                event.preventDefault();
                event.stopPropagation();
                reset();
                if (!readonly && transfer?.upload) {
                    void droppedInputs(event.dataTransfer)
                        .then((inputs) => transfer.upload?.(inputs, uploadDestination))
                        .catch((error) =>
                            toast.error("持ち込むファイルを読めませんでした。", {
                                description: error instanceof Error ? error.message : undefined,
                            }),
                        );
                }
                return;
            }
            if (!accepts(event)) return;
            event.preventDefault();
            event.stopPropagation();
            reset();
            const source = readEntryDrag(event.dataTransfer);
            endEntryDrag();
            if (
                source &&
                destination &&
                canDropEntry(destination, readonly, source) &&
                (event.altKey || !source.entries.some((entry) => entry.readonly))
            )
                void transfer?.transfer({ ...source, mode: event.altKey ? "copy" : "cut" }, destination);
        },
    };
}
