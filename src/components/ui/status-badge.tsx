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
    "bg-[rgba(126,170,123,0.12)] text-[#4a6741] border-[rgba(126,170,123,0.3)]",
  yellow:
    "bg-[rgba(222,184,65,0.12)] text-[#3d2e27] border-[rgba(222,184,65,0.3)]",
  red:
    "bg-[rgba(196,101,74,0.12)] text-[#9b3a2a] border-[rgba(196,101,74,0.3)]",
};

export const projectStatusStyles: Record<string, string> = {
  active:
    "bg-[rgba(126,170,123,0.12)] text-[#4a6741] border-[rgba(126,170,123,0.3)]",
  on_hold:
    "bg-[rgba(222,184,65,0.12)] text-[#3d2e27] border-[rgba(222,184,65,0.3)]",
  completed:
    "bg-[rgba(143,163,196,0.12)] text-[#2a2b3d] border-[rgba(143,163,196,0.3)]",
  archived: "bg-muted text-muted-foreground border-muted",
};

export const progressStyles: Record<string, string> = {
  completed:
    "bg-[rgba(126,170,123,0.12)] text-[#4a6741]",
  in_progress:
    "bg-[rgba(143,163,196,0.12)] text-[#2a2b3d]",
  planned: "bg-muted text-muted-foreground",
};
