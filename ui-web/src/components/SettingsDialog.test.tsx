// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import { DEFAULT_PREFERENCES, preferencesQuery } from "../library/preferences";
import { seedPreferences, stubMatchMedia, stubResizeObserver } from "../testing/preferences";
import { SettingsDialog } from "./SettingsDialog";

const mocks = vi.hoisted(() => ({ put: vi.fn() }));

vi.mock("../api/client", async (importOriginal) => ({
    ...(await importOriginal<typeof import("../api/client")>()),
    api: { PUT: mocks.put },
}));

let host: HTMLDivElement;
let root: Root;
let client: QueryClient;
let restore: () => void = () => {};
let restoreObserver: () => void = () => {};

beforeEach(() => {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    mocks.put.mockReset();
    mocks.put.mockImplementation(async (_path: string, options: { body: unknown }) => ({
        data: options.body,
        response: { ok: true },
    }));
    restoreObserver = stubResizeObserver();
    client = new QueryClient();
    seedPreferences(client);
    host = document.createElement("div");
    document.body.append(host);
    root = createRoot(host);
});

afterEach(async () => {
    await act(async () => root.unmount());
    host.remove();
    restore();
    restoreObserver();
});

async function open(narrow: boolean) {
    restore = stubMatchMedia(narrow);
    await act(async () =>
        root.render(
            <QueryClientProvider client={client}>
                <SettingsDialog />
            </QueryClientProvider>,
        ),
    );
    await act(async () => document.querySelector<HTMLButtonElement>("button[aria-label=設定]")!.click());
}

const section = (title: string) => document.querySelector<HTMLElement>(`section[aria-label=${title}]`)!;
const cached = () => client.getQueryData(preferencesQuery.queryKey)!;

it("shows both sets side by side and marks the one that the screen uses", async () => {
    await open(false);
    expect(section("広い画面")).not.toBeNull();
    expect(section("狭い画面")).not.toBeNull();
    expect(section("広い画面").hasAttribute("data-current")).toBe(true);
    expect(section("広い画面").textContent).toContain("この画面");
    expect(section("狭い画面").hasAttribute("data-current")).toBe(false);
    expect(section("狭い画面").textContent).toContain("6 列");
    expect(section("広い画面").textContent).toContain("96");
});

it("marks the narrow set on a narrow screen", async () => {
    await open(true);
    expect(section("狭い画面").hasAttribute("data-current")).toBe(true);
    expect(section("広い画面").hasAttribute("data-current")).toBe(false);
});

it("changes only the set that was edited", async () => {
    await open(false);
    const narrowFit = section("狭い画面").querySelector<HTMLButtonElement>("button[role=radio][value=fit]")!;
    await act(async () => narrowFit.click());
    expect(cached().narrow.thumbnailShape).toBe("fit");
    expect(cached().wide).toEqual(DEFAULT_PREFERENCES.wide);

    const wideSlider = section("広い画面").querySelector<HTMLElement>("[role=slider]")!;
    await act(async () => {
        wideSlider.focus();
        wideSlider.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true }));
    });
    expect(cached().wide.tileSize).toBe(DEFAULT_PREFERENCES.wide.tileSize + 1);
    expect(cached().narrow.tileSize).toBe(DEFAULT_PREFERENCES.narrow.tileSize);
    expect(cached().sort).toEqual(DEFAULT_PREFERENCES.sort);
});

it("writes the whole preferences once after the edits stop, whichever dialog control changed them", async () => {
    await open(false);
    await act(async () =>
        section("狭い画面").querySelector<HTMLButtonElement>("button[role=radio][value=fit]")!.click(),
    );
    await act(async () =>
        section("広い画面").querySelector<HTMLButtonElement>("button[role=radio][value=fit]")!.click(),
    );
    expect(mocks.put).not.toHaveBeenCalled();

    await vi.waitFor(
        async () => {
            await act(async () => {
                await Promise.resolve();
            });
            expect(mocks.put).toHaveBeenCalledTimes(1);
        },
        { timeout: 3000 },
    );
    expect(mocks.put).toHaveBeenCalledTimes(1);
    expect(mocks.put.mock.calls[0]?.[1]?.body).toEqual({
        ...DEFAULT_PREFERENCES,
        wide: { ...DEFAULT_PREFERENCES.wide, thumbnailShape: "fit" },
        narrow: { ...DEFAULT_PREFERENCES.narrow, thumbnailShape: "fit" },
    });
});

it("changes only a chosen view and hides its tile size settings", async () => {
    await open(false);
    await act(async () =>
        section("狭い画面").querySelector<HTMLButtonElement>('button[role="radio"][value="list"]')!.click(),
    );
    expect(cached().narrow.view).toBe("list");
    expect(cached().wide).toEqual(DEFAULT_PREFERENCES.wide);
    await vi.waitFor(async () => {
        await act(async () => {
            await Promise.resolve();
        });
        expect(section("狭い画面").querySelector('[role="slider"]')).toBeNull();
    });
    expect(section("広い画面").querySelector('[role="slider"]')).not.toBeNull();
});

const radio = (title: string, value: string) => {
    const element = section(title).querySelector<HTMLButtonElement>(`button[role=radio][value=${value}]`);
    if (!element) throw new Error(`${title} の ${value} の選択がない`);
    return element;
};

it("元の画像への差し替えの選択を、2 組のそれぞれに今の値を選んで出す", async () => {
    await open(false);
    expect(radio("広い画面", "auto").getAttribute("aria-checked")).toBe("true");
    expect(radio("広い画面", "onZoom").getAttribute("aria-checked")).toBe("false");
    expect(radio("狭い画面", "auto").getAttribute("aria-checked")).toBe("false");
    expect(radio("狭い画面", "onZoom").getAttribute("aria-checked")).toBe("true");
    expect(section("広い画面").textContent).toContain("元の画像への差し替え");
    expect(section("狭い画面").textContent).toContain("拡大したときだけ");
});

it("元の画像への差し替えを変えると、その組の originalImage だけが変わり、書き戻しに載る", async () => {
    await open(false);
    await act(async () => radio("狭い画面", "auto").click());
    expect(cached().narrow.originalImage).toBe("auto");
    expect(cached().wide).toEqual(DEFAULT_PREFERENCES.wide);

    await vi.waitFor(
        async () => {
            await act(async () => {
                await Promise.resolve();
            });
            expect(mocks.put).toHaveBeenCalledTimes(1);
        },
        { timeout: 3000 },
    );
    expect(mocks.put.mock.calls[0]?.[1]?.body).toEqual({
        ...DEFAULT_PREFERENCES,
        narrow: { ...DEFAULT_PREFERENCES.narrow, originalImage: "auto" },
    });
});

it("一覧の表示がリストの組にも、元の画像への差し替えの選択を出す", async () => {
    await open(false);
    await act(async () => radio("広い画面", "list").click());
    await vi.waitFor(async () => {
        await act(async () => {
            await Promise.resolve();
        });
        expect(section("広い画面").querySelector('[role="slider"]')).toBeNull();
    });
    expect(radio("広い画面", "auto")).not.toBeNull();
    expect(radio("広い画面", "onZoom")).not.toBeNull();
});
