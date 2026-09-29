import { describe, expect, it } from "vitest";
import { useStore } from "./store";
import type { Item } from "./lib/types";

const item = (path: string): Item => ({ path, name: path, size: 1, group: "g", kind: "dir", selected: true });

describe("removeEverywhere", () => {
  it("drops cleaned paths from every module's results", () => {
    const done = (items: Item[]) => ({ status: "done" as const, items, at: 0 });
    useStore.setState((s) => ({
      scans: { ...s.scans, system: done([item("/c1"), item("/c2")]), leftovers: done([item("/c1"), item("/x")]) },
    }));
    useStore.getState().removeEverywhere(["/c1"]);
    const st = useStore.getState().scans;
    expect(st.system.status === "done" && st.system.items.map((i) => i.path)).toEqual(["/c2"]);
    expect(st.leftovers.status === "done" && st.leftovers.items.map((i) => i.path)).toEqual(["/x"]);
  });
});
