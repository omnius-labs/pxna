// @vitest-environment jsdom
import { beforeEach, expect, it, vi } from "vitest";

import { clampWidth, INFO_PANEL_WIDTH, readWidth, TREE_WIDTH, TREE_WIDTH_KEY, writeWidth } from "./panelWidth";

beforeEach(() => window.localStorage.clear());

it("保存がなければ初期値、数でなければ初期値、範囲の外は範囲に収める", () => {
    expect(readWidth(TREE_WIDTH_KEY, TREE_WIDTH)).toBe(TREE_WIDTH.initial);
    window.localStorage.setItem(TREE_WIDTH_KEY, "abc");
    expect(readWidth(TREE_WIDTH_KEY, TREE_WIDTH)).toBe(TREE_WIDTH.initial);
    window.localStorage.setItem(TREE_WIDTH_KEY, "9999");
    expect(readWidth(TREE_WIDTH_KEY, TREE_WIDTH)).toBe(TREE_WIDTH.max);
    window.localStorage.setItem(TREE_WIDTH_KEY, "10");
    expect(readWidth(TREE_WIDTH_KEY, TREE_WIDTH)).toBe(TREE_WIDTH.min);
    window.localStorage.setItem(TREE_WIDTH_KEY, "300");
    expect(readWidth(TREE_WIDTH_KEY, TREE_WIDTH)).toBe(300);
});

it("幅を最小と最大の間に収めて整数にする", () => {
    expect(clampWidth(INFO_PANEL_WIDTH, 100)).toBe(INFO_PANEL_WIDTH.min);
    expect(clampWidth(INFO_PANEL_WIDTH, 5000)).toBe(INFO_PANEL_WIDTH.max);
    expect(clampWidth(INFO_PANEL_WIDTH, 300.6)).toBe(301);
});

it("保存できなくても例外を出さない", () => {
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
        throw new Error("quota");
    });
    expect(() => writeWidth(TREE_WIDTH_KEY, 200)).not.toThrow();
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
        throw new Error("denied");
    });
    expect(readWidth(TREE_WIDTH_KEY, TREE_WIDTH)).toBe(TREE_WIDTH.initial);
    vi.restoreAllMocks();
});
