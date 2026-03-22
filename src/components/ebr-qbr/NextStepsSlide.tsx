/**
 * NextStepsSlide — Slide 7: Next Steps.
 * Numbered list of actions with owner and timeline.
 * Modeled visually after ThePlanSlide.tsx.
 */
import { useState } from "react";
import { EditableText } from "@/components/ui/EditableText";
import type { EbrQbrContent, EbrQbrAction } from "@/types/reports";

interface NextStepsSlideProps {
  content: EbrQbrContent;
  onUpdate: (c: EbrQbrContent) => void;
}

export function NextStepsSlide({ content, onUpdate }: NextStepsSlideProps) {
  const [hoveredIndex, setHoveredIndex] = useState<number | null>(null);

  const steps = content.nextSteps;

  function updateStep(i: number, updated: EbrQbrAction) {
    const updatedSteps = [...steps];
    updatedSteps[i] = updated;
    onUpdate({ ...content, nextSteps: updatedSteps });
  }

  function removeStep(i: number) {
    onUpdate({ ...content, nextSteps: steps.filter((_, j) => j !== i) });
  }

  function addStep() {
    onUpdate({
      ...content,
      nextSteps: [...steps, { action: "", owner: "", timeline: "" }],
    });
  }

  return (
    <section
      id="next-steps"
      className="report-surface-slide"
      style={{ scrollMarginTop: 60 }}
    >
      {/* Overline */}
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 12,
          fontWeight: 600,
          textTransform: "uppercase",
          letterSpacing: "0.12em",
          color: "var(--color-garden-larkspur)",
          marginBottom: 36,
        }}
      >
        Next Steps
      </div>

      {/* Numbered list */}
      <div style={{ maxWidth: 800 }}>
        {steps.map((step, i) => (
          <div
            key={i}
            onMouseEnter={() => setHoveredIndex(i)}
            onMouseLeave={() => setHoveredIndex(null)}
            style={{
              display: "flex",
              gap: 16,
              alignItems: "baseline",
              padding: "12px 0",
              borderBottom: "1px solid var(--color-rule-light)",
            }}
          >
            {/* Number */}
            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 20,
                fontWeight: 600,
                color: "var(--color-garden-larkspur)",
                minWidth: 24,
                flexShrink: 0,
              }}
            >
              {i + 1}
            </span>

            {/* Action + owner/timeline */}
            <div style={{ flex: 1 }}>
              <EditableText
                value={step.action}
                onChange={(v) => updateStep(i, { ...step, action: v })}
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 17,
                  color: "var(--color-text-primary)",
                  display: "block",
                  marginBottom: 4,
                }}
              />

              <div style={{ display: "flex", gap: 8, alignItems: "center", marginTop: 4 }}>
                <EditableText
                  value={step.owner}
                  onChange={(v) => updateStep(i, { ...step, owner: v })}
                  multiline={false}
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 13,
                    color: "var(--color-text-primary)",
                    opacity: 0.7,
                  }}
                />
                {step.owner && step.timeline && (
                  <span
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 13,
                      color: "var(--color-text-primary)",
                      opacity: 0.7,
                    }}
                  >
                    ·
                  </span>
                )}
                <EditableText
                  value={step.timeline}
                  onChange={(v) => updateStep(i, { ...step, timeline: v })}
                  multiline={false}
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 13,
                    color: "var(--color-text-primary)",
                    opacity: 0.7,
                  }}
                />
              </div>
            </div>

            {/* Dismiss */}
            {steps.length > 1 && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  removeStep(i);
                }}
                style={{
                  opacity: hoveredIndex === i ? 0.6 : 0,
                  transition: "opacity 0.15s",
                  background: "none",
                  border: "none",
                  cursor: "pointer",
                  padding: "4px 6px",
                  fontSize: 14,
                  color: "var(--color-text-tertiary)",
                  flexShrink: 0,
                }}
                aria-label="Remove"
              >
                ✕
              </button>
            )}
          </div>
        ))}

        {/* Add step */}
        <button
          onClick={addStep}
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            opacity: 0.5,
            background: "none",
            border: "none",
            cursor: "pointer",
            padding: "12px 0 4px",
            color: "var(--color-text-secondary)",
          }}
          onMouseEnter={(e) => ((e.currentTarget as HTMLButtonElement).style.opacity = "0.8")}
          onMouseLeave={(e) => ((e.currentTarget as HTMLButtonElement).style.opacity = "0.5")}
        >
          + Add step
        </button>
      </div>
    </section>
  );
}
