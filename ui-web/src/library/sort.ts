import { t } from "i18next";
import type { SortPreference } from "./preferences";

/// 並べ替えのキーの表示名。表示の順に並べる。今の言語で引くので、描画のたびに読む。
export const SORT_LABELS: Record<SortPreference["key"], string> = {
    get name() {
        return t("sort.name");
    },
    get modified_at() {
        return t("sort.modified_at");
    },
    get size() {
        return t("sort.size");
    },
    get kind() {
        return t("sort.kind");
    },
};

export const SORT_KEYS = Object.keys(SORT_LABELS) as SortPreference["key"][];

export const ORDER_LABELS: Record<SortPreference["order"], string> = {
    get asc() {
        return t("sort.asc");
    },
    get desc() {
        return t("sort.desc");
    },
};

/// 別の見出しは昇順から始め、今の見出しをもう一度押したときだけ向きを変える。
export function sortFromHeader(current: SortPreference, key: SortPreference["key"]): SortPreference {
    return { key, order: current.key === key && current.order === "asc" ? "desc" : "asc" };
}
