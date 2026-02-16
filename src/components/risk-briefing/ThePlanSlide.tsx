/**
 * ThePlanSlide — Strategy + actions + risk caveats.
 * Slide 5: recovery strategy with timeline and assumption caveats.
 */
import { EditableText } from "@/components/ui/EditableText";
import type { RiskThePlan } from "@/types";

interface ThePlanSlideProps {
  data: RiskThePlan;
  onUpdate?: (data: RiskThePlan) => void;
}

export function ThePlanSlide({ data, onUpdate }: ThePlanSlideProps) {
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
          fontSize: 12,
          fontWeight: 600,
          textTransform: "uppercase",
          letterSpacing: "0.12em",
          color: "var(--color-text-secondary)",
          marginBottom: 24,
        }}
      >
        The Plan
      </div>

      {/* Strategy headline */}
      <EditableText
        as="h2"
        value={data.strategy}
        onChange={(v) => onUpdate?.({ ...data, strategy: v })}
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 32,
          fontWeight: 400,
          lineHeight: 1.25,
          color: "var(--color-text-primary)",
          maxWidth: 800,
          margin: "0 0 28px",
        }}
      />

      {/* Timeline badge */}
      {data.timeline && (
        <EditableText
          as="div"
          value={data.timeline}
          onChange={(v) => onUpdate?.({ ...data, timeline: v })}
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 14,
            fontWeight: 600,
            color: "var(--color-text-primary)",
            letterSpacing: "0.04em",
            marginBottom: 28,
          }}
        />
      )}

      {/* Actions — numbered steps */}
      {data.actions && data.actions.length > 0 && (
        <div style={{ marginBottom: 32, maxWidth: 800 }}>
          {data.actions.slice(0, 3).map((action, i) => (
            <div
              key={i}
              style={{
                display: "flex",
                gap: 16,
                alignItems: "baseline",
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
              <div style={{ flex: 1 }}>
                <EditableText
                  value={action.step}
                  onChange={(v) => {
                    const updated = [...(data.actions ?? [])];
                    updated[i] = { ...updated[i], step: v };
                    onUpdate?.({ ...data, actions: updated });
                  }}
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 17,
                    color: "var(--color-text-primary)",
                  }}
                />
                {(action.owner || action.timeline) && (
                  <div
                    style={{
                      display: "flex",
                      gap: 8,
                      marginTop: 4,
                    }}
                  >
                    {action.owner && (
                      <EditableText
                        value={action.owner}
                        onChange={(v) => {
                          const updated = [...(data.actions ?? [])];
                          updated[i] = { ...updated[i], owner: v };
                          onUpdate?.({ ...data, actions: updated });
                        }}
                        style={{
                          fontFamily: "var(--font-mono)",
                          fontSize: 13,
                          color: "var(--color-text-primary)",
                          opacity: 0.7,
                        }}
                      />
                    )}
                    {action.owner && action.timeline && (
                      <span style={{ fontFamily: "var(--font-mono)", fontSize: 13, color: "var(--color-text-primary)", opacity: 0.7 }}>·</span>
                    )}
                    {action.timeline && (
                      <EditableText
                        value={action.timeline}
                        onChange={(v) => {
                          const updated = [...(data.actions ?? [])];
                          updated[i] = { ...updated[i], timeline: v };
                          onUpdate?.({ ...data, actions: updated });
                        }}
                        style={{
                          fontFamily: "var(--font-mono)",
                          fontSize: 13,
                          color: "var(--color-text-primary)",
                          opacity: 0.7,
                        }}
                      />
                    )}
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Assumptions as caveats */}
      {data.assumptions && data.assumptions.length > 0 && (
        <div style={{ maxWidth: 800 }}>
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
            Assumptions
          </div>
          {data.assumptions.slice(0, 2).map((assumption, i) => (
            <EditableText
              key={i}
              as="p"
              value={assumption}
              onChange={(v) => {
                const updated = [...(data.assumptions ?? [])];
                updated[i] = v;
                onUpdate?.({ ...data, assumptions: updated });
              }}
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 15,
                lineHeight: 1.5,
                color: "var(--color-text-primary)",
                margin: "0 0 8px",
                borderLeft: "3px solid var(--color-spice-terracotta)",
                paddingLeft: 16,
              }}
            />
          ))}
        </div>
      )}
    </section>
  );
}
