import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { defaultRangeExtractor, useVirtualizer } from "@tanstack/react-virtual";
import { type KeyboardEvent, type ReactNode, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { useEntryActions } from "../library/actionContext";
import { useEntryDrop } from "../library/dnd";
import { PAGE_SIZE } from "../library/listing";
import { useNarrowScreen } from "../library/preferences";
import { librariesQuery } from "../library/queries";
import { emptySelection } from "../library/selection";
import { ancestorKeys, flattenTree, type TreeNode, treeKey, treeKeyAction } from "../library/tree";
import { useChildProbes } from "../library/useChildProbes";
import { useTreeListing } from "../library/useTreeListing";
import { type BrowseLocation, browseHref } from "../location";
import { DeleteDialog, type DeleteTarget } from "./DeleteDialog";
import { EntryIcon } from "./EntryIcon";
import { NameDialog, type NameTarget } from "./NameDialog";
import { TreeMenu } from "./TreeMenu";

interface TreeProps {
    /// 強調して出す位置。
    location?: BrowseLocation;
    /// 並べる Library。省略すると、すべての Library を並べる。
    libraryIds?: readonly string[];
    /// フォルダを選んだとき。あれば、フォルダ表示へ移らずに呼び、メニューとドロップを出さない。検索の場所を選ぶときに使う。
    onSelect?: (location: BrowseLocation) => void;
}

/// Library の階層を表すツリー。フォルダ扱いの Entry だけを並べ、展開したときに子を取得する。
export function Tree({ location, libraryIds, onSelect }: TreeProps) {
    const allLibraries = useQuery(librariesQuery);
    const libraryKey = libraryIds?.join("\n");
    // biome-ignore lint/correctness/useExhaustiveDependencies: 並べる Library は libraryKey に畳んで比べる
    const shown = useMemo(
        () => allLibraries.data?.filter((library) => libraryIds?.includes(library.id) ?? true),
        [allLibraries.data, libraryKey],
    );
    const libraries = { data: shown };
    const picking = onSelect !== undefined;
    const actions = useEntryActions();
    const narrow = useNarrowScreen();
    const [editing, setEditing] = useState<{ type: "create" | "rename"; node: TreeNode }>();
    const [deletion, setDeletion] = useState<DeleteTarget>();
    if (narrow && (editing || deletion)) {
        setEditing(undefined);
        setDeletion(undefined);
    }
    const nameTarget: NameTarget | undefined = editing && {
        ...(editing.type === "create"
            ? { type: "create" as const, parent: editing.node.path }
            : { type: "rename" as const, path: editing.node.path, folder: editing.node.kind === "folder" }),
        libraryId: editing.node.libraryId,
    };
    const navigate = useNavigate();
    const [expanded, setExpanded] = useState<ReadonlySet<string>>(new Set());
    const currentKey = location === undefined ? undefined : treeKey(location.libraryId, location.path);

    // 位置が変わったら、その位置が見えるよう、Library のルートからその位置までを展開する。
    // 位置を移すたびに effect で状態を直すと再描画が重なるので、描画の中で前の位置と比べて直す。
    const [seenKey, setSeenKey] = useState<string | undefined>(undefined);
    if (currentKey !== seenKey) {
        setSeenKey(currentKey);
        if (location !== undefined) {
            setExpanded((current) => new Set([...current, ...ancestorKeys(location.libraryId, location.path)]));
        }
    }

    const [wanted, setWanted] = useState("[]");
    const { listings, revision } = useTreeListing(expanded, wanted);
    const [probeWanted, setProbeWanted] = useState("[]");
    const probes = useChildProbes(probeWanted);
    // biome-ignore lint/correctness/useExhaustiveDependencies: store は同じ参照のままページを更新するため revision で再計算する
    const nodes = useMemo(
        () => flattenTree(libraries.data ?? [], expanded, listings, probes),
        [libraries.data, expanded, listings, revision, probes],
    );
    const [scrollElement, setScrollElement] = useState<HTMLDivElement | null>(null);
    const [pendingFocus, setPendingFocus] = useState<{
        index: number;
        parentKey?: string;
        childIndex?: number;
        snapshot?: string;
    }>();
    const [focusedKey, setFocusedKey] = useState<string | undefined>(undefined);
    const has = (key: string | undefined) =>
        key !== undefined && nodes.some((node) => !node.placeholder && node.key === key);
    const activeKey = [focusedKey, currentKey].find(has) ?? nodes[0]?.key;
    const itemRefs = useRef(new Map<string, HTMLDivElement>());
    const activeIndex = nodes.findIndex((node) => node.key === activeKey);
    const editingIndex = editing ? nodes.findIndex((node) => node.key === editing.node.key) : -1;
    const virtualizer = useVirtualizer({
        count: nodes.length,
        getScrollElement: () => scrollElement,
        estimateSize: (index) => (editing?.type === "create" && index === editingIndex ? 64 : 32),
        getItemKey: (index) => nodes[index]?.key ?? index,
        initialRect: { width: 224, height: 600 },
        overscan: 4,
        rangeExtractor: (range) =>
            [...new Set([...defaultRangeExtractor(range), activeIndex, editingIndex, pendingFocus?.index ?? -1])]
                .filter((index) => index >= 0 && index < nodes.length)
                .sort((a, b) => a - b),
    });
    const virtualRows = virtualizer.getVirtualItems();
    const visibleKey = virtualRows.map((row) => row.index).join(",");
    const probeVisibleKey = virtualRows
        .filter(
            (row) =>
                row.end > (virtualizer.scrollOffset ?? 0) &&
                row.start < (virtualizer.scrollOffset ?? 0) + (virtualizer.scrollRect?.height ?? 600),
        )
        .map((row) => row.index)
        .join(",");
    useEffect(() => {
        const targets = probeVisibleKey
            .split(",")
            .filter(Boolean)
            .map(Number)
            .flatMap((index) => {
                const node = nodes[index];
                const snapshot = node?.parentKey ? listings.get(node.parentKey)?.snapshot : undefined;
                return node && !node.placeholder && snapshot
                    ? [{ key: node.key, libraryId: node.libraryId, path: node.path, snapshot }]
                    : [];
            });
        const next = JSON.stringify(targets);
        setProbeWanted((current) => (current === next ? current : next));
    }, [probeVisibleKey, nodes, listings]);

    // biome-ignore lint/correctness/useExhaustiveDependencies: 編集行の高さが変わったら推定値を再計算する
    useEffect(() => {
        virtualizer.measure();
    }, [virtualizer, editingIndex, editing?.type]);

    useEffect(() => {
        const pages = new Map<string, Set<number>>();
        const want = (key: string, index: number) => {
            let set = pages.get(key);
            if (!set) {
                set = new Set();
                pages.set(key, set);
            }
            set.add(Math.floor(index / PAGE_SIZE));
        };
        for (const index of visibleKey.split(",").filter(Boolean).map(Number)) {
            const node = nodes[index];
            if (node?.parentKey !== undefined && node.childIndex !== undefined) want(node.parentKey, node.childIndex);
        }
        // API に path から並び位置を求める操作はない。明示的な位置移動の祖先だけは、
        // 見つかるまで順に探す。通常の展開で全兄弟を先読みすることはしない。
        if (location) {
            const keys = ancestorKeys(location.libraryId, location.path);
            for (const [index, key] of keys.entries()) {
                const childKey = keys[index + 1];
                if (!childKey) continue;
                const listing = listings.get(key);
                const childPath = childKey.slice(location.libraryId.length + 1);
                if (!listing || listing.locate(childPath) !== undefined) continue;
                const count = Math.ceil((listing.totalCount ?? 0) / PAGE_SIZE);
                for (let page = 0; page < count; page++) {
                    if (listing.pageStatus(page) !== "loaded") {
                        want(key, page * PAGE_SIZE);
                        break;
                    }
                }
            }
        }
        const next = JSON.stringify([...pages].map(([key, set]) => [key, [...set].sort((a, b) => a - b)]));
        setWanted((current) => (current === next ? current : next));
    }, [visibleKey, nodes, listings, location]);

    const revealed = useRef<string | undefined>(undefined);
    useLayoutEffect(() => {
        if (currentKey === undefined || revealed.current === currentKey) return;
        const index = nodes.findIndex((node) => !node.placeholder && node.key === currentKey);
        if (index >= 0) {
            virtualizer.scrollToIndex(index, { align: "auto" });
            revealed.current = currentKey;
        }
    }, [currentKey, nodes, virtualizer]);
    // biome-ignore lint/correctness/useExhaustiveDependencies: 可視行の DOM 生成後にも保留したフォーカスを再試行する
    useLayoutEffect(() => {
        if (!pendingFocus) return;
        const node = nodes[pendingFocus.index];
        if (!node || node.parentKey !== pendingFocus.parentKey || node.childIndex !== pendingFocus.childIndex) {
            setPendingFocus(undefined);
            return;
        }
        const listing = node.parentKey ? listings.get(node.parentKey) : undefined;
        if (pendingFocus.snapshot && pendingFocus.snapshot !== listing?.snapshot) {
            setPendingFocus(undefined);
            return;
        }
        if (node.placeholder) return;
        const element = itemRefs.current.get(node.key);
        if (element) {
            setFocusedKey(node.key);
            element.focus();
            setPendingFocus(undefined);
        }
    }, [pendingFocus, nodes, listings, visibleKey]);

    const open = (node: TreeNode) =>
        onSelect
            ? onSelect({ libraryId: node.libraryId, path: node.path })
            : void navigate({ href: browseHref({ libraryId: node.libraryId, path: node.path }) });
    const toggle = (key: string, expand: boolean) =>
        setExpanded((current) => {
            const next = new Set(current);
            if (expand) {
                next.add(key);
            } else {
                next.delete(key);
            }
            return next;
        });

    const keyDown = (event: KeyboardEvent<HTMLDivElement>) => {
        if (event.target instanceof HTMLElement && event.target.closest("input, button, [contenteditable=true]"))
            return;
        if (activeKey === undefined) {
            return;
        }
        const action = treeKeyAction(nodes, activeKey, event.key);
        if (action.type === "none") {
            return;
        }
        event.preventDefault();
        switch (action.type) {
            case "focus": {
                const index = nodes.findIndex((node) => node.key === action.key);
                const node = nodes[index];
                if (node) {
                    virtualizer.scrollToIndex(index, { align: "auto" });
                    setPendingFocus({
                        index,
                        parentKey: node.parentKey,
                        childIndex: node.childIndex,
                        snapshot: node.parentKey ? listings.get(node.parentKey)?.snapshot : undefined,
                    });
                }
                break;
            }
            case "expand":
                toggle(action.key, true);
                break;
            case "collapse":
                toggle(action.key, false);
                break;
            case "open": {
                const node = nodes.find((candidate) => candidate.key === action.key);
                if (node && !node.placeholder) {
                    open(node);
                }
                break;
            }
        }
    };

    const loadingKeys = new Set(
        [...listings].filter(([, store]) => store.pageStatus(0) === "loading").map(([key]) => key),
    );
    const failedKeys = new Set(
        [...listings].filter(([, store]) => store.pageStatus(0) === "error").map(([key]) => key),
    );

    return (
        <>
            <div ref={setScrollElement} className="h-full overflow-auto">
                <div
                    role="tree"
                    aria-label="フォルダ"
                    className="relative"
                    style={{ height: virtualizer.getTotalSize() }}
                    onKeyDown={keyDown}
                >
                    {virtualRows.map((row) => {
                        const node = nodes[row.index];
                        if (!node) return null;
                        const parent = node.parentKey ? listings.get(node.parentKey) : undefined;
                        const page = Math.floor((node.childIndex ?? 0) / PAGE_SIZE);
                        return (
                            <div
                                key={node.key}
                                role="presentation"
                                className="absolute left-0 top-0 w-full"
                                style={{ transform: `translateY(${row.start}px)`, height: row.size }}
                            >
                                {node.placeholder ? (
                                    <div
                                        role="treeitem"
                                        aria-level={node.level}
                                        aria-label={
                                            parent?.pageStatus(page) === "error" ? "読み込めませんでした" : "読み込み中"
                                        }
                                        aria-busy={parent?.pageStatus(page) !== "error" || undefined}
                                        tabIndex={-1}
                                        className="flex h-8 items-center px-2 text-muted-foreground"
                                    >
                                        {parent?.pageStatus(page) === "error" ? (
                                            <button type="button" onClick={() => parent.retry(page)}>
                                                読み直す
                                            </button>
                                        ) : (
                                            <span className="h-3 w-20 animate-pulse rounded bg-muted" />
                                        )}
                                    </div>
                                ) : (
                                    <TreeDropTarget
                                        node={node}
                                        narrow={narrow || picking}
                                        onExpand={() => toggle(node.key, true)}
                                    >
                                        <TreeMenu
                                            node={node}
                                            narrow={narrow || picking}
                                            onCreate={() => {
                                                toggle(node.key, true);
                                                setEditing({ type: "create", node });
                                            }}
                                            onRename={() => setEditing({ type: "rename", node })}
                                            onDelete={() =>
                                                setDeletion({
                                                    libraryId: node.libraryId,
                                                    items: new Map([[node.path, { folder: node.kind === "folder" }]]),
                                                })
                                            }
                                        >
                                            {/* biome-ignore lint/a11y/useKeyWithClickEvents: キーボード操作は親の ul の onKeyDown で一括処理する */}
                                            <div
                                                key={node.key}
                                                ref={(element) => {
                                                    if (element) {
                                                        itemRefs.current.set(node.key, element);
                                                    } else {
                                                        itemRefs.current.delete(node.key);
                                                    }
                                                }}
                                                role="treeitem"
                                                aria-level={node.level}
                                                aria-posinset={
                                                    node.childIndex === undefined ? undefined : node.childIndex + 1
                                                }
                                                aria-setsize={parent?.totalCount}
                                                aria-expanded={node.expandable ? node.expanded : undefined}
                                                aria-selected={node.key === currentKey}
                                                aria-busy={loadingKeys.has(node.key) || undefined}
                                                tabIndex={node.key === activeKey ? 0 : -1}
                                                className={`flex h-8 cursor-pointer items-center gap-1 truncate py-0.5 pr-2 outline-none hover:bg-sidebar-accent focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring ${node.key === currentKey ? "bg-sidebar-accent text-sidebar-accent-foreground" : ""}`}
                                                style={{ paddingLeft: `${(node.level - 1) * 0.875 + 0.25}rem` }}
                                                onClick={() => {
                                                    setFocusedKey(node.key);
                                                    open(node);
                                                }}
                                            >
                                                <span
                                                    aria-hidden="true"
                                                    className={`inline-block w-4 shrink-0 text-center text-muted-foreground ${node.expandable ? "" : "invisible"}`}
                                                    onClick={(event) => {
                                                        event.stopPropagation();
                                                        toggle(node.key, !node.expanded);
                                                    }}
                                                >
                                                    {node.expanded ? "▾" : "▸"}
                                                </span>
                                                <EntryIcon
                                                    kind={node.kind}
                                                    className="h-4 w-4 shrink-0 text-muted-foreground"
                                                />
                                                <span className="truncate">{node.name}</span>
                                                {failedKeys.has(node.key) && (
                                                    <span
                                                        role="alert"
                                                        className="ml-1 shrink-0 text-destructive"
                                                        title="中身を読めませんでした"
                                                    >
                                                        !
                                                    </span>
                                                )}
                                            </div>
                                        </TreeMenu>
                                    </TreeDropTarget>
                                )}
                            </div>
                        );
                    })}
                </div>
            </div>
            <NameDialog
                target={narrow ? undefined : nameTarget}
                onClose={() => setEditing(undefined)}
                onDone={() => actions?.setSelection(emptySelection)}
            />
            <DeleteDialog
                target={narrow ? undefined : deletion}
                onClose={() => setDeletion(undefined)}
                onDeleted={() => actions?.setSelection(emptySelection)}
            />
        </>
    );
}

function TreeDropTarget({
    node,
    narrow,
    onExpand,
    children,
}: {
    node: TreeNode;
    narrow: boolean;
    onExpand(): void;
    children: ReactNode;
}) {
    const drop = useEntryDrop(
        node.kind === "folder" ? { libraryId: node.libraryId, path: node.path } : undefined,
        narrow || node.readonly,
        node.kind === "folder" && node.expandable && !node.expanded ? onExpand : undefined,
    );
    return (
        <div
            {...drop}
            role="presentation"
            className="data-[drop-state=allowed]:ring-2 data-[drop-state=allowed]:ring-ring data-[drop-state=blocked]:cursor-not-allowed data-[drop-state=blocked]:ring-2 data-[drop-state=blocked]:ring-destructive"
        >
            {children}
        </div>
    );
}
