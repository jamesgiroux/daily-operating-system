/**
 * WhatHappenedSlide — Situation → Complication as one narrative arc.
 * Slide 3: 3-sentence narrative + health dots + key losses.
 */
import type { RiskWhatHappened } from "@/types";

interface WhatHappenedSlideProps {
  data: RiskWhatHappened;
}

const statusColors: Record<string, string> = {
  green: "var(--color-garden-sage)",
  yellow: "var(--color-spice-turmeric)",
  red: "var(--color-spice-chili)",
  amber: "var(--color-spice-terracotta)",
};

export function WhatHappenedSlide({ data }: WhatHappenedSlideProps) {
  return (
    <section
      id="what-happened"
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
          color: "var(--color-text-tertiary)",
          marginBottom: 20,
        }}
      >
        What Happened
      </div>

      {/* 3-sentence narrative */}
      <p
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 20,
          lineHeight: 1.6,
          color: "var(--color-text-primary)",
          maxWidth: 600,
          margin: "0 0 32px",
        }}
      >
        {data.narrative}
      </p>

      {/* Health arc dots */}
      {data.healthArc && data.healthArc.length > 0 && (
        <div style={{ display: "flex", gap: 16, marginBottom: 32 }}>
          {data.healthArc.map((snap, i) => (
            <div
              key={i}
              style={{ display: "flex", alignItems: "center", gap: 8 }}
            >
              <span
                style={{
                  width: 10,
                  height: 10,
                  borderRadius: "50%",
                  background:
                    statusColors[snap.status.toLowerCase()] ??
                    "var(--color-text-tertiary)",
                }}
              />
              <span
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 11,
                  color: "var(--color-text-tertiary)",
                }}
              >
                {snap.period}
              </span>
              {snap.detail && (
                <span
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 12,
                    color: "var(--color-text-secondary)",
                  }}
                >
                  {snap.detail}
                </span>
              )}
            </div>
          ))}
        </div>
      )}

      {/* Key losses */}
      {data.keyLosses && data.keyLosses.length > 0 && (
        <div style={{ maxWidth: 540 }}>
          <div
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 10,
              fontWeight: 500,
              textTransform: "uppercase",
              letterSpacing: "0.1em",
              color: "var(--color-text-tertiary)",
              marginBottom: 10,
            }}
          >
            Key Losses
          </div>
          {data.keyLosses.slice(0, 3).map((loss, i) => (
            <div
              key={i}
              style={{
                display: "flex",
                gap: 10,
                alignItems: "baseline",
                padding: "6px 0",
              }}
            >
              <span
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 12,
                  color: "var(--color-spice-terracotta)",
                  flexShrink: 0,
                }}
              >
                {i + 1}
              </span>
              <span
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 14,
                  color: "var(--color-text-primary)",
                }}
              >
                {loss}
              </span>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
