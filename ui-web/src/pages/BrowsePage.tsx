import { useQuery } from "@tanstack/react-query";
import { useNavigate, useParams, useSearch } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import { AppShell } from "../components/AppShell";
import { DeleteDialog, type DeleteTarget } from "../components/DeleteDialog";
import { EntryToolbar } from "../components/EntryMenu";
import { Grid } from "../components/Grid";
import { InfoPanel, InfoPanelToggle } from "../components/InfoPanel";
import { List } from "../components/List";
import { LoadFailure } from "../components/LoadFailure";
import { EntryNameDialog } from "../components/NameDialog";
import { SettingsMenu } from "../components/SettingsMenu";
import { StatusBar } from "../components/StatusBar";
import { UploadButton } from "../components/UploadButton";
import { Viewer } from "../components/Viewer";
import { EntryActionContext, type EntryEditing } from "../library/actionContext";
import { useClipboard } from "../library/clipboard";
import { type Entry, entryKind, isFolderLike } from "../library/entries";
import { canMutate } from "../library/entryActions";
import { rememberFolder } from "../library/folderReturn";
import { useInfoPanel } from "../library/infoPanel";
import { usePreferences } from "../library/preferences";
import { folderQuery, librariesQuery } from "../library/queries";
import { emptySelection, type Selection, selectionBytes } from "../library/selection";
import { clipboardCommand, isTypingTarget } from "../library/shortcuts";
import { useTransfer } from "../library/transfer";
import type { SelectionProgress } from "../library/useEntryInteraction";
import { type BrowseLocation, browseHref, browseLocation, relativePath, searchHref } from "../location";
import { fileContentUrl } from "../viewer/urls";

export function BrowsePage() {
    const { libraryId, _splat } = useParams({ from: "/authed/browse/$libraryId/$" });
    const search = useSearch({ from: "/authed/browse/$libraryId/$" });
    const location = browseLocation(libraryId, _splat, search);
    // タイムラインからフォルダ表示へ戻るときの行き先にする。
    useEffect(() => rememberFolder({ libraryId, path: location.path }), [libraryId, location.path]);
    return <BrowseScreen location={location} />;
}

/// フォルダ表示の画面。
function BrowseScreen({ location }: { location: BrowseLocation }) {
    const { libraryId } = location;
    const navigate = useNavigate();
    const libraries = useQuery(librariesQuery);
    const libraryReadonly = libraries.data?.find((library) => library.id === libraryId)?.readonly ?? true;
    const { set, narrow, sort, updateSort } = usePreferences();
    const infoPanel = useInfoPanel();

    const folder = useQuery(folderQuery(libraryId, location.path));
    const canCreate = !narrow && folder.isSuccess && canMutate({ readonly: libraryReadonly }, folder.data);
    const [deletion, setDeletion] = useState<DeleteTarget>();
    const [editing, setEditing] = useState<EntryEditing>();
    const finish = useCallback(() => setEditing(undefined), []);
    if (narrow && (editing || deletion)) {
        setEditing(undefined);
        setDeletion(undefined);
    }
    const create = useCallback(() => {
        if (canCreate) setEditing({ type: "create", parent: location.path });
    }, [canCreate, location.path]);

    const transfer = useTransfer();
    const clipboard = useClipboard();
    const [selection, setSelection] = useState<Selection>(emptySelection);
    const [totalCount, setTotalCount] = useState<number>();
    const [progress, setProgress] = useState<SelectionProgress>();

    /// フォルダと一覧の表示を変えたら、選択と読み込み済みの一覧を捨てる。画面の外枠とツリーは残し、展開の状態とフォーカスを保つ。
    // 位置を移すたびに effect で状態を直すと再描画が重なるので、描画の中で前の位置と比べて直す。
    const viewKey = JSON.stringify([location.libraryId, location.path, set.view]);
    const [seenView, setSeenView] = useState(viewKey);
    if (viewKey !== seenView) {
        setSeenView(viewKey);
        setSelection(emptySelection);
        setEditing(undefined);
        setDeletion(undefined);
        setTotalCount(undefined);
        setProgress(undefined);
    }

    const sortKey = `${sort.key}:${sort.order}`;
    const [seenSort, setSeenSort] = useState(sortKey);
    if (sortKey !== seenSort) {
        setSeenSort(sortKey);
        setSelection(viewKey !== seenView ? emptySelection : { ...selection, anchor: undefined });
    }

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
        if (canDelete) setDeletion({ libraryId, items: new Map(selection.items) });
    }, [canDelete, libraryId, selection.items]);
    const cut = useCallback(() => {
        if (canDelete) clipboard.store?.write("cut", libraryId, selection.items);
    }, [canDelete, clipboard.store, libraryId, selection.items]);
    const copy = useCallback(
        () => clipboard.store?.write("copy", libraryId, selection.items),
        [clipboard.store, libraryId, selection.items],
    );
    useEffect(() => {
        const keyDown = (event: globalThis.KeyboardEvent) => {
            if (
                narrow ||
                transfer?.busy ||
                isTypingTarget(event.target) ||
                document.querySelector('[role="dialog"], [role="alertdialog"]')
            )
                return;
            // `/` は、このフォルダを起点に、条件のない検索モードを開く。
            if (event.key === "/" && !event.altKey && !event.ctrlKey && !event.metaKey) {
                event.preventDefault();
                void navigate({ href: searchHref({ libraryIds: [libraryId], path: location.path, tags: [] }) });
                return;
            }
            const command = clipboardCommand(event);
            if (command === "paste" && canCreate && transfer?.canPaste) {
                event.preventDefault();
                void transfer.paste({ libraryId, path: location.path });
                return;
            }
            if (
                (command === "cut" || command === "copy") &&
                selection.items.size > 0 &&
                (command === "copy" || canDelete)
            ) {
                event.preventDefault();
                if (command === "cut") cut();
                else copy();
                return;
            }
            if (
                (event.key === "Delete" || ((event.metaKey || event.ctrlKey) && event.key === "Backspace")) &&
                canDelete
            ) {
                event.preventDefault();
                requestDelete();
                return;
            }
            if (event.key === "F2" && canRename) {
                event.preventDefault();
                rename();
                return;
            }
            if (
                event.altKey &&
                event.shiftKey &&
                !event.metaKey &&
                !event.ctrlKey &&
                event.code === "KeyN" &&
                canCreate
            ) {
                event.preventDefault();
                create();
            }
        };
        window.addEventListener("keydown", keyDown);
        return () => window.removeEventListener("keydown", keyDown);
    }, [
        narrow,
        canCreate,
        create,
        canRename,
        rename,
        canDelete,
        requestDelete,
        selection.items.size,
        cut,
        copy,
        transfer,
        libraryId,
        location.path,
        navigate,
    ]);

    const selectedFile = selection.items.size === 1 ? [...selection.items][0] : undefined;
    const downloadHref =
        selectedFile && !selectedFile[1].folder ? fileContentUrl(libraryId, selectedFile[0]) : undefined;
    const selectedBytes = selectionBytes(selection);

    // フォルダと archive はその中へ移り、画像と動画はビューアで開く。
    // ビューアを開くときは履歴に積み、同じフォルダの中で移ったり閉じたりするときは積まない。
    const open = (entry: Entry) => {
        if (isFolderLike(entry)) {
            void navigate({ href: browseHref({ libraryId: location.libraryId, path: entry.path }) });
        } else if (entryKind(entry) === "image" || entryKind(entry) === "video") {
            void navigate({ href: browseHref({ ...location, view: relativePath(location.path, entry.path) }) });
        }
    };
    const changeView = (name: string | undefined) =>
        void navigate({
            href: browseHref({ ...location, view: name }),
            replace: true,
        });
    // タイムラインの「フォルダで表示」で開いたときは、その Entry を選んで見せた後に、URL から名前を外す。
    const revealed = () => void navigate({ href: browseHref({ ...location, reveal: undefined }), replace: true });

    return (
        <EntryActionContext.Provider
            value={{
                libraryId,
                folderPath: location.path,
                libraryReadonly,
                narrow,
                canCreate,
                selection,
                setSelection,
                editing,
                create,
                cut,
                copy,
                rename,
                downloadHref,
                delete: requestDelete,
                finish,
            }}
        >
            <AppShell
                location={location}
                actions={
                    <>
                        <EntryToolbar />
                        <UploadButton />
                        {!narrow && <InfoPanelToggle open={infoPanel.open} onToggle={infoPanel.toggle} />}
                        <SettingsMenu />
                    </>
                }
                panel={infoPanel.open ? <InfoPanel libraryId={libraryId} selection={selection} /> : undefined}
                footer={
                    <StatusBar
                        totalCount={totalCount}
                        selectedCount={selection.items.size}
                        selectedBytes={selectedBytes}
                        progress={progress}
                    />
                }
            >
                {libraries.data === undefined ? (
                    libraries.isError ? (
                        <LoadFailure
                            error={libraries.error}
                            label="Library を読めませんでした。"
                            onRetry={() => {
                                void libraries.refetch();
                            }}
                        />
                    ) : (
                        <p className="p-4 text-muted-foreground">Library を読み込み中…</p>
                    )
                ) : set.view === "list" ? (
                    <List
                        key={`${viewKey}:${sortKey}`}
                        libraryReadonly={libraryReadonly}
                        source={{ kind: "folder", libraryId, path: location.path }}
                        shape={set.thumbnailShape}
                        narrow={narrow}
                        sort={sort}
                        onSort={updateSort}
                        selection={selection}
                        onSelectionChange={setSelection}
                        onTotal={setTotalCount}
                        onProgress={setProgress}
                        onOpen={open}
                        reveal={location.reveal}
                        onRevealed={revealed}
                    />
                ) : (
                    <Grid
                        key={`${viewKey}:${sortKey}`}
                        libraryReadonly={libraryReadonly}
                        source={{ kind: "folder", libraryId, path: location.path }}
                        tileSize={set.tileSize}
                        shape={set.thumbnailShape}
                        narrow={narrow}
                        sort={sort}
                        selection={selection}
                        onSelectionChange={setSelection}
                        onTotal={setTotalCount}
                        onProgress={setProgress}
                        onOpen={open}
                        reveal={location.reveal}
                        onRevealed={revealed}
                    />
                )}
            </AppShell>
            <EntryNameDialog />
            <DeleteDialog
                target={narrow ? undefined : deletion}
                onClose={() => setDeletion(undefined)}
                onDeleted={() => setSelection(emptySelection)}
            />
            {location.view !== undefined && (
                <Viewer
                    location={location}
                    source={{ kind: "folder", libraryId, path: location.path }}
                    sort={sort}
                    originalImage={set.originalImage}
                    narrow={narrow}
                    libraryReadonly={libraryReadonly}
                    onChange={changeView}
                />
            )}
        </EntryActionContext.Provider>
    );
}
