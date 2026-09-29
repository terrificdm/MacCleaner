import { motion } from "framer-motion";
import { AlertTriangle, CheckCircle2, FlaskConical, KeyRound, Terminal, Trash2, XCircle } from "lucide-react";
import { useMemo, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { create } from "zustand";
import { errorCode } from "@/lib/api";
import type { CleanResult, Item } from "@/lib/types";
import { formatBytes, shortPath, sum } from "@/lib/format";
import { useStore } from "@/store";
import { BigSize } from "./BigSize";
import { Badge, Button, Checkbox, Modal, Spinner } from "./ui";

export interface CleanRequest {
  kind: "clean" | "uninstall" | "emptyTrash";
  title?: ReactNode;
  items: Item[];
  /** Paths that must stay selected (e.g. the app bundle during uninstall). */
  locked?: string[];
  notes?: ReactNode[];
  /** Size shown for emptyTrash confirmations. */
  size?: number;
  run: (items: Item[]) => Promise<CleanResult>;
  onDone?: (r: CleanResult) => void;
}

type Phase = "confirm" | "running" | "result";

interface FlowState {
  req: CleanRequest | null;
  phase: Phase;
  result: CleanResult | null;
  error: string | null;
  start(r: CleanRequest): void;
  close(): void;
}

export const useCleanFlow = create<FlowState>((set) => ({
  req: null,
  phase: "confirm",
  result: null,
  error: null,
  start: (req) => set({ req, phase: "confirm", result: null, error: null }),
  close: () => set({ req: null }),
}));

export function CleanDialogs() {
  const { req, phase, result, error, close } = useCleanFlow();
  if (!req) return null;
  return phase === "result" ? (
    <ResultDialog req={req} result={result} error={error} onClose={close} />
  ) : (
    <ConfirmDialog key={String(req.items.length) + req.kind} req={req} running={phase === "running"} onClose={close} />
  );
}

function ConfirmDialog({ req, running, onClose }: { req: CleanRequest; running: boolean; onClose: () => void }) {
  const { t } = useTranslation();
  const dryRun = useStore((s) => s.settings?.dryRun ?? true);
  const [sel, setSel] = useState<Record<string, boolean>>(() => Object.fromEntries(req.items.map((i) => [i.path, true])));
  const locked = new Set(req.locked ?? []);
  const chosen = req.items.filter((i) => sel[i.path] || locked.has(i.path));
  const total = req.kind === "emptyTrash" ? req.size ?? 0 : sum(chosen);
  const hasPermanent = chosen.some((i) => i.kind === "command");
  const hasAdmin = chosen.some((i) => i.needsAdmin);

  const go = async () => {
    useCleanFlow.setState({ phase: "running" });
    try {
      const r = await req.run(chosen);
      useCleanFlow.setState({ phase: "result", result: r });
      req.onDone?.(r);
      useStore.getState().afterChange(r);
    } catch (e) {
      useCleanFlow.setState({ phase: "result", error: errorCode(e) });
    }
  };

  const empty = req.kind === "emptyTrash";
  const title = req.title ?? (empty ? t("confirm.emptyTitle") : req.kind === "uninstall" ? t("confirm.uninstallTitle") : t("confirm.title"));

  return (
    <Modal
      open
      onClose={running ? () => {} : onClose}
      title={title}
      description={
        empty
          ? t("confirm.emptyDesc", { size: formatBytes(total) })
          : t("confirm.desc", { count: chosen.length, size: formatBytes(total) })
      }
      width={620}
      footer={
        <>
          <Button variant="ghost" onClick={onClose} disabled={running}>
            {t("common.cancel")}
          </Button>
          <Button
            variant={empty ? "danger" : "primary"}
            onClick={go}
            disabled={running || (!empty && chosen.length === 0)}
            icon={running ? <Spinner /> : dryRun ? <FlaskConical className="size-4" /> : <Trash2 className="size-4" />}
          >
            {dryRun ? t("confirm.actionDry") : empty ? t("confirm.actionEmpty") : req.kind === "uninstall" ? t("confirm.actionUninstall") : t("confirm.action")}
          </Button>
        </>
      }
    >
      <div className="space-y-3">
        {dryRun && (
          <Notice icon={<FlaskConical className="size-4" />} tone="warn">
            {t("confirm.dryRunNote")}
          </Notice>
        )}
        {!dryRun && !empty && <Notice icon={<Trash2 className="size-4" />}>{t("confirm.trashNote")}</Notice>}
        {empty && (
          <Notice icon={<AlertTriangle className="size-4" />} tone="danger">
            {t("confirm.emptyWarning")}
          </Notice>
        )}
        {hasPermanent && (
          <Notice icon={<Terminal className="size-4" />} tone="danger">
            {t("confirm.permanentNote")}
          </Notice>
        )}
        {hasAdmin && <Notice icon={<KeyRound className="size-4" />}>{t("confirm.adminNote")}</Notice>}
        {req.notes?.map((n, i) => (
          <Notice key={i} icon={<AlertTriangle className="size-4" />} tone="warn">
            {n}
          </Notice>
        ))}
        {!empty && (
          <div className="max-h-[340px] overflow-y-auto rounded-2xl border border-line">
            {req.items.map((i) => {
              const isLocked = locked.has(i.path);
              return (
                <label key={i.path} className="flex items-center gap-3 border-b border-line px-3 py-2 last:border-0 hover:bg-card">
                  <Checkbox
                    checked={isLocked || !!sel[i.path]}
                    disabled={isLocked || running}
                    onChange={(v) => setSel({ ...sel, [i.path]: v })}
                  />
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2 truncate text-[13px]">
                      {i.name}
                      {i.kind === "command" && <Badge tone="danger">{t("notes.permanent")}</Badge>}
                      {i.needsAdmin && <Badge tone="accent">{t("notes.needsAdmin")}</Badge>}
                    </div>
                    <div className="truncate text-[11px] text-faint selectable">{shortPath(i.path)}</div>
                  </div>
                  <span className="text-[12.5px] tabular text-muted">{formatBytes(i.size)}</span>
                </label>
              );
            })}
          </div>
        )}
        {running && (
          <div className="relative h-1 overflow-hidden rounded-full bg-[var(--line)]">
            <div className="absolute inset-0 shimmer" />
          </div>
        )}
      </div>
    </Modal>
  );
}

function Notice({ icon, children, tone = "neutral" }: { icon: ReactNode; children: ReactNode; tone?: "neutral" | "warn" | "danger" }) {
  const cls = tone === "warn" ? "bg-warn/10 text-warn" : tone === "danger" ? "bg-danger/10 text-danger" : "bg-card text-muted";
  return (
    <div className={`flex items-start gap-2.5 rounded-xl px-3 py-2.5 text-[12.5px] leading-relaxed ${cls}`}>
      <span className="mt-0.5 shrink-0">{icon}</span>
      <span>{children}</span>
    </div>
  );
}

function ResultDialog({ req, result, error, onClose }: { req: CleanRequest; result: CleanResult | null; error: string | null; onClose: () => void }) {
  const { t } = useTranslation();
  const go = useStore((s) => s.go);
  const done = result?.done ?? [];
  const failed = result?.failed ?? [];
  const dry = result?.dryRun ?? false;
  const empty = req.kind === "emptyTrash";
  const movedToTrash = useMemo(() => done.some((d) => d.method === "trash" || d.method === "finder"), [done]);
  // Command items (brew/docker) delete directly instead of moving to the Trash.
  const commandPaths = useMemo(() => new Set(req.items.filter((i) => i.kind === "command").map((i) => i.path)), [req.items]);
  const isCommand = (p: string) => commandPaths.has(p) || p.startsWith("brew ") || p.startsWith("docker ");
  const dryCmd = done.filter((d) => isCommand(d.path)).length;

  const headline = error
    ? t(`errors.${error}`, { defaultValue: t("errors.unknown") })
    : dry
      ? t("result.dryTitle")
      : empty
        ? t("result.emptied")
        : movedToTrash
          ? t("result.movedTitle")
          : t("result.doneTitle");

  return (
    <Modal
      open
      onClose={onClose}
      title={t("result.title")}
      width={560}
      footer={
        <>
          {!dry && movedToTrash && !empty && (
            <Button
              variant="outline"
              onClick={() => {
                onClose();
                go("trash");
              }}
            >
              {t("result.goTrash")}
            </Button>
          )}
          <Button variant="primary" onClick={onClose}>
            {t("common.done")}
          </Button>
        </>
      }
    >
      <div className="flex flex-col items-center gap-3 py-2 text-center">
        <motion.div initial={{ scale: 0.4, opacity: 0 }} animate={{ scale: 1, opacity: 1 }} transition={{ type: "spring", bounce: 0.5 }}>
          {error ? (
            <XCircle className="size-14 text-danger" />
          ) : dry ? (
            <FlaskConical className="size-14 text-warn" />
          ) : (
            <CheckCircle2 className="size-14 text-ok" />
          )}
        </motion.div>
        <div className="text-base font-semibold">{headline}</div>
        {!error && <BigSize bytes={result?.total ?? 0} className="text-4xl gradient-text" />}
        {!error && (
          <div className="text-[13px] text-muted">
            {dry && empty
              ? t("result.dryEmptyDesc")
              : dry
              ? [done.length - dryCmd > 0 ? t("result.dryDesc", { count: done.length - dryCmd }) : "", dryCmd > 0 ? t("result.dryCmdDesc", { count: dryCmd }) : ""]
                  .filter(Boolean)
                  .join(" ")
              : empty
                ? t("result.emptiedDesc")
                : movedToTrash
                  ? t("result.trashHint", { count: done.length })
                  : t("result.doneDesc", { count: done.length })}
          </div>
        )}
      </div>
      {failed.length > 0 && (
        <div className="mt-4">
          <div className="mb-2 text-[12.5px] font-medium text-danger">{t("result.failed", { count: failed.length })}</div>
          <div className="max-h-48 overflow-y-auto rounded-xl border border-line">
            {failed.map((f) => (
              <div key={f.path} className="border-b border-line px-3 py-2 text-[12px] last:border-0">
                <div className="truncate selectable">{shortPath(f.path)}</div>
                <div className="text-danger">
                  {t(`errors.${f.code}`, { defaultValue: t("errors.unknown") })}
                  {f.detail && <span className="ml-1 text-faint selectable">({f.detail})</span>}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </Modal>
  );
}
