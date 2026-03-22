/**
 * TheStorySlide — Slide 2: Strategic relationship summary.
 * Uses storyBullets as the narrative, displayed as
 * large editorial bullets.
 */
import { useState } from "react";
import { EditableText } from "@/components/ui/EditableText";
import type { EbrQbrContent } from "@/types/reports";

interface TheStorySlideProps {
  content: EbrQbrContent;
  onUpdate: (c: EbrQbrContent) => void;
}

export function TheStorySlide({ content, onUpdate }: TheStorySlideProps) {
  const [hoveredIndex, setHoveredIndex] = useState<number | null>(null);

  const bullets = content.storyBullets.length > 0 ? content.storyBullets : [""];

  function updateBullets(updated: string[]) {
    onUpdate({ ...content, storyBullets: updated });
  }

  return (
    <section
      id="the-story"
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
          marginBottom: 48,
        }}
      >
        The Story
      </div>

      {/* Editorial bullet list — intentionally spacious */}
      <div style={{ maxWidth: 760 }}>
        {bullets.map((bullet, i) => (
          <div
            key={i}
            onMouseEnter={() => setHoveredIndex(i)}
            onMouseLeave={() => setHoveredIndex(null)}
            style={{
              display: "flex",
              alignItems: "flex-start",
              gap: 20,
              marginBottom: 32,
            }}
          >
            {/* Large editorial dot */}
            <span
              style={{
                fontFamily: "var(--font-serif)",
                fontSize: 28,
                lineHeight: 1.3,
                color: "var(--color-garden-larkspur)",
                flexShrink: 0,
                userSelect: "none",
              }}
            >
              ·
            </span>

            <EditableText
              value={bullet}
              onChange={(v) => {
                const updated = [...bullets];
                updated[i] = v;
                updateBullets(updated.filter(Boolean));
              }}
              style={{
                fontFamily: "var(--font-serif)",
                fontSize: 20,
                lineHeight: 1.5,
                color: "var(--color-text-primary)",
                flex: 1,
              }}
            />

            {/* Dismiss button — only when > 1 bullet */}
            {bullets.length > 1 && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  const updated = bullets.filter((_: string, j: number) => j !== i);
                  updateBullets(updated);
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
                  marginTop: 2,
                }}
                aria-label="Remove"
              >
                ✕
              </button>
            )}
          </div>
        ))}

        {/* Add point — only when < 5 bullets */}
        {bullets.length < 5 && (
          <button
            onClick={() => updateBullets([...bullets, ""])}
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              opacity: 0.5,
              background: "none",
              border: "none",
              cursor: "pointer",
              padding: "4px 0",
              color: "var(--color-text-secondary)",
              marginLeft: 48,
            }}
            onMouseEnter={(e) => ((e.currentTarget as HTMLButtonElement).style.opacity = "0.8")}
            onMouseLeave={(e) => ((e.currentTarget as HTMLButtonElement).style.opacity = "0.5")}
          >
            + Add point
          </button>
        )}
      </div>
    </section>
  );
}
