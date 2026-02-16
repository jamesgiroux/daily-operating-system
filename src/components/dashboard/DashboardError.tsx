/**
 * DashboardError — Editorial error state for the daily briefing.
 * Renders inside MagazinePageLayout's page container.
 */

interface DashboardErrorProps {
  message: string;
  onRetry: () => void;
}

export function DashboardError({ message, onRetry }: DashboardErrorProps) {
  return (
    <div style={{ maxWidth: 900, marginLeft: "auto", marginRight: "auto" }}>
      <div style={{ paddingTop: 120, paddingBottom: 80, textAlign: "center" }}>
        <h1
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 28,
            fontWeight: 400,
            color: "var(--color-text-primary)",
            margin: "0 0 12px 0",
          }}
        >
          Something went wrong
        </h1>
        <p
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 15,
            color: "var(--color-text-secondary)",
            maxWidth: 400,
            marginLeft: "auto",
            marginRight: "auto",
            lineHeight: 1.6,
            marginBottom: 28,
          }}
        >
          {message}
        </p>
        <button
          onClick={onRetry}
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 12,
            fontWeight: 500,
            letterSpacing: "0.04em",
            textTransform: "uppercase",
            padding: "8px 24px",
            borderRadius: 4,
            border: "1px solid var(--color-rule-heavy)",
            background: "none",
            color: "var(--color-text-primary)",
            cursor: "pointer",
          }}
        >
          Try again
        </button>
      </div>
    </div>
  );
}
