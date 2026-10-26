import { QueryClient } from "@tanstack/react-query";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import { ClientTimings } from "./clientTimings";

const mocks = vi.hoisted(() => ({ get: vi.fn(), post: vi.fn() }));

vi.mock("./client", async (importOriginal) => ({
    ...(await importOriginal<typeof import("./client")>()),
    api: { GET: mocks.get, POST: mocks.post },
}));

function capabilities(performanceLogging: boolean) {
    return {
        data: { summaryListing: true, entryDetails: true, childProbes: true, performanceLogging },
        response: new Response(null, { status: 200 }),
    };
}

beforeEach(() => {
    vi.useFakeTimers();
    mocks.get.mockReset();
    mocks.post.mockReset();
    mocks.post.mockResolvedValue({ response: new Response(null, { status: 204 }) });
});

afterEach(() => {
    vi.useRealTimers();
});

it("sends the recorded timings together when the daemon logs performance", async () => {
    mocks.get.mockResolvedValue(capabilities(true));
    const timings = new ClientTimings();
    timings.attach(new QueryClient());

    timings.record({ operation: "viewer_display", durationMs: 120 });
    timings.record({ operation: "display_image", path: "/a.jpg", durationMs: 100 });
    await vi.runAllTimersAsync();

    expect(mocks.post).toHaveBeenCalledTimes(1);
    expect(mocks.post.mock.calls[0]?.[1]).toMatchObject({
        body: { timings: [{ operation: "viewer_display" }, { operation: "display_image", path: "/a.jpg" }] },
        keepalive: true,
    });
});

it("drops the timings when the daemon does not log performance", async () => {
    mocks.get.mockResolvedValue(capabilities(false));
    const timings = new ClientTimings();
    timings.attach(new QueryClient());

    timings.record({ operation: "viewer_display", durationMs: 120 });
    await vi.runAllTimersAsync();
    timings.record({ operation: "viewer_display", durationMs: 80 });
    timings.flush();

    expect(mocks.post).not.toHaveBeenCalled();
    expect(mocks.get).toHaveBeenCalledTimes(1);
});

it("splits a large number of timings into batches of the API limit", async () => {
    mocks.get.mockResolvedValue(capabilities(true));
    const timings = new ClientTimings();
    timings.attach(new QueryClient());

    for (let index = 0; index < 250; index++) {
        timings.record({ operation: "thumbnail_image", durationMs: index });
    }
    await vi.runAllTimersAsync();

    expect(mocks.post.mock.calls.map((call) => call[1].body.timings.length)).toEqual([200, 50]);
});
