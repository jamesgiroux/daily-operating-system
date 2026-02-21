import { RefreshCw, Loader2 } from "lucide-react";

interface FolioRefreshButtonProps {
  onClick: () => void;
  loading: boolean;
  /** Label when idle. Defaults to "Refresh". */
  label?: string;
  /** Label when loading. Defaults to "Running…". */
  loadingLabel?: string;
  title?: string;
}

/**
 * Standard folio bar refresh / run button.
 * Used in DailyBriefing, WeekPage, EmailsPage.
 *
 * Shows RefreshCw when idle and Loader2 (spinning) when loading.
 * Tailwind-only, no inline styles except the mono font override on the label.
 */
export function FolioRefreshButton({
  onClick,
  loading,
  label = "Refresh",
  loadingLabel,
  title,
}: FolioRefreshButtonProps) {
  const displayLabel = loading ? (loadingLabel ?? "Running\u2026") : label;

  return (
    <button
      onClick={onClick}
      disabled={loading}
      className="flex items-center gap-1.5 rounded-sm px-2 py-1 text-xs text-muted-foreground transition-colors hover:text-foreground disabled:cursor-not-allowed disabled:opacity-50"
      title={title ?? displayLabel}
    >
      {loading ? (
        <Loader2 className="h-3 w-3 animate-spin" />
      ) : (
        <RefreshCw className="h-3 w-3" />
      )}
      <span
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 11,
          letterSpacing: "0.04em",
        }}
      >
        {displayLabel}
      </span>
    </button>
  );
}
