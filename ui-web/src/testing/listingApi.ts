// biome-ignore-all lint/suspicious/noExplicitAny: the answers are the untyped fixtures of each test
type Answer = (route: string, options: any) => any;

interface Place {
    libraryId: string;
    path: string;
}

const NO_WARNINGS = { cutOffStarts: [], walkedStarts: [], missingStarts: [] };

/// The path of `entry` below `start`, as the listings of one folder named an Entry to locate.
function below(start: string, entry: string): string {
    const base = start === "/" ? "" : start;
    return entry.startsWith(`${base}/`) ? entry.slice(base.length + 1) : entry;
}

function defined(values: Record<string, unknown>): Record<string, unknown> {
    return Object.fromEntries(Object.entries(values).filter(([, value]) => value !== undefined));
}

/// Lets a screen's test that answers listings in the shapes of the listing a screen reads — a folder, its summary, a search, the timeline and
/// its months, and the details of several Entries — answer `POST /entries`, `/entries/months`, and `/entries/metadata`.
/// The request is handed to `get` (or to `post` for the details) as the route and the query of the listing it stands for, and the answer is
/// turned into the response of the endpoint, so the screens' tests keep their fixtures. A request that names `includeHidden` is a folder's,
/// and one that does not is a search's, as `library/entries.ts` builds them; the requests themselves are checked by the tests of
/// `library/entries.ts` and `library/timeline.ts`. This goes when the screens are rebuilt on `listEntries` (M30).
export function answerEntries(get: Answer, post: Answer = get): Answer {
    return async (route: string, options: { body: any; signal?: AbortSignal }) => {
        const { body, signal } = options;
        if (route === "/entries/metadata") {
            const libraryId = body.places[0]?.libraryId ?? "";
            const result = await post("/libraries/{libraryId}/entries/details", {
                params: { path: { libraryId } },
                body: { paths: body.places.map((place: Place) => place.path) },
                signal,
            });
            if (!result?.data) return result;
            return { ...result, data: { items: result.data.results.map((item: any) => ({ libraryId, ...item })) } };
        }
        const starts: Place[] = body.scope.starts;
        const libraryIds = starts.map((start) => start.libraryId);
        if (route === "/entries/months") {
            return get("/timeline/months", { params: { query: { libraryIds } }, signal });
        }
        const page = body.page ?? {};
        if (body.sort === "timeline_date") {
            const result = await get("/timeline/entries", {
                params: {
                    query: defined({
                        libraryIds,
                        before: page.before,
                        after: page.after,
                        month: page.month,
                        around: page.around?.path,
                        aroundLibraryId: page.around?.libraryId,
                        limit: page.limit,
                    }),
                },
                signal,
            });
            if (!result?.data) return result;
            const { entries, previous, next } = result.data;
            return {
                ...result,
                data: { items: entries, page: defined({ kind: "cursor", previous, next }), warnings: NO_WARNINGS },
            };
        }
        const [start] = starts as [Place];
        const position =
            page.locate === undefined
                ? { offset: page.offset }
                : defined({
                      locate: below(start.path, page.locate.path),
                      ...(libraryIds.length > 1 ? { locateLibraryId: page.locate.libraryId } : {}),
                  });
        const filter = body.filter ?? {};
        const foldersOnly = filter.kinds?.length === 2 && filter.kinds.includes("folder") ? true : undefined;
        const options_ = body.options ?? {};
        const offsetPage = (data: any) =>
            defined({ kind: "offset", offset: data.offset, totalCount: data.totalCount, snapshot: data.snapshot });
        if (options_.fields === "summary") {
            const result = await get("/libraries/{libraryId}/folders/summaries", {
                params: {
                    path: { libraryId: start.libraryId },
                    query: defined({
                        path: start.path,
                        ...position,
                        limit: page.limit,
                        query: filter.query,
                        sort: body.sort,
                        order: body.order,
                        includeHidden: options_.includeHidden,
                        foldersFirst: options_.foldersFirst,
                        foldersOnly,
                    }),
                },
                signal,
            });
            if (!result?.data) return result;
            return {
                ...result,
                data: {
                    summaries: result.data.entries.map((summary: any) => ({ libraryId: start.libraryId, summary })),
                    page: offsetPage(result.data),
                    warnings: NO_WARNINGS,
                },
            };
        }
        if (!("includeHidden" in options_)) {
            const located = page.locate === undefined ? {} : { locateLibraryId: page.locate.libraryId };
            const result = await get("/search/entries", {
                params: {
                    query: defined({
                        libraryIds,
                        ...(start.path === "/" ? {} : { path: start.path }),
                        recursive: body.scope.recursive,
                        query: filter.query,
                        rating: filter.rating,
                        tag: filter.tags,
                        sort: body.sort,
                        order: body.order,
                        foldersFirst: options_.foldersFirst,
                        ...position,
                        ...located,
                        limit: page.limit,
                    }),
                },
                signal,
            });
            if (!result?.data) return result;
            const data = result.data;
            return {
                ...result,
                data: {
                    items: data.entries,
                    page: offsetPage(data),
                    warnings: {
                        cutOffStarts: starts.filter((place) =>
                            (data.truncatedLibraryIds ?? []).includes(place.libraryId),
                        ),
                        walkedStarts: data.indexed === false ? starts : [],
                        missingStarts: [],
                    },
                },
            };
        }
        const result = await get("/libraries/{libraryId}/folders/entries", {
            params: {
                path: { libraryId: start.libraryId },
                query: defined({
                    path: start.path,
                    ...position,
                    limit: page.limit,
                    recursive: body.scope.recursive || undefined,
                    query: filter.query,
                    sort: body.sort,
                    order: body.order,
                    includeHidden: options_.includeHidden || undefined,
                    foldersFirst: options_.foldersFirst || undefined,
                    foldersOnly,
                }),
            },
            signal,
        });
        if (!result?.data) return result;
        const data = result.data;
        return {
            ...result,
            data: {
                items: data.entries.map((entry: any) => ({ libraryId: start.libraryId, entry })),
                page: offsetPage(data),
                warnings: { ...NO_WARNINGS, cutOffStarts: data.truncated ? starts : [] },
            },
        };
    };
}
