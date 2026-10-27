import { type CSSProperties, useEffect, useMemo, useState, useSyncExternalStore } from "react";

import { imageSource } from "../images/source";
import { IDLE, specKey } from "../images/spec";
import { thumbnailSpec } from "../images/wants";
import type { DisplaySet } from "../library/preferences";
import { FLIPBOOK_INTERVAL_MS, fitFlipbookStyle, flipbookStyle, nextFrame } from "./flipbook";

/// 動画のタイルにマウスを重ねている間、スプライトシートのフレームを順に切り替える背景の指定を返す。
/// 重ねていない間と、スプライトシートを受け取り終えるまでは `undefined` を返し、タイルは 1 枚目のフレームのサムネイルを出す。
/// `size` は要求する大きさで、`side` はタイルの表示の一辺である。全体表示では、フレームの縦横比から収まる大きさを求める。
export function useFlipbook(
    active: boolean,
    libraryId: string,
    path: string,
    size: number,
    shape: DisplaySet["thumbnailShape"],
    side: number,
): CSSProperties | undefined {
    const source = imageSource();
    const owner = useMemo(() => ({}), []);
    // サムネイルと同じ大きさと形で欲しいと言う。daemon は、同じ条件のときだけ 1 回の ffmpeg の実行で両方を作り、
    // そのデコードから小さい段階の両方の形も companion として作る。
    const spec = useMemo(
        () => ({ ...thumbnailSpec(libraryId, path, size, shape), kind: "sprite" as const }),
        [libraryId, path, size, shape],
    );
    const [frame, setFrame] = useState(0);
    const [aspect, setAspect] = useState<{ url: string; frameAspect: number } | undefined>(undefined);

    // 重ねたときにスプライトシートを欲しいと言い、離れたらやめる。
    useEffect(() => {
        if (!active) {
            return;
        }
        source.setWants(owner, new Map([[specKey(spec), { spec, class: "visible", order: 0 }]]));
        return () => {
            source.release(owner);
            setFrame(0);
        };
    }, [active, source, owner, spec]);

    const state = useSyncExternalStore(source.subscribe, () => (active ? source.getState(spec) : IDLE));
    const sprite = state.status === "ready" ? state : undefined;

    // 全体表示では、フレームの縦横比が要るので、読み込んで寸法を測る。
    const spriteUrl = sprite?.url;
    const frameCountOfSprite = sprite?.frameCount ?? 1;
    useEffect(() => {
        if (spriteUrl === undefined) {
            return;
        }
        let current = true;
        const image = new Image();
        image.onload = () => {
            if (current) {
                setAspect({
                    url: spriteUrl,
                    frameAspect: image.naturalWidth / frameCountOfSprite / Math.max(1, image.naturalHeight),
                });
            }
        };
        image.src = spriteUrl;
        return () => {
            current = false;
        };
    }, [spriteUrl, frameCountOfSprite]);

    const frameCount = sprite?.frameCount ?? 1;
    useEffect(() => {
        if (frameCount < 2) {
            return;
        }
        const timer = setInterval(() => setFrame((current) => nextFrame(current, frameCount)), FLIPBOOK_INTERVAL_MS);
        return () => clearInterval(timer);
    }, [frameCount]);

    if (!active || sprite === undefined || aspect?.url !== sprite.url) {
        return undefined;
    }
    return shape === "fit"
        ? fitFlipbookStyle(sprite.url, frame, frameCount, side, aspect.frameAspect)
        : flipbookStyle(sprite.url, frame, frameCount);
}
