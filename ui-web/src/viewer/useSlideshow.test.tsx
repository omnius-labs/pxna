// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import { SLIDESHOW_INTERVAL_MS, type SlideshowStep, useSlideshow } from "./useSlideshow";

type Controls = ReturnType<typeof useSlideshow>;

beforeEach(() => {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    vi.useFakeTimers();
});
afterEach(() => {
    vi.useRealTimers();
});

function mount(advance: () => Promise<SlideshowStep>) {
    const host = document.createElement("div");
    const root = createRoot(host);
    let controls: Controls | undefined;
    const Probe = ({ settled }: { settled: string | undefined }) => {
        controls = useSlideshow({ settled, advance });
        return null;
    };
    const render = (settled: string | undefined) => act(async () => root.render(<Probe settled={settled} />));
    const current = (): Controls => {
        if (!controls) throw new Error("まだ描いていない");
        return controls;
    };
    return { render, current, unmount: () => act(async () => root.unmount()) };
}
const elapse = (ms: number) => act(async () => vi.advanceTimersByTimeAsync(ms));

it("始めた後、画像が出てから 5 秒で次へ進み、画像が出る前は数えない", async () => {
    const advance = vi.fn(async (): Promise<SlideshowStep> => "moved");
    const h = mount(advance);
    await h.render(undefined);
    await act(async () => h.current().start());
    await elapse(SLIDESHOW_INTERVAL_MS * 3);
    expect(advance).not.toHaveBeenCalled();
    await h.render("a");
    await elapse(SLIDESHOW_INTERVAL_MS - 1);
    expect(advance).not.toHaveBeenCalled();
    await elapse(1);
    expect(advance).toHaveBeenCalledTimes(1);
    await h.unmount();
});

it("画像が変わるたびに、5 秒を数え直す", async () => {
    const advance = vi.fn(async (): Promise<SlideshowStep> => "moved");
    const h = mount(advance);
    await h.render("a");
    await act(async () => h.current().start());
    await elapse(3000);
    await h.render(undefined);
    await h.render("b");
    await elapse(4999);
    expect(advance).not.toHaveBeenCalled();
    await elapse(1);
    expect(advance).toHaveBeenCalledTimes(1);
    await h.unmount();
});

it("進む先がなければ止まり、その後は進まない", async () => {
    const advance = vi.fn(async (): Promise<SlideshowStep> => "end");
    const h = mount(advance);
    await h.render("a");
    await act(async () => h.current().start());
    expect(h.current().running).toBe(true);
    await elapse(SLIDESHOW_INTERVAL_MS);
    expect(advance).toHaveBeenCalledTimes(1);
    expect(h.current().running).toBe(false);
    await elapse(SLIDESHOW_INTERVAL_MS * 3);
    expect(advance).toHaveBeenCalledTimes(1);
    await h.unmount();
});

it("別の移動の最中に時間が来たら、5 秒を数え直して進む", async () => {
    const advance = vi.fn(async (): Promise<SlideshowStep> => (advance.mock.calls.length === 1 ? "busy" : "moved"));
    const h = mount(advance);
    await h.render("a");
    await act(async () => h.current().start());
    await elapse(SLIDESHOW_INTERVAL_MS);
    expect(advance).toHaveBeenCalledTimes(1);
    await elapse(SLIDESHOW_INTERVAL_MS);
    expect(advance).toHaveBeenCalledTimes(2);
    expect(h.current().running).toBe(true);
    await h.unmount();
});

it("止めた後と、閉じた後は、時間を進めても進まない", async () => {
    const advance = vi.fn(async (): Promise<SlideshowStep> => "moved");
    const h = mount(advance);
    await h.render("a");
    await act(async () => h.current().start());
    await elapse(3000);
    await act(async () => h.current().stop());
    await elapse(SLIDESHOW_INTERVAL_MS * 3);
    expect(advance).not.toHaveBeenCalled();

    await act(async () => h.current().start());
    await elapse(3000);
    await h.unmount();
    await elapse(SLIDESHOW_INTERVAL_MS * 3);
    expect(advance).not.toHaveBeenCalled();
});
