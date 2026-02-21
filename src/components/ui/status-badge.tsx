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
    "bg-[var(--color-garden-sage-12)] text-[var(--color-garden-rosemary)] border-[var(--color-garden-sage-30)]",
  yellow:
    "bg-[var(--color-spice-saffron-12)] text-[var(--color-desk-espresso)] border-[var(--color-spice-saffron-30)]",
  red:
    "bg-[var(--color-spice-terracotta-12)] text-[var(--color-spice-chili)] border-[var(--color-spice-terracotta-30)]",
};

export const projectStatusStyles: Record<string, string> = {
  active:
    "bg-[var(--color-garden-sage-12)] text-[var(--color-garden-rosemary)] border-[var(--color-garden-sage-30)]",
  on_hold:
    "bg-[var(--color-spice-saffron-12)] text-[var(--color-desk-espresso)] border-[var(--color-spice-saffron-30)]",
  completed:
    "bg-[var(--color-garden-larkspur-12)] text-[var(--color-desk-ink)] border-[var(--color-garden-larkspur-30)]",
  archived: "bg-muted text-muted-foreground border-muted",
};

export const progressStyles: Record<string, string> = {
  completed:
    "bg-[var(--color-garden-sage-12)] text-[var(--color-garden-rosemary)]",
  in_progress:
    "bg-[var(--color-garden-larkspur-12)] text-[var(--color-desk-ink)]",
  planned: "bg-muted text-muted-foreground",
};
