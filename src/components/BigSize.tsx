import { animate, useMotionValue } from "framer-motion";
import { useEffect, useState } from "react";
import { cn, splitBytes } from "@/lib/format";

/** Animates a byte count and renders it as "12.3 GB" with a smaller unit. */
export function BigSize({ bytes, className, unitClass }: { bytes: number; className?: string; unitClass?: string }) {
  const mv = useMotionValue(0);
  const [v, setV] = useState(0);
  useEffect(() => {
    const c = animate(mv, bytes, { duration: 1.1, ease: [0.16, 1, 0.3, 1], onUpdate: (x) => setV(x) });
    return () => c.stop();
  }, [bytes, mv]);
  const [n, u] = splitBytes(v);
  return (
    <span className={cn("tabular font-semibold tracking-tight", className)}>
      {n}
      <span className={cn("ml-1.5 text-[0.42em] font-medium", unitClass)}>{u}</span>
    </span>
  );
}
