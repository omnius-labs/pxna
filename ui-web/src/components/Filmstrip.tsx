import { useVirtualizer } from "@tanstack/react-virtual";
import { useEffect, useLayoutEffect, useMemo, useRef, useState, useSyncExternalStore } from "react";
import type { ImageWants } from "../images/wants";
import { type Entry, entryKind } from "../library/entries";
import type { Listing } from "../library/listing";
import { filmstripWanted } from "../viewer/filmstrip";
import { type IndexedEntry, isViewable } from "../viewer/viewer";
import { EntryIcon } from "./EntryIcon";

interface FilmstripProps {
    listing: Listing;
    /// 開いている Entry の位置。
    index: number;
    /// The side (CSS px) of one item. The section is `side + 8` tall.
    side: number;
    wants: ImageWants;
    /// 画像か動画の位置を押したときの移動。
    onSelect: (target: IndexedEntry) => void;
    /// 見えている範囲に必要なページ。一覧を読む側へ渡す。
    onPages: (pages: ReadonlySet<number>) => void;
}

/// ビューアの下端に、一覧のすべての Entry を同じ幅で並べる横のスクロール。
/// 画像と動画はサムネイルを出して押せるようにし、それ以外の Entry は種類のアイコンを薄く出す。
export function Filmstrip({ listing, index, side, wants, onSelect, onPages }: FilmstripProps) {
    const totalCount = listing.totalCount ?? 0;
    // 先頭が読み込み済みの位置から始まる一覧（タイムライン）は、その位置から並べる。位置は一覧の絶対の値のまま扱う。
    const start = listing.loadedFrom ?? 0;
    const version = listing.version;
    const [scrollElement, setScrollElement] = useState<HTMLElement | null>(null);
    const [width, setWidth] = useState(0);
    useEffect(() => {
        if (!scrollElement) {
            return;
        }
        const observer = new ResizeObserver(([observed]) => setWidth(observed?.contentRect.width ?? 0));
        observer.observe(scrollElement);
        return () => observer.disconnect();
    }, [scrollElement]);

    const virtualizer = useVirtualizer({
        horizontal: true,
        count: Math.max(0, totalCount - start),
        getScrollElement: () => scrollElement,
        estimateSize: () => side,
        overscan: 10,
    });
    const first =
        virtualizer.range === null || virtualizer.range === undefined
            ? undefined
            : virtualizer.range.startIndex + start;
    const last =
        virtualizer.range === null || virtualizer.range === undefined ? undefined : virtualizer.range.endIndex + start;

    // A new side changes every item's size, so drop the measured sizes before the centering effect below reads them.
    // biome-ignore lint/correctness/useExhaustiveDependencies: side is the gate that makes the virtualizer measure again; the body does not read it
    useEffect(() => {
        virtualizer.measure();
    }, [side, virtualizer]);

    // 前へ読み足して先頭の位置が下がったら、見ている Entry が動かないよう、増えた分だけ送る。
    const previousStart = useRef(start);
    useLayoutEffect(() => {
        const moved = previousStart.current - start;
        previousStart.current = start;
        if (moved !== 0 && scrollElement) scrollElement.scrollLeft += moved * side;
    }, [start, scrollElement, side]);

    // 開いている位置が変わったら、その位置を中央に置く。幅が分かったときにも置き直す。
    // biome-ignore lint/correctness/useExhaustiveDependencies: width は幅が分かったときに中央へ置き直させる gate で、本体からは参照しない意図的な依存
    useEffect(() => {
        if (scrollElement) {
            virtualizer.scrollToIndex(index - start, { align: "center" });
        }
    }, [scrollElement, index, width, side, virtualizer]);

    const pagesKey =
        first === undefined || last === undefined
            ? ""
            : [...filmstripWanted({ first, last, count: totalCount, entryAt: listing.entryAt }).pages].join(",");
    const pages = useMemo(() => new Set(pagesKey === "" ? [] : pagesKey.split(",").map(Number)), [pagesKey]);
    useEffect(() => {
        onPages(pages);
    }, [pages, onPages]);
    useEffect(() => () => onPages(new Set()), [onPages]);

    // biome-ignore lint/correctness/useExhaustiveDependencies: version は読み込み済みのページが増えたときに再計算させる gate で、本体からは参照しない意図的な依存
    useEffect(() => {
        if (first === undefined || last === undefined) {
            return;
        }
        const { thumbnails, prefetch } = filmstripWanted({ first, last, count: totalCount, entryAt: listing.entryAt });
        wants.setWanted(thumbnails, new Set(), prefetch);
    }, [first, last, totalCount, version, listing, wants]);
    useEffect(() => () => wants.setWanted(new Map()), [wants]);

    return (
        <section
            ref={setScrollElement}
            aria-label="フィルムストリップ"
            className="shrink-0 overflow-x-auto overflow-y-hidden border-t border-white/10"
            style={{ height: side + 8 }}
        >
            <div style={{ width: virtualizer.getTotalSize(), height: side, position: "relative" }}>
                {virtualizer.getVirtualItems().map((item) => (
                    <div
                        key={item.key}
                        className="absolute top-0"
                        style={{ left: item.start, width: item.size, height: side }}
                    >
                        <FilmstripItem
                            entry={listing.entryAt(item.index + start)}
                            current={item.index + start === index}
                            side={side}
                            wants={wants}
                            onSelect={(entry) => onSelect({ index: item.index + start, entry })}
                        />
                    </div>
                ))}
            </div>
        </section>
    );
}

interface FilmstripItemProps {
    /// 未読の位置は `undefined`。
    entry: Entry | undefined;
    current: boolean;
    side: number;
    wants: ImageWants;
    onSelect: (entry: Entry) => void;
}

function FilmstripItem({ entry, current, side, wants, onSelect }: FilmstripItemProps) {
    // The item has a 2 CSS px margin on each side.
    const inner = { width: side - 4, height: side - 4 };
    const thumbnail = useSyncExternalStore(wants.subscribe, () => (entry ? wants.getState(entry.path) : IDLE));
    if (!entry) {
        return <div data-placeholder="" className="m-0.5 rounded bg-white/5" style={inner} />;
    }
    const kind = entryKind(entry);
    if (!isViewable(entry)) {
        return (
            <div
                role="img"
                aria-label={entry.name}
                title={entry.name}
                className="m-0.5 flex items-center justify-center rounded bg-white/5 text-white/30"
                style={inner}
            >
                <EntryIcon kind={kind} className="size-6" />
            </div>
        );
    }
    return (
        <button
            type="button"
            aria-label={entry.name}
            aria-current={current ? "true" : undefined}
            title={entry.name}
            // フィルムストリップの中は、矢印キーでの移動で足りるので、Tab で止まるのは開いている位置だけにする。
            tabIndex={current ? 0 : -1}
            style={inner}
            className={`m-0.5 flex items-center justify-center overflow-hidden rounded bg-white/10 outline-none focus-visible:ring-2 focus-visible:ring-white ${current ? "ring-2 ring-white" : "opacity-60 hover:opacity-100"}`}
            onClick={() => {
                if (!current) onSelect(entry);
            }}
        >
            {thumbnail.status === "ready" ? (
                <img src={thumbnail.url} alt="" draggable={false} className="h-full w-full object-cover" />
            ) : (
                <EntryIcon kind={kind} className="size-6 text-white/30" />
            )}
        </button>
    );
}

const IDLE = { status: "idle" } as const;
