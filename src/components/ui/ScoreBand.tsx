/**
 * ScoreBand — band-label primitive for entity-intelligence score rendering.
 *
 * Band labels per voice rule (see ADR-0083 product vocabulary):
 *   "Renders a plain-language band label. No raw number in the headline."
 *
 * Display-only. The caller decides which band to render; the primitive
 * itself has no knowledge of claims, trust factors, or substrate.
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
  /** Optional label override; defaults to the canonical product vocabulary (ADR-0083). */
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
