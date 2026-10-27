import { type ImageSource, setImageSource } from "../images/source";
import { IDLE, type ImageSpec, type ImageState, specKey, type Want } from "../images/spec";

/// 部品のテストが使う、偽の画像の窓口。何を欲しいと言われたかを記録し、状態はテストが決める。
export class FakeImageSource implements ImageSource {
    readonly #states = new Map<string, ImageState>();
    readonly #owners = new Map<object, ReadonlyMap<string, Want>>();
    readonly #listeners = new Set<() => void>();
    /// `setWants` の呼び出しの記録。
    readonly calls: Array<{ owner: object; wants: ReadonlyMap<string, Want> }> = [];
    cleared = false;

    getState = (spec: ImageSpec): ImageState => this.#states.get(specKey(spec)) ?? IDLE;

    subscribe = (listener: () => void): (() => void) => {
        this.#listeners.add(listener);
        return () => this.#listeners.delete(listener);
    };

    setWants(owner: object, wants: ReadonlyMap<string, Want>): void {
        this.calls.push({ owner, wants });
        if (wants.size === 0) {
            this.#owners.delete(owner);
        } else {
            this.#owners.set(owner, wants);
        }
    }

    release(owner: object): void {
        this.#owners.delete(owner);
    }

    async clear(): Promise<void> {
        this.cleared = true;
    }

    /// 部品がいま欲しい画像を、部品をまたいで 1 つにまとめた一覧。
    wanted(): Want[] {
        const merged = new Map<string, Want>();
        for (const wants of this.#owners.values()) {
            for (const [key, want] of wants) merged.set(key, want);
        }
        return [...merged.values()];
    }

    /// 画像の状態を決めて、購読している部品へ知らせる。
    setState(spec: ImageSpec, state: ImageState): void {
        this.#states.set(specKey(spec), state);
        for (const listener of this.#listeners) listener();
    }

    ready(spec: ImageSpec, url: string, frameCount: number | null = null): void {
        this.setState(spec, { status: "ready", url, frameCount });
    }

    /// 窓口として差し替える。返す関数で前の窓口に戻す。
    install(): () => void {
        const previous = setImageSource(this);
        return () => void setImageSource(previous);
    }
}
