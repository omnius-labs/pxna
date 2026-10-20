// @vitest-environment jsdom
import { act, StrictMode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { Modal } from "./Modal";

let root: Root;
let host: HTMLDivElement;
let opener: HTMLButtonElement;

beforeEach(() => {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    vi.spyOn(HTMLElement.prototype, "getClientRects").mockReturnValue([{}] as unknown as DOMRectList);
    Object.defineProperty(HTMLDialogElement.prototype, "showModal", {
        configurable: true,
        value: vi.fn(function (this: HTMLDialogElement) {
            this.open = true;
        }),
    });
    Object.defineProperty(HTMLDialogElement.prototype, "close", {
        configurable: true,
        value: vi.fn(function (this: HTMLDialogElement) {
            this.open = false;
        }),
    });
    opener = document.createElement("button");
    host = document.createElement("div");
    document.body.append(opener, host);
    opener.focus();
    root = createRoot(host);
});

afterEach(async () => {
    await act(async () => root.unmount());
    host.remove();
    opener.remove();
    vi.restoreAllMocks();
});

async function open(onClose = vi.fn()) {
    await act(async () =>
        root.render(
            <StrictMode>
                <Modal label="photo" onClose={onClose}>
                    <button type="button">close</button>
                    <button type="button" disabled>
                        previous
                    </button>
                    <button type="button">next</button>
                </Modal>
            </StrictMode>,
        ),
    );
    return host.querySelector("dialog")!;
}

async function tab(dialog: HTMLDialogElement, shiftKey = false) {
    await act(async () =>
        dialog.dispatchEvent(new KeyboardEvent("keydown", { key: "Tab", shiftKey, bubbles: true, cancelable: true })),
    );
}

describe("Modal", () => {
    it("cycles through enabled actions in both directions", async () => {
        const dialog = await open();
        const [close, , next] = host.querySelectorAll("button");
        expect(dialog.open).toBe(true);
        expect(document.activeElement).toBe(dialog);
        await tab(dialog);
        expect(document.activeElement).toBe(close);
        await tab(dialog);
        expect(document.activeElement).toBe(next);
        await tab(dialog);
        expect(document.activeElement).toBe(close);
        await tab(dialog, true);
        expect(document.activeElement).toBe(next);
    });

    it("skips elements that opt out of the tab order with tabindex -1", async () => {
        await act(async () =>
            root.render(
                <Modal label="photo" onClose={vi.fn()}>
                    <button type="button">close</button>
                    <button type="button" tabIndex={-1}>
                        thumbnail
                    </button>
                    <button type="button">next</button>
                </Modal>,
            ),
        );
        const dialog = host.querySelector("dialog");
        if (!dialog) throw new Error("dialog がない");
        const [close, , next] = host.querySelectorAll("button");
        await tab(dialog);
        expect(document.activeElement).toBe(close);
        await tab(dialog);
        expect(document.activeElement).toBe(next);
        await tab(dialog);
        expect(document.activeElement).toBe(close);
    });

    it("leaves Tab inside a nested menu or alert dialog to its own focus trap", async () => {
        await act(async () =>
            root.render(
                <Modal label="photo" onClose={vi.fn()}>
                    <button type="button">close</button>
                    <div role="menu">
                        <button type="button" role="menuitem">
                            item
                        </button>
                    </div>
                </Modal>,
            ),
        );
        const item = host.querySelector("[role=menuitem]");
        if (!item) throw new Error("メニューの項目がない");
        const event = new KeyboardEvent("keydown", { key: "Tab", bubbles: true, cancelable: true });
        await act(async () => item.dispatchEvent(event));
        expect(event.defaultPrevented).toBe(false);
    });

    it("hands the outer element that wraps the children to the owner", async () => {
        const outerRef = vi.fn();
        await act(async () =>
            root.render(
                <Modal label="photo" onClose={vi.fn()} outerRef={outerRef}>
                    <button type="button">close</button>
                </Modal>,
            ),
        );
        const element = outerRef.mock.calls.find(([value]) => value !== null)?.[0];
        expect(element).toBeInstanceOf(HTMLDivElement);
        expect(element.parentElement).toBe(host.querySelector("dialog"));
        expect(element.contains(host.querySelector("button"))).toBe(true);
    });

    it("restores the opener when removed, including StrictMode cleanup", async () => {
        await open();
        await act(async () => root.render(null));
        expect(document.activeElement).toBe(opener);
    });

    it("asks the owner to close when the browser cancels the dialog", async () => {
        const onClose = vi.fn();
        const dialog = await open(onClose);
        const event = new Event("cancel", { cancelable: true });
        await act(async () => dialog.dispatchEvent(event));
        expect(onClose).toHaveBeenCalledOnce();
        expect(event.defaultPrevented).toBe(true);
    });
});
