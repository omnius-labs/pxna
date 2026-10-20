// @vitest-environment jsdom
import { QueryClient } from "@tanstack/react-query";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import {
    DEFAULT_PREFERENCES,
    type Preferences,
    PreferencesWriter,
    preferencesQuery,
    preferencesWriterFor,
    replaceSet,
    replaceSort,
    selectSet,
} from "./preferences";

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

function setup(put: (preferences: Preferences) => Promise<Preferences>) {
    const client = new QueryClient();
    client.setQueryData(preferencesQuery.queryKey, DEFAULT_PREFERENCES);
    const notify = vi.fn();
    const writer = new PreferencesWriter(client, { put, notify });
    const cached = () => client.getQueryData(preferencesQuery.queryKey);
    return { client, writer, notify, cached };
}

it("selects and replaces the set for the screen width without touching the other set or the sort", () => {
    expect(selectSet(DEFAULT_PREFERENCES, false)).toBe(DEFAULT_PREFERENCES.wide);
    expect(selectSet(DEFAULT_PREFERENCES, true)).toBe(DEFAULT_PREFERENCES.narrow);

    const wide = replaceSet(DEFAULT_PREFERENCES, false, { tileSize: 5 });
    expect(wide.wide.tileSize).toBe(5);
    expect(wide.narrow).toBe(DEFAULT_PREFERENCES.narrow);
    expect(wide.sort).toBe(DEFAULT_PREFERENCES.sort);

    const narrow = replaceSet(DEFAULT_PREFERENCES, true, { thumbnailShape: "fit" });
    expect(narrow.narrow.thumbnailShape).toBe("fit");
    expect(narrow.wide).toBe(DEFAULT_PREFERENCES.wide);

    const sorted = replaceSort(DEFAULT_PREFERENCES, { order: "desc" });
    expect(sorted.sort).toEqual({ key: "name", order: "desc" });
    expect(sorted.wide).toBe(DEFAULT_PREFERENCES.wide);
});

it("updates the cache at once and sends the whole value once after the operations stop for 500 ms", async () => {
    const put = vi.fn(async (preferences: Preferences) => preferences);
    const { writer, cached } = setup(put);

    writer.update((current) => replaceSet(current, false, { tileSize: 4 }));
    expect(cached()?.wide.tileSize).toBe(4);
    await vi.advanceTimersByTimeAsync(400);
    writer.update((current) => replaceSet(current, false, { tileSize: 5 }));
    await vi.advanceTimersByTimeAsync(400);
    writer.update((current) => replaceSort(current, { key: "size" }));
    expect(put).not.toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(500);
    expect(put).toHaveBeenCalledTimes(1);
    expect(put).toHaveBeenCalledWith({
        ...DEFAULT_PREFERENCES,
        wide: { ...DEFAULT_PREFERENCES.wide, tileSize: 5 },
        sort: { key: "size", order: "asc" },
    });
});

it("restores the last saved value and notifies when the write fails", async () => {
    const put = vi.fn(async () => {
        throw new Error("500");
    });
    const { writer, cached, notify } = setup(put);

    writer.update((current) => replaceSet(current, false, { tileSize: 5 }));
    expect(cached()?.wide.tileSize).toBe(5);
    await vi.advanceTimersByTimeAsync(500);

    expect(put).toHaveBeenCalledTimes(1);
    expect(cached()).toEqual(DEFAULT_PREFERENCES);
    expect(notify).toHaveBeenCalledTimes(1);
});

it("keeps the value of a successful write as the new baseline for a later failure", async () => {
    let fail = false;
    const put = vi.fn(async (preferences: Preferences) => {
        if (fail) {
            throw new Error("500");
        }
        return preferences;
    });
    const { writer, cached } = setup(put);

    writer.update((current) => replaceSet(current, false, { tileSize: 2 }));
    await vi.advanceTimersByTimeAsync(500);
    fail = true;
    writer.update((current) => replaceSet(current, false, { tileSize: 5 }));
    await vi.advanceTimersByTimeAsync(500);

    expect(cached()?.wide.tileSize).toBe(2);
});

it("sends a pending write at once on flush and drops it on dispose", async () => {
    const put = vi.fn(async (preferences: Preferences) => preferences);
    const { writer } = setup(put);

    writer.update((current) => replaceSet(current, true, { tileSize: 1 }));
    writer.flush();
    await vi.advanceTimersByTimeAsync(0);
    expect(put).toHaveBeenCalledTimes(1);

    writer.update((current) => replaceSet(current, true, { tileSize: 2 }));
    writer.dispose();
    await vi.advanceTimersByTimeAsync(1000);
    expect(put).toHaveBeenCalledTimes(1);
});

it("shares one writer per QueryClient so that every call site batches and recovers from the same baseline", () => {
    const first = new QueryClient();
    const second = new QueryClient();
    expect(preferencesWriterFor(first)).toBe(preferencesWriterFor(first));
    expect(preferencesWriterFor(second)).not.toBe(preferencesWriterFor(first));
});
