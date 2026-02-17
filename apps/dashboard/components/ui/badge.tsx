import { type ReactNode } from "react";
import { clsx } from "clsx";

type BadgeVariant = "default" | "success" | "error" | "warning" | "secondary";

interface BadgeProps {
  children: ReactNode;
  variant?: BadgeVariant;
  className?: string;
  dot?: boolean;
}

const variantStyles: Record<BadgeVariant, string> = {
  default: "bg-[var(--accent)]/10 text-[var(--accent)] border-[var(--accent)]/20",
  success: "bg-[var(--success)]/10 text-[var(--success)] border-[var(--success)]/20",
  error: "bg-[var(--error)]/10 text-[var(--error)] border-[var(--error)]/20",
  warning: "bg-[var(--warning)]/10 text-[var(--warning)] border-[var(--warning)]/20",
  secondary: "bg-[var(--background-tertiary)] text-[var(--foreground-secondary)] border-[var(--border)]",
};

const dotStyles: Record<BadgeVariant, string> = {
  default: "bg-[var(--accent)]",
  success: "bg-[var(--success)]",
  error: "bg-[var(--error)]",
  warning: "bg-[var(--warning)]",
  secondary: "bg-[var(--foreground-muted)]",
};

export function Badge({ children, variant = "default", className, dot = false }: BadgeProps) {
  return (
    <span
      className={clsx(
        "inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-xs font-medium",
        variantStyles[variant],
        className
      )}
    >
      {dot && (
        <span className={clsx("h-1.5 w-1.5 rounded-full", dotStyles[variant])} />
      )}
      {children}
    </span>
  );
}
