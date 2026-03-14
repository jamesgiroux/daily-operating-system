/**
 * AskSlide — Leadership asks.
 * Same editorial pattern as the risk briefing's TheAskSlide.
 */
import { useState } from "react";
import { EditableText } from "@/components/ui/EditableText";
import type { BookOfBusinessContent } from "@/types/reports";

interface AskSlideProps {
  content: BookOfBusinessContent;
  onUpdate: (content: BookOfBusinessContent) => void;
}

export function AskSlide({ content, onUpdate }: AskSlideProps) {
  const [hoveredAsk, setHoveredAsk] = useState<number | null>(null);

  if (content.leadershipAsks.length === 0) return null;

  return (
    <section
      id="the-ask"
      style={{
        scrollMarginTop: 60,
        minHeight: "100vh",
        display: "flex",
        flexDirection: "column",
        justifyContent: "center",
        padding: "120px 120px 80px",
        scrollSnapAlign: "start",
      }}
    >
      {/* Overline */}
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 12,
          fontWeight: 600,
          textTransform: "uppercase",
          letterSpacing: "0.12em",
          color: "var(--color-spice-turmeric)",
          marginBottom: 24,
        }}
      >
        The Ask
      </div>

      <div style={{ maxWidth: 800 }}>
        {content.leadershipAsks.map((ask, ai) => (
          <div
            key={ai}
            onMouseEnter={() => setHoveredAsk(ai)}
            onMouseLeave={() => setHoveredAsk(null)}
            style={{
              display: "flex",
              alignItems: "baseline",
              gap: 16,
              padding: "16px 0",
              borderBottom: "1px solid var(--color-rule-light)",
            }}
          >
            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 20,
                fontWeight: 600,
                color: "var(--color-spice-turmeric)",
                minWidth: 24,
                flexShrink: 0,
              }}
            >
              {ai + 1}
            </span>
            <div style={{ flex: 1 }}>
              <EditableText
                value={ask.ask}
                onChange={(v) => {
                  const next = [...content.leadershipAsks];
                  next[ai] = { ...next[ai], ask: v };
                  onUpdate({ ...content, leadershipAsks: next });
                }}
                multiline={false}
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 17,
                  color: "var(--color-text-primary)",
                }}
              />
              <EditableText
                as="div"
                value={ask.context}
                onChange={(v) => {
                  const next = [...content.leadershipAsks];
                  next[ai] = { ...next[ai], context: v };
                  onUpdate({ ...content, leadershipAsks: next });
                }}
                multiline={false}
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 14,
                  color: "var(--color-text-secondary)",
                  marginTop: 4,
                }}
              />
              {ask.impactedAccounts.length > 0 && (
                <div
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 11,
                    color: "var(--color-text-tertiary)",
                    marginTop: 6,
                  }}
                >
                  {ask.impactedAccounts.join(" \u00b7 ")}
                </div>
              )}
            </div>
            {ask.status && (
              <span
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 10,
                  fontWeight: 600,
                  textTransform: "uppercase",
                  letterSpacing: "0.06em",
                  color: "var(--color-text-tertiary)",
                  background: "var(--color-rule-light)",
                  padding: "2px 8px",
                  borderRadius: 3,
                  flexShrink: 0,
                }}
              >
                {ask.status}
              </span>
            )}
            {content.leadershipAsks.length > 1 && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  onUpdate({
                    ...content,
                    leadershipAsks: content.leadershipAsks.filter((_, j) => j !== ai),
                  });
                }}
                style={{
                  opacity: hoveredAsk === ai ? 0.6 : 0,
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
    </section>
  );
}
