/**
 * WatchList — Chapter 4: Full-bleed linen band.
 * Risks, Wins, Unknowns (from intelligence) + Active Initiatives (programs).
 * Grouped by type with colored section titles and callout boxes.
 */
import { useState } from "react";
import type { EntityIntelligence, StrategicProgram } from "@/types";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";

interface WatchListProps {
  intelligence: EntityIntelligence | null;
  programs: StrategicProgram[];
  onProgramUpdate?: (index: number, updated: StrategicProgram) => void;
  onProgramDelete?: (index: number) => void;
  onAddProgram?: () => void;
}

/* ── type-specific colors ─────────────────────────────────────────────── */

const sectionColors = {
  risk: "var(--color-spice-terracotta)",
  win: "var(--color-garden-sage)",
  unknown: "var(--color-garden-larkspur)",
  initiative: "var(--color-spice-turmeric)",
} as const;

const dotColors = {
  risk: "var(--color-spice-terracotta)",
  win: "var(--color-garden-sage)",
  unknown: "var(--color-garden-larkspur)",
} as const;

/* ── status badge palette ─────────────────────────────────────────────── */

function statusBadgeStyle(status: string): React.CSSProperties {
  const lower = status.toLowerCase();
  if (lower === "active") {
    return { background: "rgba(126,170,123,0.14)", color: "var(--color-garden-rosemary)" };
  }
  if (lower === "planned" || lower === "planning") {
    return { background: "rgba(143,163,196,0.14)", color: "var(--color-garden-larkspur)" };
  }
  // On Hold / fallback
  return { background: "rgba(30,37,48,0.06)", color: "var(--color-text-tertiary)" };
}

/* ── sub-components ───────────────────────────────────────────────────── */

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
          ? { boxShadow: `0 0 0 3px rgba(126,170,123,0.2)` }
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

/* ── main component ───────────────────────────────────────────────────── */

export function WatchList({
  intelligence,
  programs,
  onProgramUpdate,
  onProgramDelete,
  onAddProgram,
}: WatchListProps) {
  const [expanded, setExpanded] = useState(false);

  const risks = intelligence?.risks ?? [];
  const wins = intelligence?.recentWins ?? [];
  const unknowns = intelligence?.currentState?.unknowns ?? [];

  const activePrograms = programs.filter((p) => p.status !== "Complete");

  const hasContent =
    risks.length > 0 || wins.length > 0 || unknowns.length > 0 || activePrograms.length > 0;

  // For the expand/collapse toggle we count all watch items (not programs)
  const totalWatchItems = risks.length + wins.length + unknowns.length;
  const collapsedLimit = 5;

  // Determine how many items to show per section when collapsed
  // We distribute the limit across sections in order: risks, wins, unknowns
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
    <section id="watch-list" style={{ scrollMarginTop: 60, paddingTop: 80 }}>
      {/* Full-bleed linen band */}
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
          <ChapterHeading number={4} title="Watch List" />

          {hasContent ? (
            <>
              {/* ── Risks ─────────────────────────────────────── */}
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

              {/* ── Wins ──────────────────────────────────────── */}
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

              {/* ── Unknowns ──────────────────────────────────── */}
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

              {/* expand toggle */}
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

              {/* ── Active Initiatives (programs) ─────────────── */}
              {activePrograms.length > 0 && (
                <div style={{ marginBottom: 0 }}>
                  <SectionTitle label="Active Initiatives" color={sectionColors.initiative} />
                  <div style={{ display: "flex", flexDirection: "column" }}>
                    {activePrograms.map((p) => {
                      const originalIndex = programs.indexOf(p);
                      return (
                        <div
                          key={originalIndex}
                          style={{
                            display: "flex",
                            flexDirection: "column",
                            padding: "12px 0",
                            borderBottom:
                              originalIndex === programs.indexOf(activePrograms[activePrograms.length - 1])
                                ? "none"
                                : "1px solid rgba(30,37,48,0.06)",
                          }}
                        >
                          <div
                            style={{
                              display: "flex",
                              alignItems: "baseline",
                              gap: 12,
                            }}
                          >
                            {onProgramUpdate ? (
                              <input
                                value={p.name}
                                onChange={(e) =>
                                  onProgramUpdate(originalIndex, { ...p, name: e.target.value })
                                }
                                placeholder="Initiative name"
                                style={{
                                  fontFamily: "var(--font-sans)",
                                  fontSize: 14,
                                  fontWeight: 500,
                                  color: "var(--color-text-primary)",
                                  flex: 1,
                                  background: "none",
                                  border: "none",
                                  borderBottom: "1px solid transparent",
                                  outline: "none",
                                  padding: 0,
                                }}
                                onFocus={(e) => {
                                  e.currentTarget.style.borderBottomColor = "var(--color-rule-light)";
                                }}
                                onBlur={(e) => {
                                  e.currentTarget.style.borderBottomColor = "transparent";
                                }}
                              />
                            ) : (
                              <span
                                style={{
                                  fontFamily: "var(--font-sans)",
                                  fontSize: 14,
                                  fontWeight: 500,
                                  color: "var(--color-text-primary)",
                                  flex: 1,
                                }}
                              >
                                {p.name || "Untitled"}
                              </span>
                            )}

                            {onProgramUpdate ? (
                              <select
                                value={p.status}
                                onChange={(e) =>
                                  onProgramUpdate(originalIndex, { ...p, status: e.target.value })
                                }
                                style={{
                                  fontFamily: "var(--font-mono)",
                                  fontSize: 9,
                                  fontWeight: 500,
                                  textTransform: "uppercase",
                                  letterSpacing: "0.06em",
                                  padding: "2px 7px",
                                  borderRadius: 3,
                                  border: "none",
                                  cursor: "pointer",
                                  ...statusBadgeStyle(p.status),
                                }}
                              >
                                <option value="Active">Active</option>
                                <option value="Planned">Planned</option>
                                <option value="Planning">Planning</option>
                                <option value="On Hold">On Hold</option>
                                <option value="Complete">Complete</option>
                              </select>
                            ) : (
                              <span
                                style={{
                                  fontFamily: "var(--font-mono)",
                                  fontSize: 9,
                                  fontWeight: 500,
                                  textTransform: "uppercase",
                                  letterSpacing: "0.06em",
                                  padding: "2px 7px",
                                  borderRadius: 3,
                                  ...statusBadgeStyle(p.status),
                                }}
                              >
                                {p.status}
                              </span>
                            )}

                            {onProgramDelete && (
                              <button
                                onClick={() => onProgramDelete(originalIndex)}
                                style={{
                                  background: "none",
                                  border: "none",
                                  cursor: "pointer",
                                  fontFamily: "var(--font-mono)",
                                  fontSize: 10,
                                  color: "var(--color-text-tertiary)",
                                  padding: 0,
                                }}
                              >
                                x
                              </button>
                            )}
                          </div>

                          {p.notes && (
                            <p
                              style={{
                                fontFamily: "var(--font-sans)",
                                fontSize: 13,
                                lineHeight: 1.5,
                                color: "var(--color-text-tertiary)",
                                margin: "4px 0 0",
                              }}
                            >
                              {p.notes}
                            </p>
                          )}
                        </div>
                      );
                    })}
                  </div>

                  {onAddProgram && (
                    <button
                      onClick={onAddProgram}
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 10,
                        color: "var(--color-text-tertiary)",
                        background: "none",
                        border: "none",
                        cursor: "pointer",
                        padding: "8px 0",
                        textTransform: "uppercase",
                        letterSpacing: "0.06em",
                      }}
                    >
                      + Add Initiative
                    </button>
                  )}
                </div>
              )}
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
              Build intelligence to surface risks, wins, and unknowns.
            </p>
          )}
        </div>
      </div>
    </section>
  );
}
