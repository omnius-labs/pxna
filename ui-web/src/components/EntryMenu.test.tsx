// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";
import { actionAvailability, EntryActionContext, type EntryActionState } from "../library/actionContext";
import { testEntry } from "../testing/entries";
import { EntryMenu, EntryToolbar } from "./EntryMenu";

const navigate = vi.hoisted(() => vi.fn());
vi.mock("@tanstack/react-router", () => ({ useNavigate: () => navigate }));

const state = (): EntryActionState => ({
    libraryId: "ops",
    libraryReadonly: false,
    narrow: false,
    canCreate: true,
    selection: { items: new Map(), anchor: undefined },
    setSelection: vi.fn(),
    create: vi.fn(),
    finish: vi.fn(),
});
it("選択と readonly で操作を制限", () => {
    const actions = { ...state(), rename: vi.fn(), delete: vi.fn() };
    expect(actionAvailability(actions)).toEqual({ create: true, rename: false, delete: false, download: false });
    actions.selection.items = new Map([["/a", { size: 1, folder: false, readonly: false, entry: testEntry("/a") }]]);
    expect(actionAvailability(actions).rename).toBe(true);
    actions.libraryReadonly = true;
    expect(actionAvailability(actions).delete).toBe(false);
    actions.libraryReadonly = false;
    actions.selection.items = new Map([["/a", { size: 1, folder: false, readonly: true, entry: testEntry("/a") }]]);
    expect(actionAvailability(actions).rename).toBe(false);
});
it("狭い画面でメニューと操作ボタンを出さず、右クリックで単独選択", async () => {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    const actions = state();
    const entry = { path: "/a", name: "a", kind: "file" as const, metadata: { size: 1, readonly: false } };
    const render = async (narrow: boolean) => {
        await act(async () =>
            root.render(
                <EntryActionContext.Provider value={{ ...actions, narrow }}>
                    <EntryMenu entry={entry}>
                        <span>entry</span>
                    </EntryMenu>
                    <EntryToolbar />
                </EntryActionContext.Provider>,
            ),
        );
    };
    try {
        await render(true);
        expect(host.querySelector("button")).toBeNull();
        await render(false);
        await act(async () =>
            host
                .querySelector('[data-slot="context-menu-trigger"]')!
                .dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, button: 2 })),
        );
        expect(actions.setSelection).toHaveBeenCalledWith({
            items: new Map([["/a", { size: 1, folder: false, readonly: false, entry }]]),
            anchor: undefined,
        });
        // 値が持つ Entry は、渡した Entry と同じ参照である。
        const selected = vi.mocked(actions.setSelection).mock.calls[0]?.[0].items.get("/a");
        expect(selected?.entry).toBe(entry);
    } finally {
        await act(async () => root.unmount());
        host.remove();
    }
});
it("ドラッグの開始が作る単独の選択も、その Entry と同じ参照を持つ", async () => {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    const actions = state();
    const entry = testEntry("/a");
    try {
        await act(async () =>
            root.render(
                <EntryActionContext.Provider value={actions}>
                    <EntryMenu entry={entry}>
                        <span>entry</span>
                    </EntryMenu>
                </EntryActionContext.Provider>,
            ),
        );
        const dragStart = new Event("dragstart", { bubbles: true, cancelable: true });
        Object.defineProperty(dragStart, "dataTransfer", { value: { setData: vi.fn(), effectAllowed: "none" } });
        await act(async () => host.querySelector('[data-slot="context-menu-trigger"]')?.dispatchEvent(dragStart));
        const selected = vi.mocked(actions.setSelection).mock.calls[0]?.[0].items.get("/a");
        expect(selected?.entry).toBe(entry);
    } finally {
        await act(async () => root.unmount());
        host.remove();
    }
});
it("ファイルのメニューとツールバーに download 属性付きリンクを出す", async () => {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    const actions = { ...state(), downloadHref: "/api/v1/libraries/ops/files/content?path=%2Fa" };
    actions.selection.items = new Map([["/a", { size: 1, folder: false, readonly: true, entry: testEntry("/a") }]]);
    try {
        await act(async () =>
            root.render(
                <EntryActionContext.Provider value={actions}>
                    <EntryMenu>
                        <span>file</span>
                    </EntryMenu>
                    <EntryToolbar />
                </EntryActionContext.Provider>,
            ),
        );
        expect(host.querySelector("a[download]")?.getAttribute("href")).toBe(actions.downloadHref);
        await act(async () =>
            host
                .querySelector('[data-slot="context-menu-trigger"]')!
                .dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, button: 2 })),
        );
        expect(document.querySelector('a[role="menuitem"][download]')?.getAttribute("href")).toBe(actions.downloadHref);
    } finally {
        await act(async () => root.unmount());
        host.remove();
    }
});
it.each([false, true])("切り取りはreadonlyで無効、コピーは選択があれば実行できる %s", async (selected) => {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    const actions = { ...state(), libraryReadonly: true, cut: vi.fn(), copy: vi.fn() };
    if (selected)
        actions.selection.items = new Map([["/a", { size: 1, folder: false, readonly: true, entry: testEntry("/a") }]]);
    try {
        await act(async () =>
            root.render(
                <EntryActionContext.Provider value={actions}>
                    <EntryMenu>
                        <span>empty</span>
                    </EntryMenu>
                </EntryActionContext.Provider>,
            ),
        );
        for (const [label, callback] of [
            ["Cut", actions.cut],
            ["Copy", actions.copy],
        ] as const) {
            await act(async () =>
                host
                    .querySelector('[data-slot="context-menu-trigger"]')!
                    .dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, button: 2 })),
            );
            const item = [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].find(
                (item) => item.textContent === label,
            )!;
            const enabled = selected && label === "Copy";
            expect(item.getAttribute("aria-disabled")).toBe(enabled ? null : "true");
            await act(async () => item.click());
            expect(callback).toHaveBeenCalledTimes(enabled ? 1 : 0);
            await act(async () => item.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
        }
    } finally {
        await act(async () => root.unmount());
        host.remove();
    }
});
it.each([false, true])(
    "貼り付けは readonly の宛先で無効、フォルダメニューはそのフォルダへ送る %s",
    async (readonly) => {
        const { TransferContext } = await import("../library/transfer");
        Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
        const host = document.createElement("div");
        document.body.append(host);
        const root = createRoot(host);
        const actions = { ...state(), folderPath: "/current", canCreate: !readonly, libraryReadonly: readonly };
        const transfer = {
            busy: false,
            canPaste: true,
            paste: vi.fn().mockResolvedValue(undefined),
            transfer: vi.fn().mockResolvedValue(undefined),
        };
        const entry = {
            path: "/target",
            name: "target",
            kind: "folder" as const,
            metadata: { size: 0, readonly: false },
        };
        try {
            await act(async () =>
                root.render(
                    <TransferContext.Provider value={transfer}>
                        <EntryActionContext.Provider value={actions}>
                            <EntryMenu entry={entry}>
                                <span>folder</span>
                            </EntryMenu>
                            <EntryToolbar />
                        </EntryActionContext.Provider>
                    </TransferContext.Provider>,
                ),
            );
            const button = host.querySelector<HTMLButtonElement>('button[aria-label="Paste"]')!;
            expect(button.disabled).toBe(readonly);
            await act(async () => button.click());
            if (!readonly) expect(transfer.paste).toHaveBeenLastCalledWith({ libraryId: "ops", path: "/current" });
            await act(async () =>
                host
                    .querySelector('[data-slot="context-menu-trigger"]')!
                    .dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, button: 2 })),
            );
            const item = [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].find(
                (item) => item.textContent === "Paste",
            )!;
            expect(item.getAttribute("aria-disabled")).toBe(readonly ? "true" : null);
            await act(async () => item.click());
            if (readonly) expect(transfer.paste).not.toHaveBeenCalled();
            else
                await vi.waitFor(() =>
                    expect(transfer.paste).toHaveBeenLastCalledWith({ libraryId: "ops", path: "/target" }),
                );
        } finally {
            await act(async () => root.unmount());
            host.remove();
        }
    },
);

async function openMenu(actions: EntryActionState, entry?: Parameters<typeof EntryMenu>[0]["entry"]) {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    await act(async () =>
        root.render(
            <EntryActionContext.Provider value={actions}>
                <EntryMenu entry={entry}>
                    <span>entry</span>
                </EntryMenu>
            </EntryActionContext.Provider>,
        ),
    );
    await act(async () =>
        host
            .querySelector('[data-slot="context-menu-trigger"]')!
            .dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, button: 2 })),
    );
    const items = () => [...document.querySelectorAll('[role="menuitem"]')].map((item) => item.textContent);
    return {
        items,
        select: (label: string) =>
            act(async () => {
                [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')]
                    .find((item) => item.textContent === label)
                    ?.click();
            }),
        dispose: async () => {
            await act(async () => root.unmount());
            host.remove();
        },
    };
}
it("フォルダ表示の空き領域のメニューは、そのフォルダを起点にした検索を開く", async () => {
    navigate.mockReset();
    const menu = await openMenu({ ...state(), folderPath: "/a/b" });
    try {
        expect(menu.items()).toContain("Search in this folder");
        await menu.select("Search in this folder");
        expect(navigate).toHaveBeenCalledWith({ href: "/search?lib=ops&path=%2Fa%2Fb" });
    } finally {
        await menu.dispose();
    }
});
it("Entry のメニューと、Library を横断する画面のメニューには、フォルダ内の検索を出さない", async () => {
    const entry = { path: "/a", name: "a", kind: "file" as const, metadata: { size: 1, readonly: false } };
    const onEntry = await openMenu({ ...state(), folderPath: "/" }, entry);
    try {
        expect(onEntry.items()).not.toContain("Search in this folder");
    } finally {
        await onEntry.dispose();
    }
    const cross = await openMenu({ ...state(), libraryId: "*", crossLibrary: true });
    try {
        expect(cross.items()).not.toContain("Search in this folder");
        expect(cross.items()).toContain("Show in folder");
    } finally {
        await cross.dispose();
    }
});
