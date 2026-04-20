/**
 * StrategicLandscape -- Chapter 3 "What matters to them" (Context tab).
 *
 * Matches .docs/mockups/account-context-globex.html Chapter 3.
 * Three subsections, each rendered only when data is present:
 *   1. Strategic priorities — 2-col grid, span-2 on last card when count is odd.
 *   2. Competitive landscape — 3-col grid with 3-dot threat scale.
 *   3. Regulatory & market context — full-width reg-card per item.
 */
import { X } from "lucide-react";
import type { EntityIntelligence } from "@/types";
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
  onItemFeedback,
}: StrategicLandscapeProps) {
  const dismiss = (path: string) => {
    onUpdateField?.(path, "");
    onItemFeedback?.(path, "negative");
  };
  const editable = !!onUpdateField;
  const priorities = (intelligence.strategicPriorities ?? []).filter((p) =>
    p.priority?.trim()
  );
  const competitors = (intelligence.competitiveContext ?? []).filter(
    (c) => c.competitor?.trim() || c.context?.trim()
  );
  const marketItems = (intelligence.marketContext ?? []).filter(
    (m) => m.title?.trim() || m.body?.trim()
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
            {priorities.map((p, i) => {
              const status = priorityStatus(p.status);
              const isLastOdd =
                i === priorities.length - 1 && priorities.length % 2 === 1;
              const metaBits = [p.owner, p.timeline].filter(Boolean);
              const ctxText = p.context;
              const path = `strategicPriorities[${i}].priority`;
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
                    <button
                      type="button"
                      className={css.dismissButton}
                      onClick={() => dismiss(path)}
                      title="Dismiss (feeds back into AI)"
                    >
                      <X size={13} />
                    </button>
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
            {competitors.map((c, i) => {
              const dots = threatDotsCount(c.threatLevel);
              const label = threatLabel(c.threatLevel);
              const sourceBits: string[] = [];
              if (c.detectedAt) sourceBits.push(formatDate(c.detectedAt));
              if (c.source) sourceBits.push(c.source);
              const path = `competitiveContext[${i}].competitor`;
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
                    <button
                      type="button"
                      className={css.dismissButton}
                      onClick={() => dismiss(path)}
                      title="Dismiss (feeds back into AI)"
                    >
                      <X size={13} />
                    </button>
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
          {marketItems.map((m, i) => {
            const path = `marketContext[${i}].title`;
            return (
              <article key={i} className={css.regCard}>
                <div className={css.regHead}>
                  <div className={css.regTitle}>{m.title}</div>
                  {m.category && <span className={css.xrefPill}>{m.category}</span>}
                </div>
                {m.body && <p className={css.regBody}>{m.body}</p>}
                <ProvenanceTag itemSource={m.itemSource} discrepancy={m.discrepancy} />
                {editable && (
                  <button
                    type="button"
                    className={css.dismissButton}
                    onClick={() => dismiss(path)}
                    title="Dismiss (feeds back into AI)"
                  >
                    <X size={13} />
                  </button>
                )}
              </article>
            );
          })}
        </>
      )}
    </section>
  );
}
