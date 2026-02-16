/**
 * BottomLineSlide — The whole story in one breath.
 * Slide 2: headline + risk badge + renewal window.
 */
import type { RiskBottomLine } from "@/types";

interface BottomLineSlideProps {
  data: RiskBottomLine;
}

const riskColors: Record<string, string> = {
  high: "var(--color-spice-chili)",
  medium: "var(--color-spice-terracotta)",
  low: "var(--color-garden-sage)",
};

export function BottomLineSlide({ data }: BottomLineSlideProps) {
  const riskKey = data.riskLevel?.toLowerCase() ?? "";

  return (
    <section
      id="bottom-line"
      style={{
        scrollMarginTop: 60,
        minHeight: "100vh",
        display: "flex",
        flexDirection: "column",
        justifyContent: "center",
        padding: "120px 120px 80px",
        scrollSnapAlign: "start",
      }}
    >
      {/* Overline */}
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 10,
          fontWeight: 600,
          textTransform: "uppercase",
          letterSpacing: "0.12em",
          color: "var(--color-spice-terracotta)",
          marginBottom: 20,
        }}
      >
        Bottom Line
      </div>

      {/* Headline — the whole story */}
      <h2
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 32,
          fontWeight: 400,
          lineHeight: 1.3,
          letterSpacing: "-0.01em",
          color: "var(--color-text-primary)",
          maxWidth: 600,
          margin: "0 0 28px",
        }}
      >
        {data.headline}
      </h2>

      {/* Risk badge + renewal window */}
      <div style={{ display: "flex", gap: 24, alignItems: "center" }}>
        {data.riskLevel && (
          <div style={{ display: "inline-flex", alignItems: "center", gap: 8 }}>
            <span
              style={{
                width: 8,
                height: 8,
                borderRadius: "50%",
                background: riskColors[riskKey] ?? "var(--color-text-tertiary)",
              }}
            />
            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 11,
                fontWeight: 600,
                textTransform: "uppercase",
                letterSpacing: "0.06em",
                color: riskColors[riskKey] ?? "var(--color-text-tertiary)",
              }}
            >
              {data.riskLevel} risk
            </span>
          </div>
        )}
        {data.renewalWindow && (
          <span
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 12,
              color: "var(--color-text-tertiary)",
              letterSpacing: "0.04em",
            }}
          >
            {data.renewalWindow}
          </span>
        )}
      </div>
    </section>
  );
}
