// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import { DEFAULT_PREFERENCES, preferencesQuery } from "../library/preferences";
import { seedPreferences, stubMatchMedia, stubResizeObserver } from "../testing/preferences";
import { SettingsMenu } from "./SettingsMenu";

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
    client.clear();
    restore();
    restoreObserver();
});

async function open(narrow: boolean, timeline = false) {
    restore = stubMatchMedia(narrow);
    await act(async () =>
        root.render(
            <QueryClientProvider client={client}>
                <SettingsMenu timeline={timeline} />
            </QueryClientProvider>,
        ),
    );
    await act(async () => document.querySelector<HTMLButtonElement>('button[aria-label="設定"]')!.click());
}

const cached = () => client.getQueryData(preferencesQuery.queryKey)!;
const item = (group: string, label: string) => {
    const element = [...document.querySelectorAll<HTMLButtonElement>(`[aria-label="${group}"] button`)].find(
        (button) => button.textContent === label,
    );
    if (!element) throw new Error(`${group} の ${label} の選択がない`);
    return element;
};
const settle = async (check: () => void) =>
    vi.waitFor(async () => {
        await act(async () => {
            await Promise.resolve();
        });
        check();
    });

it.each([false, true])("changes only the active set and hides grid size controls (narrow=%s)", async (narrow) => {
    await open(narrow);
    // 一覧の表示、タイルサイズ、サムネイルの形、元の画像への差し替え、並べ替えを 1 つのポップオーバーに出す。
    expect(document.querySelector('[aria-label="一覧の表示"]')).not.toBeNull();
    expect(document.querySelector('[role="slider"]')).not.toBeNull();
    expect(document.querySelector('[aria-label="サムネイルの形"]')).not.toBeNull();
    expect(document.querySelector('[aria-label="元の画像への差し替え"]')).not.toBeNull();
    expect(document.querySelector('[aria-label="並べ替えのキー"]')).not.toBeNull();
    expect(document.querySelector('[aria-label="並べ替えの向き"]')).not.toBeNull();
    await act(async () => item("一覧の表示", "リスト").click());
    const preferences = cached();
    expect(narrow ? preferences.narrow.view : preferences.wide.view).toBe("list");
    expect(narrow ? preferences.wide : preferences.narrow).toEqual(
        narrow ? DEFAULT_PREFERENCES.wide : DEFAULT_PREFERENCES.narrow,
    );
    await settle(() => expect(document.querySelector('[role="slider"]')).toBeNull());
    // リストでも、ビューアは開けるので元の画像への差し替えは出す。
    expect(item("元の画像への差し替え", "自動")).not.toBeNull();
    await act(async () => {
        window.dispatchEvent(new KeyboardEvent("keydown", { key: "+" }));
        window.dispatchEvent(new WheelEvent("wheel", { ctrlKey: true, deltaY: -100 }));
    });
    expect(cached().wide.tileSize).toBe(DEFAULT_PREFERENCES.wide.tileSize);
});

it("元の画像への差し替えを今の組の値で選び、変えるとその組だけが変わる", async () => {
    await open(true);
    expect(item("元の画像への差し替え", "拡大したときだけ").getAttribute("aria-checked")).toBe("true");
    expect(item("元の画像への差し替え", "自動").getAttribute("aria-checked")).toBe("false");
    await act(async () => item("元の画像への差し替え", "自動").click());
    expect(cached().narrow.originalImage).toBe("auto");
    expect(cached().wide).toEqual(DEFAULT_PREFERENCES.wide);
});

it("並べ替えの向きを変えると、組によらない sort だけが変わる", async () => {
    await open(false);
    await act(async () => item("並べ替えの向き", "降順").click());
    expect(cached().sort).toEqual({ ...DEFAULT_PREFERENCES.sort, order: "desc" });
    expect(cached().wide).toEqual(DEFAULT_PREFERENCES.wide);
    expect(cached().narrow).toEqual(DEFAULT_PREFERENCES.narrow);
});

it("writes the whole preferences once after the edits stop", async () => {
    await open(false);
    await act(async () => item("サムネイルの形", "全体表示").click());
    await act(async () => item("元の画像への差し替え", "拡大したときだけ").click());
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
    expect(mocks.put.mock.calls[0]?.[1]?.body).toEqual({
        ...DEFAULT_PREFERENCES,
        wide: { ...DEFAULT_PREFERENCES.wide, thumbnailShape: "fit", originalImage: "onZoom" },
    });
});

it("タイムラインでは一覧の表示と並べ替えを出さず、タイルサイズはいつも出す", async () => {
    await open(false, true);
    expect(document.querySelector('[aria-label="一覧の表示"]')).toBeNull();
    expect(document.querySelector('[aria-label="並べ替えのキー"]')).toBeNull();
    expect(document.querySelector('[aria-label="並べ替えの向き"]')).toBeNull();
    expect(document.querySelector('[role="slider"]')).not.toBeNull();
    expect(document.querySelector('[aria-label="元の画像への差し替え"]')).not.toBeNull();
});
