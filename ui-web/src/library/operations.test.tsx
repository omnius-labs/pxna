// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";
import type { Operation } from "./entryActions";
import { cancelOperation, operationsQuery, useOperations } from "./operations";

const { get, post, refresh } = vi.hoisted(() => ({ get: vi.fn(), post: vi.fn(), refresh: vi.fn() }));
vi.mock("../api/client", async (original) => ({
    ...(await original<typeof import("../api/client")>()),
    api: { GET: get, POST: post },
}));
vi.mock("./listing", () => ({ refreshLibrary: refresh }));
const op = (status: Operation["status"]): Operation => ({
    id: "op",
    libraryId: "ops",
    destinationLibraryId: "dest",
    kind: "delete_folders",
    status,
    processedCount: 0,
    skippedCount: 0,
    failedEntries: [],
    createdAt: "2026-10-07",
});
it("実行中だけ1秒で再取得し、終端の Library を一度だけ更新", async () => {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    vi.useFakeTimers();
    const client = new QueryClient();
    const root = createRoot(document.createElement("div"));
    let status: Operation["status"] = "running";
    get.mockImplementation(async () => ({ data: { operations: [op(status)] }, response: new Response() }));
    function Harness() {
        useOperations();
        return null;
    }
    try {
        await act(async () =>
            root.render(
                <QueryClientProvider client={client}>
                    <Harness />
                </QueryClientProvider>,
            ),
        );
        await act(async () => vi.advanceTimersByTimeAsync(3000));
        expect(get.mock.calls.length).toBeGreaterThanOrEqual(3);
        status = "succeeded";
        await act(async () => vi.advanceTimersByTimeAsync(1100));
        expect(refresh.mock.calls.map(([, id]) => id)).toEqual(["ops", "dest"]);
        const count = get.mock.calls.length;
        await act(async () => vi.advanceTimersByTimeAsync(3000));
        expect(get).toHaveBeenCalledTimes(count);
        await act(async () => {
            await client.refetchQueries({ queryKey: operationsQuery.queryKey });
            await vi.advanceTimersByTimeAsync(1);
        });
        expect(refresh).toHaveBeenCalledTimes(2);
    } finally {
        await act(async () => root.unmount());
        client.clear();
        vi.useRealTimers();
        get.mockReset();
        refresh.mockReset();
    }
});
it("取り消しを API に送る", async () => {
    post.mockResolvedValue({ data: { operation: op("cancelled") }, response: new Response() });
    expect((await cancelOperation("op")).status).toBe("cancelled");
    expect(post.mock.calls[0]![1].params.path).toEqual({ operationId: "op" });
    post.mockReset();
});
