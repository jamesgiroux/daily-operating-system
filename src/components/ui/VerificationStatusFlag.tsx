import clsx from "clsx";
import { CircleCheck, TriangleAlert, Wrench } from "lucide-react";
import styles from "./VerificationStatusFlag.module.css";

export type VerificationStatus = "ok" | "corrected" | "flagged";

export interface VerificationStatusFlagProps {
  status: VerificationStatus;
  label?: string;
  className?: string;
}

const STATUS_LABEL: Record<VerificationStatus, string> = {
  ok: "OK",
  corrected: "Corrected",
  flagged: "Flagged",
};

function VerificationIcon({ status }: { status: VerificationStatus }) {
  const iconProps = {
    className: styles.icon,
    "aria-hidden": true,
  } as const;

  switch (status) {
    case "corrected":
      return <Wrench {...iconProps} />;
    case "flagged":
      return <TriangleAlert {...iconProps} />;
    case "ok":
    default:
      return <CircleCheck {...iconProps} />;
  }
}

export function VerificationStatusFlag({
  status,
  label = STATUS_LABEL[status],
  className,
}: VerificationStatusFlagProps) {
  return (
    <span
      className={clsx(styles.flag, className)}
      data-status={status}
      data-ds-name="VerificationStatusFlag"
      data-ds-spec="primitives/VerificationStatusFlag.md"
    >
      <VerificationIcon status={status} />
      {label}
    </span>
  );
}
