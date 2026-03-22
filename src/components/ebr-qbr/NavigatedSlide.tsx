/**
 * NavigatedSlide — Slide 5: What We Navigated.
 * Challenges and resolutions with status-colored left borders.
 */
import { useState } from "react";
import { EditableText } from "@/components/ui/EditableText";
import type { EbrQbrContent, EbrQbrRisk } from "@/types/reports";

interface NavigatedSlideProps {
  content: EbrQbrContent;
  onUpdate: (c: EbrQbrContent) => void;
}

const statusColors: Record<string, string> = {
  resolved: "var(--color-garden-sage)",
  open: "var(--color-spice-terracotta)",
  mitigated: "var(--color-spice-turmeric)",
};

function getStatusColor(status: string): string {
  return statusColors[status.toLowerCase()] ?? "var(--color-text-secondary)";
}

export function NavigatedSlide({ content, onUpdate }: NavigatedSlideProps) {
  const [hoveredIndex, setHoveredIndex] = useState<number | null>(null);

  const items = content.challengesAndResolutions;

  function updateItem(i: number, updated: EbrQbrRisk) {
    const updatedItems = [...items];
    updatedItems[i] = updated;
    onUpdate({ ...content, challengesAndResolutions: updatedItems });
  }

  function removeItem(i: number) {
    onUpdate({ ...content, challengesAndResolutions: items.filter((_, j) => j !== i) });
  }

  function addItem() {
    onUpdate({
      ...content,
      challengesAndResolutions: [...items, { risk: "", resolution: null, status: "open" }],
    });
  }

  return (
    <section
      id="what-we-navigated"
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
        What We Navigated
      </div>

      {/* Challenge items */}
      <div style={{ maxWidth: 800 }}>
        {items.map((item, i) => {
          const borderColor = getStatusColor(item.status);

          return (
            <div
              key={i}
              onMouseEnter={() => setHoveredIndex(i)}
              onMouseLeave={() => setHoveredIndex(null)}
              style={{
                position: "relative",
                borderLeft: `3px solid ${borderColor}`,
                padding: "16px 20px",
                marginBottom: 16,
                background: "transparent",
              }}
            >
              {/* Status badge */}
              <div
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 10,
                  fontWeight: 600,
                  textTransform: "uppercase",
                  letterSpacing: "0.08em",
                  color: borderColor,
                  marginBottom: 8,
                }}
              >
                {item.status}
              </div>

              {/* Risk / challenge text */}
              <EditableText
                value={item.risk}
                onChange={(v) => updateItem(i, { ...item, risk: v })}
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 17,
                  fontWeight: 500,
                  color: "var(--color-text-primary)",
                  display: "block",
                  marginBottom: item.resolution ? 8 : 0,
                }}
              />

              {/* Resolution */}
              {item.resolution !== null && (
                <EditableText
                  as="p"
                  value={item.resolution ?? ""}
                  onChange={(v) => updateItem(i, { ...item, resolution: v || null })}
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontStyle: "italic",
                    fontSize: 15,
                    color: "var(--color-text-secondary)",
                    margin: 0,
                  }}
                />
              )}

              {/* Dismiss */}
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
          );
        })}

        {/* Add item */}
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
          + Add item
        </button>
      </div>
    </section>
  );
}
