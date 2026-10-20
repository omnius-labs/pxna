import type { SortPreference } from "./preferences";

/// 並べ替えのキーの表示名。表示の順に並べる。
export const SORT_LABELS: Record<SortPreference["key"], string> = {
    name: "名前",
    modified_at: "更新日時",
    size: "サイズ",
    kind: "種類",
};

export const SORT_KEYS = Object.keys(SORT_LABELS) as SortPreference["key"][];

export const ORDER_LABELS: Record<SortPreference["order"], string> = {
    asc: "昇順",
    desc: "降順",
};

/// 別の見出しは昇順から始め、今の見出しをもう一度押したときだけ向きを変える。
export function sortFromHeader(current: SortPreference, key: SortPreference["key"]): SortPreference {
    return { key, order: current.key === key && current.order === "asc" ? "desc" : "asc" };
}
