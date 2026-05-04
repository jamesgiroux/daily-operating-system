import clsx from "clsx";
import styles from "./SourceCoverageLine.module.css";

export interface SourceCoverageLineProps {
  sourceLabel?: string | null;
  sourceCount?: number | null;
  staleCount?: number;
  emptyLabel?: string;
  className?: string;
}

function pluralize(count: number, singular: string): string {
  return `${count} ${singular}${count === 1 ? "" : "s"}`;
}

export function SourceCoverageLine({
  sourceLabel,
  sourceCount,
  staleCount = 0,
  emptyLabel = "No source coverage",
  className,
}: SourceCoverageLineProps) {
  const hasCoverage = Boolean(sourceLabel) && typeof sourceCount === "number" && sourceCount > 0;
  const variant = hasCoverage ? (staleCount > 0 ? "withStaleCount" : "default") : "empty";

  if (!hasCoverage) {
    return (
      <span
        className={clsx(styles.line, className)}
        data-variant={variant}
        data-ds-name="SourceCoverageLine"
        data-ds-spec="primitives/SourceCoverageLine.md"
      >
        {emptyLabel}
      </span>
    );
  }

  return (
    <span
      className={clsx(styles.line, className)}
      data-variant={variant}
      data-ds-name="SourceCoverageLine"
      data-ds-spec="primitives/SourceCoverageLine.md"
    >
      <span>{sourceLabel}</span>
      <span className={styles.separator} aria-hidden="true">
        ·
      </span>
      <span>{pluralize(sourceCount, "source")}</span>
      {staleCount > 0 && (
        <>
          <span className={styles.separator} aria-hidden="true">
            ·
          </span>
          <span className={styles.stale}>{staleCount} stale</span>
        </>
      )}
    </span>
  );
}
