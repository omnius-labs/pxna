import { useCallback, useEffect, useRef, useState } from "react";

/// Fullscreen API の状態と切り替え。`target` の要素を全画面にする。
/// `supported` が偽のブラウザ（`document.fullscreenEnabled` が真でない）では、操作の入口を出さない。
/// 全画面のまま使われなくなったときは、全画面を終える。
export function useFullscreen(target: HTMLElement | null) {
    const supported = document.fullscreenEnabled === true;
    const [active, setActive] = useState(false);
    useEffect(() => {
        const sync = () => setActive(target !== null && document.fullscreenElement === target);
        sync();
        // ブラウザの操作（Esc など）で全画面が終わったときも、状態を合わせる。
        document.addEventListener("fullscreenchange", sync);
        return () => document.removeEventListener("fullscreenchange", sync);
    }, [target]);

    const latest = useRef(target);
    useEffect(() => {
        latest.current = target;
    });
    useEffect(
        () => () => {
            if (latest.current !== null && document.fullscreenElement === latest.current) {
                void leave();
            }
        },
        [],
    );

    const enter = useCallback(async () => {
        try {
            await target?.requestFullscreen();
        } catch {
            // 利用者の操作の外で呼ばれたときなど、ブラウザが断ったら、全画面にしない。
        }
    }, [target]);
    const exit = useCallback(() => leave(), []);
    const toggle = useCallback(() => (active ? exit() : enter()), [active, enter, exit]);
    return { supported, active, enter, exit, toggle };
}

async function leave() {
    try {
        await document.exitFullscreen();
    } catch {
        // すでに終わっているときは、何もしない。
    }
}
