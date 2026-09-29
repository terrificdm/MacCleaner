import type {
  AppInfo, InstallStatus, CleanResult, CleanTarget, DiskProgress, DiskSummary, HistoryRecord, Item, Perms,
  ScanModule, Settings, UninstallRequest, ViewNode, Volume,
} from "./types";

export interface Api {
  getSettings(): Promise<Settings>;
  saveSettings(s: Settings): Promise<Settings>;
  addWhitelist(path: string): Promise<Settings>;
  defaultWhitelist(): Promise<string[]>;
  appVersion(): Promise<string>;
  installStatus(): Promise<InstallStatus>;
  installNow(): Promise<void>;
  permissions(): Promise<Perms>;
  openFdaSettings(): Promise<void>;
  volumeInfo(): Promise<Volume>;
  scan(module: ScanModule): Promise<Item[]>;
  cancelScans(): Promise<void>;
  listApps(): Promise<AppInfo[]>;
  appIcon(path: string): Promise<string | null>;
  appRelated(bundleId: string, name: string, path: string): Promise<Item[]>;
  diskScan(onProgress: (p: DiskProgress) => void): Promise<DiskSummary>;
  diskView(path: string, depth: number): Promise<ViewNode>;
  clean(module: string, targets: CleanTarget[]): Promise<CleanResult>;
  emptyTrash(): Promise<CleanResult>;
  uninstall(req: UninstallRequest): Promise<CleanResult>;
  historyList(): Promise<HistoryRecord[]>;
  historyClear(): Promise<void>;
  restore(recordId: string): Promise<CleanResult>;
  reveal(path: string): Promise<void>;
  quickLook(path: string): Promise<void>;
}

export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const scanCommands: Record<ScanModule, string> = {
  system: "scan_system_junk",
  dev: "scan_dev_junk",
  large: "scan_large_files",
  leftovers: "scan_leftovers",
  trash: "scan_trash",
  downloads: "scan_downloads",
};

async function tauriApi(): Promise<Api> {
  const { invoke } = await import("@tauri-apps/api/core");
  const { listen } = await import("@tauri-apps/api/event");
  return {
    getSettings: () => invoke("get_settings"),
    saveSettings: (settings) => invoke("save_settings", { settings }),
    addWhitelist: (path) => invoke("add_whitelist", { path }),
    defaultWhitelist: () => invoke("default_whitelist"),
    appVersion: async () => (await import("@tauri-apps/api/app")).getVersion(),
    installStatus: () => invoke("install_status"),
    installNow: () => invoke("install_now"),
    permissions: () => invoke("permissions"),
    openFdaSettings: () => invoke("open_fda_settings"),
    volumeInfo: () => invoke("volume_info"),
    scan: (m) => invoke(scanCommands[m]),
    cancelScans: () => invoke("cancel_scans"),
    listApps: () => invoke("list_apps"),
    appIcon: (path) => invoke("app_icon", { path }),
    appRelated: (bundleId, name, path) => invoke("app_related", { bundleId, name, path }),
    async diskScan(onProgress) {
      const un = await listen<DiskProgress>("disk-progress", (e) => onProgress(e.payload));
      try {
        return await invoke<DiskSummary>("disk_scan");
      } finally {
        un();
      }
    },
    diskView: (path, depth) => invoke("disk_view", { path, depth }),
    clean: (module, targets) => invoke("clean", { module, targets }),
    emptyTrash: () => invoke("empty_trash"),
    uninstall: (req) => invoke("uninstall", { req }),
    historyList: () => invoke("history_list"),
    historyClear: () => invoke("history_clear"),
    restore: (recordId) => invoke("restore", { recordId }),
    reveal: (path) => invoke("reveal", { path }),
    quickLook: (path) => invoke("quick_look", { path }),
  };
}

let instance: Api | null = null;

export async function initApi(): Promise<Api> {
  if (!instance) {
    instance = isTauri ? await tauriApi() : (await import("./mock")).mockApi();
  }
  return instance;
}

/** Available after initApi() resolved (done before the app renders). */
export function api(): Api {
  if (!instance) throw new Error("api not initialised");
  return instance;
}

export function toTargets(items: Item[]): CleanTarget[] {
  return items.map((i) => ({ path: i.path, size: i.size, kind: i.kind, command: i.command ?? null }));
}

/** Tauri rejects with a plain string error code. */
export function errorCode(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return "unknown";
}
