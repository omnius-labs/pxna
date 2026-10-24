import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { Clock, FolderTree } from "lucide-react";
import type { MouseEvent } from "react";

import { lastFolderLocation } from "../library/folderReturn";
import { librariesQuery } from "../library/queries";
import { browseHref, timelineHref } from "../location";

interface ModeSwitchProps {
    /// フォルダ表示へ移るときの Library。タイムラインでは `undefined` で、最後に開いていたフォルダ（なければ先頭の Library のルート）へ移る。
    libraryId: string | undefined;
    /// 今の表示。検索モードでは、どちらも強調しない。
    mode: "folder" | "timeline" | "search";
    /// アイコンだけで出す。狭い画面のヘッダーで使う。
    compact?: boolean;
}

/// フォルダ表示とタイムラインを行き来する切り替え。タイムラインは Library を横断し、並べる Library を変えない。
export function ModeSwitch({ libraryId, mode, compact = false }: ModeSwitchProps) {
    const navigate = useNavigate();
    const libraries = useQuery(librariesQuery);
    const folder =
        libraryId !== undefined
            ? { libraryId, path: "/" }
            : (lastFolderLocation() ??
              (libraries.data?.[0] === undefined ? undefined : { libraryId: libraries.data[0].id, path: "/" }));
    const items = [
        {
            mode: "folder",
            label: "フォルダ",
            href: folder === undefined ? "/" : browseHref(folder),
            icon: <FolderTree className="size-4" />,
        },
        {
            mode: "timeline",
            label: "タイムライン",
            href: timelineHref({}),
            icon: <Clock className="size-4" />,
        },
    ] as const;
    const click = (event: MouseEvent<HTMLAnchorElement>, href: string) => {
        if (event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
        event.preventDefault();
        void navigate({ href });
    };
    return (
        <nav aria-label="表示の切り替え" className="flex shrink-0 items-center rounded-md border p-0.5">
            {items.map((item) => (
                <a
                    key={item.mode}
                    href={item.href}
                    aria-current={item.mode === mode ? "page" : undefined}
                    aria-label={compact ? item.label : undefined}
                    title={item.label}
                    onClick={(event) => click(event, item.href)}
                    className={`flex items-center gap-1 rounded px-2 py-1 text-xs ${item.mode === mode ? "bg-accent font-medium text-accent-foreground" : "text-muted-foreground hover:text-foreground"}`}
                >
                    {item.icon}
                    {!compact && item.label}
                </a>
            ))}
        </nav>
    );
}
