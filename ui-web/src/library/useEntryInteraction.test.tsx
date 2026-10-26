// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";

import { testEntry } from "../testing/entries";
import type { FolderListing } from "./entries";
import { useFolderListing } from "./listing";
import type { Selection } from "./selection";
import { type SelectionProgress, useEntryInteraction } from "./useEntryInteraction";

const { get } = vi.hoisted(() => ({ get: vi.fn() }));
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
    },
}));
afterEach(() => get.mockReset());
const page = (offset: number, snapshot = "one"): FolderListing => ({
    entries: Array.from({ length: Math.min(200, 401 - offset) }, (_, i) => ({
        path: `/f${offset + i}`,
        name: `f${offset + i}`,
        kind: "file",
        metadata: { size: offset + i + 1, readonly: false },
    })),
    totalCount: 401,
    snapshot,
    truncated: false,
});
const response = (data: FolderListing) => ({ data, response: new Response() });
const initial: Selection = {
    items: new Map([["/f0", { size: 1, folder: false, readonly: false, entry: testEntry("/f0") }]]),
    anchor: 0,
};

async function harness({ openOnClick = false } = {}) {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    const pending = new Map<number, { resolve: (snapshot?: string) => void; signal: AbortSignal }>();
    get.mockImplementation((_, options) =>
        options.params.query.offset === 0
            ? Promise.resolve(response(page(0)))
            : new Promise((resolve) =>
                  pending.set(options.params.query.offset, {
                      resolve: (snapshot) => resolve(response(page(options.params.query.offset, snapshot))),
                      signal: options.signal,
                  }),
              ),
    );
    const onOpen = vi.fn();
    const latest: {
        selection: Selection;
        progress?: SelectionProgress;
        listing?: ReturnType<typeof useFolderListing>;
    } = { selection: initial };
    const report = (
        selection: Selection,
        progress: SelectionProgress | undefined,
        listing: ReturnType<typeof useFolderListing>,
    ) => {
        latest.selection = selection;
        latest.progress = progress;
        latest.listing = listing;
    };
    function Harness() {
        const listing = useFolderListing("big", "/", { key: "name", order: "asc" }, new Set());
        const [selection, setSelection] = useState(initial);
        const [progress, setProgress] = useState<SelectionProgress>();
        const interaction = useEntryInteraction({
            listing,
            columns: 1,
            selection,
            onSelectionChange: setSelection,
            onProgress: setProgress,
            onFocus: () => {},
            onOpen,
            openOnClick,
        });
        useEffect(() => {
            report(selection, progress, listing);
        });
        return (
            <>
                <button
                    type="button"
                    data-range
                    onClick={(event) => interaction.click(event, 400)}
                    onKeyDown={(event) => interaction.keyDown(event, 199)}
                >
                    range
                </button>
                <button
                    type="button"
                    data-plain
                    onClick={(event) => interaction.click(event, 0)}
                    onDoubleClick={() => {
                        const entry = listing.entryAt(0);
                        if (entry) interaction.open(entry);
                    }}
                >
                    plain
                </button>
            </>
        );
    }
    const client = new QueryClient();
    const host = document.createElement("div");
    const root = createRoot(host);
    await act(async () =>
        root.render(
            <QueryClientProvider client={client}>
                <Harness />
            </QueryClientProvider>,
        ),
    );
    await wait(() => expect(latest.listing?.pageStatus(0)).toBe("loaded"));
    const clickRange = async () => {
        await act(async () =>
            host
                .querySelector("[data-range]")!
                .dispatchEvent(new MouseEvent("click", { bubbles: true, shiftKey: true })),
        );
    };
    return {
        latest,
        onOpen,
        pending,
        host,
        clickRange,
        dispose: async () => {
            await act(async () => root.unmount());
            client.clear();
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

it("waits for all range pages before selecting their paths and sizes", async () => {
    const h = await harness();
    try {
        await h.clickRange();
        expect(h.latest.selection.items.size).toBe(1);
        expect(h.latest.progress).toEqual({ loaded: 1, total: 3 });
        await act(async () => {
            h.pending.get(200)!.resolve();
        });
        await wait(() => expect(h.latest.progress).toEqual({ loaded: 2, total: 3 }));
        expect(h.latest.selection.items.size).toBe(1);
        await act(async () => {
            h.pending.get(400)!.resolve();
        });
        await wait(() => expect(h.latest.selection.items.size).toBe(401));
        expect([...h.latest.selection.items.values()].reduce((sum, size) => sum + size.size!, 0)).toBe(80601);
        expect(h.latest.progress).toBeUndefined();
    } finally {
        await h.dispose();
    }
});
it("discards the anchor and progress without changing selected paths when the snapshot changes", async () => {
    const h = await harness();
    try {
        await h.clickRange();
        await act(async () => {
            h.pending.get(400)!.resolve("new");
        });
        await wait(() => expect(h.latest.progress).toBeUndefined());
        expect(h.latest.selection.items).toEqual(initial.items);
        expect(h.latest.selection.anchor).toBeUndefined();
        expect(h.pending.get(200)!.signal.aborted).toBe(true);
    } finally {
        await h.dispose();
    }
});
it("cancels a range when another operation starts", async () => {
    const h = await harness();
    try {
        await h.clickRange();
        await act(async () => (h.host.querySelector("[data-plain]") as HTMLElement).click());
        expect(h.latest.progress).toBeUndefined();
        expect(h.pending.get(200)!.signal.aborted).toBe(true);
        expect(h.pending.get(400)!.signal.aborted).toBe(true);
        await act(async () => {
            for (const pending of h.pending.values()) pending.resolve();
        });
        expect(h.latest.selection.items.size).toBe(1);
    } finally {
        await h.dispose();
    }
});
it.each(["ctrlKey", "metaKey"])("selects the whole listing with %s+A", async (modifier) => {
    const h = await harness();
    try {
        await act(async () =>
            h.host
                .querySelector("[data-range]")!
                .dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, key: "a", [modifier]: true })),
        );
        expect(h.latest.selection.items.size).toBe(1);
        await act(async () => {
            for (const pending of h.pending.values()) pending.resolve();
        });
        await wait(() => expect(h.latest.selection.items.size).toBe(401));
    } finally {
        await h.dispose();
    }
});
it("extends selection with Shift+Arrow across an unread page", async () => {
    const h = await harness();
    try {
        await act(async () =>
            h.host
                .querySelector("[data-range]")!
                .dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, key: "ArrowRight", shiftKey: true })),
        );
        expect(h.latest.selection.items.size).toBe(1);
        await act(async () => {
            h.pending.get(200)!.resolve();
        });
        await wait(() => expect(h.latest.selection.items.size).toBe(201));
    } finally {
        await h.dispose();
    }
});
it("aborts a range when the folder component unmounts", async () => {
    const h = await harness();
    await h.clickRange();
    await h.dispose();
    expect(h.pending.get(200)!.signal.aborted).toBe(true);
    expect(h.pending.get(400)!.signal.aborted).toBe(true);
});
it("opens with a click without modifiers and ignores the double click when opening on click", async () => {
    const h = await harness({ openOnClick: true });
    const plain = () => h.host.querySelector("[data-plain]") ?? document.body;
    try {
        await act(async () => plain().dispatchEvent(new MouseEvent("click", { bubbles: true })));
        expect(h.onOpen.mock.calls.map(([entry]) => entry.path)).toEqual(["/f0"]);
        await act(async () => plain().dispatchEvent(new MouseEvent("dblclick", { bubbles: true })));
        expect(h.onOpen).toHaveBeenCalledOnce();
        await act(async () => plain().dispatchEvent(new MouseEvent("click", { bubbles: true, ctrlKey: true })));
        expect(h.onOpen).toHaveBeenCalledOnce();
        expect(h.latest.selection.items.size).toBe(0);
    } finally {
        await h.dispose();
    }
});
it("selects with a click and opens with a double click by default", async () => {
    const h = await harness();
    const plain = () => h.host.querySelector("[data-plain]") ?? document.body;
    try {
        await act(async () => plain().dispatchEvent(new MouseEvent("click", { bubbles: true })));
        expect(h.onOpen).not.toHaveBeenCalled();
        expect([...h.latest.selection.items.keys()]).toEqual(["/f0"]);
        await act(async () => plain().dispatchEvent(new MouseEvent("dblclick", { bubbles: true })));
        expect(h.onOpen).toHaveBeenCalledOnce();
    } finally {
        await h.dispose();
    }
});
