/**
 * WatchList — Color-accented section cards for Risks, Wins, Unknowns.
 * Each section gets a colored left accent bar + subtle tinted background.
 * Cream paper background (no linen full-bleed).
 *
 * I261: Optional onUpdateField prop enables click-to-edit on risk/win/unknown text.
 * I550: Per-item dismiss (x) and feedback (thumbs up/down) controls.
 */
import { useState } from "react";
import { X } from "lucide-react";
import type { EntityIntelligence } from "@/types";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { EditableText } from "@/components/ui/EditableText";
import { IntelligenceFeedback } from "@/components/ui/IntelligenceFeedback";

interface WatchListProps {
  intelligence: EntityIntelligence | null;
  sectionId?: string;
  chapterTitle?: string;
  /** Slot for entity-specific bottom content (e.g., programs, milestones). */
  bottomSection?: React.ReactNode;
  /** When provided, items become editable. Called with (fieldPath, newValue). */
  onUpdateField?: (fieldPath: string, value: string) => void;
  /** Per-item feedback value getter. Field path like "risks[0].text". */
  getItemFeedback?: (fieldPath: string) => "positive" | "negative" | null;
  /** Per-item feedback submit. */
  onItemFeedback?: (fieldPath: string, type: "positive" | "negative") => void;
}

/* ── section config ── */

const sectionConfig = {
  risk: {
    label: "Risks",
    borderColor: "var(--color-spice-terracotta)",
    bgColor: "var(--color-spice-terracotta-8, rgba(194, 97, 72, 0.06))",
    labelColor: "var(--color-spice-terracotta)",
  },
  win: {
    label: "Wins",
    borderColor: "var(--color-garden-sage)",
    bgColor: "var(--color-garden-sage-8, rgba(128, 147, 115, 0.06))",
    labelColor: "var(--color-garden-sage)",
  },
  unknown: {
    label: "Unknowns",
    borderColor: "var(--color-garden-larkspur)",
    bgColor: "var(--color-garden-larkspur-8, rgba(110, 137, 168, 0.06))",
    labelColor: "var(--color-garden-larkspur)",
  },
} as const;

/* ── sub-components ── */

function SectionCard({
  type,
  children,
}: {
  type: "risk" | "win" | "unknown";
  children: React.ReactNode;
}) {
  const cfg = sectionConfig[type];
  return (
    <div
      style={{
        borderLeft: `3px solid ${cfg.borderColor}`,
        background: cfg.bgColor,
        borderRadius: "0 6px 6px 0",
        padding: "24px 28px",
        marginBottom: 32,
      }}
    >
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 11,
          fontWeight: 500,
          textTransform: "uppercase",
          letterSpacing: "0.1em",
          color: cfg.labelColor,
          marginBottom: 16,
        }}
      >
        {cfg.label}
      </div>
      {children}
    </div>
  );
}

interface WatchItemProps {
  text: string;
  isLast: boolean;
  onTextChange?: (value: string) => void;
  onDismiss?: () => void;
  badge?: { text: string; color: string } | null;
  feedbackValue?: "positive" | "negative" | null;
  onFeedback?: (type: "positive" | "negative") => void;
}

function WatchItem({ text, isLast, onTextChange, onDismiss, badge, feedbackValue, onFeedback }: WatchItemProps) {
  const hasActions = !!onDismiss || !!onFeedback;
  return (
    <div
      className="watch-item-row"
      style={{
        padding: "10px 0",
        borderBottom: isLast ? "none" : "1px solid var(--color-rule-light)",
        display: "flex",
        alignItems: "flex-start",
        gap: 10,
      }}
    >
      <div style={{ flex: 1, minWidth: 0 }}>
        {onTextChange ? (
          <EditableText
            value={text}
            onChange={onTextChange}
            as="p"
            multiline
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 14,
              lineHeight: 1.6,
              color: "var(--color-text-primary)",
              margin: 0,
            }}
          />
        ) : (
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
        )}
      </div>
      {badge && (
        <span
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 9,
            fontWeight: 600,
            textTransform: "uppercase",
            letterSpacing: "0.08em",
            padding: "2px 7px",
            borderRadius: 3,
            whiteSpace: "nowrap",
            flexShrink: 0,
            marginTop: 2,
            background: badge.color === "terracotta"
              ? "var(--color-spice-terracotta-8, rgba(194, 97, 72, 0.12))"
              : badge.color === "sage"
              ? "var(--color-garden-sage-8, rgba(128, 147, 115, 0.12))"
              : "var(--color-desk-charcoal-4)",
            color: badge.color === "terracotta"
              ? "var(--color-spice-terracotta)"
              : badge.color === "sage"
              ? "var(--color-garden-sage)"
              : "var(--color-text-secondary)",
          }}
        >
          {badge.text}
        </span>
      )}
      {hasActions && (
        <div
          className="watch-item-actions"
          style={{
            display: "flex",
            alignItems: "center",
            gap: 2,
            flexShrink: 0,
            marginTop: 2,
            opacity: 0,
            transition: "opacity 0.15s ease",
          }}
        >
          {onFeedback && (
            <IntelligenceFeedback
              value={feedbackValue ?? null}
              onFeedback={onFeedback}
            />
          )}
          {onDismiss && (
            <button
              type="button"
              onClick={onDismiss}
              title="Dismiss"
              style={{
                display: "inline-flex",
                alignItems: "center",
                justifyContent: "center",
                width: 22,
                height: 22,
                padding: 0,
                border: "none",
                borderRadius: 2,
                background: "transparent",
                color: "var(--color-text-tertiary)",
                cursor: "pointer",
              }}
            >
              <X size={13} />
            </button>
          )}
        </div>
      )}
    </div>
  );
}

function urgencyBadge(urgency: string | undefined): { text: string; color: string } | null {
  if (!urgency) return null;
  return { text: urgency, color: "terracotta" };
}

/* ── main component ── */

export function WatchList({
  intelligence,
  sectionId = "watch-list",
  chapterTitle = "Watch List",
  bottomSection,
  onUpdateField,
  getItemFeedback,
  onItemFeedback,
}: WatchListProps) {
  const [expandedRisks, setExpandedRisks] = useState(false);
  const [expandedWins, setExpandedWins] = useState(false);

  const risks = intelligence?.risks ?? [];
  const wins = intelligence?.recentWins ?? [];
  const unknowns = intelligence?.currentState?.unknowns ?? [];

  const hasWatchItems = risks.length > 0 || wins.length > 0 || unknowns.length > 0;
  const hasContent = hasWatchItems || !!bottomSection;

  if (!hasContent) {
    return null;
  }

  const RISK_LIMIT = 5;
  const WIN_LIMIT = 3;

  const visibleRisks = expandedRisks ? risks : risks.slice(0, RISK_LIMIT);
  const hasMoreRisks = risks.length > RISK_LIMIT && !expandedRisks;

  const visibleWins = expandedWins ? wins : wins.slice(0, WIN_LIMIT);
  const hasMoreWins = wins.length > WIN_LIMIT && !expandedWins;

  const showMoreStyle: React.CSSProperties = {
    fontFamily: "var(--font-mono)",
    fontSize: 11,
    color: "var(--color-text-tertiary)",
    background: "none",
    border: "none",
    cursor: "pointer",
    padding: "8px 0 0",
    textTransform: "uppercase",
    letterSpacing: "0.06em",
  };

  return (
    <section id={sectionId || undefined} style={{ scrollMarginTop: sectionId ? 60 : undefined }}>
      <ChapterHeading title={chapterTitle} />

      {visibleRisks.length > 0 && (
        <SectionCard type="risk">
          {visibleRisks.map((r, i) => (
            <WatchItem
              key={`risk-${i}`}
              text={r.text}
              isLast={i === visibleRisks.length - 1 && !hasMoreRisks}
              onTextChange={
                onUpdateField
                  ? (v) => onUpdateField(`risks[${i}].text`, v)
                  : undefined
              }
              onDismiss={
                onUpdateField
                  ? () => onUpdateField(`risks[${i}].text`, "")
                  : undefined
              }
              badge={urgencyBadge(r.urgency)}
              feedbackValue={getItemFeedback?.(`risks[${i}].text`)}
              onFeedback={
                onItemFeedback
                  ? (type) => onItemFeedback(`risks[${i}].text`, type)
                  : undefined
              }
            />
          ))}
          {hasMoreRisks && (
            <button onClick={() => setExpandedRisks(true)} style={showMoreStyle}>
              Show {risks.length - RISK_LIMIT} more
            </button>
          )}
        </SectionCard>
      )}

      {visibleWins.length > 0 && (
        <SectionCard type="win">
          {visibleWins.map((w, i) => (
            <WatchItem
              key={`win-${i}`}
              text={w.text}
              isLast={i === visibleWins.length - 1 && !hasMoreWins}
              onTextChange={
                onUpdateField
                  ? (v) => onUpdateField(`recentWins[${i}].text`, v)
                  : undefined
              }
              onDismiss={
                onUpdateField
                  ? () => onUpdateField(`recentWins[${i}].text`, "")
                  : undefined
              }
              badge={null}
              feedbackValue={getItemFeedback?.(`recentWins[${i}].text`)}
              onFeedback={
                onItemFeedback
                  ? (type) => onItemFeedback(`recentWins[${i}].text`, type)
                  : undefined
              }
            />
          ))}
          {hasMoreWins && (
            <button onClick={() => setExpandedWins(true)} style={showMoreStyle}>
              Show {wins.length - WIN_LIMIT} more
            </button>
          )}
        </SectionCard>
      )}

      {unknowns.length > 0 && (
        <SectionCard type="unknown">
          {unknowns.map((u, i) => (
            <WatchItem
              key={`unknown-${i}`}
              text={u}
              isLast={i === unknowns.length - 1}
              onTextChange={
                onUpdateField
                  ? (v) => onUpdateField(`currentState.unknowns[${i}]`, v)
                  : undefined
              }
              onDismiss={
                onUpdateField
                  ? () => onUpdateField(`currentState.unknowns[${i}]`, "")
                  : undefined
              }
              feedbackValue={getItemFeedback?.(`currentState.unknowns[${i}]`)}
              onFeedback={
                onItemFeedback
                  ? (type) => onItemFeedback(`currentState.unknowns[${i}]`, type)
                  : undefined
              }
            />
          ))}
        </SectionCard>
      )}

      {/* Entity-specific bottom section */}
      {bottomSection}

      <style>{`
        .watch-item-row:hover .watch-item-actions {
          opacity: 1 !important;
        }
        .watch-item-actions:focus-within {
          opacity: 1 !important;
        }
      `}</style>
    </section>
  );
}
