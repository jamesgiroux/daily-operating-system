/**
 * TheAskSlide — Decisions + resources + escalation.
 * Slide 6: what we need from leadership to execute the plan.
 */
import type { RiskTheAsk } from "@/types";

interface TheAskSlideProps {
  data: RiskTheAsk;
}

const urgencyColors: Record<string, string> = {
  immediate: "var(--color-spice-chili)",
  this_week: "var(--color-spice-terracotta)",
  this_month: "var(--color-spice-turmeric)",
};

export function TheAskSlide({ data }: TheAskSlideProps) {
  return (
    <section
      id="the-ask"
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
        The Ask
      </div>

      {/* Requests — numbered with urgency dot */}
      {data.requests && data.requests.length > 0 && (
        <div style={{ marginBottom: 28, maxWidth: 540 }}>
          {data.requests.slice(0, 3).map((req, i) => (
            <div
              key={i}
              style={{
                display: "flex",
                alignItems: "baseline",
                gap: 12,
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
              {req.urgency && (
                <span
                  style={{
                    width: 7,
                    height: 7,
                    borderRadius: "50%",
                    background:
                      urgencyColors[req.urgency] ??
                      "var(--color-text-tertiary)",
                    flexShrink: 0,
                    marginTop: 4,
                  }}
                  title={req.urgency.replace("_", " ")}
                />
              )}
              <div style={{ flex: 1 }}>
                <span
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 14,
                    color: "var(--color-text-primary)",
                  }}
                >
                  {req.request}
                </span>
                {req.from && (
                  <div
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 11,
                      color: "var(--color-text-tertiary)",
                      marginTop: 2,
                    }}
                  >
                    {req.from}
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Decisions — checkbox visual, max 2 */}
      {data.decisions && data.decisions.length > 0 && (
        <div style={{ marginBottom: 28, maxWidth: 540 }}>
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
            Decisions Needed
          </div>
          {data.decisions.slice(0, 2).map((d, i) => (
            <div
              key={i}
              style={{
                display: "flex",
                gap: 10,
                alignItems: "center",
                marginBottom: 8,
              }}
            >
              <span
                style={{
                  width: 14,
                  height: 14,
                  borderRadius: 2,
                  border: "1.5px solid var(--color-spice-turmeric)",
                  flexShrink: 0,
                }}
              />
              <span
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 14,
                  color: "var(--color-text-primary)",
                }}
              >
                {d}
              </span>
            </div>
          ))}
        </div>
      )}

      {/* Escalation — single line */}
      {data.escalation && (
        <p
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 13,
            lineHeight: 1.5,
            color: "var(--color-text-secondary)",
            margin: 0,
            maxWidth: 540,
            borderLeft: "2px solid var(--color-spice-terracotta)",
            paddingLeft: 14,
          }}
        >
          {data.escalation}
        </p>
      )}
    </section>
  );
}
