// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";

import type { Entry } from "./entries";
import { type Listing, PAGE_SIZE } from "./listing";
import { useRevealEntry } from "./useRevealEntry";

const entry = (name: string): Entry => ({
    path: `/dir/${name}`,
    name,
    kind: "file",
    metadata: { size: 1, readonly: false },
    file: { archive: false },
});
const names = ["a.png", "b.png", "c.png"];

function fakeListing(): Listing {
    return {
        truncated: false,
        totalCount: names.length,
        snapshot: "one",
        version: 0,
        entryAt: (index) => (names[index] === undefined ? undefined : entry(names[index])),
        pageStatus: () => "loaded",
        pageError: () => null,
        loadPages: async () => {},
        retry: () => {},
    };
}

async function run(name: string | undefined, columns = 2) {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    const listing = fakeListing();
    const scrollToIndex = vi.fn();
    const onSelect = vi.fn();
    const onDone = vi.fn();
    function Harness() {
        useRevealEntry({
            listing,
            columns,
            virtualizer: { scrollToIndex },
            folderPath: "/dir",
            name,
            onSelect,
            onDone,
        });
        return null;
    }
    const root = createRoot(document.createElement("div"));
    await act(async () => root.render(<Harness />));
    await act(async () => {});
    await act(async () => root.unmount());
    return { scrollToIndex, onSelect, onDone };
}

it("selects the named entry, scrolls to its row and finishes", async () => {
    const result = await run("c.png");
    expect(result.scrollToIndex).toHaveBeenCalledWith(1);
    expect(result.onSelect).toHaveBeenCalledWith(entry("c.png"), 2);
    expect(result.onDone).toHaveBeenCalledTimes(1);
});

it("finishes without selecting when the name is not found", async () => {
    const result = await run("missing.png");
    expect(result.onSelect).not.toHaveBeenCalled();
    expect(result.onDone).toHaveBeenCalledTimes(1);
});

it("does nothing without a name", async () => {
    const result = await run(undefined);
    expect(result.onDone).not.toHaveBeenCalled();
    expect(PAGE_SIZE).toBeGreaterThan(0);
});
