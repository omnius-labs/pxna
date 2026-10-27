import { useVirtualizer } from "@tanstack/react-virtual";
import { t } from "i18next";
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, useSyncExternalStore } from "react";
import { useImageWants } from "../images/wants";
import { type Entry, isThumbnailable } from "../library/entries";
import { gridLayout, TILE_GAP } from "../library/grid";
import type { DisplaySet } from "../library/preferences";
import type { Selection } from "../library/selection";
import type { TimelineStore } from "../library/timeline";
import {
    anchorAt,
    buildRows,
    firstItemOfRow,
    formatDay,
    monthOf,
    restoreScroll,
    rowOfItem,
    rowStarts,
    type ScrollAnchor,
    type TilesRow,
    verticalTarget,
} from "../library/timelineLayout";
import { type SelectionProgress, useEntryInteraction } from "../library/useEntryInteraction";
import { tilePriority } from "../thumbnails/range";
import { EntryMenu } from "./EntryMenu";
import { LoadFailure } from "./LoadFailure";
import { Tile } from "./Tile";

interface TimelineGridProps {
    store: TimelineStore;
    libraryId: string;
    libraryReadonly?: boolean;
    /// タイルサイズの段階（1 から 5）。
    tileSize: number;
    shape: DisplaySet["thumbnailShape"];
    /// 狭い画面の区分か。段階を列の数として扱う。
    narrow: boolean;
    selection: Selection;
    onSelectionChange: (selection: Selection) => void;
    onProgress: (progress: SelectionProgress | undefined) => void;
    /// 見えている先頭の Entry の月（`YYYY-MM`）。
    onVisibleMonth: (month: string | undefined) => void;
    onOpen: (entry: Entry) => void;
    /// 範囲に 1 件もないときに出す文。
    emptyMessage: string;
}

/// 見えている範囲の前後に、サムネイルを要求する行の数。
const THUMBNAIL_MARGIN_ROWS = 4;

/// 見えている位置の前後に残す Entry の数。これより遠い Entry は捨てる。
const KEEP_ITEMS = 1000;

/// 端から何行以内に近づいたら、続きを読むか。
const LOAD_MARGIN_ROWS = 6;

/// 日ごとの見出しと、新しい順のタイルを並べるグリッド。全体の高さは確保せず、読み込み済みの範囲だけを並べ、端に近づいたら続きを読む。
export function TimelineGrid({
    store,
    libraryId,
    libraryReadonly = false,
    tileSize,
    shape,
    narrow,
    selection,
    onSelectionChange,
    onProgress,
    onVisibleMonth,
    onOpen,
    emptyMessage,
}: TimelineGridProps) {
    useSyncExternalStore(store.subscribe, store.getRevision);
    const [scrollElement, setScrollElement] = useState<HTMLDivElement | null>(null);
    const [width, setWidth] = useState(0);
    useEffect(() => {
        if (!scrollElement) return;
        const observer = new ResizeObserver(([observed]) => setWidth(observed?.contentRect.width ?? 0));
        observer.observe(scrollElement);
        return () => observer.disconnect();
    }, [scrollElement]);

    const { columns, tileSide } = gridLayout({ width, tileSize, narrow });
    const items = store.loadedItems;
    const rows = useMemo(
        () =>
            buildRows(
                items.map((item) => ({ path: item.entry.path, date: item.date })),
                columns,
                tileSide,
            ),
        [items, columns, tileSide],
    );
    const { starts, total } = useMemo(() => rowStarts(rows), [rows]);
    const virtualizer = useVirtualizer({
        count: rows.length,
        getScrollElement: () => scrollElement,
        estimateSize: (index) => rows[index]?.height ?? 0,
        getItemKey: (index) => rows[index]?.key ?? index,
        overscan: 3,
    });
    const virtualRows = virtualizer.getVirtualItems();
    const range = virtualizer.range;
    const firstRow = range?.startIndex;
    const lastRow = range?.endIndex;

    // 行の高さは先に分かっているので、スクロールの位置を、行の上端からの距離で覚える。
    // 前へ読み足して行が増えても、読み直しで範囲が差し替わっても、見ている行を同じ位置に保つ。
    const anchor = useRef<ScrollAnchor | undefined>(undefined);
    const datedItems = useMemo(() => items.map((item) => ({ path: item.entry.path, date: item.date })), [items]);
    const remember = useCallback(() => {
        if (!scrollElement) return;
        anchor.current = anchorAt(rows, starts, datedItems, scrollElement.scrollTop);
    }, [scrollElement, rows, starts, datedItems]);
    useEffect(() => {
        if (!scrollElement) return;
        scrollElement.addEventListener("scroll", remember, { passive: true });
        return () => scrollElement.removeEventListener("scroll", remember);
    }, [scrollElement, remember]);
    // biome-ignore lint/correctness/useExhaustiveDependencies: starts は rows から決まる値で、行が変わったときだけ位置を直す
    useLayoutEffect(() => {
        const saved = anchor.current;
        if (!scrollElement) return;
        // 最初に行を並べた時点の位置を覚える。スクロールの前でも、前へ読み足して行が増えたときに、見ている行を保てる。
        if (!saved) {
            remember();
            return;
        }
        const wanted = restoreScroll(rows, starts, datedItems, saved);
        if (wanted !== undefined && Math.abs(scrollElement.scrollTop - wanted) > 0.5) scrollElement.scrollTop = wanted;
    }, [rows, scrollElement, remember]);

    // 見えている先頭の行の月と Entry を、月の強調と、読み直しの目印にする。
    const topRow = firstRow === undefined ? undefined : rows[Math.min(firstRow, rows.length - 1)];
    const topItem =
        topRow === undefined
            ? undefined
            : items[topRow.type === "tiles" ? topRow.first : firstTileOfDay(rows, topRow.day)];
    const topMonth = topItem === undefined ? undefined : monthOf(topItem.date);
    const topPath = topItem?.entry.path;
    useEffect(() => {
        onVisibleMonth(topMonth);
    }, [topMonth, onVisibleMonth]);
    useEffect(() => {
        store.setFocus(topPath);
    }, [store, topPath]);

    // 端に近づいたら続きを読む。
    const { version, hasPrevious, hasNext } = store;
    // biome-ignore lint/correctness/useExhaustiveDependencies: version は読み込みが終わるたびに、まだ端に近いかを確かめ直させる gate である
    useEffect(() => {
        if (firstRow === undefined || lastRow === undefined || rows.length === 0) return;
        if (hasPrevious && firstRow <= LOAD_MARGIN_ROWS) store.wantMore("previous");
        if (hasNext && lastRow >= rows.length - 1 - LOAD_MARGIN_ROWS) store.wantMore("next");
    }, [firstRow, lastRow, rows.length, hasPrevious, hasNext, version, store]);

    // 見えている位置から遠い Entry を捨てて、長く眺めても保持する量が増え続けないようにする。捨てた側へ戻ったときは、cursor で読み直す。
    // biome-ignore lint/correctness/useExhaustiveDependencies: version は読み込みが終わるたびに保持範囲を確かめ直させる gate である
    useEffect(() => {
        if (firstRow === undefined || lastRow === undefined || rows.length === 0) return;
        const top = firstItemOfRow(rows, Math.min(firstRow, rows.length - 1));
        const bottom = firstItemOfRow(rows, Math.min(lastRow, rows.length - 1));
        store.trim(store.loadedFrom + top - KEEP_ITEMS, store.loadedFrom + bottom + KEEP_ITEMS);
    }, [firstRow, lastRow, rows, version, store]);

    // サムネイルの要求のキュー。見えている行と、その前後の行のタイルを、画面の中央に近い順に要求する。
    const { wants, size } = useImageWants(libraryId, tileSize, narrow, shape);
    // biome-ignore lint/correctness/useExhaustiveDependencies: version は読み込み済みの Entry が増えたときに要求を決め直させる gate である
    useEffect(() => {
        if (firstRow === undefined || lastRow === undefined) return;
        const first = Math.max(0, firstRow - THUMBNAIL_MARGIN_ROWS);
        const last = Math.min(rows.length - 1, lastRow + THUMBNAIL_MARGIN_ROWS);
        const center = (firstRow + lastRow) / 2;
        const wanted = new Map<string, number>();
        const prefetch = new Set<string>();
        for (let rowIndex = first; rowIndex <= last; rowIndex += 1) {
            const row = rows[rowIndex];
            if (row?.type !== "tiles") continue;
            for (let column = 0; column < row.count; column += 1) {
                const entry = items[row.first + column]?.entry;
                if (entry && isThumbnailable(entry)) {
                    wanted.set(entry.path, tilePriority(rowIndex, column, center));
                    // 画面に出ていない行は先読みで、出ている行より後にする。
                    if (rowIndex < firstRow || rowIndex > lastRow) prefetch.add(entry.path);
                }
            }
        }
        wants.setWanted(wanted, new Set(), prefetch);
    }, [firstRow, lastRow, rows, items, version, wants]);

    const focusTile = (index: number) => {
        const position = index - store.loadedFrom;
        const rowIndex = rowOfItem(rows, position);
        if (rowIndex !== undefined) virtualizer.scrollToIndex(rowIndex);
        requestAnimationFrame(() => scrollElement?.querySelector<HTMLElement>(`[data-index="${index}"]`)?.focus());
    };
    const interaction = useEntryInteraction({
        listing: store,
        libraryReadonly,
        columns,
        selection,
        onSelectionChange,
        onProgress,
        onFocus: focusTile,
        onOpen,
        openOnClick: narrow,
        selectAll: false,
        bounds: { first: store.loadedFrom, last: store.loadedTo - 1 },
        verticalTarget: (index, direction) => {
            const target = verticalTarget(rows, index - store.loadedFrom, columns, direction);
            return target === undefined ? undefined : target + store.loadedFrom;
        },
    });
    const { focusedIndex, click, keyDown } = interaction;

    if (!store.isLoaded && store.error) {
        return <LoadFailure error={store.error} label={t("timelineGrid.loadFailed")} onRetry={store.retry} />;
    }
    if (!store.isLoaded) return <p className="p-4 text-muted-foreground">{t("common.loading")}</p>;
    if (items.length === 0)
        return (
            <EntryMenu>
                <p className="p-4 text-muted-foreground">{emptyMessage}</p>
            </EntryMenu>
        );

    // フォーカス中のタイルが仮想化で外れても、Tab で入れるよう、描画している最初のタイルに tabIndex を渡す。
    const renderedIndexes = virtualRows.flatMap((row) => {
        const definition = rows[row.index];
        return definition?.type === "tiles"
            ? Array.from({ length: definition.count }, (_, column) => store.loadedFrom + definition.first + column)
            : [];
    });
    const tabbableIndex = renderedIndexes.includes(focusedIndex) ? focusedIndex : renderedIndexes[0];

    return (
        <EntryMenu>
            <div className="flex h-full flex-col">
                {store.error && (
                    <LoadFailure
                        error={store.error}
                        label={t("timelineGrid.moreFailed")}
                        onRetry={() => {
                            store.retry();
                            if (hasPrevious) store.wantMore("previous");
                            if (hasNext) store.wantMore("next");
                        }}
                        compact
                    />
                )}
                <div
                    ref={setScrollElement}
                    className="min-h-0 flex-1 overflow-auto"
                    role="listbox"
                    aria-multiselectable="true"
                    aria-label={t("common.timeline")}
                >
                    <div style={{ height: total, position: "relative" }}>
                        {virtualRows.map((virtualRow) => {
                            const row = rows[virtualRow.index];
                            if (!row) return null;
                            if (row.type === "heading")
                                return (
                                    <h2
                                        key={row.key}
                                        className="absolute left-0 flex items-end px-3 pb-1 text-sm font-semibold"
                                        style={{ top: starts[virtualRow.index], height: row.height }}
                                    >
                                        {formatDay(row.day)}
                                    </h2>
                                );
                            return (
                                <div
                                    key={row.key}
                                    className="absolute left-0 flex"
                                    style={{
                                        top: starts[virtualRow.index],
                                        height: row.height,
                                        gap: TILE_GAP,
                                        paddingLeft: TILE_GAP,
                                        paddingTop: TILE_GAP,
                                    }}
                                >
                                    {tileIndexes(row).map((position) => {
                                        const entry = items[position]?.entry;
                                        if (!entry) return null;
                                        const index = store.loadedFrom + position;
                                        return (
                                            <EntryMenu key={entry.path} entry={entry}>
                                                <Tile
                                                    entry={entry}
                                                    index={index}
                                                    selected={selection.items.has(entry.path)}
                                                    wants={wants}
                                                    libraryId={libraryId}
                                                    thumbnailSize={size}
                                                    tileSide={tileSide}
                                                    shape={shape}
                                                    focusable={index === tabbableIndex}
                                                    onClick={click}
                                                    onDoubleClick={() => interaction.open(entry)}
                                                    onKeyDown={keyDown}
                                                />
                                            </EntryMenu>
                                        );
                                    })}
                                </div>
                            );
                        })}
                    </div>
                </div>
            </div>
        </EntryMenu>
    );
}

function tileIndexes(row: TilesRow): number[] {
    return Array.from({ length: row.count }, (_, column) => row.first + column);
}

/// 日の見出しの次のタイルの行の、先頭の位置。
function firstTileOfDay(rows: readonly { type: string; day: string; first?: number }[], day: string): number {
    return rows.find((row) => row.type === "tiles" && row.day === day)?.first ?? 0;
}
