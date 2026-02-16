/**
 * DashboardError — Editorial error state for the daily briefing.
 * Renders inside MagazinePageLayout's page container.
 */

import { Button } from "@/components/ui/button";

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
        <Button
          onClick={onRetry}
          variant="outline"
          style={{ fontFamily: "var(--font-sans)", fontSize: 13 }}
        >
          Try again
        </Button>
      </div>
    </div>
  );
}
