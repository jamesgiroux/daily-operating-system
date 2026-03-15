/**
 * StrategicLandscape -- Competitive & Strategic chapter (Ledger style).
 *
 * Four subsections: Strategic Priorities (numbered list), Competitive Landscape
 * (threat matrix grid), Organizational Changes (compact timeline with icons),
 * and Blockers (terracotta accent blocks).
 *
 * Each subsection renders only when its data is non-empty.
 * Returns null when all four sources are empty.
 *
 * I550: Per-item inline editing, dismiss, and feedback controls.
 */
import { X } from "lucide-react";
import type { EntityIntelligence } from "@/types";
import { EditableText } from "@/components/ui/EditableText";
import { ProvenanceTag } from "@/components/ui/ProvenanceTag";
import { IntelligenceFeedback } from "@/components/ui/IntelligenceFeedback";
import css from "./StrategicLandscape.module.css";

interface StrategicLandscapeProps {
  intelligence: EntityIntelligence;
  onUpdateField?: (fieldPath: string, value: string) => void;
  getItemFeedback?: (fieldPath: string) => "positive" | "negative" | null;
  onItemFeedback?: (fieldPath: string, type: "positive" | "negative") => void;
}

/* -- Helpers -------------------------------------------------------------- */

function formatDate(dateStr?: string): string {
  if (!dateStr) return "";
  try {
    return new Date(dateStr).toLocaleDateString("en-US", {
      month: "short",
      day: "numeric",
    });
  } catch {
    return dateStr;
  }
}

function priorityStatusBadge(status?: string): { label: string; cls: string } {
  switch (status?.toLowerCase().replace(/[_\s-]/g, "")) {
    case "active":
      return { label: "Active", cls: css.badgeSage };
    case "atrisk":
      return { label: "At Risk", cls: css.badgeTerracotta };
    case "completed":
      return { label: "Completed", cls: css.badgeSage };
    case "paused":
      return { label: "Paused", cls: css.badgeLarkspur };
    default:
      return { label: status ?? "", cls: css.badgeNeutral };
  }
}

function threatBadge(level?: string): { label: string; cls: string } {
  switch (level?.toLowerCase().replace(/[_\s-]/g, "")) {
    case "displacement":
      return { label: "Displacement", cls: css.badgeTerracotta };
    case "evaluation":
      return { label: "Evaluation", cls: css.badgeTerracotta };
    case "mentioned":
      return { label: "Mentioned", cls: css.badgeNeutral };
    case "incumbent":
      return { label: "Incumbent", cls: css.badgeSage };
    default:
      return { label: level ?? "", cls: css.badgeNeutral };
  }
}

function normalizeChangeType(changeType: string): string {
  return changeType.toLowerCase().replace(/[_\s-]/g, "");
}

function changeIconClass(changeType: string): string {
  switch (normalizeChangeType(changeType)) {
    case "departure":
      return css.changeIconDeparture;
    case "hire":
      return css.changeIconHire;
    case "promotion":
    case "rolechange":
      return css.changeIconPromotion;
    case "reorg":
      return css.changeIconReorg;
    default:
      return css.changeIconDefault;
  }
}

function changeIconChar(changeType: string): string {
  switch (normalizeChangeType(changeType)) {
    case "departure":
      return "\u2197"; // ↗
    case "hire":
      return "+";
    case "promotion":
      return "\u2191"; // ↑
    case "rolechange":
      return "\u21c4"; // ⇄
    case "reorg":
      return "\u2725"; // ✥
    default:
      return "\u2022"; // bullet
  }
}

function changeTypeLabel(changeType: string): string {
  switch (normalizeChangeType(changeType)) {
    case "departure":
      return "Departure";
    case "hire":
      return "New Hire";
    case "promotion":
      return "Promotion";
    case "rolechange":
      return "Role Change";
    case "reorg":
      return "Reorganization";
    default:
      // Title-case the raw value
      return changeType.replace(/[_-]/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
  }
}

function impactBadge(impact?: string): { label: string; cls: string } {
  switch (impact?.toLowerCase()) {
    case "critical":
      return { label: "Critical", cls: css.badgeTerracotta };
    case "high":
      return { label: "High", cls: css.badgeTurmeric };
    case "moderate":
      return { label: "Moderate", cls: css.badgeSage };
    case "low":
      return { label: "Low", cls: css.badgeNeutral };
    default:
      return { label: impact ?? "", cls: css.badgeNeutral };
  }
}

/* -- Component ------------------------------------------------------------ */

export function StrategicLandscape({
  intelligence,
  onUpdateField,
  getItemFeedback,
  onItemFeedback,
}: StrategicLandscapeProps) {
  const priorities = (intelligence.strategicPriorities ?? []).filter((p) => p.priority?.trim());
  const competitors = (intelligence.competitiveContext ?? []).filter((c) => c.context?.trim() || c.competitor?.trim());
  const orgChanges = (intelligence.organizationalChanges ?? []).filter((o) => o.person?.trim());
  const blockers = (intelligence.blockers ?? []).filter((b) => b.description?.trim());

  const hasPriorities = priorities.length > 0;
  const hasCompetitors = competitors.length > 0;
  const hasOrgChanges = orgChanges.length > 0;
  const hasBlockers = blockers.length > 0;

  if (!hasPriorities && !hasCompetitors && !hasOrgChanges && !hasBlockers) return null;

  const showActions = !!(onUpdateField || onItemFeedback);

  return (
    <section className={css.section}>
      {/* ── Strategic Priorities ── */}
      {hasPriorities && (
        <div className={css.subsection}>
          <h3 className={css.subsectionLabel}>Strategic Priorities</h3>
          {priorities.map((p, i) => {
            const badge = p.status ? priorityStatusBadge(p.status) : null;
            const path = `strategicPriorities[${i}].priority`;
            return (
              <div key={i} className={css.priorityRow}>
                <div className={css.priorityNum}>{i + 1}</div>
                <div className={css.priorityBody}>
                  {onUpdateField ? (
                    <EditableText
                      value={p.priority}
                      onChange={(v) => onUpdateField(path, v)}
                      as="p"
                      multiline
                      className={css.priorityText}
                    />
                  ) : (
                    <p className={css.priorityText}>{p.priority}</p>
                  )}
                  <div className={css.priorityMeta}>
                    {badge && (
                      <span className={`${css.badge} ${badge.cls}`}>{badge.label}</span>
                    )}
                    {p.owner && <span>{p.owner}</span>}
                    {p.timeline && <span>{p.timeline}</span>}
                  </div>
                </div>
                {showActions && (
                  <span className={css.itemActions}>
                    {onItemFeedback && (
                      <IntelligenceFeedback
                        value={getItemFeedback?.(path) ?? null}
                        onFeedback={(type) => onItemFeedback(path, type)}
                      />
                    )}
                    {onUpdateField && (
                      <button
                        type="button"
                        className={css.dismissButton}
                        onClick={() => onUpdateField(path, "")}
                        title="Dismiss"
                      >
                        <X size={13} />
                      </button>
                    )}
                  </span>
                )}
              </div>
            );
          })}
        </div>
      )}

      {/* ── Competitive Landscape ── */}
      {hasCompetitors && (
        <div className={css.subsection}>
          <h3 className={css.subsectionLabel}>Competitive Landscape</h3>
          {competitors.map((c, i) => {
            const badge = c.threatLevel ? threatBadge(c.threatLevel) : null;
            const path = `competitiveContext[${i}].context`;
            return (
              <div key={i} className={css.competitorRow}>
                <div className={css.competitorBody}>
                  <div className={css.competitorHeader}>
                    <span className={css.competitorName}>{c.competitor}</span>
                    {badge && (
                      <span className={`${css.badge} ${badge.cls}`}>{badge.label}</span>
                    )}
                  </div>
                  {c.context && (
                    onUpdateField ? (
                      <EditableText
                        value={c.context}
                        onChange={(v) => onUpdateField(path, v)}
                        as="p"
                        multiline
                        className={css.threatContext}
                      />
                    ) : (
                      <p className={css.threatContext}>{c.context}</p>
                    )
                  )}
                  <ProvenanceTag itemSource={c.itemSource} discrepancy={c.discrepancy} />
                </div>
                {showActions && (
                  <span className={css.itemActions}>
                    {onItemFeedback && (
                      <IntelligenceFeedback
                        value={getItemFeedback?.(path) ?? null}
                        onFeedback={(type) => onItemFeedback(path, type)}
                      />
                    )}
                    {onUpdateField && (
                      <button
                        type="button"
                        className={css.dismissButton}
                        onClick={() => onUpdateField(path, "")}
                        title="Dismiss"
                      >
                        <X size={13} />
                      </button>
                    )}
                  </span>
                )}
              </div>
            );
          })}
        </div>
      )}

      {/* ── Organizational Changes ── */}
      {hasOrgChanges && (
        <div className={css.subsection}>
          <h3 className={css.subsectionLabel}>Organizational Changes</h3>
          {orgChanges.map((change, i) => {
            const path = `organizationalChanges[${i}].person`;
            const transition = [change.from, change.to].filter(Boolean);
            const hasTransition = transition.length > 0;
            return (
              <div key={i} className={css.changeRow}>
                <div className={changeIconClass(change.changeType)}>
                  {changeIconChar(change.changeType)}
                </div>
                <div className={css.changeBody}>
                  <div className={css.changeHeader}>
                    <span className={css.changeName}>{change.person}</span>
                    <span className={css.changeTypeBadge}>
                      {changeTypeLabel(change.changeType)}
                    </span>
                  </div>
                  {hasTransition && (
                    <p className={css.changeTransition}>
                      {transition.join(" \u2192 ")}
                    </p>
                  )}
                  <div className={css.changeMeta}>
                    {change.detectedAt && (
                      <span>Detected {formatDate(change.detectedAt)}</span>
                    )}
                    <ProvenanceTag itemSource={change.itemSource} discrepancy={change.discrepancy} />
                  </div>
                </div>
                {onItemFeedback && (
                  <span className={css.itemActions}>
                    <IntelligenceFeedback
                      value={getItemFeedback?.(path) ?? null}
                      onFeedback={(type) => onItemFeedback(path, type)}
                    />
                  </span>
                )}
              </div>
            );
          })}
        </div>
      )}

      {/* ── Blockers ── */}
      {hasBlockers && (
        <div className={css.subsection}>
          <h3 className={css.subsectionLabel}>Blockers</h3>
          {blockers.map((b, i) => {
            const badge = b.impact ? impactBadge(b.impact) : null;
            const path = `blockers[${i}].description`;
            return (
              <div key={i} className={css.blockerItem}>
                <div className={css.blockerInner}>
                  <div className={css.blockerContent}>
                    {onUpdateField ? (
                      <EditableText
                        value={b.description}
                        onChange={(v) => onUpdateField(path, v)}
                        as="p"
                        multiline
                        className={css.blockerDesc}
                      />
                    ) : (
                      <p className={css.blockerDesc}>{b.description}</p>
                    )}
                    <div className={css.blockerMeta}>
                      {b.owner && <span>{b.owner}</span>}
                      {b.since && <span>Since: {formatDate(b.since)}</span>}
                      {badge && (
                        <span className={`${css.badge} ${badge.cls}`}>{badge.label}</span>
                      )}
                    </div>
                  </div>
                  {showActions && (
                    <span className={css.itemActions}>
                      {onItemFeedback && (
                        <IntelligenceFeedback
                          value={getItemFeedback?.(path) ?? null}
                          onFeedback={(type) => onItemFeedback(path, type)}
                        />
                      )}
                      {onUpdateField && (
                        <button
                          type="button"
                          className={css.dismissButton}
                          onClick={() => onUpdateField(path, "")}
                          title="Dismiss"
                        >
                          <X size={13} />
                        </button>
                      )}
                    </span>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
}
