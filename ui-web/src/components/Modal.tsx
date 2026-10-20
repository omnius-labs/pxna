import { type ReactNode, useEffect, useRef } from "react";

interface ModalProps {
    label: string | undefined;
    onClose: () => void;
    /// 子を包む外枠の要素を受け取る。確認のダイアログやメニューの Portal の描画先と、全画面にする要素に使う。
    /// `dialog` の要素は全画面にできず、`body` の直下へ描いた Portal はモーダルの背面になるので、どちらも外枠の要素にする。
    outerRef?: (element: HTMLDivElement | null) => void;
    children: ReactNode;
}

/// 外枠の中に Portal で描く、Radix の確認のダイアログとメニュー。
const NESTED_LAYER = "[role='alertdialog'], [role='menu']";

/** ビューアの背面を操作不能にし、閉じた後は開く前の位置へ戻す。 */
export function Modal({ label, onClose, outerRef, children }: ModalProps) {
    const ref = useRef<HTMLDialogElement>(null);
    useEffect(() => {
        const dialog = ref.current;
        if (!dialog) {
            return;
        }
        const previous = document.activeElement;
        dialog.showModal();
        dialog.focus();
        return () => {
            dialog.close();
            if (previous instanceof HTMLElement && previous.isConnected) {
                previous.focus();
            }
        };
    }, []);

    return (
        <dialog
            ref={ref}
            aria-label={label}
            tabIndex={-1}
            className="fixed inset-0 m-0 h-dvh max-h-none w-screen max-w-none border-0 bg-black/95 p-0 text-white outline-none backdrop:bg-black/95"
            onCancel={(event) => {
                event.preventDefault();
                onClose();
            }}
            onKeyDown={(event) => {
                // 外枠の中に描いた確認のダイアログとメニューの Tab は、それぞれのフォーカストラップに任せる。
                if (event.key !== "Tab" || (event.target instanceof Element && event.target.closest(NESTED_LAYER))) {
                    return;
                }
                event.preventDefault();
                const dialog = event.currentTarget;
                const targets = [
                    ...dialog.querySelectorAll<HTMLElement>(
                        "button:not(:disabled), a[href], input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex='0']",
                    ),
                ].filter(
                    // tabindex="-1" の要素は、フィルムストリップのように、Tab で止まらない部品の中の操作である。
                    (element) =>
                        element.tabIndex >= 0 &&
                        !element.closest("[hidden], [inert]") &&
                        element.getClientRects().length > 0,
                );
                const index = targets.indexOf(document.activeElement as HTMLElement);
                const next =
                    index < 0
                        ? event.shiftKey
                            ? targets.length - 1
                            : 0
                        : (index + (event.shiftKey ? -1 : 1) + targets.length) % targets.length;
                (targets[next] ?? dialog).focus();
            }}
        >
            <div ref={outerRef} className="flex h-full flex-col text-sm">
                {children}
            </div>
        </dialog>
    );
}
