/**
 * StrategicLandscape — Competitive & Strategic chapter.
 * Surfaces strategicPriorities, competitiveContext, organizationalChanges,
 * and blockers from EntityIntelligence. Collapses entirely when all are empty.
 *
 * I550: Per-item inline editing, dismiss, and feedback controls.
 */
import { X } from "lucide-react";
import type { EntityIntelligence } from "@/types";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { EditableText } from "@/components/ui/EditableText";
import { IntelligenceFeedback } from "@/components/ui/IntelligenceFeedback";
import css from "./StrategicLandscape.module.css";

interface StrategicLandscapeProps {
  intelligence: EntityIntelligence;
  /** When provided, items become editable. Called with (fieldPath, newValue). */
  onUpdateField?: (fieldPath: string, value: string) => void;
  /** Per-item feedback value getter. */
  getItemFeedback?: (fieldPath: string) => "positive" | "negative" | null;
  /** Per-item feedback submit. */
  onItemFeedback?: (fieldPath: string, type: "positive" | "negative") => void;
}

function formatDate(dateStr?: string): string {
  if (!dateStr) return "";
  try {
    return new Date(dateStr).toLocaleDateString("en-US", {
      month: "short",
      day: "numeric",
      year: "numeric",
    });
  } catch {
    return dateStr;
  }
}

function getPriorityStatusColor(status?: string): string {
  switch (status?.toLowerCase().replace(/[_\s-]/g, "")) {
    case "active":
      return css.badgeSage;
    case "atrisk":
      return css.badgeTurmeric;
    case "completed":
      return css.badgeLarkspur;
    case "paused":
      return css.badgeNeutral;
    default:
      return css.badgeNeutral;
  }
}

function getPriorityStatusLabel(status?: string): string {
  switch (status?.toLowerCase().replace(/[_\s-]/g, "")) {
    case "active":
      return "Active";
    case "atrisk":
      return "At Risk";
    case "completed":
      return "Completed";
    case "paused":
      return "Paused";
    default:
      return status ?? "";
  }
}

function getThreatLevelColor(level?: string): string {
  switch (level?.toLowerCase().replace(/[_\s-]/g, "")) {
    case "displacement":
      return css.badgeTerracotta;
    case "evaluation":
      return css.badgeTurmeric;
    case "mentioned":
      return css.badgeSage;
    case "incumbent":
      return css.badgeNeutral;
    default:
      return css.badgeNeutral;
  }
}

function getThreatLevelLabel(level?: string): string {
  switch (level?.toLowerCase().replace(/[_\s-]/g, "")) {
    case "displacement":
      return "Displacement";
    case "evaluation":
      return "Evaluation";
    case "mentioned":
      return "Mentioned";
    case "incumbent":
      return "Incumbent";
    default:
      return level ?? "";
  }
}

function getChangeTypeLabel(changeType: string): string {
  switch (changeType.toLowerCase().replace(/[_\s-]/g, "")) {
    case "departure":
      return "Departure";
    case "hire":
      return "New Hire";
    case "promotion":
      return "Promotion";
    case "reorg":
      return "Reorg";
    case "rolechange":
      return "Role Change";
    default:
      return changeType;
  }
}

function getChangeTypeColor(changeType: string): string {
  switch (changeType.toLowerCase().replace(/[_\s-]/g, "")) {
    case "departure":
      return css.badgeTerracotta;
    case "hire":
      return css.badgeSage;
    case "promotion":
      return css.badgeLarkspur;
    case "reorg":
      return css.badgeTurmeric;
    case "rolechange":
      return css.badgeNeutral;
    default:
      return css.badgeNeutral;
  }
}

function getImpactColor(impact?: string): string {
  switch (impact?.toLowerCase()) {
    case "critical":
      return css.badgeTerracotta;
    case "high":
      return css.badgeTurmeric;
    case "moderate":
      return css.badgeSage;
    case "low":
      return css.badgeNeutral;
    default:
      return css.badgeNeutral;
  }
}

function getImpactLabel(impact?: string): string {
  if (!impact) return "";
  return impact.charAt(0).toUpperCase() + impact.slice(1);
}

export function StrategicLandscape({
  intelligence,
  onUpdateField,
  getItemFeedback,
  onItemFeedback,
}: StrategicLandscapeProps) {
  const priorities = intelligence.strategicPriorities ?? [];
  const competitors = intelligence.competitiveContext ?? [];
  const orgChanges = intelligence.organizationalChanges ?? [];
  const blockers = intelligence.blockers ?? [];

  const hasPriorities = priorities.length > 0;
  const hasCompetitors = competitors.length > 0;
  const hasOrgChanges = orgChanges.length > 0;
  const hasBlockers = blockers.length > 0;

  if (!hasPriorities && !hasCompetitors && !hasOrgChanges && !hasBlockers) return null;

  return (
    <section className={css.section}>
      <ChapterHeading title="Competitive & Strategic Landscape" />

      {/* Strategic Priorities */}
      {hasPriorities && (
        <div className={css.subsection}>
          <h3 className={css.subsectionLabel}>Strategic Priorities</h3>
          <div className={css.itemList}>
            {priorities.map((p, i) => (
              <div key={i} className={css.item}>
                <div className={css.itemHeader}>
                  {onUpdateField ? (
                    <EditableText
                      value={p.priority}
                      onChange={(v) => onUpdateField(`strategicPriorities[${i}].priority`, v)}
                      as="p"
                      multiline
                      className={css.itemText}
                      style={{
                        fontFamily: "var(--font-serif)",
                        fontSize: 16,
                        lineHeight: 1.5,
                        color: "var(--color-text-primary)",
                        margin: 0,
                        flex: 1,
                      }}
                    />
                  ) : (
                    <p className={css.itemText}>{p.priority}</p>
                  )}
                  {p.status && (
                    <span className={`${css.badge} ${getPriorityStatusColor(p.status)}`}>
                      {getPriorityStatusLabel(p.status)}
                    </span>
                  )}
                  {(onUpdateField || onItemFeedback) && (
                    <span className={css.itemActions}>
                      {onItemFeedback && (
                        <IntelligenceFeedback
                          value={getItemFeedback?.(`strategicPriorities[${i}].priority`) ?? null}
                          onFeedback={(type) => onItemFeedback(`strategicPriorities[${i}].priority`, type)}
                        />
                      )}
                      {onUpdateField && (
                        <button
                          type="button"
                          className={css.dismissButton}
                          onClick={() => onUpdateField(`strategicPriorities[${i}].priority`, "")}
                          title="Dismiss"
                        >
                          <X size={13} />
                        </button>
                      )}
                    </span>
                  )}
                </div>
                <div className={css.itemMeta}>
                  {p.owner && <span className={css.metaText}>{p.owner}</span>}
                  {p.timeline && <span className={css.metaText}>{p.timeline}</span>}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Competitive Context */}
      {hasCompetitors && (
        <div className={css.subsection}>
          <h3 className={css.subsectionLabel}>Competitive Context</h3>
          <div className={css.itemList}>
            {competitors.map((c, i) => (
              <div key={i} className={css.item}>
                <div className={css.itemHeader}>
                  <span className={css.competitorName}>{c.competitor}</span>
                  {c.threatLevel && (
                    <span className={`${css.badge} ${getThreatLevelColor(c.threatLevel)}`}>
                      {getThreatLevelLabel(c.threatLevel)}
                    </span>
                  )}
                  {(onUpdateField || onItemFeedback) && (
                    <span className={css.itemActions}>
                      {onItemFeedback && (
                        <IntelligenceFeedback
                          value={getItemFeedback?.(`competitiveContext[${i}].context`) ?? null}
                          onFeedback={(type) => onItemFeedback(`competitiveContext[${i}].context`, type)}
                        />
                      )}
                      {onUpdateField && (
                        <button
                          type="button"
                          className={css.dismissButton}
                          onClick={() => onUpdateField(`competitiveContext[${i}].context`, "")}
                          title="Dismiss"
                        >
                          <X size={13} />
                        </button>
                      )}
                    </span>
                  )}
                </div>
                {c.context && (
                  onUpdateField ? (
                    <EditableText
                      value={c.context}
                      onChange={(v) => onUpdateField(`competitiveContext[${i}].context`, v)}
                      as="p"
                      multiline
                      style={{
                        fontFamily: "var(--font-serif)",
                        fontSize: 15,
                        lineHeight: 1.55,
                        color: "var(--color-text-secondary)",
                        margin: "6px 0 0",
                      }}
                    />
                  ) : (
                    <p className={css.contextText}>{c.context}</p>
                  )
                )}
                {c.detectedAt && (
                  <span className={css.metaText}>{formatDate(c.detectedAt)}</span>
                )}
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Organizational Changes */}
      {hasOrgChanges && (
        <div className={css.subsection}>
          <h3 className={css.subsectionLabel}>Organizational Changes</h3>
          <div className={css.itemList}>
            {orgChanges.map((change, i) => (
              <div key={i} className={css.item}>
                <div className={css.itemHeader}>
                  <span className={`${css.badge} ${getChangeTypeColor(change.changeType)}`}>
                    {getChangeTypeLabel(change.changeType)}
                  </span>
                  <span className={css.personName}>{change.person}</span>
                  {onItemFeedback && (
                    <span className={css.itemActions}>
                      <IntelligenceFeedback
                        value={getItemFeedback?.(`organizationalChanges[${i}].person`) ?? null}
                        onFeedback={(type) => onItemFeedback(`organizationalChanges[${i}].person`, type)}
                      />
                    </span>
                  )}
                </div>
                {(change.from || change.to) && (
                  <p className={css.changeDetail}>
                    {change.from && <span>{change.from}</span>}
                    {change.from && change.to && <span className={css.changeArrow}>{" \u2192 "}</span>}
                    {change.to && <span>{change.to}</span>}
                  </p>
                )}
                {change.detectedAt && (
                  <span className={css.metaText}>{formatDate(change.detectedAt)}</span>
                )}
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Blockers & Risks */}
      {hasBlockers && (
        <div className={css.subsection}>
          <h3 className={css.subsectionLabelTerracotta}>Blockers & Risks</h3>
          <div className={css.itemList}>
            {blockers.map((b, i) => (
              <div key={i} className={css.blockerItem}>
                <div className={css.itemHeader}>
                  {onUpdateField ? (
                    <EditableText
                      value={b.description}
                      onChange={(v) => onUpdateField(`blockers[${i}].description`, v)}
                      as="p"
                      multiline
                      className={css.itemText}
                      style={{
                        fontFamily: "var(--font-serif)",
                        fontSize: 16,
                        lineHeight: 1.5,
                        color: "var(--color-text-primary)",
                        margin: 0,
                        flex: 1,
                      }}
                    />
                  ) : (
                    <p className={css.itemText}>{b.description}</p>
                  )}
                  {b.impact && (
                    <span className={`${css.badge} ${getImpactColor(b.impact)}`}>
                      {getImpactLabel(b.impact)}
                    </span>
                  )}
                  {(onUpdateField || onItemFeedback) && (
                    <span className={css.itemActions}>
                      {onItemFeedback && (
                        <IntelligenceFeedback
                          value={getItemFeedback?.(`blockers[${i}].description`) ?? null}
                          onFeedback={(type) => onItemFeedback(`blockers[${i}].description`, type)}
                        />
                      )}
                      {onUpdateField && (
                        <button
                          type="button"
                          className={css.dismissButton}
                          onClick={() => onUpdateField(`blockers[${i}].description`, "")}
                          title="Dismiss"
                        >
                          <X size={13} />
                        </button>
                      )}
                    </span>
                  )}
                </div>
                <div className={css.itemMeta}>
                  {b.owner && <span className={css.metaText}>{b.owner}</span>}
                  {b.since && <span className={css.metaText}>Since {formatDate(b.since)}</span>}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </section>
  );
}
