// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import { DEFAULT_PREFERENCES, type Preferences, preferencesQuery } from "../library/preferences";
import { PreferencesGate } from "./PreferencesGate";

const mocks = vi.hoisted(() => ({ get: vi.fn(), error: vi.fn() }));

vi.mock("../api/client", async (importOriginal) => ({
    ...(await importOriginal<typeof import("../api/client")>()),
    api: { GET: mocks.get },
}));
vi.mock("sonner", () => ({ toast: { error: mocks.error } }));

let host: HTMLDivElement;
let client: QueryClient;

beforeEach(() => {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    host = document.createElement("div");
    document.body.append(host);
    client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    mocks.get.mockReset();
    mocks.error.mockReset();
});

afterEach(() => host.remove());

/// `condition` が成り立つまで、再描画を流しながら待つ。負荷が高くても、固定の待ち時間に頼らない。
async function until(condition: () => boolean) {
    for (let attempt = 0; attempt < 100 && !condition(); attempt += 1) {
        await act(async () => {
            await new Promise((done) => setTimeout(done, 10));
        });
    }
}

async function render() {
    const root = createRoot(host);
    await act(async () =>
        root.render(
            <QueryClientProvider client={client}>
                <PreferencesGate>
                    <p data-child>一覧</p>
                </PreferencesGate>
            </QueryClientProvider>,
        ),
    );
    return root;
}

function response(data: Preferences) {
    return { data, response: { ok: true } };
}

it("shows only the skeleton while the preferences are loading, then the children", async () => {
    let resolve!: (value: ReturnType<typeof response>) => void;
    mocks.get.mockReturnValue(new Promise((done) => (resolve = done)));
    const root = await render();
    expect(host.querySelector("[data-child]")).toBeNull();
    expect(host.querySelector("[role=status]")).not.toBeNull();

    await act(async () => resolve(response(DEFAULT_PREFERENCES)));
    await until(() => host.querySelector("[data-child]") !== null);
    expect(host.querySelector("[data-child]")).not.toBeNull();
    expect(host.querySelector("[role=status]")).toBeNull();
    expect(mocks.error).not.toHaveBeenCalled();
    await act(async () => root.unmount());
});

it("falls back to the defaults and notifies when the preferences cannot be read", async () => {
    mocks.get.mockResolvedValue({ error: { kind: "InternalServerError" }, response: { ok: false, status: 500 } });
    const root = await render();
    await until(() => host.querySelector("[data-child]") !== null);
    expect(host.querySelector("[data-child]")).not.toBeNull();
    expect(client.getQueryData(preferencesQuery.queryKey)).toEqual(DEFAULT_PREFERENCES);
    expect(mocks.error).toHaveBeenCalledTimes(1);
    await act(async () => root.unmount());
});

it("leaves an unauthorized response to the common handler", async () => {
    mocks.get.mockResolvedValue({ error: { kind: "Unauthorized" }, response: { ok: false, status: 401 } });
    const root = await render();
    await until(() => client.getQueryState(preferencesQuery.queryKey)?.status === "error");
    expect(host.querySelector("[data-child]")).toBeNull();
    expect(client.getQueryData(preferencesQuery.queryKey)).toBeUndefined();
    expect(mocks.error).not.toHaveBeenCalled();
    await act(async () => root.unmount());
});
