// @vitest-environment jsdom
import { QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import { act, type ReactNode } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";

import type { Entry } from "../library/entries";
import { preferencesQuery } from "../library/preferences";
import { librariesQuery } from "../library/queries";
import type { Selection } from "../library/selection";
import type { BrowseLocation } from "../location";
import { testEntry } from "../testing/entries";
import { stubMatchMedia } from "../testing/preferences";
import { loadRouterAt } from "../testing/router";

vi.mock("../components/AppShell", () => ({
    AppShell: ({ children, footer }: { children: ReactNode; footer: ReactNode }) => (
        <>
            {children}
            {footer}
        </>
    ),
}));
vi.mock("../components/StatusBar", () => ({
    StatusBar: ({ selectedCount }: { selectedCount: number }) => <footer>{selectedCount}</footer>,
}));
vi.mock("../components/Grid", () => ({
    Grid: ({
        onOpen,
        selection,
        onSelectionChange,
    }: {
        onOpen: (entry: Entry) => void;
        selection: Selection;
        onSelectionChange: (selection: Selection) => void;
    }) => (
        <button
            type="button"
            data-grid
            data-anchor={selection.anchor}
            onClick={() =>
                onSelectionChange({
                    items: new Map([
                        [
                            "/videos/clip.mp4",
                            { size: 1, folder: false, readonly: false, entry: testEntry("/videos/clip.mp4") },
                        ],
                    ]),
                    anchor: 42,
                })
            }
            onDoubleClick={() =>
                onOpen({
                    name: "clip.mp4",
                    path: "/videos/clip.mp4",
                    kind: "file",
                    metadata: { size: 1, readonly: false, contentType: "video/mp4" },
                    file: { archive: false },
                })
            }
        >
            clip.mp4
        </button>
    ),
}));
vi.mock("../components/List", () => ({ List: () => <span data-list /> }));
vi.mock("../components/Viewer", () => ({
    Viewer: ({ location, onChange }: { location: BrowseLocation; onChange: (view: string | undefined) => void }) => (
        <>
            <span data-viewer>{location.view}</span>
            <button type="button" data-next onClick={() => onChange("sub/clip.mp4")}>
                次
            </button>
            <button type="button" data-close onClick={() => onChange(undefined)}>
                閉じる
            </button>
        </>
    ),
}));

async function harness(href = "/browse/photos/videos", loaded = true) {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    const restore = stubMatchMedia(false);
    const router = await loadRouterAt(href);
    const client = router.options.context.queryClient;
    if (loaded) client.setQueryData(librariesQuery.queryKey, [{ id: "photos", name: "Photos", readonly: false }]);
    await act(async () =>
        root.render(
            <QueryClientProvider client={client}>
                <RouterProvider router={router} />
            </QueryClientProvider>,
        ),
    );
    return {
        host,
        router,
        client,
        dispose: async () => {
            await act(async () => root.unmount());
            host.remove();
            router.history.destroy();
            client.clear();
            restore();
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
it("opens a video tile through BrowsePage into the viewer URL", async () => {
    const h = await harness();
    try {
        await act(async () =>
            h.host.querySelector("button")!.dispatchEvent(new MouseEvent("dblclick", { bubbles: true })),
        );
        await wait(() => expect(h.router.state.location.search).toEqual({ view: "clip.mp4" }));
        expect(h.host.querySelector("[data-viewer]")?.textContent).toBe("clip.mp4");
    } finally {
        await h.dispose();
    }
});
it("switches the listing and clears selection when the view changes", async () => {
    const h = await harness();
    try {
        await act(async () => h.host.querySelector<HTMLButtonElement>("[data-grid]")!.click());
        expect(h.host.querySelector("footer")?.textContent).toBe("1");
        await act(async () => {
            h.client.setQueryData(preferencesQuery.queryKey, (current) => ({
                ...current!,
                wide: { ...current!.wide, view: "list" as const },
            }));
        });
        await wait(() => expect(h.host.querySelector("[data-list]")).not.toBeNull());
        expect(h.host.querySelector("[data-grid]")).toBeNull();
        expect(h.host.querySelector("footer")?.textContent).toBe("0");
    } finally {
        await h.dispose();
    }
});
it("preserves selected paths and clears only the anchor on sorting", async () => {
    const h = await harness();
    try {
        await act(async () => h.host.querySelector<HTMLButtonElement>("[data-grid]")!.click());
        expect(h.host.querySelector("[data-grid]")?.getAttribute("data-anchor")).toBe("42");
        await act(async () => {
            h.client.setQueryData(preferencesQuery.queryKey, (current) => ({
                ...current!,
                sort: { key: "size" as const, order: "desc" as const },
            }));
        });
        await wait(() => expect(h.host.querySelector("[data-grid]")?.getAttribute("data-anchor")).toBeNull());
        expect(h.host.querySelector("footer")?.textContent).toBe("1");
    } finally {
        await h.dispose();
    }
});

it("ビューアを開き、移り、閉じても検索の条件を残す", async () => {
    const h = await harness("/search/photos?q=clip&path=%2Fvideos");
    try {
        await act(async () =>
            h.host.querySelector("[data-grid]")!.dispatchEvent(new MouseEvent("dblclick", { bubbles: true })),
        );
        await wait(() =>
            expect(h.router.state.location.search).toEqual({ q: "clip", path: "/videos", view: "clip.mp4" }),
        );
        await act(async () => h.host.querySelector<HTMLButtonElement>("[data-next]")!.click());
        await wait(() =>
            expect(h.router.state.location.search).toEqual({ q: "clip", path: "/videos", view: "sub/clip.mp4" }),
        );
        await act(async () => h.host.querySelector<HTMLButtonElement>("[data-close]")!.click());
        await wait(() => expect(h.router.state.location.search).toEqual({ q: "clip", path: "/videos" }));
    } finally {
        await h.dispose();
    }
});

it("検索語、範囲、検索の終了では選択を外し、view の変更では残す", async () => {
    const h = await harness("/search/photos?q=clip&path=%2Fvideos");
    const select = async () => {
        await act(async () => h.host.querySelector<HTMLButtonElement>("[data-grid]")!.click());
        expect(h.host.querySelector("footer")?.textContent).toBe("1");
    };
    const go = async (href: string) => {
        await act(async () => h.router.navigate({ href }));
        await wait(() => expect(h.router.state.location.href).toBe(href));
    };
    try {
        await select();
        await go("/search/photos?q=clip&path=%2Fvideos&flat=1");
        expect(h.host.querySelector("footer")?.textContent).toBe("0");
        await select();
        await go("/search/photos?q=clip&path=%2Fvideos&flat=1&view=clip.mp4");
        expect(h.host.querySelector("footer")?.textContent).toBe("1");
        await go("/search/photos?q=other&path=%2Fvideos&flat=1");
        expect(h.host.querySelector("footer")?.textContent).toBe("0");
        await select();
        await go("/browse/photos/videos");
        expect(h.host.querySelector("footer")?.textContent).toBe("0");
    } finally {
        await h.dispose();
    }
});

it("検索語がないときは一覧を要求せず、入力を促す", async () => {
    const h = await harness("/search/photos?path=%2Fvideos");
    try {
        expect(h.host.querySelector("[data-grid]")).toBeNull();
        expect(h.host.textContent).toContain("検索語を入力するか、評価かタグを選んでください。");
    } finally {
        await h.dispose();
    }
});

it.each(["rating=good", "tag=trip", "rating=none&tag=trip"])(
    "評価かタグだけで絞った検索は、検索語がなくても一覧を出す（%s）",
    async (filter) => {
        const h = await harness(`/search/photos?path=%2Fvideos&${filter}`);
        try {
            expect(h.host.querySelector("[data-grid]")).not.toBeNull();
            expect(h.host.textContent).not.toContain("評価かタグを選んでください");
        } finally {
            await h.dispose();
        }
    },
);

it("旧来の検索の URL を、検索モードの対応する起点と範囲へ置き換える", async () => {
    const direct = await harness("/browse/photos/videos?q=clip");
    try {
        await wait(() => expect(direct.router.state.location.href).toBe("/search/photos?q=clip&path=%2Fvideos&flat=1"));
    } finally {
        await direct.dispose();
    }
    const deep = await harness("/browse/photos?q=clip&deep=1&view=a.mp4");
    try {
        await wait(() => expect(deep.router.state.location.href).toBe("/search/photos?q=clip&view=a.mp4"));
    } finally {
        await deep.dispose();
    }
});

it("Library の取得前は選択を始めず、取得後に一覧を出す", async () => {
    const h = await harness("/browse/photos/videos", false);
    try {
        expect(h.host.querySelector("[data-grid]")).toBeNull();
        await act(async () => {
            h.client.setQueryData(librariesQuery.queryKey, [{ id: "photos", name: "Photos", readonly: false }]);
        });
        await wait(() => expect(h.host.querySelector("[data-grid]")).not.toBeNull());
    } finally {
        await h.dispose();
    }
});
