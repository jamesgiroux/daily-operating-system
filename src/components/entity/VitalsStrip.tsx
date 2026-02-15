/**
 * VitalsStrip — inline horizontal strip of key metrics with dot separators.
 * Generalized: accepts a pre-built `vitals` array instead of a specific entity detail.
 * Callers (account, project, person pages) assemble their own vitals array.
 */
import type { VitalDisplay } from "@/lib/entity-types";

interface VitalsStripProps {
  vitals: VitalDisplay[];
}

const highlightColor: Record<string, string> = {
  turmeric: "var(--color-spice-turmeric)",
  saffron: "var(--color-spice-saffron)",
  olive: "var(--color-garden-olive)",
  larkspur: "var(--color-garden-larkspur)",
};

export function VitalsStrip({ vitals }: VitalsStripProps) {
  if (vitals.length === 0) return null;

  return (
    <div
      style={{
        marginTop: 48,
        borderTop: "1px solid var(--color-rule-heavy)",
        borderBottom: "1px solid var(--color-rule-heavy)",
        padding: "14px 0",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 24, flexWrap: "wrap" }}>
        {vitals.map((v, i) => (
          <span key={i} style={{ display: "flex", alignItems: "center", gap: 24 }}>
            {i > 0 && (
              <span
                style={{
                  width: 3,
                  height: 3,
                  borderRadius: "50%",
                  background: "var(--color-text-tertiary)",
                  flexShrink: 0,
                }}
              />
            )}
            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 12,
                fontWeight: 500,
                textTransform: "uppercase",
                letterSpacing: "0.06em",
                color: v.highlight ? highlightColor[v.highlight] : "var(--color-text-secondary)",
                whiteSpace: "nowrap",
              }}
            >
              {v.text}
            </span>
          </span>
        ))}
      </div>
    </div>
  );
}
