// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";
import { CollisionDialog } from "./CollisionDialog";

it.each([
    ["置き換える", "overwrite"],
    ["両方残す", "rename"],
    ["スキップ", "skip"],
    ["中止", "cancel"],
])("衝突の件数と名前を表示し、%s を返す", async (label, choice) => {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    const selected = vi.fn();
    try {
        await act(async () =>
            root.render(
                <CollisionDialog names={Array.from({ length: 12 }, (_, i) => `name-${i}`)} onChoice={selected} />,
            ),
        );
        expect(document.querySelector('[role="alertdialog"]')?.textContent).toContain("12 件");
        expect(document.body.textContent).toContain("name-0");
        expect(document.body.textContent).toContain("ほか 2 件");
        await act(async () =>
            [...document.querySelectorAll<HTMLButtonElement>("button")]
                .find((button) => button.textContent === label)!
                .click(),
        );
        expect(selected).toHaveBeenCalledExactlyOnceWith(choice);
    } finally {
        await act(async () => root.unmount());
        host.remove();
    }
});
