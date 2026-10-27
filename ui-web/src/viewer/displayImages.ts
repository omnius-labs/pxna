import type { ImageSource } from "../images/source";
import { type ImageSpec, specKey, type Want } from "../images/spec";

/// ビューアの表示用画像の「欲しい」（表示中の画像と、前後の先読み）。
///
/// 表示中の画像は `interactive`、先読みは `prefetch` で欲しいと言う。先読み中の画像を表示するときは、同じ画像の急ぎ度を
/// `interactive` へ上げるだけで、daemon は作り始めた生成をやり直さずに前へ出す。読み終えた先読みは、そのまま使う。
/// 残す画像から外れた画像は欲しい一覧から外し、daemon が、まだ作り始めていなければ作らない。
/// 読み終えた画像は、戻ったときにすぐ出せるよう、新しいものから `KEPT_LOADED` 枚まで残す。
export class DisplayImages {
    static readonly KEPT_LOADED = 6;

    readonly #source: ImageSource;
    readonly #wants = new Map<string, Want>();
    /// 読み終えた画像の鍵。古いものが先。残す枚数の上限を超えたら、古いものから手放す。
    #loaded: string[] = [];
    readonly #pending = new Set<() => void>();

    constructor(source: ImageSource) {
        this.#source = source;
    }

    prefetch(spec: ImageSpec): void {
        const key = specKey(spec);
        if (this.#wants.get(key)?.class !== "interactive") {
            this.#wants.set(key, { spec, class: "prefetch", order: 1 });
            this.#apply();
        }
    }

    /// 表示する画像の blob の URL を返す。読めなければ拒否する。
    show(spec: ImageSpec): Promise<string> {
        this.#wants.set(specKey(spec), { spec, class: "interactive", order: 0 });
        this.#apply();
        return new Promise((resolve, reject) => {
            let unsubscribe = () => {};
            const settle = () => {
                const state = this.#source.getState(spec);
                if (state.status === "ready") {
                    this.#pending.delete(cancel);
                    unsubscribe();
                    resolve(state.url);
                } else if (state.status === "failed") {
                    this.#pending.delete(cancel);
                    unsubscribe();
                    reject(new Error("the display image could not be made"));
                }
            };
            const cancel = () => {
                unsubscribe();
                reject(new Error("the display images were disposed"));
            };
            this.#pending.add(cancel);
            unsubscribe = this.#source.subscribe(settle);
            settle();
        });
    }

    /// `specs` に含まれない画像を欲しい一覧から外す。読み終えた画像は、新しいものから `KEPT_LOADED` 枚だけ残す。
    retain(specs: Iterable<ImageSpec>): void {
        const kept = new Set<string>();
        for (const spec of specs) kept.add(specKey(spec));
        for (const [key, want] of this.#wants) {
            if (this.#source.getState(want.spec).status === "ready" && !this.#loaded.includes(key))
                this.#loaded.push(key);
        }
        const extras = this.#loaded
            .filter((key) => !kept.has(key) && this.#wants.has(key))
            .slice(-DisplayImages.KEPT_LOADED);
        for (const key of [...this.#wants.keys()]) {
            if (!kept.has(key) && !extras.includes(key)) this.#wants.delete(key);
        }
        extras.forEach((key, index) => {
            const want = this.#wants.get(key);
            if (want !== undefined) this.#wants.set(key, { ...want, class: "prefetch", order: 100 + index });
        });
        this.#loaded = this.#loaded.filter((key) => kept.has(key) || extras.includes(key));
        this.#apply();
    }

    dispose(): void {
        this.#wants.clear();
        this.#loaded = [];
        for (const cancel of [...this.#pending]) cancel();
        this.#pending.clear();
        this.#source.release(this);
    }

    #apply(): void {
        this.#source.setWants(this, new Map(this.#wants));
    }
}
