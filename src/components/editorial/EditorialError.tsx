/**
 * EditorialError — terracotta error message with retry button for editorial pages.
 * Extracted from ActionsPage for reuse across all editorial surfaces.
 */
export function EditorialError({ message, onRetry }: { message: string; onRetry?: () => void }) {
  return (
    <div style={{ maxWidth: 900, marginLeft: "auto", marginRight: "auto", paddingTop: 80, textAlign: "center" }}>
      <p style={{ fontFamily: "var(--font-sans)", fontSize: 15, color: "var(--color-spice-terracotta)" }}>
        {message}
      </p>
      {onRetry && (
        <button
          onClick={onRetry}
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 12,
            color: "var(--color-text-tertiary)",
            background: "none",
            border: "1px solid var(--color-rule-heavy)",
            borderRadius: 4,
            padding: "4px 12px",
            cursor: "pointer",
            marginTop: 12,
          }}
        >
          Retry
        </button>
      )}
    </div>
  );
}
