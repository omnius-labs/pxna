import type { QueryClient } from "@tanstack/react-query";

import { DEFAULT_PREFERENCES, type Preferences, preferencesQuery } from "../library/preferences";

/// 表示設定を取得済みの状態にする。認証済みの route が、表示設定を取りに行かないようにする。
export function seedPreferences(client: QueryClient, preferences: Preferences = DEFAULT_PREFERENCES): void {
    client.setQueryData(preferencesQuery.queryKey, preferences);
}

/// jsdom にない `matchMedia` を、画面の幅が狭いかどうかを固定したスタブにする。返す関数で元に戻す。
export function stubMatchMedia(narrow: boolean): () => void {
    const original = window.matchMedia;
    window.matchMedia = ((query: string) => ({
        matches: narrow,
        media: query,
        addEventListener: () => {},
        removeEventListener: () => {},
    })) as unknown as typeof window.matchMedia;
    return () => {
        window.matchMedia = original;
    };
}

/// jsdom にない `ResizeObserver` を、何もしないスタブにする。Radix の部品の一部（スライダーなど）が使う。返す関数で元に戻す。
export function stubResizeObserver(): () => void {
    const original = globalThis.ResizeObserver;
    globalThis.ResizeObserver = class {
        observe() {}
        unobserve() {}
        disconnect() {}
    } as unknown as typeof ResizeObserver;
    return () => {
        globalThis.ResizeObserver = original;
    };
}

/// 境界をまたぐ画面幅の変更を、購読中の全コンポーネントへ通知する。
export function stubChangingMatchMedia(initial = false) {
    const original = window.matchMedia;
    let narrow = initial;
    const listeners = new Set<() => void>();
    window.matchMedia = ((query: string) => ({
        get matches() {
            return narrow;
        },
        media: query,
        addEventListener: (_: string, listener: () => void) => listeners.add(listener),
        removeEventListener: (_: string, listener: () => void) => listeners.delete(listener),
    })) as unknown as typeof window.matchMedia;
    return {
        setNarrow(value: boolean) {
            narrow = value;
            for (const listener of listeners) listener();
        },
        restore() {
            window.matchMedia = original;
        },
    };
}
