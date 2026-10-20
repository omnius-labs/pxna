import type { EntryKind } from "../library/entries";

const PATHS: Record<EntryKind, string> = {
    folder: "M3 6a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z",
    archive: "M5 3h14v18H5zM11 3v2h2v2h-2v2h2v2h-2v2h2",
    image: "M4 5h16v14H4zM4 16l5-5 4 4 3-3 4 4M15 9.5a1.5 1.5 0 1 0 0-.01",
    video: "M4 6h12v12H4zM16 10l4-2v8l-4-2",
    other: "M6 3h8l4 4v14H6zM14 3v4h4",
};

/// タイルとツリーに出す、Entry の種類のアイコン。
export function EntryIcon({ kind, className }: { kind: EntryKind; className?: string }) {
    return (
        <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth={1.5}
            strokeLinejoin="round"
            aria-hidden="true"
            className={className}
        >
            <path d={PATHS[kind]} />
        </svg>
    );
}
