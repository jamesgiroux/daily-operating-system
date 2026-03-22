/**
 * WatchSlide — Things to monitor.
 * Slide 4: items flagged for watching with optional source.
 */
import { useState } from "react";
import { EditableText } from "@/components/ui/EditableText";
import type { WeeklyImpactContent, WeeklyImpactItem } from "@/types/reports";

interface WatchSlideProps {
  content: WeeklyImpactContent;
  onUpdate: (updated: WeeklyImpactContent) => void;
}

export function WatchSlide({ content, onUpdate }: WatchSlideProps) {
  const [hoveredItem, setHoveredItem] = useState<number | null>(null);

  const items = content.watch;

  const updateItem = (i: number, patch: Partial<WeeklyImpactItem>) => {
    const updated = [...items];
    updated[i] = { ...updated[i], ...patch };
    onUpdate({ ...content, watch: updated });
  };

  const removeItem = (i: number) => {
    onUpdate({ ...content, watch: items.filter((_, j) => j !== i) });
  };

  const addItem = () => {
    onUpdate({ ...content, watch: [...items, { text: "Something to monitor", source: null }] });
  };

  return (
    <section
      id="watch"
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
          color: "var(--color-spice-terracotta)",
          marginBottom: 24,
        }}
      >
        Watch
      </div>

      {items.length === 0 ? (
        <p
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 20,
            fontStyle: "italic",
            color: "var(--color-text-tertiary)",
            margin: 0,
          }}
        >
          Nothing flagged for monitoring.
        </p>
      ) : (
        <div style={{ maxWidth: 800 }}>
          {items.map((item, i) => (
            <div
              key={i}
              onMouseEnter={() => setHoveredItem(i)}
              onMouseLeave={() => setHoveredItem(null)}
              style={{
                display: "flex",
                gap: 12,
                alignItems: "flex-start",
                padding: "12px 0",
                borderBottom: "1px solid var(--color-rule-light)",
              }}
            >
              <div style={{ flex: 1 }}>
                <EditableText
                  value={item.text}
                  onChange={(v) => updateItem(i, { text: v })}
                  multiline
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 16,
                    lineHeight: 1.5,
                    color: "var(--color-text-primary)",
                    display: "block",
                  }}
                />
                {item.source && (
                  <span
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 11,
                      color: "var(--color-text-tertiary)",
                      marginTop: 4,
                      display: "inline-block",
                      letterSpacing: "0.04em",
                    }}
                  >
                    {item.source}
                  </span>
                )}
              </div>

              {items.length > 1 && (
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    removeItem(i);
                  }}
                  style={{
                    opacity: hoveredItem === i ? 0.6 : 0,
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
        </div>
      )}

      {/* Add button */}
      <button
        onClick={addItem}
        style={{
          marginTop: 20,
          fontFamily: "var(--font-mono)",
          fontSize: 12,
          fontWeight: 600,
          letterSpacing: "0.08em",
          textTransform: "uppercase",
          color: "var(--color-text-tertiary)",
          background: "none",
          border: "1px dashed var(--color-rule-light)",
          borderRadius: 4,
          padding: "6px 16px",
          cursor: "pointer",
          alignSelf: "flex-start",
        }}
      >
        + Add
      </button>
    </section>
  );
}
