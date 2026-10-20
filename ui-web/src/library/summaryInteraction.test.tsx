// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, type KeyboardEvent, type MouseEvent, useState } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";
import { useFolderListing } from "./listing";
import { emptySelection, type Selection, selectionBytes } from "./selection";
import { useEntryInteraction } from "./useEntryInteraction";

const { get, post } = vi.hoisted(() => ({ get: vi.fn(), post: vi.fn() }));
vi.mock("../api/client", async (original) => ({
    ...(await original<typeof import("../api/client")>()),
    api: { GET: get, POST: post },
}));

async function harness() {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    get.mockResolvedValue({
        data: {
            entries: ["a.jpg", "b.jpg"].map((name) => ({ path: `/${name}`, name, kind: "file", archive: false })),
            snapshot: "one",
            totalCount: 2,
            truncated: false,
        },
        response: new Response(),
    });
    let resolve!: (value: unknown) => void;
    post.mockImplementation(
        () =>
            new Promise((done) => {
                resolve = done;
            }),
    );
    const client = new QueryClient();
    client.setQueryData(["capabilities"], { summaryListing: true, entryDetails: true, childProbes: false });
    const root = createRoot(document.createElement("div"));
    let latest!: { selection: Selection; interaction: ReturnType<typeof useEntryInteraction> };
    let showPages!: (pages: number[]) => void;
    function Harness() {
        const [wanted, setWanted] = useState([0]);
        showPages = setWanted;
        const listing = useFolderListing("test", "/", { key: "name", order: "asc" }, new Set(wanted));
        const [selection, setSelection] = useState(emptySelection);
        const interaction = useEntryInteraction({
            listing,
            columns: 1,
            selection,
            onSelectionChange: setSelection,
            onProgress: () => {},
            onFocus: () => {},
            onOpen: () => {},
        });
        latest = { selection, interaction };
        return null;
    }
    await act(async () =>
        root.render(
            <QueryClientProvider client={client}>
                <Harness />
            </QueryClientProvider>,
        ),
    );
    await vi.waitFor(() => expect(post).toHaveBeenCalledTimes(1));
    return {
        get latest() {
            return latest;
        },
        showPages: async (pages: number[]) => {
            await act(async () => showPages(pages));
        },
        complete: async (partial = false) => {
            await act(async () =>
                resolve({
                    data: {
                        results: [
                            {
                                path: "/a.jpg",
                                detail: {
                                    path: "/a.jpg",
                                    name: "a.jpg",
                                    kind: "file",
                                    file: { archive: false },
                                    metadata: { size: 10, readonly: false, contentType: "image/jpeg" },
                                },
                            },
                            partial
                                ? { path: "/b.jpg", error: "NotFound" }
                                : {
                                      path: "/b.jpg",
                                      detail: {
                                          path: "/b.jpg",
                                          name: "b.jpg",
                                          kind: "file",
                                          file: { archive: false },
                                          metadata: { size: 20, readonly: true, contentType: "image/jpeg" },
                                      },
                                  },
                        ],
                    },
                    response: new Response(),
                }),
            );
        },
        dispose: async () => {
            await act(async () => root.unmount());
            client.clear();
            get.mockReset();
            post.mockReset();
        },
    };
}

it("名前で選択した後に詳細が届くと、選択pathを保ってサイズとreadonlyを更新する", async () => {
    const h = await harness();
    try {
        await act(async () =>
            h.latest.interaction.click({ metaKey: false, ctrlKey: false, shiftKey: false } as MouseEvent, 0),
        );
        expect([...h.latest.selection.items.keys()]).toEqual(["/a.jpg"]);
        expect(h.latest.selection.items.get("/a.jpg")?.readonly).toBe(true);
        expect(selectionBytes(h.latest.selection)).toBeUndefined();
        await h.complete();
        expect([...h.latest.selection.items.keys()]).toEqual(["/a.jpg"]);
        expect(h.latest.selection.items.get("/a.jpg")?.readonly).toBe(false);
        expect(selectionBytes(h.latest.selection)).toBe(10);
    } finally {
        await h.dispose();
    }
});

it("全選択を詳細待ちで止めず、部分失敗があれば合計を不明のままにする", async () => {
    const h = await harness();
    try {
        await act(async () =>
            h.latest.interaction.keyDown({ key: "a", ctrlKey: true, preventDefault() {} } as KeyboardEvent, 0),
        );
        expect(h.latest.selection.items.size).toBe(2);
        expect(selectionBytes(h.latest.selection)).toBeUndefined();
        await h.complete(true);
        expect(h.latest.selection.items.size).toBe(2);
        expect(h.latest.selection.items.get("/a.jpg")?.size).toBe(10);
        expect(h.latest.selection.items.get("/b.jpg")?.entry.detailError).toBe("NotFound");
        expect(h.latest.selection.items.get("/b.jpg")?.readonly).toBe(true);
        expect(selectionBytes(h.latest.selection)).toBeUndefined();
    } finally {
        await h.dispose();
    }
});

it("先読み範囲を広げても、引き続き必要なページの詳細要求を止めない", async () => {
    const h = await harness();
    try {
        const signal = post.mock.calls[0]![1].signal as AbortSignal;
        await h.showPages([0, 1]);
        expect(signal.aborted).toBe(false);
        expect(post).toHaveBeenCalledTimes(1);
    } finally {
        await h.dispose();
    }
});
