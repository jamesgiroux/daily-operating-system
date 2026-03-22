/**
 * WhereWeStandSlide — internal view of account health: working, struggling, expansion.
 * Slide 3: two-column list + expansion signal pills.
 */
import { useState } from "react";
import { EditableText } from "@/components/ui/EditableText";
import type { AccountHealthContent } from "./types";

interface WhereWeStandSlideProps {
  content: AccountHealthContent;
  onUpdate: (c: AccountHealthContent) => void;
}

export function WhereWeStandSlide({ content, onUpdate }: WhereWeStandSlideProps) {
  const [hoveredWorking, setHoveredWorking] = useState<number | null>(null);
  const [hoveredStruggling, setHoveredStruggling] = useState<number | null>(null);
  const [hoveredSignal, setHoveredSignal] = useState<number | null>(null);

  return (
    <section
      id="where-we-stand"
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
        Where We Stand
      </div>

      {/* Two-column layout */}
      <div
        style={{
          display: "flex",
          gap: 40,
          maxWidth: 900,
          marginBottom: content.expansionSignals.length > 0 ? 0 : 0,
        }}
      >
        {/* LEFT — What's Working */}
        <div style={{ flex: 1 }}>
          <div
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              fontWeight: 600,
              textTransform: "uppercase",
              letterSpacing: "0.1em",
              color: "var(--color-garden-sage)",
              marginBottom: 16,
            }}
          >
            What's Working
          </div>

          {content.whatIsWorking.map((item, i) => (
            <div
              key={i}
              onMouseEnter={() => setHoveredWorking(i)}
              onMouseLeave={() => setHoveredWorking(null)}
              style={{
                display: "flex",
                alignItems: "baseline",
                gap: 8,
                padding: "6px 0",
                borderBottom: "1px solid var(--color-rule-light)",
              }}
            >
              <span
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 12,
                  color: "var(--color-garden-sage)",
                  flexShrink: 0,
                }}
              >
                ·
              </span>
              <EditableText
                value={item}
                onChange={(v) => {
                  const updated = [...content.whatIsWorking];
                  updated[i] = v;
                  onUpdate({ ...content, whatIsWorking: updated });
                }}
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 15,
                  color: "var(--color-text-primary)",
                  flex: 1,
                  lineHeight: 1.5,
                }}
              />
              {content.whatIsWorking.length > 1 && (
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    onUpdate({
                      ...content,
                      whatIsWorking: content.whatIsWorking.filter((_, j) => j !== i),
                    });
                  }}
                  style={{
                    opacity: hoveredWorking === i ? 0.6 : 0,
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

          {/* Add item */}
          <button
            onClick={() =>
              onUpdate({ ...content, whatIsWorking: [...content.whatIsWorking, "New item"] })
            }
            style={{
              background: "none",
              border: "none",
              cursor: "pointer",
              padding: "8px 0",
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              color: "var(--color-text-tertiary)",
              letterSpacing: "0.06em",
              opacity: 0.5,
              display: "block",
              marginTop: 4,
            }}
            onMouseEnter={(e) => (e.currentTarget.style.opacity = "0.8")}
            onMouseLeave={(e) => (e.currentTarget.style.opacity = "0.5")}
          >
            + Add item
          </button>
        </div>

        {/* RIGHT — What's Struggling */}
        <div style={{ flex: 1 }}>
          <div
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              fontWeight: 600,
              textTransform: "uppercase",
              letterSpacing: "0.1em",
              color: "var(--color-spice-terracotta)",
              marginBottom: 16,
            }}
          >
            What's Struggling
          </div>

          {content.whatIsStruggling.map((item, i) => (
            <div
              key={i}
              onMouseEnter={() => setHoveredStruggling(i)}
              onMouseLeave={() => setHoveredStruggling(null)}
              style={{
                display: "flex",
                alignItems: "baseline",
                gap: 8,
                padding: "6px 0",
                borderBottom: "1px solid var(--color-rule-light)",
              }}
            >
              <span
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 12,
                  color: "var(--color-spice-terracotta)",
                  flexShrink: 0,
                }}
              >
                ·
              </span>
              <EditableText
                value={item}
                onChange={(v) => {
                  const updated = [...content.whatIsStruggling];
                  updated[i] = v;
                  onUpdate({ ...content, whatIsStruggling: updated });
                }}
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 15,
                  color: "var(--color-text-primary)",
                  flex: 1,
                  lineHeight: 1.5,
                }}
              />
              {content.whatIsStruggling.length > 1 && (
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    onUpdate({
                      ...content,
                      whatIsStruggling: content.whatIsStruggling.filter((_, j) => j !== i),
                    });
                  }}
                  style={{
                    opacity: hoveredStruggling === i ? 0.6 : 0,
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

          {/* Add item */}
          <button
            onClick={() =>
              onUpdate({
                ...content,
                whatIsStruggling: [...content.whatIsStruggling, "New item"],
              })
            }
            style={{
              background: "none",
              border: "none",
              cursor: "pointer",
              padding: "8px 0",
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              color: "var(--color-text-tertiary)",
              letterSpacing: "0.06em",
              opacity: 0.5,
              display: "block",
              marginTop: 4,
            }}
            onMouseEnter={(e) => (e.currentTarget.style.opacity = "0.8")}
            onMouseLeave={(e) => (e.currentTarget.style.opacity = "0.5")}
          >
            + Add item
          </button>
        </div>
      </div>

      {/* Expansion Indicators */}
      {content.expansionSignals.length > 0 && (
        <div style={{ maxWidth: 900, marginTop: 32 }}>
          <div
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              fontWeight: 600,
              textTransform: "uppercase",
              letterSpacing: "0.1em",
              color: "var(--color-spice-turmeric)",
              marginBottom: 12,
            }}
          >
            Expansion Indicators
          </div>
          <div style={{ display: "flex", flexWrap: "wrap", gap: 8 }}>
            {content.expansionSignals.map((signal, i) => (
              <div
                key={i}
                onMouseEnter={() => setHoveredSignal(i)}
                onMouseLeave={() => setHoveredSignal(null)}
                style={{
                  position: "relative",
                  display: "inline-flex",
                  alignItems: "center",
                  gap: 4,
                  border: "1px solid var(--color-spice-turmeric)",
                  borderRadius: 4,
                  padding: "4px 10px",
                }}
              >
                <EditableText
                  value={signal}
                  onChange={(v) => {
                    const updated = [...content.expansionSignals];
                    updated[i] = v;
                    onUpdate({ ...content, expansionSignals: updated });
                  }}
                  multiline={false}
                  style={{
                    fontFamily: "var(--font-serif)",
                    fontSize: 13,
                    color: "var(--color-text-primary)",
                  }}
                />
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    onUpdate({
                      ...content,
                      expansionSignals: content.expansionSignals.filter((_, j) => j !== i),
                    });
                  }}
                  style={{
                    opacity: hoveredSignal === i ? 0.6 : 0,
                    transition: "opacity 0.15s",
                    background: "none",
                    border: "none",
                    cursor: "pointer",
                    padding: "2px 4px",
                    fontSize: 12,
                    color: "var(--color-text-tertiary)",
                    flexShrink: 0,
                  }}
                  aria-label="Remove"
                >
                  ✕
                </button>
              </div>
            ))}
          </div>
        </div>
      )}
    </section>
  );
}
