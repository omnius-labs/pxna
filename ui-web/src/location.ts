/// web が URL で表す位置。`path` は Library 相対パスで、Library のルートは `/` である。
export interface BrowseLocation {
    libraryId: string;
    path: string;
    view?: string;
    /// 検索モードが一覧を要求するときの検索語と範囲。フォルダ表示の URL には載らない。
    query?: string;
    recursive?: boolean;
    /// 検索モードの評価とタグでの絞り込み。フォルダ表示の URL には載らない。
    rating?: RatingFilter;
    tag?: string;
    /// 開いたフォルダで選んで見せる、直下の Entry の名前。タイムラインの「フォルダで表示」が使う。
    reveal?: string;
}

/// web が URL で表す検索モードの位置。`path` は起点のフォルダの Library 相対パス、`flat` は起点の直下だけを探すか、`view` は起点からの相対パスである。
export interface SearchLocation {
    libraryId: string;
    path: string;
    query?: string;
    flat?: boolean;
    /// 評価での絞り込み。`none` は未評価の画像と動画。
    rating?: RatingFilter;
    /// タグでの絞り込み。
    tag?: string;
    view?: string;
}

/// 評価での絞り込みの値。
export type RatingFilter = "good" | "bad" | "none";

/// クエリの文字列が評価の絞り込みの値ならその値、そうでなければ `undefined`。
export function parseRatingFilter(value: unknown): RatingFilter | undefined {
    return value === "good" || value === "bad" || value === "none" ? value : undefined;
}

/// web が URL で表すタイムラインの位置。`month` は見ている月（`YYYY-MM`）、`view` はビューアで開いている Entry の Library 相対パス（先頭の `/` を除く）である。
export interface TimelineLocation {
    libraryId: string;
    month?: string;
    view?: string;
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

/// 検索モードの URL。起点が Library 全体で、語も範囲もビューアもなければ、クエリは載らない。
export function searchHref(location: SearchLocation): string {
    const params = new URLSearchParams();
    const query = location.query?.trim();
    if (query) params.set("q", query);
    if (location.rating !== undefined) params.set("rating", location.rating);
    const tag = location.tag?.trim();
    if (tag) params.set("tag", tag);
    if (pathSegments(location.path).length > 0) params.set("path", joinPath(pathSegments(location.path)));
    if (location.flat) params.set("flat", "1");
    if (location.view !== undefined) params.set("view", location.view);
    const search = params.size === 0 ? "" : `?${params.toString()}`;
    return `/search/${encodeURIComponent(location.libraryId)}${search}`;
}

/// ルータが復号したクエリから、検索モードの位置を作る。
export function searchLocation(
    libraryId: string,
    search: { q?: string; path?: string; flat?: string; rating?: string; tag?: string; view?: string },
): SearchLocation {
    const location: SearchLocation = { libraryId, path: joinPath(pathSegments(search.path ?? "")) };
    const query = search.q?.trim();
    if (query) location.query = query;
    const rating = parseRatingFilter(search.rating);
    if (rating !== undefined) location.rating = rating;
    const tag = search.tag?.trim();
    if (tag) location.tag = tag;
    if (search.flat === "1") location.flat = true;
    if (search.view !== undefined) location.view = search.view;
    return location;
}

/// 検索モードの一覧とビューアが使う位置。起点を今のフォルダとし、範囲で再帰かを決める。
export function searchBrowseLocation(search: SearchLocation): BrowseLocation {
    const location: BrowseLocation = { libraryId: search.libraryId, path: search.path };
    if (search.query) location.query = search.query;
    if (search.rating !== undefined) location.rating = search.rating;
    if (search.tag) location.tag = search.tag;
    // 名前の語がなくても、評価かタグで絞るなら、起点の配下（または直下）を探す。
    if (search.query || search.rating !== undefined || search.tag) location.recursive = !search.flat;
    if (search.view !== undefined) location.view = search.view;
    return location;
}

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
    return `/timeline/${encodeURIComponent(location.libraryId)}${search}`;
}

/// ルータが復号したパスとクエリから、タイムラインの位置を作る。月の形が違えば月を持たないものとして扱う。
export function timelineLocation(libraryId: string, search: { month?: string; view?: string }): TimelineLocation {
    const location: TimelineLocation = { libraryId };
    if (search.month !== undefined && isMonth(search.month)) location.month = search.month;
    if (search.view !== undefined) location.view = search.view;
    return location;
}

/// `YYYY-MM` の形で、月が 01 から 12 か。
export function isMonth(value: string): boolean {
    return /^\d{4}-(0[1-9]|1[0-2])$/.test(value);
}

/// タイムラインで開いた Entry の、Library 相対パス（先頭の `/` を除く）から、Library 相対パスへ戻す。
export function timelineEntryPath(view: string): string {
    return joinPath(pathSegments(view));
}

/// Library 相対パスの親フォルダ。Library のルート直下の Entry の親は `/` である。
export function parentPath(path: string): string {
    return joinPath(pathSegments(path).slice(0, -1));
}
