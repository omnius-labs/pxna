// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import { SETTLE_MS, useSettled } from "./useSettled";

beforeEach(() => {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    vi.useFakeTimers();
});
afterEach(() => {
    vi.useRealTimers();
});

function mount() {
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    let result: string | undefined;
    const Probe = ({ value }: { value: string }) => {
        result = useSettled(value);
        return null;
    };
    return {
        render: (value: string) => act(async () => root.render(<Probe value={value} />)),
        result: () => result,
        unmount: async () => {
            await act(async () => root.unmount());
            host.remove();
        },
    };
}
const elapse = (ms: number) => act(async () => vi.advanceTimersByTimeAsync(ms));

it("変わらないまま 200 ms たつまでは何も返さず、たつとその値を返す", async () => {
    const view = mount();
    await view.render("a");
    expect(view.result()).toBeUndefined();
    await elapse(SETTLE_MS - 1);
    expect(view.result()).toBeUndefined();
    await elapse(1);
    expect(view.result()).toBe("a");
    await view.unmount();
});

it("値を変えると、その描画から何も返さなくなり、また 200 ms たつと新しい値を返す", async () => {
    const view = mount();
    await view.render("a");
    await elapse(SETTLE_MS);
    expect(view.result()).toBe("a");
    await view.render("b");
    expect(view.result()).toBeUndefined();
    await elapse(SETTLE_MS);
    expect(view.result()).toBe("b");
    await view.unmount();
});

it("200 ms より短い間隔で変え続けている間は、何も返さない", async () => {
    const view = mount();
    for (const value of ["a", "b", "c", "d"]) {
        await view.render(value);
        await elapse(SETTLE_MS - 50);
        expect(view.result()).toBeUndefined();
    }
    await elapse(50);
    expect(view.result()).toBe("d");
    await view.unmount();
});

it("落ち着いた値から離れ、200 ms たつ前に戻ったときも、待ち直す", async () => {
    const view = mount();
    await view.render("a");
    await elapse(SETTLE_MS);
    expect(view.result()).toBe("a");
    await view.render("b");
    await elapse(SETTLE_MS - 50);
    await view.render("a");
    expect(view.result()).toBeUndefined();
    await elapse(SETTLE_MS - 1);
    expect(view.result()).toBeUndefined();
    await elapse(1);
    expect(view.result()).toBe("a");
    await view.unmount();
});
