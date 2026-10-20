import { createContext, useContext } from "react";
import type { Selection } from "./selection";

export type EntryEditing = { type: "create"; parent: string } | { type: "rename"; path: string; folder: boolean };
export interface EntryActionState {
    libraryId: string;
    folderPath?: string;
    libraryReadonly: boolean;
    narrow: boolean;
    canCreate: boolean;
    selection: Selection;
    setSelection(selection: Selection): void;
    editing?: EntryEditing;
    cut?(): void;
    copy?(): void;
    create(): void;
    finish(): void;
    rename?: () => void;
    delete?: () => void;
    downloadHref?: string;
    /// タイムラインの操作か。今のフォルダがないので、新しいフォルダ、貼り付け、ドラッグでの移動は出さない。
    timeline?: boolean;
    /// 選んだ Entry の親フォルダを、フォルダ表示で開く。
    reveal?: () => void;
}
export const EntryActionContext = createContext<EntryActionState | undefined>(undefined);
export const useEntryActions = () => useContext(EntryActionContext);
export function actionAvailability(state: EntryActionState) {
    const writable =
        !state.libraryReadonly &&
        state.selection.items.size > 0 &&
        [...state.selection.items.values()].every((item) => !item.readonly);
    return {
        create: state.canCreate,
        rename: writable && state.selection.items.size === 1 && !!state.rename,
        delete: writable && !!state.delete,
        download:
            state.selection.items.size === 1 && ![...state.selection.items.values()][0]?.folder && !!state.downloadHref,
    };
}
