import { t } from "i18next";
import { FolderUp, Upload } from "lucide-react";
import { useRef } from "react";
import { useEntryActions } from "../library/actionContext";
import { useTransfer } from "../library/transfer";
import { filesToInputs } from "../library/upload";
export function UploadButton() {
    const actions = useEntryActions();
    const transfer = useTransfer();
    const files = useRef<HTMLInputElement>(null);
    const folders = useRef<HTMLInputElement>(null);
    if (!actions || actions.narrow) return null;
    const disabled = !actions.canCreate || !!transfer?.busy;
    const selected = (input: HTMLInputElement) => {
        const entries = Array.from(input.files ?? []);
        input.value = "";
        void transfer?.upload?.(filesToInputs(entries), {
            libraryId: actions.libraryId,
            path: actions.folderPath ?? "/",
        });
    };
    return (
        <div className="flex items-center gap-1">
            <button
                type="button"
                aria-label={t("upload.file")}
                title={t("upload.file")}
                disabled={disabled}
                onClick={() => files.current?.click()}
                className="rounded p-1.5 disabled:opacity-40"
            >
                <Upload className="size-4" />
            </button>
            <button
                type="button"
                aria-label={t("upload.folder")}
                title={t("upload.folder")}
                disabled={disabled}
                onClick={() => folders.current?.click()}
                className="rounded p-1.5 disabled:opacity-40"
            >
                <FolderUp className="size-4" />
            </button>
            <input
                ref={files}
                type="file"
                aria-label={t("upload.fileInput")}
                multiple
                hidden
                disabled={disabled}
                onChange={(event) => selected(event.currentTarget)}
            />
            <input
                ref={folders}
                type="file"
                aria-label={t("upload.folderInput")}
                multiple
                hidden
                {...{ webkitdirectory: "" }}
                disabled={disabled}
                onChange={(event) => selected(event.currentTarget)}
            />
        </div>
    );
}
