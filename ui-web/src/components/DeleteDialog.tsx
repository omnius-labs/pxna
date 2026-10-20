import { useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { toast } from "sonner";
import { useActivity } from "../library/activity";
import { DeleteEntriesError, deleteEntries } from "../library/entryActions";
import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
} from "./ui/alert-dialog";

export interface DeleteTarget {
    libraryId: string;
    items: ReadonlyMap<string, { folder: boolean }>;
}
export function DeleteDialog({
    target,
    onClose,
    onDeleted,
    container,
    toasters,
}: {
    target?: DeleteTarget;
    onClose(): void;
    onDeleted(): void;
    /// ダイアログを描く先。モーダルの中で開くときは、モーダルの外枠の要素を渡す。省略すると `body` の直下に描く。
    container?: HTMLElement | null;
    /// 結果のトーストを出す `Toaster` の `id`。成功と失敗で分けられる。省略すると、ルートの `Toaster` に出す。
    toasters?: { success?: string; failure?: string };
}) {
    const client = useQueryClient();
    const activity = useActivity();
    const [busy, setBusy] = useState(false);
    if (!target) return null;
    const confirm = async () => {
        if (busy) return;
        setBusy(true);
        try {
            const operations = await deleteEntries(client, target.libraryId, target.items);
            if (operations.length) activity?.show(operations);
            else
                toast.success(
                    "削除しました。",
                    ...(toasters?.success === undefined ? [] : [{ toasterId: toasters.success }]),
                );
            onDeleted();
            onClose();
        } catch (error) {
            if (error instanceof DeleteEntriesError) {
                if (error.operations.length) activity?.show(error.operations);
                if (error.acceptedCount > 0) onDeleted();
            }
            toast.error("削除できない項目があります。", {
                description: error instanceof Error ? error.message : undefined,
                toasterId: toasters?.failure,
            });
            onClose();
        } finally {
            setBusy(false);
        }
    };
    const folders = [...target.items.values()].some((item) => item.folder);
    return (
        <AlertDialog
            open
            onOpenChange={(open) => {
                if (!open && !busy) onClose();
            }}
        >
            <AlertDialogContent container={container}>
                <AlertDialogHeader>
                    <AlertDialogTitle>{target.items.size} 件を削除しますか？</AlertDialogTitle>
                    <AlertDialogDescription>
                        この操作は取り消せません。{folders && "フォルダは中身ごと削除されます。"}
                    </AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                    <AlertDialogCancel disabled={busy}>取り消す</AlertDialogCancel>
                    <AlertDialogAction
                        disabled={busy}
                        onClick={(event) => {
                            event.preventDefault();
                            void confirm();
                        }}
                    >
                        削除する
                    </AlertDialogAction>
                </AlertDialogFooter>
            </AlertDialogContent>
        </AlertDialog>
    );
}
