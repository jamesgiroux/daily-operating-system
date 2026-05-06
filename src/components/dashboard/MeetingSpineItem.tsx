import clsx from "clsx";
import type { ComponentPropsWithoutRef, ReactNode } from "react";
import { Pill, type PillTone } from "@/components/ui/Pill";
import type { MeetingType } from "@/types";
import styles from "./MeetingSpineItem.module.css";

export type MeetingSpineState = "past" | "in-progress" | "upcoming" | "cancelled";
export type MeetingSpineType =
  | Extract<MeetingType, "customer" | "internal" | "one_on_one">
  | "partner"
  | "project";
export type MeetingSpinePrepState = "ready" | "building" | "needs" | "captured" | "none";

export interface MeetingSpineItemProps
  extends Omit<ComponentPropsWithoutRef<"article">, "title"> {
  time: ReactNode;
  duration?: ReactNode;
  state?: MeetingSpineState;
  type?: MeetingSpineType;
  warn?: boolean;
  entityName: ReactNode;
  title: ReactNode;
  context?: ReactNode;
  attendees?: ReactNode;
  prepState?: MeetingSpinePrepState;
  prepLabel?: ReactNode;
  briefingUrl?: string;
  briefingLabel?: ReactNode;
  createLabel?: ReactNode;
  onCreateBriefing?: () => void;
  statusLabel?: ReactNode;
  showStatus?: boolean;
}

const TYPE_CLASS: Record<MeetingSpineType, string> = {
  customer: styles.customer,
  internal: styles.internal,
  one_on_one: styles.oneOnOne,
  partner: styles.partner,
  project: styles.project,
};

const STATE_CLASS: Record<MeetingSpineState, string | undefined> = {
  past: styles.past,
  "in-progress": styles.inProgress,
  upcoming: undefined,
  cancelled: styles.cancelled,
};

const PREP_TONE: Record<MeetingSpinePrepState, PillTone> = {
  ready: "sage",
  building: "turmeric",
  needs: "terracotta",
  captured: "eucalyptus",
  none: "neutral",
};

const DEFAULT_PREP_LABEL: Record<MeetingSpinePrepState, string> = {
  ready: "Briefing fresh",
  building: "Briefing building",
  needs: "No briefing yet",
  captured: "Notes captured",
  none: "No prep",
};

function defaultStatusLabel(state: MeetingSpineState): ReactNode {
  if (state === "in-progress") return "Now";
  if (state === "upcoming") return "Up next";
  if (state === "past") return "Ended";
  return "Cancelled";
}

function renderTitle(title: ReactNode, href?: string) {
  if (!href) return <h3 className={styles.title}>{title}</h3>;
  return (
    <h3 className={styles.title}>
      <a className={styles.titleLink} href={href}>
        {title}
      </a>
    </h3>
  );
}

export function MeetingSpineItem({
  time,
  duration,
  state = "upcoming",
  type = "internal",
  warn = false,
  entityName,
  title,
  context,
  attendees,
  prepState = "none",
  prepLabel,
  briefingUrl,
  briefingLabel = "Read full briefing",
  createLabel = "Create briefing",
  onCreateBriefing,
  statusLabel,
  showStatus,
  className,
  ...rest
}: MeetingSpineItemProps) {
  const hasCreateAction = prepState === "needs" && onCreateBriefing;
  const resolvedShowStatus = showStatus ?? state === "in-progress";
  const hasFooter =
    attendees || prepState !== "none" || briefingUrl || hasCreateAction;

  return (
    <article
      className={clsx(
        styles.item,
        TYPE_CLASS[type],
        STATE_CLASS[state],
        className,
      )}
      data-ds-name="MeetingSpineItem"
      data-ds-tier="pattern"
      data-ds-spec="patterns/MeetingSpineItem.md"
      data-state={state}
      data-type={type}
      data-warn={warn ? "true" : undefined}
      {...rest}
    >
      <div className={styles.timeColumn}>
        <span className={styles.time}>{time}</span>
        {duration ? <span className={styles.duration}>{duration}</span> : null}
        {resolvedShowStatus ? (
          <span
            className={clsx(
              styles.stateTag,
              state === "in-progress" && styles.stateTagNow,
              state === "upcoming" && styles.stateTagUpcoming,
              state === "past" && styles.stateTagPast,
              state === "cancelled" && styles.stateTagCancelled,
            )}
          >
            {statusLabel ?? defaultStatusLabel(state)}
          </span>
        ) : null}
      </div>

      <div className={styles.body}>
        <div className={styles.eyebrow}>
          <span className={styles.glyph} aria-hidden="true" />
          <span className={styles.entityName}>{entityName}</span>
          <span className={styles.rule} aria-hidden="true" />
        </div>

        <div className={styles.titleRow}>
          {renderTitle(title, state === "cancelled" ? undefined : briefingUrl)}
        </div>

        {context ? <p className={styles.context}>{context}</p> : null}

        {hasFooter ? (
          <div className={styles.footer}>
            {attendees ? <span>{attendees}</span> : null}
            {attendees && (prepState !== "none" || briefingUrl || hasCreateAction) ? (
              <span className={styles.separator} aria-hidden="true" />
            ) : null}
            {prepState !== "none" ? (
              <Pill tone={PREP_TONE[prepState]} size="compact" dot>
                {prepLabel ?? DEFAULT_PREP_LABEL[prepState]}
              </Pill>
            ) : null}
            {briefingUrl && state !== "cancelled" ? (
              <a className={styles.briefingLink} href={briefingUrl}>
                {briefingLabel} {"\u2192"}
              </a>
            ) : null}
            {hasCreateAction ? (
              <button className={styles.createButton} type="button" onClick={onCreateBriefing}>
                {createLabel}
              </button>
            ) : null}
          </div>
        ) : null}
      </div>
    </article>
  );
}
