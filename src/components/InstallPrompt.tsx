import { motion } from "framer-motion";
import { AlertTriangle, ArrowRight, Download } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { api, errorCode } from "@/lib/api";
import type { InstallStatus } from "@/lib/types";
import { useStore } from "@/store";
import { AppLogo } from "./AppLogo";
import { Button, Spinner } from "./ui";

/** Offered when MacCleaner runs from outside the Applications folder. */
export function InstallPrompt() {
  const { t } = useTranslation();
  const [st, setSt] = useState<InstallStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api()
      .installStatus()
      .then((s) => {
        setSt(s);
        if (s.action !== "none") useStore.setState({ installPrompt: true });
      })
      .catch(() => {});
  }, []);

  if (!st || st.action === "none") return null;
  const close = () => {
    setSt(null);
    useStore.setState({ installPrompt: false });
  };
  const run = async () => {
    setBusy(true);
    setError(null);
    try {
      await api().installNow(); // the app quits and reopens from Applications
    } catch (e) {
      setError(errorCode(e));
      setBusy(false);
    }
  };
  const conflict = st.action === "conflict";

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/35 backdrop-blur-md">
      <motion.div
        initial={{ opacity: 0, scale: 0.94, y: 12 }}
        animate={{ opacity: 1, scale: 1, y: 0 }}
        className="w-[520px] max-w-[92vw] rounded-[28px] solid-panel p-8 text-center"
        style={{ ["--accent" as string]: "#8b5cf6", ["--accent-2" as string]: "#ec4899" }}
      >
        <AppLogo className="mx-auto size-16" />
        <h2 className="mt-4 text-xl font-semibold">{t(`install.title.${st.action}`)}</h2>
        {st.installedVersion && !conflict ? (
          <div className="mt-4 flex items-center justify-center gap-3 text-sm tabular">
            <span className="rounded-lg bg-card px-3 py-1.5 text-muted">{t("install.installed", { v: st.installedVersion })}</span>
            <ArrowRight className="size-4 text-faint" />
            <span className="rounded-lg bg-[var(--accent)]/15 px-3 py-1.5 font-medium text-[var(--accent)]">{t("install.thisCopy", { v: st.currentVersion })}</span>
          </div>
        ) : (
          <div className="mt-2 text-sm text-muted tabular">{t("install.thisCopy", { v: st.currentVersion })}</div>
        )}
        <p className="mt-4 text-[13px] leading-relaxed text-muted">{t(`install.desc.${st.action}`)}</p>
        {st.action === "downgrade" && (
          <div className="mt-3 flex items-start gap-2 rounded-xl bg-warn/10 px-3 py-2.5 text-left text-[12.5px] text-warn">
            <AlertTriangle className="mt-0.5 size-4 shrink-0" />
            {t("install.downgradeWarning")}
          </div>
        )}
        {error && <div className="mt-3 text-[12.5px] text-danger">{t(`errors.${error}`, { defaultValue: t("errors.unknown") })}</div>}
        <div className="mt-6 flex justify-center gap-2">
          <Button variant="ghost" onClick={close} disabled={busy}>
            {conflict ? t("common.done") : t("install.later")}
          </Button>
          {!conflict && (
            <Button variant="primary" onClick={run} disabled={busy} icon={busy ? <Spinner /> : <Download className="size-4" />}>
              {t(`install.action.${st.action}`)}
            </Button>
          )}
        </div>
      </motion.div>
    </div>
  );
}
