/**
 * ScoreBand — band-label primitive for entity-intelligence score rendering.
 *
 * Per DOS-325 voice rule (issue body §"What good looks like"):
 *   "Renders a plain-language band label. No raw number in the headline."
 *
 * Display-only. The caller decides which band to render; the primitive
 * itself has no knowledge of claims, trust factors, or substrate.
 * Editable variants + the EvidenceDrawer integration ship in v1.4.4
 * (see DOS-689 + DOS-690 + DOS-693).
 *
 * v1.4.3 W2 L0 Packet D §6.4 (DOS-682 + DOS-325).
 */
import styles from "./ScoreBand.module.css";

export type ScoreBandValue = "on-track" | "watching" | "action-needed" | "no-read";

export const SCORE_BAND_LABELS: Record<ScoreBandValue, string> = {
  "on-track": "On Track",
  watching: "Watching",
  "action-needed": "Action Needed",
  "no-read": "No Read",
};

const SCORE_BAND_CLASSES: Record<ScoreBandValue, string> = {
  "on-track": "scoreBandOnTrack",
  watching: "scoreBandWatching",
  "action-needed": "scoreBandActionNeeded",
  "no-read": "scoreBandNoRead",
};

export interface ScoreBandProps {
  value: ScoreBandValue;
  /** Optional label override; defaults to the canonical DOS-325 vocabulary. */
  label?: string;
}

export function ScoreBand({ value, label }: ScoreBandProps) {
  return (
    <span
      className={`${styles.scoreBand} ${styles[SCORE_BAND_CLASSES[value]]}`}
      data-ds-name="ScoreBand"
      data-ds-tier="primitive"
      data-ds-spec="primitives/ScoreBand.md"
    >
      {label ?? SCORE_BAND_LABELS[value]}
    </span>
  );
}
