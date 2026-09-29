import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";
import { meta } from "@/modules";
import type { PageId } from "@/store";
import { cn } from "@/lib/format";

export function Page({
  id, actions, children, className, hideHeader,
}: { id: PageId; actions?: ReactNode; children: ReactNode; className?: string; hideHeader?: boolean }) {
  const { t } = useTranslation();
  const m = meta(id);
  const Icon = m.icon;
  return (
    <div className="relative flex h-full min-w-0 flex-1 flex-col overflow-hidden" style={{ ["--accent" as string]: m.from, ["--accent-2" as string]: m.to }}>
      {/* Accent glow behind the content */}
      <div className="pointer-events-none absolute inset-0 overflow-hidden">
        <div
          className="absolute -left-40 -top-56 h-[520px] w-[720px] rounded-full blur-[110px]"
          style={{ background: `radial-gradient(closest-side, ${m.from}, transparent)`, opacity: "var(--glow-opacity)" }}
        />
        <div
          className="absolute -right-48 top-24 h-[420px] w-[520px] rounded-full blur-[120px]"
          style={{ background: `radial-gradient(closest-side, ${m.to}, transparent)`, opacity: "calc(var(--glow-opacity) * 0.6)" }}
        />
      </div>
      <div data-tauri-drag-region className="relative h-[52px] shrink-0" />
      {!hideHeader && (
        <header data-tauri-drag-region className="relative flex shrink-0 items-center gap-4 px-8 pb-5">
          <motion.div
            initial={{ scale: 0.6, opacity: 0 }}
            animate={{ scale: 1, opacity: 1 }}
            className="pointer-events-none flex size-11 items-center justify-center rounded-2xl text-white shadow-lg"
            style={{ background: `linear-gradient(135deg, ${m.from}, ${m.to})`, boxShadow: `0 10px 30px -10px ${m.from}` }}
          >
            <Icon className="size-5.5" strokeWidth={2.1} />
          </motion.div>
          <div className="pointer-events-none min-w-0 flex-1">
            <h1 className="text-[22px] font-semibold leading-tight tracking-tight">{t(`nav.${id}`)}</h1>
            <p className="truncate text-[13px] text-muted">{t(`modules.${id}.subtitle`)}</p>
          </div>
          <div className="flex items-center gap-2">{actions}</div>
        </header>
      )}
      <div className={cn("relative min-h-0 flex-1", className)}>{children}</div>
    </div>
  );
}
