import { useNavigate } from "@tanstack/react-router";
import { Clock, FolderTree } from "lucide-react";
import type { MouseEvent } from "react";

import { browseHref, timelineHref } from "../location";

interface ModeSwitchProps {
    libraryId: string;
    /// 今の表示。検索モードでは、どちらも強調しない。
    mode: "folder" | "timeline" | "search";
    /// アイコンだけで出す。狭い画面のヘッダーで使う。
    compact?: boolean;
}

/// 同じ Library の、フォルダ表示とタイムラインを行き来する切り替え。
export function ModeSwitch({ libraryId, mode, compact = false }: ModeSwitchProps) {
    const navigate = useNavigate();
    const items = [
        {
            mode: "folder",
            label: "フォルダ",
            href: browseHref({ libraryId, path: "/" }),
            icon: <FolderTree className="size-4" />,
        },
        {
            mode: "timeline",
            label: "タイムライン",
            href: timelineHref({ libraryId }),
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
