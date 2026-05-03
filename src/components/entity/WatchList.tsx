/**
 * WatchList — Color-accented section cards for Risks, Wins, Unknowns.
 * Each section gets a colored left accent bar + subtle tinted background.
 *
 * Optional onUpdateField prop enables click-to-edit on risk/win/unknown text.
 * Per-item dismiss (x) and feedback (thumbs up/down) on single row hover.
 */
import { useState } from "react";
import { X } from "lucide-react";
import type { EntityIntelligence } from "@/types";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { EditableText } from "@/components/ui/EditableText";
import { IntelligenceFeedback } from "@/components/ui/IntelligenceFeedback";
import s from "./WatchList.module.css";

interface WatchListProps {
  intelligence: EntityIntelligence | null;
  sectionId?: string;
  chapterTitle?: string;
  bottomSection?: React.ReactNode;
  onUpdateField?: (fieldPath: string, value: string) => void;
  getItemFeedback?: (fieldPath: string) => "positive" | "negative" | null;
  onItemFeedback?: (fieldPath: string, type: "positive" | "negative") => void;
}

/* ── sub-components ── */

function SectionCard({ type, children }: { type: "risk" | "win" | "unknown"; children: React.ReactNode }) {
  const labels = { risk: "Risks", win: "Wins", unknown: "Unknowns" } as const;
  return (
    <div className={s.sectionCard} data-type={type}>
      <div className={s.sectionLabel}>{labels[type]}</div>
      {children}
    </div>
  );
}

interface WatchItemProps {
  text: string;
  onTextChange?: (value: string) => void;
  onDismiss?: () => void;
  badge?: { text: string; color: string } | null;
  feedbackValue?: "positive" | "negative" | null;
  onFeedback?: (type: "positive" | "negative") => void;
}

function WatchItem({ text, onTextChange, onDismiss, badge, feedbackValue, onFeedback }: WatchItemProps) {
  const hasActions = !!onDismiss || !!onFeedback;
  const badgeClass = badge
    ? badge.color === "terracotta" ? s.badgeTerracotta
    : badge.color === "sage" ? s.badgeSage
    : s.badgeNeutral
    : "";

  return (
    <div className={s.itemRow}>
      <div className={s.itemText}>
        {onTextChange ? (
          <EditableText
            value={text}
            onChange={onTextChange}
            as="p"
            multiline
            className={s.itemTextContent}
          />
        ) : (
          <p className={s.itemTextContent}>{text}</p>
        )}
      </div>
      {badge && (
        <span className={`${s.badge} ${badgeClass}`}>{badge.text}</span>
      )}
      {hasActions && (
        <div className={s.itemActions}>
          {onFeedback && (
            <IntelligenceFeedback
              value={feedbackValue ?? null}
              onFeedback={onFeedback}
            />
          )}
          {onDismiss && (
            <button type="button" onClick={onDismiss} title="Remove" className={s.dismissButton}>
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

  const risks = (intelligence?.risks ?? []).filter((r) => r.text?.trim());
  const wins = (intelligence?.recentWins ?? []).filter((w) => w.text?.trim());
  const unknowns = (intelligence?.currentState?.unknowns ?? []).filter((u) => u?.trim());

  const hasWatchItems = risks.length > 0 || wins.length > 0 || unknowns.length > 0;
  const hasContent = hasWatchItems || !!bottomSection;

  if (!hasContent) return null;

  const RISK_LIMIT = 5;
  const WIN_LIMIT = 3;

  const visibleRisks = expandedRisks ? risks : risks.slice(0, RISK_LIMIT);
  const hasMoreRisks = risks.length > RISK_LIMIT && !expandedRisks;

  const visibleWins = expandedWins ? wins : wins.slice(0, WIN_LIMIT);
  const hasMoreWins = wins.length > WIN_LIMIT && !expandedWins;

  return (
    <section id={sectionId || undefined} className={s.section}>
      <ChapterHeading title={chapterTitle} />

      {visibleRisks.length > 0 && (
        <SectionCard type="risk">
          {visibleRisks.map((r, i) => (
            <WatchItem
              key={`risk-${i}`}
              text={r.text}
              onTextChange={onUpdateField ? (v) => onUpdateField(`risks[${i}].text`, v) : undefined}
              onDismiss={onUpdateField ? () => onUpdateField(`risks[${i}].text`, "") : undefined}
              badge={urgencyBadge(r.urgency)}
              feedbackValue={getItemFeedback?.(`risks[${i}].text`)}
              onFeedback={onItemFeedback ? (type) => onItemFeedback(`risks[${i}].text`, type) : undefined}
            />
          ))}
          {hasMoreRisks && (
            <button onClick={() => setExpandedRisks(true)} className={s.expandToggle}>
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
              onTextChange={onUpdateField ? (v) => onUpdateField(`recentWins[${i}].text`, v) : undefined}
              onDismiss={onUpdateField ? () => onUpdateField(`recentWins[${i}].text`, "") : undefined}
              badge={null}
              feedbackValue={getItemFeedback?.(`recentWins[${i}].text`)}
              onFeedback={onItemFeedback ? (type) => onItemFeedback(`recentWins[${i}].text`, type) : undefined}
            />
          ))}
          {hasMoreWins && (
            <button onClick={() => setExpandedWins(true)} className={s.expandToggle}>
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
              onTextChange={onUpdateField ? (v) => onUpdateField(`currentState.unknowns[${i}]`, v) : undefined}
              onDismiss={onUpdateField ? () => onUpdateField(`currentState.unknowns[${i}]`, "") : undefined}
              feedbackValue={getItemFeedback?.(`currentState.unknowns[${i}]`)}
              onFeedback={onItemFeedback ? (type) => onItemFeedback(`currentState.unknowns[${i}]`, type) : undefined}
            />
          ))}
        </SectionCard>
      )}

      {bottomSection}
    </section>
  );
}
