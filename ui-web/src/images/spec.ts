import type { ErrorKind, ItemKind, RequestClass, Shape } from "../api/imageChannel";

export type { ErrorKind, Shape };
/// 欲しい画像の種類。サムネイル、動画のスプライトシート、ビューアの表示用画像。
export type ImageKind = ItemKind;
/// どれだけ急いで欲しいか。ビューアで今見ている画像、画面に出ているもの、先読み。
export type ImageClass = RequestClass;

/// 欲しい画像 1 枚。出力の形式は daemon の設定で決まるので、持たない。
export interface ImageSpec {
    libraryId: string;
    /// Library 相対パス。
    path: string;
    kind: ImageKind;
    width?: number;
    height?: number;
    shape?: Shape;
}

/// 1 つの部品が、ある画像を欲しいと言うときの急ぎ度。`order` は同じ段の中の順で、小さいほど先。
export interface Want {
    spec: ImageSpec;
    class: ImageClass;
    order: number;
}

/// 1 枚の画像の状態。`failed` は、その画像を欲しいと言い続けている間は要求し直さない。
export type ImageState =
    | { status: "idle" }
    | { status: "loading" }
    | { status: "ready"; url: string; frameCount: number | null }
    | { status: "failed"; kind: ErrorKind | undefined };

export const IDLE: ImageState = { status: "idle" };

/// 同じ画像を指す仕様が、同じ文字列になる鍵。部品をまたいで 1 つの項目にまとめるのに使う。
export function specKey(spec: ImageSpec): string {
    return [spec.libraryId, spec.path, spec.kind, spec.width ?? "", spec.height ?? "", spec.shape ?? ""].join("\u0000");
}

const CLASS_RANK: Record<ImageClass, number> = { interactive: 0, visible: 1, prefetch: 2 };

/// 急ぎ度の比較。`a` のほうが先に欲しいときに負の値を返す。
export function compareWants(a: Pick<Want, "class" | "order">, b: Pick<Want, "class" | "order">): number {
    return CLASS_RANK[a.class] - CLASS_RANK[b.class] || a.order - b.order;
}
