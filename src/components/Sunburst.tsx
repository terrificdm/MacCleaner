import * as d3 from "d3";
import { motion } from "framer-motion";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import type { ViewNode } from "@/lib/types";
import { formatBytes } from "@/lib/format";

const PALETTE = ["#8b5cf6", "#ec4899", "#06b6d4", "#f59e0b", "#10b981", "#3b82f6", "#f43f5e", "#a855f7", "#14b8a6", "#eab308", "#6366f1", "#fb7185"];

export function nodeColor(n: ViewNode, idx: number): string {
  if (n.kind === "system") return "#64748b";
  if (n.kind === "hidden") return "#94a3b8";
  if (n.kind === "other") return "#9ca3af";
  return PALETTE[idx % PALETTE.length];
}

type H = d3.HierarchyRectangularNode<ViewNode>;

export function Sunburst({
  root, size = 460, onZoom, onBack, highlight,
}: {
  root: ViewNode;
  size?: number;
  onZoom: (n: ViewNode) => void;
  onBack?: () => void;
  highlight?: string | null;
}) {
  const { t } = useTranslation();
  const [hover, setHover] = useState<H | null>(null);
  const radius = size / 2;

  const { nodes, levels } = useMemo(() => {
    const h = d3
      .hierarchy<ViewNode>(root, (d) => d.children)
      .sum((d) => Math.max(0, d.size - (d.children ?? []).reduce((a, c) => a + c.size, 0)))
      .sort((a, b) => (b.value ?? 0) - (a.value ?? 0));
    const levels = Math.max(1, h.height);
    const p = d3.partition<ViewNode>().size([2 * Math.PI, levels + 1])(h);
    return { nodes: p.descendants().filter((d) => d.depth > 0 && d.x1 - d.x0 > 0.004), levels };
  }, [root]);

  // Assign a hue per top-level branch.
  const colorOf = useMemo(() => {
    const top = new Map<H, number>();
    let i = 0;
    for (const n of nodes) if (n.depth === 1) top.set(n, i++);
    return (n: H) => {
      let a: H = n;
      while (a.depth > 1 && a.parent) a = a.parent as H;
      const base = d3.color(nodeColor(a.data, top.get(a) ?? 0))!;
      if (n.data.kind === "other") return "#9ca3af";
      const hsl = d3.hsl(base);
      if (n.depth > 1 && n.parent) {
        const sib = n.parent.children?.indexOf(n) ?? 0;
        hsl.h += (sib % 2 ? 1 : -1) * Math.min(24, 6 + sib * 5);
        hsl.l = Math.min(0.74, hsl.l + (n.depth - 1) * 0.07);
      }
      return hsl.formatHex();
    };
  }, [nodes, root]);

  const hole = radius * 0.3;
  const ring = (radius - hole - 6) / levels;
  const arc = d3
    .arc<H>()
    .startAngle((d) => d.x0)
    .endAngle((d) => d.x1)
    .padAngle((d) => Math.min((d.x1 - d.x0) / 2, 0.004))
    .padRadius(radius)
    .innerRadius((d) => hole + (d.y0 - 1) * ring + 1.5)
    .outerRadius((d) => hole + (d.y1 - 1) * ring - 1.5)
    .cornerRadius(4);

  const ancestors = new Set(hover ? hover.ancestors() : []);
  const focus = hover?.data ?? root;

  return (
    <div className="relative" style={{ width: size, height: size }}>
      <motion.svg
        key={root.path}
        width={size}
        height={size}
        viewBox={`${-radius} ${-radius} ${size} ${size}`}
        initial={{ scale: 0.82, opacity: 0, rotate: -25 }}
        animate={{ scale: 1, opacity: 1, rotate: 0 }}
        transition={{ duration: 0.6, ease: [0.16, 1, 0.3, 1] }}
        onMouseLeave={() => setHover(null)}
      >
        <defs>
          <pattern id="hatch" width="6" height="6" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
            <rect width="6" height="6" fill="#94a3b8" opacity="0.35" />
            <line x1="0" y1="0" x2="0" y2="6" stroke="#94a3b8" strokeWidth="3" />
          </pattern>
        </defs>
        {nodes.map((n) => {
          const dim = hover ? !ancestors.has(n) : highlight ? n.data.path !== highlight : false;
          const fill = n.data.kind === "hidden" ? "url(#hatch)" : colorOf(n);
          return (
            <path
              key={n.data.path + n.depth}
              d={arc(n) ?? ""}
              fill={fill}
              opacity={dim ? 0.28 : 1}
              style={{ transition: "opacity 0.18s", cursor: n.data.children?.length ? "pointer" : "default" }}
              onMouseEnter={() => setHover(n)}
              onClick={() => n.data.kind === "dir" && n.data.children?.length && onZoom(n.data)}
            />
          );
        })}
        <circle r={hole - 4} fill="var(--card-solid)" opacity={0.85} style={{ cursor: onBack ? "pointer" : "default" }} onClick={onBack} />
      </motion.svg>
      <div className="pointer-events-none absolute inset-0 flex flex-col items-center justify-center text-center">
        <div className="max-w-[26%] truncate text-[12px] text-muted">{labelOf(focus, t)}</div>
        <div className="text-2xl font-semibold tabular">{formatBytes(focus.size)}</div>
        {hover && root.size > 0 && <div className="text-[11px] text-faint tabular">{((focus.size / root.size) * 100).toFixed(1)}%</div>}
        {!hover && onBack && <div className="mt-0.5 text-[10.5px] text-faint">{t("space.clickBack")}</div>}
      </div>
    </div>
  );
}

export function labelOf(n: ViewNode, t: TFunction): string {
  if (n.kind === "system") return t("space.system");
  if (n.kind === "hidden") return t("space.hidden");
  if (n.kind === "other") return t("space.other");
  return n.name;
}
