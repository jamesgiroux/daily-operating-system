/**
 * WatchList — Full-bleed linen band.
 * Risks, Wins, Unknowns (from intelligence) + optional bottom section slot.
 * Generalized: programs extracted to WatchListPrograms (account-specific),
 * passed as `bottomSection` ReactNode.
 */
import { useState } from "react";
import type { EntityIntelligence } from "@/types";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";

interface WatchListProps {
  intelligence: EntityIntelligence | null;
  sectionId?: string;
  chapterTitle?: string;
  emptyMessage?: string;
  /** Slot for entity-specific bottom content (e.g., programs, milestones). */
  bottomSection?: React.ReactNode;
}

/* ── type-specific colors ── */

const sectionColors = {
  risk: "var(--color-spice-terracotta)",
  win: "var(--color-garden-sage)",
  unknown: "var(--color-garden-larkspur)",
} as const;

const dotColors = {
  risk: "var(--color-spice-terracotta)",
  win: "var(--color-garden-sage)",
  unknown: "var(--color-garden-larkspur)",
} as const;

/* ── sub-components ── */

function SectionTitle({ label, color }: { label: string; color: string }) {
  return (
    <div
      style={{
        fontFamily: "var(--font-mono)",
        fontSize: 11,
        fontWeight: 500,
        textTransform: "uppercase",
        letterSpacing: "0.1em",
        color,
        marginBottom: 20,
      }}
    >
      {label}
    </div>
  );
}

interface WatchItemRowProps {
  type: "risk" | "win" | "unknown";
  text: string;
  source?: string;
  isCallout?: boolean;
  isLast: boolean;
}

function WatchItemRow({ type, text, source, isCallout, isLast }: WatchItemRowProps) {
  const dot = (
    <span
      style={{
        width: 8,
        height: 8,
        borderRadius: "50%",
        background: dotColors[type],
        flexShrink: 0,
        marginTop: 6,
        ...(isCallout && type === "risk"
          ? { boxShadow: "0 0 0 3px rgba(196,101,74,0.2)" }
          : {}),
        ...(isCallout && type === "win"
          ? { boxShadow: "0 0 0 3px rgba(126,170,123,0.2)" }
          : {}),
      }}
    />
  );

  const content = (
    <div style={{ flex: 1, minWidth: 0 }}>
      {source && (
        <div
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 10,
            fontWeight: 500,
            textTransform: "uppercase",
            letterSpacing: "0.06em",
            color: "var(--color-text-tertiary)",
            marginBottom: 4,
          }}
        >
          {source}
        </div>
      )}
      <p
        style={{
          fontFamily: "var(--font-sans)",
          fontSize: 14,
          lineHeight: 1.6,
          color: "var(--color-text-primary)",
          margin: 0,
        }}
      >
        {text}
      </p>
    </div>
  );

  if (isCallout) {
    const borderColor =
      type === "win" ? "var(--color-garden-sage)" : "var(--color-spice-terracotta)";
    return (
      <div
        style={{
          background: "rgba(30,37,48,0.04)",
          borderLeft: `3px solid ${borderColor}`,
          borderRadius: "0 6px 6px 0",
          padding: "16px 20px",
          margin: "8px 0",
        }}
      >
        <div style={{ display: "flex", gap: 14, alignItems: "flex-start" }}>
          {dot}
          {content}
        </div>
      </div>
    );
  }

  return (
    <div
      style={{
        display: "flex",
        gap: 14,
        padding: "16px 0",
        borderBottom: isLast ? "none" : "1px solid rgba(30,37,48,0.06)",
        alignItems: "flex-start",
      }}
    >
      {dot}
      {content}
    </div>
  );
}

/* ── main component ── */

export function WatchList({
  intelligence,
  sectionId = "watch-list",
  chapterTitle = "Watch List",
  emptyMessage = "Build intelligence to surface risks, wins, and unknowns.",
  bottomSection,
}: WatchListProps) {
  const [expanded, setExpanded] = useState(false);

  const risks = intelligence?.risks ?? [];
  const wins = intelligence?.recentWins ?? [];
  const unknowns = intelligence?.currentState?.unknowns ?? [];

  const hasContent = risks.length > 0 || wins.length > 0 || unknowns.length > 0 || !!bottomSection;

  const totalWatchItems = risks.length + wins.length + unknowns.length;
  const collapsedLimit = 5;

  function sliceForCollapsed<T>(items: T[], alreadyShown: number): T[] {
    if (expanded) return items;
    const remaining = collapsedLimit - alreadyShown;
    if (remaining <= 0) return [];
    return items.slice(0, remaining);
  }

  const visibleRisks = sliceForCollapsed(risks, 0);
  const visibleWins = sliceForCollapsed(wins, visibleRisks.length);
  const visibleUnknowns = sliceForCollapsed(unknowns, visibleRisks.length + visibleWins.length);
  const hasMore = !expanded && totalWatchItems > collapsedLimit;

  return (
    <section id={sectionId} style={{ scrollMarginTop: 60, paddingTop: 80 }}>
      <div
        style={{
          marginLeft: "calc(-50vw + 50%)",
          marginRight: "calc(-50vw + 50%)",
          paddingLeft: "calc(50vw - 50%)",
          paddingRight: "calc(50vw - 50%)",
          background: "var(--color-paper-linen)",
        }}
      >
        <div style={{ maxWidth: 820, margin: "0 auto", padding: "80px 48px" }}>
          <ChapterHeading title={chapterTitle} />

          {hasContent ? (
            <>
              {visibleRisks.length > 0 && (
                <div style={{ marginBottom: 48 }}>
                  <SectionTitle label="Risks" color={sectionColors.risk} />
                  {visibleRisks.map((r, i) => (
                    <WatchItemRow
                      key={`risk-${i}`}
                      type="risk"
                      text={r.text}
                      source={r.source}
                      isCallout={i === 0}
                      isLast={i === visibleRisks.length - 1}
                    />
                  ))}
                </div>
              )}

              {visibleWins.length > 0 && (
                <div style={{ marginBottom: 48 }}>
                  <SectionTitle label="Wins" color={sectionColors.win} />
                  {visibleWins.map((w, i) => (
                    <WatchItemRow
                      key={`win-${i}`}
                      type="win"
                      text={w.text}
                      source={w.source}
                      isCallout={i === 0}
                      isLast={i === visibleWins.length - 1}
                    />
                  ))}
                </div>
              )}

              {visibleUnknowns.length > 0 && (
                <div style={{ marginBottom: 48 }}>
                  <SectionTitle label="Unknowns" color={sectionColors.unknown} />
                  {visibleUnknowns.map((u, i) => (
                    <WatchItemRow
                      key={`unknown-${i}`}
                      type="unknown"
                      text={u}
                      isLast={i === visibleUnknowns.length - 1}
                    />
                  ))}
                </div>
              )}

              {hasMore && (
                <button
                  onClick={() => setExpanded(true)}
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 11,
                    color: "var(--color-text-tertiary)",
                    background: "none",
                    border: "none",
                    cursor: "pointer",
                    padding: "8px 0",
                    textTransform: "uppercase",
                    letterSpacing: "0.06em",
                  }}
                >
                  Show {totalWatchItems - collapsedLimit} more items
                </button>
              )}

              {/* Entity-specific bottom section */}
              {bottomSection}
            </>
          ) : (
            <p
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 14,
                color: "var(--color-text-tertiary)",
                fontStyle: "italic",
              }}
            >
              {emptyMessage}
            </p>
          )}
        </div>
      </div>
    </section>
  );
}
