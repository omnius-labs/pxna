// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";

import type { Operation } from "../library/entryActions";
import { ActivityPanel } from "./ActivityPanel";
import { ActivityProvider } from "./ActivityProvider";
import { StatusBar } from "./StatusBar";

const { get } = vi.hoisted(() => ({ get: vi.fn() }));
vi.mock("../api/client", async (original) => ({
    ...(await original<typeof import("../api/client")>()),
    api: { GET: get },
}));
vi.mock("../library/preferences", () => ({ useNarrowScreen: () => false }));
afterEach(() => get.mockReset());

const operation = (id: string, status: Operation["status"]): Operation => ({
    id,
    libraryId: "ops",
    kind: "delete_files",
    status,
    processedCount: 1,
    totalCount: 1,
    skippedCount: 0,
    failedEntries: [],
    createdAt: "2026-10-16",
});

async function render(operations: Operation[]) {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    get.mockResolvedValue({ data: { operations }, response: new Response() });
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    const client = new QueryClient();
    await act(async () =>
        root.render(
            <QueryClientProvider client={client}>
                <ActivityProvider>
                    <ActivityPanel />
                    <StatusBar totalCount={0} selectedCount={0} selectedBytes={0} />
                </ActivityProvider>
            </QueryClientProvider>,
        ),
    );
    await vi.waitFor(() => expect(host.querySelectorAll("article")).toHaveLength(operations.length));
    const click = async (element: Element | null | undefined) => {
        await act(async () => (element as HTMLElement).click());
    };
    return {
        host,
        client,
        articles: () => [...host.querySelectorAll("article")],
        button: (label: string) => host.querySelector(`button[aria-label="${label}"]`),
        footerButton: () =>
            [...host.querySelectorAll("footer button")].find((b) => b.textContent?.startsWith("Activity")),
        click,
        dispose: async () => {
            await act(async () => root.unmount());
            host.remove();
            client.clear();
        },
    };
}

it("畳んだ後に開き直すと、見せた項目が既読になって薄くなり、未読の数にはその後の項目だけが入る", async () => {
    const h = await render([operation("a", "succeeded"), operation("b", "failed")]);
    try {
        expect(h.articles().every((article) => !article.className.includes("opacity-60"))).toBe(true);
        await h.click(h.button("Collapse activity"));
        expect(h.footerButton()?.textContent).toBe("Activity (2)");
        await h.click(h.footerButton());
        expect(h.articles()).toHaveLength(2);
        expect(h.articles().every((article) => article.className.includes("opacity-60"))).toBe(true);
        await h.click(h.button("Collapse activity"));
        expect(h.footerButton()?.textContent).toBe("Activity");
    } finally {
        await h.dispose();
    }
});

it("終わった項目は × で消せ、進行中の項目には × を出さない", async () => {
    const h = await render([operation("done", "succeeded"), operation("busy", "running")]);
    try {
        const dismiss = h.host.querySelectorAll('button[aria-label="Dismiss this item"]');
        expect(dismiss).toHaveLength(1);
        await h.click(dismiss[0]);
        expect(h.articles()).toHaveLength(1);
        expect(h.articles()[0]?.textContent).toContain("Running");
        expect(h.host.querySelector('button[aria-label="Dismiss this item"]')).toBeNull();
    } finally {
        await h.dispose();
    }
});

it("進行中の項目は、既読になっても薄くならない", async () => {
    const h = await render([operation("busy", "running")]);
    try {
        await h.click(h.button("Collapse activity"));
        await h.click(h.footerButton());
        expect(h.articles()[0]?.className).not.toContain("opacity-60");
    } finally {
        await h.dispose();
    }
});
