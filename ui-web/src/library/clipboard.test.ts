import { expect, it, vi } from "vitest";
import { CROSS_LIBRARY_SCOPE } from "../location";
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
        sources: [
            {
                mode: "cut",
                libraryId: "a",
                entries: [
                    { path: "/file", folder: false },
                    { path: "/dir", folder: true },
                ],
            },
        ],
    });
    expect(isCut(old, "a", "/file")).toBe(true);
    expect(isCut(old, "b", "/file")).toBe(false);
    store.write("copy", "b", new Map([["/new", { folder: true }]]));
    expect(isCut(store.getSnapshot(), "b", "/new")).toBe(false);
    store.clear(old);
    expect(store.getSnapshot()?.sources[0]?.libraryId).toBe("b");
    store.clear();
    expect(store.getSnapshot()).toBeUndefined();
    expect(listener).toHaveBeenCalledTimes(3);
    unsubscribe();
    store.write("cut", "a", new Map());
    expect(listener).toHaveBeenCalledTimes(3);
});

it("タイムラインの選択は Library ごとの組に分けて保存する", () => {
    const store = new EntryClipboard();
    store.write(
        "cut",
        CROSS_LIBRARY_SCOPE,
        new Map([
            ["/a/x.jpg", { folder: false }],
            ["/b/y.jpg", { folder: false }],
            ["/a/sub/z.jpg", { folder: false }],
        ]),
    );
    const contents = store.getSnapshot();
    expect(contents?.sources.map((source) => [source.libraryId, source.entries.map((entry) => entry.path)])).toEqual([
        ["a", ["/x.jpg", "/sub/z.jpg"]],
        ["b", ["/y.jpg"]],
    ]);
    expect(isCut(contents, "b", "/y.jpg")).toBe(true);
    expect(isCut(contents, CROSS_LIBRARY_SCOPE, "/a/sub/z.jpg")).toBe(true);
    expect(isCut(contents, CROSS_LIBRARY_SCOPE, "/b/x.jpg")).toBe(false);

    // 1 つの組を送り終えても、ほかの組は残る。
    store.removeAccepted(contents!.sources[0]!, ["/x.jpg", "/sub/z.jpg"]);
    expect(store.getSnapshot()?.sources.map((source) => source.libraryId)).toEqual(["b"]);
    store.removeAccepted(contents!.sources[1]!, ["/y.jpg"]);
    expect(store.getSnapshot()).toBeUndefined();
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
    const sent = store.getSnapshot()!.sources[0]!;
    store.removeAccepted(sent, ["/file"]);
    expect(store.getSnapshot()?.sources[0]?.entries).toEqual([{ path: "/dir", folder: true }]);
    store.removeAccepted(sent, ["/dir"]);
    expect(store.getSnapshot()).toBeUndefined();
    store.write("cut", "src", new Map([["/new", { folder: false }]]));
    const newer = store.getSnapshot();
    store.removeAccepted(sent, ["/new"]);
    expect(store.getSnapshot()).toBe(newer);
    store.write("copy", "src", new Map([["/new", { folder: false }]]));
    const copied = store.getSnapshot()!;
    store.removeAccepted(copied.sources[0]!, ["/new"]);
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
    const first = store.getSnapshot()!.sources[0]!;
    store.removeAccepted(first, ["/file"]);
    const retry = store.getSnapshot()!.sources[0]!;
    expect(retry).not.toBe(first);
    store.removeAccepted(retry, ["/dir"]);
    expect(store.getSnapshot()).toBeUndefined();
});
