/**
 * EmptyState.tsx — Unified empty state component
 *
 * Replaces EditorialEmpty and EntityListEmpty with a single editorial
 * component. DashboardEmpty stays bespoke (it's the app's front door).
 */

import { Button } from "@/components/ui/button";

interface EmptyStateAction {
  label: string;
  onClick: () => void;
}

interface EmptyStateProps {
  headline: string;
  explanation: string;
  benefit?: string;
  action?: EmptyStateAction;
  secondaryAction?: EmptyStateAction;
  children?: React.ReactNode;
}

export function EmptyState({
  headline,
  explanation,
  benefit,
  action,
  secondaryAction,
  children,
}: EmptyStateProps) {
  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        textAlign: "center",
        maxWidth: 480,
        margin: "0 auto",
        paddingTop: 64,
        paddingBottom: 64,
      }}
    >
      <h2
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 24,
          fontWeight: 400,
          fontStyle: "italic",
          lineHeight: 1.3,
          color: "var(--color-text-primary)",
          margin: 0,
        }}
      >
        {headline}
      </h2>

      <p
        style={{
          fontFamily: "var(--font-sans)",
          fontSize: 15,
          lineHeight: 1.6,
          color: "var(--color-text-tertiary)",
          margin: "12px 0 0",
        }}
      >
        {explanation}
      </p>

      {benefit && (
        <p
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 14,
            fontStyle: "italic",
            lineHeight: 1.5,
            color: "var(--color-text-tertiary)",
            margin: "8px 0 0",
          }}
        >
          {benefit}
        </p>
      )}

      {(action || secondaryAction || children) && (
        <div style={{ marginTop: 24, display: "flex", flexDirection: "column", alignItems: "center", gap: 12 }}>
          {action && (
            <Button onClick={action.onClick}>
              {action.label}
            </Button>
          )}
          {secondaryAction && (
            <button
              onClick={secondaryAction.onClick}
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 11,
                letterSpacing: "0.04em",
                color: "var(--color-text-tertiary)",
                background: "none",
                border: "none",
                cursor: "pointer",
              }}
            >
              {secondaryAction.label}
            </button>
          )}
          {children}
        </div>
      )}
    </div>
  );
}
