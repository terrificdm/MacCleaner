import { motion } from "framer-motion";
import { RotateCw, Sparkles, Trash2 } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { api, toTargets } from "@/lib/api";
import type { Item, ScanModule } from "@/lib/types";
import { formatBytes, sum } from "@/lib/format";
import { selectedItems, useStore, type PageId } from "@/store";
import { BigSize } from "./BigSize";
import { useCleanFlow } from "./CleanFlow";
import { ItemList } from "./ItemList";
import { Page } from "./Page";
import { ScanHero } from "./ScanHero";
import { Button, Empty } from "./ui";

/** Open the confirm dialog for items of a module; drop moved items afterwards. */
export function startClean(module: ScanModule, items: Item[], historyModule: string = module) {
  useCleanFlow.getState().start({
    kind: "clean",
    items,
    run: (chosen) => api().clean(historyModule, toTargets(chosen)),
    onDone: (r) => {
      if (!r.dryRun) useStore.getState().removeEverywhere(r.done.map((d) => d.path));
    },
  });
}

export function ScanModulePage({
  page, module, heroIcon, banner, meta, groupLabel, emptyHint, openGroups,
}: {
  page: PageId;
  module: ScanModule;
  heroIcon: ReactNode;
  banner?: ReactNode;
  meta?: (i: Item) => ReactNode;
  groupLabel?: (g: string) => ReactNode;
  emptyHint?: ReactNode;
  openGroups?: number;
}) {
  const { t } = useTranslation();
  const scan = useStore((s) => s.scans[module]);
  const selection = useStore((s) => s.selection[module]);
  const runScan = useStore((s) => s.runScan);
  const setSelected = useStore((s) => s.setSelected);

  if (scan.status !== "done") {
    return (
      <Page id={page}>
        {banner && <div className="px-8">{banner}</div>}
        <ScanHero
          scanning={scan.status === "scanning"}
          onScan={() => runScan(module)}
          onCancel={() => api().cancelScans()}
          icon={heroIcon}
          title={t(`modules.${page}.heroTitle`)}
          hint={scan.status === "error" ? t(`errors.${scan.code}`, { defaultValue: t("errors.unknown") }) : t(`modules.${page}.heroHint`)}
        />
      </Page>
    );
  }

  const items = scan.items;
  const chosen = items.filter((i) => selection[i.path] ?? i.selected);
  const chosenSize = sum(chosen);

  return (
    <Page
      id={page}
      actions={
        <Button variant="ghost" size="sm" icon={<RotateCw className="size-3.5" />} onClick={() => runScan(module)}>
          {t("common.rescan")}
        </Button>
      }
    >
      <div className="flex h-full flex-col">
        <div className="shrink-0 px-8 pb-4">
          {banner}
          <motion.div
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            className="flex items-end justify-between gap-6 rounded-3xl glass px-6 py-5"
          >
            <div>
              <div className="text-[12.5px] text-muted">{t("common.selectedToClean")}</div>
              <BigSize bytes={chosenSize} className="text-[44px] leading-none gradient-text" />
              <div className="mt-1.5 text-xs text-faint tabular">
                {t("common.selectedOf", { count: chosen.length, total: items.length, size: formatBytes(sum(items)) })}
              </div>
            </div>
            <div className="flex items-center gap-2">
              <Button variant="ghost" size="sm" onClick={() => setSelected(module, items.map((i) => i.path), false)}>
                {t("common.selectNone")}
              </Button>
              <Button
                variant="primary"
                size="lg"
                disabled={!chosen.length}
                icon={<Trash2 className="size-4.5" />}
                onClick={() => startClean(module, selectedItems(module, items))}
              >
                {t("common.clean")}
              </Button>
            </div>
          </motion.div>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto px-8 pb-8">
          {items.length === 0 ? (
            <Empty icon={<Sparkles className="size-10 text-[var(--accent)]" />} title={t("common.allClean")} hint={emptyHint} />
          ) : (
            <ItemList
              items={items}
              selection={selection}
              onSelect={(p, v) => setSelected(module, p, v)}
              meta={meta}
              groupLabel={groupLabel}
              openGroups={openGroups}
            />
          )}
        </div>
      </div>
    </Page>
  );
}
