// @vitest-environment jsdom
import { QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";
import type { Entry } from "../library/entries";
import { DEFAULT_PREFERENCES } from "../library/preferences";
import { seedPreferences, stubChangingMatchMedia, stubResizeObserver } from "../testing/preferences";
import { loadRouterAt } from "../testing/router";

const { get, post } = vi.hoisted(() => ({ get: vi.fn(), post: vi.fn() }));
vi.mock("../api/client", async (original) => ({
    ...(await original<typeof import("../api/client")>()),
    api: { GET: get, POST: post },
}));
const virtual = vi.hoisted(() => ({
    initial: undefined as { startIndex: number; endIndex: number } | undefined,
    shift: (() => {}) as (index: number) => void,
    scroll: vi.fn(),
}));
vi.mock("@tanstack/react-virtual", async () => {
    const { useState, useRef, useMemo } = await import("react");
    return {
        useVirtualizer: (options: { count: number; estimateSize(): number }) => {
            const [range, setRange] = useState(virtual.initial);
            const current = useRef(range);
            current.current = range;
            virtual.shift = (index) => setRange({ startIndex: index, endIndex: index + 4 });
            const height = options.estimateSize();
            return useMemo(
                () => ({
                    get range() {
                        return options.count
                            ? (current.current ?? { startIndex: 0, endIndex: options.count - 1 })
                            : null;
                    },
                    scrollDirection: null,
                    measure: () => {},
                    scrollToIndex: (index: number) => {
                        virtual.scroll(index);
                        setRange((value) =>
                            value && (index < value.startIndex || index > value.endIndex)
                                ? { startIndex: index, endIndex: index + 4 }
                                : value,
                        );
                    },
                    getTotalSize: () => options.count * height,
                    getVirtualItems: () =>
                        Array.from({ length: options.count }, (_, index) => ({
                            index,
                            key: index,
                            start: index * height,
                            size: height,
                        })).filter(
                            (row) =>
                                !current.current ||
                                (row.index >= current.current.startIndex && row.index <= current.current.endIndex),
                        ),
                }),
                [options.count, height],
            );
        },
    };
});
async function wait(assertion: () => void) {
    await vi.waitFor(async () => {
        await act(async () => {
            await Promise.resolve();
        });
        assertion();
    });
}
/// 名前のダイアログの入力欄。
function nameInput(): HTMLInputElement | null {
    return document.querySelector<HTMLInputElement>('[role="dialog"] input[aria-label="Name"]');
}

/// キー操作が入力欄では効かないことを確かめるための、一覧の外の入力欄。
function typingInput(host: HTMLElement): HTMLInputElement {
    const input = document.createElement("input");
    host.append(input);
    return input;
}

async function harness(view: "grid" | "list" = "grid", initial: Entry[] = []) {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    const entries: Entry[] = [...initial];
    let snapshot = 0;
    const response = (data: unknown, status = 200) => ({ data, response: new Response(null, { status }) });
    get.mockImplementation(
        async (route: string, options?: { params?: { query?: { path?: string; offset?: number } } }) => {
            if (route.endsWith("/thumbnails"))
                return { error: { kind: "NotFound" }, response: new Response(null, { status: 404 }) };
            if (route === "/libraries") return response({ libraries: [{ id: "ops", name: "Ops", readonly: false }] });
            if (route === "/operations") return response({ operations: [] });
            if (route.endsWith("/folders"))
                return response({
                    folder: {
                        path: options?.params?.query?.path ?? "/",
                        name: "",
                        kind: "folder",
                        metadata: { size: 0, readonly: false },
                    },
                });
            return response({
                entries: entries.slice(
                    options?.params?.query?.offset ?? 0,
                    (options?.params?.query?.offset ?? 0) + 200,
                ),
                totalCount: entries.length,
                snapshot: String(snapshot),
                truncated: false,
            });
        },
    );
    post.mockImplementation(async (route, options) => {
        if (route.endsWith("/delete")) {
            const paths = options.body.srcPaths as string[];
            for (let i = entries.length - 1; i >= 0; i--) if (paths.includes(entries[i]!.path)) entries.splice(i, 1);
            snapshot++;
            return response(undefined, 204);
        }
        if (route.endsWith("/move")) {
            const index = entries.findIndex((entry) => entry.path === options.body.srcPaths[0]);
            const changed = {
                ...entries[index]!,
                path: options.body.dstPath,
                name: options.body.dstPath.split("/").at(-1)!,
            };
            entries[index] = changed;
            snapshot++;
            return response({ entries: [changed] });
        }
        const path = options.body.path;
        const entry: Entry = {
            path,
            name: path.split("/").at(-1)!,
            kind: "folder",
            metadata: { size: 0, readonly: false },
        };
        entries.push(entry);
        snapshot++;
        return response({ folder: entry }, 201);
    });
    const media = stubChangingMatchMedia();
    const restoreResize = stubResizeObserver();
    const router = await loadRouterAt("/browse/ops/empty/");
    seedPreferences(router.options.context.queryClient, {
        ...DEFAULT_PREFERENCES,
        wide: { ...DEFAULT_PREFERENCES.wide, view },
    });
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    await act(async () =>
        root.render(
            <QueryClientProvider client={router.options.context.queryClient}>
                <RouterProvider router={router} />
            </QueryClientProvider>,
        ),
    );
    await wait(() => {
        if (initial.length) expect(host.querySelector("[role=option]")).not.toBeNull();
        else expect(host.textContent).toContain("This folder is empty.");
    });
    const key = async (key: string, target: EventTarget = document.body, modifiers: KeyboardEventInit = {}) => {
        await act(async () => target.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, ...modifiers })));
    };
    const type = async (value: string) => {
        const input = nameInput()!;
        await act(async () => {
            Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value);
            input.dispatchEvent(new Event("input", { bubbles: true }));
        });
        return input;
    };
    const submit = async () => {
        await act(async () =>
            document.querySelector<HTMLButtonElement>('[role="dialog"] button[type="submit"]')!.click(),
        );
    };
    const create = async () => {
        await act(async () => host.querySelector<HTMLButtonElement>('button[aria-label="New folder"]')!.click());
    };
    return {
        host,
        key,
        type,
        submit,
        create,
        navigate: async (path: string) => {
            await act(async () => router.navigate({ href: `/browse/ops${path}/` }));
        },
        resize: async (narrow: boolean) => {
            await act(async () => media.setNarrow(narrow));
        },
        dispose: async () => {
            await act(async () => root.unmount());
            host.remove();
            router.history.destroy();
            router.options.context.queryClient.clear();
            restoreResize();
            media.restore();
            get.mockReset();
            post.mockReset();
            virtual.initial = undefined;
            virtual.scroll.mockClear();
        },
    };
}
it.each(["grid", "list"] as const)("空の %s で、ダイアログに作成先を示し、直下に作って選択を外す", async (view) => {
    const h = await harness(view);
    try {
        await h.create();
        const input = await h.type("made");
        expect(document.activeElement).toBe(input);
        expect(document.querySelector('[role="dialog"]')?.textContent).toContain("Create in: /empty");
        await h.submit();
        await wait(() => expect(document.querySelector('[role="dialog"]')).toBeNull());
        expect(post).toHaveBeenCalledOnce();
        expect(post.mock.calls[0]![1].body).toEqual({ path: "/empty/made", createParents: false });
        expect(h.host.querySelector('[role="option"][aria-selected="true"]')).toBeNull();
    } finally {
        await h.dispose();
    }
});
it("取り消すと作成せず、409 ではダイアログと入力を残す", async () => {
    const h = await harness();
    try {
        await h.create();
        const first = await h.type("cancelled");
        await h.key("Escape", first);
        await wait(() => expect(document.querySelector('[role="dialog"]')).toBeNull());
        expect(post).not.toHaveBeenCalled();
        post.mockResolvedValue({ error: { kind: "Conflict" }, response: new Response(null, { status: 409 }) });
        await h.create();
        const collision = await h.type("dup");
        await h.submit();
        await wait(() => expect(post).toHaveBeenCalledOnce());
        expect(document.querySelector('[role="dialog"]')).not.toBeNull();
        expect(document.activeElement).toBe(collision);
        expect(collision.value).toBe("dup");
    } finally {
        await h.dispose();
    }
});
it("使えない名前では作成のボタンを押せない", async () => {
    const h = await harness();
    try {
        await h.create();
        const submit = () => document.querySelector<HTMLButtonElement>('[role="dialog"] button[type="submit"]')!;
        expect(submit().disabled).toBe(true);
        await h.type("a/b");
        expect(submit().disabled).toBe(true);
        await h.type("ok");
        expect(submit().disabled).toBe(false);
    } finally {
        await h.dispose();
    }
});
it("物理的な KeyN で作成を始め、入力欄と alertdialog 内では始めない", async () => {
    const h = await harness();
    try {
        const modifiers = { altKey: true, shiftKey: true, code: "KeyN" };
        await h.key("˜", document.body, modifiers);
        await wait(() => expect(nameInput()).not.toBeNull());
        await h.key("Escape", nameInput()!);
        await wait(() => expect(document.querySelector('[role="dialog"]')).toBeNull());
        await h.key("N", typingInput(h.host), modifiers);
        expect(document.querySelector('[role="dialog"]')).toBeNull();
        const dialog = document.createElement("div");
        dialog.setAttribute("role", "alertdialog");
        const button = document.createElement("button");
        dialog.append(button);
        document.body.append(dialog);
        await h.key("N", button, modifiers);
        dialog.remove();
        expect(document.querySelector('[role="dialog"]')).toBeNull();
        expect(post).not.toHaveBeenCalled();
    } finally {
        await h.dispose();
    }
});
it("右クリックメニューを閉じた後も新しい名前の入力にフォーカスを置く", async () => {
    const h = await harness();
    try {
        await act(async () =>
            h.host
                .querySelector('main [data-slot="context-menu-trigger"]')!
                .dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, button: 2 })),
        );
        const menuitem = [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].find(
            (item) => item.textContent === "New folder",
        )!;
        await act(async () => menuitem.click());
        await wait(() => expect(nameInput()).not.toBeNull());
        expect(document.activeElement).toBe(nameInput());
    } finally {
        await h.dispose();
    }
});

const fileEntry: import("../library/entries").DetailedEntry = {
    name: "a.txt",
    path: "/empty/a.txt",
    kind: "file",
    metadata: { size: 1, readonly: false },
};
it.each(["grid", "list"] as const)(
    "%s の F2 で対象のパスを示し、拡張子の前を選び、同じ親で名前を変更",
    async (view) => {
        const h = await harness(view, [fileEntry]);
        try {
            await act(async () => h.host.querySelector<HTMLElement>("[role=option]")!.click());
            await h.key("F2");
            await wait(() => expect(nameInput()).not.toBeNull());
            const input = nameInput()!;
            expect([input.selectionStart, input.selectionEnd, input.value]).toEqual([0, 1, "a.txt"]);
            expect(document.querySelector('[role="dialog"]')?.textContent).toContain("Target: /empty/a.txt");
            await h.type("changed.txt");
            await h.submit();
            await wait(() => expect(document.querySelector('[role="dialog"]')).toBeNull());
            expect(post.mock.calls[0]![0]).toBe("/libraries/{libraryId}/files/move");
            expect(post.mock.calls[0]![1].body).toEqual({ srcPaths: ["/empty/a.txt"], dstPath: "/empty/changed.txt" });
            expect(h.host.querySelector('[role="option"][aria-selected="true"]')).toBeNull();
            expect(h.host.textContent).toContain("changed.txt");
        } finally {
            await h.dispose();
        }
    },
);
it("名前の変更は、409 の後も入力を保ち、直して送り直せる", async () => {
    const h = await harness("grid", [fileEntry]);
    try {
        await act(async () =>
            h.host
                .querySelector("main [role=option]")!
                .dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, button: 2 })),
        );
        const rename = [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].find(
            (item) => item.textContent === "Rename",
        )!;
        await act(async () => rename.click());
        await wait(() => expect(nameInput()).not.toBeNull());
        const input = await h.type("conflict.txt");
        expect(document.activeElement).toBe(input);
        post.mockResolvedValueOnce({ error: { kind: "Conflict" }, response: new Response(null, { status: 409 }) });
        await h.submit();
        await wait(() => expect(post).toHaveBeenCalledOnce());
        expect(document.activeElement).toBe(input);
        expect(input.value).toBe("conflict.txt");
        await h.type("fixed.txt");
        await h.submit();
        await wait(() => expect(document.querySelector('[role="dialog"]')).toBeNull());
        expect(post).toHaveBeenCalledTimes(2);
    } finally {
        await h.dispose();
    }
});

it("Delete は確認後に混在削除を送り、ダイアログ内のキーと取消では送らない", async () => {
    const folder: Entry = { name: "dir", path: "/empty/dir", kind: "folder", metadata: { size: 0, readonly: false } };
    const h = await harness("list", [fileEntry, folder]);
    try {
        const options = h.host.querySelectorAll<HTMLElement>("[role=option]");
        await act(async () => options[0]!.click());
        await act(async () => options[1]!.dispatchEvent(new MouseEvent("click", { bubbles: true, ctrlKey: true })));
        await h.key("Backspace", document.body, { metaKey: true });
        const dialog = document.querySelector<HTMLElement>('[role="alertdialog"]')!;
        expect(dialog.textContent).toContain("2 items");
        expect(post).not.toHaveBeenCalled();
        await h.key("F2", dialog);
        await h.key("N", dialog, { altKey: true, shiftKey: true, code: "KeyN" });
        expect(nameInput()).toBeNull();
        await act(async () =>
            [...dialog.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === "Cancel")!.click(),
        );
        expect(post).not.toHaveBeenCalled();
        await h.key("Delete");
        await act(async () =>
            [...document.querySelectorAll<HTMLButtonElement>('[role="alertdialog"] button')]
                .find((b) => b.textContent === "Delete")!
                .click(),
        );
        await wait(() => expect(h.host.textContent).toContain("This folder is empty."));
        expect(post.mock.calls.map(([route]) => route)).toEqual([
            "/libraries/{libraryId}/files/delete",
            "/libraries/{libraryId}/folders/delete",
        ]);
        expect(post.mock.calls[1]![1].body).toEqual({ srcPaths: ["/empty/dir"], recursive: true });
        expect(h.host.querySelector('[role="option"][aria-selected="true"]')).toBeNull();
    } finally {
        await h.dispose();
    }
});

it("単一ファイルだけを content API の download リンクにする", async () => {
    const folder: Entry = { name: "dir", path: "/empty/dir", kind: "folder", metadata: { size: 0, readonly: false } };
    const h = await harness("list", [fileEntry, folder]);
    try {
        const options = h.host.querySelectorAll<HTMLElement>("[role=option]");
        expect(h.host.querySelector("a[download]")).toBeNull();
        await act(async () => options[0]!.click());
        expect(h.host.querySelector("a[download]")?.getAttribute("href")).toBe(
            "/api/v1/libraries/ops/files/content?path=%2Fempty%2Fa.txt",
        );
        await act(async () => options[1]!.dispatchEvent(new MouseEvent("click", { bubbles: true, ctrlKey: true })));
        expect(h.host.querySelector("a[download]")).toBeNull();
        await act(async () => options[1]!.click());
        expect(h.host.querySelector("a[download]")).toBeNull();
    } finally {
        await h.dispose();
    }
});

it.each(["grid", "list"] as const)(
    "%s の画面外の選択を F2 で、一覧に表示されていなくても名前を変更する（400 件）",
    async (view) => {
        virtual.initial = { startIndex: 0, endIndex: 4 };
        const entries = Array.from({ length: 400 }, (_, index) => ({
            ...fileEntry,
            path: `/empty/f${index}.txt`,
            name: `f${index}.txt`,
        }));
        const h = await harness(view, entries);
        try {
            await act(async () => h.host.querySelector<HTMLElement>('[role=option][data-index="0"]')!.click());
            await act(async () => virtual.shift(300));
            await wait(() => expect(h.host.querySelector('[role=option][data-index="300"]')).not.toBeNull());
            expect(h.host.querySelector('[role=option][data-index="0"]')).toBeNull();
            await h.key("F2");
            await wait(() => expect(nameInput()).not.toBeNull());
            const input = nameInput()!;
            expect(input.value).toBe("f0.txt");
            expect(document.activeElement).toBe(input);
            await h.type("renamed-offscreen.txt");
            await h.submit();
            await wait(() => expect(post).toHaveBeenCalledOnce());
            expect(post.mock.calls[0]![1].body).toEqual({
                srcPaths: ["/empty/f0.txt"],
                dstPath: "/empty/renamed-offscreen.txt",
            });
        } finally {
            await h.dispose();
        }
    },
);
it.each(["grid", "list"] as const)("%s の編集と削除確認を狭い画面への切り替えで取り消す", async (view) => {
    const h = await harness(view, [fileEntry]);
    try {
        await h.create();
        await h.type("unsent");
        await h.resize(true);
        expect(nameInput()).toBeNull();
        expect(document.querySelector("[role=alertdialog]")).toBeNull();
        expect(post).not.toHaveBeenCalled();
        await h.resize(false);
        expect(nameInput()).toBeNull();
        await act(async () => h.host.querySelector<HTMLElement>("[role=option]")!.click());
        await h.key("F2");
        await h.type("unsent.txt");
        await h.resize(true);
        expect(nameInput()).toBeNull();
        expect(post).not.toHaveBeenCalled();
        await h.resize(false);
        await act(async () => h.host.querySelector<HTMLElement>("[role=option]")!.click());
        await h.key("Delete");
        expect(document.querySelector("[role=alertdialog]")).not.toBeNull();
        await h.resize(true);
        expect(document.querySelector("[role=alertdialog]")).toBeNull();
        expect(post).not.toHaveBeenCalled();
        await h.resize(false);
        expect(document.querySelector("[role=alertdialog]")).toBeNull();
    } finally {
        await h.dispose();
    }
});
it.each(["grid", "list"] as const)(
    "%s の切り取りは薄表示になり、入力とダイアログではコピーせず、コピーで薄表示を戻す",
    async (view) => {
        const h = await harness(view, [fileEntry]);
        try {
            const option = h.host.querySelector<HTMLElement>('[role="option"]')!;
            await act(async () => option.click());
            await h.key("x", document.body, { metaKey: true });
            expect(option.style.opacity).toBe("0.4");
            expect(option.getAttribute("aria-selected")).toBe("true");
            await h.key("c", typingInput(h.host), { metaKey: true });
            expect(option.style.opacity).toBe("0.4");
            const dialog = document.createElement("div");
            dialog.setAttribute("role", "alertdialog");
            document.body.append(dialog);
            await h.key("c", document.body, { metaKey: true });
            expect(option.style.opacity).toBe("0.4");
            dialog.remove();
            await h.key("c", document.body, { ctrlKey: true });
            expect(option.style.opacity).toBe("");
            expect(post).not.toHaveBeenCalled();
        } finally {
            await h.dispose();
        }
    },
);
it.each(["cut", "copy"] as const)(
    "%s の貼り付けは確認中に再要求せず、方針付きで送り、受理後に切り取りを空にする",
    async (mode) => {
        const h = await harness("grid", [fileEntry]);
        try {
            const option = h.host.querySelector<HTMLElement>('[role="option"]')!;
            await act(async () => option.click());
            await h.key(mode === "cut" ? "x" : "c", document.body, { ctrlKey: true });
            if (mode === "cut") await h.navigate("/dest");
            await h.key("v", document.body, { ctrlKey: true });
            await wait(() => expect(document.querySelector('[role="alertdialog"]')).not.toBeNull());
            await h.key("v", document.body, { ctrlKey: true });
            expect(post).not.toHaveBeenCalled();
            const operation = {
                id: "transfer",
                kind: "copy_files",
                status: "pending",
                libraryId: "ops",
                destinationLibraryId: "ops",
                processedCount: 0,
                skippedCount: 0,
                failedEntries: [],
                resultEntries: [],
                createdAt: "2026-10-07",
            };
            post.mockResolvedValueOnce(
                mode === "cut"
                    ? { data: { entries: [] }, response: new Response(null, { status: 200 }) }
                    : { data: { operation }, response: new Response(null, { status: 202 }) },
            );
            await act(async () =>
                [...document.querySelectorAll<HTMLButtonElement>('[role="alertdialog"] button')]
                    .find((button) => button.textContent === (mode === "cut" ? "Skip" : "Keep both"))!
                    .click(),
            );
            await wait(() => expect(post).toHaveBeenCalledOnce());
            expect(post.mock.calls[0]![0]).toBe(`/libraries/{libraryId}/files/${mode === "cut" ? "move" : "copy"}`);
            expect(post.mock.calls[0]![1].body).toEqual({
                srcPaths: [fileEntry.path],
                dstLibraryId: "ops",
                dstFolderPath: mode === "cut" ? "/dest" : "/empty",
                conflict: mode === "cut" ? "skip" : "rename",
            });
            await h.navigate("/empty");
            await wait(() => expect(h.host.querySelector<HTMLElement>('[role="option"]')!.style.opacity).toBe(""));
            if (mode === "copy") expect(h.host.querySelector('[aria-label="Activity"]')?.textContent).toContain("Copy");
            else expect(h.host.querySelector<HTMLButtonElement>('button[aria-label="Paste"]')?.disabled).toBe(true);
        } finally {
            await h.dispose();
        }
    },
);
it("混在切り取りの一方が失敗したら拒否された項目と受理済み Operation を残す", async () => {
    const directory: Entry = {
        path: "/empty/dir",
        name: "dir",
        kind: "folder",
        metadata: { size: 0, readonly: false },
    };
    const h = await harness("grid", [fileEntry, directory]);
    try {
        const options = [...h.host.querySelectorAll<HTMLElement>('[role="option"]')];
        await act(async () => {
            options[0]!.click();
        });
        await act(async () => options[1]!.dispatchEvent(new MouseEvent("click", { bubbles: true, ctrlKey: true })));
        await h.key("x", document.body, { ctrlKey: true });
        await h.navigate("/dest");
        await h.key("v", document.body, { ctrlKey: true });
        await wait(() => expect(document.querySelector('[role="alertdialog"]')).not.toBeNull());
        post.mockResolvedValueOnce({
            data: {
                operation: {
                    id: "accepted",
                    kind: "move_files",
                    status: "pending",
                    libraryId: "ops",
                    processedCount: 0,
                    skippedCount: 0,
                    failedEntries: [],
                    createdAt: "2026-10-07",
                },
            },
            response: new Response(null, { status: 202 }),
        }).mockResolvedValueOnce({ error: { kind: "Forbidden" }, response: new Response(null, { status: 403 }) });
        await act(async () =>
            [...document.querySelectorAll<HTMLButtonElement>('[role="alertdialog"] button')]
                .find((button) => button.textContent === "Keep both")!
                .click(),
        );
        await wait(() => expect(post).toHaveBeenCalledTimes(2));
        await h.navigate("/empty");
        const remaining = h.host.querySelectorAll<HTMLElement>('[role="option"]');
        expect(remaining[0]!.style.opacity).toBe("");
        expect(remaining[1]!.style.opacity).toBe("0.4");
        expect(h.host.querySelector('[aria-label="Activity"]')?.textContent).toContain("Move");
    } finally {
        await h.dispose();
    }
});
it("混在moveの同期結果はトースト、非同期結果はパネルで知らせる", async () => {
    const { toast } = await import("sonner");
    const success = vi.spyOn(toast, "success");
    const dir: Entry = { path: "/empty/dir", name: "dir", kind: "folder", metadata: { size: 0, readonly: false } };
    const h = await harness("grid", [fileEntry, { ...fileEntry, name: "b.txt", path: "/empty/b.txt" }, dir]);
    try {
        await h.key("a", h.host.querySelector('[role="option"]')!, { ctrlKey: true });
        await h.key("x", document.body, { ctrlKey: true });
        await h.navigate("/dest");
        await h.key("v", document.body, { ctrlKey: true });
        await wait(() => expect(document.querySelector('[role="alertdialog"]')).not.toBeNull());
        post.mockResolvedValueOnce({
            data: {
                operation: {
                    id: "multi-files",
                    kind: "move_files",
                    status: "pending",
                    libraryId: "ops",
                    processedCount: 0,
                    skippedCount: 0,
                    failedEntries: [],
                    createdAt: "2026-10-07",
                },
            },
            response: new Response(null, { status: 202 }),
        }).mockResolvedValueOnce({ data: { entries: [dir] }, response: new Response(null, { status: 200 }) });
        await act(async () =>
            [...document.querySelectorAll<HTMLButtonElement>('[role="alertdialog"] button')]
                .find((button) => button.textContent === "Replace")!
                .click(),
        );
        await wait(() => expect(success).toHaveBeenCalledWith("Moved."));
        expect(h.host.querySelector('[aria-label="Activity"]')?.textContent).toContain("Move files");
    } finally {
        success.mockRestore();
        await h.dispose();
    }
});

it("読み取り専用はcutを無効にしcopyを許可する", async () => {
    const h = await harness("grid", [{ ...fileEntry, metadata: { ...fileEntry.metadata, readonly: true } }]);
    try {
        const option = h.host.querySelector<HTMLElement>('[role="option"]')!;
        await act(async () => option.click());
        await h.key("x", document.body, { ctrlKey: true });
        expect(option.style.opacity).toBe("");
        expect(h.host.querySelector<HTMLButtonElement>('button[aria-label="Paste"]')!.disabled).toBe(true);
        await act(async () => option.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, button: 2 })));
        const cut = [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].find(
            (item) => item.textContent === "Cut",
        )!;
        const copy = [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].find(
            (item) => item.textContent === "Copy",
        )!;
        expect(cut.getAttribute("aria-disabled")).toBe("true");
        expect(copy.getAttribute("aria-disabled")).not.toBe("true");
        await act(async () => copy.click());
        expect(h.host.querySelector<HTMLButtonElement>('button[aria-label="Paste"]')!.disabled).toBe(false);
    } finally {
        await h.dispose();
    }
});
