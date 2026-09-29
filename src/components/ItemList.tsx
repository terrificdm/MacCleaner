import { AnimatePresence, motion } from "framer-motion";
import { ChevronRight, Eye, File, Folder, FolderSearch, ShieldPlus, Terminal } from "lucide-react";
import { useEffect, useMemo, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import { api } from "@/lib/api";
import type { Item } from "@/lib/types";
import { cn, formatBytes, shortPath, sum } from "@/lib/format";
import { useStore } from "@/store";
import { Badge, Checkbox, Tip } from "./ui";

const PAGE = 150;

export function noteBadge(note: string | null | undefined, t: TFunction): ReactNode {
  if (!note) return null;
  const tone: Record<string, "warn" | "danger" | "accent" | "neutral"> = {
    running: "warn", apple: "neutral", permanent: "danger", sameVendor: "warn", recentlyUsed: "warn", inUse: "warn", sharedWithCopy: "warn", locked: "warn",
  };
  if (tone[note]) return <Badge tone={tone[note]}>{t(`notes.${note}`)}</Badge>;
  return <Badge>{t(`groups.${note}`, { defaultValue: note })}</Badge>;
}

interface Props {
  items: Item[];
  selection: Record<string, boolean>;
  onSelect: (paths: string[], value: boolean) => void;
  groupLabel?: (g: string) => ReactNode;
  meta?: (i: Item) => ReactNode;
  /** Prefix for the second line (e.g. the location of a related file). */
  sub?: (i: Item) => string;
  grouped?: boolean;
  openGroups?: number;
  canSelect?: (i: Item) => boolean;
}

export function ItemList({ items, selection, onSelect, groupLabel, meta, sub, grouped = true, openGroups = 1, canSelect }: Props) {
  const { t } = useTranslation();
  const [menu, setMenu] = useState<{ x: number; y: number; item: Item } | null>(null);
  const groups = useMemo(() => {
    if (!grouped) return [["", items]] as [string, Item[]][];
    const m = new Map<string, Item[]>();
    for (const i of items) {
      if (!m.has(i.group)) m.set(i.group, []);
      m.get(i.group)!.push(i);
    }
    return [...m.entries()].sort((a, b) => sum(b[1]) - sum(a[1]));
  }, [items, grouped]);

  useEffect(() => {
    if (!menu) return;
    const close = () => setMenu(null);
    window.addEventListener("click", close);
    window.addEventListener("blur", close);
    return () => {
      window.removeEventListener("click", close);
      window.removeEventListener("blur", close);
    };
  }, [menu]);

  const isSel = (i: Item) => selection[i.path] ?? i.selected;

  return (
    <div className="space-y-2.5">
      {groups.map(([g, list], idx) => (
        <Group
          key={g || "all"}
          title={grouped ? (groupLabel ? groupLabel(g) : t(`groups.${g}`, { defaultValue: g })) : null}
          items={list}
          isSel={isSel}
          canSelect={canSelect}
          onSelect={onSelect}
          defaultOpen={!grouped || idx < openGroups || groups.length <= 2}
          meta={meta}
          sub={sub}
          onMenu={(e, item) => {
            e.preventDefault();
            setMenu({ x: e.clientX, y: e.clientY, item });
          }}
        />
      ))}
      {menu && <RowMenu {...menu} onClose={() => setMenu(null)} />}
    </div>
  );
}

function Group({
  title, items, isSel, onSelect, defaultOpen, meta, sub, onMenu, canSelect,
}: {
  sub?: (i: Item) => string;
  title: ReactNode;
  items: Item[];
  isSel: (i: Item) => boolean;
  onSelect: (paths: string[], v: boolean) => void;
  defaultOpen: boolean;
  meta?: (i: Item) => ReactNode;
  onMenu: (e: React.MouseEvent, i: Item) => void;
  canSelect?: (i: Item) => boolean;
}) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(defaultOpen);
  const [limit, setLimit] = useState(PAGE);
  const selectable = items.filter((i) => !canSelect || canSelect(i));
  const selCount = selectable.filter(isSel).length;
  const state = selCount === 0 ? false : selCount === selectable.length ? true : "indeterminate";
  const selSize = sum(selectable.filter(isSel));

  return (
    <div className="overflow-hidden rounded-2xl glass">
      {title !== null && (
        <div className="flex cursor-pointer items-center gap-3 px-4 py-3 hover:bg-card-hover" onClick={() => setOpen(!open)}>
          <Checkbox checked={state} onChange={(v) => onSelect(selectable.map((i) => i.path), v)} disabled={!selectable.length} />
          <motion.span animate={{ rotate: open ? 90 : 0 }} className="text-faint">
            <ChevronRight className="size-4" />
          </motion.span>
          <div className="min-w-0 flex-1 truncate text-[13.5px] font-medium">{title}</div>
          <span className="text-xs text-faint tabular">{t("common.itemCount", { count: items.length })}</span>
          {selCount > 0 && <span className="text-xs text-[var(--accent)] tabular">{formatBytes(selSize)}</span>}
          <span className="w-20 text-right text-[13px] font-semibold tabular">{formatBytes(sum(items))}</span>
        </div>
      )}
      <AnimatePresence initial={false}>
        {open && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.22, ease: "easeOut" }}
            className={cn(title !== null && "border-t border-line")}
          >
            {items.slice(0, limit).map((i) => (
              <Row key={i.path} item={i} indent={title !== null} checked={isSel(i)} disabled={canSelect ? !canSelect(i) : false} onSelect={onSelect} meta={meta} sub={sub} onMenu={onMenu} />
            ))}
            {items.length > limit && (
              <button className="w-full py-2.5 text-xs text-[var(--accent)] hover:bg-card-hover" onClick={() => setLimit(limit + PAGE * 4)}>
                {t("common.showMore", { count: items.length - limit })}
              </button>
            )}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

function Row({
  item, indent, checked, disabled, onSelect, meta, sub, onMenu,
}: {
  sub?: (i: Item) => string;
  item: Item;
  indent: boolean;
  checked: boolean;
  disabled: boolean;
  onSelect: (p: string[], v: boolean) => void;
  meta?: (i: Item) => ReactNode;
  onMenu: (e: React.MouseEvent, i: Item) => void;
}) {
  const { t } = useTranslation();
  const Icon = item.kind === "command" ? Terminal : item.kind === "dir" ? Folder : File;
  const isFs = item.path.startsWith("/");
  return (
    <div
      className={cn("group flex items-center gap-3 px-4 py-2 hover:bg-card-hover", indent && "pl-11")}
      onClick={() => !disabled && onSelect([item.path], !checked)}
      onContextMenu={(e) => isFs && onMenu(e, item)}
    >
      <Checkbox checked={checked} disabled={disabled} onChange={(v) => onSelect([item.path], v)} />
      <Icon className="size-4 shrink-0 text-faint" />
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="min-w-0 truncate text-[13px]">{item.name}</span>
          {noteBadge(item.note, t)}
          {item.needsAdmin && <Badge tone="accent" className="shrink-0">{t("notes.needsAdmin")}</Badge>}
        </div>
        <div className="truncate text-[11px] text-faint selectable">
          {sub && <span className="text-muted">{sub(item)} · </span>}
          {shortPath(item.path)}
        </div>
      </div>
      {meta && <div className="shrink-0 text-right text-[11.5px] text-muted">{meta(item)}</div>}
      {isFs && (
        <div className="hidden shrink-0 gap-0.5 group-hover:flex">
          <Tip label={t("common.quickLook")}>
            <button className="rounded-md p-1 text-muted hover:bg-card hover:text-fg" onClick={(e) => (e.stopPropagation(), api().quickLook(item.path))}>
              <Eye className="size-3.5" />
            </button>
          </Tip>
          <Tip label={t("common.reveal")}>
            <button className="rounded-md p-1 text-muted hover:bg-card hover:text-fg" onClick={(e) => (e.stopPropagation(), api().reveal(item.path))}>
              <FolderSearch className="size-3.5" />
            </button>
          </Tip>
        </div>
      )}
      <span className="w-20 shrink-0 text-right text-[13px] tabular">{formatBytes(item.size)}</span>
    </div>
  );
}

function RowMenu({ x, y, item, onClose }: { x: number; y: number; item: Item; onClose: () => void }) {
  const { t } = useTranslation();
  const loadSettings = useStore((s) => s.loadSettings);
  const entries = [
    { icon: Eye, label: t("common.quickLook"), run: () => api().quickLook(item.path) },
    { icon: FolderSearch, label: t("common.reveal"), run: () => api().reveal(item.path) },
    {
      icon: ShieldPlus,
      label: t("common.addWhitelist"),
      run: async () => {
        await api().addWhitelist(item.path);
        await loadSettings();
        useStore.getState().removeEverywhere([item.path]);
      },
    },
  ];
  const left = Math.min(x, window.innerWidth - 220);
  const top = Math.min(y, window.innerHeight - 130);
  return (
    <div className="fixed z-50 w-52 rounded-xl solid-panel p-1" style={{ left, top }} onClick={(e) => e.stopPropagation()}>
      {entries.map((e) => (
        <button
          key={e.label}
          className="flex w-full items-center gap-2 rounded-lg px-2.5 py-1.5 text-left text-[13px] hover:bg-[var(--accent)] hover:text-white"
          onClick={() => {
            e.run();
            onClose();
          }}
        >
          <e.icon className="size-3.5" />
          {e.label}
        </button>
      ))}
    </div>
  );
}
