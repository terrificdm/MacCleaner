import { motion } from "framer-motion";
import { Check, ChevronRight, RotateCw, Sparkles, Trash, Trash2 } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { BigSize } from "@/components/BigSize";
import { useCleanFlow } from "@/components/CleanFlow";
import { Page } from "@/components/Page";
import { ScanHero } from "@/components/ScanHero";
import { Button, Spinner } from "@/components/ui";
import { api, toTargets } from "@/lib/api";
import type { CleanResult, Item, ScanModule } from "@/lib/types";
import { formatBytes, sum } from "@/lib/format";
import { meta } from "@/modules";
import { selectedItems, useStore, type PageId } from "@/store";
import { emptyTrashFlow } from "./TrashDownloads";
import { uniqueByPath } from "@/lib/select";
import { largeMatch } from "./LargeFiles";

/** Modules included in smart scan, with the page that shows their details. */
const STEPS: { module: ScanModule; page: PageId; cleanable: boolean }[] = [
  { module: "system", page: "system", cleanable: true },
  { module: "dev", page: "dev", cleanable: true },
  { module: "leftovers", page: "leftovers", cleanable: true },
  { module: "downloads", page: "trash", cleanable: true },
  { module: "large", page: "large", cleanable: false },
  { module: "trash", page: "trash", cleanable: false },
];

export function SmartScanPage() {
  const { t } = useTranslation();
  const scans = useStore((s) => s.scans);
  const selection = useStore((s) => s.selection);
  const fda = useStore((s) => s.perms.fullDiskAccess);
  const runScan = useStore((s) => s.runScan);
  const go = useStore((s) => s.go);
  const [running, setRunning] = useState(false);
  const [started, setStarted] = useState(() => STEPS.some((s) => scans[s.module].status === "done"));

  const steps = STEPS.filter((s) => s.module !== "trash" || fda);

  const start = async () => {
    setRunning(true);
    setStarted(true);
    for (const s of steps) {
      const r = await runScan(s.module);
      // A cancelled scan resets the module to idle; stop the whole run.
      if (r === null && useStore.getState().scans[s.module].status === "idle") break;
    }
    setRunning(false);
  };

  const cancel = () => {
    api().cancelScans();
  };

  const settings = useStore((s) => s.settings);
  const items = (m: ScanModule): Item[] => {
    const s = scans[m];
    if (s.status !== "done") return [];
    // Same thresholds as the Large & Old Files page.
    return m === "large" && settings ? s.items.filter((i) => largeMatch(i, settings)) : s.items;
  };
  const chosen = (m: ScanModule) => items(m).filter((i) => selection[m][i.path] ?? i.selected);
  const cleanable = steps.filter((s) => s.cleanable);
  const total = cleanable.reduce((a, s) => a + sum(chosen(s.module)), 0);

  if (!started || (running && steps.every((s) => scans[s.module].status !== "done"))) {
    return (
      <Page id="smart" hideHeader>
        <ScanHero
          size={240}
          scanning={running}
          onScan={start}
          onCancel={cancel}
          icon={<Sparkles className="size-14" strokeWidth={1.5} />}
          title={t("modules.smart.heroTitle")}
          hint={running ? t("smart.scanningHint") : t("modules.smart.heroHint")}
        />
      </Page>
    );
  }

  const cleanAll = () => {
    // A folder can be listed by two modules; clean it once.
    const all = uniqueByPath(cleanable.flatMap((s) => selectedItems(s.module, items(s.module)).map((i) => ({ ...i, group: s.module }))));
    useCleanFlow.getState().start({
      kind: "clean",
      items: all,
      run: async (picked) => {
        const merged: CleanResult = { dryRun: true, done: [], failed: [], total: 0 };
        for (const s of cleanable) {
          const mine = picked.filter((i) => i.group === s.module);
          if (!mine.length) continue;
          const r = await api().clean(s.module, toTargets(mine));
          merged.dryRun = r.dryRun;
          merged.done.push(...r.done);
          merged.failed.push(...r.failed);
          merged.total += r.total;
          if (!r.dryRun) useStore.getState().removeEverywhere(r.done.map((d) => d.path));
        }
        return merged;
      },
    });
  };

  return (
    <Page
      id="smart"
      actions={
        <Button variant="ghost" size="sm" icon={<RotateCw className="size-3.5" />} onClick={start} disabled={running}>
          {t("common.rescan")}
        </Button>
      }
    >
      <div className="h-full overflow-y-auto px-8 pb-8">
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          className="relative mb-5 flex items-center justify-between gap-6 overflow-hidden rounded-3xl glass px-8 py-7"
        >
          <div>
            <div className="text-[13px] text-muted">{running ? t("smart.scanningHint") : t("smart.found")}</div>
            <BigSize bytes={total} className="text-[64px] leading-none gradient-text" />
            <div className="mt-2 text-[12px] text-faint">{t("smart.foundHint")}</div>
          </div>
          <Button variant="primary" size="lg" disabled={running || total === 0} icon={running ? <Spinner /> : <Trash2 className="size-5" />} onClick={cleanAll} className="h-14 px-9 text-[17px]">
            {t("common.clean")}
          </Button>
        </motion.div>

        <div className="grid grid-cols-3 gap-4">
          {steps.map((s, idx) => {
            const st = scans[s.module];
            const m = meta(s.page);
            const Icon = s.module === "downloads" ? meta("trash").icon : s.module === "trash" ? Trash : m.icon;
            const list = items(s.module);
            const sel = chosen(s.module);
            return (
              <motion.button
                key={s.module}
                initial={{ opacity: 0, y: 14 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: idx * 0.05 }}
                whileHover={{ y: -3 }}
                onClick={() => go(s.page)}
                className="group relative flex flex-col justify-start overflow-hidden rounded-3xl glass p-5 text-left"
              >
                <div className="absolute -right-10 -top-10 size-32 rounded-full opacity-30 blur-2xl" style={{ background: m.from }} />
                <div className="relative flex items-center justify-between">
                  <span className="flex size-10 items-center justify-center rounded-2xl text-white" style={{ background: `linear-gradient(135deg, ${m.from}, ${m.to})` }}>
                    <Icon className="size-5" />
                  </span>
                  {st.status === "scanning" ? (
                    <Spinner className="text-muted" />
                  ) : st.status === "done" ? (
                    <span className="flex size-5 items-center justify-center rounded-full bg-ok/20 text-ok">
                      <Check className="size-3" strokeWidth={3} />
                    </span>
                  ) : null}
                </div>
                <div className="relative mt-4 text-[14px] font-semibold">{t(`smart.cards.${s.module}`)}</div>
                <div className="relative mt-1 text-[22px] font-semibold tabular">
                  {st.status === "done" ? formatBytes(s.cleanable ? sum(sel) : sum(list)) : st.status === "scanning" ? "…" : "—"}
                </div>
                <div className="relative mt-1 flex items-center justify-between text-[11.5px] text-muted">
                  <span>
                    {st.status === "done"
                      ? s.cleanable
                        ? t("smart.selectedCount", { count: sel.length, total: list.length })
                        : t(`smart.review.${s.module}`, { count: list.length })
                      : st.status === "error"
                        ? t(`errors.${st.code}`, { defaultValue: t("errors.unknown") })
                        : t("smart.waiting")}
                  </span>
                  <ChevronRight className="size-4 opacity-0 transition-opacity group-hover:opacity-100" />
                </div>
                {s.module === "trash" && st.status === "done" && list.length > 0 && (
                  <div
                    role="button"
                    className="relative mt-3 inline-flex self-start items-center gap-1.5 rounded-lg bg-danger/15 px-2.5 py-1 text-[12px] font-medium text-danger hover:bg-danger/25"
                    onClick={(e) => {
                      e.stopPropagation();
                      emptyTrashFlow(sum(list));
                    }}
                  >
                    <Trash2 className="size-3.5" /> {t("trash.empty")}
                  </div>
                )}
              </motion.button>
            );
          })}
        </div>
      </div>
    </Page>
  );
}
