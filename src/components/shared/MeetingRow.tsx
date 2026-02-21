/**
 * MeetingRow — Entity meeting row (grid layout with date, title, type badge).
 *
 * Used by TheWork.tsx for entity detail pages.
 * Timeline variant removed in I364 — WeekPage now uses MeetingCard.
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
    <div
      style={{
        display: "grid",
        gridTemplateColumns: "90px 1fr auto",
        gap: 16,
        padding: "14px 0",
        borderBottom: "1px solid var(--color-rule-light)",
        alignItems: "baseline",
      }}
    >
      <span
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 12,
          fontWeight: 500,
          color: "var(--color-text-primary)",
          whiteSpace: "nowrap",
        }}
      >
        {formatDate(meeting.startTime)}
      </span>
      <span
        style={{
          fontFamily: "var(--font-sans)",
          fontSize: 14,
          fontWeight: 400,
          color: "var(--color-text-primary)",
        }}
      >
        {meeting.title}
      </span>
      <span style={{ display: "flex", gap: 8, alignItems: "baseline" }}>
        <span style={typeBadgeStyle(meeting.meetingType)}>
          {formatType(meeting.meetingType)}
        </span>
      </span>
    </div>
  );
}
