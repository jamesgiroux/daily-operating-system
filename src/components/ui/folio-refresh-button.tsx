/**
 * FolioRefreshButton — canonical refresh/run button primitive.
 *
 * Mono 11px, uppercase, bordered, tertiary color. Matches the editorial
 * design language from the account detail page. Used across all pages
 * and hero components for consistency.
 */

interface FolioRefreshButtonProps {
  onClick: () => void;
  loading: boolean;
  /** Label when idle. Defaults to "Refresh". */
  label?: string;
  /** Label when loading. Defaults to "Refreshing…". */
  loadingLabel?: string;
  /** Progress text appended when loading (e.g. "12s" or "45%"). */
  loadingProgress?: string;
  title?: string;
}

export function FolioRefreshButton({
  onClick,
  loading,
  label = "Refresh",
  loadingLabel,
  loadingProgress,
  title,
}: FolioRefreshButtonProps) {
  const displayLabel = loading
    ? (loadingLabel ?? "Refreshing\u2026") + (loadingProgress ? ` ${loadingProgress}` : "")
    : label;

  return (
    <button
      onClick={onClick}
      disabled={loading}
      title={title ?? displayLabel}
      style={{
        fontFamily: "var(--font-mono)",
        fontSize: 11,
        fontWeight: 600,
        letterSpacing: "0.06em",
        textTransform: "uppercase",
        color: "var(--color-text-tertiary)",
        background: "none",
        border: "1px solid var(--color-rule-heavy)",
        borderRadius: 4,
        padding: "2px 10px",
        cursor: loading ? "default" : "pointer",
        opacity: loading ? 0.6 : 1,
        transition: "color 150ms, border-color 150ms, opacity 150ms",
      }}
      onMouseEnter={(e) => {
        if (!loading) {
          e.currentTarget.style.color = "var(--color-text-secondary)";
          e.currentTarget.style.borderColor = "var(--color-text-tertiary)";
        }
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.color = "var(--color-text-tertiary)";
        e.currentTarget.style.borderColor = "var(--color-rule-heavy)";
      }}
    >
      {displayLabel}
    </button>
  );
}
