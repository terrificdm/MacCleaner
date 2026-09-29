import { AnimatePresence, motion } from "framer-motion";
import { Package, RotateCw, Search, ShieldAlert, Trash2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { create } from "zustand";
import { useCleanFlow } from "@/components/CleanFlow";
import { ItemList } from "@/components/ItemList";
import { Page } from "@/components/Page";
import { Badge, Button, Empty, Segmented, Spinner } from "@/components/ui";
import { api, errorCode, toTargets } from "@/lib/api";
import type { AppInfo, Item } from "@/lib/types";
import { cn, daysAgo, formatBytes, relativeTime, shortPath, sum } from "@/lib/format";
import { useStore } from "@/store";

const iconCache = new Map<string, string | null>();

export function AppIcon({ path, className }: { path: string; className?: string }) {
  const [src, setSrc] = useState<string | null | undefined>(iconCache.get(path));
  useEffect(() => {
    if (iconCache.has(path)) return setSrc(iconCache.get(path));
    let alive = true;
    api()
      .appIcon(path)
      .catch(() => null)
      .then((s) => {
        iconCache.set(path, s);
        if (alive) setSrc(s);
      });
    return () => {
      alive = false;
    };
  }, [path]);
  if (src) return <img src={src} alt="" draggable={false} className={cn("object-contain", className)} />;
  return (
    <div className={cn("flex items-center justify-center rounded-[22%] bg-gradient-to-br from-[var(--accent)] to-[var(--accent-2)] text-white", className)}>
      <Package className="size-1/2" />
    </div>
  );
}

interface AppsState {
  apps: AppInfo[] | null;
  loading: boolean;
  error: string | null;
  load(): Promise<void>;
}

const useApps = create<AppsState>((set) => ({
  apps: null,
  loading: false,
  error: null,
  async load() {
    set({ loading: true, error: null });
    try {
      set({ apps: await api().listApps(), loading: false });
    } catch (e) {
      set({ loading: false, error: errorCode(e) });
    }
  },
}));

type Filter = "all" | "suggested" | "appStore" | "brew" | "manual";
type Sort = "name" | "size" | "lastUsed";

export function UninstallerPage() {
  const { t } = useTranslation();
  const { apps, loading, error, load } = useApps();
  const unusedDays = useStore((s) => s.settings?.appUnusedDays ?? 90);
  const [q, setQ] = useState("");
  const [filter, setFilter] = useState<Filter>("all");
  const [sort, setSort] = useState<Sort>("name");
  const [current, setCurrent] = useState<AppInfo | null>(null);

  useEffect(() => {
    if (!apps && !loading) load();
  }, [apps, loading, load]);

  const suggested = (a: AppInfo) => !a.blocked && a.lastUsed != null && (daysAgo(a.lastUsed) ?? 0) >= unusedDays;

  const list = useMemo(() => {
    let l = (apps ?? []).filter((a) => a.name.toLowerCase().includes(q.toLowerCase()) || a.bundleId.toLowerCase().includes(q.toLowerCase()));
    if (filter === "suggested") l = l.filter(suggested);
    else if (filter !== "all") l = l.filter((a) => a.source === filter);
    const s = [...l];
    if (sort === "size") s.sort((a, b) => b.size - a.size);
    if (sort === "lastUsed") s.sort((a, b) => (a.lastUsed ?? Infinity) - (b.lastUsed ?? Infinity));
    return s;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [apps, q, filter, sort, unusedDays]);

  const counts = useMemo(
    () => ({
      all: apps?.length ?? 0,
      suggested: apps?.filter(suggested).length ?? 0,
      appStore: apps?.filter((a) => a.source === "appStore").length ?? 0,
      brew: apps?.filter((a) => a.source === "brew").length ?? 0,
      manual: apps?.filter((a) => a.source === "manual").length ?? 0,
    }),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [apps, unusedDays],
  );

  return (
    <Page
      id="apps"
      actions={
        <Button variant="ghost" size="sm" icon={<RotateCw className="size-3.5" />} onClick={load} disabled={loading}>
          {t("common.refresh")}
        </Button>
      }
    >
      <div className="flex h-full gap-4 px-8 pb-8">
        <div className="flex min-w-0 flex-[1.25] flex-col gap-3">
          <div className="flex items-center gap-2">
            <div className="flex h-9 flex-1 items-center gap-2 rounded-xl glass px-3">
              <Search className="size-4 text-faint" />
              <input
                value={q}
                onChange={(e) => setQ(e.target.value)}
                placeholder={t("apps.search")}
                className="h-full flex-1 bg-transparent text-[13px] outline-none placeholder:text-faint"
              />
            </div>
            <Segmented<Sort>
              value={sort}
              onChange={setSort}
              options={[
                { value: "name", label: t("apps.sortName") },
                { value: "size", label: t("apps.sortSize") },
                { value: "lastUsed", label: t("apps.sortUsed") },
              ]}
            />
          </div>
          <div className="flex flex-wrap gap-1.5">
            {(["all", "suggested", "appStore", "brew", "manual"] as Filter[]).map((f) => (
              <button
                key={f}
                onClick={() => setFilter(f)}
                className={cn(
                  "rounded-full px-3 py-1 text-[12px] transition-colors",
                  filter === f ? "bg-[var(--accent)] text-white" : "bg-card text-muted hover:text-fg",
                )}
              >
                {f === "suggested" ? t("apps.suggested", { days: unusedDays }) : t(`apps.filter.${f}`)}
                <span className="ml-1.5 opacity-60 tabular">{counts[f]}</span>
              </button>
            ))}
          </div>
          <div className="min-h-0 flex-1 overflow-y-auto rounded-2xl glass">
            {loading && !apps ? (
              <div className="flex h-full items-center justify-center text-muted">
                <Spinner />
              </div>
            ) : error ? (
              <Empty icon={<ShieldAlert className="size-10" />} title={t(`errors.${error}`, { defaultValue: t("errors.unknown") })} />
            ) : list.length === 0 ? (
              <Empty icon={<Package className="size-10 text-[var(--accent)]" />} title={t("common.noResults")} />
            ) : (
              list.map((a) => (
                <button
                  key={a.path}
                  onClick={() => setCurrent(a)}
                  className={cn(
                    "flex w-full items-center gap-3 border-b border-line px-4 py-2.5 text-left last:border-0",
                    current?.path === a.path ? "bg-[var(--accent)]/12" : "hover:bg-card-hover",
                  )}
                >
                  <AppIcon path={a.path} className="size-9 shrink-0" />
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-1.5">
                      <span className="truncate text-[13.5px] font-medium">{a.name}</span>
                      {a.running && <span className="size-1.5 shrink-0 rounded-full bg-ok" title={t("apps.running")} />}
                    </div>
                    <div className="mt-0.5 flex items-center gap-1.5">
                      <SourceBadge app={a} />
                      {suggested(a) && <Badge tone="warn">{t("apps.unusedBadge", { days: daysAgo(a.lastUsed) })}</Badge>}
                      {a.blocked && <Badge tone="danger">{t(`apps.blocked.${a.blocked}`, { defaultValue: t("apps.blocked.protected") })}</Badge>}
                    </div>
                  </div>
                  <div className="shrink-0 text-right">
                    <div className="text-[13px] tabular">{formatBytes(a.size)}</div>
                    <div className="text-[11px] text-faint">{a.lastUsed ? relativeTime(a.lastUsed) : t("apps.lastUsedUnknown")}</div>
                  </div>
                </button>
              ))
            )}
          </div>
        </div>
        <div className="min-w-0 flex-1">
          <AnimatePresence mode="wait">
            {current ? (
              <AppDetail key={current.path} app={current} onRemoved={() => (setCurrent(null), load())} />
            ) : (
              <motion.div key="none" initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="flex h-full items-center justify-center rounded-3xl glass">
                <Empty icon={<Package className="size-12 text-[var(--accent)]" />} title={t("apps.pick")} hint={t("apps.pickHint")} />
              </motion.div>
            )}
          </AnimatePresence>
        </div>
      </div>
    </Page>
  );
}

function SourceBadge({ app }: { app: AppInfo }) {
  const { t } = useTranslation();
  const tone = app.source === "appStore" ? "accent" : app.source === "brew" ? "ok" : "neutral";
  return <Badge tone={tone}>{t(`apps.source.${app.source}`)}</Badge>;
}

function AppDetail({ app, onRemoved }: { app: AppInfo; onRemoved: () => void }) {
  const { t } = useTranslation();
  const [related, setRelated] = useState<Item[] | null>(null);
  const [sel, setSel] = useState<Record<string, boolean>>({});

  useEffect(() => {
    let alive = true;
    api()
      .appRelated(app.bundleId, app.name, app.path)
      .then((r) => alive && (setRelated(r), setSel(Object.fromEntries(r.map((i) => [i.path, i.selected])))))
      .catch(() => alive && setRelated([]));
    return () => {
      alive = false;
    };
  }, [app]);

  const chosen = (related ?? []).filter((i) => sel[i.path]);
  const total = app.size + sum(chosen);

  const uninstall = () => {
    const appItem: Item = {
      path: app.path, name: app.name + ".app", size: app.size, group: "app", kind: "dir", selected: true, needsAdmin: app.needsAdmin,
    };
    const notes = [];
    if (app.running) notes.push(t("apps.willQuit", { name: app.name }));
    if (app.brewToken) notes.push(t("apps.brewUninstall", { token: app.brewToken }));
    useCleanFlow.getState().start({
      kind: "uninstall",
      title: t("apps.uninstallTitle", { name: app.name }),
      items: [appItem, ...chosen],
      locked: [app.path],
      notes,
      run: (items) =>
        api().uninstall({
          appPath: app.path,
          bundleId: app.bundleId,
          appSize: app.size,
          brewToken: app.brewToken ?? null,
          related: toTargets(items.filter((i) => i.path !== app.path)),
        }),
      onDone: (r) => {
        if (!r.dryRun) onRemoved();
      },
    });
  };

  return (
    <motion.div
      initial={{ opacity: 0, x: 16 }}
      animate={{ opacity: 1, x: 0 }}
      exit={{ opacity: 0, x: -16 }}
      transition={{ duration: 0.2 }}
      className="flex h-full flex-col rounded-3xl glass"
    >
      <div className="flex items-center gap-4 border-b border-line p-5">
        <AppIcon path={app.path} className="size-16 shrink-0" />
        <div className="min-w-0 flex-1">
          <div className="truncate text-lg font-semibold">{app.name}</div>
          <div className="truncate font-mono text-[11px] text-faint selectable">{app.bundleId}</div>
          <div className="mt-1.5 flex flex-wrap items-center gap-1.5 text-[11.5px] text-muted">
            <SourceBadge app={app} />
            {app.version && <span>v{app.version}</span>}
            <span>·</span>
            <span>{t("apps.lastUsed", { when: app.lastUsed ? relativeTime(app.lastUsed) : t("apps.lastUsedUnknown") })}</span>
          </div>
        </div>
      </div>
      {app.blocked ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-3 p-8 text-center">
          <ShieldAlert className="size-12 text-danger" />
          <div className="font-medium">{t(`apps.blocked.${app.blocked}`, { defaultValue: t("apps.blocked.protected") })}</div>
          <div className="max-w-xs text-[12.5px] text-muted">{t(`apps.blockedHint.${app.blocked}`, { defaultValue: t("apps.blockedHint.protected") })}</div>
        </div>
      ) : (
        <>
          <div className="min-h-0 flex-1 overflow-y-auto p-4">
            <div className="mb-2 flex items-center justify-between px-1 text-[12.5px]">
              <span className="font-medium">{t("apps.related")}</span>
              <span className="text-faint">{related ? t("common.itemCount", { count: related.length }) : ""}</span>
            </div>
            <div className="mb-2 flex items-center justify-between rounded-xl bg-card px-3 py-2 text-[12.5px]">
              <span className="truncate text-muted selectable">{shortPath(app.path)}</span>
              <span className="tabular">{formatBytes(app.size)}</span>
            </div>
            {!related ? (
              <div className="flex justify-center py-8 text-muted">
                <Spinner />
              </div>
            ) : related.length === 0 ? (
              <div className="py-6 text-center text-[12.5px] text-faint">{t("apps.noRelated")}</div>
            ) : (
              <ItemList
                  items={related}
                  grouped={false}
                  selection={sel}
                  onSelect={(ps, v) => setSel({ ...sel, ...Object.fromEntries(ps.map((p) => [p, v])) })}
                  sub={(i) => t(`groups.${i.group}`, { defaultValue: i.group })}
                />
            )}
          </div>
          <div className="flex items-center justify-between gap-3 border-t border-line p-5">
            <div>
              <div className="text-[11.5px] text-muted">{t("apps.total")}</div>
              <div className="text-xl font-semibold tabular gradient-text">{formatBytes(total)}</div>
            </div>
            <Button variant="danger" size="lg" icon={<Trash2 className="size-4.5" />} onClick={uninstall} disabled={!related}>
              {t("apps.uninstall")}
            </Button>
          </div>
        </>
      )}
    </motion.div>
  );
}
