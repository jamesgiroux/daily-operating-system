/**
 * Shared MeetingRow — renders a meeting in a list context.
 *
 * Consolidates duplicate implementations from:
 * - WeekPage.tsx TimelineMeetingRow (timeline: entity label, quality badge, outcomes)
 * - TheWork.tsx (entity: grid layout with date, title, type badge)
 *
 * BriefingMeetingCard is intentionally complex and NOT included here.
 *
 * ADR-0084 C3.
 */
import { Link } from "@tanstack/react-router";

interface MeetingRowTimelineProps {
  variant: "timeline";
  meeting: {
    id: string;
    title: string;
    entities?: Array<{ name: string }>;
    intelligenceQuality?: {
      level: string;
      hasNewSignals?: boolean;
      lastEnriched?: string | null;
    } | null;
    hasOutcomes?: boolean;
    outcomeSummary?: string | null;
    hasNewSignals?: boolean;
    priorMeetingId?: string | null;
  };
  isPast?: boolean;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  QualityBadge?: React.ComponentType<{ quality: any; showLabel?: boolean }>;
}

interface MeetingRowEntityProps {
  variant: "entity";
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

export type MeetingRowProps = MeetingRowTimelineProps | MeetingRowEntityProps;

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

/** Timeline variant: used in WeekPage timeline */
function TimelineMeetingRow({
  meeting,
  isPast,
  QualityBadge,
}: MeetingRowTimelineProps) {
  const entityLabel =
    meeting.entities && meeting.entities.length > 0
      ? meeting.entities.map((e) => e.name).join(", ")
      : undefined;

  const quality = meeting.intelligenceQuality
    ? {
        level: meeting.intelligenceQuality.level,
        hasNewSignals: meeting.intelligenceQuality.hasNewSignals,
        lastEnriched: meeting.intelligenceQuality.lastEnriched,
      }
    : undefined;

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 10,
        padding: "4px 0",
      }}
    >
      <Link
        to="/meeting/$meetingId"
        params={{ meetingId: meeting.id }}
        style={{
          fontFamily: "var(--font-sans)",
          fontSize: 14,
          color: "var(--color-text-primary)",
          textDecoration: "none",
          minWidth: 0,
          overflow: "hidden",
          textOverflow: "ellipsis",
          whiteSpace: "nowrap",
        }}
      >
        {meeting.title}
      </Link>

      {entityLabel && (
        <span
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 12,
            color: "var(--color-text-tertiary)",
            flexShrink: 0,
          }}
        >
          {entityLabel}
        </span>
      )}

      <span style={{ flex: 1 }} />

      {quality && QualityBadge && <QualityBadge quality={quality} showLabel />}

      {isPast && meeting.hasOutcomes && (
        <span
          style={{
            display: "inline-flex",
            alignItems: "center",
            gap: 4,
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            color: "var(--color-garden-sage)",
            flexShrink: 0,
          }}
          title={meeting.outcomeSummary || "Outcomes captured"}
        >
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <polyline points="20 6 9 17 4 12" />
          </svg>
          {meeting.outcomeSummary ? (
            <span style={{ maxWidth: 180, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
              {meeting.outcomeSummary}
            </span>
          ) : (
            "captured"
          )}
        </span>
      )}

      {!isPast && meeting.hasNewSignals && (
        <span
          style={{
            width: 6,
            height: 6,
            borderRadius: "50%",
            background: "var(--color-garden-larkspur)",
            flexShrink: 0,
          }}
          title="New signals available"
        />
      )}

      {!isPast && meeting.priorMeetingId && (
        <Link
          to="/meeting/$meetingId"
          params={{ meetingId: meeting.priorMeetingId }}
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            color: "var(--color-garden-larkspur)",
            textDecoration: "none",
            flexShrink: 0,
            whiteSpace: "nowrap",
          }}
        >
          Review last meeting &rarr;
        </Link>
      )}
    </div>
  );
}

/** Entity variant: grid row with date, title, type badge (TheWork style) */
function EntityMeetingRow({
  meeting,
  formatDate = defaultFormatDate,
  formatType = defaultFormatType,
  typeBadgeStyle = defaultTypeBadgeStyle as (t: string) => React.CSSProperties,
}: MeetingRowEntityProps) {
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

export function MeetingRow(props: MeetingRowProps) {
  if (props.variant === "timeline") return <TimelineMeetingRow {...props} />;
  return <EntityMeetingRow {...props} />;
}
