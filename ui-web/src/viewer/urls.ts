import type { paths } from "../api/schema";
import { entryAddress } from "../location";

type DisplayQuery = NonNullable<paths["/libraries/{libraryId}/thumbnails"]["get"]["parameters"]["query"]>;

function apiUrl(libraryId: string, resource: string, query: Record<string, string | number>): string {
    const params = new URLSearchParams(Object.entries(query).map(([key, value]) => [key, String(value)]));
    return `/api/v1/libraries/${encodeURIComponent(libraryId)}/${resource}?${params.toString()}`;
}

/// ビューアが最初に出す、縦横比を保った表示用画像の URL。`scope` は Library の id か、タイムラインの範囲である。
export function displayImageUrl(scope: string, scopedPath: string, size: { width: number; height: number }): string {
    const { libraryId, path } = entryAddress(scope, scopedPath);
    const query = {
        path,
        variant: "display",
        width: size.width,
        height: size.height,
        resize: "fit",
    } satisfies DisplayQuery;
    return apiUrl(libraryId, "thumbnails", query);
}

/// 拡大したときに差し替える、元の画像の URL。
export function originalImageUrl(scope: string, path: string): string {
    return fileContentUrl(scope, path);
}

/// 単一ファイルの内容。ビューアとダウンロードで同じ URL を使う。
export function fileContentUrl(scope: string, scopedPath: string): string {
    const { libraryId, path } = entryAddress(scope, scopedPath);
    return apiUrl(libraryId, "files/content", { path });
}
