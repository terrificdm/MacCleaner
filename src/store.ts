import { create } from "zustand";
import { api, errorCode } from "@/lib/api";
import type { CleanResult, Item, Perms, ScanModule, Settings } from "@/lib/types";

export type PageId =
  | "smart" | "space" | "system" | "dev" | "large" | "apps" | "leftovers" | "trash" | "history" | "settings";

export type ScanState =
  | { status: "idle" }
  | { status: "scanning" }
  | { status: "done"; items: Item[]; at: number }
  | { status: "error"; code: string };

interface State {
  page: PageId;
  settings: Settings | null;
  perms: Perms;
  /** The install-to-Applications prompt is showing (onboarding waits). */
  installPrompt: boolean;
  scans: Record<ScanModule, ScanState>;
  /** User's checkbox state per module, keyed by path. */
  selection: Record<ScanModule, Record<string, boolean>>;
  go(p: PageId): void;
  loadSettings(): Promise<void>;
  updateSettings(patch: Partial<Settings>): Promise<void>;
  refreshPerms(): Promise<Perms>;
  runScan(m: ScanModule): Promise<Item[] | null>;
  setSelected(m: ScanModule, paths: string[], value: boolean): void;
  /** Drop cleaned items from a module's results. */
  removeItems(m: ScanModule, paths: string[]): void;
  /** Drop cleaned paths from every module's results (modules can overlap). */
  removeEverywhere(paths: string[]): void;
  /** After something really moved (clean, uninstall, put back, empty): refresh
   *  the Trash scan so its list and totals aren't stale. */
  afterChange(r: CleanResult): void;
}

const idle: ScanState = { status: "idle" };

export const useStore = create<State>((set, get) => ({
  page: "smart",
  settings: null,
  perms: { fullDiskAccess: true },
  installPrompt: false,
  scans: { system: idle, dev: idle, large: idle, leftovers: idle, trash: idle, downloads: idle },
  selection: { system: {}, dev: {}, large: {}, leftovers: {}, trash: {}, downloads: {} },

  go: (page) => set({ page }),

  async loadSettings() {
    const settings = await api().getSettings();
    set({ settings });
  },

  async updateSettings(patch) {
    const cur = get().settings;
    if (!cur) return;
    const optimistic = { ...cur, ...patch };
    set({ settings: optimistic });
    const saved = await api().saveSettings(optimistic);
    set({ settings: saved });
  },

  async refreshPerms() {
    const perms = await api().permissions();
    set({ perms });
    return perms;
  },

  async runScan(m) {
    set((s) => ({ scans: { ...s.scans, [m]: { status: "scanning" } } }));
    try {
      const items = await api().scan(m);
      const sel: Record<string, boolean> = {};
      for (const i of items) sel[i.path] = i.selected;
      set((s) => ({
        scans: { ...s.scans, [m]: { status: "done", items, at: Date.now() } },
        selection: { ...s.selection, [m]: sel },
      }));
      return items;
    } catch (e) {
      const code = errorCode(e);
      set((s) => ({ scans: { ...s.scans, [m]: code === "cancelled" ? idle : { status: "error", code } } }));
      return null;
    }
  },

  setSelected(m, paths, value) {
    set((s) => {
      const next = { ...s.selection[m] };
      for (const p of paths) next[p] = value;
      return { selection: { ...s.selection, [m]: next } };
    });
  },

  afterChange(r) {
    if (r.dryRun || r.done.length === 0) return;
    if (get().scans.trash.status === "done") get().runScan("trash");
  },

  removeEverywhere(paths) {
    const gone = new Set(paths);
    set((s) => {
      const scans = { ...s.scans };
      for (const k of Object.keys(scans) as ScanModule[]) {
        const st = scans[k];
        if (st.status === "done") scans[k] = { ...st, items: st.items.filter((i) => !gone.has(i.path)) };
      }
      return { scans };
    });
  },

  removeItems(m, paths) {
    const gone = new Set(paths);
    set((s) => {
      const st = s.scans[m];
      if (st.status !== "done") return {};
      return { scans: { ...s.scans, [m]: { ...st, items: st.items.filter((i) => !gone.has(i.path)) } } };
    });
  },
}));

export function selectedItems(m: ScanModule, items: Item[]): Item[] {
  const sel = useStore.getState().selection[m];
  return items.filter((i) => sel[i.path] ?? i.selected);
}
