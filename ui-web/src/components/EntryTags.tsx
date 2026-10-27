import { useQuery, useQueryClient } from "@tanstack/react-query";
import { t } from "i18next";
import { X } from "lucide-react";
import { useId, useState } from "react";
import { toast } from "sonner";

import {
    entryTagsQuery,
    MAX_TAG_CHARS,
    MAX_TAGS_PER_ENTRY,
    normalizeTag,
    putEntryTags,
    RATING_LABELS,
    ratingQuery,
    tagVocabularyQuery,
} from "../library/ratingTags";
import { Input } from "./ui/input";

/// 情報パネルに出す、画像と動画の評価（表示だけ）とタグ（付け外し）。語は自由に入力でき、これまでに作った語を補完に出す。
export function EntryTags({ libraryId, path }: { libraryId: string; path: string }) {
    const client = useQueryClient();
    const listId = useId();
    const rating = useQuery(ratingQuery(libraryId, path));
    const tags = useQuery(entryTagsQuery(libraryId, path));
    const vocabulary = useQuery(tagVocabularyQuery);
    const [draft, setDraft] = useState("");
    const [busy, setBusy] = useState(false);
    const current = tags.data ?? [];
    // 付いている語を読めるまでは、保存しない。読む前の空の集合で置き換えると、保存済みの語を消してしまう。
    const loaded = tags.isSuccess;

    const save = async (next: string[]) => {
        if (!loaded) return false;
        setBusy(true);
        try {
            await putEntryTags(client, libraryId, path, next);
            return true;
        } catch {
            toast.error(t("entryTags.saveFailed"));
            return false;
        } finally {
            setBusy(false);
        }
    };
    const add = async () => {
        const tag = normalizeTag(draft);
        if (tag === undefined) {
            if (draft.trim() !== "") toast.error(t("entryTags.tooLong", { count: MAX_TAG_CHARS }));
            return;
        }
        if (current.includes(tag)) {
            setDraft("");
            return;
        }
        if (current.length >= MAX_TAGS_PER_ENTRY) {
            toast.error(t("entryTags.tooMany", { count: MAX_TAGS_PER_ENTRY }));
            return;
        }
        if (await save([...current, tag])) setDraft("");
    };

    return (
        <section aria-label={t("entryTags.section")} className="px-4 py-3">
            <div className="flex items-center justify-between text-xs">
                <span className="text-muted-foreground">{t("common.rating")}</span>
                <span data-rating={rating.data ?? "none"}>{RATING_LABELS[rating.data ?? "none"]}</span>
            </div>
            <h3 className="mt-3 text-xs text-muted-foreground">{t("common.tags")}</h3>
            {tags.isError ? (
                <p role="alert" className="mt-1 text-xs text-destructive">
                    {t("entryTags.loadFailed")}
                </p>
            ) : (
                <ul className="mt-1 flex flex-wrap gap-1">
                    {current.map((tag) => (
                        <li key={tag} className="flex items-center gap-1 rounded bg-muted px-1.5 py-0.5 text-xs">
                            <span className="break-all">{tag}</span>
                            <button
                                type="button"
                                aria-label={t("entryTags.remove", { tag })}
                                disabled={busy || !loaded}
                                className="text-muted-foreground hover:text-foreground"
                                onClick={() => void save(current.filter((existing) => existing !== tag))}
                            >
                                <X className="size-3" />
                            </button>
                        </li>
                    ))}
                </ul>
            )}
            <Input
                aria-label={t("entryTags.add")}
                placeholder={t("entryTags.add")}
                list={listId}
                className="mt-2 h-8"
                value={draft}
                disabled={busy || !loaded}
                onChange={(event) => setDraft(event.target.value)}
                onKeyDown={(event) => {
                    // 変換の確定の Enter は、語の追加にしない。
                    if (event.key === "Enter" && !event.nativeEvent.isComposing) {
                        event.preventDefault();
                        void add();
                    }
                }}
            />
            <datalist id={listId}>
                {(vocabulary.data ?? [])
                    .filter((word) => !current.includes(word))
                    .map((word) => (
                        <option key={word} value={word} />
                    ))}
            </datalist>
        </section>
    );
}
