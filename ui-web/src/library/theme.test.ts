// @vitest-environment jsdom
import { expect, it } from "vitest";

import { followSystemTheme } from "./theme";

function fakeMedia(initial: boolean) {
    let listener: (() => void) | undefined;
    const media = {
        matches: initial,
        addEventListener: (_type: string, callback: () => void) => (listener = callback),
        removeEventListener: () => (listener = undefined),
    } as unknown as MediaQueryList;
    return {
        media,
        set: (matches: boolean) => {
            (media as { matches: boolean }).matches = matches;
            listener?.();
        },
        listening: () => listener !== undefined,
    };
}

it("adds the dark class while the system is dark, and follows changes until stopped", () => {
    const root = document.createElement("html");
    const fake = fakeMedia(true);
    const stop = followSystemTheme(root, fake.media);
    expect(root.classList.contains("dark")).toBe(true);

    fake.set(false);
    expect(root.classList.contains("dark")).toBe(false);
    fake.set(true);
    expect(root.classList.contains("dark")).toBe(true);

    stop();
    expect(fake.listening()).toBe(false);
});

it("leaves the light theme as it is", () => {
    const root = document.createElement("html");
    followSystemTheme(root, fakeMedia(false).media);
    expect(root.classList.contains("dark")).toBe(false);
});
