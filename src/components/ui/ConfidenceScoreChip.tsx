import clsx from "clsx";
import styles from "./ConfidenceScoreChip.module.css";

export type ConfidenceBand =
  | "likely_current"
  | "use_with_caution"
  | "needs_verification";

export interface ConfidenceScoreChipProps {
  score: number | null | undefined;
  format?: "percent";
  className?: string;
}

export function deriveConfidenceBand(score: number): ConfidenceBand {
  if (score >= 0.85) return "likely_current";
  if (score >= 0.6) return "use_with_caution";
  return "needs_verification";
}

function normalizeScore(score: number): number {
  const normalized = score > 1 ? score / 100 : score;
  return Math.min(1, Math.max(0, normalized));
}

function formatPercent(score: number): string {
  return `${Math.round(normalizeScore(score) * 100)}%`;
}

export function ConfidenceScoreChip({
  score,
  format = "percent",
  className,
}: ConfidenceScoreChipProps) {
  const hasScore = typeof score === "number" && Number.isFinite(score);
  const normalizedScore = hasScore ? normalizeScore(score) : null;
  const band = normalizedScore === null ? undefined : deriveConfidenceBand(normalizedScore);
  const label =
    normalizedScore === null
      ? "--"
      : format === "percent"
        ? formatPercent(normalizedScore)
        : String(normalizedScore);

  return (
    <span
      className={clsx(styles.chip, className)}
      data-band={band}
      data-state={normalizedScore === null ? "unavailable" : "available"}
      data-ds-name="ConfidenceScoreChip"
      data-ds-spec="primitives/ConfidenceScoreChip.md"
    >
      {label}
    </span>
  );
}
