/**
 * StrategicLandscape -- Chapter 3 "What matters to them" (Context tab).
 *
 * Matches .docs/mockups/account-context-globex.html Chapter 3.
 * Three subsections, each rendered only when data is present:
 *   1. Strategic priorities — 2-col grid, span-2 on last card when count is odd.
 *   2. Competitive landscape — 3-col grid with 3-dot threat scale.
 *   3. Regulatory & market context — full-width reg-card per item.
 */
import { useState } from "react";
import type { EntityIntelligence } from "@/types";
import { IntelligenceCorrection } from "@/components/ui/IntelligenceCorrection";
import { useEntitySuppressions } from "@/hooks/useEntitySuppressions";
import { ProvenanceTag } from "@/components/ui/ProvenanceTag";
import css from "./StrategicLandscape.module.css";

interface StrategicLandscapeProps {
  intelligence: EntityIntelligence;
  /** Dismiss = clear field + fire negative feedback (Bayesian loop). */
  onUpdateField?: (fieldPath: string, value: string) => void;
  onItemFeedback?: (fieldPath: string, type: "positive" | "negative") => void;
}

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

function priorityStatus(raw?: string): { label: string; cls: string } {
  const key = raw?.toLowerCase().replace(/[_\s-]/g, "") ?? "";
  switch (key) {
    case "active":
      return { label: "Active", cls: css.statusActive };
    case "exploring":
      return { label: "Exploring", cls: css.statusExploring };
    case "evaluating":
      return { label: "Evaluating", cls: css.statusEvaluating };
    case "paused":
      return { label: "Paused", cls: css.statusNeutral };
    case "completed":
      return { label: "Completed", cls: css.statusActive };
    case "atrisk":
      return { label: "At Risk", cls: css.statusAtRisk };
    default:
      return { label: raw ?? "Active", cls: css.statusActive };
  }
}

function threatDotsCount(level?: string): 1 | 2 | 3 {
  switch (level?.toLowerCase().replace(/[_\s-]/g, "")) {
    case "displacement":
    case "incumbent":
      return 3;
    case "evaluation":
      return 2;
    case "mentioned":
    default:
      return 1;
  }
}

function threatLabel(level?: string): string {
  const key = level?.toLowerCase().replace(/[_\s-]/g, "") ?? "";
  switch (key) {
    case "displacement":
      return "Displacement";
    case "evaluation":
      return "Evaluation";
    case "incumbent":
      return "Incumbent";
    case "mentioned":
    default:
      return "Mentioned";
  }
}

export function StrategicLandscape({
  intelligence,
  onUpdateField,
  onItemFeedback: _onItemFeedback,
}: StrategicLandscapeProps) {
  const suppressions = useEntitySuppressions(intelligence.entityId);
  const [hiddenPaths, setHiddenPaths] = useState<Set<string>>(() => new Set());
  const editable = !!onUpdateField;
  const priorities = (intelligence.strategicPriorities ?? [])
    .map((item, index) => ({ item, index }))
    .filter(({ item }) => item.priority?.trim())
    .filter(({ index }) => !hiddenPaths.has(`strategicPriorities[${index}].priority`))
    .filter(({ item }) => !suppressions.isSuppressed("strategicPriorities", item.priority));
  const competitors = (intelligence.competitiveContext ?? [])
    .map((item, index) => ({ item, index }))
    .filter(({ item }) => item.competitor?.trim() || item.context?.trim())
    .filter(({ index }) => !hiddenPaths.has(`competitiveContext[${index}].competitor`))
    .filter(
      ({ item }) =>
        !suppressions.isSuppressed(
          "competitiveContext",
          item.competitor ?? item.context ?? null,
        ),
    );
  const marketItems = (intelligence.marketContext ?? [])
    .map((item, index) => ({ item, index }))
    .filter(({ item }) => item.title?.trim() || item.body?.trim())
    .filter(({ index }) => !hiddenPaths.has(`marketContext[${index}].title`))
    .filter(
      ({ item }) =>
        !suppressions.isSuppressed(
          "marketContext",
          item.title ?? item.body ?? null,
        ),
    );

  const hasPriorities = priorities.length > 0;
  const hasCompetitors = competitors.length > 0;
  const hasMarket = marketItems.length > 0;

  if (!hasPriorities && !hasCompetitors && !hasMarket) return null;

  return (
    <section className={css.section}>
      {hasPriorities && (
        <>
          <div className={css.subsectionLabel}>Strategic priorities</div>
          <div className={css.priorityGrid}>
            {priorities.map(({ item: p, index: rawIndex }, i) => {
              const status = priorityStatus(p.status);
              const isLastOdd =
                i === priorities.length - 1 && priorities.length % 2 === 1;
              const metaBits = [p.owner, p.timeline].filter(Boolean);
              const ctxText = p.context;
              const path = `strategicPriorities[${rawIndex}].priority`;
              return (
                <article
                  key={i}
                  className={`${css.priorityCard}${isLastOdd ? ` ${css.priorityCardSpan}` : ""}`}
                >
                  <div className={css.priorityHead}>
                    <div className={css.priorityName}>{p.priority}</div>
                    <span className={`${css.statusTag} ${status.cls}`}>
                      {status.label}
                    </span>
                  </div>
                  {metaBits.length > 0 && (
                    <div className={css.priorityMeta}>
                      {metaBits.join(" · ")}
                    </div>
                  )}
                  {ctxText && <div className={css.priorityContext}>{ctxText}</div>}
                  {editable && (
                    <IntelligenceCorrection
                      entityId={intelligence.entityId}
                      entityType="account"
                      field="strategicPriorities"
                      itemKey={p.priority}
                      onDismissed={async () => {
                        suppressions.markSuppressed("strategicPriorities", p.priority);
                        setHiddenPaths((prev) => new Set(prev).add(path));
                        await onUpdateField?.(path, "");
                      }}
                    />
                  )}
                </article>
              );
            })}
          </div>
        </>
      )}

      {hasCompetitors && (
        <>
          <div className={css.subsectionLabel}>Competitive landscape</div>
          <div className={css.competitorRow}>
            {competitors.map(({ item: c, index: rawIndex }, i) => {
              const dots = threatDotsCount(c.threatLevel);
              const label = threatLabel(c.threatLevel);
              const sourceBits: string[] = [];
              if (c.detectedAt) sourceBits.push(formatDate(c.detectedAt));
              if (c.source) sourceBits.push(c.source);
              const path = `competitiveContext[${rawIndex}].competitor`;
              return (
                <article key={i} className={css.competitorCard}>
                  <div className={css.competitorName}>{c.competitor ?? "—"}</div>
                  <div className={css.threatScale}>
                    <span
                      className={`${css.threatDot}${dots >= 1 ? ` ${css.threatDotOn}` : ""}`}
                    />
                    <span
                      className={`${css.threatDot}${dots >= 2 ? ` ${css.threatDotOn}` : ""}`}
                    />
                    <span
                      className={`${css.threatDot}${dots >= 3 ? ` ${css.threatDotOn}` : ""}`}
                    />
                  </div>
                  <div className={css.threatLabel}>{label}</div>
                  {c.context && (
                    <div className={css.competitorContext}>{c.context}</div>
                  )}
                  {sourceBits.length > 0 && (
                    <div className={css.competitorSource}>
                      {sourceBits.join(" · ")}
                    </div>
                  )}
                  <ProvenanceTag itemSource={c.itemSource} discrepancy={c.discrepancy} />
                  {editable && (
                    <IntelligenceCorrection
                      entityId={intelligence.entityId}
                      entityType="account"
                      field="competitiveContext"
                      itemKey={c.competitor ?? c.context ?? path}
                      onDismissed={async () => {
                        suppressions.markSuppressed(
                          "competitiveContext",
                          c.competitor ?? c.context ?? path,
                        );
                        setHiddenPaths((prev) => new Set(prev).add(path));
                        await onUpdateField?.(path, "");
                      }}
                    />
                  )}
                </article>
              );
            })}
          </div>
        </>
      )}

      {hasMarket && (
        <>
          <div className={css.subsectionLabel}>Regulatory &amp; market context</div>
          {marketItems.map(({ item: m, index: rawIndex }, i) => {
            const path = `marketContext[${rawIndex}].title`;
            return (
              <article key={i} className={css.regCard}>
                <div className={css.regHead}>
                  <div className={css.regTitle}>{m.title}</div>
                  {m.category && <span className={css.xrefPill}>{m.category}</span>}
                </div>
                {m.body && <p className={css.regBody}>{m.body}</p>}
                <ProvenanceTag itemSource={m.itemSource} discrepancy={m.discrepancy} />
                {editable && (
                <IntelligenceCorrection
                  entityId={intelligence.entityId}
                  entityType="account"
                  field="marketContext"
                  itemKey={m.title ?? m.body ?? path}
                  onDismissed={async () => {
                    suppressions.markSuppressed(
                      "marketContext",
                      m.title ?? m.body ?? path,
                    );
                    setHiddenPaths((prev) => new Set(prev).add(path));
                    await onUpdateField?.(path, "");
                  }}
                  />
                )}
              </article>
            );
          })}
        </>
      )}
    </section>
  );
}
