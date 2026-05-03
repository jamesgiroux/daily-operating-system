import clsx from "clsx";
import type { ComponentPropsWithoutRef, ReactNode } from "react";
import { Pill, type PillSize, type PillTone } from "@/components/ui/Pill";
import styles from "./MeetingStatusPill.module.css";

export type MeetingStatusState = "upcoming" | "in-progress" | "past" | "cancelled";

const STATUS_META: Record<
  MeetingStatusState,
  { label: string; tone: PillTone; className: string }
> = {
  upcoming: {
    label: "Upcoming",
    tone: "sage",
    className: styles.upcoming,
  },
  "in-progress": {
    label: "In progress",
    tone: "turmeric",
    className: styles.inProgress,
  },
  past: {
    label: "Past",
    tone: "neutral",
    className: styles.past,
  },
  cancelled: {
    label: "Cancelled",
    tone: "terracotta",
    className: styles.cancelled,
  },
};

export interface MeetingStatusPillProps
  extends Omit<ComponentPropsWithoutRef<"span">, "children"> {
  state: MeetingStatusState;
  size?: PillSize;
  children?: ReactNode;
}

export function MeetingStatusPill({
  state,
  size = "standard",
  children,
  className,
  ...rest
}: MeetingStatusPillProps) {
  const meta = STATUS_META[state];

  return (
    <Pill
      tone={meta.tone}
      dot
      size={size}
      className={clsx(styles.status, meta.className, className)}
      data-state={state}
      data-ds-name="MeetingStatusPill"
      data-ds-spec="primitives/MeetingStatusPill.md"
      {...rest}
    >
      {children ?? meta.label}
    </Pill>
  );
}
