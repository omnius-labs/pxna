// @vitest-environment jsdom
import { QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import { act, type ReactNode } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import type { Entry } from "../library/entries";
import { librariesQuery } from "../library/queries";
import type { Selection } from "../library/selection";
import { testEntry } from "../testing/entries";
import { stubMatchMedia, stubResizeObserver } from "../testing/preferences";
import { loadRouterAt } from "../testing/router";

const mock = vi.hoisted(() => ({
    get: vi.fn(),
    post: vi.fn(),
    status: { running: false, processedFiles: 0 } as Record<string, unknown>,
}));
vi.mock("../api/client", async (original) => ({
    ...(await original<typeof import("../api/client")>()),
    api: { GET: mock.get, POST: mock.post },
}));
vi.mock("../components/AppShell", () => ({
    AppShell: ({
        children,
        footer,
        actions,
        sidebar,
    }: {
        children: ReactNode;
        footer: ReactNode;
        actions: ReactNode;
        sidebar?: (close: () => void) => ReactNode;
    }) => (
        <>
            <nav>{sidebar?.(() => {})}</nav>
            <div data-actions>{actions}</div>
            {children}
            {footer}
        </>
    ),
}));
vi.mock("../components/StatusBar", () => ({
    StatusBar: ({ totalCount, selectedCount }: { totalCount: number; selectedCount: number }) => (
        <footer data-status>{`${totalCount}:${selectedCount}`}</footer>
    ),
}));
vi.mock("../components/ViewControls", () => ({
    ViewControls: ({ timeline }: { timeline?: boolean }) => <i data-controls={String(!!timeline)} />,
}));
vi.mock("../components/SettingsDialog", () => ({ SettingsDialog: () => null }));
vi.mock("../components/InfoPanel", () => ({ InfoPanel: () => <aside data-info />, InfoPanelToggle: () => null }));
vi.mock("../components/TimelineGrid", () => ({
    TimelineGrid: ({
        store,
        onOpen,
        onSelectionChange,
        onVisibleMonth,
        emptyMessage,
    }: {
        store: { anchor: unknown; loadedItems: { entry: Entry }[] };
        onOpen: (entry: Entry) => void;
        onSelectionChange: (selection: Selection) => void;
        onVisibleMonth: (month: string) => void;
        emptyMessage: string;
    }) => (
        <div data-grid data-anchor={JSON.stringify(store.anchor)} data-empty={emptyMessage}>
            {store.loadedItems.map(({ entry }) => (
                <button
                    key={entry.path}
                    type="button"
                    data-tile={entry.path}
                    onClick={() =>
                        onSelectionChange({
                            items: new Map([[entry.path, { size: 1, folder: false, readonly: false, entry }]]),
                            anchor: 0,
                        })
                    }
                    onDoubleClick={() => onOpen(entry)}
                />
            ))}
            <button type="button" data-visible onClick={() => onVisibleMonth("2024-04")} />
        </div>
    ),
}));
vi.mock("../components/Viewer", () => ({
    Viewer: () => null,
    TimelineViewer: ({ libraryId, view }: { libraryId: string; view: string }) => (
        <span data-viewer>{`${libraryId}:${view}`}</span>
    ),
}));

const entry = (path: string) => ({
    entry: testEntry(path, { metadata: { contentType: "image/png" } }),
    timelineDate: "2024-05-01T00:00:00",
    timelineDateSource: "exif",
});

beforeEach(() => {
    mock.status = { running: false, processedFiles: 0 };
    mock.get.mockImplementation((path: string, options: { params: { query?: Record<string, string> } }) => {
        if (path.endsWith("/timeline/entries"))
            return Promise.resolve({
                data: { entries: [entry(options.params.query?.month === "2024-04" ? "/old.png" : "/dir/a.png")] },
                response: new Response(),
            });
        if (path.endsWith("/timeline/months"))
            return Promise.resolve({
                data: {
                    months: [
                        { month: "2024-05", count: 3 },
                        { month: "2024-04", count: 2 },
                    ],
                },
                response: new Response(),
            });
        if (path.endsWith("/timeline/status")) return Promise.resolve({ data: mock.status, response: new Response() });
        return new Promise(() => {});
    });
    mock.post.mockResolvedValue({ data: { running: true, processedFiles: 0 }, response: new Response() });
});
afterEach(() => {
    mock.get.mockReset();
    mock.post.mockReset();
});

async function harness(href = "/timeline/photos", libraries = [{ id: "photos", name: "Photos", readonly: false }]) {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    const restoreMedia = stubMatchMedia(false);
    const restoreObserver = stubResizeObserver();
    const router = await loadRouterAt(href);
    const client = router.options.context.queryClient;
    client.setQueryData(librariesQuery.queryKey, libraries);
    await act(async () =>
        root.render(
            <QueryClientProvider client={client}>
                <RouterProvider router={router} />
            </QueryClientProvider>,
        ),
    );
    await vi.waitFor(() => expect(host.querySelector("[data-grid]")).not.toBeNull());
    return {
        host,
        router,
        entryQueries: () =>
            mock.get.mock.calls.filter(([path]) => path.endsWith("/timeline/entries")).map(([, o]) => o.params.query),
        dispose: async () => {
            await act(async () => root.unmount());
            host.remove();
            router.history.destroy();
            client.clear();
            restoreMedia();
            restoreObserver();
        },
    };
}

it("reads from the newest entry and lists the months with their counts", async () => {
    const h = await harness();
    try {
        expect(JSON.parse(h.host.querySelector("[data-grid]")?.getAttribute("data-anchor") ?? "")).toEqual({
            kind: "newest",
        });
        await vi.waitFor(() => expect(h.host.textContent).toContain("5月"));
        expect(h.host.textContent).toContain("2024年");
        expect(h.host.querySelector("[data-controls]")?.getAttribute("data-controls")).toBe("true");
        expect(h.host.querySelector("[data-status]")?.textContent).toBe("1:0");
    } finally {
        await h.dispose();
    }
});

it("reads again from a month when it is pressed and puts the month in the URL", async () => {
    const h = await harness();
    try {
        await vi.waitFor(() => expect(h.host.textContent).toContain("4月"));
        const april = [...h.host.querySelectorAll("a")].find((link) =>
            link.getAttribute("href")?.includes("month=2024-04"),
        );
        await act(async () => april?.click());
        await vi.waitFor(() => expect(h.host.querySelector("[data-tile='/old.png']")).not.toBeNull());
        expect(h.router.state.location.search).toMatchObject({ month: "2024-04" });
        expect(h.entryQueries()).toContainEqual(expect.objectContaining({ month: "2024-04" }));
        expect(JSON.parse(h.host.querySelector("[data-grid]")?.getAttribute("data-anchor") ?? "")).toMatchObject({
            kind: "month",
            month: "2024-04",
        });
    } finally {
        await h.dispose();
    }
});

it("does not read again when the visible month is written to the URL", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const h = await harness();
    try {
        const before = h.entryQueries().length;
        await act(async () => h.host.querySelector<HTMLElement>("[data-visible]")?.click());
        await act(async () => {
            await vi.advanceTimersByTimeAsync(500);
        });
        expect(h.router.state.location.search).toMatchObject({ month: "2024-04" });
        expect(h.entryQueries()).toHaveLength(before);
        expect(JSON.parse(h.host.querySelector("[data-grid]")?.getAttribute("data-anchor") ?? "")).toEqual({
            kind: "newest",
        });
    } finally {
        vi.useRealTimers();
        await h.dispose();
    }
});

it("reads again from the month when the month that is already shown is pressed", async () => {
    const h = await harness("/timeline/photos?month=2024-05");
    try {
        await vi.waitFor(() => expect(h.host.textContent).toContain("5月"));
        const before = h.entryQueries().length;
        const may = () =>
            [...h.host.querySelectorAll("a")].find((link) => link.getAttribute("href")?.includes("month=2024-05"));
        await act(async () => may()?.click());
        await vi.waitFor(() => expect(h.entryQueries().length).toBeGreaterThan(before));
        expect(h.entryQueries().at(-1)).toMatchObject({ month: "2024-05" });
        const again = h.entryQueries().length;
        await act(async () => may()?.click());
        await vi.waitFor(() => expect(h.entryQueries().length).toBeGreaterThan(again));
    } finally {
        await h.dispose();
    }
});

it("keeps the month that was pressed even when the old range reported its visible month", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const h = await harness();
    try {
        await act(async () => h.host.querySelector<HTMLElement>("[data-visible]")?.click());
        await vi.waitFor(() => expect(h.host.textContent).toContain("4月"));
        const april = [...h.host.querySelectorAll("a")].find((link) =>
            link.getAttribute("href")?.includes("month=2024-05"),
        );
        await act(async () => april?.click());
        await act(async () => {
            await vi.advanceTimersByTimeAsync(800);
        });
        expect(h.router.state.location.search).toMatchObject({ month: "2024-05" });
    } finally {
        vi.useRealTimers();
        await h.dispose();
    }
});

it("opens an image in the viewer and the parent folder with the entry revealed", async () => {
    const h = await harness();
    try {
        const tile = h.host.querySelector<HTMLElement>("[data-tile='/dir/a.png']");
        await act(async () => tile?.dispatchEvent(new MouseEvent("dblclick", { bubbles: true })));
        expect(h.router.state.location.href).toBe("/timeline/photos?view=dir%2Fa.png");
        await vi.waitFor(() => expect(h.host.querySelector("[data-viewer]")?.textContent).toBe("photos:dir/a.png"));
        await act(async () => h.router.navigate({ href: "/timeline/photos" }));

        await act(async () => h.host.querySelector<HTMLElement>("[data-tile='/dir/a.png']")?.click());
        const reveal = h.host.querySelector<HTMLButtonElement>("button[aria-label='フォルダで表示']");
        expect(reveal?.disabled).toBe(false);
        await act(async () => reveal?.click());
        expect(h.router.state.location.href).toBe("/browse/photos/dir?reveal=a.png");
    } finally {
        await h.dispose();
    }
});

it("shows that a scan is running and starts one on request", async () => {
    mock.status = { running: true, processedFiles: 42 };
    const h = await harness();
    try {
        await vi.waitFor(() => expect(h.host.textContent).toContain("集計中… 42 件を処理しました。"));
        expect(h.host.querySelector("[data-grid]")?.getAttribute("data-empty")).toContain("集計中");
        await act(async () => h.host.querySelector<HTMLElement>("button[aria-label='今すぐ更新']")?.click());
        expect(mock.post).toHaveBeenCalledWith("/libraries/{libraryId}/timeline/refresh", {
            params: { path: { libraryId: "photos" } },
        });
    } finally {
        await h.dispose();
    }
});

it("hides the operations that need a current folder", async () => {
    const h = await harness();
    try {
        expect(h.host.querySelector("button[aria-label='新しいフォルダ']")).toBeNull();
        expect(h.host.querySelector("button[aria-label='貼り付け']")).toBeNull();
        expect(h.host.querySelector("button[aria-label='削除']")).not.toBeNull();
    } finally {
        await h.dispose();
    }
});
