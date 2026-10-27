// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import { librariesQuery } from "../library/queries";
import { timelineMonthsQuery } from "../library/timeline";
import { TimelineSidebar } from "./TimelineSidebar";

const host = document.createElement("div");
const root = createRoot(host);
let client: QueryClient;
const onSelectMonth = vi.fn();

beforeEach(() => {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    document.body.append(host);
    client = new QueryClient({ defaultOptions: { queries: { staleTime: Infinity } } });
    client.setQueryData(librariesQuery.queryKey, [
        { id: "photos", name: "Photos", readonly: false },
        { id: "phone", name: "Phone", readonly: false },
    ]);
    client.setQueryData(timelineMonthsQuery(["photos"]).queryKey, [
        { month: "2025-12", count: 3 },
        { month: "2025-11", count: 2 },
        { month: "2024-05", count: 7 },
    ]);
    onSelectMonth.mockReset();
});
afterEach(async () => {
    await act(async () => root.render(null));
    client.clear();
    host.remove();
});

async function render(activeMonth: string | undefined) {
    await act(async () =>
        root.render(
            <QueryClientProvider client={client}>
                <TimelineSidebar
                    libraryIds={["photos"]}
                    onChangeLibraries={() => {}}
                    activeMonth={activeMonth}
                    onSelectMonth={onSelectMonth}
                />
            </QueryClientProvider>,
        ),
    );
}
const headings = () => [...host.querySelectorAll("h2")].map((heading) => heading.textContent);
const yearButton = (year: string) =>
    [...host.querySelectorAll("button[aria-expanded]")].find((button) => button.textContent?.includes(`${year}`))!;
const monthLabels = () => [...host.querySelectorAll("a")].map((link) => link.title);

it("Library と年と月に見出しを付ける", async () => {
    await render(undefined);
    expect(headings()).toEqual(["Library", "Years and months"]);
});

it("最初は見ている月の年だけを開き、年を押すと開け閉めする", async () => {
    await render("2024-05");
    expect(yearButton("2024").getAttribute("aria-expanded")).toBe("true");
    expect(yearButton("2025").getAttribute("aria-expanded")).toBe("false");
    expect(yearButton("2025").textContent).toContain("5");
    expect(monthLabels()).toEqual(["May 2024"]);

    await act(async () => (yearButton("2025") as HTMLButtonElement).click());
    expect(monthLabels()).toEqual(["Dec 2025", "Nov 2025", "May 2024"]);

    await act(async () => (yearButton("2024") as HTMLButtonElement).click());
    expect(monthLabels()).toEqual(["Dec 2025", "Nov 2025"]);
});

it("見ている月がなければ最新の年を開き、見ている月が別の年へ移ればその年も開く", async () => {
    await render(undefined);
    expect(monthLabels()).toEqual(["Dec 2025", "Nov 2025"]);

    await render("2024-05");
    expect(monthLabels()).toEqual(["Dec 2025", "Nov 2025", "May 2024"]);
    expect(host.querySelector("a[aria-current=true]")?.getAttribute("title")).toBe("May 2024");
});

it("見ている年を閉じても、同じ年の中で月が移るだけなら開き直さない", async () => {
    await render("2025-12");
    await act(async () => (yearButton("2025") as HTMLButtonElement).click());
    expect(monthLabels()).toEqual([]);

    await render("2025-11");
    expect(monthLabels()).toEqual([]);
});
