import { motion } from "framer-motion";
import { ChevronRight, Eye, FolderSearch, Lock, PieChart, RotateCw, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { create } from "zustand";
import { useCleanFlow } from "@/components/CleanFlow";
import { Page } from "@/components/Page";
import { ScanHero } from "@/components/ScanHero";
import { labelOf, nodeColor, Sunburst } from "@/components/Sunburst";
import { Button, Checkbox, Tip } from "@/components/ui";
import { api, errorCode, toTargets } from "@/lib/api";
import type { DiskProgress, DiskSummary, Item, ViewNode } from "@/lib/types";
import { cn, formatBytes, sum } from "@/lib/format";
import { FdaBanner } from "./ListPages";

interface LensState {
  status: "idle" | "scanning" | "done" | "error";
  error?: string;
  progress: DiskProgress | null;
  summary: DiskSummary | null;
  view: ViewNode | null;
  stale: boolean;
  scan(): Promise<void>;
  open(path: string): Promise<void>;
}

const useLens = create<LensState>((set, get) => ({
  status: "idle",
  progress: null,
  summary: null,
  view: null,
  stale: false,
  async scan() {
    set({ status: "scanning", progress: null, stale: false });
    try {
      const summary = await api().diskScan((p) => set({ progress: p }));
      const view = await api().diskView("/", 3);
      set({ status: "done", summary, view });
    } catch (e) {
      const code = errorCode(e);
      set({ status: code === "cancelled" ? "idle" : "error", error: code });
    }
  },
  async open(path) {
    try {
      const view = await api().diskView(path, 3);
      set({ view });
    } catch {
      // Keep the current view if the node disappeared.
    }
    void get;
  },
}));

function parentPath(p: string): string | null {
  if (p === "/") return null;
  const i = p.lastIndexOf("/");
  return i <= 0 ? "/" : p.slice(0, i);
}

export function SpaceLensPage() {
  const { t } = useTranslation();
  const { status, progress, summary, view, stale, error, scan, open } = useLens();
  const [basket, setBasket] = useState<Record<string, ViewNode>>({});
  const [hl, setHl] = useState<string | null>(null);
  const [rootName, setRootName] = useState("/");
  useEffect(() => {
    api().volumeInfo().then((v) => setRootName(v.name)).catch(() => {});
  }, []);

  if (status !== "done" || !view) {
    const pct = progress && progress.estimate ? Math.min(99, (progress.bytes / progress.estimate) * 100) : 0;
    return (
      <Page id="space">
        <div className="px-8">
          <FdaBanner text={t("space.fdaHint")} />
        </div>
        <ScanHero
          scanning={status === "scanning"}
          onScan={scan}
          onCancel={() => api().cancelScans()}
          icon={<PieChart className="size-12" strokeWidth={1.6} />}
          title={status === "scanning" ? t("space.scanning", { pct: pct.toFixed(0) }) : t("modules.space.heroTitle")}
          hint={
            status === "error"
              ? t(`errors.${error}`, { defaultValue: t("errors.unknown") })
              : status === "scanning" && progress
                ? t("space.progress", { files: progress.files.toLocaleString(), bytes: formatBytes(progress.bytes), total: formatBytes(progress.estimate) })
                : t("modules.space.heroHint")
          }
          status={status === "scanning" ? progress?.current : undefined}
        />
      </Page>
    );
  }

  const crumbs: { name: string; path: string }[] = [{ name: rootName, path: "/" }];
  if (view.path !== "/") {
    let acc = "";
    for (const part of view.path.split("/").filter(Boolean)) {
      acc += "/" + part;
      crumbs.push({ name: part, path: acc });
    }
  }
  const basketItems = Object.values(basket);
  const toggle = (n: ViewNode, v: boolean) => {
    const next = { ...basket };
    if (v) next[n.path] = n;
    else delete next[n.path];
    setBasket(next);
  };

  const clean = () => {
    const items: Item[] = basketItems.map((n) => ({
      path: n.path, name: n.name, size: n.size, group: "space", kind: n.kind === "file" ? "file" : "dir", selected: true,
    }));
    useCleanFlow.getState().start({
      kind: "clean",
      items,
      run: (chosen) => api().clean("space", toTargets(chosen)),
      onDone: (r) => {
        if (r.dryRun) return;
        const gone = new Set(r.done.map((d) => d.path));
        const prune = (n: ViewNode): ViewNode => ({ ...n, children: n.children.filter((c) => !gone.has(c.path)).map(prune) });
        useLens.setState({ view: prune(useLens.getState().view!), stale: true });
        setBasket({});
      },
    });
  };

  const back = parentPath(view.path);

  return (
    <Page
      id="space"
      actions={
        <Button variant="ghost" size="sm" icon={<RotateCw className="size-3.5" />} onClick={scan}>
          {t("common.rescan")}
        </Button>
      }
    >
      <div className="flex h-full flex-col px-8 pb-6">
        <div className="mb-3 flex shrink-0 items-center gap-1 text-[12.5px]">
          {crumbs.map((c, i) => (
            <span key={c.path} className="flex items-center gap-1">
              {i > 0 && <ChevronRight className="size-3.5 text-faint" />}
              <button
                className={cn("rounded-md px-1.5 py-0.5", i === crumbs.length - 1 ? "font-semibold text-fg" : "text-muted hover:bg-card hover:text-fg")}
                onClick={() => open(c.path)}
              >
                {c.name}
              </button>
            </span>
          ))}
          {summary && (
            <span className="ml-auto text-[11.5px] text-faint tabular">
              {t("space.summary", { files: summary.files.toLocaleString(), secs: (summary.durationMs / 1000).toFixed(1) })}
            </span>
          )}
        </div>
        {stale && <div className="mb-2 shrink-0 text-[11.5px] text-warn">{t("space.stale")}</div>}
        <div className="flex min-h-0 flex-1 gap-5">
          <div className="flex flex-[1.1] items-center justify-center rounded-3xl glass">
            <Sunburst root={view} size={Math.min(470, window.innerHeight - 260)} onZoom={(n) => open(n.path)} onBack={back ? () => open(back) : undefined} highlight={hl} />
          </div>
          <div className="flex min-w-0 flex-1 flex-col rounded-3xl glass">
            <div className="flex items-center justify-between border-b border-line px-5 py-3.5">
              <div className="min-w-0">
                <div className="truncate text-[14px] font-semibold">{labelOf(view, t)}</div>
                <div className="text-[11.5px] text-muted tabular">{formatBytes(view.size)}</div>
              </div>
              {!view.deletable && view.path !== "/" && (
                <span className="flex items-center gap-1 text-[11px] text-faint">
                  <Lock className="size-3" /> {t("space.viewOnly")}
                </span>
              )}
            </div>
            <div className="min-h-0 flex-1 overflow-y-auto py-1">
              {view.children.map((c, i) => {
                const pct = view.size ? (c.size / view.size) * 100 : 0;
                const synthetic = c.kind === "system" || c.kind === "hidden" || c.kind === "other";
                return (
                  <div
                    key={c.path}
                    className="group flex items-center gap-3 px-5 py-2 hover:bg-card-hover"
                    onMouseEnter={() => setHl(c.path)}
                    onMouseLeave={() => setHl(null)}
                  >
                    <Checkbox checked={!!basket[c.path]} disabled={!c.deletable} onChange={(v) => toggle(c, v)} />
                    <span className="size-2.5 shrink-0 rounded-full" style={{ background: nodeColor(c, i) }} />
                    <button
                      className={cn("min-w-0 flex-1 truncate text-left text-[13px]", c.kind === "dir" && c.children.length ? "hover:underline" : "cursor-default")}
                      onClick={() => c.kind === "dir" && open(c.path)}
                    >
                      {labelOf(c, t)}
                    </button>
                    {!synthetic && (
                      <div className="flex gap-0.5 opacity-0 group-hover:opacity-100">
                        <Tip label={t("common.quickLook")}>
                          <button className="rounded-md p-1 text-muted hover:bg-card hover:text-fg" onClick={() => api().quickLook(c.path)}>
                            <Eye className="size-3.5" />
                          </button>
                        </Tip>
                        <Tip label={t("common.reveal")}>
                          <button className="rounded-md p-1 text-muted hover:bg-card hover:text-fg" onClick={() => api().reveal(c.path)}>
                            <FolderSearch className="size-3.5" />
                          </button>
                        </Tip>
                      </div>
                    )}
                    <div className="w-14 text-right text-[11px] text-faint tabular">{pct.toFixed(1)}%</div>
                    <div className="w-20 text-right text-[13px] tabular">{formatBytes(c.size)}</div>
                  </div>
                );
              })}
            </div>
            <motion.div layout className="flex items-center justify-between gap-3 border-t border-line px-5 py-3.5">
              <div className="text-[12px] text-muted">
                {basketItems.length ? t("space.basket", { count: basketItems.length, size: formatBytes(sum(basketItems)) }) : t("space.basketHint")}
              </div>
              <Button variant="primary" size="sm" disabled={!basketItems.length} icon={<Trash2 className="size-3.5" />} onClick={clean}>
                {t("common.moveToTrash")}
              </Button>
            </motion.div>
          </div>
        </div>
      </div>
    </Page>
  );
}
