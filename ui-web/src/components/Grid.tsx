import { useEffect, useState } from "react";
import { useDragScroll } from "../library/dnd";
import { type Entry, isFolderLike, isThumbnailable } from "../library/entries";
import { gridLayout, PLACE_HEIGHT, rowHeight, TILE_GAP } from "../library/grid";
import { type ListingSource, sourceScope, useListing } from "../library/listing";
import type { DisplaySet, SortPreference } from "../library/preferences";
import { type Selection, selectionItem, selectionValue } from "../library/selection";
import { type SelectionProgress, useEntryInteraction } from "../library/useEntryInteraction";
import { useListingRows } from "../library/useListingRows";
import { useRevealEntry } from "../library/useRevealEntry";
import { type ScrollDirection, tilePriority, wantedPagesForRows, wantedRows } from "../thumbnails/range";
import { useThumbnailQueue } from "../thumbnails/useThumbnailQueue";
import { EntryMenu } from "./EntryMenu";
import { LoadFailure } from "./LoadFailure";
import { Tile } from "./Tile";

interface GridProps {
    /// 一覧の出どころ。フォルダの直下か、検索である。
    source: ListingSource;
    /// 検索の結果に出す場所。省略すると場所を出さない。
    place?: (entry: Entry) => string;
    libraryReadonly?: boolean;
    /// タイルサイズの段階（1 から 5）。
    tileSize: number;
    shape: DisplaySet["thumbnailShape"];
    /// 狭い画面の区分か。段階を列の数として扱う。
    narrow: boolean;
    sort: SortPreference;
    selection: Selection;
    onSelectionChange: (selection: Selection) => void;
    onTotal: (totalCount: number | undefined) => void;
    onProgress: (progress: SelectionProgress | undefined) => void;
    onOpen: (entry: Entry) => void;
    /// 開いたときに選んで見せる、直下の Entry の名前。
    reveal?: string;
    onRevealed?: () => void;
}

/// 全件の高さを先に確保し、見えている範囲と先読みのページだけを読む。
export function Grid({
    source,
    place,
    libraryReadonly = false,
    tileSize,
    shape,
    narrow,
    sort,
    selection,
    onSelectionChange,
    onTotal,
    onProgress,
    onOpen,
    reveal,
    onRevealed,
}: GridProps) {
    const searching = source.kind === "search";
    const recursive = place !== undefined;
    const libraryId = sourceScope(source);
    const folderPath = source.kind === "folder" ? source.path : "/";
    const [wantedPages, setWantedPages] = useState<ReadonlySet<number>>(new Set());
    const listing = useListing(source, sort, wantedPages);
    const { totalCount, version } = listing;

    // スクロール領域は、一覧を読み終えるまで描画されない。要素を state に持ち、現れたときに幅を測り始める。
    const [scrollElement, setScrollElement] = useState<HTMLDivElement | null>(null);
    useDragScroll(scrollElement);
    const [width, setWidth] = useState(0);
    useEffect(() => {
        if (!scrollElement) {
            return;
        }
        const observer = new ResizeObserver(([observed]) => setWidth(observed?.contentRect.width ?? 0));
        observer.observe(scrollElement);
        return () => observer.disconnect();
    }, [scrollElement]);

    const { columns, tileSide } = gridLayout({ width, tileSize, narrow });
    const { rowCount, virtualizer } = useListingRows(
        totalCount,
        columns,
        rowHeight(tileSide, recursive ? PLACE_HEIGHT : 0),
        scrollElement,
    );
    const virtualRows = virtualizer.getVirtualItems();
    useRevealEntry({
        listing,
        columns,
        virtualizer,
        folderPath,
        name: reveal,
        onSelect: (entry, index) =>
            onSelectionChange({
                items: new Map([[entry.path, selectionValue(selectionItem(entry, libraryReadonly))]]),
                anchor: index,
            }),
        onDone: () => onRevealed?.(),
    });

    // サムネイルの要求のキュー。見えているタイルと、スクロールの方向へ画面 1 枚分の先読みのタイルだけを、画面の中央に近い順に要求する。
    // 大きさと形は、タイルサイズか形を変えたときに決め直し、サムネイルとスプライトシートに同じ値を使う。
    // 一覧の幅が変わっただけでは決め直さない。幅を変えている間に、要求が続けて出ないようにするためである。
    const { queue, size } = useThumbnailQueue(libraryId, tileSize, narrow, shape);

    // スクロールが止まると方向は null に戻るので、最後にスクロールした方向を覚える。
    const [direction, setDirection] = useState<ScrollDirection>("forward");
    const scrolling = virtualizer.scrollDirection;
    if (scrolling !== null && scrolling !== direction) {
        setDirection(scrolling);
    }
    const range = virtualizer.range;
    const firstRow = range?.startIndex;
    const lastVisibleRow = range?.endIndex;
    const desiredRange =
        firstRow === undefined || lastVisibleRow === undefined
            ? undefined
            : wantedRows({ firstRow, lastRow: lastVisibleRow, rowCount, direction });
    const desiredPages = desiredRange
        ? wantedPagesForRows(desiredRange.first, desiredRange.last, columns, totalCount ?? 0)
        : new Set<number>();
    if ([...wantedPages].join(",") !== [...desiredPages].join(",")) setWantedPages(desiredPages);
    // biome-ignore lint/correctness/useExhaustiveDependencies: listing は version 経由で更新され、totalCount と version は再計算の gate として意図的に指定している
    useEffect(() => {
        if (firstRow === undefined || lastVisibleRow === undefined) {
            return;
        }
        const { first, last, center } = wantedRows({ firstRow, lastRow: lastVisibleRow, rowCount, direction });
        const wanted = new Map<string, number>();
        const delayed = new Set<string>();
        for (let row = first; row <= last; row += 1) {
            for (let column = 0; column < columns; column += 1) {
                const entry = listing.entryAt(row * columns + column);
                if (entry && isThumbnailable(entry)) {
                    if (isFolderLike(entry)) {
                        if (row < firstRow || row > lastVisibleRow) continue;
                        delayed.add(entry.path);
                    }
                    wanted.set(entry.path, tilePriority(row, column, center));
                }
            }
        }
        queue.setWanted(wanted, delayed);
    }, [firstRow, lastVisibleRow, direction, columns, rowCount, totalCount, version, queue]);

    useEffect(() => {
        onTotal(totalCount);
    }, [totalCount, onTotal]);
    const focusTile = (index: number) => {
        virtualizer.scrollToIndex(Math.floor(index / columns));
        requestAnimationFrame(() => scrollElement?.querySelector<HTMLElement>(`[data-index="${index}"]`)?.focus());
    };
    const interaction = useEntryInteraction({
        listing,
        libraryReadonly,
        columns,
        selection,
        onSelectionChange,
        onProgress,
        onFocus: focusTile,
        onOpen,
        openOnClick: narrow,
    });
    const { focusedIndex, click, keyDown } = interaction;

    const notice = listing.truncated ? (
        <p role="status" className="shrink-0 border-b p-3 text-sm text-muted-foreground">
            結果の一部を表示しています。Library を絞るか、場所を下げて探し直してください。
        </p>
    ) : null;
    if (totalCount === undefined && listing.pageStatus(0) === "error") {
        return (
            <LoadFailure
                error={listing.pageError(0)}
                label="中身を読めませんでした。"
                onRetry={() => listing.retry(0)}
            />
        );
    }
    if (totalCount === undefined)
        return <p className="p-4 text-muted-foreground">{recursive ? "検索中…" : "読み込み中…"}</p>;
    if (totalCount === 0)
        return (
            <EntryMenu>
                <div className="flex h-full flex-col">
                    {notice}
                    <p className="p-4 text-muted-foreground">
                        {searching ? "一致する項目がありません。" : "このフォルダは空です。"}
                    </p>
                </div>
            </EntryMenu>
        );

    // フォーカス中のタイルが仮想化で外れても、Tab で入れるよう、描画している最初のタイルに tabIndex を渡す。
    const renderedIndexes = virtualRows
        .flatMap((row) => Array.from({ length: columns }, (_, column) => row.index * columns + column))
        .filter((index) => listing.entryAt(index) !== undefined);
    const tabbableIndex = renderedIndexes.includes(focusedIndex) ? focusedIndex : renderedIndexes[0];

    return (
        <EntryMenu>
            <div className="flex h-full flex-col">
                {notice}
                <div
                    ref={setScrollElement}
                    className="min-h-0 flex-1 overflow-auto"
                    role="listbox"
                    aria-multiselectable="true"
                    aria-label={folderPath}
                >
                    <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
                        {virtualRows.map((row) => (
                            <div
                                key={row.key}
                                className="absolute left-0 flex"
                                style={{
                                    top: row.start,
                                    height: row.size,
                                    gap: TILE_GAP,
                                    paddingLeft: TILE_GAP,
                                    paddingTop: TILE_GAP,
                                }}
                            >
                                {Array.from({ length: columns }, (_, column) => row.index * columns + column).map(
                                    (index) => {
                                        const entry = listing.entryAt(index);
                                        if (index >= totalCount) return null;
                                        if (!entry)
                                            return (
                                                <div
                                                    key={index}
                                                    data-placeholder=""
                                                    className="rounded bg-muted"
                                                    style={{ width: tileSide, height: tileSide }}
                                                />
                                            );
                                        return (
                                            <EntryMenu key={entry.path} entry={entry}>
                                                <Tile
                                                    key={entry.path}
                                                    entry={entry}
                                                    place={place?.(entry)}
                                                    index={index}
                                                    selected={selection.items.has(entry.path)}
                                                    queue={queue}
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
                                    },
                                )}
                            </div>
                        ))}
                    </div>
                </div>
                {[...wantedPages]
                    .filter((page) => listing.pageStatus(page) === "error")
                    .map((page) => (
                        <LoadFailure
                            key={page}
                            error={listing.pageError(page)}
                            label="この範囲を読めませんでした。"
                            onRetry={() => listing.retry(page)}
                            compact
                        />
                    ))}
            </div>
        </EntryMenu>
    );
}
