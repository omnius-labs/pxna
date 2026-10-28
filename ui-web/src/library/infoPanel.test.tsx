// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import {
    INFO_PANEL_OPEN_KEY,
    infoThumbnailSide,
    readInfoPanelOpen,
    useInfoPanel,
    writeInfoPanelOpen,
} from "./infoPanel";

beforeEach(() => {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    window.localStorage.clear();
});
afterEach(() => {
    vi.restoreAllMocks();
});

function mount() {
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    let current: ReturnType<typeof useInfoPanel> | undefined;
    const Probe = () => {
        current = useInfoPanel();
        return null;
    };
    return {
        render: () => act(async () => root.render(<Probe />)),
        state: () => {
            if (!current) throw new Error("まだ描いていない");
            return current;
        },
        unmount: async () => {
            await act(async () => root.unmount());
            host.remove();
        },
    };
}

it("保存がなければ開いた状態で、false を保存していれば閉じた状態で始まる", async () => {
    expect(readInfoPanelOpen()).toBe(true);
    window.localStorage.setItem(INFO_PANEL_OPEN_KEY, "false");
    expect(readInfoPanelOpen()).toBe(false);
    window.localStorage.setItem(INFO_PANEL_OPEN_KEY, "true");
    expect(readInfoPanelOpen()).toBe(true);
    window.localStorage.setItem(INFO_PANEL_OPEN_KEY, "壊れた値");
    expect(readInfoPanelOpen()).toBe(true);
});

it("切り替えると、状態が変わり、保存した値も変わる", async () => {
    const view = mount();
    await view.render();
    expect(view.state().open).toBe(true);
    await act(async () => view.state().toggle());
    expect(view.state().open).toBe(false);
    expect(window.localStorage.getItem(INFO_PANEL_OPEN_KEY)).toBe("false");
    await act(async () => view.state().toggle());
    expect(view.state().open).toBe(true);
    expect(window.localStorage.getItem(INFO_PANEL_OPEN_KEY)).toBe("true");
    await view.unmount();
});

it("保存した閉じた状態から、閉じたまま始まる", async () => {
    writeInfoPanelOpen(false);
    const view = mount();
    await view.render();
    expect(view.state().open).toBe(false);
    await view.unmount();
});

it("localStorage が例外を出しても、開いた状態で動き、切り替えもできる", async () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
        throw new DOMException("denied", "SecurityError");
    });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
        throw new DOMException("denied", "SecurityError");
    });
    const view = mount();
    await view.render();
    expect(view.state().open).toBe(true);
    await act(async () => view.state().toggle());
    expect(view.state().open).toBe(false);
    await view.unmount();
});

it("サムネイルの一辺は、枠の幅と高さの大きい方が収まる最小の段で、最大の段を超えない", () => {
    expect(infoThumbnailSide(0, 224)).toBe(256);
    expect(infoThumbnailSide(256, 96)).toBe(256);
    expect(infoThumbnailSide(257, 96)).toBe(384);
    expect(infoThumbnailSide(300, 600)).toBe(640);
    expect(infoThumbnailSide(1000, 1000)).toBe(640);
});
