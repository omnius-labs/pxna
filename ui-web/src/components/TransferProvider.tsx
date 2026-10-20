import { useQueryClient } from "@tanstack/react-query";
import { type ReactNode, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { useActivity } from "../library/activity";
import { type ClipboardContents, useClipboard } from "../library/clipboard";
import { type CollisionChoice, findCollisions, resolveConflict, type TransferDestination } from "../library/collision";
import { canMutate, childPath, transferSelection } from "../library/entryActions";
import { useNarrowScreen } from "../library/preferences";
import { folderQuery, librariesQuery } from "../library/queries";
import { TransferContext } from "../library/transfer";
import { type UploadInput, useUploads } from "../library/upload";
import { CollisionDialog } from "./CollisionDialog";

export function TransferProvider({ children }: { children: ReactNode }) {
    const uploads = useUploads();
    const client = useQueryClient();
    const activity = useActivity();
    const clipboard = useClipboard();
    const narrow = useNarrowScreen();
    const [busy, setBusy] = useState(false);
    const lock = useRef(false);
    const [names, setNames] = useState<string[]>();
    const [duplicateOnly, setDuplicateOnly] = useState(false);
    const preflight = useRef<AbortController | undefined>(undefined);
    const pending = useRef<((choice: CollisionChoice) => void) | undefined>(undefined);
    const active = useRef(true);
    useEffect(() => {
        active.current = !narrow;
        if (narrow) preflight.current?.abort();
        if (narrow)
            queueMicrotask(() => {
                pending.current?.("cancel");
                pending.current = undefined;
                setNames(undefined);
            });
        return () => {
            active.current = false;
            preflight.current?.abort();
            pending.current?.("cancel");
            pending.current = undefined;
        };
    }, [narrow]);
    const choose = (choice: CollisionChoice) => {
        const resolve = pending.current;
        pending.current = undefined;
        setNames(undefined);
        resolve?.(choice);
    };
    const confirm = (collisions: string[], duplicate = false) =>
        new Promise<CollisionChoice>((resolve) => {
            if (!active.current) {
                resolve("cancel");
                return;
            }
            pending.current = resolve;
            setDuplicateOnly(duplicate);
            setNames(collisions);
        });
    const transfer = async (source: ClipboardContents, destination: TransferDestination) => {
        if (lock.current || narrow || !source.entries.length) return;
        lock.current = true;
        setBusy(true);
        const controller = new AbortController();
        preflight.current = controller;
        const signal = controller.signal;
        try {
            const libraries = await client.ensureQueryData(librariesQuery);
            const library = libraries.find((item) => item.id === destination.libraryId);
            const folder = await client.fetchQuery({
                ...folderQuery(destination.libraryId, destination.path),
                staleTime: 0,
            });
            signal.throwIfAborted();
            if (!library || !canMutate(library, folder)) throw new Error("このフォルダへは送れません。");
            if (
                source.mode === "cut" &&
                (libraries.find((item) => item.id === source.libraryId)?.readonly ||
                    source.entries.some((entry) => entry.readonly))
            )
                throw new Error("読み取り専用の項目は切り取りできません。");
            if (source.libraryId === destination.libraryId) {
                for (const entry of source.entries) {
                    const target = childPath(destination.path, entry.path.split("/").at(-1)!);
                    if (
                        (source.mode === "cut" && target === entry.path) ||
                        (entry.folder &&
                            (destination.path === entry.path || destination.path.startsWith(`${entry.path}/`)))
                    )
                        throw new Error("元と同じ場所への移動や、フォルダ自身の中への貼り付けはできません。");
                }
            }
            const incoming = source.entries.map((entry) => entry.path.split("/").at(-1)!);
            const collisions = await findCollisions(incoming, destination, signal);
            signal.throwIfAborted();
            const duplicate =
                source.mode === "copy" &&
                source.libraryId === destination.libraryId &&
                source.entries.some(
                    (entry) => childPath(destination.path, entry.path.split("/").at(-1)!) === entry.path,
                );
            const policy =
                collisions.length || duplicate
                    ? await confirm(collisions.length ? collisions : incoming, duplicate)
                    : "fail";
            if (policy === "cancel" || signal.aborted || !active.current) return;
            const result = await transferSelection(client, source, destination, policy, (accepted) => {
                if (source.mode === "cut") clipboard.store?.removeAccepted(source, accepted.acceptedPaths);
                if (accepted.operations.length) activity?.show(accepted.operations);
            });
            if (result.entries.length || result.skippedCount || (!result.operations.length && !result.failures.length))
                toast.success(
                    result.skippedCount
                        ? `${result.skippedCount} 件をスキップしました。`
                        : source.mode === "cut"
                          ? "移動しました。"
                          : "コピーしました。",
                );
            if (result.failures.length)
                toast.error("送れない項目があります。", {
                    description: result.failures
                        .map((error) => (error instanceof Error ? error.message : String(error)))
                        .join("\n"),
                });
        } catch (error) {
            if (!signal.aborted)
                toast.error("項目を送れませんでした。", {
                    description: error instanceof Error ? error.message : undefined,
                });
        } finally {
            controller.abort();
            if (preflight.current === controller) preflight.current = undefined;
            lock.current = false;
            setBusy(false);
        }
    };
    const upload = async (inputs: readonly UploadInput[], destination: TransferDestination) => {
        if (lock.current || narrow || !inputs.length || !uploads.store) return;
        lock.current = true;
        setBusy(true);
        const controller = new AbortController();
        preflight.current = controller;
        const signal = controller.signal;
        try {
            const libraries = await client.ensureQueryData(librariesQuery);
            const library = libraries.find((item) => item.id === destination.libraryId);
            const folder = await client.fetchQuery({
                ...folderQuery(destination.libraryId, destination.path),
                staleTime: 0,
            });
            signal.throwIfAborted();
            if (!library || !canMutate(library, folder)) throw new Error("このフォルダへは送れません。");
            const roots = [...new Set(inputs.map((input) => input.path.split("/")[0]!))];
            const policy = await resolveConflict(roots, destination, confirm, signal);
            if (policy === "cancel" || signal.aborted || !active.current) return;
            uploads.store.enqueue(inputs, destination, policy);
            activity?.show();
        } catch (error) {
            if (!signal.aborted)
                toast.error("アップロードを始められませんでした。", {
                    description: error instanceof Error ? error.message : undefined,
                });
        } finally {
            controller.abort();
            if (preflight.current === controller) preflight.current = undefined;
            lock.current = false;
            setBusy(false);
        }
    };
    return (
        <TransferContext.Provider
            value={{
                busy,
                canPaste: !!clipboard.contents && !busy && !narrow,
                transfer,
                upload,
                paste: async (destination) => {
                    if (clipboard.contents) await transfer(clipboard.contents, destination);
                },
            }}
        >
            {children}
            <CollisionDialog names={names} duplicateOnly={duplicateOnly} onChoice={choose} />
        </TransferContext.Provider>
    );
}
