import clsx from "clsx";
import { CircleAlert, Info } from "lucide-react";
import styles from "./DataGapNotice.module.css";

export type DataGapSeverity = "info" | "warning";

export interface DataGapNoticeProps {
  message: string;
  severity?: DataGapSeverity;
  className?: string;
}

export function DataGapNotice({
  message,
  severity = "info",
  className,
}: DataGapNoticeProps) {
  const label = message.trim();
  if (!label) return null;

  const Icon = severity === "warning" ? CircleAlert : Info;

  return (
    <span
      className={clsx(styles.notice, className)}
      data-severity={severity}
      data-ds-name="DataGapNotice"
      data-ds-spec="primitives/DataGapNotice.md"
    >
      <Icon className={styles.icon} aria-hidden="true" />
      {label}
    </span>
  );
}
