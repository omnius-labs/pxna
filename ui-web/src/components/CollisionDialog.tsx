import { t } from "i18next";
import type { CollisionChoice } from "../library/collision";
import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
} from "./ui/alert-dialog";
export function CollisionDialog({
    names,
    duplicateOnly = false,
    onChoice,
}: {
    names?: string[];
    duplicateOnly?: boolean;
    onChoice(choice: CollisionChoice): void;
}) {
    if (!names) return null;
    return (
        <AlertDialog
            open
            onOpenChange={(open) => {
                if (!open) onChoice("cancel");
            }}
        >
            <AlertDialogContent>
                <AlertDialogHeader>
                    <AlertDialogTitle>{t("collision.title", { count: names.length })}</AlertDialogTitle>
                    <AlertDialogDescription>
                        {duplicateOnly ? t("collision.duplicateOnly") : t("collision.applyAll")}
                    </AlertDialogDescription>
                </AlertDialogHeader>
                <ul className="max-h-40 overflow-auto text-sm">
                    {names.slice(0, 10).map((name) => (
                        <li className="break-all" key={name}>
                            {name}
                        </li>
                    ))}
                    {names.length > 10 && <li>{t("collision.more", { count: names.length - 10 })}</li>}
                </ul>
                <AlertDialogFooter>
                    <AlertDialogCancel
                        onClick={(event) => {
                            event.preventDefault();
                            onChoice("cancel");
                        }}
                    >
                        {t("collision.abort")}
                    </AlertDialogCancel>
                    {!duplicateOnly && (
                        <AlertDialogAction
                            onClick={(event) => {
                                event.preventDefault();
                                onChoice("skip");
                            }}
                        >
                            {t("collision.skip")}
                        </AlertDialogAction>
                    )}
                    <AlertDialogAction
                        onClick={(event) => {
                            event.preventDefault();
                            onChoice("rename");
                        }}
                    >
                        {t("collision.keepBoth")}
                    </AlertDialogAction>
                    {!duplicateOnly && (
                        <AlertDialogAction
                            onClick={(event) => {
                                event.preventDefault();
                                onChoice("overwrite");
                            }}
                        >
                            {t("collision.replace")}
                        </AlertDialogAction>
                    )}
                </AlertDialogFooter>
            </AlertDialogContent>
        </AlertDialog>
    );
}
