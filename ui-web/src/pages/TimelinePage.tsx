import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { RefreshCw } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";

import { AppShell } from "../components/AppShell";
import { DeleteDialog, type DeleteTarget } from "../components/DeleteDialog";
import { EntryToolbar } from "../components/EntryMenu";
import { InfoPanel, InfoPanelToggle } from "../components/InfoPanel";
import { EntryNameDialog } from "../components/NameDialog";
import { SettingsDialog } from "../components/SettingsDialog";
import { StatusBar } from "../components/StatusBar";
import { TimelineGrid } from "../components/TimelineGrid";
import { TimelineSidebar } from "../components/TimelineSidebar";
import { Button } from "../components/ui/button";
import { ViewControls } from "../components/ViewControls";
import { TimelineViewer } from "../components/Viewer";
import { EntryActionContext, type EntryEditing } from "../library/actionContext";
import { useClipboard } from "../library/clipboard";
import { type Entry, entryKind } from "../library/entries";
import { useInfoPanel } from "../library/infoPanel";
import { usePreferences } from "../library/preferences";
import { librariesQuery } from "../library/queries";
import { emptySelection, type Selection, selectionBytes } from "../library/selection";
import { clipboardCommand, isTypingTarget } from "../library/shortcuts";
import {
    refreshTimeline,
    startTimelineScan,
    type TimelineAnchor,
    timelineLibraryIds,
    timelineMonthsQuery,
    timelineStatusQuery,
    useTimelineStore,
} from "../library/timeline";
import type { SelectionProgress } from "../library/useEntryInteraction";
import {
    browseHref,
    CROSS_LIBRARY_SCOPE,
    entryAddress,
    parentPath,
    relativePath,
    timelineHref,
    timelineLocation,
    viewKey,
} from "../location";
import { fileContentUrl } from "../viewer/urls";

/// 見ている月を、URL に載せるまでの待ち。スクロールの間に履歴の置き換えが続かないようにする。
const MONTH_WRITE_DELAY_MS = 300;

/// 集計中に、状態と月ごとの件数を読み直す間隔。
const STATUS_POLL_MS = 2_000;
const MONTHS_POLL_MS = 10_000;

export function TimelinePage() {
    const search = useSearch({ from: "/authed/timeline" });
    const libraries = useQuery(librariesQuery);
    const { preferences } = usePreferences();
    const libraryIds = useMemo(
        () => timelineLibraryIds(libraries.data ?? [], preferences.timelineLibraries),
        [libraries.data, preferences.timelineLibraries],
    );
    if (libraries.data === undefined || libraryIds.length === 0) {
        return (
            <AppShell timeline>
                <p className="p-4 text-muted-foreground">
                    {libraries.isError
                        ? "Library を読めませんでした。"
                        : libraries.data === undefined
                          ? "Library を読み込み中…"
                          : "Library がありません。"}
                </p>
            </AppShell>
        );
    }
    // 並べる Library を変えたら、状態をすべて作り直す。
    return <TimelineScreen key={libraryIds.join("\n")} libraryIds={libraryIds} search={search} />;
}

function TimelineScreen({
    libraryIds,
    search,
}: {
    libraryIds: readonly string[];
    search: { month?: string; view?: string };
}) {
    const location = timelineLocation(search);
    const navigate = useNavigate();
    const client = useQueryClient();
    const libraries = useQuery(librariesQuery);
    const readonlyLibraries = new Set(
        (libraries.data ?? []).filter((library) => library.readonly).map((library) => library.id),
    );
    const isReadonlyLibrary = (libraryId: string) => !libraries.data || readonlyLibraries.has(libraryId);
    const { set, narrow, updateTimelineLibraries } = usePreferences();
    const infoPanel = useInfoPanel();
    const clipboard = useClipboard();

    // 読み込みの起点は、月を選んだとき（URL の月が、自分で書いたものと違うとき）だけ変える。スクロールで見ている月を URL に書いても、読み直さない。
    const written = useRef(location.month);
    const [seenMonth, setSeenMonth] = useState(location.month);
    const [anchorMonth, setAnchorMonth] = useState(location.month);
    // 月を選ぶたびに増やす。同じ月を選び直しても、その月の先頭から読み直す。
    const [jump, setJump] = useState(0);
    if (location.month !== seenMonth) {
        setSeenMonth(location.month);
        if (location.month !== written.current) setAnchorMonth(location.month);
    }
    const anchor: TimelineAnchor =
        anchorMonth === undefined ? { kind: "newest" } : { kind: "month", month: anchorMonth, nonce: jump };
    const store = useTimelineStore(libraryIds, anchor);

    // 並べている Library のどれかが集計中なら集計中とし、処理したファイルの数は和を出す。
    const status = useQuery({
        ...timelineStatusQuery(libraryIds),
        refetchInterval: (query) => (query.state.data?.some((library) => library.running) ? STATUS_POLL_MS : false),
    });
    const running = status.data?.some((library) => library.running) ?? false;
    const processedFiles = status.data?.reduce((sum, library) => sum + library.processedFiles, 0) ?? 0;
    useQuery({
        ...timelineMonthsQuery(libraryIds),
        refetchInterval: running ? MONTHS_POLL_MS : false,
    });
    // 集計が終わったら、月ごとの件数を最後に 1 回読み直す。
    const wasRunning = useRef(running);
    useEffect(() => {
        if (wasRunning.current && !running)
            void client.invalidateQueries({ queryKey: timelineMonthsQuery(libraryIds).queryKey });
        wasRunning.current = running;
    }, [running, client, libraryIds]);

    const [selection, setSelection] = useState<Selection>(emptySelection);
    const [editing, setEditing] = useState<EntryEditing>();
    const [deletion, setDeletion] = useState<DeleteTarget>();
    const [progress, setProgress] = useState<SelectionProgress>();
    const [visibleMonth, setVisibleMonth] = useState<string>();
    // 範囲が変わったら、選択と編集を捨てる。位置を移すたびに effect で状態を直すと再描画が重なるので、描画の中で直す。
    const [seenStore, setSeenStore] = useState(store);
    if (seenStore !== store) {
        setSeenStore(store);
        setSelection(emptySelection);
        setEditing(undefined);
        setDeletion(undefined);
        setProgress(undefined);
        // 古い範囲で見ていた月を、新しい範囲の URL へ書き戻さない。
        setVisibleMonth(undefined);
    }
    if (narrow && (editing || deletion)) {
        setEditing(undefined);
        setDeletion(undefined);
    }
    const finish = useCallback(() => setEditing(undefined), []);

    // 見ている月を、履歴に積まずに URL へ載せる。
    const activeMonth = visibleMonth ?? location.month;
    const { view } = location;
    useEffect(() => {
        if (visibleMonth === undefined || visibleMonth === location.month) return;
        const timer = setTimeout(() => {
            written.current = visibleMonth;
            void navigate({ href: timelineHref({ month: visibleMonth, view }), replace: true });
        }, MONTH_WRITE_DELAY_MS);
        return () => clearTimeout(timer);
    }, [visibleMonth, location.month, view, navigate]);

    const selectMonth = (month: string) => {
        setAnchorMonth(month);
        setJump((count) => count + 1);
        void navigate({ href: timelineHref({ month }) });
    };

    // 選択の鍵は Library を含むので、選んだ Entry のどれかが読み取り専用の Library にあれば、変更の操作を出さない。
    // 選択が複数の Library にまたがるときは、操作を Library ごとの要求に分ける。
    const libraryReadonly = [...selection.items.keys()].some((key) =>
        isReadonlyLibrary(entryAddress(CROSS_LIBRARY_SCOPE, key).libraryId),
    );
    const canRename =
        !narrow &&
        !libraryReadonly &&
        selection.items.size === 1 &&
        [...selection.items.values()].every((item) => !item.readonly);
    const rename = useCallback(() => {
        if (!canRename) return;
        const [path, item] = [...selection.items][0]!;
        setEditing({ type: "rename", path, folder: item.folder });
    }, [canRename, selection.items]);
    const canDelete =
        !narrow &&
        !libraryReadonly &&
        selection.items.size > 0 &&
        [...selection.items.values()].every((item) => !item.readonly);
    const requestDelete = useCallback(() => {
        if (canDelete) setDeletion({ libraryId: CROSS_LIBRARY_SCOPE, items: new Map(selection.items) });
    }, [canDelete, selection.items]);
    const cut = useCallback(() => {
        if (canDelete) clipboard.store?.write("cut", CROSS_LIBRARY_SCOPE, selection.items);
    }, [canDelete, clipboard.store, selection.items]);
    const copy = useCallback(
        () => clipboard.store?.write("copy", CROSS_LIBRARY_SCOPE, selection.items),
        [clipboard.store, selection.items],
    );

    // 選んだ 1 件の親フォルダを、フォルダ表示で開き、その Entry を選んで見せる。archive の中の Entry では、archive の中のフォルダを開く。
    const reveal = useCallback(() => {
        const only = selection.items.size === 1 ? [...selection.items.keys()][0] : undefined;
        if (only === undefined) return;
        const { libraryId, path } = entryAddress(CROSS_LIBRARY_SCOPE, only);
        void navigate({
            href: browseHref({ libraryId, path: parentPath(path), reveal: path.split("/").at(-1) }),
        });
    }, [selection.items, navigate]);

    useEffect(() => {
        const keyDown = (event: globalThis.KeyboardEvent) => {
            if (
                narrow ||
                isTypingTarget(event.target) ||
                document.querySelector('[role="dialog"], [role="alertdialog"]')
            )
                return;
            const command = clipboardCommand(event);
            if (
                (command === "cut" || command === "copy") &&
                selection.items.size > 0 &&
                (command === "copy" || canDelete)
            ) {
                event.preventDefault();
                if (command === "cut") cut();
                else copy();
            } else if (
                (event.key === "Delete" || ((event.metaKey || event.ctrlKey) && event.key === "Backspace")) &&
                canDelete
            ) {
                event.preventDefault();
                requestDelete();
            } else if (event.key === "F2" && canRename) {
                event.preventDefault();
                rename();
            }
        };
        window.addEventListener("keydown", keyDown);
        return () => window.removeEventListener("keydown", keyDown);
    }, [narrow, canDelete, canRename, requestDelete, rename, selection.items.size, cut, copy]);

    const selectedFile = selection.items.size === 1 ? [...selection.items][0] : undefined;
    const downloadHref =
        selectedFile && !selectedFile[1].folder ? fileContentUrl(CROSS_LIBRARY_SCOPE, selectedFile[0]) : undefined;
    const selectedBytes = selectionBytes(selection);

    // 画像と動画はビューアで開く。ビューアを開くときは履歴に積み、前後へ移ったり閉じたりするときは積まない。`view` は Library を含む鍵である。
    const open = (entry: Entry) => {
        const kind = entryKind(entry);
        if (kind === "image" || kind === "video")
            void navigate({ href: timelineHref({ ...location, view: relativePath("/", entry.path) }) });
    };
    const changeView = (name: string | undefined) =>
        void navigate({ href: timelineHref({ ...location, view: name }), replace: true });

    const refresh = async () => {
        try {
            await startTimelineScan(libraryIds);
            await refreshTimeline(client, libraryIds);
        } catch {
            toast.error("更新できませんでした。");
        }
    };

    return (
        <EntryActionContext.Provider
            value={{
                libraryId: CROSS_LIBRARY_SCOPE,
                libraryReadonly,
                narrow,
                canCreate: false,
                selection,
                setSelection,
                editing,
                create: () => {},
                cut,
                copy,
                rename,
                downloadHref,
                delete: requestDelete,
                finish,
                crossLibrary: true,
                reveal,
            }}
        >
            <AppShell
                timeline
                sidebar={(close) => (
                    <TimelineSidebar
                        libraryIds={libraryIds}
                        onChangeLibraries={updateTimelineLibraries}
                        activeMonth={activeMonth}
                        onSelectMonth={(month) => {
                            close();
                            selectMonth(month);
                        }}
                    />
                )}
                actions={
                    <>
                        <EntryToolbar />
                        <Button
                            variant="ghost"
                            size="icon"
                            aria-label="今すぐ更新"
                            title="今すぐ更新"
                            onClick={() => void refresh()}
                        >
                            <RefreshCw />
                        </Button>
                        <ViewControls timeline />
                        {!narrow && <InfoPanelToggle open={infoPanel.open} onToggle={infoPanel.toggle} />}
                        <SettingsDialog />
                    </>
                }
                panel={infoPanel.open ? <InfoPanel libraryId={CROSS_LIBRARY_SCOPE} selection={selection} /> : undefined}
                footer={
                    <StatusBar
                        totalCount={store.loadedItems.length}
                        countLabel="件読み込み済み"
                        selectedCount={selection.items.size}
                        selectedBytes={selectedBytes}
                        progress={progress}
                    />
                }
            >
                <div className="flex h-full flex-col">
                    {running && (
                        <p role="status" className="shrink-0 border-b p-2 text-sm text-muted-foreground">
                            集計中… {processedFiles} 件を処理しました。
                        </p>
                    )}
                    <div className="min-h-0 flex-1">
                        <TimelineGrid
                            key={`${anchorMonth ?? "newest"}:${jump}`}
                            store={store}
                            libraryId={CROSS_LIBRARY_SCOPE}
                            tileSize={set.tileSize}
                            shape={set.thumbnailShape}
                            narrow={narrow}
                            selection={selection}
                            onSelectionChange={setSelection}
                            onProgress={setProgress}
                            onVisibleMonth={setVisibleMonth}
                            onOpen={open}
                            emptyMessage={
                                running
                                    ? "集計中です。画像と動画が見つかると、ここに並びます。"
                                    : "画像と動画がありません。"
                            }
                        />
                    </div>
                </div>
            </AppShell>
            <EntryNameDialog />
            <DeleteDialog
                target={narrow ? undefined : deletion}
                onClose={() => setDeletion(undefined)}
                onDeleted={() => setSelection(emptySelection)}
            />
            {view !== undefined && (
                <TimelineViewer
                    libraryIds={libraryIds}
                    view={view}
                    originalImage={set.originalImage}
                    narrow={narrow}
                    libraryReadonly={isReadonlyLibrary(entryAddress(CROSS_LIBRARY_SCOPE, viewKey(view)).libraryId)}
                    onChange={changeView}
                />
            )}
        </EntryActionContext.Provider>
    );
}
