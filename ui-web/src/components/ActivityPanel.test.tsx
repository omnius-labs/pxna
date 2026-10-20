// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import { ActivityContext } from "../library/activity";
import type { Operation } from "../library/entryActions";
import { ActivityButton, ActivityPanel } from "./ActivityPanel";

const screen = vi.hoisted(() => ({ narrow: false }));
vi.mock("../library/preferences", () => ({ useNarrowScreen: () => screen.narrow }));
afterEach(() => {
    screen.narrow = false;
});
it("進捗、スキップ、失敗理由、取り消し、畳んだボタンを表示", async () => {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    const host = document.createElement("div");
    const root = createRoot(host);
    const cancel = vi.fn();
    const show = vi.fn();
    const collapse = vi.fn();
    const op: Operation = {
        id: "op",
        libraryId: "ops",
        kind: "delete_folders",
        status: "running",
        processedCount: 40,
        totalCount: 400,
        skippedCount: 2,
        failedEntries: [{ path: "/x", kind: "Forbidden", message: "denied" }],
        createdAt: "2026-10-07",
    };
    const render = async (open: boolean, operation = op) => {
        await act(async () =>
            root.render(
                <ActivityContext.Provider
                    value={{
                        isRead: () => false,
                        unreadCount: 1,
                        dismiss: () => {},
                        items: [{ kind: "operation", operation }],
                        open,
                        error: null,
                        cancel,
                        show,
                        collapse,
                        retry: () => {},
                    }}
                >
                    <ActivityPanel />
                    <ActivityButton />
                </ActivityContext.Provider>,
            ),
        );
    };
    try {
        await render(true);
        expect(host.textContent).toContain("40 / 400");
        expect(host.textContent).toContain("スキップ 2 件");
        expect(host.textContent).toContain("/x: denied");
        await act(async () => [...host.querySelectorAll("button")].find((b) => b.textContent === "取り消す")!.click());
        expect(cancel).toHaveBeenCalledWith("op");
        await render(true, { ...op, status: "cancelled" });
        expect(host.textContent).toContain("取り消した");
        expect(host.textContent).not.toContain("取り消す");
        await render(false);
        expect(host.querySelector("section")).toBeNull();
        expect(host.textContent).toContain("アクティビティ（1）");
    } finally {
        await act(async () => root.unmount());
    }
});
it("Operationとアップロードの状態・バイト進捗・失敗理由・取消を並べ、狭い画面で隠す", async () => {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    const host = document.createElement("div");
    const root = createRoot(host);
    const cancel = vi.fn();
    const statuses = ["pending", "running", "succeeded", "failed", "cancelled", "skipped"] as const;
    const items = statuses.map((status, index) => ({
        kind: "upload" as const,
        upload: {
            id: String(index),
            libraryId: "a",
            path: `/f${index}`,
            name: `f${index}`,
            size: 33 * 1024 * 1024,
            sent: 16 * 1024 * 1024,
            chunked: index === 1,
            status,
            error: "denied",
        },
    }));
    const operation: Operation = {
        id: "operation",
        kind: "copy_files",
        libraryId: "a",
        status: "succeeded",
        processedCount: 1,
        skippedCount: 0,
        failedEntries: [],
        createdAt: "2026-10-07",
    };
    const activity = {
        isRead: () => false,
        unreadCount: 7,
        dismiss: vi.fn(),
        items: [...items, { kind: "operation" as const, operation }],
        open: true,
        error: null,
        cancel: vi.fn(),
        cancelUpload: cancel,
        show: vi.fn(),
        collapse: vi.fn(),
        retry: vi.fn(),
    };
    try {
        await act(async () =>
            root.render(
                <ActivityContext.Provider value={activity}>
                    <ActivityPanel />
                    <ActivityButton />
                </ActivityContext.Provider>,
            ),
        );
        expect(host.textContent).toContain("ファイルのコピー");
        expect(host.textContent).toContain("16,777,216 / 34,603,008 バイト");
        expect(host.textContent).toContain("/f3: denied");
        for (const label of ["待機中", "送信中", "完了", "失敗", "取り消した", "スキップ"])
            expect(host.textContent).toContain(label);
        const buttons = host.querySelectorAll('button[aria-label*="アップロードを取り消す"]');
        expect(buttons).toHaveLength(2);
        await act(async () => (buttons[1] as HTMLButtonElement).click());
        expect(cancel).toHaveBeenCalledWith("1");
        expect(host.querySelectorAll("article p")[0]?.textContent).not.toContain("バイト");
        screen.narrow = true;
        await act(async () =>
            root.render(
                <ActivityContext.Provider value={{ ...activity }}>
                    <ActivityPanel />
                    <ActivityButton />
                </ActivityContext.Provider>,
            ),
        );
        expect(host.querySelector("section")).toBeNull();
        expect(host.querySelector("button")).toBeNull();
    } finally {
        await act(async () => root.unmount());
    }
});
