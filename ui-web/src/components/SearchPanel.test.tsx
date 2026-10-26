// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import type { SearchLocation } from "../location";
import { SearchPanel, SearchTermsField } from "./SearchPanel";

const mocks = vi.hoisted(() => ({ get: vi.fn(), navigate: vi.fn() }));
vi.mock("../api/client", async (importOriginal) => ({
    ...(await importOriginal<typeof import("../api/client")>()),
    api: { GET: mocks.get },
}));
vi.mock("@tanstack/react-router", () => ({ useNavigate: () => mocks.navigate }));
vi.mock("./Tree", () => ({
    Tree: ({
        libraryIds,
        location,
        onSelect,
    }: {
        libraryIds: string[];
        location: { path: string };
        onSelect: (location: { libraryId: string; path: string }) => void;
    }) => (
        <button type="button" data-tree onClick={() => onSelect({ libraryId: libraryIds[0]!, path: "/a/b" })}>
            {libraryIds.join(",")}:{location.path}
        </button>
    ),
}));

const host = document.createElement("div");
const root = createRoot(host);
let client: QueryClient;

beforeEach(() => {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    document.body.append(host);
    client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    mocks.navigate.mockReset();
    mocks.get.mockImplementation(async (path: string) => ({
        data:
            path === "/libraries"
                ? {
                      libraries: [
                          { id: "photos", name: "Photos", readonly: false },
                          { id: "videos", name: "Videos", readonly: true },
                      ],
                  }
                : { tags: ["cat", "trip"] },
        response: { ok: true },
    }));
});
afterEach(async () => {
    await act(async () => root.render(null));
    client.clear();
    host.remove();
});

async function render(search: Partial<SearchLocation> = {}, terms = true) {
    const location: SearchLocation = { path: "/", tags: [], ...search };
    await act(async () =>
        root.render(
            <QueryClientProvider client={client}>
                <SearchPanel search={location} terms={terms} />
            </QueryClientProvider>,
        ),
    );
    await vi.waitFor(async () => {
        await act(async () => {
            await Promise.resolve();
        });
        expect(host.querySelector("#search-library-photos")).not.toBeNull();
        expect(host.querySelector("#search-tag-cat")).not.toBeNull();
    });
}
const click = (selector: string) => act(async () => host.querySelector<HTMLElement>(selector)!.click());
const lastHref = (): string => {
    const call = mocks.navigate.mock.lastCall;
    if (!call) throw new Error("navigate was not called");
    return (call[0] as { href: string }).href;
};

it("Library は、すべてを選んでいるとき指定なしにし、外すと選んだものだけを URL に載せる", async () => {
    await render();
    expect(host.querySelector("#search-library-photos")?.getAttribute("aria-checked")).toBe("true");
    expect(host.querySelector("#search-library-videos")?.getAttribute("aria-checked")).toBe("true");
    await click("#search-library-videos");
    expect(mocks.navigate).toHaveBeenLastCalledWith({ href: "/search?lib=photos", replace: true });
    await render({ libraryIds: ["photos"] });
    await click("#search-library-videos");
    expect(lastHref()).toBe("/search");
});

it("最後の 1 つの Library は外せない", async () => {
    await render({ libraryIds: ["videos"] });
    expect(host.querySelector("#search-library-videos")?.hasAttribute("disabled")).toBe(true);
    expect(host.querySelector("#search-library-photos")?.hasAttribute("disabled")).toBe(false);
});

it("場所は Library が 1 つのときだけ選べ、複数になると全体に戻る", async () => {
    await render();
    expect(host.querySelector("section[aria-label=場所]")?.textContent).toContain("各 Library の全体");
    expect(host.querySelector('button[aria-label="場所を選ぶ"]')).toBeNull();

    await render({ libraryIds: ["photos"], path: "/a" });
    const start = host.querySelector('button[aria-label="場所を選ぶ"]');
    expect(start?.textContent).toBe("/a");
    await click("#search-library-videos");
    expect(lastHref()).toBe("/search");
});

it("場所のポップオーバーでそのフォルダを選ぶと起点にし、Library 全体へ戻せる", async () => {
    await render({ libraryIds: ["photos"], path: "/a" });
    expect(host.querySelector('button[aria-label="場所を選ぶ"]')?.textContent).toBe("/a");
    await click('button[aria-label="場所を選ぶ"]');
    const tree = document.querySelector("[data-tree]");
    expect(tree?.textContent).toBe("photos:/a");
    await act(async () => (tree as HTMLElement).click());
    expect(lastHref()).toBe("/search?lib=photos&path=%2Fa%2Fb");

    await render({ libraryIds: ["photos"], path: "/a" });
    await click('button[aria-label="場所を選ぶ"]');
    const whole = [...document.querySelectorAll("button")].find((button) => button.textContent === "Library 全体");
    await act(async () => whole!.click());
    expect(lastHref()).toBe("/search?lib=photos");
});

it("範囲、評価、タグは、変えるとすぐに URL へ載せ、開いているビューアを閉じる", async () => {
    await render({ libraryIds: ["photos"], query: "x", view: "photos/a.png", tags: ["cat"] });
    await click("#search-flat");
    expect(lastHref()).toBe("/search?lib=photos&flat=1&q=x&tag=cat");
    await click("#search-tag-trip");
    expect(lastHref()).toBe("/search?lib=photos&q=x&tag=cat&tag=trip");
    await click("#search-tag-cat");
    expect(lastHref()).toBe("/search?lib=photos&q=x");
    const bad = [...host.querySelectorAll("input[type=radio]")][2] as HTMLInputElement;
    await act(async () => bad.click());
    expect(lastHref()).toBe("/search?lib=photos&q=x&rating=bad&tag=cat");
    for (const call of mocks.navigate.mock.calls) expect(call[0].replace).toBe(true);
});

it("URL にだけあるタグも一覧に出して外せる", async () => {
    await render({ tags: ["only-in-url"] });
    expect(host.querySelector("#search-tag-only-in-url")?.getAttribute("aria-checked")).toBe("true");
    await click("#search-tag-only-in-url");
    expect(lastHref()).toBe("/search");
});

it("検索語は入力が止まってから URL へ載せ、狭い画面の一覧には出さない", async () => {
    await render({ libraryIds: ["photos"] }, false);
    expect(host.querySelector('input[type="search"]')).toBeNull();
    await act(async () =>
        root.render(
            <QueryClientProvider client={client}>
                <SearchTermsField search={{ libraryIds: ["photos"], path: "/", tags: [] }} />
            </QueryClientProvider>,
        ),
    );
    const input = host.querySelector<HTMLInputElement>('input[type="search"]')!;
    await act(async () => {
        Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, " alpha ");
        input.dispatchEvent(new Event("input", { bubbles: true }));
    });
    expect(mocks.navigate).not.toHaveBeenCalled();
    await vi.waitFor(() =>
        expect(mocks.navigate).toHaveBeenCalledWith({ href: "/search?lib=photos&q=alpha", replace: true }),
    );
});
