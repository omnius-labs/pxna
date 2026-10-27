// @vitest-environment jsdom
import { t } from "i18next";
import { afterEach, expect, it } from "vitest";

import { ApiError } from "../api/client";
import { formatDay } from "../library/timelineLayout";
import { describeError } from "./errors";
import {
    currentLanguage,
    LANGUAGE_STORAGE_KEY,
    readLanguagePreference,
    resolveLanguage,
    setLanguagePreference,
} from "./index";
import { en } from "./locales/en";
import { ja } from "./locales/ja";

afterEach(() => setLanguagePreference("en"));

it.each([
    ["auto", ["ja-JP", "en-US"], "ja"],
    ["auto", ["en-US", "ja"], "en"],
    ["auto", ["fr-FR", "ja"], "ja"],
    ["auto", ["fr-FR", "de"], "en"],
    ["auto", [], "en"],
    ["auto", ["JA"], "ja"],
    ["en", ["ja"], "en"],
    ["ja", ["en"], "ja"],
] as const)("%s とブラウザの言語 %j から %s を選ぶ", (preference, browser, expected) => {
    expect(resolveLanguage(preference, browser)).toBe(expected);
});

it("選択を保存し、auto で消し、未知の値は auto として読む", () => {
    setLanguagePreference("ja");
    expect(window.localStorage.getItem(LANGUAGE_STORAGE_KEY)).toBe("ja");
    expect(readLanguagePreference()).toBe("ja");
    expect(currentLanguage()).toBe("ja");
    expect(document.documentElement.lang).toBe("ja");

    setLanguagePreference("auto");
    expect(window.localStorage.getItem(LANGUAGE_STORAGE_KEY)).toBeNull();
    expect(currentLanguage()).toBe("en");

    window.localStorage.setItem(LANGUAGE_STORAGE_KEY, "fr");
    expect(readLanguagePreference()).toBe("auto");
});

it("言語を切り替えると、文言と日付の形が変わる", () => {
    expect(t("common.reload")).toBe("Reload");
    expect(formatDay("2025-05-03")).toBe("May 3, 2025 (Sat)");
    setLanguagePreference("ja");
    expect(t("common.reload")).toBe("読み直す");
    expect(formatDay("2025-05-03")).toBe("2025年5月3日（土）");
    expect(t("deleteDialog.title", { count: 3 })).toBe("3 件を削除しますか？");
    setLanguagePreference("en");
    expect(t("deleteDialog.title", { count: 1 })).toBe("Delete 1 item?");
    expect(t("deleteDialog.title", { count: 3 })).toBe("Delete 3 items?");
});

/// 補間の名前（`{{name}}`）を集める。
const placeholders = (text: string) => [...text.matchAll(/\{\{(\w+)\}\}/g)].map((match) => match[1]).sort();

/// 入れ子の文言を、キーの道筋と値の組に平らにする。
function flatten(node: unknown, path = ""): [string, string][] {
    if (typeof node === "string") return [[path, node]];
    if (Array.isArray(node)) return [[path, node.join("|")]];
    return Object.entries(node as Record<string, unknown>).flatMap(([key, value]) =>
        flatten(value, path === "" ? key : `${path}.${key}`),
    );
}

it("日本語は英語と同じキーを持ち、補間の名前も同じである", () => {
    const english = new Map(flatten(en).filter(([key]) => !key.endsWith("_one")));
    const japanese = new Map(flatten(ja));
    expect([...japanese.keys()].sort()).toEqual([...english.keys()].sort());
    for (const [key, text] of japanese) {
        expect(placeholders(text), key).toEqual(placeholders(english.get(key)!));
    }
});

it("API の失敗は kind から今の言語の文言にし、kind がなければ message を使う", () => {
    expect(describeError(new ApiError(403, "Forbidden", "denied"), "x")).toBe("This is not allowed.");
    setLanguagePreference("ja");
    expect(describeError(new ApiError(403, "Forbidden", "denied"), "x")).toBe("この操作は許可されていません。");
    expect(describeError(new ApiError(500, undefined, "boom"), "x")).toBe("boom");
    expect(describeError("oops", "fallback")).toBe("fallback");
});
