import { useQuery } from "@tanstack/react-query";
import { type MouseEvent, useMemo } from "react";

import { librariesQuery } from "../library/queries";
import { timelineMonthsQuery } from "../library/timeline";
import { formatMonth } from "../library/timelineLayout";
import { timelineHref } from "../location";
import { Checkbox } from "./ui/checkbox";

interface TimelineSidebarProps {
    /// 並べている Library の id。空ではない。
    libraryIds: readonly string[];
    /// 並べる Library を変えたとき。最後の 1 つは外せないので、空の選択は渡らない。
    onChangeLibraries: (libraryIds: string[]) => void;
    /// 見ている月（`YYYY-MM`）。
    activeMonth: string | undefined;
    /// 月を押したとき。その月から読み直す。
    onSelectMonth: (month: string) => void;
}

/// タイムラインの左の領域。上に並べる Library の選択、下に年と月を、並べている Library の件数の和つきで新しい順に並べる。
export function TimelineSidebar({ libraryIds, onChangeLibraries, activeMonth, onSelectMonth }: TimelineSidebarProps) {
    const libraries = useQuery(librariesQuery);
    const months = useQuery(timelineMonthsQuery(libraryIds));
    const years = useMemo(() => {
        const byYear = new Map<string, { month: string; count: number }[]>();
        for (const item of months.data ?? []) {
            const year = item.month.slice(0, 4);
            byYear.set(year, [...(byYear.get(year) ?? []), item]);
        }
        return [...byYear].map(([year, items]) => ({
            year,
            items,
            count: items.reduce((sum, item) => sum + item.count, 0),
        }));
    }, [months.data]);
    const select = (event: MouseEvent<HTMLAnchorElement>, month: string) => {
        if (event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
        event.preventDefault();
        onSelectMonth(month);
    };
    return (
        <div className="flex flex-col gap-3 p-2 text-sm">
            <section aria-label="Library">
                <ul className="flex flex-col">
                    {(libraries.data ?? []).map((library) => {
                        const checked = libraryIds.includes(library.id);
                        const last = checked && libraryIds.length === 1;
                        return (
                            <li
                                key={library.id}
                                className={`flex items-center gap-2 rounded px-2 py-1 ${last ? "" : "hover:bg-accent/50"}`}
                                title={last ? "少なくとも 1 つの Library を並べます。" : undefined}
                            >
                                <Checkbox
                                    id={`timeline-library-${library.id}`}
                                    checked={checked}
                                    disabled={last}
                                    onCheckedChange={(next) => {
                                        const chosen = new Set(libraryIds);
                                        if (next === true) chosen.add(library.id);
                                        else chosen.delete(library.id);
                                        const ids = (libraries.data ?? [])
                                            .map((item) => item.id)
                                            .filter((id) => chosen.has(id));
                                        if (ids.length > 0) onChangeLibraries(ids);
                                    }}
                                />
                                <label
                                    htmlFor={`timeline-library-${library.id}`}
                                    className={`min-w-0 flex-1 truncate ${last ? "" : "cursor-pointer"}`}
                                >
                                    {library.name}
                                </label>
                            </li>
                        );
                    })}
                </ul>
            </section>
            <section aria-label="年と月">
                {months.isError ? (
                    <p className="px-2 text-muted-foreground">年と月を読めませんでした。</p>
                ) : months.data === undefined ? (
                    <p className="px-2 text-muted-foreground">読み込み中…</p>
                ) : years.length === 0 ? (
                    <p className="px-2 text-muted-foreground">画像と動画がありません。</p>
                ) : (
                    years.map(({ year, items, count }) => (
                        <div key={year} className="mb-2">
                            <p className="flex justify-between px-2 py-1 font-semibold">
                                <span>{year}年</span>
                                <span className="font-normal text-muted-foreground">{count}</span>
                            </p>
                            <ul>
                                {items.map((item) => (
                                    <li key={item.month}>
                                        <a
                                            href={timelineHref({ month: item.month })}
                                            aria-current={item.month === activeMonth ? "true" : undefined}
                                            title={formatMonth(item.month)}
                                            onClick={(event) => select(event, item.month)}
                                            className={`flex justify-between rounded px-2 py-1 pl-5 ${item.month === activeMonth ? "bg-accent font-medium text-accent-foreground" : "hover:bg-accent/50"}`}
                                        >
                                            <span>{Number(item.month.slice(5))}月</span>
                                            <span className="text-muted-foreground">{item.count}</span>
                                        </a>
                                    </li>
                                ))}
                            </ul>
                        </div>
                    ))
                )}
            </section>
        </div>
    );
}
