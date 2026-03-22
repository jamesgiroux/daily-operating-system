/**
 * WhatAheadSlide — risks, renewal context, and recommended actions.
 * Slide 5: three stacked sections.
 */
import { useState } from "react";
import { EditableText } from "@/components/ui/EditableText";
import type { AccountHealthContent } from "./types";

interface WhatAheadSlideProps {
  content: AccountHealthContent;
  onUpdate: (c: AccountHealthContent) => void;
}

const riskStatusColors: Record<string, string> = {
  open: "var(--color-spice-terracotta)",
  mitigated: "var(--color-spice-turmeric)",
  resolved: "var(--color-garden-sage)",
};

export function WhatAheadSlide({ content, onUpdate }: WhatAheadSlideProps) {
  const [hoveredRisk, setHoveredRisk] = useState<number | null>(null);
  const [hoveredAction, setHoveredAction] = useState<number | null>(null);
  const [renewalHovered, setRenewalHovered] = useState(false);

  return (
    <section
      id="what-ahead"
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
          color: "var(--color-text-secondary)",
          marginBottom: 32,
        }}
      >
        What's Ahead
      </div>

      {/* ── RISKS ── */}
      <div style={{ maxWidth: 800, marginBottom: 40 }}>
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
          Risks
        </div>

        {content.risks.map((item, i) => (
          <div
            key={i}
            onMouseEnter={() => setHoveredRisk(i)}
            onMouseLeave={() => setHoveredRisk(null)}
            style={{
              display: "flex",
              gap: 16,
              alignItems: "baseline",
              padding: "10px 0",
              borderBottom: "1px solid var(--color-rule-light)",
            }}
          >
            <EditableText
              value={item.risk}
              onChange={(v) => {
                const updated = [...content.risks];
                updated[i] = { ...updated[i], risk: v };
                onUpdate({ ...content, risks: updated });
              }}
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 15,
                color: "var(--color-text-primary)",
                flex: 1,
                lineHeight: 1.5,
              }}
            />
            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 11,
                fontWeight: 600,
                textTransform: "uppercase",
                letterSpacing: "0.06em",
                color:
                  riskStatusColors[item.status.toLowerCase()] ?? "var(--color-text-tertiary)",
                flexShrink: 0,
              }}
            >
              {item.status}
            </span>
            {content.risks.length > 1 && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  onUpdate({
                    ...content,
                    risks: content.risks.filter((_, j) => j !== i),
                  });
                }}
                style={{
                  opacity: hoveredRisk === i ? 0.6 : 0,
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

        <button
          onClick={() =>
            onUpdate({
              ...content,
              risks: [...content.risks, { risk: "New risk", status: "open" }],
            })
          }
          style={{
            background: "none",
            border: "none",
            cursor: "pointer",
            padding: "8px 0",
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            color: "var(--color-text-tertiary)",
            letterSpacing: "0.06em",
            opacity: 0.5,
            display: "block",
            marginTop: 4,
          }}
          onMouseEnter={(e) => (e.currentTarget.style.opacity = "0.8")}
          onMouseLeave={(e) => (e.currentTarget.style.opacity = "0.5")}
        >
          + Add risk
        </button>
      </div>

      {/* ── RENEWAL ── (conditional) */}
      {content.renewalContext != null && (
        <div
          style={{ maxWidth: 800, marginBottom: 40 }}
          onMouseEnter={() => setRenewalHovered(true)}
          onMouseLeave={() => setRenewalHovered(false)}
        >
          <div
            style={{
              display: "flex",
              alignItems: "baseline",
              gap: 8,
              marginBottom: 12,
            }}
          >
            <div
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 11,
                fontWeight: 600,
                textTransform: "uppercase",
                letterSpacing: "0.1em",
                color: "var(--color-text-secondary)",
                flex: 1,
              }}
            >
              Renewal
            </div>
            <button
              onClick={() => onUpdate({ ...content, renewalContext: null })}
              style={{
                opacity: renewalHovered ? 0.6 : 0,
                transition: "opacity 0.15s",
                background: "none",
                border: "none",
                cursor: "pointer",
                padding: "4px 6px",
                fontSize: 14,
                color: "var(--color-text-tertiary)",
              }}
              aria-label="Remove renewal context"
            >
              ✕
            </button>
          </div>
          <EditableText
            as="p"
            value={content.renewalContext}
            onChange={(v) => onUpdate({ ...content, renewalContext: v })}
            multiline
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 15,
              fontStyle: "italic",
              lineHeight: 1.6,
              color: "var(--color-text-primary)",
              margin: 0,
            }}
          />
        </div>
      )}

      {/* ── RECOMMENDED ACTIONS ── */}
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
          Recommended Actions
        </div>

        {content.recommendedActions.map((action, i) => (
          <div
            key={i}
            onMouseEnter={() => setHoveredAction(i)}
            onMouseLeave={() => setHoveredAction(null)}
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
                color: "var(--color-spice-turmeric)",
                minWidth: 24,
                flexShrink: 0,
              }}
            >
              {i + 1}
            </span>
            <EditableText
              value={action}
              onChange={(v) => {
                const updated = [...content.recommendedActions];
                updated[i] = v;
                onUpdate({ ...content, recommendedActions: updated });
              }}
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 17,
                color: "var(--color-text-primary)",
                flex: 1,
              }}
            />
            {content.recommendedActions.length > 1 && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  onUpdate({
                    ...content,
                    recommendedActions: content.recommendedActions.filter((_, j) => j !== i),
                  });
                }}
                style={{
                  opacity: hoveredAction === i ? 0.6 : 0,
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

        <button
          onClick={() =>
            onUpdate({
              ...content,
              recommendedActions: [...content.recommendedActions, "New action"],
            })
          }
          style={{
            background: "none",
            border: "none",
            cursor: "pointer",
            padding: "12px 0",
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            color: "var(--color-text-tertiary)",
            letterSpacing: "0.06em",
            opacity: 0.5,
            display: "block",
          }}
          onMouseEnter={(e) => (e.currentTarget.style.opacity = "0.8")}
          onMouseLeave={(e) => (e.currentTarget.style.opacity = "0.5")}
        >
          + Add action
        </button>
      </div>
    </section>
  );
}
