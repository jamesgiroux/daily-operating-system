/**
 * IntoNextWeekSlide — What carries forward into next week.
 * Slide 5: numbered list of priorities + intentions.
 */
import { useState } from "react";
import { EditableText } from "@/components/ui/EditableText";
import type { WeeklyImpactContent } from "@/types/reports";

interface IntoNextWeekSlideProps {
  content: WeeklyImpactContent;
  onUpdate: (updated: WeeklyImpactContent) => void;
}

export function IntoNextWeekSlide({ content, onUpdate }: IntoNextWeekSlideProps) {
  const [hoveredItem, setHoveredItem] = useState<number | null>(null);

  const items = content.intoNextWeek;

  const updateItem = (i: number, value: string) => {
    const updated = [...items];
    updated[i] = value;
    onUpdate({ ...content, intoNextWeek: updated });
  };

  const removeItem = (i: number) => {
    onUpdate({ ...content, intoNextWeek: items.filter((_, j) => j !== i) });
  };

  const addItem = () => {
    onUpdate({ ...content, intoNextWeek: [...items, "Something to carry forward"] });
  };

  return (
    <section
      id="next-week"
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
          color: "var(--color-garden-eucalyptus)",
          marginBottom: 24,
        }}
      >
        Into Next Week
      </div>

      {/* Numbered items */}
      {items.length > 0 && (
        <div style={{ maxWidth: 800, marginBottom: 24 }}>
          {items.map((item, i) => (
            <div
              key={i}
              onMouseEnter={() => setHoveredItem(i)}
              onMouseLeave={() => setHoveredItem(null)}
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
                  color: "var(--color-garden-eucalyptus)",
                  minWidth: 24,
                  flexShrink: 0,
                }}
              >
                {i + 1}
              </span>
              <div style={{ flex: 1 }}>
                <EditableText
                  value={item}
                  onChange={(v) => updateItem(i, v)}
                  multiline
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 17,
                    lineHeight: 1.5,
                    color: "var(--color-text-primary)",
                  }}
                />
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
          marginTop: items.length > 0 ? 8 : 0,
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
