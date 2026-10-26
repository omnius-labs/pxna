// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import { LAST_FOLDER_KEY, lastFolderLocation, rememberFolder } from "./folderReturn";

beforeEach(() => window.localStorage.clear());
afterEach(() => vi.restoreAllMocks());

it("最後に開いたフォルダを保存し、読み出す。ビューアと選んで見せる Entry は覚えない", () => {
    rememberFolder({ libraryId: "photos", path: "/a/b", view: "b.jpg", reveal: "b.jpg" });
    expect(lastFolderLocation()).toEqual({ libraryId: "photos", path: "/a/b" });
    expect(window.localStorage.getItem(LAST_FOLDER_KEY)).toBe(JSON.stringify({ libraryId: "photos", path: "/a/b" }));
});

it("Library のルートも保存する", () => {
    rememberFolder({ libraryId: "photos", path: "/" });
    expect(lastFolderLocation()).toEqual({ libraryId: "photos", path: "/" });
});

it("読み出したパスを Library 相対パスに正規化する", () => {
    window.localStorage.setItem(LAST_FOLDER_KEY, JSON.stringify({ libraryId: "photos", path: "//a//b/" }));
    expect(lastFolderLocation()).toEqual({ libraryId: "photos", path: "/a/b" });
});

it.each([
    ["壊れた JSON", "{oops"],
    ["配列", "[]"],
    ["null", "null"],
    ["Library の id が空", JSON.stringify({ libraryId: "", path: "/" })],
    ["Library の id が文字列でない", JSON.stringify({ libraryId: 1, path: "/" })],
    ["パスが絶対でない", JSON.stringify({ libraryId: "photos", path: "a/b" })],
    ["パスが文字列でない", JSON.stringify({ libraryId: "photos", path: 1 })],
    ["パスが親を指す", JSON.stringify({ libraryId: "photos", path: "/a/../b" })],
    ["パスが今いるフォルダを指す", JSON.stringify({ libraryId: "photos", path: "/a/./b" })],
])("保存した値が%sなら、未保存として扱う", (_label, stored) => {
    window.localStorage.setItem(LAST_FOLDER_KEY, stored);
    expect(lastFolderLocation()).toBeUndefined();
});

it("localStorage が例外を出しても、保存せず、未保存として扱う", () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
        throw new Error("blocked");
    });
    expect(lastFolderLocation()).toBeUndefined();
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
        throw new Error("blocked");
    });
    expect(() => rememberFolder({ libraryId: "photos", path: "/" })).not.toThrow();
});
