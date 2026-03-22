/**
 * TheWorkSlide — Wins + What You Did this week.
 * Slide 3: editable wins list + multiline whatYouDid summary.
 */
import { useState } from "react";
import { EditableText } from "@/components/ui/EditableText";
import type { WeeklyImpactContent, WeeklyImpactItem } from "@/types/reports";

interface TheWorkSlideProps {
  content: WeeklyImpactContent;
  onUpdate: (updated: WeeklyImpactContent) => void;
}

export function TheWorkSlide({ content, onUpdate }: TheWorkSlideProps) {
  const [hoveredWin, setHoveredWin] = useState<number | null>(null);

  const wins = content.wins;

  const updateWin = (i: number, patch: Partial<WeeklyImpactItem>) => {
    const updated = [...wins];
    updated[i] = { ...updated[i], ...patch };
    onUpdate({ ...content, wins: updated });
  };

  const removeWin = (i: number) => {
    onUpdate({ ...content, wins: wins.filter((_, j) => j !== i) });
  };

  const addWin = () => {
    onUpdate({ ...content, wins: [...wins, { text: "A win from this week", source: null }] });
  };

  return (
    <section
      id="the-work"
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
        The Work
      </div>

      {/* Wins section */}
      <div style={{ maxWidth: 800, marginBottom: 48 }}>
        <div
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            fontWeight: 600,
            textTransform: "uppercase",
            letterSpacing: "0.1em",
            color: "var(--color-text-secondary)",
            marginBottom: 16,
          }}
        >
          Wins
        </div>

        {wins.map((win, i) => (
          <div
            key={i}
            onMouseEnter={() => setHoveredWin(i)}
            onMouseLeave={() => setHoveredWin(null)}
            style={{
              display: "flex",
              gap: 12,
              alignItems: "flex-start",
              padding: "10px 0",
              borderBottom: "1px solid var(--color-rule-light)",
            }}
          >
            <div style={{ flex: 1 }}>
              <EditableText
                value={win.text}
                onChange={(v) => updateWin(i, { text: v })}
                multiline
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 16,
                  lineHeight: 1.5,
                  color: "var(--color-text-primary)",
                  display: "block",
                }}
              />
              {win.source && (
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
                  {win.source}
                </span>
              )}
            </div>

            {wins.length > 1 && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  removeWin(i);
                }}
                style={{
                  opacity: hoveredWin === i ? 0.6 : 0,
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

        <button
          onClick={addWin}
          style={{
            marginTop: 12,
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
          }}
        >
          + Add
        </button>
      </div>

      {/* Thin rule divider */}
      <div
        style={{
          borderTop: "1px solid var(--color-rule-light)",
          marginBottom: 32,
        }}
      />

      {/* What You Did section */}
      <div style={{ maxWidth: 800 }}>
        <div
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            fontWeight: 600,
            textTransform: "uppercase",
            letterSpacing: "0.1em",
            color: "var(--color-text-secondary)",
            marginBottom: 16,
          }}
        >
          What Happened
        </div>

        <EditableText
          as="p"
          value={content.whatYouDid}
          onChange={(v) => onUpdate({ ...content, whatYouDid: v })}
          multiline
          placeholder="A two-sentence summary of what happened this week..."
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 20,
            fontWeight: 400,
            lineHeight: 1.6,
            color: "var(--color-text-primary)",
            margin: 0,
          }}
        />
      </div>
    </section>
  );
}
