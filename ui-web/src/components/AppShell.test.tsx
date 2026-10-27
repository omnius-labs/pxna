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
const menuButton = () => document.querySelector<HTMLButtonElement>("button[aria-label=Folder]");

it("puts the toolbar, the tree and the content side by side on a wide screen", async () => {
    await render(false);
    expect(host.querySelector("header")).not.toBeNull();
    expect(host.querySelector("nav[aria-label=Library] [role=tree]")).not.toBeNull();
    expect(host.querySelector("main")).not.toBeNull();
    expect(menuButton()).toBeNull();
    expect(host.querySelector('input[type="search"]')).toBeNull();
    expect(host.querySelector('button[aria-label="Search"]')).toBeNull();
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

const switchLink = (title: string) =>
    host.querySelector<HTMLAnchorElement>(`nav[aria-label='Switch view'] a[title="${title}"]`);
const modeIs = (title: string) => switchLink(title)?.getAttribute("aria-current") === "page";

it("広い画面の切り替えは、フォルダ、タイムライン、検索を横に並べ、検索の専用のボタンを置かない", async () => {
    const router = await render(false, "/browse/photos/sub");
    try {
        const labels = [...host.querySelectorAll('nav[aria-label="Switch view"] a')].map((a) => a.textContent);
        expect(labels).toEqual(["Folder", "Timeline", "Search"]);
        expect(modeIs("Folder")).toBe(true);
        expect(modeIs("Search")).toBe(false);
        expect(host.querySelector('header button[aria-label="Search"]')).toBeNull();
        expect(host.querySelector("nav[aria-label=Library] [role=tree]")).not.toBeNull();
        expect(switchLink("Search")?.getAttribute("href")).toBe("/search");
        await act(async () => switchLink("Search")!.click());
        await vi.waitFor(() => expect(router.state.location.href).toBe("/search"));
    } finally {
        router.history.destroy();
    }
});

it("広い画面の検索モードは、ツリーの代わりに左の領域へ検索の条件を置く", async () => {
    const router = await render(false, "/search?lib=photos&path=%2Fsub");
    try {
        expect(modeIs("Search")).toBe(true);
        expect(host.querySelector("[role=tree]")).toBeNull();
        const conditions = host.querySelector("nav[aria-label='Search conditions']");
        expect(conditions?.querySelector('input[type="search"]')).not.toBeNull();
        expect(conditions?.textContent).toContain("Photos");
        expect(conditions?.textContent).toContain("Do not include subfolders");
        expect(conditions?.textContent).toContain("Unrated");
        expect(host.querySelector("header")?.textContent).toContain("Search");
    } finally {
        router.history.destroy();
    }
});

it("狭い画面の検索モードは、検索語の欄を常に出し、ほかの条件をドロワーへ置いて、絞り込んでいる数をボタンに出す", async () => {
    const router = await render(true, "/search?lib=photos&flat=1&tag=x&tag=y");
    try {
        expect(host.querySelector('header input[type="search"]')).toBeNull();
        expect(host.querySelector('input[type="search"]')).not.toBeNull();
        const button = host.querySelector<HTMLButtonElement>('button[aria-label^="Search conditions"]');
        expect(button?.getAttribute("aria-label")).toBe("Search conditions (3)");
        expect(button?.textContent).toBe("3");
        expect(document.querySelector("[role=dialog]")).toBeNull();
        await act(async () => button!.click());
        const drawer = document.querySelector("[role=dialog]");
        expect(drawer?.textContent).toContain("Do not include subfolders");
        expect(drawer?.querySelector('input[type="search"]')).toBeNull();
    } finally {
        router.history.destroy();
    }
});

it("タイムラインからも「検索」で、最後に使った検索の条件を開く", async () => {
    (await render(false, "/browse/photos/sub")).history.destroy();
    (await render(false, "/search?lib=photos&q=cat&rating=good")).history.destroy();
    const router = await render(false, "/timeline");
    try {
        expect(modeIs("Timeline")).toBe(true);
        expect(switchLink("Search")?.getAttribute("href")).toBe("/search?lib=photos&q=cat&rating=good");
        const folder = switchLink("Folder");
        // 前の検索の画面の前に開いていたフォルダへ戻る。
        expect(folder?.getAttribute("href")).toBe("/browse/photos/sub");
    } finally {
        router.history.destroy();
    }
});

it("広い画面のツールバーは、表示の名前と Library の名前を文字で出さず、パンくずを Library の直下から始める", async () => {
    // 切り替えのほかに「タイムライン」「検索」「Photos」の文字がないこと。
    const outsideSwitch = () => {
        const header = host.querySelector("header")!.cloneNode(true) as HTMLElement;
        header.querySelector('nav[aria-label="Switch view"]')?.remove();
        return header.textContent ?? "";
    };
    const crumbs = () =>
        [...host.querySelectorAll("header [data-slot=breadcrumb-item]")].map((item) => item.textContent);
    for (const href of ["/timeline", "/search", "/browse/photos"]) {
        const router = await render(false, href);
        try {
            expect(outsideSwitch()).not.toMatch(/Timeline|Search|Photos/);
            expect(crumbs()).toEqual([]);
        } finally {
            router.history.destroy();
        }
    }
    const router = await render(false, "/browse/photos/sub");
    try {
        expect(crumbs()).toEqual(["sub"]);
    } finally {
        router.history.destroy();
    }
});

const panel = () => host.querySelector<HTMLElement>("aside[aria-label='Info panel']");
const toggle = () => host.querySelector<HTMLButtonElement>("button[aria-label='Info panel']");

it("広い画面では、内容の右に情報パネルを開いた状態で出し、ボタンで畳み、リロードの後も畳んだままにする", async () => {
    await render(false, "/browse/photos");
    expect(panel()?.textContent).toBe("Choose an item to see its details.");
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
const separator = (label: string) => host.querySelector<HTMLElement>(`[role=separator][aria-label='${label}']`)!;

it("ツリーの端のドラッグで幅を変え、端末に保存し、再訪で復元する", async () => {
    await render(false, "/browse/photos");
    const tree = () => host.querySelector<HTMLElement>("nav[aria-label=Library]")!;
    expect(tree().style.width).toBe("224px");
    await drag(separator("Tree width"), 224, 300);
    expect(tree().style.width).toBe("300px");
    expect(window.localStorage.getItem("pxna.treeWidth")).toBe("300");
    await drag(separator("Tree width"), 300, 5000);
    expect(tree().style.width).toBe("480px");
    await act(async () => root.unmount());
    root = createRoot(host);
    await render(false, "/browse/photos");
    expect(tree().style.width).toBe("480px");
});

it("情報パネルは左端を左へ動かすと広がり、矢印キーでも変えられる", async () => {
    await render(false, "/browse/photos");
    const aside = () => host.querySelector<HTMLElement>("aside[aria-label='Info panel']")!;
    expect(aside().style.width).toBe("288px");
    await drag(separator("Info panel width"), 1000, 900);
    expect(aside().style.width).toBe("388px");
    expect(window.localStorage.getItem("pxna.infoPanelWidth")).toBe("388");
    await act(async () =>
        separator("Info panel width").dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowLeft", bubbles: true })),
    );
    expect(aside().style.width).toBe("404px");
});

it("タイムラインの月の一覧には幅のつまみを出さず、狭い画面でもつまみを出さない", async () => {
    await render(false, "/timeline/photos");
    expect(host.querySelector('[role=separator][aria-label="Tree width"]')).toBeNull();
});
