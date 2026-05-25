/**
 * FolioRefreshButton — canonical refresh/run button primitive.
 *
 * Mono 11px, uppercase, bordered, tertiary color. Matches the editorial
 * design language from the account detail page. Used across all pages
 * and hero components for consistency.
 */

import styles from "./folio-refresh-button.module.css";

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
      aria-busy={loading ? "true" : "false"}
      title={title ?? displayLabel}
      className={styles.button}
    >
      {displayLabel}
    </button>
  );
}
