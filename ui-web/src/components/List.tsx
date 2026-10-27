import { t } from "i18next";
import { Fragment, type KeyboardEvent, type MouseEvent, useEffect, useState, useSyncExternalStore } from "react";
import type { ImageWants } from "../images/wants";
import { useImageWants } from "../images/wants";
import { useEntryActions } from "../library/actionContext";
import { isCut, useClipboard } from "../library/clipboard";
import { useDragScroll } from "../library/dnd";
import { ENTRY_KIND_LABELS, type Entry, entryKind, isFolderLike, isThumbnailable } from "../library/entries";
import { formatBytes, formatModifiedAt } from "../library/format";
import { type ListingSource, sourceScope, useListing } from "../library/listing";
import type { DisplaySet, SortPreference } from "../library/preferences";
import { type Selection, selectionItem, selectionValue } from "../library/selection";
import { SORT_KEYS, SORT_LABELS, sortFromHeader } from "../library/sort";
import { type SelectionProgress, useEntryInteraction } from "../library/useEntryInteraction";
import { useListingRows } from "../library/useListingRows";
import { useRevealEntry } from "../library/useRevealEntry";
import { type ScrollDirection, tilePriority, wantedPagesForRows, wantedRows } from "../thumbnails/range";
import { EntryIcon } from "./EntryIcon";
import { EntryMenu } from "./EntryMenu";
import { LoadFailure } from "./LoadFailure";

const ROW_HEIGHT = 56;
const THUMBNAIL_SIDE = 40;
const WIDE_COLUMNS = "48px minmax(0, 1fr) 150px 84px 80px";
const SEARCH_COLUMNS = "48px minmax(0, 1fr) minmax(0, 1fr) 150px 84px 80px";
interface ListProps {
    /// 一覧の出どころ。フォルダの直下か、検索である。
    source: ListingSource;
    /// 検索の結果に出す場所。省略すると場所を出さない。
    place?: (entry: Entry) => string;
    libraryReadonly?: boolean;
    shape: DisplaySet["thumbnailShape"];
    narrow: boolean;
    sort: SortPreference;
    onSort: (sort: SortPreference) => void;
    selection: Selection;
    onSelectionChange: (selection: Selection) => void;
    onTotal: (total: number | undefined) => void;
    onProgress: (progress: SelectionProgress | undefined) => void;
    onOpen: (entry: Entry) => void;
    /// 開いたときに選んで見せる、直下の Entry の名前。
    reveal?: string;
    onRevealed?: () => void;
}

/// 行の高さを固定し、未読のページでもスクロールバーが一覧全体を表すようにする。
export function List({
    source,
    place,
    libraryReadonly = false,
    shape,
    narrow,
    sort,
    onSort,
    selection,
    onSelectionChange,
    onTotal,
    onProgress,
    onOpen,
    reveal,
    onRevealed,
}: ListProps) {
    const searching = source.kind === "search";
    const recursive = place !== undefined;
    const libraryId = sourceScope(source);
    const folderPath = source.kind === "folder" ? source.path : "/";
    const [wantedPages, setWantedPages] = useState<ReadonlySet<number>>(new Set());
    const listing = useListing(source, sort, wantedPages);
    const { totalCount, version } = listing;
    const [scrollElement, setScrollElement] = useState<HTMLDivElement | null>(null);
    useDragScroll(scrollElement);
    const { rowCount, virtualizer } = useListingRows(totalCount, 1, ROW_HEIGHT, scrollElement);
    const rows = virtualizer.getVirtualItems();
    useRevealEntry({
        listing,
        columns: 1,
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
    const { wants } = useImageWants(libraryId, 1, false, shape, THUMBNAIL_SIDE);
    const [direction, setDirection] = useState<ScrollDirection>("forward");
    if (virtualizer.scrollDirection !== null && virtualizer.scrollDirection !== direction)
        setDirection(virtualizer.scrollDirection);
    const firstRow = virtualizer.range?.startIndex;
    const lastRow = virtualizer.range?.endIndex;
    const desiredRange =
        firstRow === undefined || lastRow === undefined
            ? undefined
            : wantedRows({ firstRow, lastRow: lastRow, rowCount, direction });
    const desiredPages = desiredRange
        ? wantedPagesForRows(desiredRange.first, desiredRange.last, 1, totalCount ?? 0)
        : new Set<number>();
    if ([...wantedPages].join(",") !== [...desiredPages].join(",")) setWantedPages(desiredPages);
    // biome-ignore lint/correctness/useExhaustiveDependencies: totalCount と version は listing の更新を反映して再計算させる gate で、本体からは参照しない意図的な依存
    useEffect(() => {
        if (firstRow === undefined || lastRow === undefined) return;
        const { first, last, center } = wantedRows({ firstRow, lastRow, rowCount, direction });
        const wanted = new Map<string, number>();
        const delayed = new Set<string>();
        const prefetch = new Set<string>();
        for (let index = first; index <= last; index += 1) {
            const entry = listing.entryAt(index);
            if (entry && isThumbnailable(entry)) {
                if (isFolderLike(entry)) {
                    if (index < firstRow || index > lastRow) continue;
                    delayed.add(entry.path);
                }
                wanted.set(entry.path, tilePriority(index, 0, center));
                // 画面に出ていない行は先読みで、出ている行より後にする。
                if (index < firstRow || index > lastRow) prefetch.add(entry.path);
            }
        }
        wants.setWanted(wanted, delayed, prefetch);
    }, [firstRow, lastRow, rowCount, direction, totalCount, version, listing, wants]);
    useEffect(() => {
        onTotal(totalCount);
    }, [totalCount, onTotal]);
    const focus = (index: number) => {
        virtualizer.scrollToIndex(index);
        requestAnimationFrame(() => scrollElement?.querySelector<HTMLElement>(`[data-index="${index}"]`)?.focus());
    };
    const interaction = useEntryInteraction({
        listing,
        libraryReadonly,
        columns: 1,
        selection,
        onSelectionChange,
        onProgress,
        onFocus: focus,
        onOpen,
        openOnClick: narrow,
    });
    const loadedIndexes = rows.map((row) => row.index).filter((index) => listing.entryAt(index));
    const tabbable = loadedIndexes.includes(interaction.focusedIndex) ? interaction.focusedIndex : loadedIndexes[0];
    if (totalCount === undefined && listing.pageStatus(0) === "error")
        return (
            <LoadFailure
                error={listing.pageError(0)}
                label={t("common.contentsNotFound")}
                onRetry={() => listing.retry(0)}
            />
        );
    if (totalCount === undefined)
        return <p className="p-4 text-muted-foreground">{recursive ? t("common.searching") : t("common.loading")}</p>;
    return (
        <EntryMenu>
            <div className="flex h-full flex-col" data-view="list">
                {listing.truncated && (
                    <p role="status" className="shrink-0 border-b p-3 text-sm text-muted-foreground">
                        {t("common.partialResults")}
                    </p>
                )}
                {!narrow && (
                    // biome-ignore lint/a11y/useFocusableInteractive: CSS グリッドによる疑似テーブルのヘッダ行。ネイティブ table への置換は別課題
                    // biome-ignore lint/a11y/useSemanticElements: CSS グリッドによる疑似テーブルのヘッダ行。ネイティブ table への置換は別課題
                    <div
                        className="grid shrink-0 items-center gap-2 border-b px-3 py-2 text-xs text-muted-foreground"
                        style={{ gridTemplateColumns: recursive ? SEARCH_COLUMNS : WIDE_COLUMNS }}
                        role="row"
                        aria-label={t("list.columns")}
                    >
                        {/* biome-ignore lint/a11y/useFocusableInteractive: CSS グリッドによる疑似テーブルのヘッダセル。ネイティブ table への置換は別課題 */}
                        {/* biome-ignore lint/a11y/useSemanticElements: CSS グリッドによる疑似テーブルのヘッダセル。ネイティブ table への置換は別課題 */}
                        <span role="columnheader" aria-label={t("list.thumbnail")} />
                        {SORT_KEYS.map((key) => (
                            <Fragment key={key}>
                                {/* biome-ignore lint/a11y/useFocusableInteractive: CSS グリッドによる疑似テーブルのヘッダセル。ネイティブ table への置換は別課題 */}
                                {/* biome-ignore lint/a11y/useSemanticElements: CSS グリッドによる疑似テーブルのヘッダセル。ネイティブ table への置換は別課題 */}
                                <div
                                    role="columnheader"
                                    aria-sort={
                                        sort.key === key ? (sort.order === "asc" ? "ascending" : "descending") : "none"
                                    }
                                >
                                    <button
                                        type="button"
                                        className="text-left hover:text-foreground"
                                        onClick={() => onSort(sortFromHeader(sort, key))}
                                    >
                                        {SORT_LABELS[key]}
                                        {sort.key === key ? (sort.order === "asc" ? " ↑" : " ↓") : ""}
                                    </button>
                                </div>
                                {recursive && key === "name" && (
                                    // biome-ignore lint/a11y/useFocusableInteractive: CSS グリッドによる疑似テーブルのヘッダセル。ネイティブ table への置換は別課題
                                    // biome-ignore lint/a11y/useSemanticElements: CSS グリッドによる疑似テーブルのヘッダセル。ネイティブ table への置換は別課題
                                    <span role="columnheader">{t("list.place")}</span>
                                )}
                            </Fragment>
                        ))}
                    </div>
                )}
                {totalCount === 0 ? (
                    <p className="p-4 text-muted-foreground">
                        {searching ? t("common.nothingMatches") : t("common.folderEmpty")}
                    </p>
                ) : (
                    <div
                        ref={setScrollElement}
                        role="listbox"
                        aria-label={folderPath}
                        aria-multiselectable="true"
                        className="min-h-0 flex-1 overflow-auto"
                    >
                        <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
                            {rows.map((row) => {
                                const entry = listing.entryAt(row.index);
                                return (
                                    <div
                                        key={row.key}
                                        className="absolute left-0 right-0"
                                        style={{ top: row.start, height: row.size }}
                                    >
                                        {entry ? (
                                            <EntryMenu entry={entry} allowDrop={false}>
                                                <ListRow
                                                    entry={entry}
                                                    place={place?.(entry)}
                                                    index={row.index}
                                                    narrow={narrow}
                                                    shape={shape}
                                                    wants={wants}
                                                    selected={selection.items.has(entry.path)}
                                                    focusable={row.index === tabbable}
                                                    onClick={interaction.click}
                                                    onKeyDown={interaction.keyDown}
                                                    onOpen={() => interaction.open(entry)}
                                                />
                                            </EntryMenu>
                                        ) : (
                                            <div data-placeholder="" className="h-full border-b bg-muted/40" />
                                        )}
                                    </div>
                                );
                            })}
                        </div>
                    </div>
                )}
                {[...wantedPages]
                    .filter((page) => listing.pageStatus(page) === "error")
                    .map((page) => (
                        <LoadFailure
                            key={page}
                            error={listing.pageError(page)}
                            label={t("common.rangeNotFound")}
                            onRetry={() => listing.retry(page)}
                            compact
                        />
                    ))}
            </div>
        </EntryMenu>
    );
}

interface RowProps {
    entry: Entry;
    place?: string;
    index: number;
    narrow: boolean;
    shape: DisplaySet["thumbnailShape"];
    wants: ImageWants;
    selected: boolean;
    focusable: boolean;
    onClick: (event: MouseEvent, index: number) => void;
    onKeyDown: (event: KeyboardEvent, index: number) => void;
    onOpen: () => void;
}
function ListRow({
    entry,
    place,
    index,
    narrow,
    shape,
    wants,
    selected,
    focusable,
    onClick,
    onKeyDown,
    onOpen,
}: RowProps) {
    const actions = useEntryActions();
    const clipboard = useClipboard();
    const cut = isCut(clipboard.contents, actions?.libraryId ?? "", entry.path);
    const thumbnail = useSyncExternalStore(wants.subscribe, () => wants.getState(entry.path));
    const kind = entryKind(entry);
    const modified = formatModifiedAt(entry.metadata?.modifiedAt);
    const size = formatBytes(entry.metadata?.size);
    return (
        <div
            role="option"
            data-index={index}
            data-kind={kind}
            aria-selected={selected}
            tabIndex={focusable ? 0 : -1}
            title={entry.name}
            className={`h-full items-center gap-2 border-b px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring ${narrow ? "flex" : "grid"} ${selected ? "bg-accent text-accent-foreground" : "hover:bg-muted/50"}`}
            style={{
                opacity: cut ? 0.4 : undefined,
                gridTemplateColumns: narrow ? undefined : place === undefined ? WIDE_COLUMNS : SEARCH_COLUMNS,
            }}
            onClick={(event) => onClick(event, index)}
            onKeyDown={(event) => onKeyDown(event, index)}
            onDoubleClick={onOpen}
        >
            <div
                className="flex shrink-0 items-center justify-center overflow-hidden rounded bg-muted text-muted-foreground"
                style={{ width: THUMBNAIL_SIDE, height: THUMBNAIL_SIDE }}
            >
                {thumbnail.status === "ready" ? (
                    <img
                        src={thumbnail.url}
                        alt=""
                        draggable={false}
                        className={`h-full w-full ${shape === "fit" ? "object-contain" : "object-cover"}`}
                    />
                ) : (
                    <EntryIcon kind={kind} className="size-6" />
                )}
            </div>
            {narrow ? (
                <div className="min-w-0" data-two-lines="">
                    <div className="truncate">{entry.name}</div>
                    <div className="truncate text-xs text-muted-foreground">
                        {place !== undefined && `${place} · `}
                        {modified} · {size}
                    </div>
                </div>
            ) : (
                <>
                    <span className="truncate">{entry.name}</span>
                    {place !== undefined && (
                        <span title={place} className="truncate text-xs text-muted-foreground">
                            {place}
                        </span>
                    )}
                    <span className="text-xs text-muted-foreground">{modified}</span>
                    <span className="text-xs text-muted-foreground">{size}</span>
                    <span className="text-xs text-muted-foreground">{ENTRY_KIND_LABELS[kind]}</span>
                </>
            )}
        </div>
    );
}
