import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { X } from "lucide-react";
import { useCallback, useEffect, useId, useRef, useState } from "react";

import { tagVocabularyQuery } from "../library/ratingTags";
import { isTypingTarget } from "../library/shortcuts";
import { browseHref, joinPath, parseRatingFilter, pathSegments, type SearchLocation, searchHref } from "../location";
import { Button } from "./ui/button";
import { Input } from "./ui/input";

interface SearchBoxProps {
    search: SearchLocation;
    /// Library の名前。起点の選択の先頭に出す。
    libraryName: string;
    /// 広い画面でだけ、`/` で検索語の欄へ移す。
    shortcut?: boolean;
}

/// 入力はすぐに表示し、値は入力が止まってから `commit` で URL へ反映する。
/// 自分の反映では入力中の空白を保ち、戻る操作などで URL が外から変わったときだけ入力欄へ反映する。
function useDebouncedParam(param: string | undefined, commit: (value: string) => void) {
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

/// 検索モードの操作。検索語、起点、範囲、評価、タグ、検索の終了を持つ。
export function SearchBox({ search, libraryName, shortcut = false }: SearchBoxProps) {
    const navigate = useNavigate();
    const input = useRef<HTMLInputElement>(null);
    const { libraryId, path, view, query, flat, rating, tag } = search;
    const vocabulary = useQuery(tagVocabularyQuery);
    const listId = useId();

    useEffect(() => {
        input.current?.focus();
    }, []);

    const replace = useCallback(
        (next: Partial<SearchLocation>) => {
            void navigate({
                href: searchHref({ libraryId, path, view, query, flat, rating, tag, ...next }),
                replace: true,
            });
        },
        [libraryId, path, view, query, flat, rating, tag, navigate],
    );
    const [queryDraft, setQueryDraft] = useDebouncedParam(query, (value) => replace({ query: value, view: undefined }));
    const [tagDraft, setTagDraft] = useDebouncedParam(tag, (value) => replace({ tag: value, view: undefined }));

    useEffect(() => {
        if (!shortcut) return;
        const keyDown = (event: KeyboardEvent) => {
            if (event.key !== "/" || event.altKey || event.ctrlKey || event.metaKey || isTypingTarget(event.target))
                return;
            event.preventDefault();
            input.current?.focus();
        };
        window.addEventListener("keydown", keyDown);
        return () => window.removeEventListener("keydown", keyDown);
    }, [shortcut]);

    const segments = pathSegments(path);
    const origins = [
        { label: libraryName, path: "/" },
        ...segments.map((segment, index) => ({ label: segment, path: joinPath(segments.slice(0, index + 1)) })),
    ];

    return (
        <div className="flex min-w-0 flex-wrap items-center gap-2">
            <Input
                ref={input}
                type="search"
                aria-label="検索"
                placeholder="検索"
                className="h-8 min-w-0 flex-1 basis-40 xl:max-w-64"
                value={queryDraft}
                onChange={(event) => setQueryDraft(event.target.value)}
            />
            <select
                aria-label="検索の起点"
                className="h-8 min-w-0 max-w-48 rounded-md border bg-background px-2 text-xs"
                value={joinPath(segments)}
                onChange={(event) => replace({ path: event.target.value, view: undefined })}
            >
                {origins.map((origin) => (
                    <option key={origin.path} value={origin.path}>
                        {origin.path === "/" ? `${origin.label}（全体）` : `${origin.label} の配下`}
                    </option>
                ))}
            </select>
            <select
                aria-label="評価"
                className="h-8 rounded-md border bg-background px-2 text-xs"
                value={rating ?? ""}
                onChange={(event) => replace({ rating: parseRatingFilter(event.target.value), view: undefined })}
            >
                <option value="">評価: すべて</option>
                <option value="good">評価: 良い</option>
                <option value="bad">評価: 悪い</option>
                <option value="none">評価: 未評価</option>
            </select>
            <Input
                type="text"
                aria-label="タグ"
                placeholder="タグ"
                list={listId}
                className="h-8 w-28 min-w-0"
                value={tagDraft}
                onChange={(event) => setTagDraft(event.target.value)}
            />
            <datalist id={listId}>
                {(vocabulary.data ?? []).map((word) => (
                    <option key={word} value={word} />
                ))}
            </datalist>
            <label className="flex shrink-0 items-center gap-1 whitespace-nowrap text-xs">
                <input
                    type="checkbox"
                    checked={!!flat}
                    onChange={(event) => replace({ flat: event.target.checked, view: undefined })}
                />
                サブフォルダを含めない
            </label>
            <Button
                variant="ghost"
                size="icon"
                aria-label="検索を終える"
                onClick={() => void navigate({ href: browseHref({ libraryId, path }) })}
            >
                <X />
            </Button>
        </div>
    );
}
