/**
 * ValueCommitments — Value & Commitments chapter (Ledger style).
 * Surfaces valueDelivered, successMetrics, and openCommitments
 * from EntityIntelligence. Collapses entirely when all three are empty.
 *
 * I550: Per-item inline editing, dismiss, and feedback controls.
 */
import { X, Check } from "lucide-react";
import type { EntityIntelligence } from "@/types";
import { hasBleedFlag } from "@/lib/contamination-guard";
import { ContaminationWarning } from "@/components/ui/ContaminationWarning";
import { EditableText } from "@/components/ui/EditableText";
import { IntelligenceFeedback } from "@/components/ui/IntelligenceFeedback";
import { ProvenanceTag } from "@/components/ui/ProvenanceTag";
import css from "./ValueCommitments.module.css";

interface ValueCommitmentsProps {
  intelligence: EntityIntelligence;
  /** When provided, items become editable. Called with (fieldPath, newValue). */
  onUpdateField?: (fieldPath: string, value: string) => void;
  /** Per-item feedback value getter. */
  getItemFeedback?: (fieldPath: string) => "positive" | "negative" | null;
  /** Per-item feedback submit. */
  onItemFeedback?: (fieldPath: string, type: "positive" | "negative") => void;
}

/* -- Date helpers -- */

function isOverdue(dateStr?: string): boolean {
  if (!dateStr) return false;
  try {
    return new Date(dateStr) < new Date();
  } catch {
    return false;
  }
}

/** Parse a date string into { month, day } for the large date block. */
function parseDateParts(dateStr?: string): { month: string; day: string } | null {
  if (!dateStr) return null;
  try {
    const d = new Date(dateStr);
    if (isNaN(d.getTime())) return null;
    return {
      month: d.toLocaleDateString("en-US", { month: "short" }),
      day: String(d.getDate()),
    };
  } catch {
    return null;
  }
}

function formatShortDate(dateStr?: string): string {
  if (!dateStr) return "";
  try {
    const d = new Date(dateStr);
    if (isNaN(d.getTime())) return dateStr;
    return d.toLocaleDateString("en-US", { month: "short", day: "numeric" });
  } catch {
    return dateStr;
  }
}

/* -- Status helpers -- */

function getMetricStatusColor(status?: string): string {
  switch (status?.toLowerCase().replace(/[_\s-]/g, "")) {
    case "ontrack":
    case "achieved":
      return css.metricFillSage;
    case "atrisk":
      return css.metricFillTurmeric;
    case "behind":
      return css.metricFillTerracotta;
    default:
      return css.metricFillTurmeric;
  }
}

function getCommitmentBadgeColor(status?: string): string {
  switch (status?.toLowerCase().replace(/[_\s-]/g, "")) {
    case "delivered":
      return css.badgeSage;
    case "atrisk":
      return css.badgeTurmeric;
    case "behind":
      return css.badgeTerracotta;
    case "open":
      return css.badgeNeutral;
    default:
      return css.badgeNeutral;
  }
}

function getCommitmentStatusLabel(status?: string): string {
  switch (status?.toLowerCase().replace(/[_\s-]/g, "")) {
    case "delivered":
      return "Delivered";
    case "atrisk":
      return "At Risk";
    case "behind":
      return "Behind";
    case "open":
      return "Open";
    default:
      return status ?? "Open";
  }
}

/** DOS-18: Classify impact text into the canonical revenue|cost|risk|speed enum.
 *  Falls back to "default" when no enum marker is present. The impact field can
 *  contain either a bare enum token or a sentence; both cases are handled. */
function classifyImpact(raw?: string): "revenue" | "cost" | "risk" | "speed" | "default" {
  if (!raw) return "default";
  const s = raw.toLowerCase();
  if (/\brevenue\b|\bexpansion\b|\barr\b|\bupsell\b/.test(s)) return "revenue";
  if (/\bcost\b|\bsavings?\b|\bavoid(ed|ance)?\b/.test(s)) return "cost";
  if (/\brisk\b|\bcompliance\b|\bsecurity\b|\bdora\b|\bsoc\b/.test(s)) return "risk";
  if (/\bspeed\b|\bfaster\b|\btime to\b|\bthroughput\b|\bproductivity\b/.test(s)) return "speed";
  return "default";
}

function impactTagClass(kind: string): string {
  switch (kind) {
    case "revenue": return css.impactTagRevenue;
    case "cost": return css.impactTagCost;
    case "risk": return css.impactTagRisk;
    case "speed": return css.impactTagSpeed;
    default: return css.impactTagDefault;
  }
}

function impactTagLabel(kind: string): string {
  switch (kind) {
    case "revenue": return "Revenue";
    case "cost": return "Cost";
    case "risk": return "Risk";
    case "speed": return "Speed";
    default: return "Impact";
  }
}

/** Heuristic: is this a short, display-worthy metric value (number, percentage, grade)?
 *  If not, it's narrative text that shouldn't render at 28px serif. */
function isShortValue(value?: string): boolean {
  if (!value) return true;
  // Short enough to display large (≤20 chars covers "$639,148", "9", "85%", "A+", "Q2 2026")
  return value.length <= 20;
}

/** Rough progress percent from current/target strings (best-effort numeric extraction). */
function estimateProgress(current?: string, target?: string): number | null {
  if (!current || !target) return null;
  const cur = parseFloat(current.replace(/[^0-9.]/g, ""));
  const tgt = parseFloat(target.replace(/[^0-9.]/g, ""));
  if (isNaN(cur) || isNaN(tgt) || tgt === 0) return null;
  return Math.min(100, Math.round((cur / tgt) * 100));
}

export function ValueCommitments({
  intelligence,
  onUpdateField,
  getItemFeedback,
  onItemFeedback,
}: ValueCommitmentsProps) {
  // Filter out dismissed items (empty description/statement = removed by user)
  const valueDelivered = (intelligence.valueDelivered ?? []).filter((v) => v.statement?.trim());
  const successMetrics = (intelligence.successMetrics ?? []).filter((m) => m.name?.trim());
  const openCommitments = (intelligence.openCommitments ?? []).filter((c) => c.description?.trim());

  const hasValue = valueDelivered.length > 0;
  const hasMetrics = successMetrics.length > 0;
  const hasCommitments = openCommitments.length > 0;

  if (!hasValue && !hasMetrics && !hasCommitments) return null;

  return (
    <section className={css.section}>
      {/* Value Delivered -- date timeline */}
      {hasValue && (
        <div className={css.subsection}>
          <h3 className={css.subsectionLabel}>Value Delivered</h3>
          <div className={css.valueList}>
            {valueDelivered.map((item, i) => {
              const parts = parseDateParts(item.date);
              return (
                <div key={i} className={css.valueRow}>
                  <div className={css.dateBlock}>
                    {parts ? (
                      <>
                        <div className={css.dateMonth}>{parts.month}</div>
                        <div className={css.dateDay}>{parts.day}</div>
                      </>
                    ) : (
                      <span className={css.dateFallback}>{"\u2014"}</span>
                    )}
                  </div>
                  <div className={css.valueBody}>
                    <div className={css.valueStatement}>
                      {onUpdateField ? (
                        <EditableText
                          value={item.statement}
                          onChange={(v) =>
                            onUpdateField(`valueDelivered[${i}].statement`, v)
                          }
                          as="span"
                          multiline
                        />
                      ) : (
                        item.statement
                      )}
                    </div>
                    {item.impact && (() => {
                      const kind = classifyImpact(item.impact);
                      const label = impactTagLabel(kind);
                      // DOS-230: When the raw impact string is just the enum
                      // token (e.g. "revenue" → "Revenue"), the pill already
                      // conveys it — suppress the plain-text duplicate.
                      // A screen-reader-only copy preserves semantics so the
                      // badge still has a label for assistive tech even when
                      // the visual duplicate is gone.
                      const normalized = item.impact.trim().toLowerCase();
                      const duplicatesLabel = kind !== "default" && (normalized === kind || normalized === label.toLowerCase());
                      return (
                        <div className={css.valueImpact}>
                          <span className={`${css.impactTag} ${impactTagClass(kind)}`}>{label}</span>
                          {duplicatesLabel ? (
                            <span className="sr-only">{label}</span>
                          ) : onUpdateField ? (
                            <EditableText
                              value={item.impact}
                              onChange={(v) =>
                                onUpdateField(`valueDelivered[${i}].impact`, v)
                              }
                              as="span"
                              multiline
                              className={css.impactText}
                            />
                          ) : (
                            <span className={css.impactText}>{item.impact}</span>
                          )}
                        </div>
                      );
                    })()}
                    <span className={css.provenanceRow}>
                      {item.itemSource?.source === "user_correction" && (
                        <span className={css.confirmedBadge}>
                          <Check size={10} />
                          Confirmed
                        </span>
                      )}
                      <ProvenanceTag itemSource={item.itemSource} discrepancy={item.discrepancy} />
                    </span>
                  </div>
                  {(onUpdateField || onItemFeedback) && (
                    <span className={css.itemActions}>
                      {onItemFeedback && (
                        <IntelligenceFeedback
                          value={
                            getItemFeedback?.(
                              `valueDelivered[${i}].statement`
                            ) ?? null
                          }
                          onFeedback={(type) =>
                            onItemFeedback(
                              `valueDelivered[${i}].statement`,
                              type
                            )
                          }
                        />
                      )}
                      {onUpdateField && (
                        <button
                          type="button"
                          className={css.dismissButton}
                          onClick={() =>
                            onUpdateField(`valueDelivered[${i}].statement`, "")
                          }
                          title="Remove"
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
        </div>
      )}

      {/* Success Metrics -- dashboard strip */}
      {hasMetrics && (
        <div className={css.subsection}>
          <h3 className={css.subsectionLabel}>Success Metrics</h3>
          {hasBleedFlag(intelligence.consistencyFindings, "successMetrics") ? (
            <ContaminationWarning />
          ) : (
            <div className={css.metricsStrip}>
              {successMetrics.map((metric, i) => {
                const pct = estimateProgress(metric.current, metric.target);
                const fillClass = getMetricStatusColor(metric.status);
                return (
                  <div key={i} className={css.metricCell}>
                    <div className={css.metricName}>{metric.name}</div>
                    <div className={isShortValue(metric.current) ? css.metricValue : css.metricValueLong}>
                      {metric.current ?? "\u2014"}
                    </div>
                    {metric.target && (
                      <div className={css.metricTarget}>
                        Target: {metric.target}
                      </div>
                    )}
                    <div className={css.metricBar}>
                      <div
                        className={`${css.metricFill} ${fillClass}`}
                        style={{
                          '--progress-width': pct != null ? `${pct}%` : "0%",
                        } as React.CSSProperties}
                      />
                    </div>
                    {onItemFeedback && (
                      <span className={css.metricFeedback}>
                        <IntelligenceFeedback
                          value={
                            getItemFeedback?.(
                              `successMetrics[${i}].name`
                            ) ?? null
                          }
                          onFeedback={(type) =>
                            onItemFeedback(`successMetrics[${i}].name`, type)
                          }
                        />
                      </span>
                    )}
                  </div>
                );
              })}
            </div>
          )}
        </div>
      )}

      {/* Open Commitments -- dossier-style vertical cards */}
      {hasCommitments && (
        <div className={css.subsection}>
          <h3 className={css.subsectionLabel}>Open Commitments</h3>
          <div className={css.commitmentList}>
            {openCommitments.map((commitment, i) => {
              const overdue =
                commitment.status !== "delivered" &&
                isOverdue(commitment.dueDate);
              return (
                <div key={i} className={css.commitmentItem}>
                  <div className={`${css.commitmentDot} ${overdue ? css.commitmentDotOverdue : css.commitmentDotOpen}`} />
                  <div className={css.commitmentBody}>
                    <div className={css.commitmentDesc}>
                      {onUpdateField ? (
                        <EditableText
                          value={commitment.description}
                          onChange={(v) =>
                            onUpdateField(`openCommitments[${i}].description`, v)
                          }
                          as="span"
                          multiline
                        />
                      ) : (
                        commitment.description
                      )}
                    </div>
                    <div className={css.commitmentMeta}>
                      {commitment.owner && (
                        <span className={css.commitmentOwner}>{commitment.owner}</span>
                      )}
                      {commitment.dueDate && (
                        <span className={overdue ? css.commitmentDueOverdue : css.commitmentDue}>
                          {overdue ? "Overdue: " : "Due: "}{formatShortDate(commitment.dueDate)}
                        </span>
                      )}
                      {commitment.status && (
                        <span className={`${css.statusBadge} ${getCommitmentBadgeColor(commitment.status)}`}>
                          {getCommitmentStatusLabel(commitment.status)}
                        </span>
                      )}
                      {commitment.source && (
                        <ProvenanceTag itemSource={commitment.source ? { source: commitment.source, confidence: 0, sourcedAt: "" } : undefined} />
                      )}
                    </div>
                  </div>
                  {(onUpdateField || onItemFeedback) && (
                    <span className={css.itemActions}>
                      {onItemFeedback && (
                        <IntelligenceFeedback
                          value={
                            getItemFeedback?.(`openCommitments[${i}].description`) ?? null
                          }
                          onFeedback={(type) =>
                            onItemFeedback(`openCommitments[${i}].description`, type)
                          }
                        />
                      )}
                      {onUpdateField && (
                        <button
                          type="button"
                          className={css.dismissButton}
                          onClick={() =>
                            onUpdateField(`openCommitments[${i}].description`, "")
                          }
                          title="Remove"
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
        </div>
      )}
    </section>
  );
}
