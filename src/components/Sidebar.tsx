import { motion } from "framer-motion";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { FlaskConical, HardDrive } from "lucide-react";
import { api } from "@/lib/api";
import type { Volume } from "@/lib/types";
import { cn, formatBytes } from "@/lib/format";
import { MODULES } from "@/modules";
import { useStore } from "@/store";
import { AppLogo } from "./AppLogo";

export function Sidebar() {
  const { t } = useTranslation();
  const page = useStore((s) => s.page);
  const go = useStore((s) => s.go);
  const dryRun = useStore((s) => s.settings?.dryRun);
  const [vol, setVol] = useState<Volume | null>(null);

  useEffect(() => {
    const load = () => api().volumeInfo().then(setVol).catch(() => {});
    load();
    const id = setInterval(load, 15000);
    return () => clearInterval(id);
  }, []);

  const sections = ["clean", "apps", "tools"] as const;
  const usedPct = vol && vol.total ? (vol.used / vol.total) * 100 : 0;

  return (
    <aside className="sidebar flex h-full w-[232px] shrink-0 flex-col border-r border-line">
      <div data-tauri-drag-region className="h-[52px] shrink-0" />
      <div data-tauri-drag-region className="flex items-center gap-2.5 px-5 pb-3">
        <AppLogo className="size-8" />
        <div className="pointer-events-none">
          <div className="text-[15px] font-semibold leading-tight">MacCleaner</div>
          <div className="text-[11px] text-muted">{t("app.tagline")}</div>
        </div>
      </div>

      <nav className="flex-1 space-y-3 overflow-y-auto px-3 [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
        {sections.map((sec) => (
          <div key={sec}>
            <div className="px-3 pb-1.5 text-[10.5px] font-semibold uppercase tracking-wider text-faint">{t(`nav.sections.${sec}`)}</div>
            <div className="space-y-0.5">
              {MODULES.filter((m) => m.section === sec).map((m) => {
                const active = page === m.id;
                const Icon = m.icon;
                return (
                  <button
                    key={m.id}
                    onClick={() => go(m.id)}
                    className={cn(
                      "relative flex w-full items-center gap-3 rounded-xl px-3 py-[7px] text-[13px] transition-colors",
                      active ? "text-fg" : "text-muted hover:bg-card hover:text-fg",
                    )}
                  >
                    {active && (
                      <motion.span
                        layoutId="nav-active"
                        className="absolute inset-0 rounded-xl bg-card-hover shadow-sm"
                        transition={{ type: "spring", bounce: 0.18, duration: 0.45 }}
                      />
                    )}
                    <span
                      className="relative flex size-6 items-center justify-center rounded-lg text-white shadow-sm"
                      style={{ background: `linear-gradient(135deg, ${m.from}, ${m.to})` }}
                    >
                      <Icon className="size-3.5" strokeWidth={2.2} />
                    </span>
                    <span className="relative font-medium">{t(`nav.${m.id}`)}</span>
                  </button>
                );
              })}
            </div>
          </div>
        ))}
      </nav>

      <div className="space-y-2 p-4">
        {dryRun && (
          <button
            onClick={() => go("settings")}
            className="flex w-full items-center gap-2 rounded-xl bg-warn/12 px-3 py-2 text-left text-[11.5px] text-warn hover:bg-warn/20"
          >
            <FlaskConical className="size-3.5 shrink-0" />
            <span className="font-medium">{t("dryRun.sidebar")}</span>
          </button>
        )}
        {vol && (
          <div className="rounded-xl bg-card p-3">
            <div className="flex items-center gap-2 text-[11.5px] text-muted">
              <HardDrive className="size-3.5" />
              <span className="truncate">{vol.name}</span>
            </div>
            <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-[var(--line)]">
              <motion.div
                className="h-full rounded-full"
                style={{ background: "linear-gradient(90deg, var(--accent), var(--accent-2))" }}
                initial={{ width: 0 }}
                animate={{ width: `${usedPct}%` }}
                transition={{ duration: 1.2, ease: [0.16, 1, 0.3, 1] }}
              />
            </div>
            <div className="mt-1.5 text-[11px] text-muted tabular">
              {t("sidebar.free", { free: formatBytes(vol.available), total: formatBytes(vol.total, 0) })}
            </div>
          </div>
        )}
      </div>
    </aside>
  );
}
