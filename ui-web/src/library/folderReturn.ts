import { type BrowseLocation, joinPath, pathSegments } from "../location";

/// 最後に開いたフォルダを保存する `localStorage` のキー。値は `libraryId` と `path` を持つ JSON である。
export const LAST_FOLDER_KEY = "pxna.lastFolder";

interface StoredFolder {
    libraryId: string;
    path: string;
}

/// 保存した値を読む。形が違うときは `undefined` にする。
function parseFolder(raw: string): StoredFolder | undefined {
    const value: unknown = JSON.parse(raw);
    if (typeof value !== "object" || value === null) {
        return undefined;
    }
    const { libraryId, path } = value as { libraryId?: unknown; path?: unknown };
    if (typeof libraryId !== "string" || libraryId === "") {
        return undefined;
    }
    if (typeof path !== "string" || !path.startsWith("/")) {
        return undefined;
    }
    const segments = pathSegments(path);
    // 保存した値から URL を作るので、`..` などで Library の外を指す値は使わない。
    if (segments.some((segment) => segment === "." || segment === "..")) {
        return undefined;
    }
    return { libraryId, path: joinPath(segments) };
}

/// 最後に開いていたフォルダ。まだ開いていなければ `undefined`。
export function lastFolderLocation(): BrowseLocation | undefined {
    try {
        const stored = window.localStorage.getItem(LAST_FOLDER_KEY);
        return stored === null ? undefined : parseFolder(stored);
    } catch {
        return undefined;
    }
}

/// フォルダ表示で開いたフォルダを覚える。ビューアと選んで見せる Entry は覚えない。
export function rememberFolder(location: BrowseLocation): void {
    try {
        window.localStorage.setItem(
            LAST_FOLDER_KEY,
            JSON.stringify({ libraryId: location.libraryId, path: joinPath(pathSegments(location.path)) }),
        );
    } catch {
        // 保存できなくても、閲覧は妨げない。
    }
}
