import type { BrowseLocation } from "../location";

/// タイムラインからフォルダ表示へ戻るときの行き先。最後に開いていたフォルダを、タブを開いている間だけ覚える。
let lastFolder: BrowseLocation | undefined;

/// フォルダ表示で開いたフォルダを覚える。ビューアと選んで見せる Entry は覚えない。
export function rememberFolder(location: BrowseLocation) {
    lastFolder = { libraryId: location.libraryId, path: location.path };
}

/// 最後に開いていたフォルダ。まだフォルダ表示を開いていなければ `undefined`。
export function lastFolderLocation(): BrowseLocation | undefined {
    return lastFolder;
}
