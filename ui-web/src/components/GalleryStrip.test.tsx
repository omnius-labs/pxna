// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { Entry } from "../library/entries";
import type { Listing } from "../library/listing";
import { FakeImageSource } from "../testing/images";
import { stubMatchMedia } from "../testing/preferences";
import { GalleryStrip } from "./GalleryStrip";

const AREA = { width: 1000, height: 800 };
const ITEM_HEIGHT = 600;

const image = (index: number): Entry => ({
    name: `img-${index}.png`,
    path: `/img-${index}.png`,
    kind: "file",
    metadata: { size: 1, readonly: false, contentType: "image/png" },
    file: { archive: false },
});
const video: Entry = {
    name: "clip.mp4",
    path: "/clip.mp4",
    kind: "file",
    metadata: { size: 1, readonly: false, contentType: "video/mp4" },
    file: { archive: false },
};

function listingOf(entries: (Entry | undefined)[]): Listing {
    return {
        truncated: false,
        totalCount: entries.length,
        version: 1,
        snapshot: "one",
        entryAt: (index) => entries[index],
        pageStatus: () => "loaded",
        pageError: () => null,
        loadPages: async () => {},
        retry: () => {},
    };
}

let source: FakeImageSource;
let uninstall: () => void;
let restoreMatchMedia: () => void;
const originalResizeObserver = globalThis.ResizeObserver;
beforeEach(() => {
    vi.useFakeTimers();
    source = new FakeImageSource();
    uninstall = source.install();
    restoreMatchMedia = stubMatchMedia(false);
    // The display area has a size at once, so the first layout is the real one.
    globalThis.ResizeObserver = class {
        readonly #callback: ResizeObserverCallback;
        constructor(callback: ResizeObserverCallback) {
            this.#callback = callback;
        }
        observe() {
            this.#callback([{ contentRect: AREA } as ResizeObserverEntry], this as unknown as ResizeObserver);
        }
        unobserve() {}
        disconnect() {}
    } as unknown as typeof ResizeObserver;
    Element.prototype.scrollTo = function scrollTo(this: Element, options?: ScrollToOptions | number) {
        if (typeof options === "object") this.scrollLeft = options.left ?? this.scrollLeft;
    } as typeof Element.prototype.scrollTo;
});
afterEach(() => {
    vi.useRealTimers();
    uninstall();
    restoreMatchMedia();
    globalThis.ResizeObserver = originalResizeObserver;
    document.body.replaceChildren();
});

async function render(listing: Listing, props: Partial<React.ComponentProps<typeof GalleryStrip>> = {}) {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    const onSelect = vi.fn();
    const onFocus = vi.fn();
    const onPages = vi.fn();
    let current = listing;
    const element = (index: number) => (
        <GalleryStrip
            libraryId="lib"
            side={500}
            onSelect={onSelect}
            onFocus={onFocus}
            onPages={onPages}
            {...props}
            listing={current}
            index={index}
        />
    );
    await act(async () => root.render(element(props.index ?? 0)));
    const strip = () => host.querySelector("section") as HTMLElement;
    const item = (name: string) => host.querySelector<HTMLElement>(`button[aria-label='${name}']`);
    const box = (name: string) => item(name)?.parentElement as HTMLElement;
    return {
        host,
        strip,
        item,
        box,
        onSelect,
        onFocus,
        onPages,
        rerender: (index: number) => act(async () => root.render(element(index))),
        rerenderListing: (next: Listing) => {
            current = next;
            return act(async () => root.render(element(props.index ?? 0)));
        },
    };
}

/// Gives a rendered item its thumbnail with an aspect ratio, as if the image had loaded.
async function load(view: Awaited<ReturnType<typeof render>>, entry: Entry, width: number, height: number) {
    await act(async () => {
        const spec = { libraryId: "lib", path: entry.path, kind: "thumbnail", width: 500, height: 500, shape: "fit" };
        source.ready(spec as never, `blob:${entry.name}`);
    });
    const img = view.item(entry.name)?.querySelector("img") as HTMLImageElement;
    Object.defineProperty(img, "naturalWidth", { value: width, configurable: true });
    Object.defineProperty(img, "naturalHeight", { value: height, configurable: true });
    await act(async () => {
        img.dispatchEvent(new Event("load", { bubbles: false }));
    });
}

it("lays out an item with its aspect ratio once its thumbnail loads", async () => {
    const view = await render(listingOf([image(0), image(1)]));
    expect(view.box("img-0.png").style.width).toBe(`${ITEM_HEIGHT}px`);
    await load(view, image(0), 300, 600);
    expect(view.box("img-0.png").style.width).toBe(`${ITEM_HEIGHT / 2}px`);
});

it("keeps the open Entry at the center when an earlier thumbnail changes its width", async () => {
    const view = await render(listingOf([image(0), image(1), image(2)]), { index: 1 });
    const center = () => view.strip().scrollLeft + AREA.width / 2;
    const middle = () => Number.parseFloat(view.box("img-1.png").style.left) + ITEM_HEIGHT / 2;
    expect(center()).toBe(middle());
    await load(view, image(0), 300, 600);
    expect(center()).toBe(middle());
});

it("keeps the place at the center after the user scrolls", async () => {
    const view = await render(listingOf([image(0), image(1), image(2)]), { index: 1 });
    await act(async () => {
        view.strip().dispatchEvent(new WheelEvent("wheel", { deltaY: 40, bubbles: true, cancelable: true }));
    });
    const offset = () => view.strip().scrollLeft + AREA.width / 2 - Number.parseFloat(view.box("img-1.png").style.left);
    const before = offset();
    await load(view, image(0), 300, 600);
    expect(offset()).toBeCloseTo(before);
});

it("scrolls horizontally with the wheel and does not let the page take it", async () => {
    const view = await render(listingOf([image(0), image(1)]));
    const left = view.strip().scrollLeft;
    const wheel = new WheelEvent("wheel", { deltaY: 120, bubbles: true, cancelable: true });
    await act(async () => {
        view.strip().dispatchEvent(wheel);
    });
    expect(view.strip().scrollLeft).toBe(left + 120);
    expect(wheel.defaultPrevented).toBe(true);
});

it("opens the item nearest the center after scrolling stops, without scrolling back", async () => {
    const view = await render(listingOf([image(0), image(1)]));
    const toSecond = ITEM_HEIGHT + 24;
    await act(async () => {
        view.strip().dispatchEvent(new WheelEvent("wheel", { deltaY: toSecond, bubbles: true, cancelable: true }));
    });
    expect(view.onSelect).not.toHaveBeenCalled();
    await act(async () => {
        vi.advanceTimersByTime(150);
    });
    expect(view.onSelect).toHaveBeenCalledWith({ index: 1, entry: image(1) });
    const left = view.strip().scrollLeft;
    await view.rerender(1);
    expect(view.strip().scrollLeft).toBe(left);
});

it("does not report an item while a move is keeping it at the center", async () => {
    const view = await render(listingOf([image(0), image(1)]));
    await view.rerender(1);
    await act(async () => {
        view.strip().dispatchEvent(new Event("scroll"));
        vi.advanceTimersByTime(150);
    });
    expect(view.onSelect).not.toHaveBeenCalled();
});

it("brings a position to the center when the open Entry changes from outside", async () => {
    const view = await render(listingOf([image(0), image(1)]));
    await view.rerender(1);
    const middle = Number.parseFloat(view.box("img-1.png").style.left) + ITEM_HEIGHT / 2;
    expect(view.strip().scrollLeft + AREA.width / 2).toBe(middle);
});

it("reports a click and a double click on an item", async () => {
    const view = await render(listingOf([image(0), image(1)]));
    await act(async () => view.item("img-1.png")?.click());
    expect(view.onSelect).toHaveBeenCalledWith({ index: 1, entry: image(1) });
    await act(async () => {
        view.item("img-1.png")?.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
    });
    expect(view.onFocus).toHaveBeenCalledWith({ index: 1, entry: image(1) });
});

it("shows a video as a thumbnail with a mark and no player", async () => {
    const view = await render(listingOf([video]));
    expect(view.host.querySelector("video")).toBeNull();
    expect(view.item("clip.mp4")?.querySelector("[role='img']")).not.toBeNull();
});

it("shows a placeholder at an unread position and asks for its page", async () => {
    const view = await render(listingOf([image(0), undefined]));
    expect(view.host.querySelector("[data-placeholder]")).not.toBeNull();
    expect(view.onPages).toHaveBeenCalledWith(new Set([0]));
});

it("keeps the layout to a window in a long listing, and a far move brings the window along", async () => {
    const count = 1_000_000;
    const listing = { ...listingOf([]), totalCount: count, entryAt: (index: number) => image(index) };
    const view = await render(listing, { index: 500_000 });
    const width = () => Number.parseFloat((view.strip().firstElementChild as HTMLElement).style.width);
    expect(width()).toBeLessThan(501 * (ITEM_HEIGHT + 24) + AREA.width + 1);
    await view.rerender(900_000);
    expect(view.item("img-900000.png")?.getAttribute("aria-current")).toBe("true");
    expect(width()).toBeLessThan(501 * (ITEM_HEIGHT + 24) + AREA.width + 1);
});

it("moves the window along with scrolling and keeps the place at the center", async () => {
    const listing = { ...listingOf([]), totalCount: 10_000, entryAt: (index: number) => image(index) };
    const view = await render(listing, { index: 5_000 });
    // Scrolling by 210 positions brings the display area near the end of the window.
    const distance = 210 * (ITEM_HEIGHT + 24);
    await act(async () => {
        view.strip().dispatchEvent(new WheelEvent("wheel", { deltaY: distance, bubbles: true, cancelable: true }));
    });
    const box = view.box("img-5210.png");
    expect(box).toBeDefined();
    const center = view.strip().scrollLeft + AREA.width / 2;
    expect(center - Number.parseFloat(box.style.left)).toBeCloseTo(ITEM_HEIGHT / 2);
    // The window moved, so positions behind the old window's end are now laid out.
    expect(Number.parseFloat((view.strip().firstElementChild as HTMLElement).style.width)).toBeGreaterThan(center);
    await act(async () => {
        vi.advanceTimersByTime(150);
    });
    expect(view.onSelect).toHaveBeenCalledWith({ index: 5_210, entry: image(5_210) });
});

it("waits for the page of an unread position before choosing the open Entry", async () => {
    const entries: (Entry | undefined)[] = [image(0), undefined, image(2)];
    const listing: Listing = { ...listingOf(entries), version: 1 };
    const view = await render(listing);
    await act(async () => {
        view.strip().dispatchEvent(
            new WheelEvent("wheel", { deltaY: ITEM_HEIGHT + 24, bubbles: true, cancelable: true }),
        );
    });
    await act(async () => {
        vi.advanceTimersByTime(150);
    });
    // Position 0 is loaded and near, but the place at the center is the unread position.
    expect(view.onSelect).not.toHaveBeenCalled();
    entries[1] = image(1);
    await view.rerenderListing({ ...listing, version: 2 });
    expect(view.onSelect).toHaveBeenCalledExactlyOnceWith({ index: 1, entry: image(1) });
});
