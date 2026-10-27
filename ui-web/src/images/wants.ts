import { useEffect, useMemo } from "react";
import { requestTileSide } from "../library/grid";
import type { DisplaySet } from "../library/preferences";
import { entryAddress } from "../location";
import { type ImageSource, imageSource } from "./source";
import type { ImageClass, ImageSpec, ImageState, Want } from "./spec";
import { specKey } from "./spec";

/// フォルダのタイルを、行が現れてから要求するまで待つ時間（ミリ秒）。スクロールで通り過ぎるフォルダの代表の選択を、daemon にさせないため。
const FOLDER_DELAY_MS = 200;

/// サムネイルの形（表示設定）を、画像の仕様の形に写す。切抜きは `crop`、全体表示は `fit` である。
export function shapeFor(shape: DisplaySet["thumbnailShape"]): "crop" | "fit" {
    return shape === "fit" ? "fit" : "crop";
}

/// 要求するサムネイルの一辺の大きさ。画面の画素密度を掛ける。サーバがサイズ段階へ切り上げるので、段階の値は知らなくてよい。
export function thumbnailSize(tileSize: number, pixelRatio: number): number {
    return Math.ceil(tileSize * Math.max(1, pixelRatio));
}

/// サムネイルの仕様。幅と高さに同じ値を渡し、形は `shape` で明示する。`scope` は Library の id か、タイムラインの範囲である。
export function thumbnailSpec(
    scope: string,
    scopedPath: string,
    size: number,
    shape: DisplaySet["thumbnailShape"],
): ImageSpec {
    return { ...entryAddress(scope, scopedPath), kind: "thumbnail", width: size, height: size, shape: shapeFor(shape) };
}

/// 1 つの部品（一覧、フィルムストリップなど）が欲しい画像。仕様はパスだけが違うので、パスで指す。
///
/// 呼び出し側が、いま欲しいパスと順（小さいほど先）を渡し直す。サーバが順に処理するので、同時に送る数を絞ることも、
/// 送った要求を取り消すこともしない。画面から外れたパスは欲しい一覧から外すだけで、まだ作り始めていなければ作られない。
export class ImageWants {
    readonly #source: ImageSource;
    readonly #spec: (path: string) => ImageSpec;
    #wanted: ReadonlyMap<string, number> = new Map();
    #prefetch: ReadonlySet<string> = new Set();
    readonly #delays = new Map<string, { ready: boolean; timer?: ReturnType<typeof setTimeout> }>();

    constructor(source: ImageSource, spec: (path: string) => ImageSpec) {
        this.#source = source;
        this.#spec = spec;
    }

    /// `useSyncExternalStore` に渡す。状態が変わらない間は同じ値を返す。
    getState = (path: string): ImageState => this.#source.getState(this.#spec(path));

    subscribe = (listener: () => void): (() => void) => this.#source.subscribe(listener);

    /// 欲しいパスと順を渡し直す。`delayed` のパスは、現れてから一定の間を置いて欲しいと言う。
    /// `prefetch` のパスは、画面に出ていない先読みとして、出ているものより後にする。
    setWanted(
        wanted: ReadonlyMap<string, number>,
        delayed: ReadonlySet<string> = new Set(),
        prefetch: ReadonlySet<string> = new Set(),
    ): void {
        this.#wanted = wanted;
        this.#prefetch = prefetch;
        for (const [path, delay] of this.#delays) {
            if (!wanted.has(path) || !delayed.has(path)) {
                clearTimeout(delay.timer);
                this.#delays.delete(path);
            }
        }
        for (const path of delayed) {
            if (!wanted.has(path) || this.#delays.has(path)) continue;
            const delay: { ready: boolean; timer?: ReturnType<typeof setTimeout> } = { ready: false };
            this.#delays.set(path, delay);
            // DOM の commit に伴う observer 通知の後から遅延を数える。
            queueMicrotask(() => {
                if (this.#delays.get(path) !== delay) return;
                delay.timer = setTimeout(() => {
                    if (this.#delays.get(path) !== delay) return;
                    delay.ready = true;
                    this.#apply();
                }, FOLDER_DELAY_MS);
            });
        }
        this.#apply();
    }

    /// 欲しい画像をすべてやめて、待っている遅延を取り消す。やめた後も `setWanted` で使い直せる（開発時の `StrictMode` が使い直す）。
    dispose(): void {
        this.#wanted = new Map();
        this.#prefetch = new Set();
        for (const delay of this.#delays.values()) clearTimeout(delay.timer);
        this.#delays.clear();
        this.#source.release(this);
    }

    #apply(): void {
        const wants = new Map<string, Want>();
        for (const [path, order] of this.#wanted) {
            if (this.#delays.get(path)?.ready === false) continue;
            const spec = this.#spec(path);
            const wantClass: ImageClass = this.#prefetch.has(path) ? "prefetch" : "visible";
            wants.set(specKey(spec), { spec, class: wantClass, order });
        }
        this.#source.setWants(this, wants);
    }
}

/// サムネイルを欲しいと言うための `ImageWants` と、要求する大きさ（CSS ピクセルに画素密度を掛けた値）を返す。
/// 大きさと形は、タイルサイズ、画面の幅の区分、形、Library を変えたときだけ決め直し、作り直す。古いものは手放す。
/// 一覧の幅は引数に取らないので、幅が変わっただけでは作り直さない。幅を変えている間に、欲しい画像が変わり続けないようにするためである。
export function useImageWants(
    libraryId: string,
    tileSize: number,
    narrow: boolean,
    shape: DisplaySet["thumbnailShape"],
    rowSide?: number,
) {
    const { wants, size } = useMemo(() => {
        const requested = thumbnailSize(
            rowSide ?? requestTileSide({ tileSize, narrow, screenWidth: window.innerWidth }),
            window.devicePixelRatio,
        );
        return {
            size: requested,
            wants: new ImageWants(imageSource(), (path) => thumbnailSpec(libraryId, path, requested, shape)),
        };
    }, [libraryId, tileSize, narrow, shape, rowSide]);
    useEffect(() => () => wants.dispose(), [wants]);
    return { wants, size };
}
