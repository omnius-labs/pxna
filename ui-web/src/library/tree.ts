import { joinPath, pathSegments } from "../location";
import { type Entry, type EntryKind, entryKind } from "./entries";

/// ツリーに並ぶ 1 つのノード。Library のルートは `path` が `/` のノードである。
export interface TreeNode {
    key: string;
    libraryId: string;
    path: string;
    name: string;
    readonly: boolean;
    kind: EntryKind;
    level: number;
    parentKey: string | undefined;
    expandable: boolean;
    expanded: boolean;
    /// 未取得ページの行も高さと位置を持ち、表示時にそのページだけを読む。
    placeholder?: boolean;
    childIndex?: number;
}

export interface TreeLibrary {
    id: string;
    name: string;
    readonly?: boolean;
}

export function treeKey(libraryId: string, path: string): string {
    return `${libraryId}:${path}`;
}

/// 位置を表示するために展開しておくノード。Library のルートから、位置のフォルダ自身までを含む。
export function ancestorKeys(libraryId: string, path: string): string[] {
    const segments = pathSegments(path);
    return Array.from({ length: segments.length + 1 }, (_, depth) =>
        treeKey(libraryId, joinPath(segments.slice(0, depth))),
    );
}

/// フォルダの `hasChildren` が false のときだけ展開できないとする。archive は中を読むまで分からないので展開できるとする。
function isExpandable(entry: Entry): boolean {
    return entry.kind !== "folder" || entry.folder?.hasChildren !== false;
}

/// 展開しているノードの子を、表示する順に平らに並べる。子を読み終えていないノードは、子を持たないものとして並べる。
export function flattenTree(
    libraries: readonly TreeLibrary[],
    expanded: ReadonlySet<string>,
    children: ReadonlyMap<
        string,
        readonly Entry[] | { totalCount: number | undefined; entryAt(index: number): Entry | undefined }
    >,
    probes: ReadonlyMap<string, boolean> = new Map(),
): TreeNode[] {
    const nodes: TreeNode[] = [];
    const visit = (node: TreeNode, libraryReadonly: boolean) => {
        nodes.push(node);
        if (!node.expanded) {
            return;
        }
        const listing = children.get(node.key);
        const count = !listing ? 0 : "entryAt" in listing ? (listing.totalCount ?? 0) : listing.length;
        for (let childIndex = 0; childIndex < count; childIndex++) {
            const entry: Entry | undefined = !listing
                ? undefined
                : "entryAt" in listing
                  ? listing.entryAt(childIndex)
                  : listing[childIndex];
            if (!entry) {
                nodes.push({
                    ...node,
                    key: `${node.key}\0${childIndex}`,
                    parentKey: node.key,
                    level: node.level + 1,
                    name: "",
                    expanded: false,
                    expandable: false,
                    placeholder: true,
                    childIndex,
                });
                continue;
            }
            const key = treeKey(node.libraryId, entry.path);
            visit(
                {
                    key,
                    libraryId: node.libraryId,
                    path: entry.path,
                    name: entry.name,
                    readonly: libraryReadonly || entry.metadata?.readonly !== false,
                    kind: entryKind(entry),
                    level: node.level + 1,
                    parentKey: node.key,
                    childIndex,
                    expandable: probes.get(key) ?? isExpandable(entry),
                    expanded: expanded.has(key),
                },
                libraryReadonly,
            );
        }
    };
    for (const library of libraries) {
        const key = treeKey(library.id, "/");
        visit(
            {
                key,
                libraryId: library.id,
                path: "/",
                name: library.name,
                readonly: library.readonly ?? false,
                kind: "folder",
                level: 1,
                parentKey: undefined,
                expandable: true,
                expanded: expanded.has(key),
            },
            library.readonly ?? false,
        );
    }
    return nodes;
}

/// ツリーのキー操作の結果。WAI-ARIA の tree の型に合わせる。
export type TreeAction =
    | { type: "focus"; key: string }
    | { type: "expand"; key: string }
    | { type: "collapse"; key: string }
    | { type: "open"; key: string }
    | { type: "none" };

export function treeKeyAction(nodes: readonly TreeNode[], focusedKey: string, keyName: string): TreeAction {
    const index = nodes.findIndex((node) => node.key === focusedKey);
    const node = nodes[index];
    if (node === undefined) {
        return { type: "none" };
    }
    const focusAt = (target: number): TreeAction => {
        const next = nodes[target];
        return next === undefined ? { type: "none" } : { type: "focus", key: next.key };
    };
    switch (keyName) {
        case "ArrowDown":
            return focusAt(index + 1);
        case "ArrowUp":
            return focusAt(index - 1);
        case "Home":
            return focusAt(0);
        case "End":
            return focusAt(nodes.length - 1);
        case "ArrowRight":
            if (node.expandable && !node.expanded) {
                return { type: "expand", key: node.key };
            }
            // 展開済みなら最初の子へ移る。子がまだなければ何もしない。
            return nodes[index + 1]?.parentKey === node.key ? focusAt(index + 1) : { type: "none" };
        case "ArrowLeft":
            if (node.expanded) {
                return { type: "collapse", key: node.key };
            }
            return node.parentKey === undefined ? { type: "none" } : { type: "focus", key: node.parentKey };
        case "Enter":
            return { type: "open", key: node.key };
        default:
            return { type: "none" };
    }
}
