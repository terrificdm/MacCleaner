import React from "react";
import ReactDOM from "react-dom/client";
import "./styles.css";
import "./i18n";
import { applyLanguage } from "./i18n";
import { initApi, isTauri } from "./lib/api";
import { useStore } from "./store";
import App from "./App";

if (isTauri) document.documentElement.classList.add("tauri");

// Browser preview: ?theme=dark|light and ?lang=en|zh-CN to force a variant.
const params = new URLSearchParams(location.search);

initApi()
  .then(() => useStore.getState().loadSettings())
  .then(async () => {
    const st = useStore.getState();
    const theme = params.get("theme");
    const lang = params.get("lang");
    const page = params.get("page");
    if (!isTauri && (theme || lang)) {
      await st.updateSettings({
        ...(theme ? { theme: theme as "light" | "dark" } : {}),
        ...(lang ? { language: lang as "en" | "zh-CN" } : {}),
      });
    }
    if (!isTauri && page) st.go(page as never);
    applyLanguage(useStore.getState().settings!.language);
  })
  .finally(() => {
    ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
      <React.StrictMode>
        <App />
      </React.StrictMode>,
    );
  });
