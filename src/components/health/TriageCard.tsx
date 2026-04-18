/**
 * TriageCard — single "Needs attention" row (DOS-203).
 *
 * Layout per `.docs/mockups/account-health-outlook-globex.html`:
 *   [spine] [body: kind · headline · evidence · sources] [actions]
 *
 * Spine colour + kind label drive the emotional register. Sources render as
 * discrete tags (Local vs Glean). Evidence accepts a string with `<strong>`
 * passthrough via ReactNode so callers can emphasise dates/figures.
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

export interface TriageCardProps {
  tone: TriageTone;
  kind: string;
  headline: string;
  evidence?: ReactNode;
  sources?: TriageSource[];
  /** Optional action row; omit for read-only cards. */
  actions?: ReactNode;
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

export function TriageCard({ tone, kind, headline, evidence, sources, actions }: TriageCardProps) {
  return (
    <article className={styles.triageCard}>
      <div className={`${styles.triageSpine} ${spineClass[tone]}`} />
      <div className={styles.triageBody}>
        <span className={`${styles.triageKind} ${kindClass[tone]}`}>{kind}</span>
        <div className={styles.triageHeadline}>{headline}</div>
        {evidence ? <div className={styles.triageEvidence}>{evidence}</div> : null}
        {sources && sources.length > 0 ? (
          <div className={styles.triageSources}>
            {sources.map((s, i) => (
              <span key={i} style={{ display: "inline-flex", alignItems: "center", gap: 8 }}>
                <span
                  className={s.origin === "glean" ? styles.sourceTagGlean : styles.sourceTagLocal}
                >
                  {s.origin === "glean" ? "Glean" : "Local"}
                </span>
                {s.label ? <span>{s.label}</span> : null}
              </span>
            ))}
          </div>
        ) : null}
      </div>
      {actions ? <div>{actions}</div> : <div />}
    </article>
  );
}
