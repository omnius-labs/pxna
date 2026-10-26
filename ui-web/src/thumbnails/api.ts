import { api, unwrap } from "../api/client";
import { clientTimings } from "../api/clientTimings";
import type { DisplaySet } from "../library/preferences";
import { entryAddress } from "../location";

export type ThumbnailVariant = "thumbnail" | "sprite";

export interface FetchedThumbnail {
    blob: Blob;
    /// スプライトシートが持つフレームの数。サムネイルでは 1 とする。
    frameCount: number;
}

/// サムネイルの形（表示設定）を、API の `resize` に写す。切抜きは `crop`、全体表示は `fit` である。
export function resizeFor(shape: DisplaySet["thumbnailShape"]): "crop" | "fit" {
    return shape === "fit" ? "fit" : "crop";
}

/// サムネイルかスプライトシートを取得する。幅と高さに同じ値を渡し、形は `resize` で明示する。
/// 切抜きでは、サムネイルも、スプライトシートの各フレームも、同じ大きさの正方形になる。全体表示では、縦横比を保って正方形に収まる。
/// サーバは、サムネイルとスプライトシートが同じ幅、高さ、形、出力形式で要求されたときだけ、動画のフレームを 1 回でまとめて生成する。
/// `scope` は Library の id か、タイムラインの範囲である。
export async function fetchThumbnail(
    scope: string,
    scopedPath: string,
    size: number,
    variant: ThumbnailVariant,
    shape: DisplaySet["thumbnailShape"],
    signal: AbortSignal,
): Promise<FetchedThumbnail> {
    const { libraryId, path } = entryAddress(scope, scopedPath);
    const startedAt = performance.now();
    const result = await api.GET("/libraries/{libraryId}/thumbnails", {
        params: { path: { libraryId }, query: { path, variant, width: size, height: size, resize: resizeFor(shape) } },
        parseAs: "blob",
        signal,
    });
    const blob = unwrap(result);
    clientTimings.recordResource("thumbnail_image", result.response.url, scope, scopedPath, startedAt);
    const header = result.response.headers.get("Pxna-Frame-Count");
    return { blob, frameCount: header === null ? 1 : Math.max(1, Number.parseInt(header, 10) || 1) };
}

/// タイルのサムネイルの大きさ。画面の画素密度を掛ける。サーバがサイズ段階へ切り上げるので、段階の値は知らなくてよい。
export function thumbnailSize(tileSize: number, pixelRatio: number): number {
    return Math.ceil(tileSize * Math.max(1, pixelRatio));
}
