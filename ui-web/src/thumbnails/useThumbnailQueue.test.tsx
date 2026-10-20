// @vitest-environment jsdom
import { act, useEffect } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import type { DisplaySet } from "../library/preferences";
import { ThumbnailQueue } from "./queue";
import { useThumbnailQueue } from "./useThumbnailQueue";

interface Props {
    libraryId: string;
    tileSize: number;
    narrow: boolean;
    shape: DisplaySet["thumbnailShape"];
}

let host: HTMLDivElement;
let root: Root;
type Result = { queue: ThumbnailQueue; size: number };

/// フックの結果を、描画の後に `onResult` へ渡す。
function Probe({ onResult, ...props }: Props & { onResult: (result: Result) => void }) {
    const result = useThumbnailQueue(props.libraryId, props.tileSize, props.narrow, props.shape);
    useEffect(() => {
        onResult(result);
    });
    return null;
}

const base: Props = { libraryId: "photos", tileSize: 3, narrow: false, shape: "crop" };

async function render(props: Props): Promise<Result> {
    let latest: Result | undefined;
    await act(async () => root.render(<Probe {...props} onResult={(result) => (latest = result)} />));
    if (latest === undefined) {
        throw new Error("the hook did not report a result");
    }
    return latest;
}

beforeEach(() => {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    host = document.createElement("div");
    document.body.append(host);
    root = createRoot(host);
    Object.assign(window, { devicePixelRatio: 2, innerWidth: 1280 });
});

afterEach(async () => {
    await act(async () => root.unmount());
    host.remove();
    vi.restoreAllMocks();
});

it("keeps the queue while nothing that decides the request changes, even if the window is resized", async () => {
    const first = await render(base);
    Object.assign(window, { innerWidth: 900 });
    window.dispatchEvent(new Event("resize"));
    const again = await render({ ...base });
    expect(again.queue).toBe(first.queue);
    expect(again.size).toBe(first.size);
});

it("requests the minimum side of the step times the pixel ratio on a wide screen", async () => {
    expect((await render({ ...base, tileSize: 3 })).size).toBe(176 * 2);
    expect((await render({ ...base, tileSize: 5 })).size).toBe(320 * 2);
});

it("makes a new queue and disposes the old one when the tile size, the shape, the width class or the library changes", async () => {
    const dispose = vi.spyOn(ThumbnailQueue.prototype, "dispose");
    for (const change of [{ tileSize: 4 }, { shape: "fit" as const }, { narrow: true }, { libraryId: "videos" }]) {
        const before = await render(base);
        const disposedBefore = dispose.mock.calls.length;
        const after = await render({ ...base, ...change });
        expect(after.queue, JSON.stringify(change)).not.toBe(before.queue);
        expect(dispose.mock.calls.length, JSON.stringify(change)).toBeGreaterThan(disposedBefore);
    }
});
