// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";
import type { Entry } from "../library/entries";
import { refreshLibrary } from "../library/listing";
import { stubChangingMatchMedia, stubResizeObserver } from "../testing/preferences";
import { Tree } from "./Tree";

const { get, post, navigate } = vi.hoisted(() => ({ get: vi.fn(), post: vi.fn(), navigate: vi.fn() }));
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
        POST: post,
    },
}));
vi.mock("@tanstack/react-router", () => ({ useNavigate: () => navigate }));
// jsdom は layout を計算しないので、viewport と scroll だけを固定する。
vi.mock("@tanstack/react-virtual", async () => {
    const { useState } = await import("react");
    return {
        defaultRangeExtractor: ({ startIndex, endIndex }: { startIndex: number; endIndex: number }) =>
            Array.from({ length: endIndex - startIndex + 1 }, (_, i) => startIndex + i),
        useVirtualizer: (options: {
            count: number;
            estimateSize(index: number): number;
            rangeExtractor(range: { startIndex: number; endIndex: number }): number[];
        }) => {
            const [first, setFirst] = useState(0);
            return {
                scrollOffset: first * 32,
                scrollRect: { height: 600, width: 224 },
                measure: () => {},
                scrollToIndex: (index: number) => setFirst(Math.max(0, index - 10)),
                getTotalSize: () => options.count * 32,
                getVirtualItems: () =>
                    options
                        .rangeExtractor({ startIndex: first, endIndex: Math.min(first + 20, options.count - 1) })
                        .map((index) => ({
                            index,
                            key: index,
                            start: index * 32,
                            size: options.estimateSize(index),
                            end: index * 32 + options.estimateSize(index),
                        })),
            };
        },
    };
});

async function wait(assertion: () => void) {
    await vi.waitFor(async () => {
        await act(async () => {
            await Promise.resolve();
        });
        assertion();
    });
}
async function harness(narrow = false, folderCount = 0, blockLaterPages = false, probes: boolean | "ready" = false) {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    const folder = (path: string): Entry => ({
        path,
        name: path.split("/").at(-1)!,
        kind: "folder",
        metadata: { size: 0, readonly: false },
    });
    const archive: Entry = {
        path: "/pack.zip",
        name: "pack.zip",
        kind: "file",
        file: { archive: true },
        metadata: { size: 0, readonly: false },
    };
    const children = new Map<string, Entry[]>([
        [
            "/",
            folderCount
                ? Array.from({ length: folderCount }, (_, i) => folder(`/folder-${String(i).padStart(5, "0")}`))
                : [folder("/tree"), archive],
        ],
        ["/tree", []],
    ]);
    let snapshot = 0;
    const response = (data: unknown, status = 200) => ({ data, response: new Response(null, { status }) });
    get.mockImplementation(async (route, options) => {
        if (route === "/libraries")
            return response({
                libraries: [
                    { id: "ops", name: "Ops", readonly: false },
                    ...(folderCount ? [] : [{ id: "ro", name: "Ro", readonly: true }]),
                ],
            });
        const path = options.params.query.path;
        if (route.endsWith("/children"))
            return probes === "ready"
                ? response({ hasChildren: false, sourceVersion: "v1", maxAgeMs: 30000 })
                : new Promise(() => {});
        if (route.endsWith("/folders"))
            return response({ folder: { ...folder(path), metadata: { size: 0, readonly: path.endsWith(".zip") } } });
        const entries = options.params.path.libraryId === "ro" ? [] : (children.get(path) ?? []);
        const offset = options.params.query.offset ?? 0;
        if (blockLaterPages && offset > 0) return new Promise(() => {});
        return response({
            entries: entries.slice(offset, offset + 200),
            totalCount: entries.length,
            snapshot: String(snapshot),
            truncated: false,
        });
    });
    post.mockImplementation(async (route, options) => {
        snapshot++;
        if (route.endsWith("/move")) {
            const src = options.body.srcPaths[0];
            const parent = src.slice(0, src.lastIndexOf("/")) || "/";
            const items = children.get(parent)!;
            const index = items.findIndex((entry) => entry.path === src);
            const changed = {
                ...items[index]!,
                path: options.body.dstPath,
                name: options.body.dstPath.split("/").at(-1)!,
            };
            items[index] = changed;
            return response({ entries: [changed] });
        }
        if (route.endsWith("/delete")) {
            for (const [path, entries] of children)
                children.set(
                    path,
                    entries.filter((entry) => !options.body.srcPaths.includes(entry.path)),
                );
            return response(undefined, 204);
        }
        const path = options.body.path;
        const parent = path.slice(0, path.lastIndexOf("/")) || "/";
        const entry = folder(path);
        children.set(parent, [...(children.get(parent) ?? []), entry]);
        return response({ folder: entry }, 201);
    });
    const media = stubChangingMatchMedia(narrow);
    const restoreResize = stubResizeObserver();
    const client = new QueryClient();
    if (probes)
        client.setQueryData(["capabilities"], { summaryListing: false, entryDetails: false, childProbes: true });
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    await act(async () =>
        root.render(
            <QueryClientProvider client={client}>
                <Tree location={{ libraryId: "ops", path: "/" }} />
            </QueryClientProvider>,
        ),
    );
    await wait(() => expect(host.textContent).toContain(folderCount ? "folder-00000" : "tree"));
    const item = (name: string) =>
        [...host.querySelectorAll<HTMLElement>('[role="treeitem"]')].find(
            (item) => item.textContent?.replace(/[▾▸]/g, "") === name,
        )!;
    const menu = async (name: string) => {
        await act(async () => item(name).dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, button: 2 })));
    };
    const select = async (label: string) => {
        const target = [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].find(
            (item) => item.textContent === label,
        )!;
        await wait(() => expect(target.getAttribute("aria-disabled")).not.toBe("true"));
        await act(async () => target.click());
    };
    const type = async (value: string) => {
        await wait(() => expect(document.querySelector('input[aria-label="名前"]')).not.toBeNull());
        const input = document.querySelector<HTMLInputElement>('input[aria-label="名前"]')!;
        expect(document.activeElement).toBe(input);
        await act(async () => {
            Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value);
            input.dispatchEvent(new Event("input", { bubbles: true }));
        });
        await act(async () =>
            document.querySelector<HTMLButtonElement>('[role="dialog"] button[type="submit"]')!.click(),
        );
    };
    return {
        client,
        host,
        item,
        menu,
        select,
        type,
        resize: async (value: boolean) => {
            await act(async () => media.setNarrow(value));
        },
        dispose: async () => {
            await act(async () => root.unmount());
            host.remove();
            client.clear();
            media.restore();
            restoreResize();
            get.mockReset();
            post.mockReset();
            navigate.mockReset();
        },
    };
}
it("閉じたフォルダを展開し、その直下で作成・名前の変更・削除", async () => {
    const h = await harness();
    try {
        await h.menu("tree");
        await h.select("新しいフォルダ");
        await wait(() => expect(document.querySelector('[role="dialog"]')?.textContent).toContain("作成先: /tree"));
        await h.type("made");
        await wait(() => expect(h.item("made")).toBeDefined());
        expect(h.item("tree").getAttribute("aria-expanded")).toBe("true");
        expect(post.mock.calls[0]![1].body).toEqual({ path: "/tree/made", createParents: false });
        await h.menu("made");
        await h.select("名前の変更");
        await wait(() => expect(document.querySelector('[role="dialog"]')?.textContent).toContain("対象: /tree/made"));
        expect(document.querySelector<HTMLInputElement>('input[aria-label="名前"]')!.value).toBe("made");
        await h.type("made2");
        await wait(() => expect(h.item("made2")).toBeDefined());
        await h.menu("made2");
        await h.select("削除");
        await act(async () =>
            [...document.querySelectorAll<HTMLButtonElement>('[role="alertdialog"] button')]
                .find((b) => b.textContent === "削除する")!
                .click(),
        );
        await wait(() => expect(h.item("made2")).toBeUndefined());
        expect(navigate).not.toHaveBeenCalled();
    } finally {
        await h.dispose();
    }
});
it("archive は files/move、ルートは rename/delete 非表示、readonly は無効", async () => {
    const h = await harness();
    try {
        await h.menu("pack.zip");
        await h.select("名前の変更");
        await h.type("renamed.zip");
        await wait(() => expect(h.item("renamed.zip")).toBeDefined());
        expect(post.mock.calls[0]![0]).toBe("/libraries/{libraryId}/files/move");
        await h.menu("Ops");
        expect([...document.querySelectorAll('[role="menuitem"]')].map((item) => item.textContent)).toEqual([
            "新しいフォルダ",
            "貼り付け",
        ]);
        await act(async () =>
            document.activeElement!.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })),
        );
        await h.menu("Ro");
        expect(document.querySelector('[role="menuitem"]')?.getAttribute("aria-disabled")).toBe("true");
    } finally {
        await h.dispose();
    }
});
it("狭いツリーにはメニューを出さない", async () => {
    const h = await harness(true);
    try {
        await h.menu("tree");
        expect(document.querySelector('[role="menu"]')).toBeNull();
    } finally {
        await h.dispose();
    }
});

it("ツリーの入力と削除確認も狭い画面への切り替えで取り消す", async () => {
    const h = await harness();
    try {
        await h.menu("tree");
        await h.select("新しいフォルダ");
        await wait(() => expect(document.querySelector('input[aria-label="名前"]')).not.toBeNull());
        await h.resize(true);
        expect(document.querySelector('input[aria-label="名前"]')).toBeNull();
        expect(post).not.toHaveBeenCalled();
        await h.resize(false);
        await h.menu("tree");
        await h.select("名前の変更");
        await wait(() => expect(document.querySelector('input[aria-label="名前"]')).not.toBeNull());
        await h.resize(true);
        expect(document.querySelector('input[aria-label="名前"]')).toBeNull();
        expect(post).not.toHaveBeenCalled();
        await h.resize(false);
        await h.menu("tree");
        await h.select("削除");
        expect(document.querySelector("[role=alertdialog]")).not.toBeNull();
        await h.resize(true);
        expect(document.querySelector("[role=alertdialog]")).toBeNull();
        expect(post).not.toHaveBeenCalled();
        await h.resize(false);
        expect(document.querySelector("[role=alertdialog]")).toBeNull();
    } finally {
        await h.dispose();
    }
});

it("22432 件でも最初のページだけで可視行を描画し、遅い続きに待たされない", async () => {
    const h = await harness(false, 22432, true);
    try {
        expect(h.item("folder-00000")).toBeDefined();
        expect(h.host.querySelectorAll('[role="treeitem"]').length).toBeLessThan(200);
        expect(
            get.mock.calls
                .filter(([route]) => route.endsWith("/entries"))
                .map(([, options]) => options.params.query.offset),
        ).toEqual([0]);
        await act(async () => h.item("folder-00000").click());
        expect(navigate).toHaveBeenCalled();
    } finally {
        await h.dispose();
    }
});

it("End は末尾のページだけを取得して画面外の行へフォーカスする", async () => {
    const h = await harness(false, 22432);
    try {
        const tree = h.host.querySelector('[role="tree"]')!;
        await act(async () => tree.dispatchEvent(new KeyboardEvent("keydown", { key: "End", bubbles: true })));
        await wait(() => expect(document.activeElement?.textContent).toContain("folder-22431"));
        const offsets = get.mock.calls
            .filter(([route]) => route.endsWith("/entries"))
            .map(([, options]) => options.params.query.offset);
        expect(new Set(offsets)).toEqual(new Set([0, 22400]));
        expect(h.host.querySelectorAll('[role="treeitem"]').length).toBeLessThan(200);
    } finally {
        await h.dispose();
    }
});

it("親を閉じると展開状態を保った子の購読も止め、変更通知で非表示の子を再取得しない", async () => {
    const h = await harness();
    try {
        const tree = h.host.querySelector('[role="tree"]')!;
        await act(async () => tree.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true })));
        await wait(() => expect(document.activeElement?.textContent).toContain("tree"));
        await act(async () => tree.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true })));
        await wait(() =>
            expect(
                get.mock.calls.some(
                    ([route, options]) => route.endsWith("/entries") && options.params.query.path === "/tree",
                ),
            ).toBe(true),
        );
        await act(async () => h.item("Ops").querySelector<HTMLElement>("span[aria-hidden]")!.click());
        await wait(() => expect(h.item("tree")).toBeUndefined());
        get.mockClear();
        await act(async () => refreshLibrary(h.client, "ops"));
        expect(
            get.mock.calls.filter(
                ([route, options]) => route.endsWith("/entries") && options.params.query.path === "/tree",
            ),
        ).toHaveLength(0);
    } finally {
        await h.dispose();
    }
});

it("可視行の探査だけを送り、探査と後続ページが止まっても子一覧を開始する", async () => {
    const h = await harness(false, 22432, true, true);
    try {
        await wait(() => expect(get.mock.calls.filter(([route]) => route.endsWith("/children"))).toHaveLength(1));
        const probe = get.mock.calls.find(([route]) => route.endsWith("/children"))!;
        expect(probe[1].params.query.path).toBe("/folder-00000");
        const tree = h.host.querySelector('[role="tree"]')!;
        await act(async () => tree.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true })));
        await act(async () => tree.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true })));
        await wait(() =>
            expect(
                get.mock.calls.some(
                    ([route, options]) => route.endsWith("/entries") && options.params.query.path === "/folder-00000",
                ),
            ).toBe(true),
        );
        expect(get.mock.calls.filter(([route]) => route.endsWith("/children"))).toHaveLength(1);
    } finally {
        await h.dispose();
    }
});

it("22k件のうち実際に見える行だけを探査し、End後に画面外の行を探査し続けない", async () => {
    const h = await harness(false, 22432, false, "ready");
    try {
        await wait(() =>
            expect(get.mock.calls.filter(([route]) => route.endsWith("/children")).length).toBeGreaterThan(1),
        );
        const initial = get.mock.calls.filter(([route]) => route.endsWith("/children"));
        expect(initial.length).toBeLessThanOrEqual(19);
        expect(initial.every(([, options]) => Number(options.params.query.path.split("-").at(-1)) < 19)).toBe(true);
        const tree = h.host.querySelector('[role="tree"]')!;
        await act(async () => tree.dispatchEvent(new KeyboardEvent("keydown", { key: "End", bubbles: true })));
        await wait(() => expect(document.activeElement?.textContent).toContain("folder-22431"));
        const requests = get.mock.calls.filter(([route]) => route.endsWith("/children"));
        expect(requests.length).toBeLessThan(40);
        await act(async () => h.item("folder-22431").click());
        expect(navigate).toHaveBeenCalled();
    } finally {
        await h.dispose();
    }
});
