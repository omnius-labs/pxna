// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import { LAST_FOLDER_KEY } from "../library/folderReturn";
import { librariesQuery } from "../library/queries";
import { rememberSearch } from "../library/searchReturn";
import { ModeSwitch } from "./ModeSwitch";

const navigate = vi.hoisted(() => vi.fn());
vi.mock("@tanstack/react-router", () => ({ useNavigate: () => navigate }));

const host = document.createElement("div");
const root = createRoot(host);
let client: QueryClient;

beforeEach(() => {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    window.localStorage.clear();
    document.body.append(host);
    client = new QueryClient();
    client.setQueryData(librariesQuery.queryKey, [{ id: "photos", name: "Photos", readonly: false }]);
    navigate.mockReset();
});
afterEach(async () => {
    await act(async () => root.render(null));
    client.clear();
    host.remove();
});

async function render(mode: "folder" | "timeline" | "search", compact = false) {
    await act(async () =>
        root.render(
            <QueryClientProvider client={client}>
                <ModeSwitch libraryId={undefined} mode={mode} compact={compact} />
            </QueryClientProvider>,
        ),
    );
}
const links = () => [...host.querySelectorAll("a")];

it("フォルダ、タイムライン、検索を横に並べ、今の表示だけを強調する", async () => {
    for (const [mode, label] of [
        ["folder", "フォルダ"],
        ["timeline", "タイムライン"],
        ["search", "検索"],
    ] as const) {
        await render(mode);
        expect(links().map((link) => link.textContent)).toEqual(["フォルダ", "タイムライン", "検索"]);
        expect(
            links()
                .filter((link) => link.getAttribute("aria-current") === "page")
                .map((link) => link.title),
        ).toEqual([label]);
    }
});

it("狭い画面ではアイコンだけにし、名前を aria-label に持つ", async () => {
    await render("search", true);
    expect(links().map((link) => [link.textContent, link.getAttribute("aria-label")])).toEqual([
        ["", "フォルダ"],
        ["", "タイムライン"],
        ["", "検索"],
    ]);
});

it("検索は、最初は条件のない検索へ、使った後は最後の条件へ移る", async () => {
    await render("folder");
    const search = () => links().find((link) => link.title === "検索")!;
    rememberSearch({ path: "/", tags: [] });
    await render("timeline");
    expect(search().getAttribute("href")).toBe("/search");
    rememberSearch({ libraryIds: ["photos"], path: "/a", query: "cat", flat: true, rating: "good", tags: ["x", "y"] });
    await render("folder");
    expect(search().getAttribute("href")).toBe("/search?lib=photos&path=%2Fa&flat=1&q=cat&rating=good&tag=x&tag=y");
    await act(async () =>
        search().dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true, button: 0 })),
    );
    expect(navigate).toHaveBeenCalledWith({
        href: "/search?lib=photos&path=%2Fa&flat=1&q=cat&rating=good&tag=x&tag=y",
    });
});

it("フォルダは最後に開いたフォルダへ、なければ先頭の Library のルートへ移る", async () => {
    await render("search");
    const folder = () => links().find((link) => link.title === "フォルダ")!;
    expect(folder().getAttribute("href")).toMatch(/^\/browse\/(photos|photos\/.+)$/);
    window.localStorage.setItem(LAST_FOLDER_KEY, JSON.stringify({ libraryId: "photos", path: "/a/b" }));
    await render("search");
    expect(folder().getAttribute("href")).toBe("/browse/photos/a/b");
});
