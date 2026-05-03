/**
 * TriageCard — single "Needs attention" row.
 *
 * Layout per `.docs/mockups/account-health-outlook-globex.html` lines 649-671:
 *   [spine] [body: kind · headline · evidence · sources] [actions + feedback]
 *
 * The spine colour and kind label drive the emotional register; the action
 * column carries primary + optional secondary CTAs; the feedback slot sits
 * beneath the actions so every card can be thumbed-up / annotated / corrected
 * individually — Intelligence Loop per-card signal attribution.
 *
 * `evidence` accepts `ReactNode` so callers can emphasise dates/figures via
 * `<strong>`. `citations` render as discrete dated links (or plain labels)
 * next to the coloured source tag (Activity vs Glean). The `origin` code
 * identifier stays `"local" | "glean"` per ADR-0083 — only the user-visible
 * label renders as "Activity" (warm, specific, self-evident — the data came
 * from the user's own emails/meetings/renewals, not from Glean).
 */
import type { ReactNode } from "react";
import styles from "./health.module.css";

export type TriageTone = "urgent" | "soon" | "gap" | "divergence" | "meta";
export type TriageSourceOrigin = "local" | "glean";

export interface TriageSource {
  /** Origin tag — renders as a small coloured pill. */
  origin: TriageSourceOrigin;
  /** Short label (e.g. "Transcript · Feb 17"). Rendered as plain text beside the tag. */
  label?: string;
}

export interface TriageCitation {
  /** Dated link label (e.g. "Transcript · Feb 17"). */
  label: string;
  /** Optional href. When omitted, renders as plain text (no underline). */
  href?: string;
}

export interface TriageAction {
  /** Button label. */
  label: string;
  /** Primary action gets filled/dark styling (only one per card). */
  primary?: boolean;
  /** Optional click handler. */
  onClick?: () => void;
}

export interface TriageCardProps {
  tone: TriageTone;
  kind: string;
  headline: string;
  evidence?: ReactNode;
  /** Source-origin pills (Local / Glean) shown first. */
  sources?: TriageSource[];
  /** Dated citation links or plain labels, rendered alongside source tags. */
  citations?: TriageCitation[];
  /** Optional primary + secondary pill buttons. */
  actions?: TriageAction[];
  /** Optional per-card feedback slot (binary validation prompt). */
  feedbackSlot?: ReactNode;
}

const spineClass: Record<TriageTone, string> = {
  urgent: styles.spineUrgent,
  soon: styles.spineSoon,
  gap: styles.spineGap,
  divergence: styles.spineDivergence,
  meta: styles.spineMeta,
};
const kindClass: Record<TriageTone, string> = {
  urgent: styles.kindUrgent,
  soon: styles.kindSoon,
  gap: styles.kindGap,
  divergence: styles.kindDivergence,
  meta: styles.kindMeta,
};

export function TriageCard({
  tone,
  kind,
  headline,
  evidence,
  sources,
  citations,
  actions,
  feedbackSlot,
}: TriageCardProps) {
  const hasSources = (sources && sources.length > 0) || (citations && citations.length > 0);
  const hasActionColumn = (actions && actions.length > 0) || !!feedbackSlot;

  return (
    <article className={styles.triageCard}>
      <div className={`${styles.triageSpine} ${spineClass[tone]}`} />
      <div className={styles.triageBody}>
        <span className={`${styles.triageKind} ${kindClass[tone]}`}>{kind}</span>
        <div className={styles.triageHeadline}>{headline}</div>
        {evidence ? <div className={styles.triageEvidence}>{evidence}</div> : null}
        {hasSources ? (
          <div className={styles.triageSources}>
            {sources?.map((s, i) => (
              <span key={`src-${i}`} className={styles.sourceTagCluster}>
                <span
                  className={s.origin === "glean" ? styles.sourceTagGlean : styles.sourceTagLocal}
                >
                  {s.origin === "glean" ? "Glean" : "Activity"}
                </span>
                {s.label ? <span>{s.label}</span> : null}
              </span>
            ))}
            {citations?.map((c, i) =>
              c.href ? (
                <a key={`cite-${i}`} className={styles.triageCitation} href={c.href}>
                  {c.label}
                </a>
              ) : (
                <span key={`cite-${i}`} className={styles.triageCitationPlain}>
                  {c.label}
                </span>
              ),
            )}
          </div>
        ) : null}
      </div>
      {hasActionColumn ? (
        <div className={styles.triageActions}>
          {actions && actions.length > 0 ? (
            <div className={styles.triageActionsRow}>
              {actions.map((a, i) => (
                <button
                  key={`act-${i}`}
                  type="button"
                  className={`${styles.triageBtn} ${a.primary ? styles.triageBtnPrimary : ""}`}
                  onClick={a.onClick}
                >
                  {a.label}
                </button>
              ))}
            </div>
          ) : null}
          {feedbackSlot ? <div className={styles.triageFeedback}>{feedbackSlot}</div> : null}
        </div>
      ) : (
        <div />
      )}
    </article>
  );
}
