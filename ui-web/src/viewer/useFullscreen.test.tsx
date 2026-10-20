// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import { useFullscreen } from "./useFullscreen";

let fullscreenElement: Element | null = null;
const exitFullscreen = vi.fn(async () => {
    fullscreenElement = null;
    document.dispatchEvent(new Event("fullscreenchange"));
});
const requestFullscreen = vi.fn(async function (this: HTMLElement) {
    fullscreenElement = this;
    document.dispatchEvent(new Event("fullscreenchange"));
});

function stubFullscreen(enabled: boolean) {
    Object.defineProperty(document, "fullscreenEnabled", { configurable: true, value: enabled });
    Object.defineProperty(document, "fullscreenElement", { configurable: true, get: () => fullscreenElement });
    Object.defineProperty(document, "exitFullscreen", { configurable: true, value: exitFullscreen });
    Object.defineProperty(HTMLElement.prototype, "requestFullscreen", { configurable: true, value: requestFullscreen });
}

beforeEach(() => {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    fullscreenElement = null;
    exitFullscreen.mockClear();
    requestFullscreen.mockClear();
    stubFullscreen(true);
});
afterEach(() => {
    for (const key of ["fullscreenEnabled", "fullscreenElement", "exitFullscreen"])
        Reflect.deleteProperty(document, key);
    Reflect.deleteProperty(HTMLElement.prototype, "requestFullscreen");
});

type Controls = ReturnType<typeof useFullscreen>;
function mount() {
    const host = document.createElement("div");
    document.body.append(host);
    const target = document.createElement("div");
    document.body.append(target);
    const root = createRoot(host);
    let controls: Controls | undefined;
    const Probe = ({ element }: { element: HTMLElement | null }) => {
        controls = useFullscreen(element);
        return null;
    };
    return {
        target,
        render: (element: HTMLElement | null = target) => act(async () => root.render(<Probe element={element} />)),
        current: (): Controls => {
            if (!controls) throw new Error("まだ描いていない");
            return controls;
        },
        unmount: async () => {
            await act(async () => root.unmount());
            host.remove();
            target.remove();
        },
    };
}

it("対象の要素を全画面にし、fullscreenchange で状態を合わせ、もう一度で戻す", async () => {
    const h = mount();
    await h.render();
    expect(h.current().supported).toBe(true);
    expect(h.current().active).toBe(false);
    await act(async () => h.current().toggle());
    expect(requestFullscreen).toHaveBeenCalledOnce();
    expect(requestFullscreen.mock.contexts[0]).toBe(h.target);
    expect(h.current().active).toBe(true);
    await act(async () => h.current().toggle());
    expect(exitFullscreen).toHaveBeenCalledOnce();
    expect(h.current().active).toBe(false);
    await h.unmount();
});

it("ブラウザの操作で全画面が終わったときも、状態を戻す", async () => {
    const h = mount();
    await h.render();
    await act(async () => h.current().enter());
    expect(h.current().active).toBe(true);
    await act(async () => {
        fullscreenElement = null;
        document.dispatchEvent(new Event("fullscreenchange"));
    });
    expect(h.current().active).toBe(false);
    expect(exitFullscreen).not.toHaveBeenCalled();
    await h.unmount();
});

it("別の要素が全画面になっても、対象が全画面とは扱わない", async () => {
    const h = mount();
    await h.render();
    await act(async () => {
        fullscreenElement = document.body;
        document.dispatchEvent(new Event("fullscreenchange"));
    });
    expect(h.current().active).toBe(false);
    await h.unmount();
    expect(exitFullscreen).not.toHaveBeenCalled();
});

it("全画面のまま使われなくなったら、全画面を終える", async () => {
    const h = mount();
    await h.render();
    await act(async () => h.current().enter());
    await h.unmount();
    expect(exitFullscreen).toHaveBeenCalledOnce();
});

it("fullscreenEnabled が真でないブラウザでは、対応していないとする", async () => {
    stubFullscreen(false);
    const h = mount();
    await h.render();
    expect(h.current().supported).toBe(false);
    await h.unmount();
});

it("要求を断られても、例外を出さず、全画面にしない", async () => {
    requestFullscreen.mockRejectedValueOnce(new TypeError("denied"));
    const h = mount();
    await h.render();
    await act(async () => h.current().enter());
    expect(h.current().active).toBe(false);
    await h.unmount();
});
