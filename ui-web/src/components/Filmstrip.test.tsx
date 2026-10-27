// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import type { ImageWants } from "../images/wants";
import type { Entry } from "../library/entries";
import type { Listing } from "../library/listing";
import { stubResizeObserver } from "../testing/preferences";
import { Filmstrip } from "./Filmstrip";

const mock = vi.hoisted(() => ({
    first: 0,
    last: 2,
    scrollToIndex: vi.fn(),
    measure: vi.fn(),
    options: undefined as { horizontal?: boolean; count: number } | undefined,
}));
vi.mock("@tanstack/react-virtual", () => ({
    useVirtualizer: (options: { horizontal?: boolean; count: number; estimateSize: () => number }) => {
        mock.options = options;
        return {
            range: { startIndex: mock.first, endIndex: mock.last },
            scrollToIndex: mock.scrollToIndex,
            measure: mock.measure,
            getTotalSize: () => options.count * options.estimateSize(),
            getVirtualItems: () =>
                Array.from({ length: mock.last - mock.first + 1 }, (_, offset) => ({
                    index: mock.first + offset,
                    key: mock.first + offset,
                    start: (mock.first + offset) * options.estimateSize(),
                    size: options.estimateSize(),
                })),
        };
    },
}));
afterEach(() => {
    mock.scrollToIndex.mockReset();
    mock.measure.mockReset();
    mock.first = 0;
    mock.last = 2;
});

const image = (index: number): Entry => ({
    name: `img-${index}.png`,
    path: `/dir/img-${index}.png`,
    kind: "file",
    metadata: { size: 1, readonly: false, contentType: "image/png" },
    file: { archive: false },
});
const video: Entry = {
    name: "clip.mp4",
    path: "/dir/clip.mp4",
    kind: "file",
    metadata: { size: 1, readonly: false, contentType: "video/mp4" },
    file: { archive: false },
};
const folder: Entry = { name: "sub", path: "/dir/sub", kind: "folder", metadata: { size: 0, readonly: false } };

/// 0 は動画、1 はフォルダ、2 以降は画像、`loaded` を超える位置は未読。
function listingOf(totalCount: number, loaded: number): Listing {
    return {
        truncated: false,
        totalCount,
        version: 1,
        snapshot: "one",
        entryAt: (index) => (index >= loaded ? undefined : index === 0 ? video : index === 1 ? folder : image(index)),
        pageStatus: () => "loaded",
        pageError: () => null,
        loadPages: async () => {},
        retry: () => {},
    };
}

const IDLE = { status: "idle" };
function fakeWants() {
    const setWanted = vi.fn();
    const wants = {
        subscribe: () => () => {},
        getState: () => IDLE,
        setWanted,
    } as unknown as ImageWants;
    return { wants, setWanted };
}

async function render(props: Partial<React.ComponentProps<typeof Filmstrip>> = {}) {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    const restore = stubResizeObserver();
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    const { wants, setWanted } = fakeWants();
    const onSelect = vi.fn();
    const onPages = vi.fn();
    const all = {
        listing: listingOf(1000, 200),
        index: 3,
        side: 56,
        wants,
        onSelect,
        onPages,
        ...props,
    };
    await act(async () => root.render(<Filmstrip {...all} />));
    return {
        host,
        onSelect,
        onPages,
        setWanted,
        rerender: (next: Partial<React.ComponentProps<typeof Filmstrip>>) =>
            act(async () => root.render(<Filmstrip {...all} {...next} />)),
        dispose: async () => {
            await act(async () => root.unmount());
            host.remove();
            restore();
        },
    };
}
const byName = (host: HTMLElement, name: string) => host.querySelector<HTMLElement>(`[aria-label="${name}"]`);

it("1000 件の一覧に、1 件 56 px の横の仮想スクロールで、全体の幅を先に確保する", async () => {
    const h = await render();
    try {
        expect(mock.options?.horizontal).toBe(true);
        expect(mock.options?.count).toBe(1000);
        const inner = byName(h.host, "フィルムストリップ")?.firstElementChild as HTMLElement;
        expect(inner.style.width).toBe("56000px");
        // 描くのは、仮想スクロールが返した見えている範囲だけである。
        expect(h.host.querySelectorAll("[aria-label$='.png'], [aria-label$='.mp4'], [aria-label=sub]")).toHaveLength(3);
    } finally {
        await h.dispose();
    }
});

it.each([
    { level: 1, side: 40 },
    { level: 5, side: 96 },
])("大きさの段階 $level では、枠と位置と中の画像と全体の幅が $side px から決まる", async ({ side }) => {
    mock.first = 0;
    mock.last = 2;
    const h = await render({ side });
    try {
        const section = byName(h.host, "フィルムストリップ") as HTMLElement;
        expect(section.style.height).toBe(`${side + 8}px`);
        const inner = section.firstElementChild as HTMLElement;
        expect(inner.style.width).toBe(`${1000 * side}px`);
        expect(inner.style.height).toBe(`${side}px`);
        expect((inner.firstElementChild as HTMLElement).style.width).toBe(`${side}px`);
        expect((inner.firstElementChild as HTMLElement).style.height).toBe(`${side}px`);
        for (const name of ["clip.mp4", "sub", "img-2.png"]) {
            const element = byName(h.host, name) as HTMLElement;
            expect([element.style.width, element.style.height], name).toEqual([`${side - 4}px`, `${side - 4}px`]);
        }
    } finally {
        await h.dispose();
    }
});

it("未読の位置の枠も、大きさの段階の位置より 4 px 小さい", async () => {
    mock.first = 199;
    mock.last = 200;
    const h = await render({ side: 96, index: 199 });
    try {
        const placeholder = h.host.querySelector<HTMLElement>("[data-placeholder]");
        expect([placeholder?.style.width, placeholder?.style.height]).toEqual(["92px", "92px"]);
    } finally {
        await h.dispose();
    }
});

it("位置の大きさが変わると、測り直して、開いている位置を中央に置き直す", async () => {
    const h = await render({ index: 3, side: 56 });
    try {
        const centered = mock.scrollToIndex.mock.calls.length;
        const measured = mock.measure.mock.calls.length;
        await h.rerender({ side: 96 });
        expect(mock.measure.mock.calls.length).toBeGreaterThan(measured);
        expect(mock.scrollToIndex.mock.calls.length).toBeGreaterThan(centered);
        expect(mock.scrollToIndex).toHaveBeenLastCalledWith(3, { align: "center" });
        expect(mock.measure.mock.invocationCallOrder.at(-1)).toBeLessThan(
            mock.scrollToIndex.mock.invocationCallOrder.at(-1) as number,
        );
        expect(mock.options?.count).toBe(1000);
        const section = byName(h.host, "フィルムストリップ") as HTMLElement;
        expect(section.style.height).toBe("104px");
    } finally {
        await h.dispose();
    }
});

it("画像の位置はボタンで、押すとその位置と Entry で移動し、開いている位置を強調する", async () => {
    mock.first = 2;
    mock.last = 4;
    const h = await render({ index: 3 });
    try {
        const current = byName(h.host, "img-3.png");
        expect(current?.tagName).toBe("BUTTON");
        expect(current?.getAttribute("aria-current")).toBe("true");
        expect(byName(h.host, "img-4.png")?.hasAttribute("aria-current")).toBe(false);
        await act(async () => byName(h.host, "img-4.png")?.click());
        expect(h.onSelect).toHaveBeenCalledExactlyOnceWith({ index: 4, entry: image(4) });
        await act(async () => current?.click());
        expect(h.onSelect).toHaveBeenCalledTimes(1);
    } finally {
        await h.dispose();
    }
});

it("動画の位置もボタンで、押すと動画へ移動する", async () => {
    mock.first = 0;
    mock.last = 2;
    const h = await render();
    try {
        const element = byName(h.host, "clip.mp4");
        expect(element?.tagName).toBe("BUTTON");
        await act(async () => element?.click());
        expect(h.onSelect).toHaveBeenCalledExactlyOnceWith({ index: 0, entry: video });
    } finally {
        await h.dispose();
    }
});

it("画像でも動画でもない Entry の位置は、ボタンにせず、種類のアイコンを出す", async () => {
    mock.first = 0;
    mock.last = 2;
    const h = await render();
    try {
        const element = byName(h.host, "sub");
        expect(element?.tagName).toBe("DIV");
        expect(element?.querySelector("svg")).not.toBeNull();
        await act(async () => element?.click());
        expect(h.onSelect).not.toHaveBeenCalled();
        expect(h.host.querySelectorAll("button")).toHaveLength(2);
    } finally {
        await h.dispose();
    }
});

it("未読の位置は、空の枠を出して、ボタンにしない", async () => {
    mock.first = 198;
    mock.last = 201;
    const h = await render({ listing: listingOf(1000, 200), index: 198 });
    try {
        expect(h.host.querySelectorAll("[data-placeholder]")).toHaveLength(2);
        expect(h.host.querySelectorAll("button")).toHaveLength(2);
    } finally {
        await h.dispose();
    }
});

it("開いている位置を変えると、その位置を中央にするスクロールを指定する", async () => {
    const h = await render({ index: 3 });
    try {
        expect(mock.scrollToIndex).toHaveBeenLastCalledWith(3, { align: "center" });
        await h.rerender({ index: 7 });
        expect(mock.scrollToIndex).toHaveBeenLastCalledWith(7, { align: "center" });
    } finally {
        await h.dispose();
    }
});

it("見えている範囲のページを渡し、範囲が動くと前のページを外し、閉じると空にする", async () => {
    mock.first = 0;
    mock.last = 22;
    const h = await render();
    try {
        expect([...(h.onPages.mock.lastCall?.[0] ?? [])]).toEqual([0]);
        mock.first = 909;
        mock.last = 931;
        await h.rerender({});
        expect([...(h.onPages.mock.lastCall?.[0] ?? [])]).toEqual([4]);
    } finally {
        await h.dispose();
    }
    expect([...(h.onPages.mock.lastCall?.[0] ?? [])]).toEqual([]);
});

it("見えている範囲の画像と動画だけサムネイルを要求し、閉じると要求を取り消す", async () => {
    mock.first = 0;
    mock.last = 4;
    const h = await render();
    const wanted = h.setWanted.mock.lastCall?.[0] as ReadonlyMap<string, number>;
    expect(wanted.has("/dir/img-2.png")).toBe(true);
    expect(wanted.has("/dir/clip.mp4")).toBe(true);
    expect(wanted.has("/dir/sub")).toBe(false);
    await h.dispose();
    expect(h.setWanted.mock.lastCall?.[0]).toEqual(new Map());
});
