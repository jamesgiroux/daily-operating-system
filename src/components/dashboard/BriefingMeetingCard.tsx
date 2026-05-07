/**
 * BriefingMeetingCard.tsx - Editorial schedule row.
 *
 * A compact row in the legacy daily briefing schedule. Non-cancelled
 * meetings navigate to the routed meeting detail page.
 */

import clsx from "clsx";
import { formatMeetingType } from "@/lib/utils";
import { formatEntityByline } from "@/lib/entity-helpers";
import { MeetingCard } from "@/components/shared/MeetingCard";
import { Pill } from "@/components/ui/Pill";
import type { Meeting, CalendarEvent, Action } from "@/types";
import s from "@/styles/editorial-briefing.module.css";

export {
  buildLegacyPrepGridItems,
  normalizePrepGridText,
  parsePrepGridItem,
  partitionLegacyPrepGrid,
} from "@/components/meeting/meeting-prep-utils";

interface BriefingMeetingCardProps {
  meeting: Meeting;
  now: number;
  currentMeeting?: CalendarEvent;
  meetingActions?: Action[];
  onComplete?: (id: string) => void;
  completedIds?: Set<string>;
  onEntitiesChanged?: () => void;
  capturedActionCount?: number;
  suggestedActionCount?: number;
  isUpNext?: boolean;
  userDomain?: string;
}

type TemporalState = "upcoming" | "in-progress" | "past" | "cancelled";

function parseDisplayTimeMs(timeStr: string | undefined): number | null {
  if (!timeStr) return null;
  const match = timeStr.match(/^(\d{1,2}):(\d{2})\s*(AM|PM)$/i);
  if (!match) return null;
  let hours = parseInt(match[1], 10);
  const minutes = parseInt(match[2], 10);
  const period = match[3].toUpperCase();
  if (period === "PM" && hours !== 12) hours += 12;
  if (period === "AM" && hours === 12) hours = 0;
  const d = new Date();
  d.setHours(hours, minutes, 0, 0);
  return d.getTime();
}

function getMeetingEndMs(meeting: Meeting): number | null {
  return parseDisplayTimeMs(meeting.endTime) ?? parseDisplayTimeMs(meeting.time);
}

function getMeetingStartMs(meeting: Meeting): number | null {
  return parseDisplayTimeMs(meeting.time);
}

export function getTemporalState(meeting: Meeting, now: number, currentMeeting?: CalendarEvent): TemporalState {
  if (meeting.overlayStatus === "cancelled") return "cancelled";
  if (currentMeeting) {
    if (meeting.calendarEventId && meeting.calendarEventId === currentMeeting.id) return "in-progress";
    if (meeting.title === currentMeeting.title || meeting.id === currentMeeting.id) return "in-progress";
  }
  const startMs = getMeetingStartMs(meeting);
  const endMs = getMeetingEndMs(meeting);
  if (startMs && endMs && startMs <= now && now < endMs) return "in-progress";
  if (endMs && now > endMs) return "past";
  return "upcoming";
}

export function getAccentColor(meeting: Meeting, state: TemporalState): string {
  if (state === "in-progress") return "var(--color-spice-turmeric)";
  switch (meeting.type) {
    case "customer":
    case "qbr":
    case "partnership":
    case "external":
      return "var(--color-spice-turmeric)";
    case "personal":
      return "var(--color-garden-sage)";
    case "internal":
    case "team_sync":
    case "one_on_one":
      return "var(--color-paper-linen)";
    default:
      return "var(--color-text-tertiary)";
  }
}

export function formatDuration(meeting: Meeting): string | null {
  const start = getMeetingStartMs(meeting);
  const end = getMeetingEndMs(meeting);
  if (!start || !end || end <= start) return null;
  const mins = Math.round((end - start) / 60000);
  if (mins < 60) return `${mins}m`;
  const hrs = Math.floor(mins / 60);
  const rem = mins % 60;
  return rem > 0 ? `${hrs}h ${rem}m` : `${hrs}h`;
}

function getAccentCssClass(meeting: Meeting): string {
  switch (meeting.type) {
    case "customer":
    case "qbr":
    case "partnership":
    case "external":
      return s.scheduleRowCustomer;
    case "personal":
      return s.scheduleRowPersonal;
    case "one_on_one":
      return s.scheduleRowLarkspur;
    case "internal":
    case "team_sync":
    case "all_hands":
      return s.scheduleRowInternal;
    default:
      return "";
  }
}

export function BriefingMeetingCard({
  meeting,
  now,
  currentMeeting,
  capturedActionCount,
  suggestedActionCount,
  isUpNext = false,
}: BriefingMeetingCardProps) {
  const state = getTemporalState(meeting, now, currentMeeting);
  const duration = formatDuration(meeting);
  const accentClass = getAccentCssClass(meeting);

  if (state === "cancelled") {
    return (
      <div className={clsx(s.scheduleRow, s.scheduleRowCancelled, accentClass)}>
        <div className={s.scheduleTime}>
          {meeting.time}
          {duration && <span className={s.scheduleTimeDuration}>{duration}</span>}
        </div>
        <div className={s.scheduleContent}>
          <span className={s.scheduleTitle}>{meeting.title}</span>
          <div className={s.scheduleSubtitle}>
            {formatEntityByline(meeting.linkedEntities) ?? formatMeetingType(meeting.type)} &middot; Cancelled
          </div>
        </div>
      </div>
    );
  }

  const attendeeCount = meeting.calendarAttendees?.length ?? meeting.prep?.stakeholders?.length;
  const subtitleParts: string[] = [
    formatEntityByline(meeting.linkedEntities) ?? formatMeetingType(meeting.type),
  ];
  if (attendeeCount && attendeeCount > 0) {
    subtitleParts.push(`${attendeeCount} attendee${attendeeCount !== 1 ? "s" : ""}`);
  }

  return (
    <MeetingCard
      id={meeting.id}
      title={meeting.title}
      displayTime={meeting.time}
      duration={duration ?? undefined}
      meetingType={meeting.type}
      entityByline={subtitleParts.join(" \u00B7 ")}
      intelligenceQuality={meeting.intelligenceQuality ?? undefined}
      temporalState={state}
      showNavigationHint={state === "past"}
      className={clsx(
        s.briefingCardOverride,
        state === "in-progress" && s.briefingCardOverrideActive,
      )}
      titleExtra={(
        <>
          {isUpNext && state !== "in-progress" && (
            <Pill tone="sage" size="compact">
              UP NEXT
            </Pill>
          )}
        </>
      )}
    >
      {state === "past" && capturedActionCount != null && capturedActionCount > 0 && (
        <div className={s.capturedSummary}>
          {capturedActionCount} action{capturedActionCount !== 1 ? "s" : ""} captured
          {suggestedActionCount != null && suggestedActionCount > 0 && (
            <span className={s.capturedSummaryReview}>
              {" \u00B7 "}{suggestedActionCount} needs review
            </span>
          )}
        </div>
      )}
    </MeetingCard>
  );
}
