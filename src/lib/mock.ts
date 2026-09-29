// Fake data for the browser prototype (`npm run dev` outside Tauri).
// Nothing here touches the filesystem.
import type { Api } from "./api";
import type {
  AppInfo, CleanResult, HistoryRecord, Item, ScanModule, Settings, ViewNode,
} from "./types";

const H = "/Users/demo";
const GB = 1e9;
const MB = 1e6;
const DAY = 86_400_000;
const now = Date.now();

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

function it(path: string, size: number, group: string, extra: Partial<Item> = {}): Item {
  return {
    path,
    name: path.split("/").pop()!,
    size,
    group,
    kind: "dir",
    selected: true,
    ...extra,
  };
}

const data: Record<ScanModule, Item[]> = {
  system: [
    it(`${H}/Library/Caches/ChatApp`, 2.9 * GB, "userCaches", { selected: false, note: "running" }),
    it(`${H}/Library/Caches/colima`, 2.13 * GB, "userCaches"),
    it(`${H}/Library/Caches/Google`, 1.77 * GB, "userCaches", { selected: false, note: "running" }),
    it(`${H}/Library/Caches/com.spotify.client`, 820 * MB, "userCaches"),
    it(`${H}/Library/Caches/com.microsoft.VSCode.ShipIt`, 410 * MB, "userCaches"),
    it(`${H}/Library/Caches/com.apple.Safari`, 160 * MB, "userCaches", { selected: false, note: "apple" }),
    it(`${H}/Library/Caches/Firefox`, 380 * MB, "userCaches"),
    it(`${H}/Library/Logs/JetBrains`, 310 * MB, "logs"),
    it(`${H}/Library/Logs/EditorApp`, 220 * MB, "logs"),
    it(`${H}/Library/Logs/DiagnosticReports/Discord-2026-09-12.ips`, 1.2 * MB, "crashReports", { kind: "file" }),
    it(`/private/var/folders/xx/T/installer-download-1a2b3c`, 1.06 * GB, "tempFiles"),
    it(`/private/var/folders/xx/T/installer-download-4d5e6f`, 1.06 * GB, "tempFiles"),
    it(`/private/var/folders/xx/T/com.google.Chrome.x8Zz`, 640 * MB, "tempFiles"),
  ],
  dev: [
    it(`${H}/.npm/_cacache`, 4.76 * GB, "npm"),
    it(`${H}/.npm/_npx`, 730 * MB, "npm"),
    it(`${H}/Library/Caches/pip`, 690 * MB, "pip"),
    it(`${H}/Library/Caches/Homebrew`, 510 * MB, "homebrew", {
      name: "brew cleanup --prune=all", kind: "command", command: "brewCleanup", note: "permanent",
    }),
    it(`${H}/Library/Caches/ms-playwright`, 1.65 * GB, "playwright", { selected: false }),
    it(`${H}/.bun/install/cache`, 490 * MB, "bun"),
    it(`docker://system`, 3.2 * GB, "docker", {
      name: "docker system prune -f", kind: "command", command: "dockerPrune", selected: false, note: "permanent",
    }),
    it(`${H}/Projects/old-dashboard/node_modules`, 612 * MB, "nodeModules", { name: "old-dashboard", modified: now - 220 * DAY }),
    it(`${H}/Projects/hackathon-2025/node_modules`, 388 * MB, "nodeModules", { name: "hackathon-2025", modified: now - 340 * DAY }),
    it(`${H}/temp/demo-app/node_modules`, 205 * MB, "nodeModules", { name: "demo-app", modified: now - 120 * DAY }),
  ],
  large: [
    ["Downloads/Quarterly review.pptx", 750, "document", 400, 30],
    ["Downloads/SomeApp.dmg", 580, "diskImage", 25, null],
    ["Downloads/Conference notes.pptx", 560, "document", 280, 200],
    ["Movies/screen-recording-2025-03.mov", 2400, "video", 540, null],
    ["Downloads/ubuntu-24.04-desktop-arm64.iso", 3100, "diskImage", 410, null],
    ["Documents/../Desktop/archive-2024.zip", 900, "archive", 380, 380],
    ["Music/podcast-raw.wav", 420, "audio", 700, null],
    ["Downloads/EditorApp-arm64.dmg", 440, "diskImage", 60, null],
    ["Pictures/export/panorama.tif", 180, "image", 800, 800],
  ].map(([p, mb, kind, mod, used]) => it(`${H}/${String(p).replace("Documents/../", "")}`, (mb as number) * MB, kind as string, {
    kind: "file",
    selected: false,
    modified: now - (mod as number) * DAY,
    lastUsed: used === null ? null : now - (used as number) * DAY,
  })),
  leftovers: [
    it(`${H}/Library/Application Support/com.docker.docker`, 180 * MB, "com.docker.docker", { note: "Application Support", modified: now - 200 * DAY }),
    it(`${H}/Library/Containers/com.sketch.app`, 95 * MB, "com.sketch.app", { note: "Containers", modified: now - 400 * DAY }),
    it(`${H}/Library/Preferences/com.sketch.app.plist`, 0.1 * MB, "com.sketch.app", { note: "Preferences", kind: "file", modified: now - 400 * DAY }),
    it(`${H}/Library/Group Containers/UBF8T346G9.com.microsoft.teams`, 210 * MB, "com.microsoft.teams", { selected: false, note: "sameVendor" }),
    it(`${H}/Library/Caches/com.hackemist.SDImageCache`, 3 * MB, "com.hackemist.SDImageCache", { note: "Caches" }),
    it(`${H}/Library/Saved Application State/com.electron.dockerdesktop.savedState`, 0.4 * MB, "com.electron.dockerdesktop", { note: "Saved Application State" }),
  ],
  trash: [
    it(`${H}/.Trash/old-screenshots`, 1.2 * GB, "trash", { selected: false, modified: now - 10 * DAY }),
    it(`${H}/.Trash/Installer.pkg`, 350 * MB, "trash", { kind: "file", selected: false, modified: now - 3 * DAY }),
  ],
  downloads: [
    it(`${H}/Downloads/SomeApp.dmg`, 580 * MB, "installers", { kind: "file", modified: now - 45 * DAY }),
    it(`${H}/Downloads/EditorApp-arm64.dmg`, 440 * MB, "installers", { kind: "file", modified: now - 60 * DAY }),
    it(`${H}/Downloads/zoom.pkg`, 52 * MB, "installers", { kind: "file", selected: false, modified: now - 5 * DAY }),
    it(`${H}/Downloads/slides-assets.zip`, 210 * MB, "archives", { kind: "file", selected: false, modified: now - 90 * DAY }),
    it(`${H}/Downloads/Quarterly review.pptx`, 750 * MB, "downloads", { kind: "file", selected: false, modified: now - 400 * DAY }),
    it(`${H}/Downloads/invoice-2026-08.pdf`, 1.2 * MB, "downloads", { kind: "file", selected: false, modified: now - 40 * DAY }),
  ],
};

const apps: AppInfo[] = [
  ["Discord", "com.hnc.Discord", 0.5, "manual", 2, false],
  ["Google Chrome", "com.google.Chrome", 2.24, "manual", 0, true],
  ["Microsoft Word", "com.microsoft.Word", 2.92, "manual", null, false],
  ["Microsoft Excel", "com.microsoft.Excel", 2.7, "manual", 200, false],
  ["iMovie", "com.apple.iMovieApp", 3.94, "appStore", null, false],
  ["GarageBand", "com.apple.garageband10", 1.2, "appStore", 500, false],
  ["Obsidian", "md.obsidian", 0.51, "manual", 1, false],
  ["Slack", "com.tinyspeck.slackmacgap", 0.33, "manual", 0, true],
  ["VNC Viewer", "com.realvnc.vncviewer", 0.02, "brew", 150, false],
  ["zoom.us", "us.zoom.xos", 0.9, "manual", 120, false],
  ["Visual Studio Code", "com.microsoft.VSCode", 1.54, "manual", 95, false],
  ["Security Agent", "com.example.security.agent", 0.1, "manual", null, false],
].map(([name, id, gb, source, used, running]) => ({
  path: `/Applications/${name}.app`,
  name: name as string,
  bundleId: id as string,
  version: "1.0",
  size: (gb as number) * GB,
  source: source as AppInfo["source"],
  brewToken: source === "brew" ? "vnc-viewer" : null,
  lastUsed: used === null ? null : now - (used as number) * DAY,
  running: running as boolean,
  blocked: name === "Security Agent" ? "systemExtension" : null,
  needsAdmin: source !== "manual",
}));

function tree(): ViewNode {
  const leaf = (name: string, gb: number, kind: ViewNode["kind"] = "dir", del = true, children: ViewNode[] = []): ViewNode => ({
    name, path: `/${name}`, size: gb * GB, kind, deletable: del, children,
  });
  const withPaths = (n: ViewNode, base: string): ViewNode => {
    const path = n.kind === "system" || n.kind === "hidden" ? `::${n.kind}` : `${base}/${n.name}`.replace("//", "/");
    return { ...n, path, children: n.children.map((c) => withPaths(c, path)) };
  };
  const users = leaf("Users", 240, "dir", false, [
    leaf("demo", 238, "dir", false, [
      leaf("Library", 120, "dir", false, [
        leaf("Application Support", 48, "dir", false, [leaf("Discord", 2.1), leaf("Code", 6.2), leaf("Google", 9.8), leaf("Lark", 14)]),
        leaf("Caches", 13, "dir", false, [leaf("ChatApp", 2.9), leaf("colima", 2.1), leaf("Google", 1.8)]),
        leaf("Containers", 30, "dir", false, [leaf("com.docker.docker", 18), leaf("com.microsoft.Outlook", 6)]),
        leaf("Group Containers", 22),
      ]),
      leaf("Downloads", 13.5, "dir", false, [leaf("ubuntu.iso", 3.1, "file"), leaf("Deck.pptx", 0.75, "file")]),
      leaf("Projects", 38, "dir", false, [leaf("web", 12), leaf("ml", 21), leaf("scripts", 1.2)]),
      leaf(".npm", 5.5),
      leaf("Movies", 22, "dir", false, [leaf("recordings", 18)]),
      leaf("Pictures", 16),
      leaf("other", 7, "other", false),
    ]),
  ]);
  const root: ViewNode = {
    name: "Macintosh HD", path: "/", size: 0, kind: "dir", deletable: false,
    children: [
      users,
      leaf("Applications", 45, "dir", false, [leaf("iMovie.app", 3.9), leaf("Microsoft Word.app", 2.9), leaf("Microsoft Outlook.app", 2.8)]),
      leaf("Library", 12, "dir", false),
      leaf("private", 9, "dir", false),
      leaf("opt", 4, "dir", false),
      leaf("system", 15, "system", false),
      leaf("hidden", 6, "hidden", false),
    ],
  };
  const r = withPaths(root, "");
  r.path = "/";
  r.children = r.children.map((c) => withPaths(c, ""));
  const total = (n: ViewNode): number => (n.size = Math.max(n.size, n.children.reduce((a, c) => a + total(c), 0)));
  total(r);
  return r;
}

function find(n: ViewNode, path: string): ViewNode | null {
  if (n.path === path) return n;
  for (const c of n.children) {
    const f = find(c, path);
    if (f) return f;
  }
  return null;
}

const DEFAULT_WHITELIST = [
  "~/Documents", "~/.ssh", "~/.gnupg", "~/Library/Mobile Documents", "~/Library/CloudStorage", "~/Library/Mail",
  "~/Library/Messages", "~/Library/Safari", "~/Library/Accounts", "~/Library/Calendars", "~/Library/Photos",
  "~/Library/Application Support/AddressBook", "~/Library/Application Support/CallHistoryDB", "~/Library/Application Support/MobileSync",
];

let settings: Settings = {
  language: "system",
  theme: "system",
  dryRun: true,
  whitelist: [...DEFAULT_WHITELIST, `${H}/Projects/keep-me`],
  largeMinMb: 500,
  oldMinMb: 100,
  oldDays: 365,
  appUnusedDays: 90,
  nodeModulesDays: 90,
  nodeModulesRoots: ["~"],
  onboardingDone: new URLSearchParams(location.search).has("onboarding") ? false : true,
};

const history: HistoryRecord[] = [
  {
    id: "1", time: now - 2 * DAY, module: "system", dryRun: true, total: 5.2 * GB, failed: 0, restorable: 0, superseded: 0,
    items: data.system.slice(1, 4).map((i) => ({ path: i.path, size: i.size, method: "dryRun" })),
  },
  {
    id: "2", time: now - 6 * DAY, module: "dev", dryRun: false, total: 4.8 * GB, failed: 1, restorable: 1, superseded: 1,
    items: [{ path: `${H}/.npm/_cacache`, size: 4.8 * GB, method: "trash", trashPath: `${H}/.Trash/_cacache` }],
  },
];

function fakeClean(targets: { path: string; size: number }[]): CleanResult {
  const dry = settings.dryRun;
  return {
    dryRun: dry,
    total: targets.reduce((a, b) => a + b.size, 0),
    done: targets.map((t) => ({ path: t.path, size: t.size, method: dry ? "dryRun" : "trash" })),
    failed: [],
  };
}

export function mockApi(): Api {
  let cancel = false;
  return {
    getSettings: async () => settings,
    saveSettings: async (s) => (settings = s),
    defaultWhitelist: async () => DEFAULT_WHITELIST,
    appVersion: async () => "0.1.0 (preview)",
    // Preview with ?install=fresh|upgrade|reinstall|downgrade|conflict
    installStatus: async () => {
      const a = (new URLSearchParams(location.search).get("install") ?? "none") as never;
      return { action: a, currentVersion: "0.2.0", installedVersion: a === "fresh" ? null : a === "downgrade" ? "0.3.0" : a === "reinstall" ? "0.2.0" : "0.1.0", runningFrom: "/Volumes/MacCleaner 0.2.0/MacCleaner.app", translocated: true };
    },
    installNow: async () => {
      await sleep(1200);
    },
    addWhitelist: async (p) => (settings = { ...settings, whitelist: [...settings.whitelist, p] }),
    permissions: async () => ({ fullDiskAccess: !new URLSearchParams(location.search).has("nofda") }),
    openFdaSettings: async () => {},
    volumeInfo: async () => ({ name: "Macintosh HD", used: 322 * GB, total: 494 * GB, available: 121 * GB }),
    async scan(m) {
      cancel = false;
      await sleep(900 + Math.random() * 900);
      if (cancel) throw "cancelled";
      return structuredClone(data[m]);
    },
    cancelScans: async () => {
      cancel = true;
    },
    listApps: async () => (await sleep(500), structuredClone(apps)),
    appIcon: async () => null,
    async appRelated(id, name) {
      await sleep(300);
      return [
        it(`${H}/Library/Application Support/${name}`, 1.4 * GB, "Application Support"),
        it(`${H}/Library/Caches/${id}`, 120 * MB, "Caches"),
        it(`${H}/Library/Preferences/${id}.plist`, 0.02 * MB, "Preferences", { kind: "file" }),
        it(`${H}/Library/Saved Application State/${id}.savedState`, 0.3 * MB, "Saved Application State"),
        it(`/Library/LaunchDaemons/${id}.helper.plist`, 0.01 * MB, "system", { kind: "file", needsAdmin: true }),
      ];
    },
    async diskScan(onProgress) {
      cancel = false;
      const est = 322 * GB;
      for (let i = 1; i <= 25; i++) {
        await sleep(90);
        if (cancel) throw "cancelled";
        onProgress({ files: i * 61234, bytes: (est * i) / 25, current: `${H}/Library/Caches/step-${i}`, estimate: est });
      }
      return { total: 337 * GB, scanned: 316 * GB, files: 1_530_000, denied: 12, durationMs: 2400 };
    },
    async diskView(path) {
      const t = tree();
      const n = find(t, path) ?? t;
      return structuredClone(n);
    },
    clean: async (_m, targets) => (await sleep(700), fakeClean(targets)),
    emptyTrash: async () => fakeClean([{ path: `${H}/.Trash`, size: 1.55 * GB }]),
    uninstall: async (req) =>
      (await sleep(900), fakeClean([{ path: req.appPath, size: req.appSize }, ...req.related])),
    historyList: async () => history,
    restore: async (id) => {
      const r = history.find((h) => h.id === id);
      await sleep(600);
      const items = (r?.items ?? []).filter((i) => i.method === "trash" || i.method === "finder");
      return {
        dryRun: false,
        total: items.reduce((a, b) => a + b.size, 0),
        done: items.map((i) => ({ ...i, method: "restored" as const })),
        failed: [],
        skipped: [{ path: `${H}/.npm/_logs`, code: "restoreExists" }],
      };
    },
    historyClear: async () => {
      history.length = 0;
    },
    reveal: async () => {},
    quickLook: async () => {},
  };
}
