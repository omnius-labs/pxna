import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { type ReactNode, useEffect, useRef, useState } from "react";
import { folderListingStore } from "../library/listing";
import { folderQuery } from "../library/queries";
import { useTransfer } from "../library/transfer";
import type { TreeNode } from "../library/tree";
import { searchHref } from "../location";
import { ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuTrigger } from "./ui/context-menu";

export function TreeMenu({
    node,
    narrow,
    children,
    onCreate,
    onRename,
    onDelete,
}: {
    node: TreeNode;
    narrow: boolean;
    children: ReactNode;
    onCreate(): void;
    onRename(): void;
    onDelete(): void;
}) {
    const transfer = useTransfer();
    const navigate = useNavigate();
    const [open, setOpen] = useState(false);
    const preserveFocus = useRef(false);
    const client = useQueryClient();
    const [detailOwner] = useState(() => ({}));
    useEffect(() => {
        if (!open || narrow || node.path === "/") return;
        const parent = node.path.slice(0, node.path.lastIndexOf("/")) || "/";
        const store = folderListingStore(client, node.libraryId, parent, { key: "name", order: "asc" }, true);
        store.setDetailPriority(detailOwner, [node.path]);
        return () => store.releaseDetailPriority(detailOwner);
    }, [open, narrow, node.libraryId, node.path, client, detailOwner]);
    const folder = useQuery({ ...folderQuery(node.libraryId, node.path), enabled: !narrow && open });
    if (narrow) return children;
    const canCreate = !node.readonly && folder.isSuccess && !folder.data.metadata.readonly;
    return (
        <ContextMenu onOpenChange={setOpen}>
            <ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
            <ContextMenuContent
                onCloseAutoFocus={(event) => {
                    if (preserveFocus.current) {
                        event.preventDefault();
                        preserveFocus.current = false;
                    }
                }}
            >
                <ContextMenuItem
                    disabled={!canCreate}
                    onSelect={() => {
                        preserveFocus.current = true;
                        requestAnimationFrame(onCreate);
                    }}
                >
                    新しいフォルダ
                </ContextMenuItem>
                <ContextMenuItem
                    disabled={!transfer?.canPaste || !canCreate}
                    onSelect={() => {
                        void transfer?.paste({ libraryId: node.libraryId, path: node.path });
                    }}
                >
                    貼り付け
                </ContextMenuItem>
                <ContextMenuItem
                    onSelect={() =>
                        void navigate({ href: searchHref({ libraryIds: [node.libraryId], path: node.path, tags: [] }) })
                    }
                >
                    このフォルダ内を検索
                </ContextMenuItem>
                {node.path !== "/" && (
                    <>
                        <ContextMenuItem
                            disabled={node.readonly}
                            onSelect={() => {
                                preserveFocus.current = true;
                                requestAnimationFrame(onRename);
                            }}
                        >
                            名前の変更
                        </ContextMenuItem>
                        <ContextMenuItem disabled={node.readonly} onSelect={onDelete}>
                            削除
                        </ContextMenuItem>
                    </>
                )}
            </ContextMenuContent>
        </ContextMenu>
    );
}
