// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";
import { ApiError } from "../api/client";
import { ActivityContext } from "../library/activity";
import { DeleteEntriesError, type Operation } from "../library/entryActions";
import { DeleteDialog } from "./DeleteDialog";

const { remove, success, error } = vi.hoisted(() => ({ remove: vi.fn(), success: vi.fn(), error: vi.fn() }));
vi.mock("../library/entryActions", async (original) => ({
    ...(await original<typeof import("../library/entryActions")>()),
    deleteEntries: remove,
}));
vi.mock("sonner", () => ({ toast: { success, error } }));
const op = (id: string): Operation => ({
    id,
    libraryId: "ops",
    kind: "delete_folders",
    status: "pending",
    processedCount: 0,
    skippedCount: 0,
    failedEntries: [],
    createdAt: "2026-10-07",
});
async function harness(extra: { container?: HTMLElement; toasters?: { success?: string; failure?: string } } = {}) {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    const client = new QueryClient();
    const show = vi.fn();
    const close = vi.fn();
    const deleted = vi.fn();
    await act(async () =>
        root.render(
            <QueryClientProvider client={client}>
                <ActivityContext.Provider
                    value={{
                        isRead: () => false,
                        unreadCount: 0,
                        dismiss: () => {},
                        items: [],
                        open: false,
                        error: null,
                        show,
                        collapse: () => {},
                        cancel: async () => {},
                        retry: () => {},
                    }}
                >
                    <DeleteDialog
                        target={{
                            libraryId: "ops",
                            items: new Map([
                                ["/a", { folder: false }],
                                ["/dir", { folder: true }],
                            ]),
                        }}
                        onClose={close}
                        onDeleted={deleted}
                        {...extra}
                    />
                </ActivityContext.Provider>
            </QueryClientProvider>,
        ),
    );
    const click = async (text: string) => {
        await act(async () =>
            [...document.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === text)!.click(),
        );
    };
    return {
        show,
        close,
        deleted,
        click,
        dispose: async () => {
            await act(async () => root.unmount());
            host.remove();
            client.clear();
            remove.mockReset();
            success.mockReset();
            error.mockReset();
        },
    };
}
it("確認の件数・不可逆性・中身を示し、取り消しでは送信しない", async () => {
    const h = await harness();
    try {
        expect(document.body.textContent).toContain("2 件");
        expect(document.body.textContent).toContain("取り消せません");
        expect(document.body.textContent).toContain("中身ごと");
        expect(remove).not.toHaveBeenCalled();
        await h.click("取り消す");
        expect(remove).not.toHaveBeenCalled();
        expect(h.close).toHaveBeenCalled();
    } finally {
        await h.dispose();
    }
});
it.each([false, true])("確定でのみ送信し、同期はトースト、非同期は全Operationを表示 %s", async (asyncOperation) => {
    const h = await harness();
    const operations = asyncOperation ? [op("files"), op("folders")] : [];
    try {
        remove.mockResolvedValue(operations);
        await h.click("削除する");
        expect(remove).toHaveBeenCalledOnce();
        expect(h.deleted).toHaveBeenCalledOnce();
        if (asyncOperation) expect(h.show).toHaveBeenCalledWith(operations);
        else expect(success).toHaveBeenCalledWith("削除しました。");
    } finally {
        await h.dispose();
    }
});
it("片方失敗でも成功Operationを表示し、失敗を知らせる", async () => {
    const h = await harness();
    try {
        remove.mockRejectedValue(new DeleteEntriesError([new ApiError(403, "Forbidden", "denied")], [op("files")], 1));
        await h.click("削除する");
        expect(h.show).toHaveBeenCalledWith([op("files")]);
        expect(error).toHaveBeenCalledOnce();
        expect(h.deleted).toHaveBeenCalledOnce();
    } finally {
        await h.dispose();
    }
});

it("渡した描画先へダイアログを描き、結果のトーストを指定した Toaster に出す", async () => {
    const container = document.createElement("div");
    document.body.append(container);
    const h = await harness({ container, toasters: { success: "viewer", failure: "viewer" } });
    try {
        const dialog = document.querySelector("[role=alertdialog]");
        expect(dialog).not.toBeNull();
        expect(container.contains(dialog)).toBe(true);
        remove.mockResolvedValue([]);
        await h.click("削除する");
        expect(success).toHaveBeenCalledExactlyOnceWith("削除しました。", { toasterId: "viewer" });
    } finally {
        await h.dispose();
        container.remove();
    }
});

it("失敗のトーストだけを指定した Toaster に出し、成功のトーストはルートの Toaster に出す", async () => {
    const h = await harness({ toasters: { failure: "viewer" } });
    try {
        remove.mockRejectedValue(new ApiError(403, "Forbidden", "denied"));
        await h.click("削除する");
        expect(error).toHaveBeenCalledExactlyOnceWith("削除できない項目があります。", {
            description: "denied",
            toasterId: "viewer",
        });
    } finally {
        await h.dispose();
    }
});

it("描画先を渡さなければ、body の直下に描く", async () => {
    const h = await harness();
    try {
        expect(document.querySelector("[role=alertdialog]")?.parentElement).toBe(document.body);
    } finally {
        await h.dispose();
    }
});

it("成功のトーストの出し先を指定しなければ、失敗の出し先だけを指定しても、ルートの Toaster に出す", async () => {
    const h = await harness({ toasters: { failure: "viewer" } });
    try {
        remove.mockResolvedValue([]);
        await h.click("削除する");
        expect(success).toHaveBeenCalledExactlyOnceWith("削除しました。");
    } finally {
        await h.dispose();
    }
});
