import { AnimatePresence, motion } from "framer-motion";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "./ui";

export function ScanHero({
  scanning, onScan, onCancel, title, hint, icon, status, size = 220,
}: {
  scanning: boolean;
  onScan: () => void;
  onCancel?: () => void;
  title: ReactNode;
  hint?: ReactNode;
  icon: ReactNode;
  status?: ReactNode;
  size?: number;
}) {
  const { t } = useTranslation();
  return (
    <div className="flex h-full flex-col items-center justify-center gap-8 pb-10">
      <div className="relative" style={{ width: size, height: size }}>
        {/* Halo */}
        <motion.div
          className="absolute -inset-10 rounded-full blur-3xl"
          style={{ background: "radial-gradient(closest-side, var(--accent), transparent)" }}
          animate={{ opacity: scanning ? [0.35, 0.7, 0.35] : 0.35, scale: scanning ? [1, 1.08, 1] : 1 }}
          transition={{ duration: 2.4, repeat: Infinity, ease: "easeInOut" }}
        />
        {/* Rotating conic ring */}
        <div
          className={scanning ? "absolute inset-0 rounded-full animate-spin-fast" : "absolute inset-0 rounded-full animate-spin-slow"}
          style={{
            background: "conic-gradient(from 0deg, transparent 0deg, var(--accent) 120deg, var(--accent-2) 240deg, transparent 360deg)",
            mask: "radial-gradient(farthest-side, transparent calc(100% - 5px), #000 calc(100% - 4px))",
            WebkitMask: "radial-gradient(farthest-side, transparent calc(100% - 5px), #000 calc(100% - 4px))",
          }}
        />
        {/* Orbiting particles while scanning */}
        <AnimatePresence>
          {scanning &&
            [0, 1, 2, 3, 4, 5].map((i) => (
              <motion.div
                key={i}
                className="absolute inset-0"
                initial={{ opacity: 0, rotate: i * 60 }}
                animate={{ opacity: 1, rotate: i * 60 + 360 }}
                exit={{ opacity: 0 }}
                transition={{ rotate: { duration: 3 + i * 0.4, repeat: Infinity, ease: "linear" }, opacity: { duration: 0.4 } }}
              >
                <span
                  className="absolute left-1/2 top-[-6px] size-2 -translate-x-1/2 rounded-full"
                  style={{ background: i % 2 ? "var(--accent)" : "var(--accent-2)", boxShadow: "0 0 12px 2px var(--accent)" }}
                />
              </motion.div>
            ))}
        </AnimatePresence>
        {/* Core button */}
        <motion.button
          onClick={scanning ? undefined : onScan}
          whileHover={scanning ? undefined : { scale: 1.04 }}
          whileTap={scanning ? undefined : { scale: 0.96 }}
          className="absolute inset-[14px] flex flex-col items-center justify-center gap-2 rounded-full text-white"
          style={{
            background: "linear-gradient(145deg, var(--accent), var(--accent-2))",
            boxShadow: "inset 0 2px 20px rgba(255,255,255,0.35), 0 24px 60px -20px var(--accent)",
          }}
        >
          <motion.div animate={scanning ? { scale: [1, 1.12, 1] } : { scale: 1 }} transition={{ duration: 1.2, repeat: Infinity }}>
            {icon}
          </motion.div>
          <span className="text-lg font-semibold tracking-wide">{scanning ? t("common.scanning") : t("common.scan")}</span>
        </motion.button>
      </div>
      <div className="max-w-md text-center">
        <div className="text-xl font-semibold">{title}</div>
        {hint && <div className="mt-2 text-sm text-muted">{hint}</div>}
        {status && <div className="mt-3 h-5 truncate text-xs text-faint tabular">{status}</div>}
      </div>
      {scanning && onCancel && (
        <Button variant="ghost" size="sm" onClick={onCancel}>
          {t("common.cancel")}
        </Button>
      )}
    </div>
  );
}
