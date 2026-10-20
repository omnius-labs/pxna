import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { ChevronLeft, LogOut, Menu, Search, User } from "lucide-react";
import { type ReactNode, useEffect } from "react";
import { toast } from "sonner";

import { ApiError } from "../api/client";
import { currentUserQuery, logout } from "../auth/session";
import { breadcrumbs, parentLocation } from "../library/breadcrumb";
import { librariesQuery } from "../library/queries";
import { isTypingTarget } from "../library/shortcuts";
import { type BrowseLocation, browseHref, pathSegments, type SearchLocation, searchHref } from "../location";
import { BrowseLink } from "./BrowseLink";
import { ModeSwitch } from "./ModeSwitch";
import { SearchBox } from "./SearchBox";
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

interface ToolbarProps {
    /// 今いる位置。Library を選ぶ前は `undefined`。
    location?: BrowseLocation;
    /// 検索モードの位置。あれば、パンくずの代わりに検索の操作を出す。
    search?: SearchLocation;
    /// タイムラインの Library。あれば、パンくずの代わりに、Library の名前を出す。
    timelineLibraryId?: string;
    /// 表示の操作など、ツールバーの右に並べる操作。
    actions?: ReactNode;
    /// 狭い画面でだけ使う、ツリーのドロワーを開く操作。
    onOpenTree?: () => void;
}

/// 広い画面のツールバー。パンくずと、操作と、ユーザのメニューを並べる。
export function Toolbar({ location, search, timelineLibraryId, actions }: ToolbarProps) {
    const libraryId = timelineLibraryId ?? location?.libraryId;
    const libraryName = useLibraryName(libraryId);
    return (
        <header className="flex h-12 shrink-0 items-center gap-2 border-b px-3">
            <div className="flex w-auto shrink-0 lg:w-[13.25rem] items-center gap-2">
                <div className="flex size-7 items-center justify-center rounded-lg bg-primary text-sm font-bold text-primary-foreground">
                    P
                </div>
                <span className="font-heading text-sm font-semibold tracking-tight">Pxna</span>
            </div>
            {libraryId !== undefined && (
                <ModeSwitch
                    libraryId={libraryId}
                    mode={search ? "search" : timelineLibraryId ? "timeline" : "folder"}
                />
            )}
            {timelineLibraryId !== undefined && <TimelineTitle libraryId={timelineLibraryId} />}
            {location !== undefined && !search && <PathBreadcrumb location={location} />}
            {search !== undefined && <SearchBox search={search} libraryName={libraryName} shortcut />}
            {libraryId !== undefined && !search && (
                <SearchEntry libraryId={libraryId} path={location?.path ?? "/"} shortcut />
            )}
            <div className="ml-auto flex items-center gap-1">
                {actions}
                <UserMenu />
            </div>
        </header>
    );
}

/// 狭い画面のヘッダー。ツリーのドロワーを開く操作、1 つ上のフォルダへの操作、今のフォルダの名前と、操作を並べる。
export function NarrowHeader({ location, search, timelineLibraryId, actions, onOpenTree }: ToolbarProps) {
    const navigate = useNavigate();
    const parent = location === undefined ? undefined : parentLocation(location);
    const libraryId = timelineLibraryId ?? location?.libraryId;
    const libraryName = useLibraryName(libraryId);
    const segments = location === undefined ? [] : pathSegments(location.path);
    const title = search ? "検索" : timelineLibraryId ? libraryName : (segments.at(-1) ?? libraryName);
    const trail = search
        ? [libraryName, ...segments].join(" / ")
        : timelineLibraryId
          ? "タイムライン"
          : segments.length > 0
            ? [libraryName, ...segments.slice(0, -1)].join(" / ")
            : "";

    return (
        <>
            <header className="flex h-12 shrink-0 items-center gap-1 border-b px-1.5">
                <Button variant="ghost" size="icon" aria-label="フォルダ" onClick={onOpenTree}>
                    <Menu />
                </Button>
                {timelineLibraryId === undefined && (
                    <Button
                        variant="ghost"
                        size="icon"
                        aria-label="上のフォルダ"
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
                {libraryId !== undefined && (
                    <ModeSwitch
                        libraryId={libraryId}
                        mode={search ? "search" : timelineLibraryId ? "timeline" : "folder"}
                        compact
                    />
                )}
                {libraryId !== undefined && !search && (
                    <SearchEntry libraryId={libraryId} path={location?.path ?? "/"} />
                )}
                {actions}
                <UserMenu />
            </header>
            {search !== undefined && (
                <div className="shrink-0 border-b p-2">
                    <SearchBox search={search} libraryName={libraryName} />
                </div>
            )}
        </>
    );
}

function useLibraryName(libraryId: string | undefined): string {
    const libraries = useQuery(librariesQuery);
    return libraries.data?.find((library) => library.id === libraryId)?.name ?? libraryId ?? "Pxna";
}

/// 検索モードへ移る入口。起点の既定として、今のフォルダ（タイムラインでは Library 全体）を渡す。
function SearchEntry({ libraryId, path, shortcut = false }: { libraryId: string; path: string; shortcut?: boolean }) {
    const navigate = useNavigate();
    const enter = () => void navigate({ href: searchHref({ libraryId, path }) });
    useEffect(() => {
        if (!shortcut) return;
        const keyDown = (event: KeyboardEvent) => {
            if (event.key !== "/" || event.altKey || event.ctrlKey || event.metaKey || isTypingTarget(event.target))
                return;
            event.preventDefault();
            void navigate({ href: searchHref({ libraryId, path }) });
        };
        window.addEventListener("keydown", keyDown);
        return () => window.removeEventListener("keydown", keyDown);
    }, [shortcut, libraryId, path, navigate]);
    return (
        <Button variant="ghost" size="icon" aria-label="検索" onClick={enter}>
            <Search />
        </Button>
    );
}

/// タイムラインで、Library の名前を出す。
function TimelineTitle({ libraryId }: { libraryId: string }) {
    const libraries = useQuery(librariesQuery);
    const name = libraries.data?.find((library) => library.id === libraryId)?.name ?? libraryId;
    return <span className="min-w-0 truncate font-medium">{name}</span>;
}

/// Library から今のフォルダまでの経路。最後の項目だけが現在の位置で、ほかは移動できる。
function PathBreadcrumb({ location }: { location: BrowseLocation }) {
    const libraries = useQuery(librariesQuery);
    const libraryName =
        libraries.data?.find((library) => library.id === location.libraryId)?.name ?? location.libraryId;
    const crumbs = breadcrumbs(location, libraryName);
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
            toast.error("ログアウトできませんでした。");
        },
    });

    return (
        <DropdownMenu>
            <DropdownMenuTrigger asChild>
                <Button variant="ghost" size="icon" aria-label="ユーザ">
                    <User />
                </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-48">
                <DropdownMenuLabel className="truncate">{me.data?.user.username}</DropdownMenuLabel>
                <DropdownMenuSeparator />
                <DropdownMenuItem disabled={signOut.isPending} onSelect={() => signOut.mutate()}>
                    <LogOut />
                    ログアウト
                </DropdownMenuItem>
            </DropdownMenuContent>
        </DropdownMenu>
    );
}
