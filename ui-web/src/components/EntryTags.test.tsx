// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import { EntryTags } from "./EntryTags";

const mocks = vi.hoisted(() => ({ get: vi.fn(), put: vi.fn() }));
vi.mock("../api/client", async (importOriginal) => ({
    ...(await importOriginal<typeof import("../api/client")>()),
    api: { GET: mocks.get, PUT: mocks.put },
}));

const ok = (data: unknown) => ({ data, response: new Response() });

/// 保存済みの値を持つ daemon の代わり。
function daemon(initial: { rating?: string; tags?: string[]; vocabulary?: string[] }) {
    const state = { rating: initial.rating ?? "none", tags: initial.tags ?? [], vocabulary: initial.vocabulary ?? [] };
    mocks.get.mockImplementation(async (route: string) => {
        if (route === "/libraries/{libraryId}/rating") return ok({ rating: state.rating });
        if (route === "/libraries/{libraryId}/tags") return ok({ tags: state.tags });
        return ok({ tags: state.vocabulary });
    });
    mocks.put.mockImplementation(async (_route: string, options: { body: { tags: string[] } }) => {
        state.tags = [...new Set(options.body.tags)].sort();
        state.vocabulary = [...new Set([...state.vocabulary, ...state.tags])].sort();
        return ok({ tags: state.tags });
    });
    return state;
}

let host: HTMLDivElement;
let root: ReturnType<typeof createRoot>;
let client: QueryClient;

beforeEach(async () => {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    host = document.createElement("div");
    document.body.append(host);
    root = createRoot(host);
    client = new QueryClient();
});
afterEach(async () => {
    await act(async () => root.unmount());
    host.remove();
    mocks.get.mockReset();
    mocks.put.mockReset();
});

async function render() {
    await act(async () =>
        root.render(
            <QueryClientProvider client={client}>
                <EntryTags libraryId="photos" path="/trip/a.png" />
            </QueryClientProvider>,
        ),
    );
    await vi.waitFor(() => expect(host.querySelector("input")?.disabled).toBe(false));
}
const input = () => host.querySelector<HTMLInputElement>('input[aria-label="Add a tag"]')!;
const type = async (value: string) => {
    await act(async () => {
        Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input(), value);
        input().dispatchEvent(new Event("input", { bubbles: true }));
    });
};
const enter = () =>
    act(async () =>
        input().dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true })),
    );
const chips = () => [...host.querySelectorAll("li span")].map((chip) => chip.textContent);

it("保存済みの評価とタグを出す", async () => {
    daemon({ rating: "good", tags: ["cat", "trip"] });
    await render();
    await vi.waitFor(() => expect(chips()).toEqual(["cat", "trip"]));
    expect(host.querySelector("[data-rating]")?.textContent).toBe("Good");
});

it("Enter で語を足し、足した語だけを含む集合を保存して、入力を空にする", async () => {
    daemon({ tags: ["cat"] });
    await render();
    await vi.waitFor(() => expect(chips()).toEqual(["cat"]));
    await type("  trip ");
    await enter();
    await vi.waitFor(() => expect(chips()).toEqual(["cat", "trip"]));
    expect(mocks.put).toHaveBeenCalledOnce();
    expect(mocks.put.mock.calls[0]?.[1].body).toEqual({ path: "/trip/a.png", tags: ["cat", "trip"] });
    expect(input().value).toBe("");
});

it("新しい語は付与したときに作られ、次から補完に出る", async () => {
    daemon({ vocabulary: ["dog"] });
    await render();
    const options = () =>
        [...host.querySelectorAll("datalist option")].map((option) => (option as HTMLOptionElement).value);
    await vi.waitFor(() => expect(options()).toEqual(["dog"]));
    await type("brand-new");
    await enter();
    await vi.waitFor(() => expect(chips()).toEqual(["brand-new"]));
    // 付けた語は補完から外れ、残りは語彙のままである。
    expect(options()).toEqual(["dog"]);
    expect(client.getQueryData(["tag-vocabulary"])).toEqual(["brand-new", "dog"]);
});

it("補完には、付いている語を出さない", async () => {
    daemon({ tags: ["cat"], vocabulary: ["cat", "dog"] });
    await render();
    await vi.waitFor(() => expect(chips()).toEqual(["cat"]));
    expect([...host.querySelectorAll("datalist option")].map((option) => (option as HTMLOptionElement).value)).toEqual([
        "dog",
    ]);
});

it("× で語を外すと、残りの集合を保存する", async () => {
    daemon({ tags: ["cat", "trip"] });
    await render();
    await vi.waitFor(() => expect(chips()).toEqual(["cat", "trip"]));
    await act(async () =>
        host.querySelector<HTMLButtonElement>("button[aria-label='Remove the tag \"cat\"']")!.click(),
    );
    await vi.waitFor(() => expect(chips()).toEqual(["trip"]));
    expect(mocks.put.mock.calls[0]?.[1].body.tags).toEqual(["trip"]);
});

it("付いている語、空の語、使えない語は送らない", async () => {
    daemon({ tags: ["cat"] });
    await render();
    await vi.waitFor(() => expect(chips()).toEqual(["cat"]));
    for (const value of ["cat", "   ", "a\u0007b", "あ".repeat(65)]) {
        await type(value);
        await enter();
    }
    expect(mocks.put).not.toHaveBeenCalled();
});

it("変換の確定の Enter は語の追加にしない", async () => {
    daemon({});
    await render();
    await type("かな");
    await act(async () =>
        input().dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, isComposing: true })),
    );
    expect(mocks.put).not.toHaveBeenCalled();
});

it("保存に失敗したら、入力と元の語を残す", async () => {
    daemon({ tags: ["cat"] });
    await render();
    await vi.waitFor(() => expect(chips()).toEqual(["cat"]));
    mocks.put.mockResolvedValue({
        error: { kind: "InvalidRequest", message: "x" },
        response: new Response(null, { status: 400 }),
    });
    await type("trip");
    await enter();
    await vi.waitFor(() => expect(mocks.put).toHaveBeenCalled());
    expect(chips()).toEqual(["cat"]);
    expect(input().value).toBe("trip");
});

it("付いている語を読み終えるまでは入力できず、保存済みの語を消さない", async () => {
    const state = daemon({ tags: ["cat"] });
    let release: (() => void) | undefined;
    mocks.get.mockImplementation(async (route: string) => {
        if (route === "/libraries/{libraryId}/tags") {
            await new Promise<void>((done) => {
                release = done;
            });
            return ok({ tags: state.tags });
        }
        return ok(route === "/libraries/{libraryId}/rating" ? { rating: "none" } : { tags: [] });
    });
    await act(async () =>
        root.render(
            <QueryClientProvider client={client}>
                <EntryTags libraryId="photos" path="/trip/a.png" />
            </QueryClientProvider>,
        ),
    );
    expect(input().disabled).toBe(true);
    await enter();
    expect(mocks.put).not.toHaveBeenCalled();
    await act(async () => release?.());
    await vi.waitFor(() => expect(input().disabled).toBe(false));
    expect(chips()).toEqual(["cat"]);
    await type("trip");
    await enter();
    await vi.waitFor(() => expect(chips()).toEqual(["cat", "trip"]));
});
