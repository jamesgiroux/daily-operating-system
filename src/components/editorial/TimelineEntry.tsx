/**
 * TimelineEntry — vertical timeline with positioned dots + type badges.
 * Mockup: vertical line on left, 9px colored dots, date + type badge, title, detail text.
 */
import { Link } from "@tanstack/react-router";
import { ClaimTextRenderer } from "@/components/ui/ClaimTextRenderer";
import type { RenderableClaimText } from "@/types";
import styles from "./TimelineEntry.module.css";

export type TimelineEntryType = "meeting" | "email" | "capture" | "event" | "value" | "risk" | "decision" | "context";
export type TimelineEntryText = string | RenderableClaimText | React.ReactNode;

interface TimelineEntryProps {
  date: string;
  type: TimelineEntryType;
  title: TimelineEntryText;
  subtitle?: TimelineEntryText;
  subtitleSuffix?: React.ReactNode;
  claimSurface?: string;
  linkTo?: string;
  linkParams?: Record<string, string>;
}

const dotClass: Record<TimelineEntryType, string> = {
  meeting: styles.dotMeeting,
  email: styles.dotEmail,
  capture: styles.dotCapture,
  event: styles.dotEvent,
  value: styles.dotValue,
  risk: styles.dotRisk,
  decision: styles.dotDecision,
  context: styles.dotContext,
};

const badgeClass: Record<TimelineEntryType, string> = {
  meeting: styles.typeMeeting,
  email: styles.typeEmail,
  capture: styles.typeCapture,
  event: styles.typeEvent,
  value: styles.typeValue,
  risk: styles.typeRisk,
  decision: styles.typeDecision,
  context: styles.typeContext,
};

const typeLabels: Record<TimelineEntryType, string> = {
  meeting: "Meeting",
  email: "Email",
  capture: "Capture",
  event: "Lifecycle",
  value: "Value",
  risk: "Risk",
  decision: "Decision",
  context: "Note",
};

export function TimelineEntry({
  date,
  type,
  title,
  subtitle,
  subtitleSuffix,
  claimSurface,
  linkTo,
  linkParams,
}: TimelineEntryProps) {
  const content = (
    <div className={styles.entry}>
      <div className={`${styles.dot} ${dotClass[type]}`} />
      <div className={styles.dateLine}>
        <span className={styles.date}>{date}</span>
        <span className={`${styles.typeBadge} ${badgeClass[type]}`}>{typeLabels[type]}</span>
      </div>
      <div className={styles.title}>{renderTimelineText(title, claimSurface)}</div>
      {hasTimelineText(subtitle) && (
        <div className={styles.detail}>
          {renderTimelineText(subtitle, claimSurface)}
          {subtitleSuffix}
        </div>
      )}
    </div>
  );

  if (linkTo && linkParams) {
    return (
      <Link to={linkTo} params={linkParams} className={styles.entryLink}>
        {content}
      </Link>
    );
  }

  return content;
}

/** Container component that wraps timeline entries with the vertical line */
export function TimelineContainer({ children }: { children: React.ReactNode }) {
  return <div className={styles.timeline}>{children}</div>;
}

function isRenderableClaimText(value: TimelineEntryText): value is RenderableClaimText {
  return Boolean(
    value
      && typeof value === "object"
      && "text" in value
      && "policy" in value,
  );
}

function renderTimelineText(value: TimelineEntryText, surface?: string) {
  if (isRenderableClaimText(value)) {
    return <ClaimTextRenderer value={value} surface={surface} />;
  }

  return value;
}

function hasTimelineText(value: TimelineEntryText | undefined): value is TimelineEntryText {
  return value !== undefined && value !== null && value !== "";
}
