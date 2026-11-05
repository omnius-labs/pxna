import { t } from "i18next";
import { useEffect, useRef } from "react";
import { toast } from "sonner";
import { describeError } from "../i18n/errors";

import { joinPath, pathSegments } from "../location";
import type { Entry } from "./entries";
import type { Listing } from "./listing";

/// 開いたフォルダの直下の Entry を、名前から探して選び、見える位置までスクロールする。見つけても見つけられなくても、終わったら `onDone` を呼ぶ。
/// タイムラインの「フォルダで表示」が、フォルダ表示の URL の `reveal` に載せた名前で使う。
export function useRevealEntry({
    listing,
    columns,
    virtualizer,
    folderPath,
    name,
    onSelect,
    onDone,
}: {
    listing: Listing;
    columns: number;
    virtualizer: { scrollToIndex(index: number): void };
    folderPath: string;
    name: string | undefined;
    onSelect: (entry: Entry, index: number) => void;
    onDone: () => void;
}) {
    // 同じ名前を繰り返し探さない。選択や位置が変わって再描画しても、終えた名前は探し直さない。
    const handled = useRef<string>(undefined);
    const latest = useRef({ onSelect, onDone, virtualizer, columns });
    useEffect(() => {
        latest.current = { onSelect, onDone, virtualizer, columns };
    });
    useEffect(() => {
        if (name === undefined || handled.current === name) return;
        const controller = new AbortController();
        const path = joinPath([...pathSegments(folderPath), name]);
        void findIndex(listing, path, controller.signal)
            .then((index) => {
                if (controller.signal.aborted) return;
                handled.current = name;
                const entry = index === undefined ? undefined : listing.entryAt(index);
                if (index === undefined || entry === undefined) toast.error(t("errors.revealNotFound"));
                else {
                    latest.current.virtualizer.scrollToIndex(Math.floor(index / latest.current.columns));
                    latest.current.onSelect(entry, index);
                }
                latest.current.onDone();
            })
            .catch((error) => {
                if (controller.signal.aborted) return;
                handled.current = name;
                toast.error(describeError(error, t("errors.revealFailed")));
                latest.current.onDone();
            });
        return () => controller.abort();
    }, [listing, folderPath, name]);
}

export async function findIndex(listing: Listing, path: string, signal: AbortSignal): Promise<number | undefined> {
    signal.throwIfAborted();
    if (listing.totalCount === undefined) await listing.loadPages([0], signal);
    signal.throwIfAborted();
    // Answer from the loaded range when it holds the Entry.
    const loaded = listing.locate?.(path);
    if (loaded !== undefined) return loaded;
    // An Entry on an unread page is found by asking the server for its page.
    if (listing.locateRemote) {
        const index = await listing.locateRemote(path, signal);
        signal.throwIfAborted();
        return index;
    }
    return undefined;
}
