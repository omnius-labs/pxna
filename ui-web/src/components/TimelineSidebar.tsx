import { useQuery } from "@tanstack/react-query";
import { type MouseEvent, useMemo, useState } from "react";

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

/// 行の見た目。フォルダのツリーと同じ高さで、開閉の印（またはチェックボックス）の列を左端の `pl-1` に置く。
const ROW =
    "flex h-8 items-center gap-1 pr-2 outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring";
/// 月の行の左の余白。フォルダのツリーの 1 段分（0.875rem）だけ年より下げる。
const MONTH_INDENT = "pl-[1.125rem]";

/// タイムラインの左の領域。上に並べる Library の選択、下に年と月を、並べている Library の件数の和つきで新しい順に並べる。
/// 年と月はフォルダのツリーと同じ形で、Library の名前、年、月の文字の左端はツリーの段に揃う。
/// 年は開閉でき、最初は見ている月の年（なければ最新の年）だけを開く。見ている月が別の年へ移ると、その年も開く。
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
    const autoYear = activeMonth?.slice(0, 4) ?? years[0]?.year;
    // 開いている年と、最後に自動で開いた年。自動で開く年が変わったときだけ開くので、見ている年を閉じても開き直さない。
    const [expanded, setExpanded] = useState<{ years: ReadonlySet<string>; auto: string | undefined }>({
        years: new Set(),
        auto: undefined,
    });
    if (autoYear !== undefined && autoYear !== expanded.auto) {
        setExpanded({ years: new Set([...expanded.years, autoYear]), auto: autoYear });
    }
    const toggleYear = (year: string) =>
        setExpanded((current) => {
            const next = new Set(current.years);
            if (!next.delete(year)) next.add(year);
            return { ...current, years: next };
        });
    const select = (event: MouseEvent<HTMLAnchorElement>, month: string) => {
        if (event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
        event.preventDefault();
        onSelectMonth(month);
    };
    return (
        <div className="flex flex-col gap-3 py-2 text-sm">
            <section aria-labelledby="timeline-libraries-heading">
                <h2 id="timeline-libraries-heading" className="px-1 py-1 text-xs font-semibold text-muted-foreground">
                    Library
                </h2>
                <ul className="flex flex-col">
                    {(libraries.data ?? []).map((library) => {
                        const checked = libraryIds.includes(library.id);
                        const last = checked && libraryIds.length === 1;
                        return (
                            <li
                                key={library.id}
                                className={`${ROW} pl-1 ${last ? "" : "hover:bg-sidebar-accent"}`}
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
            <section aria-labelledby="timeline-months-heading">
                <h2 id="timeline-months-heading" className="px-1 py-1 text-xs font-semibold text-muted-foreground">
                    年と月
                </h2>
                {months.isError ? (
                    <p className="px-1 text-muted-foreground">年と月を読めませんでした。</p>
                ) : months.data === undefined ? (
                    <p className="px-1 text-muted-foreground">読み込み中…</p>
                ) : years.length === 0 ? (
                    <p className="px-1 text-muted-foreground">画像と動画がありません。</p>
                ) : (
                    <ul>
                        {years.map(({ year, items, count }) => {
                            const open = expanded.years.has(year);
                            return (
                                <li key={year}>
                                    <button
                                        type="button"
                                        aria-expanded={open}
                                        aria-controls={`timeline-year-${year}`}
                                        onClick={() => toggleYear(year)}
                                        className={`${ROW} w-full pl-1 text-left font-semibold hover:bg-sidebar-accent`}
                                    >
                                        <span
                                            aria-hidden="true"
                                            className="inline-block w-4 shrink-0 text-center font-normal text-muted-foreground"
                                        >
                                            {open ? "▾" : "▸"}
                                        </span>
                                        <span className="min-w-0 flex-1 truncate">{year}年</span>
                                        <span className="font-normal tabular-nums text-muted-foreground">{count}</span>
                                    </button>
                                    {open && (
                                        <ul id={`timeline-year-${year}`}>
                                            {items.map((item) => (
                                                <li key={item.month}>
                                                    <a
                                                        href={timelineHref({ month: item.month })}
                                                        aria-current={item.month === activeMonth ? "true" : undefined}
                                                        title={formatMonth(item.month)}
                                                        onClick={(event) => select(event, item.month)}
                                                        className={`${ROW} ${MONTH_INDENT} ${item.month === activeMonth ? "bg-sidebar-accent font-medium text-sidebar-accent-foreground" : "hover:bg-sidebar-accent"}`}
                                                    >
                                                        <span
                                                            aria-hidden="true"
                                                            className="inline-block w-4 shrink-0"
                                                        />
                                                        <span className="min-w-0 flex-1 truncate">
                                                            {Number(item.month.slice(5))}月
                                                        </span>
                                                        <span className="tabular-nums text-muted-foreground">
                                                            {item.count}
                                                        </span>
                                                    </a>
                                                </li>
                                            ))}
                                        </ul>
                                    )}
                                </li>
                            );
                        })}
                    </ul>
                )}
            </section>
        </div>
    );
}
