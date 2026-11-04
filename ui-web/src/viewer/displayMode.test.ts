// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import { readViewerDisplayMode, VIEWER_MODE_KEY, writeViewerDisplayMode } from "./displayMode";

afterEach(() => {
    vi.restoreAllMocks();
    window.localStorage.clear();
});

it("reads back the saved mode", () => {
    writeViewerDisplayMode("gallery");
    expect(window.localStorage.getItem(VIEWER_MODE_KEY)).toBe("gallery");
    expect(readViewerDisplayMode()).toBe("gallery");
});

it("opens in standard mode without a saved value or with an unknown one", () => {
    expect(readViewerDisplayMode()).toBe("standard");
    window.localStorage.setItem(VIEWER_MODE_KEY, "strip");
    expect(readViewerDisplayMode()).toBe("standard");
});

it("does not throw when the storage fails", () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
        throw new Error("denied");
    });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
        throw new Error("denied");
    });
    expect(readViewerDisplayMode()).toBe("standard");
    expect(() => writeViewerDisplayMode("focus")).not.toThrow();
});
