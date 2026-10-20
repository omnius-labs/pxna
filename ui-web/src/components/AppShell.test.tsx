// @vitest-environment jsdom
import { QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import { stubMatchMedia, stubResizeObserver } from "../testing/preferences";
import { loadRouterAt } from "../testing/router";

const mocks = vi.hoisted(() => ({ get: vi.fn() }));

vi.mock("../api/client", async (importOriginal) => ({
    ...(await importOriginal<typeof import("../api/client")>()),
    api: { GET: mocks.get },
}));

let host: HTMLDivElement;
let root: Root;
let restore: () => void = () => {};
let restoreObserver: () => void = () => {};

beforeEach(() => {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    window.localStorage.clear();
    restoreObserver = stubResizeObserver();
    mocks.get.mockImplementation(async (path: string) => ({
        data:
            path === "/libraries"
                ? { libraries: [{ id: "photos", name: "Photos", readonly: false }] }
                : { entries: [], totalCount: 0, snapshot: "s", truncated: false },
        response: { ok: true },
    }));
    host = document.createElement("div");
    document.body.append(host);
    root = createRoot(host);
});

afterEach(async () => {
    await act(async () => root.unmount());
    host.remove();
    restore();
    restoreObserver();
});

async function render(narrow: boolean, href = "/") {
    restore = stubMatchMedia(narrow);
    const router = await loadRouterAt(href);
    await act(async () =>
        root.render(
            <QueryClientProvider client={router.options.context.queryClient}>
                <RouterProvider router={router} />
            </QueryClientProvider>,
        ),
    );
    await act(async () => {
        await new Promise((done) => setTimeout(done, 0));
    });
    return router;
}

const trees = () => document.querySelectorAll("[role=tree]").length;
const menuButton = () => document.querySelector<HTMLButtonElement>("button[aria-label=フォルダ]");

it("puts the toolbar, the tree and the content side by side on a wide screen", async () => {
    await render(false);
    expect(host.querySelector("header")).not.toBeNull();
    expect(host.querySelector("nav[aria-label=Library] [role=tree]")).not.toBeNull();
    expect(host.querySelector("main")).not.toBeNull();
    expect(menuButton()).toBeNull();
    expect(host.querySelector('input[type="search"]')).toBeNull();
    expect(host.querySelector('button[aria-label="検索"]')).toBeNull();
});

it("folds the tree into a drawer on a narrow screen, opened by the menu button and closed with Escape", async () => {
    await render(true);
    expect(trees()).toBe(0);
    expect(menuButton()).not.toBeNull();

    await act(async () => menuButton()!.click());
    expect(document.querySelector("[role=dialog] [role=tree]")).not.toBeNull();

    await act(async () => {
        document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    });
    expect(trees()).toBe(0);
});

it("狭い画面の検索ボタンで検索モードへ移り、検索の段を常に出す", async () => {
    const router = await render(true, "/browse/photos/sub");
    try {
        const input = () => host.querySelector<HTMLInputElement>('input[type="search"]');
        expect(input()).toBeNull();
        await act(async () => host.querySelector<HTMLButtonElement>('button[aria-label="検索"]')!.click());
        await vi.waitFor(() => expect(router.state.location.href).toBe("/search/photos?path=%2Fsub"));
        expect(input()).not.toBeNull();
        expect(host.querySelector('button[aria-label="検索"]')).toBeNull();
        await act(async () => host.querySelector<HTMLButtonElement>('button[aria-label="検索を終える"]')!.click());
        await vi.waitFor(() => expect(router.state.location.href).toBe("/browse/photos/sub"));
        expect(input()).toBeNull();
    } finally {
        router.history.destroy();
    }
});
it("広い画面のフォルダには検索ボタンだけを置き、検索モードでは検索の操作を置く", async () => {
    const router = await render(false, "/browse/photos/sub");
    try {
        expect(host.querySelector('header input[type="search"]')).toBeNull();
        await act(async () => host.querySelector<HTMLButtonElement>('header button[aria-label="検索"]')!.click());
        await vi.waitFor(() => expect(router.state.location.href).toBe("/search/photos?path=%2Fsub"));
        expect(host.querySelector('header input[type="search"]')).not.toBeNull();
        expect(host.querySelector("header")?.textContent).toContain("サブフォルダを含めない");
        expect(host.querySelector<HTMLSelectElement>("header select")?.value).toBe("/sub");
    } finally {
        router.history.destroy();
    }
});
it("タイムラインの検索ボタンは、Library 全体を起点にした検索モードへ移る", async () => {
    const router = await render(false, "/timeline/photos");
    try {
        await act(async () => host.querySelector<HTMLButtonElement>('header button[aria-label="検索"]')!.click());
        await vi.waitFor(() => expect(router.state.location.href).toBe("/search/photos"));
    } finally {
        router.history.destroy();
    }
});

const panel = () => host.querySelector<HTMLElement>("aside[aria-label=情報パネル]");
const toggle = () => host.querySelector<HTMLButtonElement>("button[aria-label=情報パネル]");

it("広い画面では、内容の右に情報パネルを開いた状態で出し、ボタンで畳み、リロードの後も畳んだままにする", async () => {
    await render(false, "/browse/photos");
    expect(panel()?.textContent).toBe("Entry を選ぶと、詳細が出ます。");
    expect(host.querySelector("main")?.nextElementSibling?.nextElementSibling).toBe(panel());
    expect(toggle()?.getAttribute("aria-pressed")).toBe("true");

    await act(async () => toggle()?.click());
    expect(panel()).toBeNull();
    expect(toggle()?.getAttribute("aria-pressed")).toBe("false");
    expect(window.localStorage.getItem("pxna.infoPanelOpen")).toBe("false");

    // リロードに当たる、描き直し。
    await act(async () => root.unmount());
    root = createRoot(host);
    await render(false, "/browse/photos");
    expect(panel()).toBeNull();

    await act(async () => toggle()?.click());
    expect(panel()).not.toBeNull();
    expect(window.localStorage.getItem("pxna.infoPanelOpen")).toBe("true");
});

it("狭い画面では、情報パネルもボタンも出さない", async () => {
    await render(true, "/browse/photos");
    expect(panel()).toBeNull();
    expect(toggle()).toBeNull();
});

const drag = async (handle: HTMLElement, from: number, to: number) => {
    const fire = (type: string, clientX: number) =>
        act(async () => handle.dispatchEvent(new MouseEvent(type, { bubbles: true, clientX })));
    await fire("pointerdown", from);
    await fire("pointermove", to);
    await fire("pointerup", to);
};
const separator = (label: string) => host.querySelector<HTMLElement>(`[role=separator][aria-label="${label}"]`)!;

it("ツリーの端のドラッグで幅を変え、端末に保存し、再訪で復元する", async () => {
    await render(false, "/browse/photos");
    const tree = () => host.querySelector<HTMLElement>("nav[aria-label=Library]")!;
    expect(tree().style.width).toBe("224px");
    await drag(separator("ツリーの幅"), 224, 300);
    expect(tree().style.width).toBe("300px");
    expect(window.localStorage.getItem("pxna.treeWidth")).toBe("300");
    await drag(separator("ツリーの幅"), 300, 5000);
    expect(tree().style.width).toBe("480px");
    await act(async () => root.unmount());
    root = createRoot(host);
    await render(false, "/browse/photos");
    expect(tree().style.width).toBe("480px");
});

it("情報パネルは左端を左へ動かすと広がり、矢印キーでも変えられる", async () => {
    await render(false, "/browse/photos");
    const aside = () => host.querySelector<HTMLElement>("aside[aria-label=情報パネル]")!;
    expect(aside().style.width).toBe("288px");
    await drag(separator("情報パネルの幅"), 1000, 900);
    expect(aside().style.width).toBe("388px");
    expect(window.localStorage.getItem("pxna.infoPanelWidth")).toBe("388");
    await act(async () =>
        separator("情報パネルの幅").dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowLeft", bubbles: true })),
    );
    expect(aside().style.width).toBe("404px");
});

it("タイムラインの月の一覧には幅のつまみを出さず、狭い画面でもつまみを出さない", async () => {
    await render(false, "/timeline/photos");
    expect(host.querySelector('[role=separator][aria-label="ツリーの幅"]')).toBeNull();
});
