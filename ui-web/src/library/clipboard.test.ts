import { expect, it, vi } from "vitest";
import { EntryClipboard, isCut } from "./clipboard";

it("切り取りとコピーは選択の Library と種類を保存し、置換とクリアを通知する", () => {
    const store = new EntryClipboard();
    const listener = vi.fn();
    const unsubscribe = store.subscribe(listener);
    store.write(
        "cut",
        "a",
        new Map([
            ["/file", { folder: false }],
            ["/dir", { folder: true }],
        ]),
    );
    const old = store.getSnapshot();
    expect(old).toEqual({
        mode: "cut",
        libraryId: "a",
        entries: [
            { path: "/file", folder: false },
            { path: "/dir", folder: true },
        ],
    });
    expect(isCut(old, "a", "/file")).toBe(true);
    expect(isCut(old, "b", "/file")).toBe(false);
    store.write("copy", "b", new Map([["/new", { folder: true }]]));
    expect(isCut(store.getSnapshot(), "b", "/new")).toBe(false);
    store.clear(old);
    expect(store.getSnapshot()?.libraryId).toBe("b");
    store.clear();
    expect(store.getSnapshot()).toBeUndefined();
    expect(listener).toHaveBeenCalledTimes(3);
    unsubscribe();
    store.write("cut", "a", new Map());
    expect(listener).toHaveBeenCalledTimes(3);
});

it("受理された項目を同じ世代から順に除き、新しい切り取りとコピーを保つ", () => {
    const store = new EntryClipboard();
    store.write(
        "cut",
        "src",
        new Map([
            ["/file", { folder: false }],
            ["/dir", { folder: true }],
        ]),
    );
    const sent = store.getSnapshot()!;
    store.removeAccepted(sent, ["/file"]);
    expect(store.getSnapshot()?.entries).toEqual([{ path: "/dir", folder: true }]);
    store.removeAccepted(sent, ["/dir"]);
    expect(store.getSnapshot()).toBeUndefined();
    store.write("cut", "src", new Map([["/new", { folder: false }]]));
    const newer = store.getSnapshot();
    store.removeAccepted(sent, ["/new"]);
    expect(store.getSnapshot()).toBe(newer);
    store.write("copy", "src", new Map([["/new", { folder: false }]]));
    const copied = store.getSnapshot()!;
    store.removeAccepted(copied, ["/new"]);
    expect(store.getSnapshot()).toBe(copied);
});
it("読み取り専用の選択はコピーだけを許可する", () => {
    const store = new EntryClipboard();
    const selected = new Map([["/archive/inner", { folder: false, readonly: true }]]);
    store.write("cut", "src", selected);
    expect(store.getSnapshot()).toBeUndefined();
    store.write("copy", "src", selected);
    expect(store.getSnapshot()?.mode).toBe("copy");
});

it("即時エラーで残った内容の次の貼り付けでも受理された項目を除く", () => {
    const store = new EntryClipboard();
    store.write(
        "cut",
        "src",
        new Map([
            ["/file", { folder: false }],
            ["/dir", { folder: true }],
        ]),
    );
    const first = store.getSnapshot()!;
    store.removeAccepted(first, ["/file"]);
    const retry = store.getSnapshot()!;
    expect(retry).not.toBe(first);
    store.removeAccepted(retry, ["/dir"]);
    expect(store.getSnapshot()).toBeUndefined();
});
