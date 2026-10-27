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
    await act(async () => document.querySelector<HTMLButtonElement>('button[aria-label="Settings"]')!.click());
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
    // 一覧の表示、タイルサイズ、サムネイルの形、並べ替えを 1 つのポップオーバーに出す。元の画像への差し替えは、ビューアの設定にある。
    expect(document.querySelector('[aria-label="List view"]')).not.toBeNull();
    expect(document.querySelector('[role="slider"]')).not.toBeNull();
    expect(document.querySelector('[aria-label="Thumbnail shape"]')).not.toBeNull();
    expect(document.querySelector('[aria-label="Switch to the original image"]')).toBeNull();
    expect(document.querySelector('[aria-label="Sort key"]')).not.toBeNull();
    expect(document.querySelector('[aria-label="Sort order"]')).not.toBeNull();
    await act(async () => item("List view", "List").click());
    const preferences = cached();
    expect(narrow ? preferences.narrow.view : preferences.wide.view).toBe("list");
    expect(narrow ? preferences.wide : preferences.narrow).toEqual(
        narrow ? DEFAULT_PREFERENCES.wide : DEFAULT_PREFERENCES.narrow,
    );
    await settle(() => expect(document.querySelector('[role="slider"]')).toBeNull());
    await act(async () => {
        window.dispatchEvent(new KeyboardEvent("keydown", { key: "+" }));
        window.dispatchEvent(new WheelEvent("wheel", { ctrlKey: true, deltaY: -100 }));
    });
    expect(cached().wide.tileSize).toBe(DEFAULT_PREFERENCES.wide.tileSize);
});

it.each([false, true])(
    "一覧の設定に、ビューアだけの設定（元の画像への差し替えとフィルムストリップ）を出さない (narrow=%s)",
    async (narrow) => {
        await open(narrow);
        expect(document.querySelector('[aria-label="Switch to the original image"]')).toBeNull();
        expect(document.querySelector('[aria-label="Filmstrip size"]')).toBeNull();
        expect(document.body.textContent).not.toContain("Switch to the original image");
    },
);

it("並べ替えの向きを変えると、組によらない sort だけが変わる", async () => {
    await open(false);
    await act(async () => item("Sort order", "Descending").click());
    expect(cached().sort).toEqual({ ...DEFAULT_PREFERENCES.sort, order: "desc" });
    expect(cached().wide).toEqual(DEFAULT_PREFERENCES.wide);
    expect(cached().narrow).toEqual(DEFAULT_PREFERENCES.narrow);
});

it("writes the whole preferences once after the edits stop", async () => {
    await open(false);
    await act(async () => item("Thumbnail shape", "Fit").click());
    await act(async () => item("List view", "List").click());
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
        wide: { ...DEFAULT_PREFERENCES.wide, thumbnailShape: "fit", view: "list" },
    });
});

it("タイムラインでは一覧の表示と並べ替えを出さず、タイルサイズはいつも出す", async () => {
    await open(false, true);
    expect(document.querySelector('[aria-label="List view"]')).toBeNull();
    expect(document.querySelector('[aria-label="Sort key"]')).toBeNull();
    expect(document.querySelector('[aria-label="Sort order"]')).toBeNull();
    expect(document.querySelector('[role="slider"]')).not.toBeNull();
});

it("言語を選ぶとブラウザに保存し、画面の言語を切り替える", async () => {
    Element.prototype.scrollIntoView = () => {};
    await open(false);
    expect(document.querySelector('button[aria-label="Language"]')?.textContent).toBe("Auto");
    await act(async () => document.querySelector<HTMLButtonElement>('button[aria-label="Language"]')!.click());
    const option = [...document.querySelectorAll<HTMLElement>('[role="option"]')].find(
        (o) => o.textContent === "日本語",
    );
    await act(async () => option!.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })));
    expect(window.localStorage.getItem("pxna.language")).toBe("ja");
    expect(document.documentElement.lang).toBe("ja");
});

it("今の表示と同じ言語に解決される選択でも、選んだ項目を表示し、保存する", async () => {
    Element.prototype.scrollIntoView = () => {};
    await open(false);
    const trigger = () => document.querySelector('button[aria-label="Language"]');
    const choose = async (name: string) => {
        await act(async () => (trigger() as HTMLButtonElement).click());
        const option = [...document.querySelectorAll<HTMLElement>('[role="option"]')].find(
            (o) => o.textContent === name,
        );
        await act(async () => option!.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })));
    };
    // ブラウザの言語が英語なので、English を選んでも画面の言語は変わらない。
    await choose("English");
    expect(trigger()?.textContent).toBe("English");
    expect(window.localStorage.getItem("pxna.language")).toBe("en");
    await choose("Auto");
    expect(trigger()?.textContent).toBe("Auto");
    expect(window.localStorage.getItem("pxna.language")).toBeNull();
});
