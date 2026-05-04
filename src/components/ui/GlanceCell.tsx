import clsx from "clsx";
import type { ComponentPropsWithoutRef, ReactNode } from "react";
import styles from "./GlanceCell.module.css";

export type GlanceCellStatus = "none" | "healthy" | "warn" | "error" | "neutral";

export interface GlanceCellProps extends ComponentPropsWithoutRef<"div"> {
  label: ReactNode;
  value: ReactNode;
  status?: GlanceCellStatus;
}

export function GlanceCell({
  label,
  value,
  status = "none",
  className,
  ...rest
}: GlanceCellProps) {
  const hasStatus = status !== "none";

  return (
    <div
      className={clsx(styles.cell, className)}
      data-status={status}
      data-ds-name="GlanceCell"
      data-ds-spec="primitives/GlanceCell.md"
      {...rest}
    >
      <span className={styles.label}>{label}</span>
      <span className={styles.valueRow}>
        {hasStatus ? (
          <span className={clsx(styles.dot, styles[status])} aria-hidden="true" />
        ) : null}
        <span className={styles.value}>{value}</span>
      </span>
    </div>
  );
}
