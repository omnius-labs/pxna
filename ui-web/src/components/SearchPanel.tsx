import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { t } from "i18next";
import { ChevronDown } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { librariesQuery } from "../library/queries";
import { tagVocabularyQuery } from "../library/ratingTags";
import { searchLibraryIds } from "../library/searchCriteria";
import { isTypingTarget } from "../library/shortcuts";
import { useDebouncedParam } from "../library/useDebouncedParam";
import { parseRatingFilter, type SearchLocation, searchHref } from "../location";
import { Tree } from "./Tree";
import { Button } from "./ui/button";
import { Checkbox } from "./ui/checkbox";
import { Input } from "./ui/input";
import { Popover, PopoverContent, PopoverTrigger } from "./ui/popover";

/// 検索の条件を変える。URL を置き換え、ビューアで開いている Entry は閉じる。
export function useSearchChange(search: SearchLocation) {
    const navigate = useNavigate();
    return (next: Partial<SearchLocation>) =>
        void navigate({ href: searchHref({ ...search, view: undefined, ...next }), replace: true });
}

/// 検索語の欄。入力が止まってから URL へ反映する。広い画面では `/` でここへ移る。
export function SearchTermsField({ search, shortcut = false }: { search: SearchLocation; shortcut?: boolean }) {
    const input = useRef<HTMLInputElement>(null);
    const change = useSearchChange(search);
    const [draft, setDraft] = useDebouncedParam(search.query, (value) => change({ query: value }));
    useEffect(() => {
        input.current?.focus();
    }, []);
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
    return (
        <Input
            ref={input}
            type="search"
            aria-label={t("search.query")}
            placeholder={t("search.placeholder")}
            className="h-8 min-w-0"
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
        />
    );
}

const ratingOptions = () =>
    [
        { value: "", label: t("search.all") },
        { value: "good", label: t("rating.good") },
        { value: "bad", label: t("rating.bad") },
        { value: "none", label: t("rating.none") },
    ] as const;

/// 検索モードの左の領域。Library、場所、範囲、評価、タグを選ぶ。検索語は、狭い画面ではヘッダーの下に出すので、`terms` のときだけここに出す。
export function SearchPanel({ search, terms }: { search: SearchLocation; terms: boolean }) {
    const libraries = useQuery(librariesQuery);
    const vocabulary = useQuery(tagVocabularyQuery);
    const change = useSearchChange(search);
    const [startOpen, setStartOpen] = useState(false);
    const all = libraries.data ?? [];
    const chosen = searchLibraryIds(search, all);
    const only = chosen.length === 1 ? chosen[0] : undefined;
    const onlyName = all.find((library) => library.id === only)?.name;
    const words = [...new Set([...(vocabulary.data ?? []), ...search.tags])];

    const toggleLibrary = (id: string, checked: boolean) => {
        const next = new Set(chosen);
        if (checked) next.add(id);
        else next.delete(id);
        const ids = all.map((library) => library.id).filter((candidate) => next.has(candidate));
        if (ids.length === 0) return;
        // すべてを選ぶことは「指定なし」と同じで、後から加わった Library も探す。
        change({ libraryIds: ids.length === all.length ? undefined : ids, path: ids.length === 1 ? search.path : "/" });
    };
    const toggleTag = (word: string, checked: boolean) =>
        change({ tags: checked ? [...search.tags, word] : search.tags.filter((tag) => tag !== word) });

    return (
        <div className="flex flex-col gap-4 p-3 text-sm">
            {terms && <SearchTermsField search={search} shortcut />}
            <section aria-label="Library" className="flex flex-col gap-1">
                <h2 className="text-xs font-semibold text-muted-foreground">Library</h2>
                {all.map((library) => {
                    const checked = chosen.includes(library.id);
                    const last = checked && chosen.length === 1;
                    return (
                        <div
                            key={library.id}
                            className="flex items-center gap-2"
                            title={last ? t("search.minLibrary") : undefined}
                        >
                            <Checkbox
                                id={`search-library-${library.id}`}
                                checked={checked}
                                disabled={last}
                                onCheckedChange={(next) => toggleLibrary(library.id, next === true)}
                            />
                            <label
                                htmlFor={`search-library-${library.id}`}
                                className={`min-w-0 flex-1 truncate ${last ? "" : "cursor-pointer"}`}
                            >
                                {library.name}
                            </label>
                        </div>
                    );
                })}
            </section>
            <section aria-label={t("search.place")} className="flex flex-col gap-1">
                <h2 className="text-xs font-semibold text-muted-foreground">{t("search.place")}</h2>
                {only === undefined ? (
                    <p className="text-muted-foreground">{t("search.wholeEachLibrary")}</p>
                ) : (
                    <Popover open={startOpen} onOpenChange={setStartOpen}>
                        <PopoverTrigger asChild>
                            <Button
                                variant="outline"
                                size="sm"
                                className="justify-between"
                                aria-label={t("search.choosePlace")}
                            >
                                <span className="min-w-0 truncate">
                                    {search.path === "/"
                                        ? t("search.wholeOf", { name: onlyName ?? only })
                                        : search.path}
                                </span>
                                <ChevronDown />
                            </Button>
                        </PopoverTrigger>
                        <PopoverContent align="start" className="flex w-72 flex-col gap-2 p-2">
                            <Button
                                variant="ghost"
                                size="sm"
                                className="justify-start"
                                onClick={() => {
                                    change({ path: "/" });
                                    setStartOpen(false);
                                }}
                            >
                                {t("search.wholeLibrary")}
                            </Button>
                            <div className="h-72 overflow-hidden rounded border">
                                <Tree
                                    libraryIds={[only]}
                                    location={{ libraryId: only, path: search.path }}
                                    onSelect={(location) => {
                                        change({ path: location.path });
                                        setStartOpen(false);
                                    }}
                                />
                            </div>
                        </PopoverContent>
                    </Popover>
                )}
                <div className="flex items-center gap-2 pt-1">
                    <Checkbox
                        id="search-flat"
                        checked={!!search.flat}
                        onCheckedChange={(next) => change({ flat: next === true })}
                    />
                    <label htmlFor="search-flat" className="cursor-pointer">
                        {t("search.noSubfolders")}
                    </label>
                </div>
            </section>
            <section aria-label={t("common.rating")} className="flex flex-col gap-1">
                <h2 className="text-xs font-semibold text-muted-foreground">{t("common.rating")}</h2>
                <div role="radiogroup" aria-label={t("common.rating")} className="flex flex-wrap gap-x-3 gap-y-1">
                    {ratingOptions().map((rating) => (
                        <label key={rating.value} className="flex cursor-pointer items-center gap-1">
                            <input
                                type="radio"
                                name="search-rating"
                                checked={(search.rating ?? "") === rating.value}
                                onChange={() => change({ rating: parseRatingFilter(rating.value) })}
                            />
                            {rating.label}
                        </label>
                    ))}
                </div>
            </section>
            <section aria-label={t("common.tags")} className="flex flex-col gap-1">
                <h2 className="text-xs font-semibold text-muted-foreground">{t("common.tags")}</h2>
                {words.length === 0 ? (
                    <p className="text-muted-foreground">{t("search.noTags")}</p>
                ) : (
                    words.map((word) => (
                        <div key={word} className="flex items-center gap-2">
                            <Checkbox
                                id={`search-tag-${word}`}
                                checked={search.tags.includes(word)}
                                onCheckedChange={(next) => toggleTag(word, next === true)}
                            />
                            <label htmlFor={`search-tag-${word}`} className="min-w-0 flex-1 cursor-pointer truncate">
                                {word}
                            </label>
                        </div>
                    ))
                )}
            </section>
        </div>
    );
}
