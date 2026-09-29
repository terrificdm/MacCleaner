export type Language = "system" | "zh-CN" | "en";
export type Theme = "system" | "light" | "dark";

export interface Settings {
  language: Language;
  theme: Theme;
  dryRun: boolean;
  whitelist: string[];
  defaultWhitelistApplied?: boolean;
  largeMinMb: number;
  oldMinMb: number;
  oldDays: number;
  appUnusedDays: number;
  nodeModulesDays: number;
  nodeModulesRoots: string[];
  onboardingDone: boolean;
}

export interface Item {
  path: string;
  name: string;
  size: number;
  group: string;
  kind: "file" | "dir" | "command";
  selected: boolean;
  note?: string | null;
  modified?: number | null;
  lastUsed?: number | null;
  command?: string | null;
  needsAdmin?: boolean;
}

export interface CleanTarget {
  path: string;
  size: number;
  kind: string;
  command?: string | null;
}

export interface Done {
  path: string;
  size: number;
  trashPath?: string | null;
  method: "trash" | "finder" | "command" | "dryRun" | "emptyTrash" | "restored";
}

export interface Failed {
  path: string;
  code: string;
  detail?: string | null;
}

export interface CleanResult {
  dryRun: boolean;
  done: Done[];
  failed: Failed[];
  /** Deliberately not processed (e.g. a newer copy exists); not an error. */
  skipped?: Failed[];
  total: number;
}

export interface AppInfo {
  path: string;
  name: string;
  bundleId: string;
  version: string;
  size: number;
  source: "appStore" | "brew" | "manual";
  brewToken?: string | null;
  lastUsed?: number | null;
  running: boolean;
  blocked?: string | null;
  needsAdmin: boolean;
}

export interface UninstallRequest {
  appPath: string;
  bundleId: string;
  appSize: number;
  brewToken?: string | null;
  related: CleanTarget[];
}

export interface Volume {
  name: string;
  used: number;
  total: number;
  available: number;
}

export interface DiskProgress {
  files: number;
  bytes: number;
  current: string;
  estimate: number;
}

export interface DiskSummary {
  total: number;
  scanned: number;
  files: number;
  denied: number;
  durationMs: number;
}

export interface ViewNode {
  name: string;
  path: string;
  size: number;
  kind: "dir" | "file" | "other" | "system" | "hidden";
  deletable: boolean;
  children: ViewNode[];
}

export interface HistoryRecord {
  id: string;
  time: number;
  module: string;
  dryRun: boolean;
  total: number;
  failed: number;
  items: Done[];
  /** Items of this record still in the Trash. */
  restorable: number;
  /** Old copies in the Trash whose original path now has a newer copy. */
  superseded: number;
}

export interface InstallStatus {
  action: "none" | "fresh" | "upgrade" | "reinstall" | "downgrade" | "conflict";
  currentVersion: string;
  installedVersion?: string | null;
  runningFrom?: string | null;
  translocated: boolean;
}

export interface Perms {
  fullDiskAccess: boolean;
}

/** Scan modules that produce Item lists. */
export type ScanModule = "system" | "dev" | "large" | "leftovers" | "trash" | "downloads";
