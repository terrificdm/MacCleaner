import {
  Sparkles, PieChart, Trash2, Code2, FileBox, Package, Ghost, Download, History, Settings,
  type LucideIcon,
} from "lucide-react";
import type { PageId } from "@/store";

export interface ModuleMeta {
  id: PageId;
  icon: LucideIcon;
  /** Two gradient stops. */
  from: string;
  to: string;
  section: "clean" | "apps" | "tools";
}

export const MODULES: ModuleMeta[] = [
  { id: "smart", icon: Sparkles, from: "#8b5cf6", to: "#ec4899", section: "clean" },
  { id: "space", icon: PieChart, from: "#06b6d4", to: "#3b82f6", section: "clean" },
  { id: "system", icon: Trash2, from: "#10b981", to: "#06b6d4", section: "clean" },
  { id: "dev", icon: Code2, from: "#f59e0b", to: "#ef4444", section: "clean" },
  { id: "large", icon: FileBox, from: "#6366f1", to: "#0ea5e9", section: "clean" },
  { id: "apps", icon: Package, from: "#f43f5e", to: "#a855f7", section: "apps" },
  { id: "leftovers", icon: Ghost, from: "#a855f7", to: "#6366f1", section: "apps" },
  { id: "trash", icon: Download, from: "#f43f5e", to: "#fb923c", section: "tools" },
  { id: "history", icon: History, from: "#0ea5e9", to: "#8b5cf6", section: "tools" },
  { id: "settings", icon: Settings, from: "#64748b", to: "#8b5cf6", section: "tools" },
];

export const meta = (id: PageId) => MODULES.find((m) => m.id === id)!;
