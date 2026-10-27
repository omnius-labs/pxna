// @vitest-environment jsdom
import { act, useEffect } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import type { DisplaySet } from "../library/preferences";
import { FakeImageSource } from "../testing/images";
import { filmstripItemSide } from "../viewer/filmstrip";
import { ImageWants, useImageWants } from "./wants";

interface Props {
    libraryId: string;
    tileSize: number;
    narrow: boolean;
    shape: DisplaySet["thumbnailShape"];
    rowSide?: number;
}

let host: HTMLDivElement;
let root: Root;
type Result = { wants: ImageWants; size: number };
let uninstall: () => void;

/// フックの結果を、描画の後に `onResult` へ渡す。
function Probe({ onResult, ...props }: Props & { onResult: (result: Result) => void }) {
    const result = useImageWants(props.libraryId, props.tileSize, props.narrow, props.shape, props.rowSide);
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
    uninstall = new FakeImageSource().install();
});

afterEach(async () => {
    await act(async () => root.unmount());
    host.remove();
    uninstall();
    vi.restoreAllMocks();
});

it("keeps the wants while nothing that decides the request changes, even if the window is resized", async () => {
    const first = await render(base);
    Object.assign(window, { innerWidth: 900 });
    window.dispatchEvent(new Event("resize"));
    const again = await render({ ...base });
    expect(again.wants).toBe(first.wants);
    expect(again.size).toBe(first.size);
});

it("requests the minimum side of the step times the pixel ratio on a wide screen", async () => {
    expect((await render({ ...base, tileSize: 3 })).size).toBe(176 * 2);
    expect((await render({ ...base, tileSize: 5 })).size).toBe(320 * 2);
});

it("makes new wants and disposes the old ones when the tile size, the shape, the width class or the library changes", async () => {
    const dispose = vi.spyOn(ImageWants.prototype, "dispose");
    for (const change of [{ tileSize: 4 }, { shape: "fit" as const }, { narrow: true }, { libraryId: "videos" }]) {
        const before = await render(base);
        const disposedBefore = dispose.mock.calls.length;
        const after = await render({ ...base, ...change });
        expect(after.wants, JSON.stringify(change)).not.toBe(before.wants);
        expect(dispose.mock.calls.length, JSON.stringify(change)).toBeGreaterThan(disposedBefore);
    }
});

it("requests the side times the pixel ratio for a filmstrip level and makes new wants when the level changes", async () => {
    const dispose = vi.spyOn(ImageWants.prototype, "dispose");
    const filmstrip = { ...base, tileSize: 1, shape: "crop" as const };
    const small = await render({ ...filmstrip, rowSide: filmstripItemSide(1) });
    expect(small.size).toBe(40 * 2);
    const disposed = dispose.mock.calls.length;
    const large = await render({ ...filmstrip, rowSide: filmstripItemSide(5) });
    expect(large.size).toBe(96 * 2);
    expect(large.wants).not.toBe(small.wants);
    expect(dispose.mock.calls.length).toBeGreaterThan(disposed);
    expect((await render({ ...filmstrip, rowSide: filmstripItemSide(5) })).wants).toBe(large.wants);
});
