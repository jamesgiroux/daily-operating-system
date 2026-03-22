/**
 * ValueDeliveredEbrSlide — Slide 3: The most important slide.
 * Shows value items with outcome + impact + source citation.
 * Optional customer quote blockquote at the bottom.
 * Uses customerQuote from EbrQbrContent for the quote field.
 */
import { useState } from "react";
import { EditableText } from "@/components/ui/EditableText";
import type { EbrQbrContent, EbrQbrValueItem } from "@/types/reports";

interface ValueDeliveredEbrSlideProps {
  content: EbrQbrContent;
  onUpdate: (c: EbrQbrContent) => void;
}

export function ValueDeliveredEbrSlide({ content, onUpdate }: ValueDeliveredEbrSlideProps) {
  const [hoveredIndex, setHoveredIndex] = useState<number | null>(null);

  const items = content.valueDelivered;
  const quote = content.customerQuote ?? null;

  function updateItem(i: number, updated: EbrQbrValueItem) {
    const updatedItems = [...items];
    updatedItems[i] = updated;
    onUpdate({ ...content, valueDelivered: updatedItems });
  }

  function removeItem(i: number) {
    onUpdate({ ...content, valueDelivered: items.filter((_, j) => j !== i) });
  }

  function addItem() {
    onUpdate({
      ...content,
      valueDelivered: [...items, { outcome: "", source: "", impact: null }],
    });
  }

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
          color: "var(--color-garden-larkspur)",
          marginBottom: 36,
        }}
      >
        Value Delivered
      </div>

      {/* Value items */}
      <div style={{ maxWidth: 760, marginBottom: 40 }}>
        {items.map((item, i) => (
          <div
            key={i}
            onMouseEnter={() => setHoveredIndex(i)}
            onMouseLeave={() => setHoveredIndex(null)}
            style={{
              position: "relative",
              borderLeft: "3px solid var(--color-garden-larkspur)",
              padding: "16px 20px",
              marginBottom: 16,
            }}
          >
            {/* Outcome text */}
            <EditableText
              value={item.outcome}
              onChange={(v) => updateItem(i, { ...item, outcome: v })}
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 18,
                fontWeight: 500,
                color: "var(--color-text-primary)",
                display: "block",
                marginBottom: item.impact ? 6 : 4,
              }}
            />

            {/* Impact callout — turmeric */}
            {item.impact && (
              <div
                style={{
                  marginBottom: 6,
                }}
              >
                <EditableText
                  as="span"
                  value={item.impact}
                  onChange={(v) => updateItem(i, { ...item, impact: v || null })}
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 14,
                    color: "var(--color-spice-turmeric)",
                    fontWeight: 500,
                  }}
                />
              </div>
            )}

            {/* Source citation */}
            <EditableText
              as="span"
              value={item.source}
              onChange={(v) => updateItem(i, { ...item, source: v })}
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 12,
                color: "var(--color-text-tertiary)",
                fontStyle: "italic",
                display: "block",
              }}
            />

            {/* Dismiss button */}
            {items.length > 1 && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  removeItem(i);
                }}
                style={{
                  position: "absolute",
                  top: 12,
                  right: 8,
                  opacity: hoveredIndex === i ? 0.6 : 0,
                  transition: "opacity 0.15s",
                  background: "none",
                  border: "none",
                  cursor: "pointer",
                  padding: "4px 6px",
                  fontSize: 14,
                  color: "var(--color-text-tertiary)",
                }}
                aria-label="Remove"
              >
                ✕
              </button>
            )}
          </div>
        ))}

        {/* Add outcome button */}
        <button
          onClick={addItem}
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            opacity: 0.5,
            background: "none",
            border: "none",
            cursor: "pointer",
            padding: "4px 0",
            color: "var(--color-text-secondary)",
            marginLeft: 24,
          }}
          onMouseEnter={(e) => ((e.currentTarget as HTMLButtonElement).style.opacity = "0.8")}
          onMouseLeave={(e) => ((e.currentTarget as HTMLButtonElement).style.opacity = "0.5")}
        >
          + Add outcome
        </button>
      </div>

      {/* Customer quote blockquote */}
      {quote !== null ? (
        <div
          style={{
            maxWidth: 760,
            position: "relative",
          }}
        >
          {/* Opening quotation mark */}
          <div
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 60,
              lineHeight: 0.5,
              color: "var(--color-garden-larkspur)",
              marginBottom: 16,
              userSelect: "none",
            }}
          >
            "
          </div>

          <EditableText
            as="p"
            value={quote}
            onChange={(v) => onUpdate({ ...content, customerQuote: v || null })}
            style={{
              fontFamily: "var(--font-serif)",
              fontStyle: "italic",
              fontSize: 20,
              lineHeight: 1.6,
              color: "var(--color-text-primary)",
              maxWidth: 700,
              margin: "0 0 12px",
            }}
          />

          <div
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 12,
              color: "var(--color-text-tertiary)",
            }}
          >
            — Customer
          </div>

          {/* Clear quote button */}
          <button
            onClick={() => onUpdate({ ...content, customerQuote: null })}
            style={{
              position: "absolute",
              top: 0,
              right: 0,
              opacity: 0.4,
              background: "none",
              border: "none",
              cursor: "pointer",
              padding: "4px 6px",
              fontSize: 14,
              color: "var(--color-text-tertiary)",
            }}
            onMouseEnter={(e) => ((e.currentTarget as HTMLButtonElement).style.opacity = "0.7")}
            onMouseLeave={(e) => ((e.currentTarget as HTMLButtonElement).style.opacity = "0.4")}
            aria-label="Remove quote"
          >
            ✕
          </button>
        </div>
      ) : (
        <button
          onClick={() => onUpdate({ ...content, customerQuote: "" })}
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            opacity: 0.5,
            background: "none",
            border: "none",
            cursor: "pointer",
            padding: "4px 0",
            color: "var(--color-text-secondary)",
          }}
          onMouseEnter={(e) => ((e.currentTarget as HTMLButtonElement).style.opacity = "0.8")}
          onMouseLeave={(e) => ((e.currentTarget as HTMLButtonElement).style.opacity = "0.5")}
        >
          + Add customer quote
        </button>
      )}
    </section>
  );
}
