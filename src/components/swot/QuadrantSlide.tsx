/**
 * QuadrantSlide — Reusable full-viewport slide for any SWOT quadrant.
 * Used for Strengths, Weaknesses, Opportunities, and Threats slides.
 * Each item is editable (text + source). Items can be added or removed.
 */
import { useState } from "react";
import { EditableText } from "@/components/ui/EditableText";
import type { SwotItem } from "@/types/reports";

interface QuadrantSlideProps {
  id: string;
  overline: string;
  accentColor: string;
  items: SwotItem[];
  onUpdate: (items: SwotItem[]) => void;
  emptyLabel?: string;
}

export function QuadrantSlide({
  id,
  overline,
  accentColor,
  items,
  onUpdate,
  emptyLabel,
}: QuadrantSlideProps) {
  const [hoveredIndex, setHoveredIndex] = useState<number | null>(null);

  function handleTextChange(i: number, text: string) {
    const updated = [...items];
    updated[i] = { ...updated[i], text };
    onUpdate(updated);
  }

  function handleSourceChange(i: number, source: string) {
    const updated = [...items];
    updated[i] = { ...updated[i], source: source || null };
    onUpdate(updated);
  }

  function handleRemove(i: number) {
    onUpdate(items.filter((_, j) => j !== i));
  }

  function handleAdd() {
    onUpdate([...items, { text: "New item", source: null }]);
  }

  return (
    <section
      id={id}
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
          color: accentColor,
          marginBottom: 24,
        }}
      >
        {overline}
      </div>

      {/* Items list */}
      <div style={{ maxWidth: 800 }}>
        {items.length === 0 && emptyLabel && (
          <div
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 15,
              color: "var(--color-text-tertiary)",
              fontStyle: "italic",
              padding: "14px 0",
            }}
          >
            {emptyLabel}
          </div>
        )}

        {items.map((item, i) => (
          <div
            key={i}
            onMouseEnter={() => setHoveredIndex(i)}
            onMouseLeave={() => setHoveredIndex(null)}
            style={{
              display: "flex",
              gap: 16,
              alignItems: "flex-start",
              padding: "14px 0",
              borderBottom: "1px solid var(--color-rule-light)",
            }}
          >
            {/* Accent dot */}
            <span
              style={{
                width: 8,
                height: 8,
                borderRadius: "50%",
                background: accentColor,
                flexShrink: 0,
                marginTop: 8,
              }}
            />

            {/* Text + source */}
            <div style={{ flex: 1, minWidth: 0 }}>
              <EditableText
                value={item.text}
                onChange={(v) => handleTextChange(i, v)}
                multiline
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 17,
                  lineHeight: 1.5,
                  color: "var(--color-text-primary)",
                  display: "block",
                }}
              />
              {/* Source — always rendered; shows placeholder when empty */}
              <EditableText
                value={item.source ?? ""}
                onChange={(v) => handleSourceChange(i, v)}
                multiline={false}
                placeholder="Add source"
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 11,
                  color: "var(--color-text-tertiary)",
                  fontStyle: "italic",
                  display: "block",
                  marginTop: 4,
                }}
              />
            </div>

            {/* Hover-reveal dismiss button — only shown when more than 1 item */}
            {items.length > 1 && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  handleRemove(i);
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
                  marginTop: 4,
                }}
                aria-label="Remove item"
              >
                ✕
              </button>
            )}
          </div>
        ))}

        {/* Add item */}
        <button
          onClick={handleAdd}
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            color: "var(--color-text-tertiary)",
            background: "none",
            border: "none",
            cursor: "pointer",
            padding: "12px 0 0",
            opacity: 0.5,
            transition: "opacity 0.15s",
            letterSpacing: "0.06em",
          }}
          onMouseEnter={(e) => (e.currentTarget.style.opacity = "0.8")}
          onMouseLeave={(e) => (e.currentTarget.style.opacity = "0.5")}
        >
          + Add item
        </button>
      </div>
    </section>
  );
}
