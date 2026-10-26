import { type KeyboardEvent, type MouseEvent, useEffect, useRef, useState } from "react";
import { toast } from "sonner";

import type { Entry } from "./entries";
import { type Listing, ListingSnapshotChanged, PAGE_SIZE } from "./listing";
import { clickSelection, extendSelection, type Selection, selectAll, selectionItem, selectionValue } from "./selection";

export interface SelectionProgress {
    loaded: number;
    total: number;
}
interface InteractionOptions {
    listing: Listing;
    libraryReadonly?: boolean;
    columns: number;
    selection: Selection;
    onSelectionChange: (selection: Selection) => void;
    onProgress: (progress: SelectionProgress | undefined) => void;
    onFocus: (index: number) => void;
    onOpen: (entry: Entry) => void;
    /// 修飾キーのないクリックで開くか。狭い画面は 1 回のタップで開き、ダブルクリックでは開かない。既定は開かない。
    openOnClick?: boolean;
    /// Ctrl か ⌘ を押しながらの A で、全件を選ぶか。読み込み済みの範囲だけを持つ一覧は選ばない。既定は選ぶ。
    selectAll?: boolean;
    /// 矢印キーで動ける位置の範囲。省略すると、一覧の先頭から末尾までである。
    bounds?: { first: number; last: number };
    /// 上下の矢印キーで移る先の位置。省略すると、列の数だけ動く。動けなければ `undefined` を返す。
    verticalTarget?: (index: number, direction: -1 | 1) => number | undefined;
}

/// 範囲を読み終えるまでは選択を変えず、途中の別操作と版の変更で取り消す。
export function useEntryInteraction({
    listing,
    libraryReadonly = false,
    columns,
    selection,
    onSelectionChange,
    onProgress,
    onFocus,
    onOpen,
    openOnClick = false,
    selectAll: allowSelectAll = true,
    bounds,
    verticalTarget,
}: InteractionOptions) {
    const [focusedIndex, setFocusedIndex] = useState(0);
    const [operation, setOperation] = useState<{ pages: number[]; controller: AbortController }>();
    const current = useRef<AbortController | undefined>(undefined);
    const previousSnapshot = useRef(listing.snapshot);
    const { snapshot, version } = listing;
    const progressLoaded = operation?.pages.filter((page) => listing.pageStatus(page) === "loaded").length;
    const progressTotal = operation?.pages.length;
    useEffect(() => {
        onProgress(progressTotal === undefined ? undefined : { loaded: progressLoaded ?? 0, total: progressTotal });
    }, [progressLoaded, progressTotal, onProgress]);
    useEffect(
        () => () => {
            current.current?.abort();
            current.current = undefined;
            onProgress(undefined);
        },
        [onProgress],
    );
    useEffect(() => {
        if (previousSnapshot.current !== undefined && previousSnapshot.current !== snapshot) {
            current.current?.abort();
            current.current = undefined;
            setOperation(undefined);
            onSelectionChange({ ...selection, anchor: undefined });
        }
        previousSnapshot.current = snapshot;
    }, [snapshot, selection, onSelectionChange]);

    const [detailOwner] = useState(() => ({}));
    useEffect(() => {
        listing.setDetailPriority?.(detailOwner, [...selection.items.keys()]);
    }, [listing, detailOwner, selection.items]);
    useEffect(() => () => listing.releaseDetailPriority?.(detailOwner), [listing, detailOwner]);
    // biome-ignore lint/correctness/useExhaustiveDependencies: version observes the mutable listing store
    useEffect(() => {
        const attributes = (entry: Entry) =>
            JSON.stringify([
                entry.name,
                entry.kind,
                !!entry.file?.archive,
                entry.metadata?.size,
                entry.metadata?.readonly,
                entry.metadata?.createdAt,
                entry.metadata?.modifiedAt,
                entry.metadata?.accessedAt,
                entry.metadata?.contentType,
                entry.folder?.hasChildren,
                entry.link,
                entry.detailError,
            ]);
        let items: Map<string, import("./selection").SelectionValue> | undefined;
        for (const [path, value] of selection.items) {
            const index = listing.locate?.(path);
            const entry = index === undefined ? undefined : listing.entryAt(index);
            if (entry !== undefined && entry !== value.entry && attributes(entry) !== attributes(value.entry)) {
                items ??= new Map(selection.items);
                items.set(path, selectionValue(selectionItem(entry, libraryReadonly)));
            }
        }
        if (items) onSelectionChange({ ...selection, items });
    }, [listing, version, selection, libraryReadonly, onSelectionChange]);

    useEffect(() => {
        const drag = () => {
            current.current?.abort();
            current.current = undefined;
            setOperation(undefined);
        };
        window.addEventListener("dragstart", drag);
        return () => window.removeEventListener("dragstart", drag);
    }, []);
    const cancel = () => {
        current.current?.abort();
        current.current = undefined;
        setOperation(undefined);
    };
    const itemAt = (index: number) => {
        const entry = listing.entryAt(index);
        return entry ? selectionItem(entry, libraryReadonly) : undefined;
    };
    const focus = (index: number) => {
        setFocusedIndex(index);
        onFocus(index);
    };
    const [pendingFocus, setPendingFocus] = useState<number>();
    // biome-ignore lint/correctness/useExhaustiveDependencies: version は listing の遅延ロード後にフォーカスを再試行させる gate で、本体からは参照しない意図的な依存
    useEffect(() => {
        if (pendingFocus !== undefined && listing.entryAt(pendingFocus)) {
            const frame = requestAnimationFrame(() => {
                onFocus(pendingFocus);
                setPendingFocus(undefined);
            });
            return () => cancelAnimationFrame(frame);
        }
    }, [pendingFocus, version, listing, onFocus]);

    const readRange = async (from: number, to: number, apply: () => Selection) => {
        cancel();
        const pages = [];
        for (
            let page = Math.floor(Math.min(from, to) / PAGE_SIZE);
            page <= Math.floor(Math.max(from, to) / PAGE_SIZE);
            page += 1
        )
            pages.push(page);
        if (pages.every((page) => listing.pageStatus(page) === "loaded")) {
            onSelectionChange(apply());
            return;
        }
        const controller = new AbortController();
        current.current = controller;
        setOperation({ pages, controller });
        try {
            await listing.loadPages(pages, controller.signal);
            if (!controller.signal.aborted && current.current === controller) onSelectionChange(apply());
        } catch (error) {
            if (current.current === controller && !controller.signal.aborted) {
                if (error instanceof ListingSnapshotChanged) onSelectionChange({ ...selection, anchor: undefined });
                else toast.error("選択する範囲を読めませんでした。");
            }
        } finally {
            if (current.current === controller) {
                current.current = undefined;
                setOperation(undefined);
            }
        }
    };
    const click = (event: MouseEvent, index: number) => {
        setFocusedIndex(index);
        setPendingFocus(undefined);
        const modifiers = { toggle: event.metaKey || event.ctrlKey, range: event.shiftKey };
        const entry = listing.entryAt(index);
        if (openOnClick && !modifiers.toggle && !modifiers.range && entry) {
            cancel();
            onOpen(entry);
        } else if (modifiers.range)
            void readRange(selection.anchor ?? index, index, () => clickSelection(selection, index, modifiers, itemAt));
        else {
            cancel();
            onSelectionChange(clickSelection(selection, index, modifiers, itemAt));
        }
    };
    const keyDown = (event: KeyboardEvent, index: number) => {
        const count = listing.totalCount ?? 0;
        if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "a") {
            event.preventDefault();
            if (allowSelectAll && count > 0) void readRange(0, count - 1, () => selectAll(count, itemAt));
            return;
        }
        const vertical = (direction: -1 | 1) =>
            verticalTarget ? (verticalTarget(index, direction) ?? index) : index + direction * columns;
        const moves: Record<string, number> = {
            ArrowLeft: index - 1,
            ArrowRight: index + 1,
            ArrowUp: vertical(-1),
            ArrowDown: vertical(1),
        };
        const moved = moves[event.key];
        if (moved !== undefined && count > 0) {
            event.preventDefault();
            const target = Math.min(Math.max(moved, bounds?.first ?? 0), bounds?.last ?? count - 1);
            focus(target);
            setPendingFocus(target);
            if (event.shiftKey)
                void readRange(selection.anchor ?? index, target, () =>
                    extendSelection(selection, index, target, itemAt),
                );
            else cancel();
        } else if (event.key === "Enter") {
            event.preventDefault();
            cancel();
            const entry = listing.entryAt(index);
            if (entry) onOpen(entry);
        } else if (event.key === " ") {
            event.preventDefault();
            cancel();
            onSelectionChange(clickSelection(selection, index, { toggle: true, range: false }, itemAt));
        }
    };
    /// ダブルクリックで開く。クリックで開く一覧では、2 度目のクリックと重ねて開かないよう、何もしない。
    const open = (entry: Entry) => {
        if (openOnClick) return;
        cancel();
        onOpen(entry);
    };
    return { focusedIndex, click, keyDown, open };
}
