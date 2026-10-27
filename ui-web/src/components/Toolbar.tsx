import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { t } from "i18next";
import { ChevronLeft, LogOut, Menu, User } from "lucide-react";
import type { ReactNode } from "react";
import { toast } from "sonner";

import { ApiError } from "../api/client";
import { currentUserQuery, logout } from "../auth/session";
import { breadcrumbs, parentLocation } from "../library/breadcrumb";
import { librariesQuery } from "../library/queries";
import { type BrowseLocation, browseHref, pathSegments } from "../location";
import { BrowseLink } from "./BrowseLink";
import { ModeSwitch } from "./ModeSwitch";
import {
    Breadcrumb,
    BreadcrumbItem,
    BreadcrumbLink,
    BreadcrumbList,
    BreadcrumbPage,
    BreadcrumbSeparator,
} from "./ui/breadcrumb";
import { Button } from "./ui/button";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuLabel,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
} from "./ui/dropdown-menu";

/// 検索モードが外枠に渡すもの。
export interface SearchChrome {
    /// 狭い画面でヘッダーの下に常に出す、検索語の欄。
    field: ReactNode;
    /// 検索語のほかに絞り込んでいる条件の数。狭い画面のドロワーのボタンに出す。
    conditions: number;
}

interface ToolbarProps {
    /// 今いる位置。Library を選ぶ前は `undefined`。
    location?: BrowseLocation;
    /// 検索モードか。広い画面ではパンくずを出さず、狭い画面では今のフォルダの代わりに「検索」と出す。
    search?: SearchChrome;
    /// タイムラインか。Library を横断するので、広い画面ではパンくずを出さず、狭い画面では今のフォルダの代わりに「タイムライン」と出す。
    timeline?: boolean;
    /// 表示の操作など、ツールバーの右に並べる操作。
    actions?: ReactNode;
    /// 狭い画面でだけ使う、ツリーのドロワーを開く操作。
    onOpenTree?: () => void;
}

/// 広い画面のツールバー。表示の切り替え、パンくず、操作、ユーザのメニューを並べる。
/// 今の表示は切り替えの強調で、今の Library はツリーで分かるので、表示の名前と Library の名前は文字で出さない。
export function Toolbar({ location, search, timeline = false, actions }: ToolbarProps) {
    const libraryId = location?.libraryId;
    return (
        <header className="flex h-12 shrink-0 items-center gap-2 border-b px-3">
            <div className="flex w-auto shrink-0 lg:w-[13.25rem] items-center gap-2">
                <div className="flex size-7 items-center justify-center rounded-lg bg-primary text-sm font-bold text-primary-foreground">
                    P
                </div>
                <span className="font-heading text-sm font-semibold tracking-tight">Pxna</span>
            </div>
            {(libraryId !== undefined || timeline || search) && (
                <ModeSwitch libraryId={libraryId} mode={search ? "search" : timeline ? "timeline" : "folder"} />
            )}
            {location !== undefined && !search && <PathBreadcrumb location={location} />}
            <div className="ml-auto flex items-center gap-1">
                {actions}
                <UserMenu />
            </div>
        </header>
    );
}

/// 狭い画面のヘッダー。ドロワーを開く操作、1 つ上のフォルダへの操作、今のフォルダの名前と、操作を並べる。
/// 検索モードでは、ヘッダーの下に検索語の欄を常に出し、ドロワーのボタンに絞り込んでいる条件の数を出す。
export function NarrowHeader({ location, search, timeline = false, actions, onOpenTree }: ToolbarProps) {
    const navigate = useNavigate();
    const parent = location === undefined ? undefined : parentLocation(location);
    const libraryId = location?.libraryId;
    const libraryName = useLibraryName(libraryId);
    const segments = location === undefined ? [] : pathSegments(location.path);
    const title = search ? t("common.search") : timeline ? t("common.timeline") : (segments.at(-1) ?? libraryName);
    const trail =
        search || timeline ? "" : segments.length > 0 ? [libraryName, ...segments.slice(0, -1)].join(" / ") : "";

    return (
        <>
            <header className="flex h-12 shrink-0 items-center gap-1 border-b px-1.5">
                <Button
                    variant="ghost"
                    size="icon"
                    aria-label={
                        search
                            ? search.conditions > 0
                                ? t("toolbar.searchConditionsCount", { count: search.conditions })
                                : t("toolbar.searchConditions")
                            : t("common.folder")
                    }
                    className="relative"
                    onClick={onOpenTree}
                >
                    <Menu />
                    {search !== undefined && search.conditions > 0 && (
                        <span
                            aria-hidden="true"
                            className="absolute right-0.5 top-0.5 flex size-4 items-center justify-center rounded-full bg-primary text-[10px] text-primary-foreground"
                        >
                            {search.conditions}
                        </span>
                    )}
                </Button>
                {!timeline && !search && (
                    <Button
                        variant="ghost"
                        size="icon"
                        aria-label={t("toolbar.up")}
                        disabled={parent === undefined}
                        onClick={() => parent !== undefined && void navigate({ href: browseHref(parent) })}
                    >
                        <ChevronLeft />
                    </Button>
                )}
                <div className="min-w-0 flex-1">
                    <p className="truncate text-sm font-semibold">{title}</p>
                    {trail !== "" && <p className="truncate text-[11px] text-muted-foreground">{trail}</p>}
                </div>
                {(libraryId !== undefined || timeline || search) && (
                    <ModeSwitch
                        libraryId={libraryId}
                        mode={search ? "search" : timeline ? "timeline" : "folder"}
                        compact
                    />
                )}
                {actions}
                <UserMenu />
            </header>
            {search !== undefined && <div className="shrink-0 border-b p-2">{search.field}</div>}
        </>
    );
}

function useLibraryName(libraryId: string | undefined): string {
    const libraries = useQuery(librariesQuery);
    return libraries.data?.find((library) => library.id === libraryId)?.name ?? libraryId ?? "Pxna";
}

/// Library の直下から今のフォルダまでの経路。最後の項目だけが現在の位置で、ほかは移動できる。Library のルートでは何も出さない。
function PathBreadcrumb({ location }: { location: BrowseLocation }) {
    const crumbs = breadcrumbs(location);
    if (crumbs.length === 0) return null;
    return (
        <Breadcrumb className="min-w-0">
            <BreadcrumbList className="flex-nowrap">
                {crumbs.map((crumb, index) => {
                    const last = index === crumbs.length - 1;
                    return (
                        <span key={crumb.location.path} className="contents">
                            <BreadcrumbItem className={last ? "min-w-0" : "hidden lg:inline-flex"}>
                                {last ? (
                                    <BreadcrumbPage className="truncate font-medium">{crumb.label}</BreadcrumbPage>
                                ) : (
                                    <BreadcrumbLink asChild>
                                        <BrowseLink location={crumb.location}>{crumb.label}</BrowseLink>
                                    </BreadcrumbLink>
                                )}
                            </BreadcrumbItem>
                            {!last && <BreadcrumbSeparator className="hidden lg:inline-flex" />}
                        </span>
                    );
                })}
            </BreadcrumbList>
        </Breadcrumb>
    );
}

/// ユーザ名と、ログアウトを持つメニュー。
function UserMenu() {
    const me = useQuery(currentUserQuery);
    const navigate = useNavigate();
    const queryClient = useQueryClient();
    const toLogin = () => navigate({ to: "/login", search: {}, replace: true });
    const signOut = useMutation({
        mutationFn: () => logout(queryClient),
        onSuccess: toLogin,
        // セッションがすでに失効していれば、ログアウトできたのと同じ状態である。
        onError: async (error) => {
            if (error instanceof ApiError && error.unauthorized) {
                queryClient.clear();
                await toLogin();
                return;
            }
            toast.error(t("toolbar.logoutFailed"));
        },
    });

    return (
        <DropdownMenu>
            <DropdownMenuTrigger asChild>
                <Button variant="ghost" size="icon" aria-label={t("toolbar.user")}>
                    <User />
                </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-48">
                <DropdownMenuLabel className="truncate">{me.data?.user.username}</DropdownMenuLabel>
                <DropdownMenuSeparator />
                <DropdownMenuItem disabled={signOut.isPending} onSelect={() => signOut.mutate()}>
                    <LogOut />
                    {t("toolbar.logout")}
                </DropdownMenuItem>
            </DropdownMenuContent>
        </DropdownMenu>
    );
}
