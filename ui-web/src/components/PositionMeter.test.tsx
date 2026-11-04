// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";
import { PositionMeter } from "./PositionMeter";

async function render(props: React.ComponentProps<typeof PositionMeter>) {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    const host = document.createElement("div");
    document.body.append(host);
    await act(async () => createRoot(host).render(<PositionMeter {...props} />));
    return host;
}

it("marks the open position and moves on a click", async () => {
    const onSeek = vi.fn();
    const host = await render({ first: 100, count: 4, index: 102, onSeek });
    const bars = [...host.querySelectorAll("button")];
    expect(bars).toHaveLength(4);
    expect(bars.map((bar) => bar.getAttribute("aria-current"))).toEqual([null, null, "true", null]);
    expect(bars.map((bar) => bar.tabIndex)).toEqual([-1, -1, 0, -1]);
    await act(async () => bars[0]?.click());
    expect(onSeek).toHaveBeenCalledWith(100);
});

it("limits the bars for a long range", async () => {
    const host = await render({ first: 0, count: 10_000, index: 9_999, onSeek: () => {} });
    const bars = [...host.querySelectorAll("button")];
    expect(bars).toHaveLength(48);
    expect(bars.at(-1)?.getAttribute("aria-current")).toBe("true");
});
