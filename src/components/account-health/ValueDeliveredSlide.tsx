/**
 * ValueDeliveredSlide — evidence of impact.
 * Slide 4: vertical list of outcome cards, each editable with source attribution.
 */
import { useState } from "react";
import { EditableText } from "@/components/ui/EditableText";
import type { AccountHealthContent } from "./types";

interface ValueDeliveredSlideProps {
  content: AccountHealthContent;
  onUpdate: (c: AccountHealthContent) => void;
}

export function ValueDeliveredSlide({ content, onUpdate }: ValueDeliveredSlideProps) {
  const [hoveredItem, setHoveredItem] = useState<number | null>(null);

  return (
    <section
      id="value-delivered"
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
          marginBottom: 24,
        }}
      >
        Value Delivered
      </div>

      {/* Outcome list */}
      <div style={{ maxWidth: 800 }}>
        {content.valueDelivered.map((item, i) => (
          <div
            key={i}
            onMouseEnter={() => setHoveredItem(i)}
            onMouseLeave={() => setHoveredItem(null)}
            style={{
              display: "flex",
              gap: 16,
              alignItems: "flex-start",
              padding: "16px 0",
              borderBottom: "1px solid var(--color-rule-light)",
            }}
          >
            <div style={{ flex: 1 }}>
              <EditableText
                value={item.text}
                onChange={(v) => {
                  const updated = [...content.valueDelivered];
                  updated[i] = { ...updated[i], text: v };
                  onUpdate({ ...content, valueDelivered: updated });
                }}
                multiline
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 17,
                  lineHeight: 1.5,
                  color: "var(--color-text-primary)",
                  display: "block",
                  marginBottom: item.source ? 6 : 0,
                }}
              />
              {item.source && (
                <span
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 11,
                    color: "var(--color-text-tertiary)",
                    fontStyle: "italic",
                  }}
                >
                  {item.source}
                </span>
              )}
            </div>

            {content.valueDelivered.length > 1 && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  onUpdate({
                    ...content,
                    valueDelivered: content.valueDelivered.filter((_, j) => j !== i),
                  });
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
                  marginTop: 2,
                }}
                aria-label="Remove"
              >
                ✕
              </button>
            )}
          </div>
        ))}

        {/* Add outcome */}
        <button
          onClick={() =>
            onUpdate({
              ...content,
              valueDelivered: [
                ...content.valueDelivered,
                { text: "New outcome", source: null },
              ],
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
          + Add outcome
        </button>
      </div>
    </section>
  );
}
