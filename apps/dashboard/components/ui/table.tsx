import { type ReactNode, type ThHTMLAttributes, type TdHTMLAttributes } from "react";
import { clsx } from "clsx";

interface TableProps {
  children: ReactNode;
  className?: string;
}

export function Table({ children, className }: TableProps) {
  return (
    <div className={clsx("overflow-x-auto rounded-lg border border-[var(--border)]", className)}>
      <table className="min-w-full divide-y divide-[var(--border)]">
        {children}
      </table>
    </div>
  );
}

interface TableHeaderProps {
  children: ReactNode;
}

export function TableHeader({ children }: TableHeaderProps) {
  return (
    <thead className="bg-[var(--background-tertiary)]">
      {children}
    </thead>
  );
}

interface TableBodyProps {
  children: ReactNode;
}

export function TableBody({ children }: TableBodyProps) {
  return (
    <tbody className="divide-y divide-[var(--border)] bg-[var(--background-secondary)]">
      {children}
    </tbody>
  );
}

interface TableRowProps {
  children: ReactNode;
  className?: string;
  onClick?: () => void;
}

export function TableRow({ children, className, onClick }: TableRowProps) {
  return (
    <tr
      className={clsx(
        "transition-colors",
        onClick && "cursor-pointer",
        "hover:bg-[var(--background-tertiary)]/50",
        className
      )}
      onClick={onClick}
    >
      {children}
    </tr>
  );
}

interface TableHeadProps extends ThHTMLAttributes<HTMLTableCellElement> {
  children?: ReactNode;
}

export function TableHead({ children, className, ...props }: TableHeadProps) {
  return (
    <th
      className={clsx(
        "px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-[var(--foreground-muted)]",
        className
      )}
      {...props}
    >
      {children}
    </th>
  );
}

interface TableCellProps extends TdHTMLAttributes<HTMLTableCellElement> {
  children?: ReactNode;
}

export function TableCell({ children, className, ...props }: TableCellProps) {
  return (
    <td
      className={clsx("px-4 py-3 text-sm text-[var(--foreground)]", className)}
      {...props}
    >
      {children}
    </td>
  );
}

interface TableEmptyProps {
  colSpan: number;
  message?: string;
  icon?: ReactNode;
}

export function TableEmpty({ colSpan, message = "No data available", icon }: TableEmptyProps) {
  return (
    <tr>
      <td colSpan={colSpan} className="px-4 py-12 text-center">
        <div className="flex flex-col items-center gap-3">
          {icon && (
            <div className="text-[var(--foreground-muted)]">{icon}</div>
          )}
          <p className="text-sm text-[var(--foreground-muted)]">{message}</p>
        </div>
      </td>
    </tr>
  );
}
