import { t } from "i18next";
import { type ReactNode, useState } from "react";

import {
    INFO_PANEL_WIDTH,
    INFO_PANEL_WIDTH_KEY,
    TREE_WIDTH,
    TREE_WIDTH_KEY,
    usePanelWidth,
} from "../library/panelWidth";
import { useNarrowScreen } from "../library/preferences";
import type { BrowseLocation } from "../location";
import { ActivityPanel } from "./ActivityPanel";
import { ResizeHandle } from "./ResizeHandle";
import { NarrowHeader, type SearchChrome, Toolbar } from "./Toolbar";
import { Tree } from "./Tree";
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from "./ui/sheet";

interface AppShellProps {
    /// 今いる位置。ツリーで強調し、パンくずに出す。
    location?: BrowseLocation;
    /// 検索モードか。あれば、パンくずの代わりに「検索」と出し、狭い画面ではヘッダーの下に検索語の欄を出す。
    search?: SearchChrome;
    /// タイムラインか。パンくずと検索の代わりに「タイムライン」と出す。
    timeline?: boolean;
    /// 左の領域。省略するとツリーを出す。狭い画面では、ドロワーの中に出す。引数で、ドロワーを閉じられる。
    sidebar?: (close: () => void) => ReactNode;
    /// ツールバーの右に並べる操作。
    actions?: ReactNode;
    children?: ReactNode;
    /// 内容の右に置くパネル。広い画面にだけ出す。
    panel?: ReactNode;
    /// 下のステータスバー。狭い画面には出さない。
    footer?: ReactNode;
}

/// ツールバー、左のツリー、右の内容、下のステータスバーを持つ画面の外枠。
/// 狭い画面では、ツリーをドロワーに畳み、ステータスバーを出さない。
export function AppShell({
    location,
    search,
    timeline = false,
    sidebar,
    actions,
    children,
    panel,
    footer,
}: AppShellProps) {
    const narrow = useNarrowScreen();
    const [drawerOpen, setDrawerOpen] = useState(false);

    // フォルダを選んで位置が変わったら、ドロワーを閉じる。位置を移すたびに effect で状態を直すと再描画が重なるので、描画の中で前の位置と比べて直す。
    const key = location === undefined ? undefined : `${location.libraryId}:${location.path}`;
    const [seenKey, setSeenKey] = useState(key);
    if (key !== seenKey) {
        setSeenKey(key);
        setDrawerOpen(false);
    }

    const treeWidth = usePanelWidth(TREE_WIDTH_KEY, TREE_WIDTH);
    const panelWidth = usePanelWidth(INFO_PANEL_WIDTH_KEY, INFO_PANEL_WIDTH);
    const closeDrawer = () => setDrawerOpen(false);
    const left = sidebar ? sidebar(closeDrawer) : <Tree location={location} />;

    if (narrow) {
        return (
            <div className="flex h-dvh flex-col bg-background text-sm text-foreground">
                <NarrowHeader
                    location={location}
                    search={search}
                    timeline={timeline}
                    actions={actions}
                    onOpenTree={() => setDrawerOpen(true)}
                />
                <main className="min-h-0 flex-1 overflow-hidden">{children}</main>
                <Sheet open={drawerOpen} onOpenChange={setDrawerOpen}>
                    <SheetContent side="left" className="w-72 p-0">
                        <SheetHeader className="border-b">
                            <SheetTitle>
                                {timeline
                                    ? t("appShell.libraryAndMonth")
                                    : search
                                      ? t("appShell.searchConditions")
                                      : t("common.folder")}
                            </SheetTitle>
                            <SheetDescription className="sr-only">
                                {timeline
                                    ? t("appShell.timelineDescription")
                                    : search
                                      ? t("appShell.searchDescription")
                                      : t("appShell.browseDescription")}
                            </SheetDescription>
                        </SheetHeader>
                        <nav
                            aria-label={search ? t("appShell.searchConditions") : t("appShell.libraryNav")}
                            className="min-h-0 flex-1 overflow-y-auto"
                        >
                            {left}
                        </nav>
                    </SheetContent>
                </Sheet>
            </div>
        );
    }

    return (
        <div className="flex h-dvh flex-col bg-background text-sm text-foreground">
            <Toolbar location={location} search={search} timeline={timeline} actions={actions} />
            <div className="flex min-h-0 flex-1">
                <nav
                    aria-label={search ? t("appShell.searchConditions") : t("appShell.libraryNav")}
                    className="shrink-0 overflow-y-auto border-r bg-sidebar"
                    style={{ width: sidebar ? (search ? "16rem" : "14rem") : treeWidth.width }}
                >
                    {left}
                </nav>
                {/* 幅を変えられるのはツリーだけで、タイムラインの月の一覧と検索の条件（`sidebar`）は固定である。 */}
                {!sidebar && (
                    <ResizeHandle
                        label={t("appShell.treeWidth")}
                        range={TREE_WIDTH}
                        width={treeWidth.width}
                        grows="right"
                        onResize={treeWidth.resize}
                        onCommit={treeWidth.commit}
                    />
                )}
                <main className="min-w-0 flex-1 overflow-hidden">{children}</main>
                {panel !== undefined && (
                    <>
                        <ResizeHandle
                            label={t("appShell.infoWidth")}
                            range={INFO_PANEL_WIDTH}
                            width={panelWidth.width}
                            grows="left"
                            onResize={panelWidth.resize}
                            onCommit={panelWidth.commit}
                        />
                        <aside
                            aria-label={t("appShell.infoPanel")}
                            className="shrink-0 overflow-y-auto border-l"
                            style={{ width: panelWidth.width }}
                        >
                            {panel}
                        </aside>
                    </>
                )}
            </div>
            {footer}
            <ActivityPanel />
        </div>
    );
}
