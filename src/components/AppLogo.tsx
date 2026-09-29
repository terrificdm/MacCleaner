import logo from "@/assets/logo.svg";
import { cn } from "@/lib/format";

/** The icon artwork has Apple's standard padding; scale it up in-app. */
export function AppLogo({ className }: { className?: string }) {
  return (
    <span className={cn("relative inline-block overflow-visible", className)}>
      <img src={logo} alt="" draggable={false} className="absolute inset-0 size-full scale-[1.24]" />
    </span>
  );
}
