import type { ReactNode } from "react";
import { Link } from "@tanstack/react-router";
import { cn } from "@/lib/utils";

interface ListRowProps {
  to: string;
  params: Record<string, string>;
  signalColor?: string;
  name: string;
  badges?: ReactNode;
  subtitle?: ReactNode;
  columns?: ReactNode;
  className?: string;
}

export function ListRow({
  to,
  params,
  signalColor,
  name,
  badges,
  subtitle,
  columns,
  className,
}: ListRowProps) {
  return (
    <Link to={to} params={params}>
      <div
        className={cn(
          "flex items-center gap-3 border-b border-border px-2 py-2 hover:bg-muted/50 transition-colors cursor-pointer",
          className
        )}
      >
        {signalColor && (
          <span
            className={cn("size-2.5 shrink-0 rounded-full", signalColor)}
          />
        )}
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <span className="truncate text-sm font-medium">{name}</span>
            {badges}
          </div>
          {subtitle && (
            <div className="truncate text-xs text-muted-foreground">
              {subtitle}
            </div>
          )}
        </div>
        {columns && (
          <div className="flex shrink-0 items-center gap-4">{columns}</div>
        )}
      </div>
    </Link>
  );
}

/** Right-aligned value + label column helper. */
export function ListColumn({
  value,
  label,
  className,
}: {
  value: ReactNode;
  label?: string;
  className?: string;
}) {
  return (
    <div className={cn("text-right", className)}>
      <div className="text-sm">{value}</div>
      {label && (
        <div className="text-xs text-muted-foreground">{label}</div>
      )}
    </div>
  );
}
