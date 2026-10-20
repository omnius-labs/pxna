// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import type { SearchLocation } from "../location";
import { SearchBox } from "./SearchBox";

const { navigate, get } = vi.hoisted(() => ({ navigate: vi.fn(), get: vi.fn() }));
vi.mock("../api/client", async (original) => ({
    ...(await original<typeof import("../api/client")>()),
    api: { GET: get },
}));
vi.mock("@tanstack/react-router", () => ({ useNavigate: () => navigate }));
beforeEach(() => {
    vi.useFakeTimers();
    navigate.mockReset();
    get.mockResolvedValue({ data: { tags: ["cat", "trip"] }, response: new Response() });
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
});
afterEach(() => vi.useRealTimers());

async function harness(search: SearchLocation = { libraryId: "photos", path: "/a" }, shortcut = true) {
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    const client = new QueryClient();
    const render = async (search: SearchLocation) => {
        await act(async () =>
            root.render(
                <QueryClientProvider client={client}>
                    <SearchBox search={search} libraryName="Photos" shortcut={shortcut} />
                </QueryClientProvider>,
            ),
        );
    };
    await render(search);
    const input = () => host.querySelector<HTMLInputElement>('input[type="search"]')!;
    const fill = async (value: string) => {
        await act(async () => {
            Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input(), value);
            input().dispatchEvent(new Event("input", { bubbles: true }));
        });
    };
    const tick = async (ms: number) => {
        await act(async () => vi.advanceTimersByTimeAsync(ms));
    };
    return {
        host,
        input,
        fill,
        tick,
        render,
        select: () => host.querySelector<HTMLSelectElement>("select")!,
        check: async () => {
            await act(async () => host.querySelector<HTMLInputElement>('input[type="checkbox"]')!.click());
        },
        dispose: async () => {
            await act(async () => root.unmount());
            host.remove();
        },
    };
}

it("連続する入力が 300 ms 止まった後に、最後の語だけで URL を置き換える", async () => {
    const h = await harness();
    try {
        await h.fill("a");
        await h.tick(299);
        expect(navigate).not.toHaveBeenCalled();
        await h.fill("alpha");
        await h.tick(299);
        expect(navigate).not.toHaveBeenCalled();
        await h.tick(1);
        expect(navigate).toHaveBeenCalledExactlyOnceWith({ href: "/search/photos?q=alpha&path=%2Fa", replace: true });
    } finally {
        await h.dispose();
    }
});
it.each(["", "   "])("空の語で、検索語だけを URL から外す %s", async (value) => {
    const h = await harness({ libraryId: "photos", path: "/a", query: "alpha", flat: true });
    try {
        await h.fill(value);
        await h.tick(300);
        expect(navigate).toHaveBeenCalledExactlyOnceWith({
            href: "/search/photos?path=%2Fa&flat=1",
            replace: true,
        });
    } finally {
        await h.dispose();
    }
});
it("範囲の切り替えは語が空でもすぐ URL に反映する", async () => {
    const h = await harness();
    try {
        await h.check();
        expect(navigate).toHaveBeenLastCalledWith({ href: "/search/photos?path=%2Fa&flat=1", replace: true });
        await h.render({ libraryId: "photos", path: "/a", query: "alpha", flat: true });
        await h.check();
        expect(navigate).toHaveBeenLastCalledWith({ href: "/search/photos?q=alpha&path=%2Fa", replace: true });
    } finally {
        await h.dispose();
    }
});
it("起点の選択は、Library 全体と今の起点の祖先を並べ、語と範囲を保って置き換える", async () => {
    const h = await harness({ libraryId: "photos", path: "/a/b", query: "alpha", flat: true });
    try {
        expect([...h.select().options].map((option) => [option.value, option.textContent])).toEqual([
            ["/", "Photos（全体）"],
            ["/a", "a の配下"],
            ["/a/b", "b の配下"],
        ]);
        expect(h.select().value).toBe("/a/b");
        await act(async () => {
            h.select().value = "/";
            h.select().dispatchEvent(new Event("change", { bubbles: true }));
        });
        expect(navigate).toHaveBeenLastCalledWith({ href: "/search/photos?q=alpha&flat=1", replace: true });
    } finally {
        await h.dispose();
    }
});
it("検索を終えると、起点のフォルダ表示へ移る", async () => {
    const h = await harness({ libraryId: "photos", path: "/a", query: "alpha" });
    try {
        await act(async () => h.host.querySelector<HTMLButtonElement>('button[aria-label="検索を終える"]')!.click());
        expect(navigate).toHaveBeenLastCalledWith({ href: "/browse/photos/a" });
    } finally {
        await h.dispose();
    }
});
it("自分の URL 置き換えでは末尾の空白を保ち、外からの位置変更に追従する", async () => {
    const h = await harness();
    try {
        await h.fill("alpha ");
        await h.tick(300);
        await h.render({ libraryId: "photos", path: "/a", query: "alpha" });
        expect(h.input().value).toBe("alpha ");
        await h.fill("alpha  ");
        await h.tick(300);
        expect(navigate).toHaveBeenCalledTimes(1);
        await h.render({ libraryId: "photos", path: "/a", query: "notes", flat: true });
        expect(h.input().value).toBe("notes");
        expect(h.host.querySelector<HTMLInputElement>('input[type="checkbox"]')!.checked).toBe(true);
    } finally {
        await h.dispose();
    }
});
it("外からの位置変更で、未反映の入力の timer を取り消す", async () => {
    const h = await harness();
    try {
        await h.fill("old");
        await h.tick(200);
        await h.render({ libraryId: "photos", path: "/a", query: "new" });
        await h.tick(500);
        expect(navigate).not.toHaveBeenCalled();
        expect(h.input().value).toBe("new");
    } finally {
        await h.dispose();
    }
});
it("評価の選択は、すぐ URL に反映し、語と起点を保つ。すべてに戻すと外す", async () => {
    const h = await harness({ libraryId: "photos", path: "/a", query: "alpha" });
    try {
        const select = h.host.querySelector<HTMLSelectElement>('select[aria-label="評価"]')!;
        expect([...select.options].map((option) => option.value)).toEqual(["", "good", "bad", "none"]);
        await act(async () => {
            select.value = "good";
            select.dispatchEvent(new Event("change", { bubbles: true }));
        });
        expect(navigate).toHaveBeenLastCalledWith({
            href: "/search/photos?q=alpha&rating=good&path=%2Fa",
            replace: true,
        });
        await h.render({ libraryId: "photos", path: "/a", query: "alpha", rating: "good" });
        expect(h.host.querySelector<HTMLSelectElement>('select[aria-label="評価"]')!.value).toBe("good");
        await act(async () => {
            h.host.querySelector<HTMLSelectElement>('select[aria-label="評価"]')!.value = "";
            h.host
                .querySelector<HTMLSelectElement>('select[aria-label="評価"]')!
                .dispatchEvent(new Event("change", { bubbles: true }));
        });
        expect(navigate).toHaveBeenLastCalledWith({ href: "/search/photos?q=alpha&path=%2Fa", replace: true });
    } finally {
        await h.dispose();
    }
});
it("タグは入力が止まってから URL に反映し、これまでの語を補完に出し、空にすると外す", async () => {
    const h = await harness({ libraryId: "photos", path: "/a" });
    try {
        await vi.waitFor(() => expect(h.host.querySelectorAll("datalist option")).toHaveLength(2));
        const tag = () => h.host.querySelector<HTMLInputElement>('input[aria-label="タグ"]')!;
        const type = async (value: string) => {
            await act(async () => {
                Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(tag(), value);
                tag().dispatchEvent(new Event("input", { bubbles: true }));
            });
        };
        await type(" trip ");
        await h.tick(299);
        expect(navigate).not.toHaveBeenCalled();
        await h.tick(1);
        expect(navigate).toHaveBeenCalledExactlyOnceWith({ href: "/search/photos?tag=trip&path=%2Fa", replace: true });
        await h.render({ libraryId: "photos", path: "/a", tag: "trip" });
        await type("");
        await h.tick(300);
        expect(navigate).toHaveBeenLastCalledWith({ href: "/search/photos?path=%2Fa", replace: true });
    } finally {
        await h.dispose();
    }
});
it("開いたときに検索語の欄へフォーカスする", async () => {
    const h = await harness();
    try {
        expect(document.activeElement).toBe(h.input());
    } finally {
        await h.dispose();
    }
});
it("広い画面で入力欄とダイアログの外の / だけを拾う", async () => {
    const h = await harness();
    try {
        const outside = document.createElement("button");
        h.host.append(outside);
        outside.focus();
        const slash = new KeyboardEvent("keydown", { key: "/", bubbles: true, cancelable: true });
        outside.dispatchEvent(slash);
        expect(document.activeElement).toBe(h.input());
        expect(slash.defaultPrevented).toBe(true);
        expect(h.input().value).toBe("");
        const inside = new KeyboardEvent("keydown", { key: "/", bubbles: true, cancelable: true });
        h.input().dispatchEvent(inside);
        expect(inside.defaultPrevented).toBe(false);
        const dialog = document.createElement("div");
        dialog.setAttribute("role", "dialog");
        const button = document.createElement("button");
        dialog.append(button);
        h.host.append(dialog);
        button.focus();
        button.dispatchEvent(new KeyboardEvent("keydown", { key: "/", bubbles: true }));
        expect(document.activeElement).toBe(button);
    } finally {
        await h.dispose();
    }
});
it("狭い画面では / を拾わない", async () => {
    const h = await harness(undefined, false);
    try {
        const button = document.createElement("button");
        h.host.append(button);
        button.focus();
        button.dispatchEvent(new KeyboardEvent("keydown", { key: "/", bubbles: true }));
        expect(document.activeElement).toBe(button);
    } finally {
        await h.dispose();
    }
});
