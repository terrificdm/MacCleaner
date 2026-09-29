import { beforeEach, describe, expect, it, vi } from "vitest";
import { useStore } from "./store";
import type { CleanResult, Item, ScanModule } from "./lib/types";

const result = (dryRun: boolean, done: number): CleanResult => ({
  dryRun,
  total: 0,
  failed: [],
  done: Array.from({ length: done }, (_, i) => ({ path: `/p${i}`, size: 1, method: dryRun ? "dryRun" : "trash" })),
});

describe("afterChange refreshes the Trash scan", () => {
  let runScan: ReturnType<typeof vi.fn<(m: ScanModule) => Promise<Item[] | null>>>;
  beforeEach(() => {
    runScan = vi.fn<(m: ScanModule) => Promise<Item[] | null>>().mockResolvedValue([]);
    useStore.setState((s) => ({ runScan, scans: { ...s.scans, trash: { status: "done", items: [], at: 0 } } }));
  });

  it("rescans the Trash after a real clean that moved something", () => {
    useStore.getState().afterChange(result(false, 3));
    expect(runScan).toHaveBeenCalledWith("trash");
  });

  it("does nothing after a dry run or when nothing moved", () => {
    useStore.getState().afterChange(result(true, 3));
    useStore.getState().afterChange(result(false, 0));
    expect(runScan).not.toHaveBeenCalled();
  });

  it("does not start a Trash scan the user never ran", () => {
    useStore.setState((s) => ({ scans: { ...s.scans, trash: { status: "idle" } } }));
    useStore.getState().afterChange(result(false, 3));
    expect(runScan).not.toHaveBeenCalled();
  });
});
