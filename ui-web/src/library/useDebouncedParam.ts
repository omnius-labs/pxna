import { useEffect, useRef, useState } from "react";

/// 入力はすぐに表示し、値は入力が止まってから `commit` で URL へ反映する。
/// 自分の反映では入力中の空白を保ち、戻る操作などで URL が外から変わったときだけ入力欄へ反映する。
export function useDebouncedParam(param: string | undefined, commit: (value: string) => void) {
    const key = param ?? "";
    const [draft, setDraft] = useState({ value: key, seen: key, pending: undefined as string | undefined });
    if (draft.seen !== key) {
        setDraft(
            draft.pending === key
                ? { ...draft, seen: key, pending: undefined }
                : { value: key, seen: key, pending: undefined },
        );
    }
    const latest = useRef(commit);
    useEffect(() => {
        latest.current = commit;
    });
    useEffect(() => {
        const value = draft.value.trim();
        if (value === key) return;
        const timer = setTimeout(() => {
            setDraft((current) => ({ ...current, pending: value }));
            latest.current(value);
        }, 300);
        return () => clearTimeout(timer);
    }, [draft.value, key]);
    return [draft.value, (value: string) => setDraft((current) => ({ ...current, value }))] as const;
}
