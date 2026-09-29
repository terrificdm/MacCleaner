import { Code2, Ghost, Lock, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ScanModulePage } from "@/components/ScanModulePage";
import { Button } from "@/components/ui";
import { api } from "@/lib/api";
import { relativeTime } from "@/lib/format";
import { useStore } from "@/store";

export function SystemJunkPage() {
  return <ScanModulePage page="system" module="system" heroIcon={<Trash2 className="size-12" strokeWidth={1.6} />} />;
}

export function DevJunkPage() {
  const { t } = useTranslation();
  return (
    <ScanModulePage
      page="dev"
      module="dev"
      heroIcon={<Code2 className="size-12" strokeWidth={1.6} />}
      openGroups={2}
      meta={(i) => (i.group === "nodeModules" && i.modified ? t("dev.projectIdle", { when: relativeTime(i.modified) }) : null)}
    />
  );
}

export function FdaBanner({ text }: { text: string }) {
  const { t } = useTranslation();
  const fda = useStore((s) => s.perms.fullDiskAccess);
  if (fda) return null;
  return (
    <div className="mb-3 flex items-center gap-3 rounded-2xl bg-warn/10 px-4 py-3 text-[12.5px] text-warn">
      <Lock className="size-4 shrink-0" />
      <span className="flex-1">{text}</span>
      <Button size="sm" variant="outline" onClick={() => api().openFdaSettings()}>
        {t("perm.open")}
      </Button>
    </div>
  );
}

export function LeftoversPage() {
  const { t } = useTranslation();
  return (
    <ScanModulePage
      page="leftovers"
      module="leftovers"
      heroIcon={<Ghost className="size-12" strokeWidth={1.6} />}
      banner={<FdaBanner text={t("leftovers.fdaHint")} />}
      groupLabel={(g) => <span className="font-mono text-[12.5px]">{g}</span>}
      openGroups={4}
      meta={(i) => (i.modified ? relativeTime(i.modified) : null)}
      emptyHint={t("leftovers.empty")}
    />
  );
}
