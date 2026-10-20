import { useParams, useSearch } from "@tanstack/react-router";

import { searchBrowseLocation, searchLocation } from "../location";
import { BrowseScreen } from "./BrowsePage";

/// 検索モード。起点と範囲を URL に持ち、一覧とビューアはフォルダ表示と同じ画面を使う。
export function SearchPage() {
    const { libraryId } = useParams({ from: "/authed/search/$libraryId" });
    const query = useSearch({ from: "/authed/search/$libraryId" });
    const search = searchLocation(libraryId, query);
    return <BrowseScreen location={searchBrowseLocation(search)} search={search} />;
}
