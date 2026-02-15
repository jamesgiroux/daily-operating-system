/**
 * TimelineEntry — vertical timeline with positioned dots + type badges.
 * Mockup: vertical line on left, 9px colored dots, date + type badge, title, detail text.
 */
import { Link } from "@tanstack/react-router";
import styles from "./TimelineEntry.module.css";

export type TimelineEntryType = "meeting" | "email" | "capture" | "event" | "value" | "risk" | "decision";

interface TimelineEntryProps {
  date: string;
  type: TimelineEntryType;
  title: string;
  subtitle?: string;
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
};

const badgeClass: Record<TimelineEntryType, string> = {
  meeting: styles.typeMeeting,
  email: styles.typeEmail,
  capture: styles.typeCapture,
  event: styles.typeEvent,
  value: styles.typeValue,
  risk: styles.typeRisk,
  decision: styles.typeDecision,
};

const typeLabels: Record<TimelineEntryType, string> = {
  meeting: "Meeting",
  email: "Email",
  capture: "Capture",
  event: "Lifecycle",
  value: "Value",
  risk: "Risk",
  decision: "Decision",
};

export function TimelineEntry({ date, type, title, subtitle, linkTo, linkParams }: TimelineEntryProps) {
  const content = (
    <div className={styles.entry}>
      <div className={`${styles.dot} ${dotClass[type]}`} />
      <div className={styles.dateLine}>
        <span className={styles.date}>{date}</span>
        <span className={`${styles.typeBadge} ${badgeClass[type]}`}>{typeLabels[type]}</span>
      </div>
      <div className={styles.title}>{title}</div>
      {subtitle && <div className={styles.detail}>{subtitle}</div>}
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
