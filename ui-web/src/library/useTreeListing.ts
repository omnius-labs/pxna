import { useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useMemo, useState, useSyncExternalStore } from "react";
import { folderListingStore } from "./listing";
import { ancestorKeys } from "./tree";

/// 展開中の階層は先頭と可視ページだけを購読し、閉じた階層の未完了要求を止める。
export function useTreeListing(expanded: ReadonlySet<string>, wanted: string) {
    const client = useQueryClient();
    const [owner] = useState(() => ({}));
    const listings = useMemo(
        () =>
            new Map(
                [...expanded]
                    .filter((key) => {
                        const separator = key.indexOf(":");
                        return ancestorKeys(key.slice(0, separator), key.slice(separator + 1))
                            .slice(0, -1)
                            .every((parent) => expanded.has(parent));
                    })
                    .map((key) => {
                        const separator = key.indexOf(":");
                        return [
                            key,
                            folderListingStore(
                                client,
                                key.slice(0, separator),
                                key.slice(separator + 1),
                                { key: "name", order: "asc" },
                                undefined,
                                true,
                            ),
                        ] as const;
                    }),
            ),
        [client, expanded],
    );
    const subscribe = useCallback(
        (listener: () => void) => {
            const stops = [...listings.values()].map((store) => store.subscribe(listener));
            return () => {
                for (const stop of stops) stop();
            };
        },
        [listings],
    );
    const revision = useSyncExternalStore(subscribe, () =>
        [...listings.values()].map((store) => store.version).join(","),
    );
    useEffect(() => {
        const pages = new Map<string, number[]>(JSON.parse(wanted));
        let active = true;
        queueMicrotask(() => {
            if (active) for (const [key, store] of listings) store.setWanted(owner, new Set(pages.get(key) ?? []));
        });
        return () => {
            active = false;
        };
    }, [listings, owner, wanted]);
    useEffect(
        () => () => {
            for (const store of listings.values()) store.release(owner);
        },
        [listings, owner],
    );
    return { listings, revision };
}
