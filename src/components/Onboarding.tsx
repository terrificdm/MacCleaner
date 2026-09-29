import { motion } from "framer-motion";
import { CheckCircle2, FlaskConical, Lock, ShieldCheck, Trash2 } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { api } from "@/lib/api";
import { useStore } from "@/store";
import { AppLogo } from "./AppLogo";
import { Button } from "./ui";

export function Onboarding() {
  const { t } = useTranslation();
  const settings = useStore((s) => s.settings);
  const perms = useStore((s) => s.perms);
  const refreshPerms = useStore((s) => s.refreshPerms);
  const update = useStore((s) => s.updateSettings);
  const [checking, setChecking] = useState(false);
  const installPrompt = useStore((s) => s.installPrompt);

  if (!settings || settings.onboardingDone || installPrompt) return null;

  const points = [
    { icon: Trash2, title: t("onboarding.trashTitle"), text: t("onboarding.trashText") },
    { icon: FlaskConical, title: t("onboarding.dryTitle"), text: t("onboarding.dryText") },
    { icon: ShieldCheck, title: t("onboarding.safeTitle"), text: t("onboarding.safeText") },
  ];

  return (
    <div className="fixed inset-0 z-40 flex items-center justify-center bg-black/35 backdrop-blur-md">
      <motion.div
        initial={{ opacity: 0, scale: 0.94, y: 12 }}
        animate={{ opacity: 1, scale: 1, y: 0 }}
        transition={{ type: "spring", bounce: 0.25 }}
        className="w-[640px] max-w-[92vw] overflow-hidden rounded-[28px] solid-panel"
        style={{ ["--accent" as string]: "#8b5cf6", ["--accent-2" as string]: "#ec4899" }}
      >
        <div className="relative px-8 pb-6 pt-8 text-center">
          <div className="pointer-events-none absolute inset-x-0 -top-24 mx-auto h-64 w-[420px] rounded-full bg-gradient-to-r from-[#8b5cf6] to-[#ec4899] opacity-30 blur-3xl" />
          <AppLogo className="relative mx-auto size-20 animate-float" />
          <h2 className="relative mt-4 text-2xl font-semibold">{t("onboarding.title")}</h2>
          <p className="relative mt-1 text-[13px] text-muted">{t("onboarding.subtitle")}</p>
        </div>
        <div className="grid grid-cols-3 gap-3 px-8">
          {points.map((p) => (
            <div key={p.title} className="rounded-2xl bg-card p-4">
              <p.icon className="size-5 text-[var(--accent)]" />
              <div className="mt-2 text-[13px] font-semibold">{p.title}</div>
              <div className="mt-1 text-[11.5px] leading-relaxed text-muted">{p.text}</div>
            </div>
          ))}
        </div>
        <div className="mx-8 mt-4 rounded-2xl border border-line p-4">
          <div className="flex items-center gap-2 text-[13px] font-semibold">
            {perms.fullDiskAccess ? <CheckCircle2 className="size-4 text-ok" /> : <Lock className="size-4 text-warn" />}
            {t("onboarding.fdaTitle")}
          </div>
          {perms.fullDiskAccess ? (
            <div className="mt-1 text-[12px] text-ok">{t("perm.granted")}</div>
          ) : (
            <>
              <ol className="mt-2 list-decimal space-y-1 pl-5 text-[12px] text-muted">
                <li>{t("onboarding.fda1")}</li>
                <li>{t("onboarding.fda2")}</li>
                <li>{t("onboarding.fda3")}</li>
              </ol>
              <div className="mt-3 flex gap-2">
                <Button size="sm" variant="outline" onClick={() => api().openFdaSettings()}>
                  {t("perm.open")}
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  disabled={checking}
                  onClick={async () => {
                    setChecking(true);
                    await refreshPerms();
                    setChecking(false);
                  }}
                >
                  {t("perm.recheck")}
                </Button>
              </div>
            </>
          )}
        </div>
        <div className="flex items-center justify-end gap-2 px-8 py-6">
          {!perms.fullDiskAccess && (
            <Button variant="ghost" onClick={() => update({ onboardingDone: true })}>
              {t("onboarding.later")}
            </Button>
          )}
          <Button variant="primary" onClick={() => update({ onboardingDone: true })}>
            {t("onboarding.start")}
          </Button>
        </div>
      </motion.div>
    </div>
  );
}
