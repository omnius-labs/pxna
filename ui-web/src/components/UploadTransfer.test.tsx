// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";
import { api } from "../api/client";
import { useTransfer } from "../library/transfer";
import { UploadContext, UploadStore } from "../library/upload";
import { TransferProvider } from "./TransferProvider";

vi.mock("../api/client", async (original) => ({
    ...(await original<typeof import("../api/client")>()),
    api: { GET: vi.fn() },
}));
vi.mock("../library/preferences", () => ({ useNarrowScreen: () => false }));
it.each(["両方残す", "スキップ", "置き換える", "中止"])(
    "アップロードは衝突を確認し %s を一度だけ enqueue へ反映する",
    async (choice) => {
        Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
        const response = (data: unknown) => ({ data, response: new Response(null, { status: 200 }) });
        vi.mocked(api.GET).mockImplementation(async (route: string) => {
            if (route === "/libraries") return response({ libraries: [{ id: "dest", readonly: false }] }) as never;
            if (route.endsWith("/folders")) return response({ folder: { metadata: { readonly: false } } }) as never;
            return response({ entries: [{ name: "same.txt" }], totalCount: 1, snapshot: "s" }) as never;
        });
        const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
        const store = new UploadStore();
        const enqueue = vi.spyOn(store, "enqueue").mockImplementation(() => {});
        const input = [{ path: "same.txt", file: new File(["data"], "same.txt") }];
        function Start() {
            const transfer = useTransfer();
            return (
                <button
                    type="button"
                    onClick={() => {
                        void transfer?.upload?.(input, { libraryId: "dest", path: "/" });
                    }}
                >
                    start
                </button>
            );
        }
        const host = document.createElement("div");
        document.body.append(host);
        const root = createRoot(host);
        try {
            await act(async () =>
                root.render(
                    <QueryClientProvider client={client}>
                        <UploadContext.Provider value={store}>
                            <TransferProvider>
                                <Start />
                            </TransferProvider>
                        </UploadContext.Provider>
                    </QueryClientProvider>,
                ),
            );
            await act(async () => host.querySelector<HTMLButtonElement>("button")!.click());
            await vi.waitFor(async () => {
                await act(async () => {});
                expect(document.querySelector('[role="alertdialog"]')?.textContent).toContain("same.txt");
            });
            expect(enqueue).not.toHaveBeenCalled();
            await act(async () =>
                [...document.querySelectorAll<HTMLButtonElement>('[role="alertdialog"] button')]
                    .find((button) => button.textContent === choice)!
                    .click(),
            );
            if (choice === "中止") expect(enqueue).not.toHaveBeenCalled();
            else
                expect(enqueue).toHaveBeenCalledExactlyOnceWith(
                    input,
                    { libraryId: "dest", path: "/" },
                    { 両方残す: "rename", スキップ: "skip", 置き換える: "overwrite" }[choice],
                );
        } finally {
            await act(async () => root.unmount());
            host.remove();
            client.clear();
            vi.clearAllMocks();
        }
    },
);
