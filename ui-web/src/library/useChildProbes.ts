import { useQuery } from "@tanstack/react-query";
import { useEffect, useState, useSyncExternalStore } from "react";
import { ChildProbeQueue, type ChildProbeTarget } from "./childProbes";
import { capabilitiesQuery } from "./entries";

export function useChildProbes(wanted: string) {
    const capabilities = useQuery(capabilitiesQuery);
    const [queue] = useState(() => new ChildProbeQueue());
    const results = useSyncExternalStore(queue.subscribe, queue.snapshot);
    useEffect(() => {
        queue.setWanted(capabilities.data?.childProbes ? (JSON.parse(wanted) as ChildProbeTarget[]) : []);
    }, [queue, wanted, capabilities.data?.childProbes]);
    useEffect(() => () => queue.setWanted([]), [queue]);
    return results;
}
