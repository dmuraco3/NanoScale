"use client";

import { useState } from "react";
import { usePathname } from "next/navigation";
import Link from "next/link";
import { clsx } from "clsx";
import {
  Search,
  Moon,
  Sun,
  User,
  LogOut,
  ChevronRight,
} from "lucide-react";
import { useTheme } from "@/components/theme-provider";
import { Dropdown, DropdownItem } from "@/components/ui/dropdown";
import { clientApiBaseUrl } from "@/lib/api-base-url";

const breadcrumbMap: Record<string, string> = {
  "/": "Overview",
  "/projects": "Projects",
  "/projects/new": "New Project",
  "/servers": "Servers",
  "/settings": "Settings",
  "/help": "Help",
};

export function Header() {
  const pathname = usePathname();
  const { setTheme, resolvedTheme } = useTheme();
  const [searchQuery, setSearchQuery] = useState("");

  const breadcrumbs = generateBreadcrumbs(pathname);

  function generateBreadcrumbs(path: string): { label: string; href: string }[] {
    const segments = path.split("/").filter(Boolean);
    const crumbs: { label: string; href: string }[] = [];

    let currentPath = "";
    for (const segment of segments) {
      currentPath += `/${segment}`;
      const label = breadcrumbMap[currentPath] ?? segment;
      crumbs.push({ label, href: currentPath });
    }

    return crumbs;
  }

  async function handleLogout() {
    await fetch(`${clientApiBaseUrl()}/api/auth/logout`, {
      method: "POST",
      credentials: "include",
    });
    window.location.assign("/login");
  }

  function toggleTheme() {
    setTheme(resolvedTheme === "dark" ? "light" : "dark");
  }

  return (
    <header className="sticky top-0 z-30 flex h-14 items-center justify-between border-b border-[var(--border)] bg-[var(--background)]/80 px-6 backdrop-blur-sm">
      {/* Breadcrumbs */}
      <nav className="flex items-center gap-1 text-sm">
        <Link
          href="/"
          className="text-[var(--foreground-muted)] transition-colors hover:text-[var(--foreground)]"
        >
          Home
        </Link>
        {breadcrumbs.map((crumb) => (
          <span key={crumb.href} className="flex items-center gap-1">
            <ChevronRight className="h-4 w-4 text-[var(--foreground-muted)]" />
            <Link
              href={crumb.href}
              className="text-[var(--foreground)] font-medium"
            >
              {crumb.label}
            </Link>
          </span>
        ))}
      </nav>

      {/* Right side */}
      <div className="flex items-center gap-3">
        {/* Search */}
        <div className="relative">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-[var(--foreground-muted)]" />
          <input
            type="text"
            placeholder="Search... (⌘K)"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className={clsx(
              "h-9 w-64 rounded-md border border-[var(--border)] bg-[var(--background-secondary)] pl-9 pr-3 text-sm",
              "placeholder:text-[var(--foreground-muted)]",
              "focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
            )}
          />
        </div>

        {/* Theme toggle */}
        <button
          onClick={toggleTheme}
          className="flex h-9 w-9 items-center justify-center rounded-md border border-[var(--border)] bg-[var(--background-secondary)] text-[var(--foreground-secondary)] transition-colors hover:text-[var(--foreground)]"
          aria-label="Toggle theme"
        >
          {resolvedTheme === "dark" ? (
            <Sun className="h-4 w-4" />
          ) : (
            <Moon className="h-4 w-4" />
          )}
        </button>

        {/* User menu */}
        <Dropdown
          trigger={
            <button className="flex h-9 w-9 items-center justify-center rounded-full bg-[var(--background-tertiary)] text-[var(--foreground-secondary)] transition-colors hover:text-[var(--foreground)]">
              <User className="h-4 w-4" />
            </button>
          }
        >
          <div className="px-3 py-2 border-b border-[var(--border)]">
            <p className="text-sm font-medium text-[var(--foreground)]">Admin</p>
            <p className="text-xs text-[var(--foreground-muted)]">admin@nanoscale.local</p>
          </div>
          <DropdownItem onClick={handleLogout}>
            <span className="flex items-center gap-2">
              <LogOut className="h-4 w-4" />
              Sign out
            </span>
          </DropdownItem>
        </Dropdown>
      </div>
    </header>
  );
}
