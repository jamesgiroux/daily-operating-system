/**
 * ValueCommitments — Chapter 4 "What we've built together" (Context tab).
 *
 * Matches .docs/mockups/account-context-globex.html Chapter 4.
 * Two subsections, each rendered only when data is present:
 *   1. Value delivered — 3-col card grid (impact pill, serif headline, mono source).
 *   2. Success metrics — 3-col card grid (name, current/target, status pill + owner).
 *
 * Open Commitments is NOT part of this chapter — commitments live on the Work tab.
 *
 * Per-item dismiss (hover X) calls both `onUpdateField(path, "")` (hides the item)
 * and `onItemFeedback(path, "negative")` (feeds Bayesian source weights).
 */
import { useState } from "react";
import type { EntityIntelligence } from "@/types";
import { hasBleedFlag } from "@/lib/contamination-guard";
import { ContaminationWarning } from "@/components/ui/ContaminationWarning";
import { IntelligenceCorrection } from "@/components/ui/IntelligenceCorrection";
import { useEntitySuppressions } from "@/hooks/useEntitySuppressions";
import { ProvenanceTag } from "@/components/ui/ProvenanceTag";
import css from "./ValueCommitments.module.css";

interface ValueCommitmentsProps {
  intelligence: EntityIntelligence;
  onUpdateField?: (fieldPath: string, value: string) => void;
  onItemFeedback?: (fieldPath: string, type: "positive" | "negative") => void;
}

/* -- Helpers -------------------------------------------------------------- */

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

function metricStatus(raw?: string): { label: string; cls: string } | null {
  const key = raw?.toLowerCase().replace(/[_\s-]/g, "") ?? "";
  switch (key) {
    case "achieved":
      return { label: "Achieved", cls: css.statusAchieved };
    case "ontrack":
      return { label: "On track", cls: css.statusOnTrack };
    case "atrisk":
      return { label: "At risk", cls: css.statusAtRisk };
    case "behind":
      return { label: "Behind", cls: css.statusBehind };
    default:
      return raw ? { label: raw, cls: css.statusNeutral } : null;
  }
}

/* -- Component ------------------------------------------------------------ */

export function ValueCommitments({
  intelligence,
  onUpdateField,
  onItemFeedback: _onItemFeedback,
}: ValueCommitmentsProps) {
  const suppressions = useEntitySuppressions(intelligence.entityId);
  const [hiddenPaths, setHiddenPaths] = useState<Set<string>>(() => new Set());
  const valueDelivered = (intelligence.valueDelivered ?? [])
    .map((item, index) => ({ item, index }))
    .filter(({ item }) => item.statement?.trim())
    .filter(({ index }) => !hiddenPaths.has(`valueDelivered[${index}].statement`))
    .filter(({ item }) => !suppressions.isSuppressed("valueDelivered", item.statement));
  const successMetrics = (intelligence.successMetrics ?? [])
    .map((item, index) => ({ item, index }))
    .filter(({ item }) => item.name?.trim())
    .filter(({ index }) => !hiddenPaths.has(`successMetrics[${index}].name`))
    .filter(({ item }) => !suppressions.isSuppressed("successMetrics", item.name));

  const hasValue = valueDelivered.length > 0;
  const hasMetrics = successMetrics.length > 0;

  if (!hasValue && !hasMetrics) return null;

  const metricsBleed = hasBleedFlag(intelligence.consistencyFindings, "successMetrics");

  return (
    <section className={css.section}>
      {hasValue && (
        <>
          <div className={css.subsectionLabel}>Value delivered</div>
          <div className={css.valueGrid}>
            {valueDelivered.map(({ item, index }, i) => {
              const kind = classifyImpact(item.impact);
              const sourceBits: string[] = [];
              if (item.date) sourceBits.push(formatShortDate(item.date));
              if (item.source) sourceBits.push(item.source);
              const path = `valueDelivered[${index}].statement`;
              return (
                <article key={i} className={css.valueCard}>
                  <span className={`${css.impactTag} ${impactTagClass(kind)}`}>
                    {impactTagLabel(kind)}
                  </span>
                  <div className={css.valueHeadline}>{item.statement}</div>
                  {sourceBits.length > 0 && (
                    <div className={css.valueSource}>{sourceBits.join(" · ")}</div>
                  )}
                  <ProvenanceTag itemSource={item.itemSource} discrepancy={item.discrepancy} />
                  {onUpdateField && (
                    <IntelligenceCorrection
                      entityId={intelligence.entityId}
                      entityType="account"
                      field="valueDelivered"
                      itemKey={item.statement}
                      onDismissed={async () => {
                        suppressions.markSuppressed("valueDelivered", item.statement);
                        setHiddenPaths((prev) => new Set(prev).add(path));
                        await onUpdateField(path, "");
                      }}
                    />
                  )}
                </article>
              );
            })}
          </div>
        </>
      )}

      {hasMetrics && (
        <>
          <div className={css.subsectionLabel}>Success metrics</div>
          {metricsBleed ? (
            <ContaminationWarning />
          ) : (
            <div className={css.metricRow}>
              {successMetrics.map(({ item: metric, index }, i) => {
                const status = metricStatus(metric.status);
                const path = `successMetrics[${index}].name`;
                return (
                  <article key={i} className={css.metricCard}>
                    <div className={css.metricName}>{metric.name}</div>
                    <div className={css.metricValues}>
                      <span className={css.metricCurrent}>{metric.current ?? "\u2014"}</span>
                      {metric.target && (
                        <span className={css.metricTarget}>/ {metric.target}</span>
                      )}
                    </div>
                    <div className={css.metricFooter}>
                      {status && (
                        <span className={`${css.metricStatus} ${status.cls}`}>
                          {status.label}
                        </span>
                      )}
                      {metric.owner && (
                        <span className={css.metricOwner}>Owner: {metric.owner}</span>
                      )}
                    </div>
                    {onUpdateField && (
                      <IntelligenceCorrection
                        entityId={intelligence.entityId}
                        entityType="account"
                        field="successMetrics"
                        itemKey={metric.name}
                        onDismissed={async () => {
                          suppressions.markSuppressed("successMetrics", metric.name);
                          setHiddenPaths((prev) => new Set(prev).add(path));
                          await onUpdateField(path, "");
                        }}
                      />
                    )}
                  </article>
                );
              })}
            </div>
          )}
        </>
      )}
    </section>
  );
}
