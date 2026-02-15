/**
 * FinisMarker — three spaced asterisks + enrichment timestamp.
 * Mockup: Montserrat 18px, turmeric color, 0.4em letter-spacing.
 * Marks the end of an editorial briefing. "When you've read it, you're briefed."
 */
interface FinisMarkerProps {
  enrichedAt?: string;
}

export function FinisMarker({ enrichedAt }: FinisMarkerProps) {
  return (
    <div style={{ textAlign: "center", padding: "72px 0 24px" }}>
      <div
        style={{
          fontFamily: "var(--font-mark)",
          fontSize: 18,
          letterSpacing: "0.4em",
          color: "var(--color-spice-turmeric)",
        }}
      >
        * * *
      </div>
      {enrichedAt && (
        <div
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 10,
            color: "var(--color-text-tertiary)",
            letterSpacing: "0.06em",
            marginTop: 16,
          }}
        >
          Last enriched: {enrichedAt}
        </div>
      )}
    </div>
  );
}
