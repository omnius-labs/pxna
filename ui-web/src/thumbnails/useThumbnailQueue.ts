import { useEffect, useMemo } from "react";

import { requestTileSide } from "../library/grid";
import type { DisplaySet } from "../library/preferences";
import { fetchThumbnail, thumbnailSize } from "./api";
import { ThumbnailQueue } from "./queue";

/// サムネイルを要求するキューと、要求する大きさ（CSS ピクセルに画素密度を掛けた値）を返す。
/// 大きさと形は、タイルサイズ、画面の幅の区分、形、Library を変えたときだけ決め直し、キューを作り直す。古いキューは破棄する。
/// 一覧の幅は引数に取らないので、幅が変わっただけでは作り直さない。幅を変えている間に、要求が続けて出ないようにするためである。
export function useThumbnailQueue(
    libraryId: string,
    tileSize: number,
    narrow: boolean,
    shape: DisplaySet["thumbnailShape"],
    rowSide?: number,
) {
    const { queue, size } = useMemo(() => {
        const requested = thumbnailSize(
            rowSide ?? requestTileSide({ tileSize, narrow, screenWidth: window.innerWidth }),
            window.devicePixelRatio,
        );
        return {
            size: requested,
            queue: new ThumbnailQueue((path, signal) =>
                fetchThumbnail(libraryId, path, requested, "thumbnail", shape, signal).then((result) => result.blob),
            ),
        };
    }, [libraryId, tileSize, narrow, shape, rowSide]);
    useEffect(() => () => queue.dispose(), [queue]);
    return { queue, size };
}
