/**
 * PrioritiesMovedSlide — What actually moved forward this week.
 * Slide 2: priority alignment with what happened + source citation.
 */
import { useState } from "react";
import { EditableText } from "@/components/ui/EditableText";
import type { WeeklyImpactContent, WeeklyImpactMove } from "@/types/reports";

interface PrioritiesMovedSlideProps {
  content: WeeklyImpactContent;
  onUpdate: (updated: WeeklyImpactContent) => void;
}

export function PrioritiesMovedSlide({ content, onUpdate }: PrioritiesMovedSlideProps) {
  const [hoveredItem, setHoveredItem] = useState<number | null>(null);

  const moves = content.prioritiesMoved;

  const updateMove = (i: number, patch: Partial<WeeklyImpactMove>) => {
    const updated = [...moves];
    updated[i] = { ...updated[i], ...patch };
    onUpdate({ ...content, prioritiesMoved: updated });
  };

  const removeMove = (i: number) => {
    onUpdate({ ...content, prioritiesMoved: moves.filter((_, j) => j !== i) });
  };

  const addMove = () => {
    onUpdate({
      ...content,
      prioritiesMoved: [
        ...moves,
        { priorityText: "Priority", whatHappened: "What happened this week", source: "" },
      ],
    });
  };

  return (
    <section
      id="priorities"
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
        Priorities Moved
      </div>

      {moves.length === 0 ? (
        <div style={{ maxWidth: 600 }}>
          <p
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 20,
              fontStyle: "italic",
              color: "var(--color-text-tertiary)",
              margin: "0 0 12px",
            }}
          >
            No priority movement this week.
          </p>
          <p
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 12,
              color: "var(--color-text-tertiary)",
              letterSpacing: "0.04em",
            }}
          >
            Add priorities on /me to track alignment.
          </p>
        </div>
      ) : (
        <div style={{ maxWidth: 800 }}>
          {moves.map((move, i) => (
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
                {/* Priority label */}
                <div
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 11,
                    fontWeight: 600,
                    textTransform: "uppercase",
                    letterSpacing: "0.1em",
                    color: "var(--color-text-tertiary)",
                    marginBottom: 6,
                  }}
                >
                  {move.priorityText}
                </div>

                {/* What happened */}
                <EditableText
                  value={move.whatHappened}
                  onChange={(v) => updateMove(i, { whatHappened: v })}
                  multiline
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 17,
                    lineHeight: 1.5,
                    color: "var(--color-text-primary)",
                    display: "block",
                    marginBottom: 6,
                  }}
                />

                {/* Source citation */}
                {move.source && (
                  <div
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 11,
                      fontStyle: "italic",
                      color: "var(--color-text-tertiary)",
                    }}
                  >
                    {move.source}
                  </div>
                )}
              </div>

              {/* Dismiss — only when more than 1 */}
              {moves.length > 1 && (
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    removeMove(i);
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
        onClick={addMove}
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
