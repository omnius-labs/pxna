import type { SearchLocation } from "../location";

/// 切り替えの「検索」で戻る先。最後に使った検索の条件を、タブを開いている間だけ覚える。
let lastSearch: SearchLocation | undefined;

/// 検索モードで使った条件を覚える。ビューアで開いている Entry は覚えない。
export function rememberSearch(location: SearchLocation) {
    const { view: _view, ...conditions } = location;
    lastSearch = conditions;
}

/// 最後に使った検索の条件。まだ検索モードを開いていなければ `undefined`。
export function lastSearchLocation(): SearchLocation | undefined {
    return lastSearch;
}
