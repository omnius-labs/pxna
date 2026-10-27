import { t } from "i18next";
import type { Entry } from "./entries";

/// 選択時のサイズを残し、一覧のページを捨てても合計を数えられるようにする。
export interface Selection {
    items: ReadonlyMap<string, SelectionValue>;
    anchor: number | undefined;
}

export const emptySelection: Selection = { items: new Map(), anchor: undefined };
export interface SelectionValue {
    size: number | undefined;
    folder: boolean;
    readonly: boolean;
    /// 選んだ Entry。一覧のページを探さずに、情報パネルが名前や更新日時を出せるようにする。
    entry: Entry;
}
export interface SelectionItem extends SelectionValue {
    path: string;
}

/// Entry と、その Library が読み取り専用かから、選択の項目を作る。
export function selectionItem(entry: Entry, libraryReadonly: boolean): SelectionItem {
    return {
        path: entry.path,
        size: entry.metadata?.size,
        folder: entry.kind === "folder",
        readonly: libraryReadonly || entry.metadata?.readonly !== false,
        entry,
    };
}

/// 選択の項目から、パスを除いた、選択が持つ値を取り出す。
export function selectionValue(item: SelectionItem): SelectionValue {
    return { size: item.size, folder: item.folder, readonly: item.readonly, entry: item.entry };
}
export type ItemAt = (index: number) => SelectionItem | undefined;
export interface ClickModifiers {
    toggle: boolean;
    range: boolean;
}

/// 範囲のすべての位置を読み終えてから呼ぶ。未読が残っていれば部分的な選択を返さない。
export function clickSelection(
    selection: Selection,
    index: number,
    modifiers: ClickModifiers,
    itemAt: ItemAt,
): Selection {
    const item = itemAt(index);
    if (!item) return selection;
    if (modifiers.range) {
        const anchor = selection.anchor ?? index;
        const items = new Map(modifiers.toggle ? selection.items : []);
        for (let i = Math.min(anchor, index); i <= Math.max(anchor, index); i += 1) {
            const entry = itemAt(i);
            if (!entry) return selection;
            items.set(entry.path, selectionValue(entry));
        }
        return { items, anchor };
    }
    if (modifiers.toggle) {
        const items = new Map(selection.items);
        if (items.has(item.path)) items.delete(item.path);
        else items.set(item.path, selectionValue(item));
        return { items, anchor: index };
    }
    return {
        items: new Map([[item.path, selectionValue(item)]]),
        anchor: index,
    };
}

/// 起点がまだなければ、動かす前のフォーカスの位置を起点にする。
export function extendSelection(selection: Selection, from: number, to: number, itemAt: ItemAt): Selection {
    return clickSelection(
        { ...selection, anchor: selection.anchor ?? from },
        to,
        { toggle: false, range: true },
        itemAt,
    );
}

export function selectAll(count: number, itemAt: ItemAt): Selection {
    const items = new Map<string, SelectionValue>();
    for (let index = 0; index < count; index += 1) {
        const item = itemAt(index);
        if (!item) throw new Error(t("errors.selectionUnread"));
        items.set(item.path, selectionValue(item));
    }
    return { items, anchor: count > 0 ? 0 : undefined };
}

/// 一部が未取得なら、既知分だけを完全な合計として表示しない。
export function selectionBytes(selection: Selection): number | undefined {
    let total = 0;
    for (const item of selection.items.values()) {
        if (item.size === undefined) return undefined;
        total += item.size;
    }
    return total;
}
