import { useQuery } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { useCallback, useEffect, useMemo, useState } from "react";

import { AppShell } from "../components/AppShell";
import { DeleteDialog, type DeleteTarget } from "../components/DeleteDialog";
import { EntryToolbar } from "../components/EntryMenu";
import { Grid } from "../components/Grid";
import { InfoPanel, InfoPanelToggle } from "../components/InfoPanel";
import { List } from "../components/List";
import { LoadFailure } from "../components/LoadFailure";
import { EntryNameDialog } from "../components/NameDialog";
import { SearchPanel, SearchTermsField } from "../components/SearchPanel";
import { SettingsDialog } from "../components/SettingsDialog";
import { StatusBar } from "../components/StatusBar";
import { ViewControls } from "../components/ViewControls";
import { Viewer } from "../components/Viewer";
import { EntryActionContext, type EntryEditing } from "../library/actionContext";
import { useClipboard } from "../library/clipboard";
import { type Entry, entryKind, entryPlace, isFolderLike } from "../library/entries";
import { useInfoPanel } from "../library/infoPanel";
import { hasSearchConditions, type ListingSource } from "../library/listing";
import { usePreferences } from "../library/preferences";
import { librariesQuery } from "../library/queries";
import { activeConditionCount, searchCriteria, searchLibraryIds } from "../library/searchCriteria";
import { rememberSearch } from "../library/searchReturn";
import { emptySelection, type Selection, selectionBytes } from "../library/selection";
import { clipboardCommand, isTypingTarget } from "../library/shortcuts";
import type { SelectionProgress } from "../library/useEntryInteraction";
import {
    browseHref,
    CROSS_LIBRARY_SCOPE,
    entryAddress,
    parentPath,
    relativePath,
    searchHref,
    searchLocation,
    viewKey,
} from "../location";
import { fileContentUrl } from "../viewer/urls";

/// 検索モード。Library を横断するので、タイムラインと同じく、Entry は Library を含む鍵で区別する。
export function SearchPage() {
    const query = useSearch({ from: "/authed/search" });
    const search = useMemo(() => searchLocation(query), [query]);
    const navigate = useNavigate();
    const libraries = useQuery(librariesQuery);
    const { set, narrow, sort, updateSort } = usePreferences();
    const infoPanel = useInfoPanel();
    const clipboard = useClipboard();

    const libraryList = libraries.data ?? [];
    const readonlyLibraries = new Set(libraryList.filter((library) => library.readonly).map((library) => library.id));
    const isReadonlyLibrary = (libraryId: string) => !libraries.data || readonlyLibraries.has(libraryId);
    const libraryIds = searchLibraryIds(search, libraryList);
    const criteria = searchCriteria(search, libraryIds);
    const source: ListingSource = { kind: "search", criteria };
    const searching = hasSearchConditions(criteria);
    const conditions = activeConditionCount(search, libraryIds, libraryList.length);

    // 最後に使った条件を、切り替えの「検索」で開けるよう覚える。
    const remembered = JSON.stringify({ ...search, view: undefined });
    // biome-ignore lint/correctness/useExhaustiveDependencies: 内容が同じ間は覚え直さない
    useEffect(() => rememberSearch(search), [remembered]);

    const [selection, setSelection] = useState<Selection>(emptySelection);
    const [editing, setEditing] = useState<EntryEditing>();
    const [deletion, setDeletion] = useState<DeleteTarget>();
    const [totalCount, setTotalCount] = useState<number>();
    const [progress, setProgress] = useState<SelectionProgress>();
    // 条件か一覧の表示を変えたら、選択と読み込み済みの一覧を捨てる。位置を移すたびに effect で状態を直すと再描画が重なるので、描画の中で直す。
    const viewState = JSON.stringify([criteria, set.view]);
    const [seenView, setSeenView] = useState(viewState);
    if (viewState !== seenView) {
        setSeenView(viewState);
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
        setSelection(viewState !== seenView ? emptySelection : { ...selection, anchor: undefined });
    }
    if (narrow && (editing || deletion)) {
        setEditing(undefined);
        setDeletion(undefined);
    }
    const finish = useCallback(() => setEditing(undefined), []);

    // 選んだ Entry のどれかが読み取り専用の Library にあれば、変更の操作を出さない。選択が複数の Library にまたがるときは、操作を Library ごとの要求に分ける。
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

    // 選んだ 1 件の親フォルダを、フォルダ表示で開き、その Entry を選んで見せる。
    const reveal = useCallback(() => {
        const only = selection.items.size === 1 ? [...selection.items.keys()][0] : undefined;
        if (only === undefined) return;
        const { libraryId, path } = entryAddress(CROSS_LIBRARY_SCOPE, only);
        void navigate({ href: browseHref({ libraryId, path: parentPath(path), reveal: path.split("/").at(-1) }) });
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

    // フォルダと archive はフォルダ表示でその中へ移り、検索は終わる。画像と動画はビューアで開く。ビューアを開くときは履歴に積み、前後へ移ったり閉じたりするときは積まない。
    const open = (entry: Entry) => {
        if (isFolderLike(entry)) {
            const { libraryId, path } = entryAddress(CROSS_LIBRARY_SCOPE, entry.path);
            void navigate({ href: browseHref({ libraryId, path }) });
        } else if (entryKind(entry) === "image" || entryKind(entry) === "video") {
            void navigate({ href: searchHref({ ...search, view: relativePath("/", entry.path) }) });
        }
    };
    const changeView = (name: string | undefined) =>
        void navigate({ href: searchHref({ ...search, view: name }), replace: true });

    // 親フォルダの表示。Library が 1 つなら起点からの相対パス、複数なら Library の名前に Library 相対パスを続ける。
    const place = (() => {
        if (!criteria.recursive && libraryIds.length === 1) return undefined;
        const names = new Map(libraryList.map((library) => [library.id, library.name]));
        return (entry: Entry) => {
            const address = entryAddress(CROSS_LIBRARY_SCOPE, entry.path);
            if (libraryIds.length === 1) return entryPlace(criteria.path, { ...entry, path: address.path });
            const parent = parentPath(address.path);
            return `${names.get(address.libraryId) ?? address.libraryId}${parent === "/" ? "" : parent}`;
        };
    })();

    const viewing = search.view;
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
                search={{ field: <SearchTermsField search={search} />, conditions }}
                sidebar={() => <SearchPanel search={search} terms={!narrow} />}
                actions={
                    <>
                        <EntryToolbar />
                        <ViewControls />
                        {!narrow && <InfoPanelToggle open={infoPanel.open} onToggle={infoPanel.toggle} />}
                        <SettingsDialog />
                    </>
                }
                panel={infoPanel.open ? <InfoPanel libraryId={CROSS_LIBRARY_SCOPE} selection={selection} /> : undefined}
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
                ) : !searching ? (
                    <p className="p-4 text-muted-foreground">検索語を入力するか、評価かタグを選んでください。</p>
                ) : set.view === "list" ? (
                    <List
                        key={`${viewState}:${sortKey}`}
                        source={source}
                        place={place}
                        shape={set.thumbnailShape}
                        narrow={narrow}
                        sort={sort}
                        onSort={updateSort}
                        selection={selection}
                        onSelectionChange={setSelection}
                        onTotal={setTotalCount}
                        onProgress={setProgress}
                        onOpen={open}
                    />
                ) : (
                    <Grid
                        key={`${viewState}:${sortKey}`}
                        source={source}
                        place={place}
                        tileSize={set.tileSize}
                        shape={set.thumbnailShape}
                        narrow={narrow}
                        sort={sort}
                        selection={selection}
                        onSelectionChange={setSelection}
                        onTotal={setTotalCount}
                        onProgress={setProgress}
                        onOpen={open}
                    />
                )}
            </AppShell>
            <EntryNameDialog />
            <DeleteDialog
                target={narrow ? undefined : deletion}
                onClose={() => setDeletion(undefined)}
                onDeleted={() => setSelection(emptySelection)}
            />
            {viewing !== undefined && searching && (
                <Viewer
                    location={{ libraryId: CROSS_LIBRARY_SCOPE, path: "/", view: viewing }}
                    source={source}
                    sort={sort}
                    originalImage={set.originalImage}
                    narrow={narrow}
                    libraryReadonly={isReadonlyLibrary(entryAddress(CROSS_LIBRARY_SCOPE, viewKey(viewing)).libraryId)}
                    onChange={changeView}
                />
            )}
        </EntryActionContext.Provider>
    );
}
