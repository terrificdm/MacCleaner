import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";
import i18n from "@/i18n";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/** Decimal units, matching Finder. */
export function formatBytes(bytes: number, digits = 1): string {
  if (!bytes || bytes < 0) return "0 KB";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let v = bytes;
  let i = 0;
  while (v >= 1000 && i < units.length - 1) {
    v /= 1000;
    i++;
  }
  if (i === 0) return `${bytes} B`;
  const d = v >= 100 ? 0 : digits;
  return `${v.toFixed(d)} ${units[i]}`;
}

export function splitBytes(bytes: number): [string, string] {
  const s = formatBytes(bytes);
  const [n, u] = s.split(" ");
  return [n, u ?? ""];
}

export const DAY = 86_400_000;

export function daysAgo(ms?: number | null): number | null {
  if (!ms) return null;
  return Math.max(0, Math.floor((Date.now() - ms) / DAY));
}

export function relativeTime(ms?: number | null): string {
  const t = i18n.t.bind(i18n);
  const d = daysAgo(ms);
  if (d === null) return t("common.unknown");
  if (d === 0) return t("time.today");
  if (d === 1) return t("time.yesterday");
  if (d < 30) return t("time.daysAgo", { count: d });
  if (d < 365) return t("time.monthsAgo", { count: Math.floor(d / 30) });
  return t("time.yearsAgo", { count: Math.floor(d / 365) });
}

export function formatDateTime(ms: number): string {
  return new Date(ms).toLocaleString(i18n.language === "en" ? "en-US" : "zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function sum(items: { size: number }[]): number {
  return items.reduce((a, b) => a + b.size, 0);
}

export function shortPath(p: string): string {
  const home = p.match(/^\/Users\/[^/]+/);
  return home ? "~" + p.slice(home[0].length) : p;
}
