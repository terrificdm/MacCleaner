import { describe, expect, it } from "vitest";
import { uniqueByPath } from "./select";

describe("uniqueByPath", () => {
  it("keeps the first item for each path and preserves order", () => {
    const items = [
      { path: "/a", group: "system" },
      { path: "/b", group: "system" },
      { path: "/a", group: "leftovers" },
      { path: "/c", group: "leftovers" },
    ];
    expect(uniqueByPath(items)).toEqual([
      { path: "/a", group: "system" },
      { path: "/b", group: "system" },
      { path: "/c", group: "leftovers" },
    ]);
  });
});
