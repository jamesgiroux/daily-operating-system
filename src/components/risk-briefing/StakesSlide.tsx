/**
 * StakesSlide — Who matters + what's the money.
 * Slide 4: financial headline + stakeholder cards + worst case.
 * Merges TheRoom + Commercial from v2 into one stakes view.
 */
import { useState } from "react";
import { EditableText } from "@/components/ui/EditableText";
import type { RiskStakes } from "@/types";

interface StakesSlideProps {
  data: RiskStakes;
  onUpdate?: (data: RiskStakes) => void;
}

const alignmentColors: Record<string, string> = {
  champion: "var(--color-garden-sage)",
  neutral: "var(--color-spice-turmeric)",
  detractor: "var(--color-spice-chili)",
  unknown: "var(--color-text-secondary)",
};

export function StakesSlide({ data, onUpdate }: StakesSlideProps) {
  const [hoveredStakeholder, setHoveredStakeholder] = useState<number | null>(null);

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
          fontSize: 12,
          fontWeight: 600,
          textTransform: "uppercase",
          letterSpacing: "0.12em",
          color: "var(--color-text-secondary)",
          marginBottom: 24,
        }}
      >
        The Stakes
      </div>

      {/* Financial headline */}
      {data.financialHeadline && (
        <EditableText
          as="h2"
          value={data.financialHeadline}
          onChange={(v) => onUpdate?.({ ...data, financialHeadline: v })}
          multiline={false}
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 32,
            fontWeight: 400,
            lineHeight: 1.25,
            color: "var(--color-text-primary)",
            maxWidth: 800,
            margin: "0 0 36px",
          }}
        />
      )}

      {/* Stakeholder cards — max 4, full width */}
      {data.stakeholders && data.stakeholders.length > 0 && (
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fill, minmax(240px, 1fr))",
            gap: 16,
            marginBottom: 36,
            maxWidth: 800,
          }}
        >
          {data.stakeholders.slice(0, 4).map((s, i) => (
            <div
              key={i}
              onMouseEnter={() => setHoveredStakeholder(i)}
              onMouseLeave={() => setHoveredStakeholder(null)}
              style={{
                padding: "16px 20px",
                border: "1px solid var(--color-rule-light)",
                borderRadius: 6,
                position: "relative",
              }}
            >
              {/* Dismiss */}
              {(data.stakeholders?.length ?? 0) > 1 && (
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    onUpdate?.({
                      ...data,
                      stakeholders: (data.stakeholders ?? []).filter((_, j) => j !== i),
                    });
                  }}
                  style={{
                    position: "absolute",
                    top: 8,
                    right: 8,
                    opacity: hoveredStakeholder === i ? 0.6 : 0,
                    transition: "opacity 0.15s",
                    background: "none",
                    border: "none",
                    cursor: "pointer",
                    padding: "4px 6px",
                    fontSize: 14,
                    color: "var(--color-text-tertiary)",
                    zIndex: 1,
                  }}
                  aria-label="Remove"
                >
                  ✕
                </button>
              )}
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 10,
                  marginBottom: 8,
                }}
              >
                {/* Alignment dot */}
                <span
                  style={{
                    width: 9,
                    height: 9,
                    borderRadius: "50%",
                    background:
                      alignmentColors[s.alignment?.toLowerCase() ?? ""] ??
                      "var(--color-text-secondary)",
                    flexShrink: 0,
                  }}
                />
                <EditableText
                  value={s.name}
                  onChange={(v) => {
                    const updated = [...(data.stakeholders ?? [])];
                    updated[i] = { ...updated[i], name: v };
                    onUpdate?.({ ...data, stakeholders: updated });
                  }}
                  multiline={false}
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 16,
                    fontWeight: 600,
                    color: "var(--color-text-primary)",
                  }}
                />
              </div>
              {s.role && (
                <EditableText
                  as="div"
                  value={s.role}
                  multiline={false}
                  onChange={(v) => {
                    const updated = [...(data.stakeholders ?? [])];
                    updated[i] = { ...updated[i], role: v };
                    onUpdate?.({ ...data, stakeholders: updated });
                  }}
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 14,
                    color: "var(--color-text-primary)",
                    marginBottom: 8,
                  }}
                />
              )}
              {/* Badges row */}
              <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                {s.engagement && (
                  <span
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 10,
                      fontWeight: 600,
                      textTransform: "uppercase",
                      letterSpacing: "0.06em",
                      color: "var(--color-text-secondary)",
                      border: "1px solid var(--color-rule-light)",
                      borderRadius: 3,
                      padding: "3px 8px",
                    }}
                  >
                    {s.engagement}
                  </span>
                )}
                {s.decisionWeight && (
                  <span
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 10,
                      fontWeight: 600,
                      textTransform: "uppercase",
                      letterSpacing: "0.06em",
                      color: "var(--color-text-secondary)",
                      border: "1px solid var(--color-rule-light)",
                      borderRadius: 3,
                      padding: "3px 8px",
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
      <div style={{ display: "flex", flexDirection: "column", gap: 16, maxWidth: 800 }}>
        {data.decisionMaker && (
          <div style={{ display: "flex", gap: 12, alignItems: "baseline" }}>
            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 11,
                fontWeight: 600,
                textTransform: "uppercase",
                letterSpacing: "0.08em",
                color: "var(--color-text-secondary)",
              }}
            >
              Decision Maker
            </span>
            <EditableText
              value={data.decisionMaker}
              onChange={(v) => onUpdate?.({ ...data, decisionMaker: v })}
              multiline={false}
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 16,
                color: "var(--color-text-primary)",
              }}
            />
          </div>
        )}
        {data.worstCase && (
          <EditableText
            as="p"
            value={data.worstCase}
            onChange={(v) => onUpdate?.({ ...data, worstCase: v })}
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 16,
              lineHeight: 1.5,
              color: "var(--color-spice-chili)",
              margin: 0,
              borderLeft: "3px solid var(--color-spice-chili)",
              paddingLeft: 16,
            }}
          />
        )}
      </div>
    </section>
  );
}
