import type { ImageSpec, ImageState, Want } from "./spec";

/// 部品が画像を受け取る窓口。1 つのタブに 1 つあり、部品の「欲しい」をまとめて daemon の image channel へ伝える（Client Design §4）。
export interface ImageSource {
    getState(spec: ImageSpec): ImageState;
    /// 状態が変わったときに呼ぶ。`useSyncExternalStore` に渡す。
    subscribe(listener: () => void): () => void;
    /// `owner` が欲しい画像を、これで置き換える。キーは `specKey`。同じ画像を複数の部品が欲しいときは、1 つにまとめる。
    setWants(owner: object, wants: ReadonlyMap<string, Want>): void;
    /// `owner` が欲しかった画像をすべてやめる。`owner` だけが欲しかった画像は解放し、失敗した状態も捨てる。
    release(owner: object): void;
    /// 手元の画像と置き場をすべて捨てて、接続を閉じる。ログアウトで使う。
    clear(): Promise<void>;
}

let current: ImageSource | undefined;
let sessionEnded: () => void = () => {};

/// このタブの窓口。アプリの起動時に `setImageSource` で差し込む。
export function imageSource(): ImageSource {
    if (current === undefined) {
        throw new Error("the image source is not initialized");
    }
    return current;
}

/// 窓口を差し替える。テストが偽物を渡すために使う。前の窓口を返すので、後で戻せる。`undefined` を渡すと、次に使うときに作り直す。
export function setImageSource(source: ImageSource | undefined): ImageSource | undefined {
    const previous = current;
    current = source;
    return previous;
}

/// セッションが終わったと daemon に言われたときにすることを決める。アプリがログインへ移すために使う。
export function onImageSessionEnded(handler: () => void): void {
    sessionEnded = handler;
}

/// セッションが終わったと daemon に言われたときに、登録された処理を呼ぶ。
export function notifyImageSessionEnded(): void {
    sessionEnded();
}
