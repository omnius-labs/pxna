// @vitest-environment jsdom
import { describe, expect, it } from "vitest";

import { clampTileSize, directionFromKey, directionFromWheel, isTypingTarget, stepTileSize } from "./shortcuts";

const key = (value: string, modifiers: Partial<{ ctrlKey: boolean; metaKey: boolean; altKey: boolean }> = {}) => ({
    key: value,
    ctrlKey: false,
    metaKey: false,
    altKey: false,
    ...modifiers,
});

describe("stepTileSize", () => {
    it("moves one step and stops at both ends", () => {
        expect(stepTileSize(3, 1)).toBe(4);
        expect(stepTileSize(3, -1)).toBe(2);
        expect(stepTileSize(5, 1)).toBe(5);
        expect(stepTileSize(1, -1)).toBe(1);
    });

    it("clamps a value outside the range", () => {
        expect(clampTileSize(0)).toBe(1);
        expect(clampTileSize(9)).toBe(5);
    });
});

describe("directionFromKey", () => {
    it("makes the tiles larger with + and smaller with -", () => {
        expect(directionFromKey(key("+"))).toBe(1);
        expect(directionFromKey(key("="))).toBe(1);
        expect(directionFromKey(key("-"))).toBe(-1);
        expect(directionFromKey(key("_"))).toBe(-1);
    });

    it("ignores other keys and the keys that the browser uses to zoom", () => {
        expect(directionFromKey(key("a"))).toBeUndefined();
        expect(directionFromKey(key("+", { ctrlKey: true }))).toBeUndefined();
        expect(directionFromKey(key("-", { metaKey: true }))).toBeUndefined();
        expect(directionFromKey(key("+", { altKey: true }))).toBeUndefined();
    });
});

describe("directionFromWheel", () => {
    it("steps only while Ctrl or ⌘ is held, toward larger when the wheel is rolled forward", () => {
        expect(directionFromWheel({ deltaY: -100, ctrlKey: true, metaKey: false })).toBe(1);
        expect(directionFromWheel({ deltaY: 100, ctrlKey: false, metaKey: true })).toBe(-1);
        expect(directionFromWheel({ deltaY: 100, ctrlKey: false, metaKey: false })).toBeUndefined();
        expect(directionFromWheel({ deltaY: 0, ctrlKey: true, metaKey: false })).toBeUndefined();
    });
});

describe("isTypingTarget", () => {
    it("is true inside inputs, editable elements and dialogs", () => {
        document.body.innerHTML = `<input id="a"><div contenteditable="true"><span id="b"></span></div><dialog open><button id="c"></button></dialog><div role="dialog"><i id="d"></i></div><button id="e"></button>`;
        const at = (id: string) => document.getElementById(id);
        expect(isTypingTarget(at("a"))).toBe(true);
        expect(isTypingTarget(at("b"))).toBe(true);
        expect(isTypingTarget(at("c"))).toBe(true);
        expect(isTypingTarget(at("d"))).toBe(true);
        expect(isTypingTarget(at("e"))).toBe(false);
        expect(isTypingTarget(null)).toBe(false);
        expect(isTypingTarget(document)).toBe(false);
    });
});

it("alertdialog の中では入力中とみなす", () => {
    const dialog = document.createElement("div");
    dialog.setAttribute("role", "alertdialog");
    const button = document.createElement("button");
    dialog.append(button);
    expect(isTypingTarget(button)).toBe(true);
});

it("クリップボードのキーは Ctrl と Meta に対応し、Alt では始めない", async () => {
    const { clipboardCommand } = await import("./shortcuts");
    for (const metaKey of [false, true]) {
        expect(clipboardCommand({ key: "X", ctrlKey: !metaKey, metaKey, altKey: false })).toBe("cut");
        expect(clipboardCommand({ key: "c", ctrlKey: !metaKey, metaKey, altKey: false })).toBe("copy");
    }
    expect(clipboardCommand({ key: "x", ctrlKey: true, metaKey: false, altKey: true })).toBeUndefined();
});
