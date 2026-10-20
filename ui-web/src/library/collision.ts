import type { components } from "../api/schema";
import { fetchFolderPage } from "./entries";

export type ConflictPolicy = components["schemas"]["ConflictPolicy"];
export type CollisionChoice = ConflictPolicy | "cancel";
export interface TransferDestination {
    libraryId: string;
    path: string;
}

async function readNames(destination: TransferDestination, query?: string, signal?: AbortSignal): Promise<Set<string>> {
    // ページ間で宛先が変わったら、別の版を混ぜずに照合をやり直す。
    for (let attempt = 0; attempt < 3; attempt++) {
        const names = new Set<string>();
        let offset = 0;
        let snapshot: string | undefined;
        while (true) {
            const page = await fetchFolderPage(
                destination.libraryId,
                destination.path,
                offset,
                { query, recursive: false, includeHidden: true },
                signal,
            );
            if (snapshot !== undefined && snapshot !== page.snapshot) break;
            snapshot = page.snapshot;
            for (const entry of page.entries) names.add(entry.name);
            offset += page.entries.length;
            if (offset >= page.totalCount) return names;
            if (!page.entries.length) throw new Error("宛先の一覧を最後まで読めませんでした。");
        }
    }
    throw new Error("宛先が更新されています。もう一度お試しください。");
}
export async function findCollisions(
    names: readonly string[],
    destination: TransferDestination,
    signal?: AbortSignal,
): Promise<string[]> {
    const unique = [...new Set(names)];
    const collisions = new Set<string>();
    const seen = new Set<string>();
    for (const name of names) {
        if (seen.has(name)) collisions.add(name);
        seen.add(name);
    }
    if (names.length > 50) {
        const existing = await readNames(destination, undefined, signal);
        return unique.filter((name) => existing.has(name) || collisions.has(name));
    }
    let next = 0;
    await Promise.all(
        Array.from({ length: Math.min(8, unique.length) }, async () => {
            while (next < unique.length) {
                const name = unique[next++]!;
                if ((await readNames(destination, name, signal)).has(name)) collisions.add(name);
            }
        }),
    );
    return unique.filter((name) => collisions.has(name));
}
export async function resolveConflict(
    names: readonly string[],
    destination: TransferDestination,
    confirm: (names: string[]) => Promise<CollisionChoice>,
    signal?: AbortSignal,
): Promise<CollisionChoice> {
    const collisions = await findCollisions(names, destination, signal);
    return collisions.length ? confirm(collisions) : "fail";
}
