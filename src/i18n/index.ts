import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import zh from "./zh-CN.json";
import en from "./en.json";
import type { Language } from "@/lib/types";

export function resolveLanguage(lang: Language): "zh-CN" | "en" {
  if (lang === "zh-CN" || lang === "en") return lang;
  const sys = (navigator.languages?.[0] ?? navigator.language ?? "en").toLowerCase();
  return sys.startsWith("zh") ? "zh-CN" : "en";
}

i18n.use(initReactI18next).init({
  resources: { "zh-CN": { translation: zh }, en: { translation: en } },
  lng: resolveLanguage("system"),
  fallbackLng: "en",
  interpolation: { escapeValue: false },
  returnNull: false,
});

export function applyLanguage(lang: Language) {
  const l = resolveLanguage(lang);
  if (i18n.language !== l) i18n.changeLanguage(l);
  document.documentElement.lang = l;
}

export default i18n;
