import { Checkbox as RCheckbox, Dialog as RDialog, Slider as RSlider, Switch as RSwitch, Tooltip as RTooltip } from "radix-ui";
import { Check, Minus, X } from "lucide-react";
import { motion } from "framer-motion";
import { forwardRef, type ButtonHTMLAttributes, type ReactNode } from "react";
import { cn } from "@/lib/format";

type Variant = "primary" | "ghost" | "outline" | "danger" | "subtle";

export const Button = forwardRef<
  HTMLButtonElement,
  ButtonHTMLAttributes<HTMLButtonElement> & { variant?: Variant; size?: "sm" | "md" | "lg"; icon?: ReactNode }
>(({ variant = "outline", size = "md", icon, className, children, ...rest }, ref) => (
  <button
    ref={ref}
    className={cn(
      "inline-flex items-center justify-center gap-2 rounded-xl font-medium transition-all active:scale-[0.97] disabled:opacity-40 disabled:pointer-events-none whitespace-nowrap",
      size === "sm" && "h-7 px-2.5 text-xs rounded-lg",
      size === "md" && "h-9 px-4 text-sm",
      size === "lg" && "h-12 px-7 text-base rounded-2xl",
      variant === "primary" &&
        "text-white bg-gradient-to-r from-[var(--accent)] to-[var(--accent-2)] shadow-[0_8px_24px_-8px_var(--accent)] hover:brightness-110",
      variant === "danger" && "text-white bg-gradient-to-r from-[#ff5a6e] to-[#ff8a4c] shadow-[0_8px_24px_-8px_#ff5a6e] hover:brightness-110",
      variant === "outline" && "glass hover:bg-card-hover text-fg",
      variant === "ghost" && "hover:bg-card-hover text-muted hover:text-fg",
      variant === "subtle" && "bg-card hover:bg-card-hover text-fg",
      className,
    )}
    {...rest}
  >
    {icon}
    {children}
  </button>
));

export function Checkbox({
  checked, onChange, disabled, className,
}: { checked: boolean | "indeterminate"; onChange: (v: boolean) => void; disabled?: boolean; className?: string }) {
  return (
    <RCheckbox.Root
      checked={checked}
      disabled={disabled}
      onCheckedChange={(v) => onChange(v === true)}
      onClick={(e) => e.stopPropagation()}
      className={cn(
        "size-[18px] shrink-0 rounded-[6px] border border-[var(--faint)]/60 flex items-center justify-center transition-colors",
        "data-[state=checked]:border-transparent data-[state=indeterminate]:border-transparent",
        "data-[state=checked]:bg-[var(--accent)] data-[state=indeterminate]:bg-[var(--accent)]",
        "disabled:opacity-30",
        className,
      )}
    >
      <RCheckbox.Indicator>
        {checked === "indeterminate" ? <Minus className="size-3 text-white" strokeWidth={3} /> : <Check className="size-3 text-white" strokeWidth={3} />}
      </RCheckbox.Indicator>
    </RCheckbox.Root>
  );
}

export function Switch({ checked, onChange, disabled }: { checked: boolean; onChange: (v: boolean) => void; disabled?: boolean }) {
  return (
    <RSwitch.Root
      checked={checked}
      disabled={disabled}
      onCheckedChange={onChange}
      className="relative h-6 w-11 shrink-0 rounded-full bg-[var(--line)] transition-colors data-[state=checked]:bg-gradient-to-r data-[state=checked]:from-[var(--accent)] data-[state=checked]:to-[var(--accent-2)]"
    >
      <RSwitch.Thumb className="block size-5 translate-x-0.5 rounded-full bg-white shadow transition-transform data-[state=checked]:translate-x-[22px]" />
    </RSwitch.Root>
  );
}

export function Slider({
  value, min, max, step, onChange, onCommit,
}: { value: number; min: number; max: number; step: number; onChange: (v: number) => void; onCommit?: (v: number) => void }) {
  return (
    <RSlider.Root
      className="relative flex h-5 w-full touch-none select-none items-center"
      value={[value]}
      min={min}
      max={max}
      step={step}
      onValueChange={(v) => onChange(v[0])}
      onValueCommit={(v) => onCommit?.(v[0])}
    >
      <RSlider.Track className="relative h-1.5 grow rounded-full bg-[var(--line)]">
        <RSlider.Range className="absolute h-full rounded-full bg-gradient-to-r from-[var(--accent)] to-[var(--accent-2)]" />
      </RSlider.Track>
      <RSlider.Thumb className="block size-4 rounded-full bg-white shadow-md ring-2 ring-[var(--accent)] outline-none" />
    </RSlider.Root>
  );
}

export function Badge({ children, tone = "neutral", className }: { children: ReactNode; tone?: "neutral" | "accent" | "warn" | "danger" | "ok"; className?: string }) {
  return (
    <span
      className={cn(
        "inline-flex shrink-0 items-center gap-1 rounded-md px-1.5 py-0.5 text-[10.5px] font-medium leading-none whitespace-nowrap",
        tone === "neutral" && "bg-[var(--line)] text-muted",
        tone === "accent" && "bg-[var(--accent)]/15 text-[var(--accent)]",
        tone === "warn" && "bg-warn/15 text-warn",
        tone === "danger" && "bg-danger/15 text-danger",
        tone === "ok" && "bg-ok/15 text-ok",
        className,
      )}
    >
      {children}
    </span>
  );
}

export function Segmented<T extends string>({
  value, options, onChange,
}: { value: T; options: { value: T; label: ReactNode }[]; onChange: (v: T) => void }) {
  return (
    <div className="inline-flex rounded-xl bg-[var(--line)] p-0.5">
      {options.map((o) => (
        <button
          key={o.value}
          onClick={() => onChange(o.value)}
          className={cn(
            "relative rounded-[10px] px-3 py-1.5 text-xs font-medium transition-colors",
            value === o.value ? "text-fg" : "text-muted hover:text-fg",
          )}
        >
          {value === o.value && (
            <motion.span layoutId={`seg-${options.map((x) => x.value).join()}`} className="absolute inset-0 rounded-[10px] solid-panel" transition={{ type: "spring", bounce: 0.2, duration: 0.4 }} />
          )}
          <span className="relative">{o.label}</span>
        </button>
      ))}
    </div>
  );
}

export function Tip({ label, children }: { label: ReactNode; children: ReactNode }) {
  return (
    <RTooltip.Root delayDuration={400}>
      <RTooltip.Trigger asChild>{children}</RTooltip.Trigger>
      <RTooltip.Portal>
        <RTooltip.Content sideOffset={6} className="z-50 rounded-lg solid-panel px-2 py-1 text-xs text-fg">
          {label}
        </RTooltip.Content>
      </RTooltip.Portal>
    </RTooltip.Root>
  );
}

export const TooltipProvider = RTooltip.Provider;

export function Modal({
  open, onClose, title, description, children, footer, width = 560,
}: {
  open: boolean;
  onClose: () => void;
  title: ReactNode;
  description?: ReactNode;
  children?: ReactNode;
  footer?: ReactNode;
  width?: number;
}) {
  return (
    <RDialog.Root open={open} onOpenChange={(o) => !o && onClose()}>
      <RDialog.Portal>
        <RDialog.Overlay className="fixed inset-0 z-40 bg-black/40 backdrop-blur-sm data-[state=open]:animate-in" />
        <RDialog.Content
          style={{ width }}
          className="fixed left-1/2 top-1/2 z-50 max-h-[86vh] max-w-[92vw] -translate-x-1/2 -translate-y-1/2 flex flex-col rounded-3xl solid-panel outline-none"
        >
          <div className="flex items-start justify-between gap-4 px-6 pt-6">
            <div>
              <RDialog.Title className="text-lg font-semibold">{title}</RDialog.Title>
              {description ? (
                <RDialog.Description className="mt-1 text-sm text-muted">{description}</RDialog.Description>
              ) : (
                <RDialog.Description className="sr-only">{String(title)}</RDialog.Description>
              )}
            </div>
            <RDialog.Close className="rounded-lg p-1 text-muted hover:bg-card-hover hover:text-fg">
              <X className="size-4" />
            </RDialog.Close>
          </div>
          <div className="min-h-0 flex-1 overflow-y-auto px-6 py-4">{children}</div>
          {footer && <div className="flex items-center justify-end gap-2 border-t border-line px-6 py-4">{footer}</div>}
        </RDialog.Content>
      </RDialog.Portal>
    </RDialog.Root>
  );
}

export function Spinner({ className }: { className?: string }) {
  return <span className={cn("inline-block size-4 rounded-full border-2 border-current border-t-transparent animate-spin", className)} />;
}

export function Empty({ icon, title, hint }: { icon: ReactNode; title: ReactNode; hint?: ReactNode }) {
  return (
    <div className="flex flex-col items-center justify-center gap-3 py-16 text-center text-muted">
      <div className="animate-float opacity-70">{icon}</div>
      <div className="text-sm font-medium text-fg">{title}</div>
      {hint && <div className="max-w-sm text-xs">{hint}</div>}
    </div>
  );
}
