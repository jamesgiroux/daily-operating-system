/**
 * ValueCommitments — Value & Commitments chapter.
 * Surfaces valueDelivered, successMetrics, and openCommitments
 * from EntityIntelligence. Collapses entirely when all three are empty.
 *
 * I550: Per-item inline editing, dismiss, and feedback controls.
 */
import { X } from "lucide-react";
import type { EntityIntelligence } from "@/types";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { EditableText } from "@/components/ui/EditableText";
import { IntelligenceFeedback } from "@/components/ui/IntelligenceFeedback";
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

function isOverdue(dateStr?: string): boolean {
  if (!dateStr) return false;
  try {
    return new Date(dateStr) < new Date();
  } catch {
    return false;
  }
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

function getStatusColor(status?: string): string {
  switch (status?.toLowerCase().replace(/[_\s-]/g, "")) {
    case "ontrack":
      return css.badgeSage;
    case "achieved":
      return css.badgeSageCheck;
    case "atrisk":
      return css.badgeTurmeric;
    case "behind":
      return css.badgeTerracotta;
    default:
      return css.badgeNeutral;
  }
}

function getStatusLabel(status?: string): string {
  switch (status?.toLowerCase().replace(/[_\s-]/g, "")) {
    case "ontrack":
      return "On Track";
    case "achieved":
      return "Achieved";
    case "atrisk":
      return "At Risk";
    case "behind":
      return "Behind";
    default:
      return status ?? "Unknown";
  }
}

function getCommitmentStatusColor(status?: string): string {
  switch (status?.toLowerCase().replace(/[_\s-]/g, "")) {
    case "delivered":
      return css.badgeSage;
    case "atrisk":
      return css.badgeTurmeric;
    case "open":
      return css.badgeNeutral;
    default:
      return css.badgeNeutral;
  }
}

function getImpactLabel(impact?: string): string {
  if (!impact) return "";
  return impact.charAt(0).toUpperCase() + impact.slice(1);
}

export function ValueCommitments({
  intelligence,
  onUpdateField,
  getItemFeedback,
  onItemFeedback,
}: ValueCommitmentsProps) {
  const valueDelivered = intelligence.valueDelivered ?? [];
  const successMetrics = intelligence.successMetrics ?? [];
  const openCommitments = intelligence.openCommitments ?? [];

  const hasValue = valueDelivered.length > 0;
  const hasMetrics = successMetrics.length > 0;
  const hasCommitments = openCommitments.length > 0;

  if (!hasValue && !hasMetrics && !hasCommitments) return null;

  return (
    <section className={css.section}>
      <ChapterHeading title="Value & Commitments" />

      {/* Value Delivered — editorial table */}
      {hasValue && (
        <div className={css.subsection}>
          <h3 className={css.subsectionLabel}>Value Delivered</h3>
          <div className={css.valueTable}>
            <div className={css.tableHeader}>
              <span className={css.colDate}>Date</span>
              <span className={css.colStatement}>Outcome</span>
              <span className={css.colImpact}>Impact</span>
            </div>
            {valueDelivered.map((item, i) => (
              <div key={i} className={css.tableRow}>
                <span className={css.cellDate}>
                  {item.date ? formatDate(item.date) : "\u2014"}
                </span>
                <span className={css.cellStatement}>
                  {onUpdateField ? (
                    <EditableText
                      value={item.statement}
                      onChange={(v) => onUpdateField(`valueDelivered[${i}].statement`, v)}
                      as="span"
                      multiline
                      style={{ font: "inherit", color: "inherit" }}
                    />
                  ) : (
                    item.statement
                  )}
                </span>
                <span className={css.cellImpact}>
                  {item.impact && (
                    <span className={css.impactBadge}>
                      {onUpdateField ? (
                        <EditableText
                          value={getImpactLabel(item.impact)}
                          onChange={(v) => onUpdateField(`valueDelivered[${i}].impact`, v)}
                          as="span"
                          multiline={false}
                          style={{ font: "inherit", color: "inherit" }}
                        />
                      ) : (
                        getImpactLabel(item.impact)
                      )}
                    </span>
                  )}
                </span>
                {(onUpdateField || onItemFeedback) && (
                  <span className={css.itemActions}>
                    {onItemFeedback && (
                      <IntelligenceFeedback
                        value={getItemFeedback?.(`valueDelivered[${i}].statement`) ?? null}
                        onFeedback={(type) => onItemFeedback(`valueDelivered[${i}].statement`, type)}
                      />
                    )}
                    {onUpdateField && (
                      <button
                        type="button"
                        className={css.dismissButton}
                        onClick={() => onUpdateField(`valueDelivered[${i}].statement`, "")}
                        title="Dismiss"
                      >
                        <X size={13} />
                      </button>
                    )}
                  </span>
                )}
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Success Metrics — card grid */}
      {hasMetrics && (
        <div className={css.subsection}>
          <h3 className={css.subsectionLabel}>Success Metrics</h3>
          <div className={css.metricsGrid}>
            {successMetrics.map((metric, i) => (
              <div key={i} className={css.metricCard}>
                <div className={css.metricHeader}>
                  <span className={css.metricName}>{metric.name}</span>
                  <div className={css.metricHeaderRight}>
                    <span className={`${css.statusBadge} ${getStatusColor(metric.status)}`}>
                      {getStatusLabel(metric.status)}
                    </span>
                    {onItemFeedback && (
                      <span className={css.metricFeedback}>
                        <IntelligenceFeedback
                          value={getItemFeedback?.(`successMetrics[${i}].name`) ?? null}
                          onFeedback={(type) => onItemFeedback(`successMetrics[${i}].name`, type)}
                        />
                      </span>
                    )}
                  </div>
                </div>
                {(metric.target || metric.current) && (
                  <div className={css.metricValues}>
                    {metric.target && (
                      <span className={css.metricValue}>
                        <span className={css.metricValueLabel}>Target</span> {metric.target}
                      </span>
                    )}
                    {metric.current && (
                      <span className={css.metricValue}>
                        <span className={css.metricValueLabel}>Current</span> {metric.current}
                      </span>
                    )}
                  </div>
                )}
                {metric.owner && (
                  <span className={css.metricOwner}>{metric.owner}</span>
                )}
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Open Commitments — timeline list */}
      {hasCommitments && (
        <div className={css.subsection}>
          <h3 className={css.subsectionLabel}>Open Commitments</h3>
          <div className={css.commitmentList}>
            {openCommitments.map((commitment, i) => {
              const overdue = commitment.status !== "delivered" && isOverdue(commitment.dueDate);
              return (
                <div key={i} className={css.commitmentItem}>
                  <div className={css.commitmentContent}>
                    {onUpdateField ? (
                      <EditableText
                        value={commitment.description}
                        onChange={(v) => onUpdateField(`openCommitments[${i}].description`, v)}
                        as="p"
                        multiline
                        style={{
                          fontFamily: "var(--font-serif)",
                          fontSize: 16,
                          lineHeight: 1.5,
                          color: "var(--color-text-primary)",
                          margin: 0,
                        }}
                      />
                    ) : (
                      <p className={css.commitmentDescription}>{commitment.description}</p>
                    )}
                    <div className={css.commitmentMeta}>
                      {commitment.owner && (
                        <span className={css.commitmentOwner}>{commitment.owner}</span>
                      )}
                      {commitment.dueDate && (
                        <span className={overdue ? css.commitmentDateOverdue : css.commitmentDate}>
                          {overdue ? "Overdue: " : "Due: "}{formatDate(commitment.dueDate)}
                        </span>
                      )}
                      {commitment.status && (
                        <span className={`${css.statusBadge} ${getCommitmentStatusColor(commitment.status)}`}>
                          {commitment.status}
                        </span>
                      )}
                      {(onUpdateField || onItemFeedback) && (
                        <span className={css.itemActions}>
                          {onItemFeedback && (
                            <IntelligenceFeedback
                              value={getItemFeedback?.(`openCommitments[${i}].description`) ?? null}
                              onFeedback={(type) => onItemFeedback(`openCommitments[${i}].description`, type)}
                            />
                          )}
                          {onUpdateField && (
                            <button
                              type="button"
                              className={css.dismissButton}
                              onClick={() => onUpdateField(`openCommitments[${i}].description`, "")}
                              title="Dismiss"
                            >
                              <X size={13} />
                            </button>
                          )}
                        </span>
                      )}
                    </div>
                    {commitment.source && (
                      <span className={css.commitmentSource}>Source: {commitment.source}</span>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}
    </section>
  );
}
