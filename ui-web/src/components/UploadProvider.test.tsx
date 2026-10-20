// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";
import { useUploads } from "../library/upload";
import { UploadProvider } from "./UploadProvider";

const { refresh } = vi.hoisted(() => ({ refresh: vi.fn().mockResolvedValue(undefined) }));
vi.mock("../library/listing", () => ({ refreshLibrary: refresh }));
it("完了したファイルごとに同じQueryClientでLibraryを一度だけ読み直す", async () => {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    const client = new QueryClient();
    const host = document.createElement("div");
    const root = createRoot(host);
    let send!: () => void;
    function Child() {
        const uploads = useUploads();
        send = () =>
            uploads.store!.enqueue(
                [{ path: "a", file: new File(["a"], "a") }],
                { libraryId: "dest", path: "/" },
                "fail",
            );
        return <p>{uploads.items[0]?.status}</p>;
    }
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(null, { status: 201 })));
    try {
        await act(async () =>
            root.render(
                <QueryClientProvider client={client}>
                    <UploadProvider>
                        <Child />
                    </UploadProvider>
                </QueryClientProvider>,
            ),
        );
        await act(async () => send());
        await vi.waitFor(async () => {
            await act(async () => {});
            expect(host.textContent).toBe("succeeded");
        });
        expect(refresh).toHaveBeenCalledExactlyOnceWith(client, "dest");
        await act(async () =>
            root.render(
                <QueryClientProvider client={client}>
                    <UploadProvider>
                        <Child />
                    </UploadProvider>
                </QueryClientProvider>,
            ),
        );
        expect(refresh).toHaveBeenCalledOnce();
    } finally {
        await act(async () => root.unmount());
        client.clear();
        refresh.mockClear();
        vi.unstubAllGlobals();
    }
});
