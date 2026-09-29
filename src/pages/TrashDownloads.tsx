import { motion } from "framer-motion";
import { Download, Lock, RotateCw, Trash, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { BigSize } from "@/components/BigSize";
import { useCleanFlow } from "@/components/CleanFlow";
import { ItemList } from "@/components/ItemList";
import { Page } from "@/components/Page";
import { ScanHero } from "@/components/ScanHero";
import { startClean } from "@/components/ScanModulePage";
import { Button, Empty } from "@/components/ui";
import { api } from "@/lib/api";
import { formatBytes, relativeTime, sum } from "@/lib/format";
import { useStore } from "@/store";

export function scanTrashAndDownloads() {
  const s = useStore.getState();
  const jobs = [s.runScan("downloads")];
  if (s.perms.fullDiskAccess) jobs.push(s.runScan("trash"));
  return Promise.all(jobs);
}

export function emptyTrashFlow(size: number) {
  useCleanFlow.getState().start({
    kind: "emptyTrash",
    items: [],
    size,
    run: () => api().emptyTrash(),
  });
}

export function TrashDownloadsPage() {
  const { t } = useTranslation();
  const trash = useStore((s) => s.scans.trash);
  const dl = useStore((s) => s.scans.downloads);
  const selection = useStore((s) => s.selection.downloads);
  const setSelected = useStore((s) => s.setSelected);
  const fda = useStore((s) => s.perms.fullDiskAccess);

  const scanning = trash.status === "scanning" || dl.status === "scanning";
  if (dl.status !== "done" && !(trash.status === "done" && !scanning)) {
    return (
      <Page id="trash">
        <ScanHero
          scanning={scanning}
          onScan={scanTrashAndDownloads}
          onCancel={() => api().cancelScans()}
          icon={<Download className="size-12" strokeWidth={1.6} />}
          title={t("modules.trash.heroTitle")}
          hint={t("modules.trash.heroHint")}
        />
      </Page>
    );
  }

  const trashItems = trash.status === "done" ? trash.items : [];
  const trashSize = sum(trashItems);
  const dlItems = dl.status === "done" ? dl.items : [];
  const chosen = dlItems.filter((i) => selection[i.path] ?? i.selected);

  return (
    <Page
      id="trash"
      actions={
        <Button variant="ghost" size="sm" icon={<RotateCw className="size-3.5" />} onClick={scanTrashAndDownloads}>
          {t("common.rescan")}
        </Button>
      }
    >
      <div className="h-full overflow-y-auto px-8 pb-8">
        <div className="grid grid-cols-2 gap-4">
          <motion.div initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }} className="relative overflow-hidden rounded-3xl glass p-6">
            <div className="flex items-center gap-2 text-[13px] text-muted">
              <Trash className="size-4" /> {t("trash.trash")}
            </div>
            {fda ? (
              <>
                <BigSize bytes={trashSize} className="mt-2 block text-[40px] leading-none" />
                <div className="mt-1 text-xs text-faint">{t("common.itemCount", { count: trashItems.length })}</div>
                <p className="mt-3 text-[12px] text-muted">{t("trash.hint")}</p>
                <Button className="mt-4" variant="danger" disabled={!trashItems.length} icon={<Trash2 className="size-4" />} onClick={() => emptyTrashFlow(trashSize)}>
                  {t("trash.empty")}
                </Button>
              </>
            ) : (
              <div className="mt-3 flex flex-col items-start gap-3">
                <div className="flex items-center gap-2 text-[13px] text-warn">
                  <Lock className="size-4" /> {t("trash.locked")}
                </div>
                <Button size="sm" variant="outline" onClick={() => api().openFdaSettings()}>
                  {t("perm.open")}
                </Button>
              </div>
            )}
          </motion.div>
          <motion.div initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: 0.05 }} className="rounded-3xl glass p-6">
            <div className="flex items-center gap-2 text-[13px] text-muted">
              <Download className="size-4" /> {t("trash.downloads")}
            </div>
            <BigSize bytes={sum(chosen)} className="mt-2 block text-[40px] leading-none gradient-text" />
            <div className="mt-1 text-xs text-faint tabular">
              {t("common.selectedOf", { count: chosen.length, total: dlItems.length, size: formatBytes(sum(dlItems)) })}
            </div>
            <p className="mt-3 text-[12px] text-muted">{t("trash.downloadsHint")}</p>
            <Button className="mt-4" variant="primary" disabled={!chosen.length} icon={<Trash2 className="size-4" />} onClick={() => startClean("downloads", chosen)}>
              {t("common.moveToTrash")}
            </Button>
          </motion.div>
        </div>
        <div className="mt-5">
          {dlItems.length === 0 ? (
            <Empty icon={<Download className="size-10 text-[var(--accent)]" />} title={t("trash.downloadsEmpty")} />
          ) : (
            <ItemList
              items={dlItems}
              selection={selection}
              onSelect={(p, v) => setSelected("downloads", p, v)}
              openGroups={1}
              meta={(i) => <span className="inline-block w-24">{t("trash.added", { when: relativeTime(i.modified) })}</span>}
            />
          )}
        </div>
      </div>
    </Page>
  );
}
