import { useQueryClient } from "@tanstack/react-query";
import { useRef, useState } from "react";
import { toast } from "sonner";

import { ApiError } from "../api/client";
import { useEntryActions } from "../library/actionContext";
import { childPath, createFolder, renameEntry, validateName } from "../library/entryActions";
import { emptySelection } from "../library/selection";
import { Button } from "./ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "./ui/dialog";
import { Input } from "./ui/input";

/// 名前を入れる操作の対象。`parent` と `path` は Library 相対パスで、ダイアログに示す。
export type NameTarget =
    | { type: "create"; libraryId: string; parent: string }
    | { type: "rename"; libraryId: string; path: string; folder: boolean };

interface NameDialogProps {
    target?: NameTarget;
    onClose(): void;
    /// 作成か名前の変更が成功したときに、閉じる前に呼ぶ。
    onDone(): void;
}

/// フォルダの作成と名前の変更の入力を受けるモーダルダイアログ。操作の対象の Library 相対パスを示す。
export function NameDialog({ target, onClose, onDone }: NameDialogProps) {
    if (!target) return null;
    const key = target.type === "create" ? `create:${target.libraryId}:${target.parent}` : `rename:${target.path}`;
    return <NameForm key={key} target={target} onClose={onClose} onDone={onDone} />;
}

function basename(path: string): string {
    return path.slice(path.lastIndexOf("/") + 1);
}

function NameForm({ target, onClose, onDone }: NameDialogProps & { target: NameTarget }) {
    const client = useQueryClient();
    const input = useRef<HTMLInputElement>(null);
    const initialName = target.type === "rename" ? basename(target.path) : "";
    const folder = target.type === "create" || target.folder;
    const [name, setName] = useState(initialName);
    const [busy, setBusy] = useState(false);
    const invalid = validateName(name) !== undefined || (target.type === "rename" && name === initialName);

    const submit = async () => {
        if (busy || invalid) return;
        setBusy(true);
        try {
            if (target.type === "create") {
                await createFolder(client, target.libraryId, childPath(target.parent, name));
                toast.success("フォルダを作成しました。");
            } else {
                await renameEntry(client, target.libraryId, { path: target.path, folder: target.folder }, name);
                toast.success("名前を変更しました。");
            }
            onDone();
            onClose();
        } catch (error) {
            toast.error(
                error instanceof ApiError && error.status === 409
                    ? "同じ名前の項目があります。"
                    : error instanceof ApiError && error.status === 403
                      ? "この場所は変更できません。"
                      : "変更できませんでした。",
            );
            input.current?.focus();
        } finally {
            setBusy(false);
        }
    };

    const title = target.type === "create" ? "フォルダを作成" : "名前を変更";
    const place = target.type === "create" ? target.parent : target.path;
    return (
        <Dialog
            open
            onOpenChange={(open) => {
                if (!open && !busy) onClose();
            }}
        >
            <DialogContent
                showCloseButton={false}
                onOpenAutoFocus={(event) => {
                    // 名前の変更では、拡張子を除いた部分を選ぶ。
                    event.preventDefault();
                    input.current?.focus();
                    const dot = initialName.lastIndexOf(".");
                    input.current?.setSelectionRange(0, !folder && dot > 0 ? dot : initialName.length);
                }}
            >
                <form
                    className="grid gap-4"
                    onSubmit={(event) => {
                        event.preventDefault();
                        void submit();
                    }}
                >
                    <DialogHeader>
                        <DialogTitle>{title}</DialogTitle>
                        <DialogDescription>
                            {target.type === "create" ? "作成先" : "対象"}: <span className="break-all">{place}</span>
                        </DialogDescription>
                    </DialogHeader>
                    <Input
                        ref={input}
                        aria-label="名前"
                        value={name}
                        readOnly={busy}
                        onChange={(event) => setName(event.target.value)}
                    />
                    <DialogFooter>
                        <Button type="button" variant="outline" disabled={busy} onClick={onClose}>
                            取り消す
                        </Button>
                        <Button type="submit" disabled={busy || invalid}>
                            {target.type === "create" ? "作成する" : "変更する"}
                        </Button>
                    </DialogFooter>
                </form>
            </DialogContent>
        </Dialog>
    );
}

/// `EntryActionContext` の `editing` に従って、名前のダイアログを開く。フォルダ表示とタイムラインの画面が、文脈の中に置く。
export function EntryNameDialog() {
    const actions = useEntryActions();
    const editing = actions?.editing;
    if (!actions || actions.narrow || !editing) return null;
    const target: NameTarget =
        editing.type === "create"
            ? { type: "create", libraryId: actions.libraryId, parent: editing.parent }
            : { type: "rename", libraryId: actions.libraryId, path: editing.path, folder: editing.folder };
    return <NameDialog target={target} onClose={actions.finish} onDone={() => actions.setSelection(emptySelection)} />;
}
