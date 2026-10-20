import { afterEach, expect, it, vi } from "vitest";
import { ChildProbeQueue, type ChildProbeTarget } from "./childProbes";

const queues: ChildProbeQueue[] = [];
afterEach(() => {
    for (const queue of queues.splice(0)) queue.dispose();
    vi.useRealTimers();
});
const target = (path: string, snapshot = "one"): ChildProbeTarget => ({
    key: `lib:${path}`,
    libraryId: "lib",
    path,
    snapshot,
});
const known = (hasChildren: boolean, maxAgeMs = 30000) => ({ hasChildren, maxAgeMs, sourceVersion: "version" });
async function flush() {
    for (let i = 0; i < 12; i++) await Promise.resolve();
}
function queue(request: ConstructorParameters<typeof ChildProbeQueue>[0]) {
    const result = new ChildProbeQueue(request);
    queues.push(result);
    return result;
}

it("可視対象だけを1要求ずつ送信する", async () => {
    const finishes: ((value: ReturnType<typeof known>) => void)[] = [];
    const request = vi.fn(
        (_target: ChildProbeTarget, _signal: AbortSignal) =>
            new Promise<ReturnType<typeof known>>((done) => finishes.push(done)),
    );
    const probes = queue(request);
    const all = Array.from({ length: 22432 }, (_, i) => target(`/folder-${i}`));
    probes.setWanted(all.slice(100, 119));
    expect(request).toHaveBeenCalledTimes(1);
    expect(request.mock.calls[0]![0]).toEqual(all[100]);
    for (let i = 0; i < 19; i++) {
        finishes[i]!(known(false));
        await flush();
    }
    expect(request).toHaveBeenCalledTimes(19);
    expect(probes.snapshot().size).toBe(19);
});
it("画面外と世代変更で取り消し、遅い応答を反映しない", async () => {
    const finishes: ((value: ReturnType<typeof known>) => void)[] = [];
    const request = vi.fn(
        (_target: ChildProbeTarget, _signal: AbortSignal) =>
            new Promise<ReturnType<typeof known>>((done) => finishes.push(done)),
    );
    const probes = queue(request);
    probes.setWanted([target("/a")]);
    const old = request.mock.calls[0]![1];
    probes.setWanted([target("/a", "two")]);
    expect(old.aborted).toBe(true);
    finishes[0]!(known(false));
    await flush();
    expect(probes.snapshot().has("lib:/a")).toBe(false);
    expect(request).toHaveBeenCalledTimes(2);
    probes.setWanted([]);
    expect(request.mock.calls[1]![1].aborted).toBe(true);
    finishes[1]!(known(true));
    await flush();
    expect(probes.snapshot().size).toBe(0);
});
it("未確認は500msから上限30秒で再試行する", async () => {
    vi.useFakeTimers();
    const request = vi.fn(async () => ({ hasChildren: null, maxAgeMs: 0 }));
    const probes = queue(request);
    probes.setWanted([target("/a")]);
    await flush();
    for (const delay of [500, 1000, 2000, 4000, 8000, 16000, 30000, 30000]) {
        const before = request.mock.calls.length;
        await vi.advanceTimersByTimeAsync(delay - 1);
        expect(request).toHaveBeenCalledTimes(before);
        await vi.advanceTimersByTimeAsync(1);
        expect(request).toHaveBeenCalledTimes(before + 1);
    }
    expect(probes.snapshot().size).toBe(0);
});
it("残存TTLを延長せず、期限後に再確認する", async () => {
    vi.useFakeTimers();
    const request = vi.fn(async () => known(false, 1000));
    const probes = queue(request);
    probes.setWanted([target("/a")]);
    await flush();
    expect(probes.snapshot().get("lib:/a")).toBe(false);
    await vi.advanceTimersByTimeAsync(900);
    probes.setWanted([target("/a")]);
    expect(request).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(100);
    expect(request).toHaveBeenCalledTimes(2);
});
it("他の探査が停止していても期限切れの表示を未確認へ戻す", async () => {
    vi.useFakeTimers();
    const request = vi.fn(async (value: ChildProbeTarget) =>
        value.path === "/a" ? known(false, 1000) : new Promise<ReturnType<typeof known>>(() => {}),
    );
    const probes = queue(request);
    probes.setWanted([target("/a"), target("/b"), target("/c")]);
    await flush();
    expect(request).toHaveBeenCalledTimes(2);
    expect(probes.snapshot().get("lib:/a")).toBe(false);
    await vi.advanceTimersByTimeAsync(1000);
    expect(probes.snapshot().has("lib:/a")).toBe(false);
    expect(request).toHaveBeenCalledTimes(2);
});
it("版不明と寿命0は空として保持しない", async () => {
    const request = vi.fn(async () => ({ hasChildren: false, maxAgeMs: 30000 }));
    const probes = queue(request);
    probes.setWanted([target("/a")]);
    await flush();
    expect(probes.snapshot().size).toBe(0);
});
