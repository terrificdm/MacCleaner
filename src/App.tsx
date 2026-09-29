import { AnimatePresence, motion } from "framer-motion";
import { useEffect, useState } from "react";
import { applyLanguage } from "@/i18n";
import { isTauri } from "@/lib/api";
import type { Theme } from "@/lib/types";
import { useStore, type PageId } from "@/store";
import { CleanDialogs } from "@/components/CleanFlow";
import { Onboarding } from "@/components/Onboarding";
import { InstallPrompt } from "@/components/InstallPrompt";
import { Sidebar } from "@/components/Sidebar";
import { TooltipProvider } from "@/components/ui";
import { SmartScanPage } from "@/pages/SmartScan";
import { SpaceLensPage } from "@/pages/SpaceLens";
import { DevJunkPage, LeftoversPage, SystemJunkPage } from "@/pages/ListPages";
import { LargeFilesPage } from "@/pages/LargeFiles";
import { UninstallerPage } from "@/pages/Uninstaller";
import { TrashDownloadsPage } from "@/pages/TrashDownloads";
import { HistoryPage } from "@/pages/History";
import { SettingsPage } from "@/pages/Settings";

const PAGES: Record<PageId, () => React.ReactElement> = {
  smart: SmartScanPage,
  space: SpaceLensPage,
  system: SystemJunkPage,
  dev: DevJunkPage,
  large: LargeFilesPage,
  apps: UninstallerPage,
  leftovers: LeftoversPage,
  trash: TrashDownloadsPage,
  history: HistoryPage,
  settings: SettingsPage,
};

function useSystemDark() {
  const q = window.matchMedia("(prefers-color-scheme: dark)");
  const [dark, setDark] = useState(q.matches);
  useEffect(() => {
    const on = (e: MediaQueryListEvent) => setDark(e.matches);
    q.addEventListener("change", on);
    return () => q.removeEventListener("change", on);
  }, [q]);
  return dark;
}

function useApplyTheme(theme: Theme | undefined) {
  const sysDark = useSystemDark();
  useEffect(() => {
    if (!theme) return;
    const dark = theme === "dark" || (theme === "system" && sysDark);
    document.documentElement.dataset.theme = dark ? "dark" : "light";
    if (isTauri) {
      import("@tauri-apps/api/window").then(({ getCurrentWindow }) =>
        getCurrentWindow()
          .setTheme(theme === "system" ? null : theme)
          .catch(() => {}),
      );
    }
  }, [theme, sysDark]);
}

export default function App() {
  const page = useStore((s) => s.page);
  const settings = useStore((s) => s.settings);
  const refreshPerms = useStore((s) => s.refreshPerms);

  useApplyTheme(settings?.theme);
  useEffect(() => {
    if (settings) applyLanguage(settings.language);
  }, [settings?.language]);

  // Permissions change in System Settings while the app is open.
  useEffect(() => {
    refreshPerms();
    const on = () => refreshPerms();
    window.addEventListener("focus", on);
    return () => window.removeEventListener("focus", on);
  }, [refreshPerms]);

  const Current = PAGES[page];
  return (
    <TooltipProvider>
      <div className="flex h-full">
        <Sidebar />
        <main className="main-surface relative flex min-w-0 flex-1">
          <AnimatePresence mode="wait">
            <motion.div
              key={page}
              className="flex min-w-0 flex-1"
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -6 }}
              transition={{ duration: 0.2, ease: "easeOut" }}
            >
              <Current />
            </motion.div>
          </AnimatePresence>
        </main>
      </div>
      <CleanDialogs />
      <Onboarding />
      <InstallPrompt />
    </TooltipProvider>
  );
}
