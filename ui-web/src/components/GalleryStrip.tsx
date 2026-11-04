import { t } from "i18next";
import { useEffect, useLayoutEffect, useMemo, useRef, useState, useSyncExternalStore } from "react";
import type { ImageWants } from "../images/wants";
import { useImageWants } from "../images/wants";
import { type Entry, entryKind } from "../library/entries";
import type { Listing } from "../library/listing";
import { useFlipbook } from "../thumbnails/useFlipbook";
import { FILMSTRIP_LOOKAHEAD, filmstripWanted } from "../viewer/filmstrip";
import {
    anchoredScrollLeft,
    centeredScrollLeft,
    galleryAnchor,
    galleryItemHeight,
    galleryLayout,
    galleryVisibleRange,
    galleryWindow,
    galleryWindowCenter,
    nearestViewable,
} from "../viewer/gallery";
import type { IndexedEntry } from "../viewer/viewer";
import { EntryIcon } from "./EntryIcon";

/// How long (ms) scrolling must stop before the item at the center becomes the open Entry.
const SCROLL_IDLE_MS = 150;
/// The longest distance, in display areas, that a move scrolls smoothly. Beyond it, the move is immediate.
const SMOOTH_SCROLL_SCREENS = 2;
/// The pixels of one line and one page of a wheel that does not report pixels.
const WHEEL_LINE_PX = 16;

interface GalleryStripProps {
    listing: Listing;
    /// The scope of the wanted images: a Library id or the cross-Library scope.
    libraryId: string;
    /// The open Entry's position.
    index: number;
    /// The side (CSS px) the thumbnails are requested at, before the pixel density.
    side: number;
    /// Opens an item: a click, or the item nearest the center after scrolling stops.
    onSelect: (target: IndexedEntry) => void;
    /// Opens an item in focus mode: a double click.
    onFocus: (target: IndexedEntry) => void;
    /// The pages the visible range needs. They go to the side that reads the listing.
    onPages: (pages: ReadonlySet<number>) => void;
}

/// Lays out the listing's images and videos horizontally with their own aspect ratios, in the viewer's gallery mode.
/// Only a window of positions around the viewed place is laid out, and it moves along with scrolling and moves.
/// Each aspect ratio comes from the loaded thumbnail, so widths change as thumbnails arrive. The open Entry stays at
/// the center until the user scrolls, and after that the place at the center stays where it is.
export function GalleryStrip({ listing, libraryId, index, side, onSelect, onFocus, onPages }: GalleryStripProps) {
    const totalCount = listing.totalCount ?? 0;
    const listingFirst = listing.loadedFrom ?? 0;
    const version = listing.version;
    const [element, setElement] = useState<HTMLElement | null>(null);
    const [area, setArea] = useState({ width: 0, height: 0 });
    useEffect(() => {
        if (!element) {
            return;
        }
        const observer = new ResizeObserver(([observed]) =>
            setArea({ width: observed?.contentRect.width ?? 0, height: observed?.contentRect.height ?? 0 }),
        );
        observer.observe(element);
        return () => observer.disconnect();
    }, [element]);

    const { wants, size } = useImageWants(libraryId, 1, false, "fit", side);
    const [aspects] = useState(() => new Map<string, number>());
    const [aspectVersion, setAspectVersion] = useState(0);
    const recordAspect = (path: string, aspect: number) => {
        if (aspects.get(path) !== aspect) {
            aspects.set(path, aspect);
            setAspectVersion((value) => value + 1);
        }
    };

    // A move outside the window, such as from the position meter, brings the window along before anything is drawn.
    const [windowCenter, setWindowCenter] = useState(index);
    const span = galleryWindow(windowCenter, listingFirst, totalCount);
    if ((index < span.first || index >= span.first + span.count) && windowCenter !== index) {
        setWindowCenter(index);
    }

    const itemHeight = galleryItemHeight(area.height);
    // biome-ignore lint/correctness/useExhaustiveDependencies: version and aspectVersion are gates for the reads of entryAt and aspects, which keep their identity
    const layout = useMemo(
        () =>
            galleryLayout({
                first: span.first,
                count: span.count,
                height: itemHeight,
                viewport: area.width,
                entryAt: listing.entryAt,
                aspectOf: (path) => aspects.get(path),
            }),
        [span.first, span.count, itemHeight, area.width, listing, version, aspects, aspectVersion],
    );

    // The position to keep at the center. A move sets it, and a scroll by the user clears it.
    const follow = useRef<number | undefined>(index);
    // The position this strip reported after scrolling stopped. Coming back as `index`, it must not move the scroll.
    const reported = useRef<number | undefined>(undefined);
    const [visible, setVisible] = useState<{ first: number; last: number } | undefined>(undefined);
    const syncVisible = (scrollLeft: number) => {
        const range = galleryVisibleRange(layout, scrollLeft);
        setVisible((previous) =>
            previous?.first === range?.first && previous?.last === range?.last ? previous : range,
        );
    };

    // Notes a move before the layout below runs, so that a move that also changes the window centers on its target.
    const moved = useRef(false);
    useLayoutEffect(() => {
        if (reported.current === index) {
            reported.current = undefined;
            return;
        }
        reported.current = undefined;
        follow.current = index;
        moved.current = true;
    }, [index]);

    // Keeps the place while widths change, before the browser paints.
    const laidOut = useRef(layout);
    // biome-ignore lint/correctness/useExhaustiveDependencies: syncVisible reads the layout of this render, the only dependency that matters
    useLayoutEffect(() => {
        const previous = laidOut.current;
        laidOut.current = layout;
        if (!element || area.width === 0) {
            return;
        }
        if (follow.current !== undefined) {
            element.scrollLeft = centeredScrollLeft(layout, follow.current);
        } else {
            const anchor = previous.viewport > 0 ? galleryAnchor(previous, element.scrollLeft) : undefined;
            if (anchor) element.scrollLeft = anchoredScrollLeft(layout, anchor);
        }
        syncVisible(element.scrollLeft);
    }, [layout, element, area.width]);

    // The window follows the display area before it reaches an end of the window.
    useEffect(() => {
        if (visible === undefined) {
            return;
        }
        const center = galleryWindowCenter(layout, visible, listingFirst, totalCount);
        if (center !== undefined) setWindowCenter(center);
    }, [layout, visible, listingFirst, totalCount]);

    // A move from outside the strip brings the new position to the center. A position the strip reported itself stays where it is.
    // biome-ignore lint/correctness/useExhaustiveDependencies: only a change of the open position moves the scroll, and layout changes are handled above
    useEffect(() => {
        if (!moved.current) {
            return;
        }
        moved.current = false;
        if (!element || area.width === 0) {
            return;
        }
        const left = centeredScrollLeft(layout, index);
        const near = Math.abs(left - element.scrollLeft) < area.width * SMOOTH_SCROLL_SCREENS;
        const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
        element.scrollTo({ left, behavior: near && !reduced ? "smooth" : "auto" });
        syncVisible(element.scrollLeft);
    }, [index]);

    // After scrolling by the user stops, the item nearest the center becomes the open Entry.
    const idleTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
    useEffect(() => () => clearTimeout(idleTimer.current), []);
    const scrolled = () => {
        if (!element) {
            return;
        }
        syncVisible(element.scrollLeft);
        clearTimeout(idleTimer.current);
        awaitingPage.current = false;
        if (follow.current !== undefined) {
            return;
        }
        idleTimer.current = setTimeout(() => settle.current(), SCROLL_IDLE_MS);
    };
    // The timer reads the values of the latest render, because thumbnails can change the layout while it waits.
    const settle = useRef(() => {});
    // Scrolling stopped on an unread position, so the choice waits for its page.
    const awaitingPage = useRef(false);
    settle.current = () => {
        awaitingPage.current = false;
        if (!element || follow.current !== undefined) {
            return;
        }
        const anchor = galleryAnchor(layout, element.scrollLeft);
        if (anchor !== undefined && listing.entryAt(anchor.index) === undefined) {
            awaitingPage.current = true;
            return;
        }
        const nearest = nearestViewable(layout, element.scrollLeft, listing.entryAt);
        if (nearest !== undefined && nearest.index !== index) {
            reported.current = nearest.index;
            onSelect(nearest);
        }
    };
    // biome-ignore lint/correctness/useExhaustiveDependencies: version is the gate: a page that has arrived can decide a waiting choice
    useEffect(() => {
        if (awaitingPage.current) settle.current();
    }, [version]);
    const onScroll = useRef(scrolled);
    onScroll.current = scrolled;

    // The wheel scrolls horizontally. It is not passive, so that the page does not take it.
    useEffect(() => {
        if (!element) {
            return;
        }
        const wheel = (event: WheelEvent) => {
            if (event.ctrlKey) {
                return;
            }
            event.preventDefault();
            follow.current = undefined;
            const delta = Math.abs(event.deltaX) > Math.abs(event.deltaY) ? event.deltaX : event.deltaY;
            const unit =
                event.deltaMode === WheelEvent.DOM_DELTA_LINE
                    ? WHEEL_LINE_PX
                    : event.deltaMode === WheelEvent.DOM_DELTA_PAGE
                      ? element.clientWidth
                      : 1;
            element.scrollLeft += delta * unit;
            onScroll.current();
        };
        element.addEventListener("wheel", wheel, { passive: false });
        return () => element.removeEventListener("wheel", wheel);
    }, [element]);

    // biome-ignore lint/correctness/useExhaustiveDependencies: version is the gate that recalculates when more pages are read
    const wanted = useMemo(
        () =>
            visible === undefined
                ? undefined
                : filmstripWanted({
                      first: visible.first,
                      last: visible.last,
                      count: totalCount,
                      entryAt: listing.entryAt,
                  }),
        [visible, totalCount, version, listing],
    );
    const pagesKey = wanted === undefined ? "" : [...wanted.pages].join(",");
    const pages = useMemo(() => new Set(pagesKey === "" ? [] : pagesKey.split(",").map(Number)), [pagesKey]);
    useEffect(() => {
        onPages(pages);
    }, [pages, onPages]);
    useEffect(() => () => onPages(new Set()), [onPages]);
    useEffect(() => {
        if (wanted !== undefined) wants.setWanted(wanted.thumbnails, new Set(), wanted.prefetch);
    }, [wanted, wants]);

    const rendered: number[] = [];
    if (visible !== undefined) {
        const from = Math.max(layout.first, visible.first - FILMSTRIP_LOOKAHEAD);
        const to = Math.min(layout.first + layout.count - 1, visible.last + FILMSTRIP_LOOKAHEAD);
        for (let position = from; position <= to; position += 1) {
            if ((layout.widths[position - layout.first] ?? 0) > 0) rendered.push(position);
        }
    }

    return (
        <section
            ref={setElement}
            aria-label={t("gallery.label")}
            className="absolute inset-0 overflow-x-auto overflow-y-hidden select-none"
            style={{ scrollbarWidth: "none" }}
            onScroll={scrolled}
            onPointerDown={() => {
                follow.current = undefined;
            }}
        >
            <div className="relative h-full" style={{ width: layout.total }}>
                {rendered.map((position) => {
                    const entry = listing.entryAt(position);
                    return (
                        <div
                            key={position}
                            className="absolute"
                            style={{
                                left: layout.starts[position - layout.first],
                                width: layout.widths[position - layout.first],
                                top: (area.height - itemHeight) / 2,
                                height: itemHeight,
                            }}
                        >
                            {entry ? (
                                <GalleryItem
                                    entry={entry}
                                    current={position === index}
                                    libraryId={libraryId}
                                    wants={wants}
                                    size={size}
                                    side={Math.max(layout.widths[position - layout.first] ?? 0, itemHeight)}
                                    onAspect={recordAspect}
                                    onClick={() => {
                                        if (position === index) {
                                            follow.current = index;
                                            element?.scrollTo({
                                                left: centeredScrollLeft(layout, index),
                                                behavior: "smooth",
                                            });
                                        } else {
                                            onSelect({ index: position, entry });
                                        }
                                    }}
                                    onDoubleClick={() => onFocus({ index: position, entry })}
                                />
                            ) : (
                                <div data-placeholder="" className="h-full w-full rounded bg-white/5" />
                            )}
                        </div>
                    );
                })}
            </div>
        </section>
    );
}

interface GalleryItemProps {
    entry: Entry;
    current: boolean;
    libraryId: string;
    wants: ImageWants;
    size: number;
    side: number;
    onAspect: (path: string, aspect: number) => void;
    onClick: () => void;
    onDoubleClick: () => void;
}

function GalleryItem({
    entry,
    current,
    libraryId,
    wants,
    size,
    side,
    onAspect,
    onClick,
    onDoubleClick,
}: GalleryItemProps) {
    const thumbnail = useSyncExternalStore(wants.subscribe, () => wants.getState(entry.path));
    const kind = entryKind(entry);
    // While the mouse is over a video, the frames of its sprite sheet cycle. Nothing starts a transcode here.
    const [hovering, setHovering] = useState(false);
    const flipbook = useFlipbook(hovering && kind === "video", libraryId, entry.path, size, "fit", side);
    return (
        <button
            type="button"
            aria-label={entry.name}
            aria-current={current ? "true" : undefined}
            data-kind={kind}
            title={entry.name}
            // Tab stops only at the open position, as in the filmstrip.
            tabIndex={current ? 0 : -1}
            className={`relative flex h-full w-full items-center justify-center overflow-hidden rounded outline-none transition-opacity focus-visible:ring-2 focus-visible:ring-white ${current ? "" : "opacity-60 hover:opacity-100"}`}
            onClick={onClick}
            onDoubleClick={onDoubleClick}
            onMouseEnter={() => setHovering(true)}
            onMouseLeave={() => setHovering(false)}
        >
            {flipbook ? (
                <div data-flipbook="" style={flipbook} />
            ) : thumbnail.status === "ready" ? (
                <img
                    src={thumbnail.url}
                    alt=""
                    draggable={false}
                    className="h-full w-full object-contain"
                    onLoad={(event) => {
                        const { naturalWidth, naturalHeight } = event.currentTarget;
                        if (naturalWidth > 0 && naturalHeight > 0) onAspect(entry.path, naturalWidth / naturalHeight);
                    }}
                />
            ) : (
                <EntryIcon kind={kind} className="size-16 text-white/30" />
            )}
            {kind === "video" && (
                <span
                    role="img"
                    aria-label={t("tile.video")}
                    className="absolute bottom-2 right-2 rounded bg-black/60 px-1.5 text-xs leading-5 text-white"
                >
                    ▶
                </span>
            )}
        </button>
    );
}
