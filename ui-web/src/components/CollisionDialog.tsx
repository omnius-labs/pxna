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
                    <AlertDialogTitle>{names.length} 件の名前が衝突しています</AlertDialogTitle>
                    <AlertDialogDescription>
                        {duplicateOnly
                            ? "元の項目を残し、別名で複製します。"
                            : "選んだ方法を今回のすべての項目に適用します。フォルダの置き換えでは中身を統合し、同名ファイルを上書きします。"}
                    </AlertDialogDescription>
                </AlertDialogHeader>
                <ul className="max-h-40 overflow-auto text-sm">
                    {names.slice(0, 10).map((name) => (
                        <li className="break-all" key={name}>
                            {name}
                        </li>
                    ))}
                    {names.length > 10 && <li>ほか {names.length - 10} 件</li>}
                </ul>
                <AlertDialogFooter>
                    <AlertDialogCancel
                        onClick={(event) => {
                            event.preventDefault();
                            onChoice("cancel");
                        }}
                    >
                        中止
                    </AlertDialogCancel>
                    {!duplicateOnly && (
                        <AlertDialogAction
                            onClick={(event) => {
                                event.preventDefault();
                                onChoice("skip");
                            }}
                        >
                            スキップ
                        </AlertDialogAction>
                    )}
                    <AlertDialogAction
                        onClick={(event) => {
                            event.preventDefault();
                            onChoice("rename");
                        }}
                    >
                        両方残す
                    </AlertDialogAction>
                    {!duplicateOnly && (
                        <AlertDialogAction
                            onClick={(event) => {
                                event.preventDefault();
                                onChoice("overwrite");
                            }}
                        >
                            置き換える
                        </AlertDialogAction>
                    )}
                </AlertDialogFooter>
            </AlertDialogContent>
        </AlertDialog>
    );
}
