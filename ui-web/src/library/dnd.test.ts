// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import { beginEntryDrag, canDropEntry, createDragScroll, endEntryDrag, readEntryDrag } from "./dnd";

afterEach(() => {
    endEntryDrag();
    vi.useRealTimers();
});
it("独自 MIME の選択を保存し、跨ぎと readonly の宛先を拒否する", () => {
    const values = new Map<string, string>();
    const data = {
        setData: (key: string, value: string) => values.set(key, value),
        getData: (key: string) => values.get(key) ?? "",
    } as unknown as DataTransfer;
    const source = { mode: "cut" as const, libraryId: "a", entries: [{ path: "/file", folder: false }] };
    beginEntryDrag(data, source);
    expect(data.effectAllowed).toBe("copyMove");
    expect(readEntryDrag(data)).toEqual(source);
    expect(canDropEntry({ libraryId: "a", path: "/target" }, false)).toBe(true);
    expect(canDropEntry({ libraryId: "b", path: "/target" }, false)).toBe(false);
    expect(canDropEntry({ libraryId: "a", path: "/target" }, true)).toBe(false);
    expect(canDropEntry(undefined, false)).toBe(false);
    values.clear();
    expect(readEntryDrag(data)).toBeUndefined();
});
it("下端と上端にいる間スクロールし、中央とドラッグ終了で止まる", () => {
    vi.useFakeTimers();
    const element = document.createElement("div");
    element.getBoundingClientRect = () => ({ top: 0, bottom: 500 }) as DOMRect;
    const scroll = createDragScroll(element);
    scroll.move(490);
    vi.advanceTimersByTime(100);
    expect(element.scrollTop).toBe(48);
    scroll.move(250);
    vi.advanceTimersByTime(100);
    expect(element.scrollTop).toBe(48);
    scroll.move(10);
    vi.advanceTimersByTime(33);
    expect(element.scrollTop).toBe(32);
    scroll.stop();
    vi.advanceTimersByTime(100);
    expect(element.scrollTop).toBe(32);
});
it("drop の Alt で copy/move を切り替え、閉じたフォルダを1秒で開き、取消とreadonlyで送らない", async () => {
    const { act, createElement } = await import("react");
    const { createRoot } = await import("react-dom/client");
    const { TransferContext } = await import("./transfer");
    const { useEntryDrop } = await import("./dnd");
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    vi.useFakeTimers();
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    const send = vi.fn();
    const expand = vi.fn();
    const transfer = { busy: false, canPaste: false, paste: vi.fn(), transfer: send };
    function Target({ readonly }: { readonly: boolean }) {
        const drop = useEntryDrop({ libraryId: "a", path: "/target" }, readonly, expand);
        return createElement("div", { ...drop, "data-target": "" });
    }
    const values = new Map<string, string>();
    const data = {
        types: ["application/x-pxna-entries"],
        setData: (key: string, value: string) => values.set(key, value),
        getData: (key: string) => values.get(key) ?? "",
    } as unknown as DataTransfer;
    const drag = async (type: string, altKey = false) => {
        await act(async () => {
            const event = new Event(type, { bubbles: true, cancelable: true });
            Object.assign(event, { dataTransfer: data, altKey, relatedTarget: null });
            host.querySelector("[data-target]")!.dispatchEvent(event);
        });
    };
    try {
        await act(async () =>
            root.render(
                createElement(
                    TransferContext.Provider,
                    { value: transfer },
                    createElement(Target, { readonly: false }),
                ),
            ),
        );
        beginEntryDrag(data, { mode: "cut", libraryId: "a", entries: [{ path: "/file", folder: false }] });
        await drag("dragover");
        expect(host.querySelector("[data-target]")?.getAttribute("data-drop-state")).toBe("allowed");
        await act(async () => vi.advanceTimersByTime(999));
        expect(expand).not.toHaveBeenCalled();
        await act(async () => vi.advanceTimersByTime(1));
        expect(expand).toHaveBeenCalledOnce();
        await drag("drop", true);
        expect(send.mock.calls[0]![0].mode).toBe("copy");
        expect(send.mock.calls[0]![1]).toEqual({ libraryId: "a", path: "/target" });
        await drag("drop");
        expect(send.mock.calls[1]![0].mode).toBe("cut");
        await drag("dragover");
        await act(async () => window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" })));
        await act(async () => vi.advanceTimersByTime(1000));
        expect(expand).toHaveBeenCalledOnce();
        expect(send).toHaveBeenCalledTimes(2);
        await act(async () =>
            root.render(
                createElement(TransferContext.Provider, { value: transfer }, createElement(Target, { readonly: true })),
            ),
        );
        await drag("dragover");
        expect(host.querySelector("[data-target]")?.getAttribute("data-drop-state")).toBe("blocked");
        expect(data.dropEffect).toBe("none");
        await drag("drop");
        expect(send).toHaveBeenCalledTimes(2);
    } finally {
        await act(async () => root.unmount());
        host.remove();
    }
});
it("OS のファイルは copy の効果で受け入れ、readonly では送らない", async () => {
    const { act, createElement } = await import("react");
    const { createRoot } = await import("react-dom/client");
    const { TransferContext } = await import("./transfer");
    const { useEntryDrop } = await import("./dnd");
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    const upload = vi.fn();
    const transfer = { busy: false, canPaste: false, paste: vi.fn(), transfer: vi.fn(), upload };
    const destination = { libraryId: "a", path: "/target" };
    function Target({ readonly }: { readonly: boolean }) {
        return createElement("div", {
            ...useEntryDrop(undefined, readonly, undefined, destination),
            "data-target": "",
        });
    }
    const file = new File(["contents"], "image.png");
    const data = { types: ["Files"], files: [file], items: [], effectAllowed: "copy" } as unknown as DataTransfer;
    const drag = async (type: string) => {
        await act(async () => {
            const event = new Event(type, { bubbles: true, cancelable: true });
            Object.assign(event, { dataTransfer: data, relatedTarget: null });
            host.querySelector("[data-target]")!.dispatchEvent(event);
        });
    };
    try {
        await act(async () =>
            root.render(
                createElement(
                    TransferContext.Provider,
                    { value: transfer },
                    createElement(Target, { readonly: false }),
                ),
            ),
        );
        await drag("dragover");
        expect(data.dropEffect).toBe("copy");
        await drag("drop");
        expect(upload).toHaveBeenCalledExactlyOnceWith([{ path: "image.png", file }], destination);
        await act(async () =>
            root.render(
                createElement(TransferContext.Provider, { value: transfer }, createElement(Target, { readonly: true })),
            ),
        );
        await drag("dragover");
        expect(data.dropEffect).toBe("none");
        await drag("drop");
        expect(upload).toHaveBeenCalledOnce();
    } finally {
        await act(async () => root.unmount());
        host.remove();
    }
});

it("読み取り専用のドラッグはAltのcopyだけを許可する", async () => {
    const { act, createElement } = await import("react");
    const { createRoot } = await import("react-dom/client");
    const { TransferContext } = await import("./transfer");
    const { useEntryDrop } = await import("./dnd");
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    const send = vi.fn();
    const values = new Map<string, string>();
    const data = {
        types: ["application/x-pxna-entries"],
        setData: (key: string, value: string) => values.set(key, value),
        getData: (key: string) => values.get(key) ?? "",
    } as unknown as DataTransfer;
    function Target() {
        return createElement("div", { ...useEntryDrop({ libraryId: "a", path: "/dest" }, false), "data-target": "" });
    }
    const drag = async (type: string, altKey = false) => {
        await act(async () => {
            const event = new Event(type, { bubbles: true, cancelable: true });
            Object.assign(event, { dataTransfer: data, altKey, relatedTarget: null });
            host.querySelector("[data-target]")!.dispatchEvent(event);
        });
    };
    try {
        await act(async () =>
            root.render(
                createElement(
                    TransferContext.Provider,
                    { value: { busy: false, canPaste: false, paste: vi.fn(), transfer: send } },
                    createElement(Target),
                ),
            ),
        );
        const source = {
            mode: "cut" as const,
            libraryId: "a",
            entries: [{ path: "/archive/inner", folder: false, readonly: true }],
        };
        beginEntryDrag(data, source);
        expect(readEntryDrag(data)).toEqual(source);
        await drag("dragover");
        expect(data.dropEffect).toBe("none");
        await drag("drop");
        expect(send).not.toHaveBeenCalled();
        beginEntryDrag(data, source);
        await drag("dragover", true);
        expect(data.dropEffect).toBe("copy");
        await drag("drop", true);
        expect(send).toHaveBeenCalledExactlyOnceWith({ ...source, mode: "copy" }, { libraryId: "a", path: "/dest" });
    } finally {
        await act(async () => root.unmount());
        host.remove();
    }
});
