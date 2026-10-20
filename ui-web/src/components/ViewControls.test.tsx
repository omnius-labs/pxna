// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";

import { DEFAULT_PREFERENCES, preferencesQuery } from "../library/preferences";
import { seedPreferences, stubMatchMedia, stubResizeObserver } from "../testing/preferences";
import { ViewControls } from "./ViewControls";

vi.mock("../api/client", async (original) => ({
    ...(await original<typeof import("../api/client")>()),
    api: { PUT: async (_: string, options: { body: unknown }) => ({ data: options.body, response: new Response() }) },
}));

it.each([false, true])("changes only the active view and hides grid size controls (narrow=%s)", async (narrow) => {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    const restore = stubMatchMedia(narrow);
    const restoreObserver = stubResizeObserver();
    const client = new QueryClient();
    seedPreferences(client);
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    try {
        await act(async () =>
            root.render(
                <QueryClientProvider client={client}>
                    <ViewControls />
                </QueryClientProvider>,
            ),
        );
        await act(async () => document.querySelector<HTMLButtonElement>('button[aria-label="表示"]')!.click());
        // 一覧の表示、タイルサイズ、並べ替えは、広い画面でも「表示」のメニューの中にある。
        expect(document.querySelector('[aria-label="一覧の表示"]')).not.toBeNull();
        expect(document.querySelector('[role="slider"]')).not.toBeNull();
        expect(document.querySelector('button[aria-label="並べ替え"]')).not.toBeNull();
        await act(async () => document.querySelector<HTMLButtonElement>('button[aria-label="リスト"]')!.click());
        const preferences = client.getQueryData(preferencesQuery.queryKey)!;
        expect(narrow ? preferences.narrow.view : preferences.wide.view).toBe("list");
        expect(narrow ? preferences.wide : preferences.narrow).toEqual(
            narrow ? DEFAULT_PREFERENCES.wide : DEFAULT_PREFERENCES.narrow,
        );
        await vi.waitFor(async () => {
            await act(async () => {
                await Promise.resolve();
            });
            expect(document.querySelector('[role="slider"]')).toBeNull();
        });
        await act(async () => {
            window.dispatchEvent(new KeyboardEvent("keydown", { key: "+" }));
            window.dispatchEvent(new WheelEvent("wheel", { ctrlKey: true, deltaY: -100 }));
        });
        expect(client.getQueryData(preferencesQuery.queryKey)!.wide.tileSize).toBe(DEFAULT_PREFERENCES.wide.tileSize);
    } finally {
        await act(async () => root.unmount());
        host.remove();
        client.clear();
        restore();
        restoreObserver();
    }
});
