// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, type ReactNode } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { Entry, FolderListing } from "../library/entries";
import { stubMatchMedia, stubResizeObserver } from "../testing/preferences";
import { Viewer } from "./Viewer";

const { get, range, remove, rate } = vi.hoisted(() => ({
    get: vi.fn(),
    range: { first: 0, last: 2 },
    remove: vi.fn(),
    rate: vi.fn(),
}));
vi.mock("../library/ratingTags", async (original) => ({
    ...(await original<typeof import("../library/ratingTags")>()),
    putRating: rate,
}));
vi.mock("../library/entryActions", async (original) => ({
    ...(await original<typeof import("../library/entryActions")>()),
    deleteEntries: remove,
}));
vi.mock("@tanstack/react-virtual", () => ({
    useVirtualizer: (options: { count: number; estimateSize: () => number }) => ({
        range: { startIndex: range.first, endIndex: range.last },
        scrollToIndex: () => {},
        getTotalSize: () => options.count * options.estimateSize(),
        getVirtualItems: () =>
            Array.from({ length: range.last - range.first + 1 }, (_, offset) => ({
                index: range.first + offset,
                key: range.first + offset,
                start: (range.first + offset) * options.estimateSize(),
                size: options.estimateSize(),
            })),
    }),
}));
vi.mock("../thumbnails/api", async (original) => ({
    ...(await original<typeof import("../thumbnails/api")>()),
    fetchThumbnail: () => new Promise(() => {}),
}));
vi.mock("../api/client", async (original) => ({
    ...(await original<typeof import("../api/client")>()),
    api: {
        GET: (route: string, options?: unknown) =>
            route === "/capabilities"
                ? Promise.resolve({
                      data: { summaryListing: false, entryDetails: false, childProbes: false },
                      response: new Response(),
                  })
                : get(route, options),
    },
}));
// 動画の再生は VideoPlayer のテストが受け持つ。ここでは、終わったと知らせる onDone だけを確かめる。
vi.mock("./VideoPlayer", () => ({
    VideoPlayer: ({ onDone }: { onDone?: () => void }) => (
        // biome-ignore lint/a11y/useMediaCaption: テスト用の代役で、字幕を扱わない
        <video aria-label="動画プレイヤー" onEnded={() => onDone?.()} />
    ),
}));
vi.mock("./Modal", () => ({
    Modal: ({
        children,
        outerRef,
        onClose,
    }: {
        children: ReactNode;
        outerRef?: (element: HTMLDivElement | null) => void;
        onClose: () => void;
    }) => (
        <dialog
            open
            onCancel={(event) => {
                event.preventDefault();
                onClose();
            }}
        >
            <div ref={outerRef}>{children}</div>
        </dialog>
    ),
}));
afterEach(() => {
    get.mockReset();
    remove.mockReset();
    rate.mockReset();
    range.first = 0;
    range.last = 2;
});
const entry = (index: number): Entry => ({
    name: `${index}.png`,
    path: `/${index}.png`,
    kind: "file",
    metadata: { size: 1, readonly: false, contentType: "image/png" },
    file: { archive: false },
});
const response = (entries: Entry[]) => ({
    data: { entries, totalCount: 201, snapshot: "one", truncated: false } satisfies FolderListing,
    response: new Response(),
});

async function harness(
    initialView = "199.png",
    path = "/",
    originalImage: "auto" | "onZoom" = "onZoom",
    container?: { width: number; height: number },
    narrow = true,
    libraryReadonly = false,
) {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    const restoreObserver = stubResizeObserver();
    // ビューアの中の `Toaster` が、配色の判定に使う。
    const restoreMedia = stubMatchMedia(narrow);
    if (container) {
        globalThis.ResizeObserver = class {
            constructor(private readonly callback: ResizeObserverCallback) {}
            observe() {
                this.callback([{ contentRect: container } as ResizeObserverEntry], this as unknown as ResizeObserver);
            }
            unobserve() {}
            disconnect() {}
        } as unknown as typeof ResizeObserver;
    }
    const client = new QueryClient();
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    const onChange = vi.fn();
    const render = async (view: string, nextNarrow = narrow) => {
        await act(async () =>
            root.render(
                <QueryClientProvider client={client}>
                    <Viewer
                        location={{ libraryId: "big", path, view }}
                        source={{ kind: "folder", libraryId: "big", path }}
                        sort={{ key: "name", order: "asc" }}
                        originalImage={originalImage}
                        narrow={nextNarrow}
                        libraryReadonly={libraryReadonly}
                        onChange={onChange}
                    />
                </QueryClientProvider>,
            ),
        );
    };
    await render(initialView);
    return {
        host,
        onChange,
        render,
        dispose: async () => {
            await act(async () => root.unmount());
            client.clear();
            host.remove();
            restoreObserver();
            restoreMedia();
        },
    };
}
async function wait(assertion: () => void) {
    await vi.waitFor(async () => {
        await act(async () => {
            await Promise.resolve();
        });
        assertion();
    });
}
it("keeps the current image while reading and moves after one arrow press", async () => {
    let resolve: ((value: ReturnType<typeof response>) => void) | undefined;
    get.mockImplementation((_, options) =>
        options.params.query.offset === 0
            ? Promise.resolve(response(Array.from({ length: 200 }, (_, index) => entry(index))))
            : new Promise((done) => {
                  resolve = done;
              }),
    );
    const h = await harness();
    try {
        await wait(() => expect(h.host.querySelector("img")?.alt).toBe("199.png"));
        await act(async () => window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight" })));
        await wait(() => expect(resolve).toBeDefined());
        expect(h.host.querySelector("img")?.alt).toBe("199.png");
        expect(h.host.querySelector("[role=status]")?.textContent).toBe("読み込み中…");
        expect(h.onChange).not.toHaveBeenCalled();
        await act(async () => {
            resolve!(response([entry(200)]));
        });
        await wait(() => expect(h.onChange).toHaveBeenCalledExactlyOnceWith("200.png"));
        await h.render("200.png");
        await wait(() => expect(h.host.querySelector("img")?.alt).toBe("200.png"));
    } finally {
        await h.dispose();
    }
});
it("cancels pending navigation when the viewer closes", async () => {
    get.mockImplementation((_, options) =>
        options.params.query.offset === 0
            ? Promise.resolve(response(Array.from({ length: 200 }, (_, index) => entry(index))))
            : new Promise(() => {}),
    );
    const h = await harness();
    await wait(() => expect(h.host.querySelector("img")?.alt).toBe("199.png"));
    await act(async () => window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight" })));
    await wait(() => expect(get.mock.calls.some(([, options]) => options.params.query.offset === 200)).toBe(true));
    const signal = get.mock.calls.find(([, options]) => options.params.query.offset === 200)![1].signal as AbortSignal;
    await h.dispose();
    expect(signal.aborted).toBe(true);
    expect(h.onChange).not.toHaveBeenCalled();
});

it("同名の画像を相対パスで開き、前後への移動でも区別する", async () => {
    const entries = ["a", "b"].map((folder) => ({ ...entry(0), name: "same.png", path: `/pair/${folder}/same.png` }));
    get.mockResolvedValue({
        data: { entries, totalCount: 2, snapshot: "pair", truncated: false } satisfies FolderListing,
        response: new Response(),
    });
    const h = await harness("b/same.png", "/pair");
    try {
        await wait(() => expect(h.host.querySelector("img")?.getAttribute("src")).toContain("%2Fpair%2Fb%2Fsame.png"));
        await act(async () => window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowLeft" })));
        await wait(() => expect(h.onChange).toHaveBeenCalledExactlyOnceWith("a/same.png"));
    } finally {
        await h.dispose();
    }
});

const image = (name: string, contentType: string, size: number): import("../library/entries").DetailedEntry => ({
    name,
    path: `/${name}`,
    kind: "file",
    metadata: { size, readonly: false, contentType },
    file: { archive: false },
});
const listed = (entries: Entry[]) =>
    get.mockResolvedValue({
        data: { entries, totalCount: entries.length, snapshot: "one", truncated: false } satisfies FolderListing,
        response: new Response(),
    });
const findDisplayImage = (h: { host: HTMLElement }) =>
    h.host.querySelector<HTMLImageElement>("img:not([data-original])");
const displayImage = (h: { host: HTMLElement }) => {
    const element = findDisplayImage(h);
    if (!element) throw new Error("表示用画像がない");
    return element;
};
const originalImage = (h: { host: HTMLElement }) => h.host.querySelector<HTMLImageElement>("img[data-original]");
/// jsdom は画像を読まないので、表示用画像の `load` を自分で起こす。
const loadDisplay = async (h: { host: HTMLElement }, naturalWidth = 0) => {
    const element = displayImage(h);
    Object.defineProperty(element, "naturalWidth", { configurable: true, value: naturalWidth });
    Object.defineProperty(element, "naturalHeight", { configurable: true, value: naturalWidth });
    await act(async () => element.dispatchEvent(new Event("load")));
};
const zoomIn = (h: { host: HTMLElement }) =>
    act(async () => {
        h.host
            .querySelector(".select-none")
            ?.dispatchEvent(new WheelEvent("wheel", { deltaY: -500, cancelable: true }));
    });

describe("元の画像への差し替え", () => {
    it("「自動」で 30 MB 以下の画像は、表示用画像が出た後に拡大しなくても差し替える", async () => {
        listed([image("a.png", "image/png", 30_000_000)]);
        const h = await harness("a.png", "/", "auto");
        try {
            await wait(() => expect(findDisplayImage(h)).not.toBeNull());
            expect(originalImage(h)).toBeNull();
            await loadDisplay(h);
            expect(originalImage(h)?.getAttribute("src")).toContain("files/content");
            expect(originalImage(h)?.getAttribute("src")).toContain("a.png");
        } finally {
            await h.dispose();
        }
    });

    it("「自動」で 30 MB を超える画像と、「拡大したときだけ」の画像は、拡大するまで差し替えない", async () => {
        for (const [name, size, mode] of [
            ["big.png", 30_000_001, "auto"],
            ["small.png", 1, "onZoom"],
        ] as const) {
            listed([image(name, "image/png", size)]);
            const h = await harness(name, "/", mode, { width: 800, height: 600 });
            try {
                await wait(() => expect(findDisplayImage(h)).not.toBeNull());
                await loadDisplay(h, 400);
                expect(originalImage(h)).toBeNull();
                await zoomIn(h);
                expect(originalImage(h)).not.toBeNull();
            } finally {
                await h.dispose();
            }
        }
    });

    it("ブラウザで表示できない形式は、どの設定でも、拡大しても差し替えない", async () => {
        for (const mode of ["auto", "onZoom"] as const) {
            listed([image("scan.tiff", "image/tiff", 1)]);
            const h = await harness("scan.tiff", "/", mode, { width: 800, height: 600 });
            try {
                await wait(() => expect(findDisplayImage(h)).not.toBeNull());
                await loadDisplay(h, 400);
                await zoomIn(h);
                expect(originalImage(h)).toBeNull();
            } finally {
                await h.dispose();
            }
        }
    });

    it("差し替える画像の表示用画像は画素密度を掛けずに要求し、先読みの URL は移った先の要求と一致する", async () => {
        Object.defineProperty(window, "devicePixelRatio", { configurable: true, value: 2 });
        const prefetched: string[] = [];
        const source = Object.getOwnPropertyDescriptor(HTMLImageElement.prototype, "src");
        const getSource = source?.get;
        const setSource = source?.set;
        if (!source || !getSource || !setSource) throw new Error("HTMLImageElement の src が見つからない");
        Object.defineProperty(HTMLImageElement.prototype, "src", {
            configurable: true,
            get() {
                return getSource?.call(this);
            },
            set(value: string) {
                prefetched.push(value);
                setSource?.call(this, value);
            },
        });
        try {
            listed([image("a.png", "image/png", 1), image("b.png", "image/png", 30_000_001)]);
            const h = await harness("a.png", "/", "auto");
            try {
                await wait(() => expect(findDisplayImage(h)).not.toBeNull());
                expect(displayImage(h).getAttribute("src")).toContain("width=1024&height=768");
                await wait(() => expect(prefetched.some((url) => url.includes("b.png"))).toBe(true));
                await h.render("b.png");
                await wait(() => expect(displayImage(h).getAttribute("src")).toContain("b.png"));
                expect(displayImage(h).getAttribute("src")).toContain("width=2048&height=1536");
                expect(prefetched).toContain(displayImage(h).getAttribute("src"));
            } finally {
                await h.dispose();
            }
        } finally {
            Object.defineProperty(HTMLImageElement.prototype, "src", source);
            Reflect.deleteProperty(window, "devicePixelRatio");
        }
    });
});

describe("フィルムストリップ", () => {
    const page = (offset: number) => Array.from({ length: 200 }, (_, index) => entry(offset + index));
    const thousand = () =>
        get.mockImplementation((_, options) =>
            Promise.resolve({
                data: {
                    entries: page(options.params.query.offset),
                    totalCount: 1000,
                    snapshot: "one",
                    truncated: false,
                } satisfies FolderListing,
                response: new Response(),
            }),
        );
    const offsets = () => get.mock.calls.map(([, options]) => options.params.query.offset as number);
    const strip = (h: { host: HTMLElement }) => h.host.querySelector("[aria-label=フィルムストリップ]");

    it("広い画面の画像と動画のビューアに出し、狭い画面には出さない", async () => {
        thousand();
        const wide = await harness("5.png", "/", "onZoom", undefined, false);
        try {
            await wait(() => expect(strip(wide)).not.toBeNull());
        } finally {
            await wide.dispose();
        }
        const narrow = await harness("5.png", "/", "onZoom", undefined, true);
        try {
            await wait(() => expect(narrow.host.querySelector("img")?.alt).toBe("5.png"));
            expect(strip(narrow)).toBeNull();
        } finally {
            await narrow.dispose();
        }
        get.mockReset();
        get.mockResolvedValue({
            data: {
                entries: [
                    {
                        ...entry(1),
                        name: "clip.mp4",
                        path: "/clip.mp4",
                        metadata: { size: 1, readonly: false, contentType: "video/mp4" },
                    },
                ],
                totalCount: 1,
                snapshot: "one",
                truncated: false,
            } satisfies FolderListing,
            response: new Response(),
        });
        const video = await harness("clip.mp4", "/", "onZoom", undefined, false);
        try {
            await wait(() => expect(video.host.querySelector("video")).not.toBeNull());
            expect(strip(video)).not.toBeNull();
        } finally {
            await video.dispose();
        }
    });

    it("前後へ移っても作り直さず、探し直す間も読み込み中を挟まない", async () => {
        thousand();
        range.first = 0;
        range.last = 22;
        const h = await harness("5.png", "/", "onZoom", undefined, false);
        try {
            await wait(() => expect(h.host.querySelector("img")?.alt).toBe("5.png"));
            const before = strip(h);
            await act(async () => window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight" })));
            await wait(() => expect(h.onChange).toHaveBeenCalledExactlyOnceWith("6.png"));
            await h.render("6.png");
            expect(h.host.querySelector("img")?.alt).toBe("6.png");
            expect(h.host.textContent).not.toContain("読み込み中…");
            expect(strip(h)).toBe(before);
            expect(
                h.host.querySelector("[aria-label=フィルムストリップ] [aria-current=true]")?.getAttribute("aria-label"),
            ).toBe("6.png");
        } finally {
            await h.dispose();
        }
    });

    it("開いた直後は先頭のページだけを読み、最後の範囲へスクロールすると最後のページだけを読む", async () => {
        thousand();
        range.first = 0;
        range.last = 22;
        const h = await harness("5.png", "/", "onZoom", undefined, false);
        try {
            await wait(() => expect(strip(h)).not.toBeNull());
            await act(async () => {});
            expect(new Set(offsets())).toEqual(new Set([0]));
            range.first = 909;
            range.last = 931;
            await h.render("5.png");
            await wait(() => expect(offsets()).toContain(800));
            expect(new Set(offsets())).toEqual(new Set([0, 800]));
        } finally {
            await h.dispose();
        }
    });

    it("フィルムストリップの画像を押すと、その画像の相対パスで移動し、探し直さずに開く", async () => {
        thousand();
        range.first = 0;
        range.last = 22;
        const h = await harness("5.png", "/", "onZoom", undefined, false);
        try {
            await wait(() =>
                expect(h.host.querySelector("[aria-label=フィルムストリップ] [aria-label='9.png']")).not.toBeNull(),
            );
            await act(async () =>
                h.host.querySelector<HTMLElement>("[aria-label=フィルムストリップ] [aria-label='9.png']")?.click(),
            );
            expect(h.onChange).toHaveBeenCalledExactlyOnceWith("9.png");
            await h.render("9.png");
            expect(h.host.querySelector("img")?.alt).toBe("9.png");
        } finally {
            await h.dispose();
        }
    });
});

describe("スライドショー", () => {
    const wideViewer = (view: string) => harness(view, "/", "onZoom", undefined, false);
    const button = (h: { host: HTMLElement }) =>
        h.host.querySelector<HTMLButtonElement>("button[aria-label^=スライドショー]");
    const label = (h: { host: HTMLElement }) => button(h)?.getAttribute("aria-label");
    /// モックの一覧は応答済みなので、移動の探索は、タイマーに頼らずマイクロタスクだけで終わる。
    const settle = async () => {
        for (let count = 0; count < 5; count += 1) await act(async () => Promise.resolve());
    };
    const elapse = (ms: number) => act(async () => vi.advanceTimersByTimeAsync(ms));
    afterEach(() => vi.useRealTimers());

    it("始めると、表示用画像が出てから 5 秒ごとに次へ進み、最後の画像で止まる", async () => {
        listed([0, 1, 2].map((index) => image(`${index}.png`, "image/png", 1)));
        const h = await wideViewer("0.png");
        try {
            await wait(() => expect(findDisplayImage(h)).not.toBeNull());
            vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
            expect(label(h)).toBe("スライドショー");
            await act(async () => button(h)?.click());
            expect(label(h)).toBe("スライドショーを止める");

            await elapse(20_000);
            expect(h.onChange).not.toHaveBeenCalled();
            await loadDisplay(h);
            await elapse(4999);
            expect(h.onChange).not.toHaveBeenCalled();
            await elapse(1);
            expect(h.onChange).toHaveBeenCalledExactlyOnceWith("1.png");

            await h.render("1.png");
            await settle();
            await loadDisplay(h);
            await elapse(5000);
            expect(h.onChange).toHaveBeenLastCalledWith("2.png");

            await h.render("2.png");
            await settle();
            await loadDisplay(h);
            await elapse(5000);
            expect(h.onChange).toHaveBeenCalledTimes(2);
            expect(label(h)).toBe("スライドショー");
            await elapse(20_000);
            expect(h.onChange).toHaveBeenCalledTimes(2);
        } finally {
            await h.dispose();
        }
    });

    it("動画に着いたら 5 秒では進まず、再生が終わると次へ進み、最後の動画の後で止まる", async () => {
        listed([image("0.png", "image/png", 1), image("1.mp4", "video/mp4", 1), image("2.mp4", "video/mp4", 1)]);
        const h = await wideViewer("0.png");
        try {
            await wait(() => expect(findDisplayImage(h)).not.toBeNull());
            vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
            await act(async () => button(h)?.click());
            await loadDisplay(h);
            await elapse(5000);
            expect(h.onChange).toHaveBeenCalledExactlyOnceWith("1.mp4");

            await h.render("1.mp4");
            await settle();
            expect(h.host.querySelector("video")).not.toBeNull();
            expect(label(h)).toBe("スライドショーを止める");
            await elapse(60_000);
            expect(h.onChange).toHaveBeenCalledTimes(1);
            await act(async () => h.host.querySelector("video")!.dispatchEvent(new Event("ended")));
            await settle();
            expect(h.onChange).toHaveBeenLastCalledWith("2.mp4");

            await h.render("2.mp4");
            await settle();
            await act(async () => h.host.querySelector("video")!.dispatchEvent(new Event("ended")));
            await settle();
            expect(h.onChange).toHaveBeenCalledTimes(2);
            expect(button(h)).toBeNull();
        } finally {
            await h.dispose();
        }
    });

    it("途中で矢印キーで移ると、移った先の画像から 5 秒を数え直す", async () => {
        listed([0, 1, 2].map((index) => image(`${index}.png`, "image/png", 1)));
        const h = await wideViewer("0.png");
        try {
            await wait(() => expect(findDisplayImage(h)).not.toBeNull());
            vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
            await act(async () => button(h)?.click());
            await loadDisplay(h);
            await elapse(3000);
            await act(async () => window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight" })));
            await settle();
            expect(h.onChange).toHaveBeenCalledExactlyOnceWith("1.png");
            await h.render("1.png");
            await settle();
            await elapse(10_000);
            expect(h.onChange).toHaveBeenCalledTimes(1);
            await loadDisplay(h);
            await elapse(4999);
            expect(h.onChange).toHaveBeenCalledTimes(1);
            await elapse(1);
            expect(h.onChange).toHaveBeenLastCalledWith("2.png");
        } finally {
            await h.dispose();
        }
    });

    it("止めた後と、ビューアを閉じた後は進まない", async () => {
        listed([0, 1].map((index) => image(`${index}.png`, "image/png", 1)));
        const stopped = await wideViewer("0.png");
        try {
            await wait(() => expect(findDisplayImage(stopped)).not.toBeNull());
            vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
            await act(async () => button(stopped)?.click());
            await loadDisplay(stopped);
            await elapse(3000);
            await act(async () => button(stopped)?.click());
            expect(label(stopped)).toBe("スライドショー");
            await elapse(20_000);
            expect(stopped.onChange).not.toHaveBeenCalled();
        } finally {
            await stopped.dispose();
        }
        vi.useRealTimers();
        const closed = await wideViewer("0.png");
        try {
            await wait(() => expect(findDisplayImage(closed)).not.toBeNull());
            vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
            await act(async () => button(closed)?.click());
            await loadDisplay(closed);
            await elapse(3000);
            await closed.dispose();
            await elapse(20_000);
            expect(closed.onChange).not.toHaveBeenCalled();
        } finally {
            vi.useRealTimers();
        }
    });

    it("次の範囲を読み込んでいる間に止めると、読み終えても移らず、移動も続けられる", async () => {
        let resolve: ((value: ReturnType<typeof response>) => void) | undefined;
        get.mockImplementation((_, options) =>
            options.params.query.offset === 0
                ? Promise.resolve(response(Array.from({ length: 200 }, (_, index) => entry(index))))
                : new Promise((done) => {
                      resolve = done;
                  }),
        );
        const h = await wideViewer("199.png");
        try {
            await wait(() => expect(findDisplayImage(h)).not.toBeNull());
            vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
            await act(async () => button(h)?.click());
            await loadDisplay(h);
            await elapse(5000);
            await settle();
            expect(resolve).toBeDefined();
            expect(h.onChange).not.toHaveBeenCalled();
            await act(async () => button(h)?.click());
            expect(label(h)).toBe("スライドショー");
            await act(async () => {
                resolve?.(response([entry(200)]));
            });
            await settle();
            await elapse(20_000);
            expect(h.onChange).not.toHaveBeenCalled();
            expect(h.host.querySelector<HTMLButtonElement>("button[aria-label=次へ]")?.disabled).toBe(false);
        } finally {
            await h.dispose();
        }
    });

    it("動画のビューアには出さない", async () => {
        listed([{ ...image("clip.mp4", "video/mp4", 1) }]);
        const h = await wideViewer("clip.mp4");
        try {
            await wait(() => expect(h.host.querySelector("video")).not.toBeNull());
            expect(button(h)).toBeNull();
        } finally {
            await h.dispose();
        }
    });
});

describe("全画面表示と「その他」のメニュー", () => {
    let fullscreenElement: Element | null = null;
    const exitFullscreen = vi.fn(async () => {
        fullscreenElement = null;
        document.dispatchEvent(new Event("fullscreenchange"));
    });
    const requestFullscreen = vi.fn(async function (this: HTMLElement) {
        fullscreenElement = this;
        document.dispatchEvent(new Event("fullscreenchange"));
    });
    const stubFullscreen = (enabled: boolean) => {
        Object.defineProperty(document, "fullscreenEnabled", { configurable: true, value: enabled });
        Object.defineProperty(document, "fullscreenElement", { configurable: true, get: () => fullscreenElement });
        Object.defineProperty(document, "exitFullscreen", { configurable: true, value: exitFullscreen });
        Object.defineProperty(HTMLElement.prototype, "requestFullscreen", {
            configurable: true,
            value: requestFullscreen,
        });
    };
    beforeEach(() => {
        fullscreenElement = null;
        exitFullscreen.mockClear();
        requestFullscreen.mockClear();
        stubFullscreen(true);
    });
    afterEach(() => {
        for (const key of ["fullscreenEnabled", "fullscreenElement", "exitFullscreen"]) {
            Reflect.deleteProperty(document, key);
        }
        Reflect.deleteProperty(HTMLElement.prototype, "requestFullscreen");
    });

    const trio = () => listed([0, 1, 2].map((index) => image(`${index}.png`, "image/png", 1)));
    const open = async (narrow: boolean) => {
        trio();
        const h = await harness("1.png", "/", "onZoom", undefined, narrow);
        await wait(() => expect(findDisplayImage(h)).not.toBeNull());
        return h;
    };
    const byLabel = (h: { host: HTMLElement }, label: string) =>
        h.host.querySelector<HTMLButtonElement>(`button[aria-label="${label}"]`);
    const outer = (h: { host: HTMLElement }) => h.host.querySelector("dialog > div");
    const key = (target: EventTarget, name: string) =>
        act(async () =>
            target.dispatchEvent(new KeyboardEvent("keydown", { key: name, bubbles: true, cancelable: true })),
        );
    const pointerOpen = (element: HTMLElement | null) =>
        act(async () => element?.dispatchEvent(new MouseEvent("pointerdown", { bubbles: true, button: 0 })));
    const menuItems = (h: { host: HTMLElement }) =>
        [...h.host.querySelectorAll("[role=menuitem]")].map((item) => item.textContent);

    it("広い画面のボタンは外枠の要素を全画面にし、状態に合わせて表示が変わり、もう一度で戻る", async () => {
        const h = await open(false);
        try {
            expect(byLabel(h, "全画面表示")).not.toBeNull();
            await act(async () => byLabel(h, "全画面表示")?.click());
            expect(requestFullscreen).toHaveBeenCalledOnce();
            expect(requestFullscreen.mock.contexts[0]).toBe(outer(h));
            expect(byLabel(h, "全画面表示を終える")).not.toBeNull();
            await act(async () => byLabel(h, "全画面表示を終える")?.click());
            expect(exitFullscreen).toHaveBeenCalledOnce();
            expect(byLabel(h, "全画面表示")).not.toBeNull();
        } finally {
            await h.dispose();
        }
    });

    it("全画面の間の Esc は全画面だけを終え、ビューアを閉じない。全画面でなければ閉じる", async () => {
        const h = await open(false);
        try {
            await act(async () => byLabel(h, "全画面表示")?.click());
            await key(window, "Escape");
            expect(exitFullscreen).toHaveBeenCalledOnce();
            expect(h.onChange).not.toHaveBeenCalled();
            await key(window, "Escape");
            expect(h.onChange).toHaveBeenCalledExactlyOnceWith(undefined);
        } finally {
            await h.dispose();
        }
    });

    it("全画面の間にダイアログの cancel が届いても、全画面だけを終える", async () => {
        const h = await open(false);
        try {
            await act(async () => byLabel(h, "全画面表示")?.click());
            const event = new Event("cancel", { cancelable: true });
            await act(async () => h.host.querySelector("dialog")?.dispatchEvent(event));
            expect(exitFullscreen).toHaveBeenCalledOnce();
            expect(h.onChange).not.toHaveBeenCalled();
        } finally {
            await h.dispose();
        }
    });

    it("全画面のままビューアを閉じると、全画面を終える", async () => {
        const h = await open(false);
        await act(async () => byLabel(h, "全画面表示")?.click());
        expect(exitFullscreen).not.toHaveBeenCalled();
        await h.dispose();
        expect(exitFullscreen).toHaveBeenCalledOnce();
    });

    it("Fullscreen API を使えないブラウザでは、広い画面のボタンも、狭い画面のメニューの項目も出さない", async () => {
        stubFullscreen(false);
        const wide = await open(false);
        try {
            expect(byLabel(wide, "全画面表示")).toBeNull();
            expect(byLabel(wide, "スライドショー")).not.toBeNull();
        } finally {
            await wide.dispose();
        }
        const narrow = await open(true);
        try {
            await pointerOpen(byLabel(narrow, "その他"));
            expect(menuItems(narrow)).toEqual(["スライドショー"]);
        } finally {
            await narrow.dispose();
        }
    });

    it("狭い画面は「その他」のメニューにスライドショーと全画面表示を置き、選ぶとボタンと同じ動作になる", async () => {
        const h = await open(true);
        try {
            expect(byLabel(h, "スライドショー")).toBeNull();
            expect(byLabel(h, "全画面表示")).toBeNull();
            await pointerOpen(byLabel(h, "その他"));
            expect(menuItems(h)).toEqual(["スライドショー", "全画面表示"]);
            // メニューは、全画面にする外枠の要素の中に描く。
            expect(outer(h)?.contains(h.host.querySelector("[role=menu]"))).toBe(true);
            await act(async () => h.host.querySelectorAll<HTMLElement>("[role=menuitem]")[1]?.click());
            expect(requestFullscreen).toHaveBeenCalledOnce();
            expect(requestFullscreen.mock.contexts[0]).toBe(outer(h));
            await pointerOpen(byLabel(h, "その他"));
            expect(menuItems(h)).toEqual(["スライドショー", "全画面表示を終える"]);
            await act(async () => h.host.querySelectorAll<HTMLElement>("[role=menuitem]")[0]?.click());
            await pointerOpen(byLabel(h, "その他"));
            expect(menuItems(h)).toEqual(["スライドショーを止める", "全画面表示を終える"]);
        } finally {
            await h.dispose();
        }
    });

    it("メニューが開いている間は、矢印キーで移らず、Esc はメニューだけを閉じる", async () => {
        const h = await open(true);
        try {
            await pointerOpen(byLabel(h, "その他"));
            expect(h.host.querySelector("[role=menu]")).not.toBeNull();
            await key(window, "ArrowRight");
            await key(window, "ArrowLeft");
            expect(h.onChange).not.toHaveBeenCalled();
            const menu = h.host.querySelector("[role=menu]");
            if (!menu) throw new Error("メニューがない");
            await key(menu, "Escape");
            expect(h.host.querySelector("[role=menu]")).toBeNull();
            expect(h.onChange).not.toHaveBeenCalled();
            // 閉じた後は、矢印キーでも Esc でも、ふつうに動く。
            await key(window, "ArrowRight");
            await wait(() => expect(h.onChange).toHaveBeenCalledExactlyOnceWith("2.png"));
        } finally {
            await h.dispose();
        }
    });

    it("動画のビューアには、どちらの入口も出さない", async () => {
        listed([image("clip.mp4", "video/mp4", 1)]);
        const wide = await harness("clip.mp4", "/", "onZoom", undefined, false);
        const narrow = await harness("clip.mp4", "/", "onZoom", undefined, true);
        try {
            await wait(() => expect(wide.host.querySelector("video")).not.toBeNull());
            await wait(() => expect(narrow.host.querySelector("video")).not.toBeNull());
            expect(byLabel(wide, "全画面表示")).toBeNull();
            expect(byLabel(narrow, "その他")).toBeNull();
        } finally {
            await wide.dispose();
            await narrow.dispose();
        }
    });
});

describe("動画との行き来", () => {
    const mixed = () =>
        listed([image("0.png", "image/png", 1), image("1.mp4", "video/mp4", 1), image("2.png", "image/png", 1)]);
    const key = (target: EventTarget, name: string) =>
        act(async () =>
            target.dispatchEvent(new KeyboardEvent("keydown", { key: name, bubbles: true, cancelable: true })),
        );

    it("矢印キーとボタンで、画像から動画へ、動画から画像へ移る", async () => {
        mixed();
        const h = await harness("0.png", "/", "onZoom", undefined, false);
        try {
            await wait(() => expect(findDisplayImage(h)).not.toBeNull());
            await key(window, "ArrowRight");
            await wait(() => expect(h.onChange).toHaveBeenLastCalledWith("1.mp4"));
            await h.render("1.mp4");
            await wait(() => expect(h.host.querySelector("video")).not.toBeNull());
            await key(window, "ArrowRight");
            await wait(() => expect(h.onChange).toHaveBeenLastCalledWith("2.png"));
            await h.render("1.mp4");
            await act(async () => h.host.querySelector<HTMLButtonElement>("button[aria-label=前へ]")?.click());
            await wait(() => expect(h.onChange).toHaveBeenLastCalledWith("0.png"));
        } finally {
            await h.dispose();
        }
    });

    it("動画のプレーヤーにフォーカスがあるときの矢印キーでは移らない", async () => {
        mixed();
        const h = await harness("1.mp4", "/", "onZoom", undefined, false);
        try {
            await wait(() => expect(h.host.querySelector("video")).not.toBeNull());
            const video = h.host.querySelector("video");
            if (!video) throw new Error("動画がない");
            await key(video, "ArrowRight");
            await act(async () => Promise.resolve());
            expect(h.onChange).not.toHaveBeenCalled();
        } finally {
            await h.dispose();
        }
    });
});

describe("ダウンロードと削除", () => {
    const trio = (overrides: Partial<Entry["metadata"]> = {}) =>
        listed(
            [0, 1, 2].map((index) => {
                const value = image(`${index}.png`, "image/png", 1);
                return { ...value, metadata: { ...value.metadata, ...overrides } };
            }),
        );
    const open = async (view: string, { readonly = false, narrow = false } = {}) => {
        const h = await harness(view, "/", "onZoom", undefined, narrow, readonly);
        await wait(() => expect(findDisplayImage(h)).not.toBeNull());
        return h;
    };
    const control = (h: { host: HTMLElement }, label: string) =>
        h.host.querySelector<HTMLButtonElement & HTMLAnchorElement>(`[aria-label="${label}"]`);
    const outer = (h: { host: HTMLElement }) => h.host.querySelector("dialog > div");
    const confirmation = (h: { host: HTMLElement }) => h.host.querySelector("[role=alertdialog]");
    const press = async (text: string) =>
        act(async () =>
            [...document.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === text)?.click(),
        );
    const key = (target: EventTarget, name: string) =>
        act(async () =>
            target.dispatchEvent(new KeyboardEvent("keydown", { key: name, bubbles: true, cancelable: true })),
        );
    const askDeletion = async (h: { host: HTMLElement }) => {
        await act(async () => control(h, "削除")?.click());
        await wait(() => expect(confirmation(h)).not.toBeNull());
    };

    it("ダウンロードは、開いている画像の content API の URL を、画像の名前で保存するリンクにする。読み取り専用の Library でも出る", async () => {
        trio();
        for (const readonly of [false, true]) {
            const h = await open("1.png", { readonly });
            try {
                const link = control(h, "ダウンロード");
                expect(link?.tagName).toBe("A");
                expect(link?.getAttribute("href")).toContain("/files/content?path=%2F1.png");
                expect(link?.getAttribute("download")).toBe("1.png");
            } finally {
                await h.dispose();
            }
        }
    });

    it("削除は、読み取り専用の Library と、読み取り専用の Entry では選べない", async () => {
        trio();
        const writable = await open("1.png");
        try {
            expect(control(writable, "削除")?.disabled).toBe(false);
        } finally {
            await writable.dispose();
        }
        const locked = await open("1.png", { readonly: true });
        try {
            expect(control(locked, "削除")?.disabled).toBe(true);
        } finally {
            await locked.dispose();
        }
        trio({ readonly: true });
        const entryLocked = await open("1.png");
        try {
            expect(control(entryLocked, "削除")?.disabled).toBe(true);
        } finally {
            await entryLocked.dispose();
        }
    });

    it("狭い画面と動画のビューアには、2 つのボタンを出さない", async () => {
        trio();
        const narrow = await open("1.png", { narrow: true });
        try {
            expect(control(narrow, "ダウンロード")).toBeNull();
            expect(control(narrow, "削除")).toBeNull();
        } finally {
            await narrow.dispose();
        }
        listed([image("clip.mp4", "video/mp4", 1)]);
        const video = await harness("clip.mp4", "/", "onZoom", undefined, false);
        try {
            await wait(() => expect(video.host.querySelector("video")).not.toBeNull());
            expect(control(video, "ダウンロード")).toBeNull();
            expect(control(video, "削除")).toBeNull();
        } finally {
            await video.dispose();
        }
    });

    it("確認のダイアログを外枠の要素の中に出し、「削除する」で削除して、次の画像へ移る", async () => {
        trio();
        remove.mockResolvedValue([]);
        const h = await open("1.png");
        try {
            await askDeletion(h);
            expect(outer(h)?.contains(confirmation(h))).toBe(true);
            expect(remove).not.toHaveBeenCalled();
            await press("削除する");
            expect(remove).toHaveBeenCalledOnce();
            expect([...(remove.mock.calls[0]?.[2] ?? [])]).toEqual([["/1.png", { folder: false }]]);
            expect(h.onChange).toHaveBeenCalledExactlyOnceWith("2.png");
            await wait(() => expect(confirmation(h)).toBeNull());
        } finally {
            await h.dispose();
        }
    });

    it("次がなければ前の画像へ移り、どちらもなければビューアを閉じる", async () => {
        trio();
        remove.mockResolvedValue([]);
        const last = await open("2.png");
        try {
            await askDeletion(last);
            await press("削除する");
            expect(last.onChange).toHaveBeenCalledExactlyOnceWith("1.png");
        } finally {
            await last.dispose();
        }
        listed([image("only.png", "image/png", 1)]);
        const only = await open("only.png");
        try {
            await askDeletion(only);
            await press("削除する");
            expect(only.onChange).toHaveBeenCalledExactlyOnceWith(undefined);
        } finally {
            await only.dispose();
        }
    });

    it("「取り消す」では削除せず、移動もしない", async () => {
        trio();
        const h = await open("1.png");
        try {
            await askDeletion(h);
            await press("取り消す");
            await wait(() => expect(confirmation(h)).toBeNull());
            expect(remove).not.toHaveBeenCalled();
            expect(h.onChange).not.toHaveBeenCalled();
        } finally {
            await h.dispose();
        }
    });

    it("削除に失敗すると、ビューアの中に失敗のトーストを出し、開いている画像は変わらない", async () => {
        trio();
        remove.mockRejectedValue(new Error("denied"));
        const h = await open("1.png");
        try {
            await askDeletion(h);
            await press("削除する");
            await wait(() => expect(outer(h)?.textContent).toContain("削除できない項目があります。"));
            expect(h.onChange).not.toHaveBeenCalled();
            expect(confirmation(h)).toBeNull();
            expect(findDisplayImage(h)?.alt).toBe("1.png");
        } finally {
            await h.dispose();
        }
    });

    it("ダイアログが出ている間は、矢印キーで移らず、Esc はダイアログだけを閉じる", async () => {
        trio();
        const h = await open("1.png");
        try {
            await askDeletion(h);
            await key(window, "ArrowRight");
            await key(window, "ArrowLeft");
            expect(h.onChange).not.toHaveBeenCalled();
            const dialog = confirmation(h);
            if (!dialog) throw new Error("ダイアログがない");
            await key(dialog, "Escape");
            await wait(() => expect(confirmation(h)).toBeNull());
            expect(h.onChange).not.toHaveBeenCalled();
            expect(remove).not.toHaveBeenCalled();
        } finally {
            await h.dispose();
        }
    });

    it("削除のボタンを押すと、スライドショーが止まる", async () => {
        trio();
        const h = await open("1.png");
        try {
            await act(async () => control(h, "スライドショー")?.click());
            expect(control(h, "スライドショーを止める")).not.toBeNull();
            await askDeletion(h);
            expect(control(h, "スライドショー")).not.toBeNull();
        } finally {
            await h.dispose();
        }
    });

    it("画像が 1 枚だけでも、削除に失敗すると、ビューアの中に失敗のトーストを出し、ビューアを閉じない", async () => {
        listed([image("only.png", "image/png", 1)]);
        remove.mockRejectedValue(new Error("denied-for-only-image"));
        const h = await open("only.png");
        try {
            await askDeletion(h);
            await press("削除する");
            await wait(() => expect(outer(h)?.textContent).toContain("denied-for-only-image"));
            expect(h.onChange).not.toHaveBeenCalled();
        } finally {
            await h.dispose();
        }
    });

    it("次の範囲を読み込んでいる間は削除を選べず、読み終えると選べる", async () => {
        let resolve: ((value: ReturnType<typeof response>) => void) | undefined;
        get.mockImplementation((_, options) =>
            options.params.query.offset === 0
                ? Promise.resolve(response(Array.from({ length: 200 }, (_, index) => entry(index))))
                : new Promise((done) => {
                      resolve = done;
                  }),
        );
        const h = await harness("199.png", "/", "onZoom", undefined, false);
        try {
            await wait(() => expect(findDisplayImage(h)).not.toBeNull());
            expect(control(h, "削除")?.disabled).toBe(false);
            await act(async () => window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight" })));
            await wait(() => expect(resolve).toBeDefined());
            expect(control(h, "削除")?.disabled).toBe(true);
            await act(async () => control(h, "削除")?.click());
            expect(confirmation(h)).toBeNull();
            await act(async () => {
                resolve?.(response([entry(200)]));
            });
            await wait(() => expect(h.onChange).toHaveBeenCalledExactlyOnceWith("200.png"));
            await wait(() => expect(control(h, "削除")?.disabled).toBe(false));
        } finally {
            await h.dispose();
        }
    });

    it("削除の確認の準備中にスライドショーを始めようとしても、始まらず、確認の間に移動しない", async () => {
        let resolve: ((value: ReturnType<typeof response>) => void) | undefined;
        get.mockImplementation((_, options) =>
            options.params.query.offset === 0
                ? Promise.resolve(response(Array.from({ length: 200 }, (_, index) => entry(index))))
                : new Promise((done) => {
                      resolve = done;
                  }),
        );
        const h = await harness("199.png", "/", "onZoom", undefined, false);
        try {
            await wait(() => expect(findDisplayImage(h)).not.toBeNull());
            vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
            await loadDisplay(h);
            await act(async () => control(h, "削除")?.click());
            await act(async () => vi.advanceTimersByTimeAsync(0));
            expect(resolve).toBeDefined();
            await act(async () => control(h, "スライドショー")?.click());
            expect(control(h, "スライドショー")).not.toBeNull();
            await act(async () => {
                resolve?.(response([entry(200)]));
            });
            await wait(() => expect(confirmation(h)).not.toBeNull());
            await act(async () => vi.advanceTimersByTimeAsync(20_000));
            expect(h.onChange).not.toHaveBeenCalled();
            expect(control(h, "スライドショー")).not.toBeNull();
        } finally {
            vi.useRealTimers();
            await h.dispose();
        }
    });

    it("ダイアログが出たまま狭い画面へ切り替わると、ダイアログを取り消す", async () => {
        trio();
        const h = await open("1.png");
        try {
            await askDeletion(h);
            await h.render("1.png", true);
            await wait(() => expect(confirmation(h)).toBeNull());
            expect(remove).not.toHaveBeenCalled();
        } finally {
            await h.dispose();
        }
    });
});

describe("レートモード", () => {
    const wideViewer = (view: string) => harness(view, "/", "onZoom", undefined, false);
    const rateButton = (h: { host: HTMLElement }) =>
        h.host.querySelector<HTMLButtonElement>('button[aria-label="レートモード"]');
    const press = (key: string) => act(async () => window.dispatchEvent(new KeyboardEvent("keydown", { key })));
    const badge = (h: { host: HTMLElement }) => h.host.querySelector("[data-rating]");
    /// 一覧と、保存済みの評価を返す daemon の代わり。
    const answering = (entries: Entry[], ratings: Record<string, string> = {}) =>
        get.mockImplementation(async (route: string, options?: { params?: { query?: { path?: string } } }) =>
            route === "/libraries/{libraryId}/rating"
                ? { data: { rating: ratings[options?.params?.query?.path ?? ""] ?? "none" }, response: new Response() }
                : {
                      data: { entries, totalCount: entries.length, snapshot: "one", truncated: false },
                      response: new Response(),
                  },
        );
    const images = [0, 1, 2].map((index) => image(`${index}.png`, "image/png", 1));

    it("入るまでは評価せず、← → は画像の移動のままである", async () => {
        answering(images);
        rate.mockResolvedValue(undefined);
        const h = await wideViewer("1.png");
        try {
            await wait(() => expect(findDisplayImage(h)).not.toBeNull());
            expect(rateButton(h)?.getAttribute("aria-pressed")).toBe("false");
            await press("ArrowRight");
            await wait(() => expect(h.onChange).toHaveBeenCalledWith("2.png"));
            expect(rate).not.toHaveBeenCalled();
            expect(badge(h)).toBeNull();
        } finally {
            await h.dispose();
        }
    });

    it("→ は good、← は bad、↓ は none を記録して次へ移る", async () => {
        answering(images);
        rate.mockResolvedValue(undefined);
        const h = await wideViewer("0.png");
        try {
            await wait(() => expect(findDisplayImage(h)).not.toBeNull());
            await act(async () => rateButton(h)?.click());
            expect(rateButton(h)?.getAttribute("aria-pressed")).toBe("true");

            await press("ArrowRight");
            await wait(() => expect(h.onChange).toHaveBeenLastCalledWith("1.png"));
            expect(rate.mock.calls.map(([, libraryId, path, value]) => [libraryId, path, value])).toEqual([
                ["big", "/0.png", "good"],
            ]);

            await h.render("1.png");
            await press("ArrowLeft");
            await wait(() => expect(h.onChange).toHaveBeenLastCalledWith("2.png"));
            expect(rate.mock.calls.at(-1)?.slice(1)).toEqual(["big", "/1.png", "bad"]);

            await h.render("2.png");
            await press("ArrowDown");
            await wait(() => expect(rate).toHaveBeenCalledTimes(3));
            expect(rate.mock.calls.at(-1)?.slice(1)).toEqual(["big", "/2.png", "none"]);
            expect(h.onChange).toHaveBeenCalledTimes(2);
        } finally {
            await h.dispose();
        }
    });

    it("記録は移動を待たせず、最後の Entry でも評価は残る", async () => {
        answering(images);
        rate.mockReturnValue(new Promise(() => {}));
        const h = await wideViewer("1.png");
        try {
            await wait(() => expect(findDisplayImage(h)).not.toBeNull());
            await press("r");
            await press("ArrowRight");
            await wait(() => expect(h.onChange).toHaveBeenLastCalledWith("2.png"));
            await h.render("2.png");
            await press("ArrowRight");
            await wait(() => expect(rate).toHaveBeenCalledTimes(2));
            expect(rate.mock.calls.at(-1)?.slice(1)).toEqual(["big", "/2.png", "good"]);
            expect(h.onChange).toHaveBeenCalledTimes(1);
        } finally {
            await h.dispose();
        }
    });

    it("↑ は最後の評価の操作から順に、その Entry へ戻って記録する前の評価へ書き戻す", async () => {
        answering(images, { "/0.png": "bad" });
        rate.mockResolvedValue(undefined);
        const h = await wideViewer("0.png");
        try {
            await wait(() => expect(findDisplayImage(h)).not.toBeNull());
            await press("r");
            await press("ArrowUp");
            expect(rate).not.toHaveBeenCalled();
            await wait(() => expect(badge(h)?.getAttribute("data-rating")).toBe("bad"));

            await press("ArrowRight");
            await wait(() => expect(h.onChange).toHaveBeenLastCalledWith("1.png"));
            await h.render("1.png");
            await press("ArrowLeft");
            await wait(() => expect(h.onChange).toHaveBeenLastCalledWith("2.png"));
            await h.render("2.png");
            await wait(() => expect(rate).toHaveBeenCalledTimes(2));

            await press("ArrowUp");
            await wait(() => expect(h.onChange).toHaveBeenLastCalledWith("1.png"));
            await wait(() => expect(rate).toHaveBeenCalledTimes(3));
            expect(rate.mock.calls.at(-1)?.slice(1)).toEqual(["big", "/1.png", "none"]);
            await h.render("1.png");

            await press("ArrowUp");
            await wait(() => expect(h.onChange).toHaveBeenLastCalledWith("0.png"));
            await wait(() => expect(rate).toHaveBeenCalledTimes(4));
            expect(rate.mock.calls.at(-1)?.slice(1)).toEqual(["big", "/0.png", "bad"]);
            await h.render("0.png");

            const calls = h.onChange.mock.calls.length;
            await press("ArrowUp");
            await act(async () => Promise.resolve());
            expect(rate).toHaveBeenCalledTimes(4);
            expect(h.onChange).toHaveBeenCalledTimes(calls);
        } finally {
            await h.dispose();
        }
    });

    it("評価のボタンをレートモードの間だけ出し、キーと同じ操作をする", async () => {
        answering(images);
        rate.mockResolvedValue(undefined);
        const h = await wideViewer("0.png");
        const pad = (label: string) =>
            h.host.querySelector<HTMLButtonElement>(`fieldset[aria-label=評価] button[aria-label="${label}"]`);
        try {
            await wait(() => expect(findDisplayImage(h)).not.toBeNull());
            expect(h.host.querySelector("fieldset[aria-label=評価]")).toBeNull();
            await press("r");
            expect(pad("元に戻す")?.disabled).toBe(true);
            await act(async () => pad("良い")?.click());
            await wait(() => expect(h.onChange).toHaveBeenLastCalledWith("1.png"));
            expect(rate.mock.calls.at(-1)?.slice(1)).toEqual(["big", "/0.png", "good"]);
            await h.render("1.png");
            await act(async () => pad("悪い")?.click());
            await wait(() => expect(h.onChange).toHaveBeenLastCalledWith("2.png"));
            expect(rate.mock.calls.at(-1)?.slice(1)).toEqual(["big", "/1.png", "bad"]);
            await h.render("2.png");
            await act(async () => pad("未評価")?.click());
            await wait(() => expect(rate).toHaveBeenCalledTimes(3));
            expect(rate.mock.calls.at(-1)?.slice(1)).toEqual(["big", "/2.png", "none"]);
            expect(pad("元に戻す")?.disabled).toBe(false);
            await act(async () => pad("元に戻す")?.click());
            await wait(() => expect(rate).toHaveBeenCalledTimes(4));
            expect(rate.mock.calls.at(-1)?.slice(1)).toEqual(["big", "/2.png", "none"]);
            await press("r");
            expect(h.host.querySelector("fieldset[aria-label=評価]")).toBeNull();
        } finally {
            await h.dispose();
        }
    });

    it("保存済みの評価を開いたときに出し、未評価のときは操作を案内する", async () => {
        answering(images, { "/0.png": "good", "/1.png": "bad" });
        const h = await wideViewer("0.png");
        try {
            await wait(() => expect(findDisplayImage(h)).not.toBeNull());
            await press("r");
            await wait(() => expect(badge(h)?.getAttribute("data-rating")).toBe("good"));
            expect(badge(h)?.textContent).toContain("良い");
            await h.render("1.png");
            await wait(() => expect(badge(h)?.getAttribute("data-rating")).toBe("bad"));
            await h.render("2.png");
            await wait(() => expect(badge(h)?.getAttribute("data-rating")).toBe("none"));
            expect(badge(h)?.textContent).toContain("→ 良い");
        } finally {
            await h.dispose();
        }
    });

    it("動画にも評価でき、次へは動画にも止まる", async () => {
        answering([images[0]!, image("1.mp4", "video/mp4", 1), images[2]!]);
        rate.mockResolvedValue(undefined);
        const h = await wideViewer("0.png");
        try {
            await wait(() => expect(findDisplayImage(h)).not.toBeNull());
            await press("r");
            await press("ArrowRight");
            await wait(() => expect(h.onChange).toHaveBeenLastCalledWith("1.mp4"));
            await h.render("1.mp4");
            await wait(() => expect(h.host.querySelector("video")).not.toBeNull());
            expect(rateButton(h)).not.toBeNull();
            await press("ArrowLeft");
            await wait(() => expect(h.onChange).toHaveBeenLastCalledWith("2.png"));
            expect(rate.mock.calls.at(-1)?.slice(1)).toEqual(["big", "/1.mp4", "bad"]);
        } finally {
            await h.dispose();
        }
    });

    it("スライドショーを始めるとレートモードを終え、レートモードに入るとスライドショーを止める", async () => {
        answering(images);
        const h = await wideViewer("0.png");
        try {
            await wait(() => expect(findDisplayImage(h)).not.toBeNull());
            await act(async () => rateButton(h)?.click());
            await act(async () =>
                h.host.querySelector<HTMLButtonElement>("button[aria-label=スライドショー]")?.click(),
            );
            expect(rateButton(h)?.getAttribute("aria-pressed")).toBe("false");
            expect(h.host.querySelector("button[aria-label=スライドショーを止める]")).not.toBeNull();
            await act(async () => rateButton(h)?.click());
            expect(h.host.querySelector("button[aria-label=スライドショー]")).not.toBeNull();
            expect(rateButton(h)?.getAttribute("aria-pressed")).toBe("true");
        } finally {
            await h.dispose();
        }
    });

    it("狭い画面にはレートモードを出さず、r のキーも効かない", async () => {
        answering(images);
        const h = await harness("0.png", "/", "onZoom", undefined, true);
        try {
            await wait(() => expect(findDisplayImage(h)).not.toBeNull());
            expect(rateButton(h)).toBeNull();
            await press("r");
            await press("ArrowRight");
            await wait(() => expect(h.onChange).toHaveBeenCalledWith("1.png"));
            expect(rate).not.toHaveBeenCalled();
        } finally {
            await h.dispose();
        }
    });

    it("削除の確認が開いている間は、評価のキーを受け付けない", async () => {
        answering(images);
        rate.mockResolvedValue(undefined);
        const h = await wideViewer("1.png");
        try {
            await wait(() => expect(findDisplayImage(h)).not.toBeNull());
            await press("r");
            await act(async () => h.host.querySelector<HTMLButtonElement>('button[aria-label="削除"]')?.click());
            await wait(() => expect(document.querySelector("[role=alertdialog]")).not.toBeNull());
            await press("ArrowRight");
            expect(rate).not.toHaveBeenCalled();
        } finally {
            await h.dispose();
        }
    });
});
