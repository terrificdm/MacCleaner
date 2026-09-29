import { motion } from "framer-motion";
import { FileBox, RotateCw, SlidersHorizontal, Trash2 } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { BigSize } from "@/components/BigSize";
import { ItemList } from "@/components/ItemList";
import { Page } from "@/components/Page";
import { ScanHero } from "@/components/ScanHero";
import { startClean } from "@/components/ScanModulePage";
import { Button, Empty, Segmented, Slider } from "@/components/ui";
import { api } from "@/lib/api";
import type { Item } from "@/lib/types";
import { cn, DAY, formatBytes, relativeTime, sum } from "@/lib/format";
import { useStore } from "@/store";

/** Scanner only collects files of at least this size. */
export const LARGE_FLOOR_MB = 50;

export interface LargeRules {
  largeMinMb: number;
  oldMinMb: number;
  oldDays: number;
}

/** Large: at least largeMinMb. Old: at least oldMinMb and unused (or unmodified) for oldDays. */
export function largeMatch(i: Item, r: LargeRules, criteria: "any" | "large" | "old" = "any", now = Date.now()): boolean {
  const isLarge = i.size >= r.largeMinMb * 1e6;
  const ref = i.lastUsed ?? i.modified;
  const isOld = i.size >= r.oldMinMb * 1e6 && !!ref && now - ref >= r.oldDays * DAY;
  return criteria === "large" ? isLarge : criteria === "old" ? isOld : isLarge || isOld;
}

const KINDS = ["all", "video", "diskImage", "archive", "document", "audio", "image", "other"] as const;
type Criteria = "any" | "large" | "old";

export function LargeFilesPage() {
  const { t } = useTranslation();
  const scan = useStore((s) => s.scans.large);
  const selection = useStore((s) => s.selection.large);
  const settings = useStore((s) => s.settings!);
  const runScan = useStore((s) => s.runScan);
  const setSelected = useStore((s) => s.setSelected);
  const updateSettings = useStore((s) => s.updateSettings);
  const [kind, setKind] = useState<(typeof KINDS)[number]>("all");
  const [criteria, setCriteria] = useState<Criteria>("any");
  const [showThresholds, setShowThresholds] = useState(false);
  const [draft, setDraft] = useState({ large: settings.largeMinMb, oldMin: settings.oldMinMb, oldDays: settings.oldDays });

  const items = scan.status === "done" ? scan.items : [];
  const filtered = useMemo(() => {
    const rules = { largeMinMb: draft.large, oldMinMb: draft.oldMin, oldDays: draft.oldDays };
    return items.filter((i) => largeMatch(i, rules, criteria) && (kind === "all" || i.group === kind));
  }, [items, draft, criteria, kind]);

  if (scan.status !== "done") {
    return (
      <Page id="large">
        <ScanHero
          scanning={scan.status === "scanning"}
          onScan={() => runScan("large")}
          onCancel={() => api().cancelScans()}
          icon={<FileBox className="size-12" strokeWidth={1.6} />}
          title={t("modules.large.heroTitle")}
          hint={t("modules.large.heroHint")}
        />
      </Page>
    );
  }

  const chosen = filtered.filter((i) => selection[i.path] ?? i.selected);
  const meta = (i: Item) => (
    <div className="w-28">
      {i.lastUsed ? (
        <span>{t("large.used", { when: relativeTime(i.lastUsed) })}</span>
      ) : (
        <span className="text-faint">{t("large.modified", { when: relativeTime(i.modified) })}</span>
      )}
    </div>
  );

  return (
    <Page
      id="large"
      actions={
        <>
          <Button variant={showThresholds ? "subtle" : "ghost"} size="sm" icon={<SlidersHorizontal className="size-3.5" />} onClick={() => setShowThresholds(!showThresholds)}>
            {t("large.thresholds")}
          </Button>
          <Button variant="ghost" size="sm" icon={<RotateCw className="size-3.5" />} onClick={() => runScan("large")}>
            {t("common.rescan")}
          </Button>
        </>
      }
    >
      <div className="flex h-full flex-col">
        <div className="shrink-0 space-y-3 px-8 pb-4">
          {showThresholds && (
            <motion.div initial={{ opacity: 0, y: -6 }} animate={{ opacity: 1, y: 0 }} className="grid grid-cols-3 gap-5 rounded-2xl glass px-5 py-4">
              {(
                [
                  ["large", "largeMinMb", 100, 5000, 50, (v: number) => formatBytes(v * 1e6, 0)],
                  ["oldMin", "oldMinMb", LARGE_FLOOR_MB, 2000, 10, (v: number) => formatBytes(v * 1e6, 0)],
                  ["oldDays", "oldDays", 30, 1095, 15, (v: number) => t("large.days", { count: v })],
                ] as const
              ).map(([k, key, min, max, step, fmt]) => (
                <div key={k}>
                  <div className="mb-2 flex justify-between text-[12px]">
                    <span className="text-muted">{t(`large.${k}`)}</span>
                    <span className="font-medium tabular">{fmt(draft[k])}</span>
                  </div>
                  <Slider
                    value={draft[k]}
                    min={min}
                    max={max}
                    step={step}
                    onChange={(v) => setDraft({ ...draft, [k]: v })}
                    onCommit={(v) => updateSettings({ [key]: v })}
                  />
                </div>
              ))}
            </motion.div>
          )}
          <div className="flex items-end justify-between gap-6 rounded-3xl glass px-6 py-5">
            <div>
              <div className="text-[12.5px] text-muted">{t("common.selectedToClean")}</div>
              <BigSize bytes={sum(chosen)} className="text-[44px] leading-none gradient-text" />
              <div className="mt-1.5 text-xs text-faint tabular">
                {t("large.matching", { count: filtered.length, size: formatBytes(sum(filtered)) })}
              </div>
            </div>
            <Button variant="primary" size="lg" disabled={!chosen.length} icon={<Trash2 className="size-4.5" />} onClick={() => startClean("large", chosen)}>
              {t("common.moveToTrash")}
            </Button>
          </div>
          <div className="flex items-center justify-between gap-3">
            <div className="flex flex-wrap gap-1.5">
              {KINDS.map((k) => (
                <button
                  key={k}
                  onClick={() => setKind(k)}
                  className={cn(
                    "rounded-full px-3 py-1 text-[12px] transition-colors",
                    kind === k ? "bg-[var(--accent)] text-white" : "bg-card text-muted hover:text-fg",
                  )}
                >
                  {k === "all" ? t("common.all") : t(`groups.${k}`)}
                </button>
              ))}
            </div>
            <Segmented<Criteria>
              value={criteria}
              onChange={setCriteria}
              options={[
                { value: "any", label: t("large.any") },
                { value: "large", label: t("large.onlyLarge") },
                { value: "old", label: t("large.onlyOld") },
              ]}
            />
          </div>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto px-8 pb-8">
          {filtered.length === 0 ? (
            <Empty icon={<FileBox className="size-10 text-[var(--accent)]" />} title={t("large.none")} hint={t("large.noneHint")} />
          ) : (
            <ItemList items={filtered} grouped={false} selection={selection} onSelect={(p, v) => setSelected("large", p, v)} meta={meta} />
          )}
        </div>
      </div>
    </Page>
  );
}
