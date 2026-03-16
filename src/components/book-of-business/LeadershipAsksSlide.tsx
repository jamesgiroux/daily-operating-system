/**
 * LeadershipAsksSlide — Decisions & leadership asks.
 * Slides 10+14 combined: what I need + decisions.
 */
import { useState } from "react";
import { EditableText } from "@/components/ui/EditableText";
import type { BookOfBusinessContent } from "@/types/reports";

interface LeadershipAsksSlideProps {
  content: BookOfBusinessContent;
  onUpdate: (content: BookOfBusinessContent) => void;
}

export function LeadershipAsksSlide({ content, onUpdate }: LeadershipAsksSlideProps) {
  const [hoveredAsk, setHoveredAsk] = useState<number | null>(null);

  if (content.leadershipAsks.length === 0) return null;

  const addAsk = () => {
    onUpdate({
      ...content,
      leadershipAsks: [
        ...content.leadershipAsks,
        { supportNeeded: "New ask", whyItMatters: "", impactedAccounts: [], dollarImpact: null, timing: "" },
      ],
    });
  };

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
        Decisions & Leadership Asks
      </div>

      <div style={{ maxWidth: 900 }}>
        {/* Table header */}
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 120px 100px 100px 32px", gap: 12, paddingBottom: 8, borderBottom: "2px solid var(--color-rule-heavy)" }}>
          {["What I Need", "Why It Matters", "Accounts", "$ Impact", "Timing", ""].map((h) => (
            <div key={h} style={{ fontFamily: "var(--font-mono)", fontSize: 10, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.08em", color: "var(--color-text-tertiary)" }}>
              {h}
            </div>
          ))}
        </div>

        {content.leadershipAsks.map((ask, ai) => (
          <div
            key={ai}
            onMouseEnter={() => setHoveredAsk(ai)}
            onMouseLeave={() => setHoveredAsk(null)}
            style={{
              display: "grid",
              gridTemplateColumns: "1fr 1fr 120px 100px 100px 32px",
              gap: 12,
              padding: "12px 0",
              borderBottom: "1px solid var(--color-rule-light)",
              alignItems: "baseline",
            }}
          >
            <EditableText
              value={ask.supportNeeded}
              onChange={(v) => {
                const next = [...content.leadershipAsks];
                next[ai] = { ...next[ai], supportNeeded: v };
                onUpdate({ ...content, leadershipAsks: next });
              }}
              multiline={false}
              style={{ fontFamily: "var(--font-sans)", fontSize: 14, fontWeight: 500, color: "var(--color-text-primary)" }}
            />
            <EditableText
              value={ask.whyItMatters}
              onChange={(v) => {
                const next = [...content.leadershipAsks];
                next[ai] = { ...next[ai], whyItMatters: v };
                onUpdate({ ...content, leadershipAsks: next });
              }}
              multiline={false}
              style={{ fontFamily: "var(--font-sans)", fontSize: 13, color: "var(--color-text-secondary)" }}
            />
            <div style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--color-text-tertiary)" }}>
              {ask.impactedAccounts.join(", ")}
            </div>
            <EditableText
              value={ask.dollarImpact ?? ""}
              onChange={(v) => {
                const next = [...content.leadershipAsks];
                next[ai] = { ...next[ai], dollarImpact: v || null };
                onUpdate({ ...content, leadershipAsks: next });
              }}
              multiline={false}
              placeholder="$—"
              style={{ fontFamily: "var(--font-mono)", fontSize: 13, color: "var(--color-text-primary)" }}
            />
            <EditableText
              value={ask.timing}
              onChange={(v) => {
                const next = [...content.leadershipAsks];
                next[ai] = { ...next[ai], timing: v };
                onUpdate({ ...content, leadershipAsks: next });
              }}
              multiline={false}
              placeholder="When"
              style={{ fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--color-text-secondary)" }}
            />
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

        <button
          onClick={addAsk}
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            fontWeight: 600,
            textTransform: "uppercase",
            letterSpacing: "0.06em",
            color: "var(--color-spice-turmeric)",
            background: "none",
            border: "none",
            cursor: "pointer",
            padding: "12px 0",
          }}
        >
          + Add Ask
        </button>
      </div>
    </section>
  );
}
