import clsx from "clsx";
import type { ComponentPropsWithoutRef, ReactNode } from "react";
import { Link } from "@tanstack/react-router";
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
  /** Local meeting id. When present (and not cancelled) the title and the
   *  briefing affordance link into the meeting detail surface via the same
   *  `/meeting/$meetingId` route the legacy card used. */
  meetingId?: string;
  briefingLabel?: ReactNode;
  createLabel?: ReactNode;
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

function renderTitle(title: ReactNode, meetingId?: string) {
  if (!meetingId) return <h3 className={styles.title}>{title}</h3>;
  return (
    <h3 className={styles.title}>
      <Link
        className={styles.titleLink}
        to="/meeting/$meetingId"
        params={{ meetingId }}
      >
        {title}
      </Link>
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
  meetingId,
  briefingLabel = "Read full briefing",
  createLabel = "Create briefing",
  statusLabel,
  showStatus,
  className,
  ...rest
}: MeetingSpineItemProps) {
  const resolvedShowStatus = showStatus ?? state === "in-progress";
  const hasPrepPill = prepState !== "none" || Boolean(prepLabel);
  const canOpen = Boolean(meetingId) && state !== "cancelled";
  const hasFooter = attendees || hasPrepPill || canOpen;

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
          {renderTitle(title, canOpen ? meetingId : undefined)}
        </div>

        {context ? <p className={styles.context}>{context}</p> : null}

        {hasFooter ? (
          <div className={styles.footer}>
            {attendees ? <span>{attendees}</span> : null}
            {attendees && (hasPrepPill || canOpen) ? (
              <span className={styles.separator} aria-hidden="true" />
            ) : null}
            {hasPrepPill ? (
              <Pill tone={PREP_TONE[prepState]} size="compact" dot>
                {prepLabel ?? DEFAULT_PREP_LABEL[prepState]}
              </Pill>
            ) : null}
            {meetingId && state !== "cancelled" ? (
              state === "past" ? (
                <Link
                  className={styles.briefingLink}
                  to="/meeting/$meetingId"
                  params={{ meetingId }}
                >
                  Notes &amp; actions {"\u2192"}
                </Link>
              ) : prepState === "needs" ? (
                <Link
                  className={styles.createButton}
                  to="/meeting/$meetingId"
                  params={{ meetingId }}
                >
                  {createLabel}
                </Link>
              ) : (
                <Link
                  className={styles.briefingLink}
                  to="/meeting/$meetingId"
                  params={{ meetingId }}
                >
                  {briefingLabel} {"\u2192"}
                </Link>
              )
            ) : null}
          </div>
        ) : null}
      </div>
    </article>
  );
}
