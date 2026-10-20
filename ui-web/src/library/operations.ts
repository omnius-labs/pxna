import { queryOptions, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef } from "react";
import { api, unwrap } from "../api/client";
import type { Operation } from "./entryActions";
import { refreshLibrary } from "./listing";

export function isRunning(operation: Operation): boolean {
    return operation.status === "pending" || operation.status === "running";
}
export const operationsQuery = queryOptions({
    queryKey: ["operations"],
    queryFn: async ({ signal }) => unwrap(await api.GET("/operations", { signal })).operations,
    retry: false,
    refetchOnWindowFocus: false,
    refetchInterval: (query) => (query.state.data?.some(isRunning) ? 1000 : false),
    refetchIntervalInBackground: true,
});
export async function cancelOperation(id: string): Promise<Operation> {
    return unwrap(await api.POST("/operations/{operationId}/cancel", { params: { path: { operationId: id } } }))
        .operation;
}
export function useOperations() {
    const client = useQueryClient();
    const query = useQuery(operationsQuery);
    const seen = useRef(new Set<string>());
    useEffect(() => {
        for (const operation of query.data ?? []) {
            if (isRunning(operation) || seen.current.has(operation.id)) continue;
            seen.current.add(operation.id);
            const libraries = new Set(
                [operation.libraryId, operation.destinationLibraryId].filter((id): id is string => id !== undefined),
            );
            for (const id of libraries) void refreshLibrary(client, id);
        }
    }, [client, query.data]);
    return query;
}
