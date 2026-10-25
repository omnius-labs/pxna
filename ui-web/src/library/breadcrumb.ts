import { type BrowseLocation, joinPath, pathSegments } from "../location";

/// パンくずの 1 項目。`location` はその項目へ移る位置で、ビューアで開いている Entry は含まない。
export interface Crumb {
    label: string;
    location: BrowseLocation;
}

/// Library の直下から今のフォルダまでの経路。Library はツリーで分かるので含めず、ルートでは空になる。最後が今いるフォルダである。
export function breadcrumbs(location: BrowseLocation): Crumb[] {
    const segments = pathSegments(location.path);
    return segments.map((segment, index) => ({
        label: segment,
        location: { libraryId: location.libraryId, path: joinPath(segments.slice(0, index + 1)) },
    }));
}

/// 1 つ上のフォルダ。Library のルートより上へは辿らないので、ルートでは `undefined` を返す。
export function parentLocation(location: BrowseLocation): BrowseLocation | undefined {
    const segments = pathSegments(location.path);
    return segments.length === 0 ? undefined : { libraryId: location.libraryId, path: joinPath(segments.slice(0, -1)) };
}
