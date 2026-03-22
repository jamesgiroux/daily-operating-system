/**
 * MetricsSlide — Slide 4: By the Numbers.
 * Success metrics as a clean table-style layout with trend arrows.
 */
import { useState } from "react";
import { EditableText } from "@/components/ui/EditableText";
import type { EbrQbrContent, EbrQbrMetric } from "@/types/reports";

interface MetricsSlideProps {
  content: EbrQbrContent;
  onUpdate: (c: EbrQbrContent) => void;
}

function trendArrow(trend: string | null | undefined): { symbol: string; color: string } {
  if (!trend) return { symbol: "→", color: "var(--color-text-secondary)" };
  const t = trend.toLowerCase();
  if (t === "up" || t === "↑" || t === "increasing" || t === "positive") {
    return { symbol: "↑", color: "var(--color-garden-sage)" };
  }
  if (t === "down" || t === "↓" || t === "decreasing" || t === "negative") {
    return { symbol: "↓", color: "var(--color-spice-terracotta)" };
  }
  return { symbol: "→", color: "var(--color-text-secondary)" };
}

export function MetricsSlide({ content, onUpdate }: MetricsSlideProps) {
  const [hoveredIndex, setHoveredIndex] = useState<number | null>(null);

  const metrics = content.successMetrics;

  function updateMetric(i: number, updated: EbrQbrMetric) {
    const updatedMetrics = [...metrics];
    updatedMetrics[i] = updated;
    onUpdate({ ...content, successMetrics: updatedMetrics });
  }

  function removeMetric(i: number) {
    onUpdate({ ...content, successMetrics: metrics.filter((_, j) => j !== i) });
  }

  function addMetric() {
    onUpdate({
      ...content,
      successMetrics: [
        ...metrics,
        { metric: "", baseline: null, current: "", trend: null },
      ],
    });
  }

  return (
    <section
      id="by-the-numbers"
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
        By the Numbers
      </div>

      {/* Metrics table */}
      <div style={{ maxWidth: 700 }}>
        {metrics.map((metric, i) => {
          const { symbol, color } = trendArrow(metric.trend);

          return (
            <div
              key={i}
              onMouseEnter={() => setHoveredIndex(i)}
              onMouseLeave={() => setHoveredIndex(null)}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 16,
                padding: "14px 0",
                borderBottom: "1px solid var(--color-rule-light)",
              }}
            >
              {/* Metric name */}
              <div style={{ flex: 1 }}>
                <EditableText
                  value={metric.metric}
                  onChange={(v) => updateMetric(i, { ...metric, metric: v })}
                  multiline={false}
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 16,
                    color: "var(--color-text-primary)",
                  }}
                />
              </div>

              {/* Baseline → current */}
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 6,
                  flexShrink: 0,
                }}
              >
                {metric.baseline && (
                  <>
                    <EditableText
                      value={metric.baseline}
                      onChange={(v) => updateMetric(i, { ...metric, baseline: v || null })}
                      multiline={false}
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 14,
                        color: "var(--color-text-tertiary)",
                      }}
                    />
                    <span
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 12,
                        color: "var(--color-text-tertiary)",
                      }}
                    >
                      →
                    </span>
                  </>
                )}
                <EditableText
                  value={metric.current}
                  onChange={(v) => updateMetric(i, { ...metric, current: v })}
                  multiline={false}
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 14,
                    fontWeight: 600,
                    color: "var(--color-text-primary)",
                  }}
                />
              </div>

              {/* Trend arrow */}
              <div
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 18,
                  fontWeight: 600,
                  color,
                  flexShrink: 0,
                  width: 24,
                  textAlign: "center",
                }}
              >
                {symbol}
              </div>

              {/* Dismiss */}
              {metrics.length > 1 && (
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    removeMetric(i);
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
          );
        })}

        {/* Add metric */}
        <button
          onClick={addMetric}
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
          + Add metric
        </button>
      </div>
    </section>
  );
}
