import { useNavigate } from "@tanstack/react-router";
import { ClipboardPaste, Download, FolderPlus, FolderSearch, Pencil, Trash2 } from "lucide-react";
import { type ReactNode, useRef } from "react";
import { actionAvailability, useEntryActions } from "../library/actionContext";
import { beginEntryDrag, endEntryDrag, useEntryDrop } from "../library/dnd";
import type { Entry } from "../library/entries";
import { selectionItem, selectionValue } from "../library/selection";
import { useTransfer } from "../library/transfer";
import { searchHref } from "../location";
import { ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuTrigger } from "./ui/context-menu";

export function EntryMenu({
    children,
    entry,
    allowDrop = true,
}: {
    children: ReactNode;
    entry?: Entry;
    allowDrop?: boolean;
}) {
    const transfer = useTransfer();
    const navigate = useNavigate();
    const actions = useEntryActions();
    const dropAllowed = allowDrop && !actions?.crossLibrary;
    const drop = useEntryDrop(
        entry?.kind === "folder" && dropAllowed && actions
            ? { libraryId: actions.libraryId, path: entry.path }
            : undefined,
        !actions ||
            actions.narrow ||
            actions.libraryReadonly ||
            (entry ? entry.metadata?.readonly !== false || !!entry.file?.archive : !actions.canCreate),
        undefined,
        actions && !actions.crossLibrary && (!entry || (entry.kind === "folder" && allowDrop) || !!entry.file?.archive)
            ? { libraryId: actions.libraryId, path: entry?.path ?? actions.folderPath ?? "/" }
            : undefined,
    );
    const preserveFocus = useRef(false);
    if (!actions || actions.narrow) return children;
    const available = actionAvailability(actions);
    const canCut =
        !actions.libraryReadonly &&
        actions.selection.items.size > 0 &&
        [...actions.selection.items.values()].every((item) => !item.readonly);
    const downloadName = [...actions.selection.items.keys()][0]?.split("/").at(-1);
    return (
        <ContextMenu>
            <ContextMenuTrigger asChild>
                {/* biome-ignore lint/a11y/noStaticElementInteractions: HTML5 ドラッグ＆ドロップのハンドラで、キーボード操作はメニュー側で担う */}
                <div
                    {...drop}
                    className="h-full data-[drop-state=allowed]:rounded data-[drop-state=allowed]:ring-2 data-[drop-state=allowed]:ring-ring data-[drop-state=blocked]:cursor-not-allowed data-[drop-state=blocked]:ring-2 data-[drop-state=blocked]:ring-destructive"
                    draggable={!!entry && !actions.editing && !actions.crossLibrary}
                    onDragStart={(event) => {
                        event.stopPropagation();
                        if (!entry || actions.editing) {
                            event.preventDefault();
                            return;
                        }
                        const items = actions.selection.items.has(entry.path)
                            ? actions.selection.items
                            : new Map([[entry.path, selectionValue(selectionItem(entry, actions.libraryReadonly))]]);
                        if (!actions.selection.items.has(entry.path))
                            actions.setSelection({ items, anchor: undefined });
                        beginEntryDrag(event.dataTransfer, {
                            mode: "cut",
                            libraryId: actions.libraryId,
                            entries: [...items].map(([path, item]) => ({
                                path,
                                folder: item.folder,
                                readonly: actions.libraryReadonly || item.readonly,
                            })),
                        });
                    }}
                    onDragEnd={endEntryDrag}
                    onContextMenu={(event) => {
                        event.stopPropagation();
                        if (entry && !actions.selection.items.has(entry.path))
                            actions.setSelection({
                                items: new Map([
                                    [entry.path, selectionValue(selectionItem(entry, actions.libraryReadonly))],
                                ]),
                                anchor: undefined,
                            });
                    }}
                >
                    {children}
                </div>
            </ContextMenuTrigger>
            <ContextMenuContent
                onCloseAutoFocus={(event) => {
                    if (preserveFocus.current) {
                        event.preventDefault();
                        preserveFocus.current = false;
                    }
                }}
            >
                {!actions.crossLibrary && (
                    <ContextMenuItem
                        disabled={!available.create}
                        onSelect={() => {
                            preserveFocus.current = true;
                            requestAnimationFrame(() => actions.create());
                        }}
                    >
                        新しいフォルダ
                    </ContextMenuItem>
                )}
                <ContextMenuItem
                    disabled={!available.rename}
                    onSelect={() => {
                        preserveFocus.current = true;
                        requestAnimationFrame(() => actions.rename?.());
                    }}
                >
                    名前の変更
                </ContextMenuItem>
                <ContextMenuItem disabled={!available.delete} onSelect={() => actions.delete?.()}>
                    削除
                </ContextMenuItem>
                <ContextMenuItem disabled={!canCut} onSelect={() => actions.cut?.()}>
                    切り取り
                </ContextMenuItem>
                <ContextMenuItem disabled={!actions.selection.items.size} onSelect={() => actions.copy?.()}>
                    コピー
                </ContextMenuItem>
                {!actions.crossLibrary && (
                    <ContextMenuItem
                        disabled={
                            !transfer?.canPaste ||
                            (entry
                                ? entry.kind !== "folder" ||
                                  actions.libraryReadonly ||
                                  entry.metadata?.readonly !== false
                                : !actions.canCreate)
                        }
                        onSelect={() => {
                            requestAnimationFrame(() => {
                                void transfer?.paste({
                                    libraryId: actions.libraryId,
                                    path: entry?.path ?? actions.folderPath ?? "/",
                                });
                            });
                        }}
                    >
                        貼り付け
                    </ContextMenuItem>
                )}
                {actions.crossLibrary && (
                    <ContextMenuItem disabled={actions.selection.items.size !== 1} onSelect={() => actions.reveal?.()}>
                        フォルダで表示
                    </ContextMenuItem>
                )}
                {!actions.crossLibrary && !entry && (
                    <ContextMenuItem
                        onSelect={() =>
                            void navigate({
                                href: searchHref({
                                    libraryIds: [actions.libraryId],
                                    path: actions.folderPath ?? "/",
                                    tags: [],
                                }),
                            })
                        }
                    >
                        このフォルダ内を検索
                    </ContextMenuItem>
                )}
                <ContextMenuItem disabled={!available.download} asChild={available.download}>
                    {available.download ? (
                        <a href={actions.downloadHref} download={downloadName}>
                            ダウンロード
                        </a>
                    ) : (
                        "ダウンロード"
                    )}
                </ContextMenuItem>
            </ContextMenuContent>
        </ContextMenu>
    );
}
export function EntryToolbar() {
    const transfer = useTransfer();
    const actions = useEntryActions();
    if (!actions || actions.narrow) return null;
    const available = actionAvailability(actions);
    const downloadName = [...actions.selection.items.keys()][0]?.split("/").at(-1);
    return (
        <div className="flex items-center gap-1">
            {!actions.crossLibrary && (
                <button
                    type="button"
                    aria-label="新しいフォルダ"
                    title="新しいフォルダ（Alt+Shift+N）"
                    disabled={!available.create}
                    onClick={actions.create}
                    className="rounded p-1.5 disabled:opacity-40"
                >
                    <FolderPlus className="size-4" />
                </button>
            )}
            <button
                type="button"
                aria-label="名前の変更"
                title="名前の変更（F2）"
                disabled={!available.rename}
                onClick={actions.rename}
                className="rounded p-1.5 disabled:opacity-40"
            >
                <Pencil className="size-4" />
            </button>
            <button
                type="button"
                aria-label="削除"
                title="削除（Delete）"
                disabled={!available.delete}
                onClick={actions.delete}
                className="rounded p-1.5 disabled:opacity-40"
            >
                <Trash2 className="size-4" />
            </button>
            {!actions.crossLibrary && (
                <button
                    type="button"
                    aria-label="貼り付け"
                    title="貼り付け（Ctrl/⌘+V）"
                    disabled={!transfer?.canPaste || !actions.canCreate}
                    onClick={() => {
                        void transfer?.paste({ libraryId: actions.libraryId, path: actions.folderPath ?? "/" });
                    }}
                    className="rounded p-1.5 disabled:opacity-40"
                >
                    <ClipboardPaste className="size-4" />
                </button>
            )}
            {actions.crossLibrary && (
                <button
                    type="button"
                    aria-label="フォルダで表示"
                    title="フォルダで表示"
                    disabled={actions.selection.items.size !== 1}
                    onClick={actions.reveal}
                    className="rounded p-1.5 disabled:opacity-40"
                >
                    <FolderSearch className="size-4" />
                </button>
            )}
            {available.download ? (
                <a
                    aria-label="ダウンロード"
                    title="ダウンロード"
                    href={actions.downloadHref}
                    download={downloadName}
                    className="rounded p-1.5"
                >
                    <Download className="size-4" />
                </a>
            ) : (
                <button
                    type="button"
                    aria-label="ダウンロード"
                    title="ダウンロード"
                    disabled
                    aria-disabled="true"
                    className="rounded p-1.5 opacity-40"
                >
                    <Download className="size-4" />
                </button>
            )}
        </div>
    );
}
