import styles from "./MeetingRow.module.css";

/**
 * MeetingRow — Entity meeting row (grid layout with date, title, type badge).
 *
 * Used by TheWork.tsx for entity detail pages.
 * Timeline variant removed in WeekPage now uses MeetingCard.
 */

export interface MeetingRowProps {
  meeting: {
    id: string;
    title: string;
    startTime: string;
    meetingType: string;
  };
  formatDate?: (d: string) => string;
  formatType?: (t: string) => string;
  typeBadgeStyle?: (t: string) => React.CSSProperties;
}

function defaultFormatDate(d: string): string {
  try {
    const dt = new Date(d);
    return dt.toLocaleDateString(undefined, { weekday: "short", month: "short", day: "numeric" });
  } catch {
    return d;
  }
}

function defaultFormatType(t: string): string {
  return t.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
}

function defaultTypeBadgeStyle(): React.CSSProperties {
  return {
    fontFamily: "var(--font-mono)",
    fontSize: 10,
    fontWeight: 500,
    letterSpacing: "0.06em",
    textTransform: "uppercase",
    color: "var(--color-text-tertiary)",
  };
}

export function MeetingRow({
  meeting,
  formatDate = defaultFormatDate,
  formatType = defaultFormatType,
  typeBadgeStyle = defaultTypeBadgeStyle as (t: string) => React.CSSProperties,
}: MeetingRowProps) {
  return (
    <div className={styles.row}>
      <span className={styles.date}>{formatDate(meeting.startTime)}</span>
      <span className={styles.title}>{meeting.title}</span>
      <span className={styles.badgeWrap}>
        <span style={typeBadgeStyle(meeting.meetingType)}>
          {formatType(meeting.meetingType)}
        </span>
      </span>
    </div>
  );
}
