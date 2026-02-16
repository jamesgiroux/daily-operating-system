/**
 * StakesSlide — Who matters + what's the money.
 * Slide 4: financial headline + stakeholder cards + worst case.
 * Merges TheRoom + Commercial from v2 into one stakes view.
 */
import type { RiskStakes } from "@/types";

interface StakesSlideProps {
  data: RiskStakes;
}

const alignmentColors: Record<string, string> = {
  champion: "var(--color-garden-sage)",
  neutral: "var(--color-spice-turmeric)",
  detractor: "var(--color-spice-chili)",
  unknown: "var(--color-text-tertiary)",
};

export function StakesSlide({ data }: StakesSlideProps) {
  return (
    <section
      id="stakes"
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
        The Stakes
      </div>

      {/* Financial headline */}
      {data.financialHeadline && (
        <h2
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 28,
            fontWeight: 400,
            lineHeight: 1.25,
            color: "var(--color-text-primary)",
            maxWidth: 540,
            margin: "0 0 32px",
          }}
        >
          {data.financialHeadline}
        </h2>
      )}

      {/* Stakeholder cards — max 4 */}
      {data.stakeholders && data.stakeholders.length > 0 && (
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fill, minmax(200px, 1fr))",
            gap: 16,
            marginBottom: 32,
            maxWidth: 600,
          }}
        >
          {data.stakeholders.slice(0, 4).map((s, i) => (
            <div
              key={i}
              style={{
                padding: "14px 16px",
                border: "1px solid var(--color-rule-light)",
                borderRadius: 6,
              }}
            >
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 8,
                  marginBottom: 6,
                }}
              >
                {/* Alignment dot */}
                <span
                  style={{
                    width: 7,
                    height: 7,
                    borderRadius: "50%",
                    background:
                      alignmentColors[s.alignment?.toLowerCase() ?? ""] ??
                      "var(--color-text-tertiary)",
                    flexShrink: 0,
                  }}
                />
                <span
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 14,
                    fontWeight: 600,
                    color: "var(--color-text-primary)",
                  }}
                >
                  {s.name}
                </span>
              </div>
              {s.role && (
                <div
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 12,
                    color: "var(--color-text-secondary)",
                    marginBottom: 4,
                  }}
                >
                  {s.role}
                </div>
              )}
              {/* Badges row */}
              <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                {s.engagement && (
                  <span
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 9,
                      fontWeight: 600,
                      textTransform: "uppercase",
                      letterSpacing: "0.06em",
                      color: "var(--color-text-tertiary)",
                      border: "1px solid var(--color-rule-light)",
                      borderRadius: 3,
                      padding: "2px 6px",
                    }}
                  >
                    {s.engagement}
                  </span>
                )}
                {s.decisionWeight && (
                  <span
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 9,
                      fontWeight: 600,
                      textTransform: "uppercase",
                      letterSpacing: "0.06em",
                      color: "var(--color-text-tertiary)",
                      border: "1px solid var(--color-rule-light)",
                      borderRadius: 3,
                      padding: "2px 6px",
                    }}
                  >
                    {s.decisionWeight.replace("_", " ")}
                  </span>
                )}
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Decision maker + worst case */}
      <div style={{ display: "flex", flexDirection: "column", gap: 12, maxWidth: 540 }}>
        {data.decisionMaker && (
          <div style={{ display: "flex", gap: 8, alignItems: "baseline" }}>
            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                fontWeight: 600,
                textTransform: "uppercase",
                letterSpacing: "0.08em",
                color: "var(--color-text-tertiary)",
              }}
            >
              Decision Maker
            </span>
            <span
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 14,
                color: "var(--color-text-primary)",
              }}
            >
              {data.decisionMaker}
            </span>
          </div>
        )}
        {data.worstCase && (
          <p
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 14,
              lineHeight: 1.5,
              color: "var(--color-spice-terracotta)",
              margin: 0,
              borderLeft: "2px solid var(--color-spice-terracotta)",
              paddingLeft: 14,
            }}
          >
            {data.worstCase}
          </p>
        )}
      </div>
    </section>
  );
}
