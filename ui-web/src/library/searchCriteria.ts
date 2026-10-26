import type { SearchLocation } from "../location";
import type { SearchCriteria } from "./listing";

/// 実際に探す Library の id。URL に Library がなければ、今あるすべての Library である。
/// URL の Library のうち今もあるものを、Library の一覧の順に並べる。1 つも残らなければ、すべての Library を探す。
export function searchLibraryIds(search: SearchLocation, libraries: readonly { id: string }[]): string[] {
    const all = libraries.map((library) => library.id);
    const chosen = search.libraryIds === undefined ? undefined : new Set(search.libraryIds);
    const ids = all.filter((id) => chosen?.has(id) ?? true);
    return ids.length > 0 ? ids : all;
}

/// 位置と、実際に探す Library から、検索の条件を作る。起点は Library が 1 つのときだけ使う。
export function searchCriteria(search: SearchLocation, libraryIds: readonly string[]): SearchCriteria {
    return {
        libraryIds,
        path: libraryIds.length === 1 ? search.path : "/",
        recursive: !search.flat,
        query: search.query ?? "",
        rating: search.rating,
        tags: search.tags,
    };
}

/// 検索語のほかに、絞り込んでいる条件の数。狭い画面のドロワーのボタンに出す。
export function activeConditionCount(search: SearchLocation, libraryIds: readonly string[], allCount: number): number {
    return (
        (libraryIds.length < allCount ? 1 : 0) +
        (libraryIds.length === 1 && search.path !== "/" ? 1 : 0) +
        (search.flat ? 1 : 0) +
        (search.rating === undefined ? 0 : 1) +
        search.tags.length
    );
}
