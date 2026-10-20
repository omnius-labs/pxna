import { expect, it } from "vitest";

import { sortFromHeader } from "./sort";

it("starts another sort key in ascending order", () => {
    expect(sortFromHeader({ key: "name", order: "desc" }, "modified_at")).toEqual({ key: "modified_at", order: "asc" });
});
it("reverses the current key in both directions", () => {
    expect(sortFromHeader({ key: "size", order: "asc" }, "size")).toEqual({ key: "size", order: "desc" });
    expect(sortFromHeader({ key: "size", order: "desc" }, "size")).toEqual({ key: "size", order: "asc" });
});
