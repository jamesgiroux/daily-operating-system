/**
 * TheAskSlide — Decisions + resources + escalation.
 * Slide 6: what we need from leadership to execute the plan.
 */
import { EditableText } from "@/components/ui/EditableText";
import type { RiskTheAsk } from "@/types";

interface TheAskSlideProps {
  data: RiskTheAsk;
  onUpdate?: (data: RiskTheAsk) => void;
}

const urgencyColors: Record<string, string> = {
  immediate: "var(--color-spice-chili)",
  this_week: "var(--color-spice-terracotta)",
  this_month: "var(--color-spice-turmeric)",
};

export function TheAskSlide({ data, onUpdate }: TheAskSlideProps) {
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
          fontSize: 12,
          fontWeight: 600,
          textTransform: "uppercase",
          letterSpacing: "0.12em",
          color: "var(--color-text-secondary)",
          marginBottom: 24,
        }}
      >
        The Ask
      </div>

      {/* Requests — numbered with urgency dot */}
      {data.requests && data.requests.length > 0 && (
        <div style={{ marginBottom: 32, maxWidth: 800 }}>
          {data.requests.slice(0, 3).map((req, i) => (
            <div
              key={i}
              style={{
                display: "flex",
                alignItems: "baseline",
                gap: 16,
                padding: "12px 0",
                borderBottom: "1px solid var(--color-rule-light)",
              }}
            >
              <span
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 20,
                  fontWeight: 600,
                  color: "var(--color-spice-terracotta)",
                  minWidth: 24,
                  flexShrink: 0,
                }}
              >
                {i + 1}
              </span>
              {req.urgency && (
                <span
                  style={{
                    width: 9,
                    height: 9,
                    borderRadius: "50%",
                    background:
                      urgencyColors[req.urgency] ??
                      "var(--color-text-secondary)",
                    flexShrink: 0,
                    marginTop: 4,
                  }}
                  title={req.urgency.replace("_", " ")}
                />
              )}
              <div style={{ flex: 1 }}>
                <EditableText
                  value={req.request}
                  onChange={(v) => {
                    const updated = [...(data.requests ?? [])];
                    updated[i] = { ...updated[i], request: v };
                    onUpdate?.({ ...data, requests: updated });
                  }}
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 17,
                    color: "var(--color-text-primary)",
                  }}
                />
                {req.from && (
                  <EditableText
                    as="div"
                    value={req.from}
                    onChange={(v) => {
                      const updated = [...(data.requests ?? [])];
                      updated[i] = { ...updated[i], from: v };
                      onUpdate?.({ ...data, requests: updated });
                    }}
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 13,
                      color: "var(--color-text-primary)",
                      opacity: 0.7,
                      marginTop: 4,
                    }}
                  />
                )}
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Decisions — checkbox visual, max 2 */}
      {data.decisions && data.decisions.length > 0 && (
        <div style={{ marginBottom: 32, maxWidth: 800 }}>
          <div
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              fontWeight: 600,
              textTransform: "uppercase",
              letterSpacing: "0.1em",
              color: "var(--color-text-secondary)",
              marginBottom: 12,
            }}
          >
            Decisions Needed
          </div>
          {data.decisions.slice(0, 2).map((d, i) => (
            <div
              key={i}
              style={{
                display: "flex",
                gap: 12,
                alignItems: "center",
                marginBottom: 10,
              }}
            >
              <span
                style={{
                  width: 16,
                  height: 16,
                  borderRadius: 2,
                  border: "2px solid var(--color-spice-terracotta)",
                  flexShrink: 0,
                }}
              />
              <EditableText
                value={d}
                onChange={(v) => {
                  const updated = [...(data.decisions ?? [])];
                  updated[i] = v;
                  onUpdate?.({ ...data, decisions: updated });
                }}
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 17,
                  color: "var(--color-text-primary)",
                }}
              />
            </div>
          ))}
        </div>
      )}

      {/* Escalation — single line */}
      {data.escalation && (
        <EditableText
          as="p"
          value={data.escalation}
          onChange={(v) => onUpdate?.({ ...data, escalation: v })}
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 15,
            lineHeight: 1.5,
            color: "var(--color-text-primary)",
            margin: 0,
            maxWidth: 800,
            borderLeft: "3px solid var(--color-spice-terracotta)",
            paddingLeft: 16,
          }}
        />
      )}
    </section>
  );
}
