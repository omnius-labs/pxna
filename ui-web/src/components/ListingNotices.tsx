import { t } from "i18next";

interface ListingNoticesProps {
    /// The result is cut off, so part of it is not shown.
    truncated: boolean;
    /// The search is one the name index answers, but the server walked, so the result may be incomplete.
    indexing: boolean;
}

/// The notices above a listing's results.
export function ListingNotices({ truncated, indexing }: ListingNoticesProps) {
    return (
        <>
            {indexing && (
                <p role="status" className="shrink-0 border-b p-3 text-sm text-muted-foreground">
                    {t("common.indexBuilding")}
                </p>
            )}
            {truncated && (
                <p role="status" className="shrink-0 border-b p-3 text-sm text-muted-foreground">
                    {t("common.partialResults")}
                </p>
            )}
        </>
    );
}
