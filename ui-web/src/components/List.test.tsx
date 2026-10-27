// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";
import { ImageWants } from "../images/wants";
import { type Entry, entryPlace, type FolderListing } from "../library/entries";
import type { ListingSource } from "../library/listing";
import type { SortPreference } from "../library/preferences";
import { emptySelection } from "../library/selection";
import { CROSS_LIBRARY_SCOPE, entryAddress } from "../location";
import { List } from "./List";

const { get } = vi.hoisted(() => ({ get: vi.fn() }));
vi.mock("../api/client", async (original) => ({
    ...(await original<typeof import("../api/client")>()),
    api: { GET: get },
}));
vi.mock("@tanstack/react-virtual", () => ({
    useVirtualizer: (options: { count: number; estimateSize: () => number }) => ({
        range: { startIndex: 0, endIndex: 1 },
        scrollDirection: null,
        measure: () => {},
        scrollToIndex: () => {},
        getTotalSize: () => options.count * options.estimateSize(),
        getVirtualItems: () =>
            [0, 1].map((index) => ({
                index,
                key: index,
                start: index * options.estimateSize(),
                size: options.estimateSize(),
            })),
    }),
}));

const listPage = (): FolderListing => ({
    entries: [0, 1].map((i) => ({
        name: `f${i}.txt`,
        path: i === 0 ? "/base/f0.txt" : "/base/deep/f1.txt",
        kind: "file",
        metadata: { size: 123, readonly: false, modifiedAt: "2026-10-05T01:02:03Z" },
    })),
    totalCount: 2500,
    snapshot: "one",
    truncated: false,
});
const searchSource = (recursive: boolean, query = "txt"): ListingSource => ({
    kind: "search",
    criteria: { libraryIds: ["search"], path: "/base", recursive, query, tags: [] },
});
const placeUnder = (entry: Entry) =>
    entryPlace("/base", { ...entry, path: entryAddress(CROSS_LIBRARY_SCOPE, entry.path).path });
/// 検索の応答。Entry は Library の id を添える。
const searchPage = (page: FolderListing) => ({
    entries: page.entries.map((entry) => ({ libraryId: "search", entry })),
    totalCount: page.totalCount,
    snapshot: page.snapshot,
    truncatedLibraryIds: page.truncated ? ["search"] : [],
});
const folderSource: ListingSource = { kind: "folder", libraryId: "big", path: "/" };
async function harness(
    narrow: boolean,
    source: ListingSource = folderSource,
    data: FolderListing | ReturnType<typeof searchPage> | null = listPage(),
    place?: (entry: Entry) => string,
) {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    get.mockImplementation(() =>
        data === null ? new Promise(() => {}) : Promise.resolve({ data, response: new Response() }),
    );
    const client = new QueryClient();
    const host = document.createElement("div");
    const actualRoot = createRoot(host);
    const onSort = vi.fn();
    const render = async (sort: SortPreference = { key: "name", order: "asc" }) => {
        await act(async () =>
            actualRoot.render(
                <QueryClientProvider client={client}>
                    <List
                        source={source}
                        place={place}
                        shape="crop"
                        narrow={narrow}
                        sort={sort}
                        onSort={onSort}
                        selection={emptySelection}
                        onSelectionChange={() => {}}
                        onTotal={() => {}}
                        onProgress={() => {}}
                        onOpen={() => {}}
                    />
                </QueryClientProvider>,
            ),
        );
    };
    await render();
    if (data)
        await vi.waitFor(async () => {
            await act(async () => {
                await Promise.resolve();
            });
            expect(host.querySelector("[data-view=list]")).not.toBeNull();
        });
    return {
        host,
        render,
        onSort,
        dispose: async () => {
            await act(async () => {
                actualRoot.unmount();
            });
            client.clear();
            get.mockReset();
        },
    };
}
it("renders five wide columns and sorts through their headings", async () => {
    const h = await harness(false);
    try {
        expect(h.host.querySelectorAll("[role=columnheader]")).toHaveLength(5);
        expect(h.host.querySelector("[role=option]")!.children).toHaveLength(5);
        expect((h.host.querySelector("[role=listbox] > div") as HTMLElement).style.height).toBe("140000px");
        const modified = [...h.host.querySelectorAll("button")].find((button) =>
            button.textContent?.startsWith("Modified"),
        )!;
        await act(async () => modified.click());
        expect(h.onSort).toHaveBeenLastCalledWith({ key: "modified_at", order: "asc" });
        await h.render({ key: "modified_at", order: "asc" });
        await vi.waitFor(async () => {
            await act(async () => {
                await Promise.resolve();
            });
            expect(h.host.querySelector('[role="columnheader"][aria-sort="ascending"]')?.textContent).toContain(
                "Modified",
            );
        });
        const currentModified = [...h.host.querySelectorAll("button")].find((button) =>
            button.textContent?.startsWith("Modified"),
        )!;
        await act(async () => currentModified.click());
        expect(h.onSort).toHaveBeenLastCalledWith({ key: "modified_at", order: "desc" });
    } finally {
        await h.dispose();
    }
});
it("renders two lines without headings on a narrow screen", async () => {
    const h = await harness(true);
    try {
        expect(h.host.querySelectorAll("[role=columnheader]")).toHaveLength(0);
        const lines = h.host.querySelector("[data-two-lines]")!;
        expect(lines.children).toHaveLength(2);
        expect(lines.children[0]!.textContent).toBe("f0.txt");
        expect(lines.children[1]!.textContent).toContain(" · 123 B");
    } finally {
        await h.dispose();
    }
});

it("広いリストは名前の次に、並べ替えない場所の列を足す", async () => {
    const h = await harness(false, searchSource(true), searchPage(listPage()), placeUnder);
    try {
        expect(h.host.querySelectorAll("[role=columnheader]")).toHaveLength(6);
        const heading = [...h.host.querySelectorAll("[role=columnheader]")].find((el) => el.textContent === "Place")!;
        expect(heading.querySelector("button")).toBeNull();
        const rows = [...h.host.querySelectorAll("[role=option]")];
        expect(rows[0]!.children[2]!.textContent).toBe(".");
        expect(rows[1]!.children[2]!.textContent).toBe("deep");
    } finally {
        await h.dispose();
    }
});
it("狭いリストは配下の検索の場所を 2 段目の頭に置く", async () => {
    const h = await harness(true, searchSource(true), searchPage(listPage()), placeUnder);
    try {
        const lines = [...h.host.querySelectorAll("[data-two-lines]")];
        expect(lines[0]!.children[1]!.textContent).toMatch(/^\. · /);
        expect(lines[1]!.children[1]!.textContent).toMatch(/^deep · /);
        expect(h.host.querySelectorAll("[role=columnheader]")).toHaveLength(0);
    } finally {
        await h.dispose();
    }
});
it("直下の絞り込みでは場所を出さない", async () => {
    const h = await harness(false, searchSource(false), searchPage(listPage()));
    try {
        expect(h.host.querySelectorAll("[role=columnheader]")).toHaveLength(5);
        expect(h.host.querySelector("[title=deep]")).toBeNull();
    } finally {
        await h.dispose();
    }
});
it.each([true, false])("リストは配下の最初のページだけ検索中と示す %s", async (recursive) => {
    const h = await harness(false, searchSource(recursive), null, recursive ? placeUnder : undefined);
    try {
        expect(h.host.textContent).toBe(recursive ? "Searching…" : "Loading…");
    } finally {
        await h.dispose();
    }
});
it.each([false, true])("リストは検索の 0 件と打ち切りを示す %s", async (truncated) => {
    const h = await harness(
        false,
        searchSource(true, "missing"),
        searchPage({ entries: [], totalCount: 0, snapshot: "empty", truncated }),
        placeUnder,
    );
    try {
        expect(h.host.textContent).toContain("No matching items.");
        expect(h.host.querySelector('[role="status"]')?.textContent).toBe(
            truncated
                ? "Showing part of the results. Narrow the Libraries or go deeper into a location and search again."
                : undefined,
        );
    } finally {
        await h.dispose();
    }
});
it("リストは結果があるときも打ち切りを示す", async () => {
    const h = await harness(false, searchSource(true), searchPage({ ...listPage(), truncated: true }), placeUnder);
    try {
        expect(h.host.querySelector('[role="status"]')?.textContent).toContain("Narrow the Libraries");
    } finally {
        await h.dispose();
    }
});
it("リストは 0 件の通常の一覧を空のフォルダと示す", async () => {
    const h = await harness(
        false,
        { kind: "folder", libraryId: "search", path: "/base" },
        { entries: [], totalCount: 0, snapshot: "empty", truncated: false },
    );
    try {
        expect(h.host.textContent).toContain("This folder is empty.");
    } finally {
        await h.dispose();
    }
});

it("only schedules visible folders but retains prefetched images and videos", async () => {
    const spy = vi.spyOn(ImageWants.prototype, "setWanted").mockImplementation(() => {});
    const data: FolderListing = {
        entries: [
            { name: "dir", path: "/dir", kind: "folder" },
            { name: "a.zip", path: "/a.zip", kind: "file", file: { archive: true } },
            { name: "i.png", path: "/i.png", kind: "file" },
            { name: "v.mp4", path: "/v.mp4", kind: "file" },
            { name: "offscreen", path: "/offscreen", kind: "folder" },
        ],
        totalCount: 5,
        snapshot: "thumb-visible",
        truncated: false,
    };
    const h = await harness(false, { kind: "folder", libraryId: "thumb-visible", path: "/" }, data);
    try {
        const last = spy.mock.calls.at(-1);
        if (!last?.[1]) throw new Error("thumbnail demand missing");
        expect([...last[0].keys()]).toEqual(["/dir", "/a.zip", "/i.png", "/v.mp4"]);
        expect([...last[1]]).toEqual(["/dir", "/a.zip"]);
    } finally {
        await h.dispose();
        spy.mockRestore();
    }
});
