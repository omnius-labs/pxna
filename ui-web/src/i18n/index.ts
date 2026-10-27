import i18next from "i18next";
import { useSyncExternalStore } from "react";

import { en } from "./locales/en";
import { ja } from "./locales/ja";

export const LANGUAGES = ["en", "ja"] as const;
export type Language = (typeof LANGUAGES)[number];
/// 利用者の選択。`auto` はブラウザの言語に従う。
export type LanguagePreference = "auto" | Language;

export const DEFAULT_LANGUAGE: Language = "en";
export const LANGUAGE_STORAGE_KEY = "pxna.language";

/// 選択とブラウザの言語（優先順）から、使う言語を決める。対応する言語がなければ既定の言語にする。
export function resolveLanguage(preference: LanguagePreference, browserLanguages: readonly string[]): Language {
    if (preference !== "auto") return preference;
    for (const tag of browserLanguages) {
        const primary = tag.toLowerCase().split("-")[0];
        const found = LANGUAGES.find((language) => language === primary);
        if (found !== undefined) return found;
    }
    return DEFAULT_LANGUAGE;
}

/// 保存した選択を読む。読めない、または未知の値のときは `auto` にする。
export function readLanguagePreference(): LanguagePreference {
    try {
        if (typeof window === "undefined") return "auto";
        const value = window.localStorage.getItem(LANGUAGE_STORAGE_KEY);
        return value === "en" || value === "ja" ? value : "auto";
    } catch {
        return "auto";
    }
}

/// ブラウザの言語（優先順）。`window` のない環境（node のテストなど）では空にする。
function browserLanguages(): readonly string[] {
    if (typeof window === "undefined") return [];
    return window.navigator.languages.length > 0 ? window.navigator.languages : [window.navigator.language];
}

function apply(language: Language): void {
    if (typeof document !== "undefined") document.documentElement.lang = language;
}

/// 選択を保存し、表示の言語を切り替える。保存できなくても、このページの間は切り替える。
export function setLanguagePreference(preference: LanguagePreference): void {
    try {
        if (preference === "auto") window.localStorage.removeItem(LANGUAGE_STORAGE_KEY);
        else window.localStorage.setItem(LANGUAGE_STORAGE_KEY, preference);
    } catch {
        // 保存先が使えなくても、表示は切り替える。
    }
    void i18next.changeLanguage(resolveLanguage(preference, browserLanguages()));
}

/// 今の表示の言語。
export function currentLanguage(): Language {
    return i18next.language === "ja" ? "ja" : "en";
}

void i18next.init({
    lng: resolveLanguage(readLanguagePreference(), browserLanguages()),
    fallbackLng: DEFAULT_LANGUAGE,
    resources: { en: { translation: en }, ja: { translation: ja } },
    interpolation: { escapeValue: false },
    returnNull: false,
    initAsync: false,
});
apply(currentLanguage());
i18next.on("languageChanged", () => apply(currentLanguage()));

function subscribe(listener: () => void): () => void {
    i18next.on("languageChanged", listener);
    return () => i18next.off("languageChanged", listener);
}

/// 表示の言語を購読する。切り替わると再描画する。
/// 文字列は描画のたびに `t` で引くので、言語が変わったら画面全体を作り直す（`key` に使う）。
export function useLanguage(): Language {
    return useSyncExternalStore(subscribe, currentLanguage);
}
