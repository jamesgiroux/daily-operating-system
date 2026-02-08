import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

interface StatusBadgeProps {
  value: string;
  styles: Record<string, string>;
  /** Style applied when value doesn't match any key in styles */
  fallback?: string;
  /** Format the display label. Defaults to replacing underscores with spaces. */
  formatLabel?: (value: string) => string;
  className?: string;
}

function defaultFormat(value: string): string {
  return value.replace(/_/g, " ");
}

export function StatusBadge({
  value,
  styles,
  fallback = "",
  formatLabel = defaultFormat,
  className,
}: StatusBadgeProps) {
  return (
    <Badge
      variant="outline"
      className={cn("text-xs", styles[value] ?? fallback, className)}
    >
      {formatLabel(value)}
    </Badge>
  );
}

// --- Pre-configured style maps ---

export const healthStyles: Record<string, string> = {
  green:
    "bg-green-100 text-green-800 border-green-300 dark:bg-green-900/30 dark:text-green-400 dark:border-green-700",
  yellow:
    "bg-yellow-100 text-yellow-800 border-yellow-300 dark:bg-yellow-900/30 dark:text-yellow-400 dark:border-yellow-700",
  red:
    "bg-red-100 text-red-800 border-red-300 dark:bg-red-900/30 dark:text-red-400 dark:border-red-700",
};

export const projectStatusStyles: Record<string, string> = {
  active:
    "bg-green-100 text-green-800 border-green-300 dark:bg-green-900/30 dark:text-green-400 dark:border-green-700",
  on_hold:
    "bg-yellow-100 text-yellow-800 border-yellow-300 dark:bg-yellow-900/30 dark:text-yellow-400 dark:border-yellow-700",
  completed:
    "bg-blue-100 text-blue-800 border-blue-300 dark:bg-blue-900/30 dark:text-blue-400 dark:border-blue-700",
  archived: "bg-muted text-muted-foreground border-muted",
};

export const progressStyles: Record<string, string> = {
  completed:
    "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400",
  in_progress:
    "bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-400",
  planned: "bg-muted text-muted-foreground",
};
