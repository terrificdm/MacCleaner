import { AnimatePresence, motion } from "framer-motion";
import { ChevronRight, FlaskConical, FolderSearch, History, RotateCcw, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Page } from "@/components/Page";
import { Badge, Button, Empty, Modal, Tip } from "@/components/ui";
import { api, errorCode } from "@/lib/api";
import type { CleanResult, HistoryRecord } from "@/lib/types";
import { formatBytes, formatDateTime, shortPath } from "@/lib/format";
import { useStore } from "@/store";

export function HistoryPage() {
  const { t } = useTranslation();
  const [records, setRecords] = useState<HistoryRecord[] | null>(null);
  const [open, setOpen] = useState<string | null>(null);
  const [confirm, setConfirm] = useState(false);
  const [restoring, setRestoring] = useState<HistoryRecord | null>(null);
  const [restoreBusy, setRestoreBusy] = useState(false);
  const [restoreResult, setRestoreResult] = useState<CleanResult | string | null>(null);
  const doRestore = async () => {
    if (!restoring) return;
    setRestoreBusy(true);
    try {
      const r = await api().restore(restoring.id);
      setRestoreResult(r);
      useStore.getState().afterChange(r);
    } catch (e) {
      setRestoreResult(errorCode(e));
    }
    setRestoreBusy(false);
    load();
  };

  const load = () => api().historyList().then(setRecords);
  useEffect(() => {
    load();
  }, []);

  const real = (records ?? []).filter((r) => !r.dryRun);
  // Emptying the Trash would count the same bytes a second time.
  const totalMoved = real.filter((r) => r.module !== "emptyTrash").reduce((a, r) => a + r.total, 0);

  return (
    <Page
      id="history"
      actions={
        <Button variant="ghost" size="sm" icon={<Trash2 className="size-3.5" />} disabled={!records?.length} onClick={() => setConfirm(true)}>
          {t("history.clear")}
        </Button>
      }
    >
      <div className="h-full overflow-y-auto px-8 pb-8">
        {records && records.length > 0 && (
          <div className="mb-4 grid grid-cols-3 gap-4">
            <Stat label={t("history.statTotal")} value={formatBytes(totalMoved)} />
            <Stat label={t("history.statRuns")} value={String(real.length)} />
            <Stat label={t("history.statDry")} value={String(records.length - real.length)} />
          </div>
        )}
        {!records ? null : records.length === 0 ? (
          <Empty icon={<History className="size-10 text-[var(--accent)]" />} title={t("history.empty")} hint={t("history.emptyHint")} />
        ) : (
          <div className="space-y-2">
            {records.map((r) => (
              <div key={r.id} className="overflow-hidden rounded-2xl glass">
                <button className="flex w-full items-center gap-3 px-4 py-3 text-left hover:bg-card-hover" onClick={() => setOpen(open === r.id ? null : r.id)}>
                  <motion.span animate={{ rotate: open === r.id ? 90 : 0 }} className="text-faint">
                    <ChevronRight className="size-4" />
                  </motion.span>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2 text-[13.5px] font-medium">
                      {t(`history.module.${r.module}`, { defaultValue: r.module })}
                      {r.dryRun && (
                        <Badge tone="warn">
                          <FlaskConical className="size-3" /> {t("dryRun.badge")}
                        </Badge>
                      )}
                      {r.failed > 0 && <Badge tone="danger">{t("history.failed", { count: r.failed })}</Badge>}
                    </div>
                    <div className="text-[11.5px] text-faint">{formatDateTime(r.time)}</div>
                  </div>
                  {!r.dryRun && r.restorable > 0 && (
                    <span
                      role="button"
                      className="flex items-center gap-1 rounded-lg px-2 py-1 text-[12px] text-muted hover:bg-card hover:text-fg"
                      onClick={(e) => {
                        e.stopPropagation();
                        setRestoreResult(null);
                        setRestoring(r);
                      }}
                    >
                      <RotateCcw className="size-3.5" /> {t("history.restoreCount", { count: r.restorable })}
                    </span>
                  )}
                  {!r.dryRun && r.superseded > 0 && (
                    <Tip label={t("history.supersededHint")}>
                      <span className="text-[11.5px] text-faint">{t("history.superseded", { count: r.superseded })}</span>
                    </Tip>
                  )}
                  <span className="text-xs text-faint">{t("common.itemCount", { count: r.items.length })}</span>
                  <span className="w-24 text-right text-[13.5px] font-semibold tabular">{formatBytes(r.total)}</span>
                </button>
                <AnimatePresence initial={false}>
                  {open === r.id && (
                    <motion.div initial={{ height: 0 }} animate={{ height: "auto" }} exit={{ height: 0 }} className="border-t border-line">
                      {r.items.map((i) => (
                        <div key={i.path} className="group flex items-center gap-3 px-4 py-2 pl-11 text-[12.5px] hover:bg-card-hover">
                          <div className="min-w-0 flex-1">
                            <div className="truncate selectable">{shortPath(i.path)}</div>
                            {i.trashPath && <div className="truncate text-[11px] text-faint selectable">→ {shortPath(i.trashPath)}</div>}
                          </div>
                          <Badge>{t(`history.method.${i.method}`)}</Badge>
                          {i.trashPath && (
                            <button className="rounded-md p-1 text-muted opacity-0 hover:bg-card hover:text-fg group-hover:opacity-100" onClick={() => api().reveal(i.trashPath!)}>
                              <FolderSearch className="size-3.5" />
                            </button>
                          )}
                          <span className="w-20 text-right tabular text-muted">{formatBytes(i.size)}</span>
                        </div>
                      ))}
                    </motion.div>
                  )}
                </AnimatePresence>
              </div>
            ))}
          </div>
        )}
      </div>
      <Modal
        open={restoring !== null}
        onClose={() => !restoreBusy && setRestoring(null)}
        title={t("history.restoreTitle")}
        description={
          restoring && !restoreResult
            ? t("history.restoreDesc", { count: restoring.restorable })
            : undefined
        }
        footer={
          restoreResult ? (
            <Button variant="primary" onClick={() => setRestoring(null)}>
              {t("common.done")}
            </Button>
          ) : (
            <>
              <Button variant="ghost" onClick={() => setRestoring(null)} disabled={restoreBusy}>
                {t("common.cancel")}
              </Button>
              <Button variant="primary" onClick={doRestore} disabled={restoreBusy} icon={<RotateCcw className="size-4" />}>
                {t("history.restore")}
              </Button>
            </>
          )
        }
      >
        {typeof restoreResult === "string" && <div className="text-[13px] text-danger">{t(`errors.${restoreResult}`, { defaultValue: t("errors.unknown") })}</div>}
        {restoreResult && typeof restoreResult !== "string" && (
          <div className="space-y-2 text-[13px]">
            <div>{t("history.restoreDone", { count: restoreResult.done.length })}</div>
            {(restoreResult.skipped?.length ?? 0) > 0 && (
              <div className="rounded-xl bg-card px-3 py-2.5">
                <div className="font-medium">{t("history.skippedTitle", { count: restoreResult.skipped!.length })}</div>
                <div className="mt-0.5 text-[12px] text-muted">{t("history.supersededHint")}</div>
                <div className="mt-2 max-h-32 space-y-0.5 overflow-y-auto text-[12px] text-faint">
                  {restoreResult.skipped!.map((f) => (
                    <div key={f.path} className="truncate selectable">
                      {shortPath(f.path)}
                    </div>
                  ))}
                </div>
              </div>
            )}
            {restoreResult.failed.length > 0 && (
              <div className="max-h-48 overflow-y-auto rounded-xl border border-line">
                {restoreResult.failed.map((f) => (
                  <div key={f.path} className="border-b border-line px-3 py-2 text-[12px] last:border-0">
                    <div className="truncate selectable">{shortPath(f.path)}</div>
                    <div className="text-danger">{t(`errors.${f.code}`, { defaultValue: t("errors.unknown") })}</div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </Modal>
      <Modal
        open={confirm}
        onClose={() => setConfirm(false)}
        title={t("history.clearTitle")}
        description={t("history.clearDesc")}
        footer={
          <>
            <Button variant="ghost" onClick={() => setConfirm(false)}>
              {t("common.cancel")}
            </Button>
            <Button
              variant="danger"
              onClick={async () => {
                await api().historyClear();
                setConfirm(false);
                load();
              }}
            >
              {t("history.clear")}
            </Button>
          </>
        }
      />
    </Page>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-2xl glass px-5 py-4">
      <div className="text-[12px] text-muted">{label}</div>
      <div className="mt-1 text-2xl font-semibold tabular">{value}</div>
    </div>
  );
}
