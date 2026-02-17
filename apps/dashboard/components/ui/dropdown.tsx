"use client";

import { useState, useRef, type ReactNode } from "react";
import { clsx } from "clsx";

interface DropdownProps {
  trigger: ReactNode;
  children: ReactNode;
  align?: "left" | "right";
}

export function Dropdown({ trigger, children, align = "right" }: DropdownProps) {
  const [isOpen, setOpen] = useState(false);
  const timeoutRef = useRef<NodeJS.Timeout | null>(null);

  function handleMouseEnter() {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
    }
    setOpen(true);
  }

  function handleMouseLeave() {
    timeoutRef.current = setTimeout(() => {
      setOpen(false);
    }, 100);
  }

  function handleClick() {
    setOpen((prev) => !prev);
  }

  return (
    <div
      className="relative inline-block"
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      <div onClick={handleClick}>{trigger}</div>
      {isOpen && (
        <div
          className={clsx(
            "absolute top-full mt-1 z-50 min-w-[160px] rounded-md border border-[var(--border)] bg-[var(--background-secondary)] py-1 shadow-lg",
            "animate-in fade-in-0 zoom-in-95 duration-100",
            align === "right" ? "right-0" : "left-0"
          )}
        >
          {children}
        </div>
      )}
    </div>
  );
}

interface DropdownItemProps {
  children: ReactNode;
  onClick?: () => void;
  disabled?: boolean;
  destructive?: boolean;
}

export function DropdownItem({
  children,
  onClick,
  disabled = false,
  destructive = false,
}: DropdownItemProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      className={clsx(
        "w-full px-3 py-2 text-left text-sm transition-colors",
        "hover:bg-[var(--background-tertiary)]",
        "disabled:opacity-50 disabled:cursor-not-allowed",
        destructive
          ? "text-[var(--error)] hover:text-[var(--error)]"
          : "text-[var(--foreground)]"
      )}
    >
      {children}
    </button>
  );
}

export function DropdownDivider() {
  return <div className="my-1 h-px bg-[var(--border)]" />;
}
