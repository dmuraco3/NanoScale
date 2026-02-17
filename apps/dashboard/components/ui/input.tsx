"use client";

import { forwardRef, type InputHTMLAttributes } from "react";
import { clsx } from "clsx";

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  error?: string;
  hint?: string;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(function Input(
  { label, error, hint, className, id, ...props },
  ref
) {
  const inputId = id ?? label?.toLowerCase().replace(/\s+/g, "-");

  return (
    <div className="space-y-1.5">
      {label && (
        <label
          htmlFor={inputId}
          className="block text-sm font-medium text-[var(--foreground)]"
        >
          {label}
        </label>
      )}
      <input
        ref={ref}
        id={inputId}
        className={clsx(
          "w-full rounded-md border bg-[var(--background)] px-3 py-2 text-sm text-[var(--foreground)]",
          "placeholder:text-[var(--foreground-muted)]",
          "transition-colors duration-150",
          "focus:outline-none focus:ring-2 focus:ring-[var(--accent)] focus:ring-offset-1 focus:ring-offset-[var(--background)]",
          "disabled:cursor-not-allowed disabled:opacity-50",
          error
            ? "border-[var(--error)] focus:ring-[var(--error)]"
            : "border-[var(--border)] hover:border-[var(--border-hover)]",
          className
        )}
        {...props}
      />
      {error && (
        <p className="text-sm text-[var(--error)]">{error}</p>
      )}
      {hint && !error && (
        <p className="text-sm text-[var(--foreground-muted)]">{hint}</p>
      )}
    </div>
  );
});
