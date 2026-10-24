/// web が URL で表す位置。`path` は Library 相対パスで、Library のルートは `/` である。
export interface BrowseLocation {
    libraryId: string;
    path: string;
    view?: string;
    /// 開いたフォルダで選んで見せる、直下の Entry の名前。タイムラインの「フォルダで表示」が使う。
    reveal?: string;
}

/// web が URL で表す検索モードの位置。
export interface SearchLocation {
    /// 探す Library の id。`undefined` はすべての Library で、後から加わった Library も探す。
    libraryIds?: readonly string[];
    /// 起点のフォルダの Library 相対パス。Library を 1 つに絞ったときだけ `/` 以外になる。
    path: string;
    query?: string;
    /// 起点の直下だけを探すか。
    flat?: boolean;
    /// 評価での絞り込み。`none` は未評価の画像と動画。
    rating?: RatingFilter;
    /// すべてを持つ Entry を選ぶタグ。
    tags: readonly string[];
    /// ビューアで開いている Entry の鍵（先頭の `/` を除く）。Library の id を含む。
    view?: string;
}

/// 評価での絞り込みの値。
export type RatingFilter = "good" | "bad" | "none";

/// クエリの文字列が評価の絞り込みの値ならその値、そうでなければ `undefined`。
export function parseRatingFilter(value: unknown): RatingFilter | undefined {
    return value === "good" || value === "bad" || value === "none" ? value : undefined;
}

/// web が URL で表すタイムラインの位置。`month` は見ている月（`YYYY-MM`）、`view` はビューアで開いている Entry の鍵（先頭の `/` を除く）である。
/// タイムラインは Library を横断するので、並べる Library は URL に載せず、表示設定に保存する。
export interface TimelineLocation {
    month?: string;
    view?: string;
}

/// Entry の所在。Library の id と Library 相対パスの組である。
export interface EntryAddress {
    libraryId: string;
    path: string;
}

/// Library を横断する範囲（タイムラインと検索）。Library の id に使えない文字なので、どの Library の id とも重ならない。
/// この範囲の Entry の path は、`scopedKey` が作る鍵である。
export const CROSS_LIBRARY_SCOPE = "*";

/// Library を横断する範囲の Entry の鍵。先頭の `/` の後に Library の id と Library 相対パスを続ける。
/// Library の id は `/` を含まないので、2 つ目の `/` から後が Library 相対パスである。
export function scopedKey(libraryId: string, path: string): string {
    return `/${libraryId}${path}`;
}

/// 範囲（Library の id か `CROSS_LIBRARY_SCOPE`）と、その範囲での path から、Entry の所在を求める。
export function entryAddress(scope: string, path: string): EntryAddress {
    if (scope !== CROSS_LIBRARY_SCOPE) return { libraryId: scope, path };
    const slash = path.indexOf("/", 1);
    return slash < 0
        ? { libraryId: path.slice(1), path: "/" }
        : { libraryId: path.slice(1, slash), path: path.slice(slash) };
}

/// 範囲での path を、Library ごとの Library 相対パスに分ける。タイムラインの選択は、複数の Library にまたがる。
export function groupByLibrary<T>(scope: string, items: ReadonlyMap<string, T>): Map<string, Map<string, T>> {
    const groups = new Map<string, Map<string, T>>();
    for (const [scopedPath, item] of items) {
        const { libraryId, path } = entryAddress(scope, scopedPath);
        let group = groups.get(libraryId);
        if (!group) {
            group = new Map();
            groups.set(libraryId, group);
        }
        group.set(path, item);
    }
    return groups;
}

/// Library 相対パスをセグメントに分ける。空のセグメントは捨てる。
export function pathSegments(path: string): string[] {
    return path.split("/").filter((segment) => segment.length > 0);
}

export function joinPath(segments: readonly string[]): string {
    return `/${segments.join("/")}`;
}

/// 起点のフォルダから Entry への相対パス。直下なら名前と同じになる。
export function relativePath(base: string, entryPath: string): string {
    return pathSegments(entryPath).slice(pathSegments(base).length).join("/");
}

/// セグメントごとに percent-encode し、名前の `#`、`?`、`%` を URL の区切りと取り違えない。
export function browseHref(location: BrowseLocation): string {
    const segments = [location.libraryId, ...pathSegments(location.path)].map(encodeURIComponent);
    const params = new URLSearchParams();
    if (location.view !== undefined) params.set("view", location.view);
    if (location.reveal !== undefined) params.set("reveal", location.reveal);
    const search = params.size === 0 ? "" : `?${params.toString()}`;
    return `/browse/${segments.join("/")}${search}`;
}

/// ルータが復号したパスとクエリから位置を作る。
export function browseLocation(
    libraryId: string,
    splat: string | undefined,
    search: { view?: string; reveal?: string },
): BrowseLocation {
    const location: BrowseLocation = { libraryId, path: joinPath(pathSegments(splat ?? "")) };
    if (search.view !== undefined) location.view = search.view;
    if (search.reveal !== undefined) location.reveal = search.reveal;
    return location;
}

/// 検索モードの URL。すべての Library を探し、条件もビューアもなければ、クエリは載らない。
export function searchHref(location: SearchLocation): string {
    const params = new URLSearchParams();
    for (const libraryId of location.libraryIds ?? []) params.append("lib", libraryId);
    const segments = pathSegments(location.path);
    if (segments.length > 0 && location.libraryIds?.length === 1) params.set("path", joinPath(segments));
    if (location.flat) params.set("flat", "1");
    const query = location.query?.trim();
    if (query) params.set("q", query);
    if (location.rating !== undefined) params.set("rating", location.rating);
    for (const tag of location.tags) if (tag.trim()) params.append("tag", tag.trim());
    if (location.view !== undefined) params.set("view", location.view);
    const search = params.size === 0 ? "" : `?${params.toString()}`;
    return `/search${search}`;
}

/// クエリの値。繰り返したキーは配列になる。
export type SearchParam = string | readonly string[] | undefined;

/// 繰り返したキーも 1 つだけのキーも、値の配列にそろえる。
export function paramList(value: SearchParam): string[] {
    return value === undefined ? [] : typeof value === "string" ? [value] : [...value];
}

/// ルータが復号したクエリから、検索モードの位置を作る。
export function searchLocation(search: {
    lib?: SearchParam;
    path?: SearchParam;
    flat?: SearchParam;
    q?: SearchParam;
    rating?: SearchParam;
    tag?: SearchParam;
    view?: SearchParam;
}): SearchLocation {
    const single = (value: SearchParam) => paramList(value).at(-1);
    const libraryIds = [...new Set(paramList(search.lib).filter((id) => id !== ""))];
    const location: SearchLocation = {
        path: libraryIds.length === 1 ? joinPath(pathSegments(single(search.path) ?? "")) : "/",
        tags: [
            ...new Set(
                paramList(search.tag)
                    .map((tag) => tag.trim())
                    .filter((tag) => tag !== ""),
            ),
        ],
    };
    if (libraryIds.length > 0) location.libraryIds = libraryIds;
    const query = single(search.q)?.trim();
    if (query) location.query = query;
    const rating = parseRatingFilter(single(search.rating));
    if (rating !== undefined) location.rating = rating;
    if (single(search.flat) === "1") location.flat = true;
    const view = single(search.view);
    if (view !== undefined) location.view = view;
    return location;
}

/// 条件のない、すべての Library を探す検索モードの位置。
export const EMPTY_SEARCH: SearchLocation = { path: "/", tags: [] };

/// ログインの後に戻る先として受け付ける URL。別のオリジンへ移す値は受け付けない。
export function safeRedirect(redirect: string | undefined): string {
    if (
        redirect === undefined ||
        !redirect.startsWith("/") ||
        redirect.startsWith("//") ||
        redirect.startsWith("/\\")
    ) {
        return "/";
    }
    return redirect;
}

/// タイムラインの URL。月は `YYYY-MM` の形のときだけ載せる。
export function timelineHref(location: TimelineLocation): string {
    const params = new URLSearchParams();
    if (location.month !== undefined && isMonth(location.month)) params.set("month", location.month);
    if (location.view !== undefined) params.set("view", location.view);
    const search = params.size === 0 ? "" : `?${params.toString()}`;
    return `/timeline${search}`;
}

/// ルータが復号したクエリから、タイムラインの位置を作る。月の形が違えば月を持たないものとして扱う。
export function timelineLocation(search: { month?: string; view?: string }): TimelineLocation {
    const location: TimelineLocation = {};
    if (search.month !== undefined && isMonth(search.month)) location.month = search.month;
    if (search.view !== undefined) location.view = search.view;
    return location;
}

/// `YYYY-MM` の形で、月が 01 から 12 か。
export function isMonth(value: string): boolean {
    return /^\d{4}-(0[1-9]|1[0-2])$/.test(value);
}

/// タイムラインで開いた Entry の鍵（先頭の `/` を除く）から、鍵へ戻す。
export function viewKey(view: string): string {
    return joinPath(pathSegments(view));
}

/// Library 相対パスの親フォルダ。Library のルート直下の Entry の親は `/` である。
export function parentPath(path: string): string {
    return joinPath(pathSegments(path).slice(0, -1));
}
