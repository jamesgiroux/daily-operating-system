/**
 * ThePlanSlide — Strategy + actions + risk caveats.
 * Slide 5: recovery strategy with timeline and assumption caveats.
 */
import type { RiskThePlan } from "@/types";

interface ThePlanSlideProps {
  data: RiskThePlan;
}

export function ThePlanSlide({ data }: ThePlanSlideProps) {
  return (
    <section
      id="the-plan"
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
        The Plan
      </div>

      {/* Strategy headline */}
      <h2
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 28,
          fontWeight: 400,
          lineHeight: 1.25,
          color: "var(--color-text-primary)",
          maxWidth: 540,
          margin: "0 0 28px",
        }}
      >
        {data.strategy}
      </h2>

      {/* Timeline badge */}
      {data.timeline && (
        <div
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 12,
            color: "var(--color-spice-turmeric)",
            letterSpacing: "0.04em",
            marginBottom: 24,
          }}
        >
          {data.timeline}
        </div>
      )}

      {/* Actions — numbered steps */}
      {data.actions && data.actions.length > 0 && (
        <div style={{ marginBottom: 28, maxWidth: 540 }}>
          {data.actions.slice(0, 3).map((action, i) => (
            <div
              key={i}
              style={{
                display: "flex",
                gap: 12,
                alignItems: "baseline",
                padding: "10px 0",
                borderBottom: "1px solid var(--color-rule-light)",
              }}
            >
              <span
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 16,
                  fontWeight: 600,
                  color: "var(--color-spice-terracotta)",
                  minWidth: 20,
                  flexShrink: 0,
                }}
              >
                {i + 1}
              </span>
              <div style={{ flex: 1 }}>
                <span
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 14,
                    color: "var(--color-text-primary)",
                  }}
                >
                  {action.step}
                </span>
                {(action.owner || action.timeline) && (
                  <div
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 11,
                      color: "var(--color-text-tertiary)",
                      marginTop: 2,
                    }}
                  >
                    {[action.owner, action.timeline].filter(Boolean).join(" · ")}
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Assumptions as caveats — folded from red team */}
      {data.assumptions && data.assumptions.length > 0 && (
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
            Assumptions
          </div>
          {data.assumptions.slice(0, 2).map((assumption, i) => (
            <p
              key={i}
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 13,
                lineHeight: 1.5,
                color: "var(--color-text-secondary)",
                margin: "0 0 6px",
                borderLeft: "2px solid var(--color-spice-turmeric)",
                paddingLeft: 12,
              }}
            >
              {assumption}
            </p>
          ))}
        </div>
      )}
    </section>
  );
}
