/**
 * WhatHappenedSlide — Situation → Complication as one narrative arc.
 * Slide 3: 3-sentence narrative + health arc timeline + key losses.
 */
import { EditableText } from "@/components/ui/EditableText";
import type { RiskWhatHappened } from "@/types";

interface WhatHappenedSlideProps {
  data: RiskWhatHappened;
  onUpdate?: (data: RiskWhatHappened) => void;
}

const statusColors: Record<string, string> = {
  green: "var(--color-garden-sage)",
  yellow: "var(--color-spice-turmeric)",
  red: "var(--color-spice-chili)",
  amber: "var(--color-spice-terracotta)",
};

export function WhatHappenedSlide({ data, onUpdate }: WhatHappenedSlideProps) {
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
          fontSize: 12,
          fontWeight: 600,
          textTransform: "uppercase",
          letterSpacing: "0.12em",
          color: "var(--color-text-secondary)",
          marginBottom: 24,
        }}
      >
        What Happened
      </div>

      {/* 3-sentence narrative */}
      <EditableText
        as="p"
        value={data.narrative}
        onChange={(v) => onUpdate?.({ ...data, narrative: v })}
        multiline
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 22,
          lineHeight: 1.6,
          color: "var(--color-text-primary)",
          maxWidth: 800,
          margin: "0 0 36px",
        }}
      />

      {/* Health arc — horizontal timeline with colored bars */}
      {data.healthArc && data.healthArc.length > 0 && (
        <div style={{ display: "flex", gap: 4, marginBottom: 36, maxWidth: 800 }}>
          {data.healthArc.map((snap, i) => {
            const color =
              statusColors[snap.status.toLowerCase()] ?? "var(--color-text-secondary)";
            return (
              <div
                key={i}
                style={{
                  flex: 1,
                  display: "flex",
                  flexDirection: "column",
                  gap: 8,
                }}
              >
                {/* Color bar */}
                <div
                  style={{
                    height: 6,
                    borderRadius: 3,
                    background: color,
                  }}
                />
                {/* Period label */}
                <EditableText
                  as="div"
                  value={snap.period}
                  onChange={(v) => {
                    const updated = [...(data.healthArc ?? [])];
                    updated[i] = { ...updated[i], period: v };
                    onUpdate?.({ ...data, healthArc: updated });
                  }}
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 12,
                    fontWeight: 600,
                    color: "var(--color-text-primary)",
                  }}
                />
                {snap.detail && (
                  <EditableText
                    as="div"
                    value={snap.detail}
                    onChange={(v) => {
                      const updated = [...(data.healthArc ?? [])];
                      updated[i] = { ...updated[i], detail: v };
                      onUpdate?.({ ...data, healthArc: updated });
                    }}
                    style={{
                      fontFamily: "var(--font-sans)",
                      fontSize: 13,
                      color: "var(--color-text-secondary)",
                    }}
                  />
                )}
              </div>
            );
          })}
        </div>
      )}

      {/* Key losses */}
      {data.keyLosses && data.keyLosses.length > 0 && (
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
            Key Losses
          </div>
          {data.keyLosses.slice(0, 3).map((loss, i) => (
            <div
              key={i}
              style={{
                display: "flex",
                gap: 12,
                alignItems: "baseline",
                padding: "8px 0",
              }}
            >
              <span
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 16,
                  fontWeight: 600,
                  color: "var(--color-spice-terracotta)",
                  flexShrink: 0,
                  minWidth: 20,
                }}
              >
                {i + 1}
              </span>
              <EditableText
                value={loss}
                onChange={(v) => {
                  const updated = [...(data.keyLosses ?? [])];
                  updated[i] = v;
                  onUpdate?.({ ...data, keyLosses: updated });
                }}
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 16,
                  color: "var(--color-text-primary)",
                }}
              />
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
