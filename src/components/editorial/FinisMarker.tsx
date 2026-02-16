/**
 * FinisMarker — three spaced asterisks + enrichment timestamp.
 * Mockup: Montserrat 18px, turmeric color, 0.4em letter-spacing.
 * Marks the end of an editorial briefing. "When you've read it, you're briefed."
 */
import { BrandMark } from '../ui/BrandMark';

interface FinisMarkerProps {
  enrichedAt?: string;
}

export function FinisMarker({ enrichedAt }: FinisMarkerProps) {
  return (
    <div style={{ textAlign: "center", padding: "72px 0 24px" }}>
      <div
        style={{
          display: "flex",
          justifyContent: "center",
          gap: "0.4em",
          color: "var(--color-spice-turmeric)",
        }}
      >
        <BrandMark size={18} />
        <BrandMark size={18} />
        <BrandMark size={18} />
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
