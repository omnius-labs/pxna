import { useNavigate } from "@tanstack/react-router";
import type { MouseEvent, ReactNode } from "react";

import { type BrowseLocation, browseHref } from "../location";

interface BrowseLinkProps {
    location: BrowseLocation;
    className?: string;
    children: ReactNode;
}

/// 位置への link。修飾キーのないクリックは画面を読み直さずに移り、それ以外はブラウザの既定の動作に任せる。
export function BrowseLink({ location, className, children }: BrowseLinkProps) {
    const navigate = useNavigate();
    const href = browseHref(location);

    const click = (event: MouseEvent<HTMLAnchorElement>) => {
        if (event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) {
            return;
        }
        event.preventDefault();
        void navigate({ href });
    };

    return (
        <a href={href} className={className} onClick={click}>
            {children}
        </a>
    );
}
