import { type CSSProperties, useEffect, useState } from "react";

import type { DisplaySet } from "../library/preferences";
import { fetchThumbnail } from "./api";
import { FLIPBOOK_INTERVAL_MS, fitFlipbookStyle, flipbookStyle, nextFrame } from "./flipbook";

/// 動画のタイルにマウスを重ねている間、スプライトシートのフレームを順に切り替える背景の指定を返す。
/// 重ねていない間と、スプライトシートを取得し終えるまでは `undefined` を返し、タイルは 1 枚目のフレームのサムネイルを出す。
/// `size` は要求する大きさで、`side` はタイルの表示の一辺である。全体表示では、フレームの縦横比から収まる大きさを求める。
export function useFlipbook(
    active: boolean,
    libraryId: string,
    path: string,
    size: number,
    shape: DisplaySet["thumbnailShape"],
    side: number,
): CSSProperties | undefined {
    const [sprite, setSprite] = useState<{ url: string; frameCount: number; frameAspect: number } | undefined>(
        undefined,
    );
    const [frame, setFrame] = useState(0);

    // 重ねたときにスプライトシートを要求し、離れたら取り消して解放する。
    useEffect(() => {
        if (!active) {
            return;
        }
        const controller = new AbortController();
        let url: string | undefined;
        fetchThumbnail(libraryId, path, size, "sprite", shape, controller.signal).then(
            ({ blob, frameCount }) => {
                // 取り消した後に届いた結果は捨てる。
                if (controller.signal.aborted) {
                    return;
                }
                const objectUrl = URL.createObjectURL(blob);
                url = objectUrl;
                // 全体表示では、フレームの縦横比が要るので、読み込んで寸法を測る。
                const image = new Image();
                image.onload = () => {
                    if (!controller.signal.aborted) {
                        setSprite({
                            url: objectUrl,
                            frameCount,
                            frameAspect: image.naturalWidth / frameCount / Math.max(1, image.naturalHeight),
                        });
                    }
                };
                image.src = objectUrl;
            },
            () => undefined,
        );
        return () => {
            controller.abort();
            if (url !== undefined) {
                URL.revokeObjectURL(url);
            }
            setSprite(undefined);
            setFrame(0);
        };
    }, [active, libraryId, path, size, shape]);

    const frameCount = sprite?.frameCount ?? 1;
    useEffect(() => {
        if (frameCount < 2) {
            return;
        }
        const timer = setInterval(() => setFrame((current) => nextFrame(current, frameCount)), FLIPBOOK_INTERVAL_MS);
        return () => clearInterval(timer);
    }, [frameCount]);

    if (!active || sprite === undefined) {
        return undefined;
    }
    return shape === "fit"
        ? fitFlipbookStyle(sprite.url, frame, sprite.frameCount, side, sprite.frameAspect)
        : flipbookStyle(sprite.url, frame, sprite.frameCount);
}
