// @vitest-environment jsdom
import { QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import { act, type ReactNode } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";

import type { Entry } from "../library/entries";
import { librariesQuery } from "../library/queries";
import { lastSearchLocation } from "../library/searchReturn";
import type { Selection } from "../library/selection";
import { testEntry } from "../testing/entries";
import { stubMatchMedia } from "../testing/preferences";
import { loadRouterAt } from "../testing/router";

const grid = vi.hoisted(() => ({ source: undefined as unknown, place: undefined as unknown }));
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
        source,
        place,
        onOpen,
        onSelectionChange,
    }: {
        source: unknown;
        place?: (entry: Entry) => string;
        onOpen: (entry: Entry) => void;
        onSelectionChange: (selection: Selection) => void;
    }) => {
        grid.source = source;
        grid.place = place;
        return (
            <>
                <button
                    type="button"
                    data-grid
                    onClick={() =>
                        onSelectionChange({
                            items: new Map([
                                [
                                    "/photos/videos/clip.mp4",
                                    {
                                        size: 1,
                                        folder: false,
                                        readonly: false,
                                        entry: testEntry("/photos/videos/clip.mp4"),
                                    },
                                ],
                            ]),
                            anchor: 0,
                        })
                    }
                    onDoubleClick={() =>
                        onOpen({
                            name: "clip.mp4",
                            path: "/photos/videos/clip.mp4",
                            kind: "file",
                            metadata: { size: 1, readonly: false, contentType: "video/mp4" },
                            file: { archive: false },
                        })
                    }
                >
                    clip.mp4
                </button>
                <button
                    type="button"
                    data-folder
                    onClick={() => onOpen({ name: "sub", path: "/photos/videos/sub", kind: "folder" })}
                >
                    sub
                </button>
            </>
        );
    },
}));
vi.mock("../components/List", () => ({ List: () => <span data-list /> }));
vi.mock("../components/Viewer", () => ({
    Viewer: ({
        location,
        source,
        onChange,
    }: {
        location: { view?: string };
        source: unknown;
        onChange: (view: string | undefined) => void;
    }) => (
        <>
            <span data-viewer>{location.view}</span>
            <span data-viewer-source>{JSON.stringify(source)}</span>
            <button type="button" data-next onClick={() => onChange("photos/videos/next.mp4")}>
                次
            </button>
            <button type="button" data-close onClick={() => onChange(undefined)}>
                閉じる
            </button>
        </>
    ),
}));

const libraries = [
    { id: "photos", name: "Photos", readonly: false },
    { id: "videos", name: "Videos", readonly: true },
];

async function harness(href: string) {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    const restore = stubMatchMedia(false);
    const router = await loadRouterAt(href);
    const client = router.options.context.queryClient;
    client.setQueryData(librariesQuery.queryKey, libraries);
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
afterEach(() => {
    grid.source = undefined;
    grid.place = undefined;
});

it("条件がないときは一覧を要求せず、入力を促す", async () => {
    const h = await harness("/search");
    try {
        expect(h.host.querySelector("[data-grid]")).toBeNull();
        expect(h.host.textContent).toContain("検索語を入力するか、評価かタグを選んでください。");
    } finally {
        await h.dispose();
    }
});

it.each(["rating=good", "tag=trip", "rating=none&tag=trip&tag=sea"])(
    "評価かタグだけで絞った検索は、検索語がなくても一覧を出す（%s）",
    async (filter) => {
        const h = await harness(`/search?${filter}`);
        try {
            expect(h.host.querySelector("[data-grid]")).not.toBeNull();
            expect(h.host.textContent).not.toContain("評価かタグを選んでください");
        } finally {
            await h.dispose();
        }
    },
);

it("URL の Library、起点、範囲、条件から、一覧の出どころを作る", async () => {
    const h = await harness("/search?lib=photos&path=%2Fvideos&flat=1&q=clip&rating=good&tag=a&tag=b");
    try {
        expect(grid.source).toEqual({
            kind: "search",
            criteria: {
                libraryIds: ["photos"],
                path: "/videos",
                recursive: false,
                query: "clip",
                rating: "good",
                tags: ["a", "b"],
            },
        });
        expect(grid.place).toBeUndefined();
    } finally {
        await h.dispose();
    }
});

it("Library を指定しなければすべての Library を探し、複数のときは起点を使わず、場所に Library の名前を出す", async () => {
    const h = await harness("/search?q=clip&path=%2Fvideos");
    try {
        expect(grid.source).toMatchObject({
            kind: "search",
            criteria: { libraryIds: ["photos", "videos"], path: "/", recursive: true },
        });
        const place = grid.place as (entry: Entry) => string;
        expect(place(testEntry("/photos/a/b/x.png"))).toBe("Photos/a/b");
        expect(place(testEntry("/videos/x.mp4"))).toBe("Videos");
    } finally {
        await h.dispose();
    }
});

it("Library が 1 つのときの場所は、起点からの相対パスである", async () => {
    const h = await harness("/search?lib=photos&path=%2Fa&q=x");
    try {
        const place = grid.place as (entry: Entry) => string;
        expect(place(testEntry("/photos/a/x.png"))).toBe(".");
        expect(place(testEntry("/photos/a/b/x.png"))).toBe("b");
    } finally {
        await h.dispose();
    }
});

it("ビューアを開き、移り、閉じても検索の条件を残す", async () => {
    const h = await harness("/search?lib=photos&q=clip");
    try {
        await act(async () =>
            h.host.querySelector("[data-grid]")!.dispatchEvent(new MouseEvent("dblclick", { bubbles: true })),
        );
        await wait(() =>
            expect(h.router.state.location.search).toEqual({
                lib: "photos",
                q: "clip",
                view: "photos/videos/clip.mp4",
            }),
        );
        expect(h.host.querySelector("[data-viewer]")?.textContent).toBe("photos/videos/clip.mp4");
        await act(async () => h.host.querySelector<HTMLButtonElement>("[data-next]")!.click());
        await wait(() =>
            expect(h.router.state.location.search).toEqual({
                lib: "photos",
                q: "clip",
                view: "photos/videos/next.mp4",
            }),
        );
        await act(async () => h.host.querySelector<HTMLButtonElement>("[data-close]")!.click());
        await wait(() => expect(h.router.state.location.search).toEqual({ lib: "photos", q: "clip" }));
    } finally {
        await h.dispose();
    }
});

it("フォルダの結果を開くと、フォルダ表示でそのフォルダへ移る", async () => {
    const h = await harness("/search?q=sub");
    try {
        await act(async () => h.host.querySelector<HTMLButtonElement>("[data-folder]")!.click());
        await wait(() => expect(h.router.state.location.pathname).toBe("/browse/photos/videos/sub"));
    } finally {
        await h.dispose();
    }
});

it("条件を変えると選択を外し、view の変更では残す", async () => {
    const h = await harness("/search?lib=photos&q=clip");
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
        await go("/search?lib=photos&q=clip&flat=1");
        expect(h.host.querySelector("footer")?.textContent).toBe("0");
        await select();
        await go("/search?lib=photos&q=clip&flat=1&view=photos%2Fvideos%2Fclip.mp4");
        expect(h.host.querySelector("footer")?.textContent).toBe("1");
        await go("/search?lib=photos&lib=videos&q=clip&flat=1");
        expect(h.host.querySelector("footer")?.textContent).toBe("0");
        await select();
        await go("/search?lib=photos&lib=videos&q=clip&flat=1&tag=x");
        expect(h.host.querySelector("footer")?.textContent).toBe("0");
    } finally {
        await h.dispose();
    }
});

it("開いた検索の条件を覚え、ビューアで開いている Entry は覚えない", async () => {
    const h = await harness("/search?lib=photos&q=clip&tag=a&view=photos%2Fx.png");
    try {
        expect(lastSearchLocation()).toEqual({ libraryIds: ["photos"], path: "/", query: "clip", tags: ["a"] });
    } finally {
        await h.dispose();
    }
});

it("旧来の Library ごとの検索の URL を、その Library を探す検索へ置き換え、view を鍵へ直す", async () => {
    const h = await harness("/search/photos?q=clip&path=%2Fvideos&flat=1&rating=good&tag=trip&view=sub%2Fa.mp4");
    try {
        await wait(() =>
            expect(h.router.state.location.href).toBe(
                "/search?lib=photos&path=%2Fvideos&flat=1&q=clip&rating=good&tag=trip&view=photos%2Fvideos%2Fsub%2Fa.mp4",
            ),
        );
    } finally {
        await h.dispose();
    }
});

it("旧来の検索語つきのフォルダ表示の URL を、その場所を起点にした検索へ置き換える", async () => {
    const direct = await harness("/browse/photos/videos?q=clip");
    try {
        await wait(() =>
            expect(direct.router.state.location.href).toBe("/search?lib=photos&path=%2Fvideos&flat=1&q=clip"),
        );
    } finally {
        await direct.dispose();
    }
    const deep = await harness("/browse/photos?q=clip&deep=1&view=a.mp4");
    try {
        await wait(() => expect(deep.router.state.location.href).toBe("/search?lib=photos&q=clip&view=photos%2Fa.mp4"));
    } finally {
        await deep.dispose();
    }
});
