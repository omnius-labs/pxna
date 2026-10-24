// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import { ClipboardContext, EntryClipboard } from "../library/clipboard";
import { type TransferState, useTransfer } from "../library/transfer";
import { TransferProvider } from "./TransferProvider";

const { get, post, fetchPage, refresh, show, notify, screen } = vi.hoisted(() => ({
    get: vi.fn(),
    post: vi.fn(),
    fetchPage: vi.fn(),
    refresh: vi.fn(),
    show: vi.fn(),
    notify: vi.fn(),
    screen: { narrow: false },
}));
vi.mock("../api/client", async (original) => ({
    ...(await original<typeof import("../api/client")>()),
    api: { GET: get, POST: post },
}));
vi.mock("../library/entries", () => ({ fetchFolderPage: fetchPage }));
vi.mock("../library/listing", () => ({ refreshLibrary: refresh }));
vi.mock("../library/activity", () => ({ useActivity: () => ({ show }) }));
vi.mock("../library/preferences", () => ({ useNarrowScreen: () => screen.narrow }));
vi.mock("sonner", () => ({ toast: { success: vi.fn(), error: notify } }));
afterEach(() => {
    get.mockReset();
    post.mockReset();
    fetchPage.mockReset();
    refresh.mockReset();
    show.mockReset();
    notify.mockReset();
    screen.narrow = false;
});
const response = (data: unknown, status = 200) => ({ data, response: new Response(null, { status }) });
const operation = {
    id: "accepted",
    kind: "move_files",
    status: "pending",
    libraryId: "src",
    destinationLibraryId: "dest",
    processedCount: 0,
    skippedCount: 0,
    failedEntries: [],
    createdAt: "2026-10-08",
};
const selected = () =>
    new Map([
        ["/a.txt", { size: 1, folder: false, readonly: false }],
        ["/dir", { size: 0, folder: true, readonly: false }],
    ]);
const destination = { libraryId: "dest", path: "/target" };
async function flush(assertion: () => void) {
    await vi.waitFor(async () => {
        await act(async () => {
            await Promise.resolve();
        });
        assertion();
    });
}
async function harness() {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    client.setQueryData(
        ["libraries"],
        [
            { id: "src", readonly: false },
            { id: "dest", readonly: false },
        ],
    );
    get.mockResolvedValue(response({ folder: { metadata: { readonly: false } } }));
    fetchPage.mockResolvedValue({ entries: [], totalCount: 0, snapshot: "one" });
    post.mockResolvedValue(response({ operation }, 202));
    const store = new EntryClipboard();
    let transfer!: TransferState;
    function Probe() {
        transfer = useTransfer()!;
        return null;
    }
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    const render = async () => {
        await act(async () =>
            root.render(
                <QueryClientProvider client={client}>
                    <ClipboardContext.Provider value={store}>
                        <TransferProvider>
                            <Probe />
                        </TransferProvider>
                    </ClipboardContext.Provider>
                </QueryClientProvider>,
            ),
        );
    };
    await render();
    return {
        get state() {
            return { value: store.getSnapshot(), busy: transfer.busy, capture: store.write.bind(store) };
        },
        capture: async (mode: "cut" | "copy", libraryId = "src") => {
            await act(async () => store.write(mode, libraryId, selected()));
        },
        begin: async (target = destination) => {
            let promise!: Promise<void>;
            await act(async () => {
                promise = transfer.paste(target);
            });
            return { promise };
        },
        choose: async (text: string) => {
            await act(async () =>
                [...document.querySelectorAll<HTMLButtonElement>("[role=alertdialog] button")]
                    .find((button) => button.textContent === text)!
                    .click(),
            );
        },
        render,
        dispose: async () => {
            await act(async () => root.unmount());
            host.remove();
            client.clear();
        },
    };
}
it("混在した移動は先に受理した項目だけ除き、拒否された項目を残す", async () => {
    const h = await harness();
    try {
        let finish!: (data: unknown) => void;
        post.mockResolvedValueOnce(response({ operation }, 202)).mockImplementationOnce(
            () =>
                new Promise((resolve) => {
                    finish = resolve;
                }),
        );
        await h.capture("cut");
        const { promise } = await h.begin();
        await flush(() =>
            expect(h.state.value!.sources.flatMap((source) => source.entries).map((entry) => entry.path)).toEqual([
                "/dir",
            ]),
        );
        expect(show).toHaveBeenCalledWith([operation]);
        finish({ error: { kind: "Forbidden" }, response: new Response(null, { status: 403 }) });
        await act(async () => promise);
        expect(h.state.value!.sources.flatMap((source) => source.entries).map((entry) => entry.path)).toEqual(["/dir"]);
        expect(notify).toHaveBeenCalled();
    } finally {
        await h.dispose();
    }
});
it("重複した貼り付けを受け付けず、古い応答で新しい切り取りを消さない", async () => {
    const h = await harness();
    try {
        let finish!: (data: unknown) => void;
        post.mockImplementation(
            () =>
                new Promise((resolve) => {
                    finish = resolve;
                }),
        );
        await act(async () => h.state.capture("cut", "src", new Map([["/a.txt", selected().get("/a.txt")!]])));
        const { promise } = await h.begin();
        await flush(() => expect(post).toHaveBeenCalledOnce());
        await h.begin();
        expect(post).toHaveBeenCalledOnce();
        await h.capture("cut");
        const newer = h.state.value;
        finish(response({ operation }, 202));
        await act(async () => promise);
        expect(h.state.value).toBe(newer);
    } finally {
        await h.dispose();
    }
});
it.each(["置き換える", "両方残す", "スキップ", "中止"])(
    "衝突の %s を1回選び、コピーの内容を保持する",
    async (choice) => {
        const h = await harness();
        try {
            fetchPage.mockResolvedValue({ entries: [{ name: "a.txt" }], totalCount: 1, snapshot: "one" });
            await h.capture("copy");
            const { promise } = await h.begin();
            await flush(() => expect(document.querySelector("[role=alertdialog]")).not.toBeNull());
            await h.choose(choice);
            await act(async () => promise);
            if (choice === "中止") expect(post).not.toHaveBeenCalled();
            else
                expect(post.mock.calls[0]![1].body.conflict).toBe(
                    ({ 置き換える: "overwrite", 両方残す: "rename", スキップ: "skip" } as Record<string, string>)[
                        choice
                    ],
                );
            expect(h.state.value?.sources.flatMap((source) => source.entries).length).toBe(2);
        } finally {
            await h.dispose();
        }
    },
);
it("同じ場所へのコピーは別名と中止だけを出し、move と自己配下は送らない", async () => {
    const h = await harness();
    try {
        await h.capture("copy", "dest");
        const { promise } = await h.begin({ libraryId: "dest", path: "/" });
        await flush(() => expect(document.querySelector("[role=alertdialog]")).not.toBeNull());
        expect([...document.querySelectorAll("[role=alertdialog] button")].map((button) => button.textContent)).toEqual(
            ["中止", "両方残す"],
        );
        await h.choose("両方残す");
        await act(async () => promise);
        expect(post.mock.calls[0]![1].body.conflict).toBe("rename");
        post.mockClear();
        await h.capture("cut", "dest");
        await act(async () => (await h.begin({ libraryId: "dest", path: "/" })).promise);
        expect(post).not.toHaveBeenCalled();
        await h.capture("copy", "dest");
        await act(async () => (await h.begin({ libraryId: "dest", path: "/dir/sub" })).promise);
        expect(post).not.toHaveBeenCalled();
    } finally {
        await h.dispose();
    }
});
it("読み取り専用の宛先、照合失敗、狭い画面への切り替えでは送らず切り取りを残す", async () => {
    const h = await harness();
    try {
        await h.capture("cut");
        get.mockResolvedValueOnce(response({ folder: { metadata: { readonly: true } } }));
        await act(async () => (await h.begin()).promise);
        expect(post).not.toHaveBeenCalled();
        fetchPage.mockRejectedValueOnce(new Error("offline"));
        await act(async () => (await h.begin()).promise);
        expect(post).not.toHaveBeenCalled();
        fetchPage.mockResolvedValue({ entries: [{ name: "a.txt" }], totalCount: 1, snapshot: "one" });
        const { promise } = await h.begin();
        await flush(() => expect(document.querySelector("[role=alertdialog]")).not.toBeNull());
        screen.narrow = true;
        await h.render();
        await act(async () => promise);
        expect(document.querySelector("[role=alertdialog]")).toBeNull();
        expect(post).not.toHaveBeenCalled();
        expect(h.state.value?.sources.flatMap((source) => source.entries).length).toBe(2);
    } finally {
        await h.dispose();
    }
});
