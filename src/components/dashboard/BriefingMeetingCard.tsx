/**
 * BriefingMeetingCard.tsx — Editorial schedule row with inline expansion
 *
 * A compact row in the schedule section. Click to expand prep details inline
 * (not navigate). Past meetings navigate to detail page on click.
 *
 * Also exports shared sub-components (KeyPeopleFlow, PrepGrid,
 * MeetingActionChecklist) used by both expansion panels and the lead story.
 *
 * Temporal states:
 * - Upcoming with prep: expandable, click toggles inline panel
 * - In Progress: auto-expanded, NOW pill, warm glow background
 * - Past: muted, click navigates to meeting detail page
 * - Cancelled: line-through, no interaction
 */

import { useState, useRef, useLayoutEffect, useCallback } from "react";
import { Link, useNavigate } from "@tanstack/react-router";
import clsx from "clsx";
import { stripMarkdown, formatMeetingType } from "@/lib/utils";
import { formatEntityByline } from "@/lib/entity-helpers";
import { IntelligenceQualityBadge } from "@/components/entity/IntelligenceQualityBadge";
import type { Meeting, CalendarEvent, Action, Stakeholder, CalendarAttendee } from "@/types";
import s from "@/styles/editorial-briefing.module.css";

// ─── Types ───────────────────────────────────────────────────────────────────

interface BriefingMeetingCardProps {
  meeting: Meeting;
  now: number;
  currentMeeting?: CalendarEvent;
  /** Actions related to this meeting (for "Before this meeting" checklist) */
  meetingActions?: Action[];
  /** Completion callback for meeting actions */
  onComplete?: (id: string) => void;
  /** Set of completed action IDs (optimistic UI) */
  completedIds?: Set<string>;
  /** Callback when entities change (parent should refetch) */
  onEntitiesChanged?: () => void;
  /** Number of total actions captured from this meeting (for past meetings) */
  capturedActionCount?: number;
  /** Number of proposed actions needing review from this meeting */
  proposedActionCount?: number;
  /** When true, this is the next upcoming meeting — renders expanded by default with richer context */
  isUpNext?: boolean;
  /** User's org domain for internal/external attendee grouping */
  userDomain?: string;
}

type TemporalState = "upcoming" | "in-progress" | "past" | "cancelled";

// ─── Helpers ─────────────────────────────────────────────────────────────────

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

function getExpansionTintClass(meeting: Meeting): string {
  switch (meeting.type) {
    case "customer":
    case "qbr":
    case "partnership":
    case "external":
      return s.expansionTintTurmeric;
    case "personal":
      return s.expansionTintSage;
    case "one_on_one":
      return s.expansionTintLarkspur;
    default:
      return "";
  }
}

// ─── Shared Sub-Components ───────────────────────────────────────────────────
// Exported for use in DailyBriefing lead story section.

/** "The Room" — calendar invitees grouped by side (their/our).
 *  Uses raw Google Calendar attendee data, not AI-enriched stakeholders. */
export function KeyPeopleFlow({
  attendees,
  userDomain,
  stakeholders,
}: {
  attendees?: CalendarAttendee[];
  userDomain?: string;
  /** Fallback: AI-enriched stakeholders if calendar attendees unavailable */
  stakeholders?: Stakeholder[];
}) {
  // Use calendar attendees when available, fall back to prep stakeholders
  if (attendees && attendees.length > 0 && userDomain) {
    const external = attendees.filter((a) => a.domain !== userDomain);
    const internal = attendees.filter((a) => a.domain === userDomain);
    const hasBothSides = external.length > 0 && internal.length > 0;

    const renderAttendee = (a: CalendarAttendee) => (
      <div key={a.email} className={s.theRoomPerson}>
        <span className={s.theRoomName}>{a.name}</span>
        {a.domain && a.domain !== userDomain && (
          <span className={s.theRoomCompany}>{a.domain}</span>
        )}
        {a.rsvp === "tentative" && (
          <span className={s.theRoomRole} style={{ fontStyle: "italic" }}>tentative</span>
        )}
      </div>
    );

    return (
      <div className={s.theRoom}>
        <div className={s.theRoomLabel}>The Room</div>
        {hasBothSides ? (
          <>
            <div className={s.theRoomGroup}>
              <div className={s.theRoomGroupLabel}>Their Side</div>
              {external.map(renderAttendee)}
            </div>
            <div className={s.theRoomGroup}>
              <div className={s.theRoomGroupLabel}>Our Side</div>
              {internal.map(renderAttendee)}
            </div>
          </>
        ) : (
          <div className={s.theRoomGroup}>
            {attendees.map(renderAttendee)}
          </div>
        )}
      </div>
    );
  }

  // Fallback: prep stakeholders (legacy)
  if (!stakeholders || stakeholders.length === 0) return null;

  const external = stakeholders.filter((p) => p.relationship === "external" || p.relationship === "unknown" || !p.relationship);
  const internal = stakeholders.filter((p) => p.relationship === "internal");
  const hasBothSides = external.length > 0 && internal.length > 0;

  const renderPerson = (person: Stakeholder) => (
    <div key={person.name} className={s.theRoomPerson}>
      <span className={s.theRoomName}>{person.name}</span>
      {person.role && (
        <>
          <span className={s.theRoomSep}>&middot;</span>
          <span className={s.theRoomRole}>{person.role}</span>
        </>
      )}
    </div>
  );

  return (
    <div className={s.theRoom}>
      <div className={s.theRoomLabel}>The Room</div>
      {hasBothSides ? (
        <>
          <div className={s.theRoomGroup}>
            <div className={s.theRoomGroupLabel}>Their Side</div>
            {external.map(renderPerson)}
          </div>
          <div className={s.theRoomGroup}>
            <div className={s.theRoomGroupLabel}>Our Side</div>
            {internal.map(renderPerson)}
          </div>
        </>
      ) : (
        <div className={s.theRoomGroup}>
          {stakeholders.map(renderPerson)}
        </div>
      )}
    </div>
  );
}

/** 2-column prep grid: Discuss, Watch, Wins, At a Glance. */
export function PrepGrid({ meeting }: { meeting: Meeting }) {
  const prep = meeting.prep;
  if (!prep) return null;

  const discuss = prep.actions ?? prep.questions ?? [];
  const watch = prep.risks ?? [];
  const wins = prep.wins ?? [];

  const hasSections = discuss.length > 0 || watch.length > 0 || wins.length > 0;
  if (!hasSections) return null;

  return (
    <div className={s.prepGrid}>
      {discuss.length > 0 && (
        <div className={s.prepSection}>
          <div className={clsx(s.prepLabel, s.prepLabelDiscuss)}>Discuss</div>
          {discuss.slice(0, 1).map((item, i) => (
            <div key={i} className={s.prepItem}>
              <span className={clsx(s.prepDot, s.prepDotTurmeric)} />
              <span>{stripMarkdown(item)}</span>
            </div>
          ))}
        </div>
      )}

      {watch.length > 0 && (
        <div className={s.prepSection}>
          <div className={clsx(s.prepLabel, s.prepLabelWatch)}>Watch</div>
          {watch.slice(0, 1).map((item, i) => (
            <div key={i} className={s.prepItem}>
              <span className={clsx(s.prepDot, s.prepDotTerracotta)} />
              <span>{stripMarkdown(item)}</span>
            </div>
          ))}
        </div>
      )}

      {wins.length > 0 && (
        <div className={s.prepSection}>
          <div className={clsx(s.prepLabel, s.prepLabelWins)}>Wins</div>
          {wins.slice(0, 1).map((item, i) => (
            <div key={i} className={s.prepItem}>
              <span className={clsx(s.prepDot, s.prepDotSage)} />
              <span>{stripMarkdown(item)}</span>
            </div>
          ))}
        </div>
      )}

    </div>
  );
}

/** "Before this meeting" action checklist with completion circles. */
export function MeetingActionChecklist({
  actions,
  completedIds,
  onComplete,
}: {
  actions: Action[];
  completedIds?: Set<string>;
  onComplete?: (id: string) => void;
}) {
  if (actions.length === 0) return null;

  return (
    <div className={s.meetingActions}>
      <div className={s.meetingActionsLabel}>Before this meeting</div>
      {actions.slice(0, 3).map((action) => {
        const done = action.status === "completed" || completedIds?.has(action.id);
        return (
          <div
            key={action.id}
            className={clsx(s.meetingActionsItem, done && s.meetingActionsItemCompleted)}
          >
            <button
              className={clsx(
                s.meetingActionsCheck,
                done && s.meetingActionsCheckChecked,
                action.isOverdue && !done && s.meetingActionsCheckOverdue,
              )}
              onClick={(e) => {
                e.stopPropagation();
                if (!done && onComplete) onComplete(action.id);
              }}
              disabled={done}
            >
              {done && (
                <svg width="8" height="8" viewBox="0 0 12 12" fill="none">
                  <path d="M2.5 6L5 8.5L9.5 4" stroke="#fff" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
                </svg>
              )}
            </button>
            <div>
              <div className={s.meetingActionsText}>{stripMarkdown(action.title)}</div>
              {(action.isOverdue || action.dueDate || action.account) && (
                <div className={clsx(s.meetingActionsContext, action.isOverdue && s.meetingActionsContextOverdue)}>
                  {action.isOverdue && action.daysOverdue
                    ? `${action.daysOverdue} day${action.daysOverdue !== 1 ? "s" : ""} overdue`
                    : action.dueDate ?? ""}
                  {action.account && ` \u00B7 ${action.account}`}
                </div>
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
}

// ─── Main Component ──────────────────────────────────────────────────────────

export function BriefingMeetingCard({
  meeting,
  now,
  currentMeeting,
  meetingActions: _meetingActions = [],
  onComplete: _onComplete,
  completedIds: _completedIds,
  onEntitiesChanged: _onEntitiesChanged,
  capturedActionCount,
  proposedActionCount,
  isUpNext = false,
  userDomain,
}: BriefingMeetingCardProps) {
  const navigate = useNavigate();
  const state = getTemporalState(meeting, now, currentMeeting);
  const isInitiallyExpanded = state === "in-progress" || isUpNext;
  const [isExpanded, setIsExpanded] = useState(isInitiallyExpanded);
  const innerRef = useRef<HTMLDivElement>(null);
  const [measuredHeight, setMeasuredHeight] = useState<number>(isInitiallyExpanded ? 2000 : 0);

  const duration = formatDuration(meeting);
  const hasPrepContent = !!(meeting.prep && Object.keys(meeting.prep).length > 0);
  const canExpand = (state === "upcoming" || state === "in-progress") && (hasPrepContent || isUpNext);

  // Measure expansion panel content
  useLayoutEffect(() => {
    if (isExpanded && innerRef.current) {
      setMeasuredHeight(innerRef.current.scrollHeight);
    } else if (!isExpanded) {
      setMeasuredHeight(0);
    }
  }, [isExpanded]);

  const handleRowClick = useCallback(() => {
    if (state === "past") {
      navigate({ to: "/meeting/$meetingId", params: { meetingId: meeting.id } });
    } else if (canExpand) {
      setIsExpanded((prev) => !prev);
    }
  }, [state, canExpand, navigate, meeting.id]);

  const accentClass = getAccentCssClass(meeting);
  const tintClass = getExpansionTintClass(meeting);

  // ── Cancelled ──
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

  // ── Schedule Row (all non-cancelled states) ──
  const attendeeCount = meeting.calendarAttendees?.length ?? meeting.prep?.stakeholders?.length;
  const subtitleParts: string[] = [];
  subtitleParts.push(formatEntityByline(meeting.linkedEntities) ?? formatMeetingType(meeting.type));
  if (attendeeCount && attendeeCount > 0) {
    subtitleParts.push(`${attendeeCount} attendee${attendeeCount !== 1 ? "s" : ""}`);
  }

  return (
    <>
      {/* Collapsed row */}
      <div
        className={clsx(
          s.scheduleRow,
          accentClass,
          state === "in-progress" && s.scheduleRowActive,
          state === "past" && s.scheduleRowPast,
          state === "past" && s.scheduleRowPastNavigate,
          canExpand && s.scheduleRowExpandable,
          isExpanded && s.scheduleRowExpanded,
        )}
        onClick={handleRowClick}
      >
        <div className={s.scheduleTime}>
          {meeting.time}
          {duration && <span className={s.scheduleTimeDuration}>{duration}</span>}
        </div>
        <div className={s.scheduleContent}>
          <div className={s.scheduleTitleRow}>
            <span className={s.scheduleTitle}>{meeting.title}</span>
            {isUpNext && state !== "in-progress" && <span className={s.upNextPill}>UP NEXT</span>}
            {state === "in-progress" && <span className={s.nowPill}>NOW</span>}
            {state === "past" && <span className={s.pastArrow}>&rarr;</span>}
            {canExpand && (
              <span className={s.expandHint}>
                {isExpanded ? "collapse" : "expand"}
              </span>
            )}
          </div>
          <div className={s.scheduleSubtitle}>
            {subtitleParts.join(" \u00B7 ")}
            {meeting.intelligenceQuality ? (
              <IntelligenceQualityBadge
                quality={meeting.intelligenceQuality}
                enrichedAt={meeting.hasPrep ? meeting.intelligenceQuality.lastEnriched : undefined}
              />
            ) : meeting.hasPrep ? (
              <span className={s.schedulePrepDot} title="Prep available" />
            ) : isUpNext ? (
              <span className={s.schedulePrepDotMuted} title="No prep yet" />
            ) : null}
          </div>
          {state === "past" && capturedActionCount != null && capturedActionCount > 0 && (
            <div
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 11,
                color: "var(--color-text-tertiary)",
                marginTop: 2,
              }}
            >
              {capturedActionCount} action{capturedActionCount !== 1 ? "s" : ""} captured
              {proposedActionCount != null && proposedActionCount > 0 && (
                <span style={{ color: "var(--color-spice-turmeric)" }}>
                  {" \u00B7 "}{proposedActionCount} needs review
                </span>
              )}
            </div>
          )}
        </div>
      </div>

      {/* Expansion panel */}
      {canExpand && (
        <div
          className={clsx(s.expansionPanel, tintClass, isExpanded && s.expansionPanelOpen)}
          style={{ maxHeight: isExpanded ? measuredHeight : 0 }}
        >
          <div ref={innerRef} className={s.expansionInner}>
            {/* Meeting context: calendar description (organizer's words) or AI brief */}
            {meeting.calendarDescription ? (
              <p
                style={{
                  fontFamily: "var(--font-body)",
                  fontSize: 14,
                  fontWeight: 400,
                  color: "var(--color-text-primary)",
                  margin: "0 0 20px 0",
                  lineHeight: 1.55,
                  maxWidth: 560,
                }}
              >
                {meeting.calendarDescription.length > 400
                  ? `${meeting.calendarDescription.slice(0, 400)}…`
                  : meeting.calendarDescription}
              </p>
            ) : meeting.prep?.context ? (
              <p
                style={{
                  fontFamily: "var(--font-serif)",
                  fontSize: 15,
                  fontWeight: 300,
                  fontStyle: "italic",
                  color: "var(--color-text-secondary)",
                  margin: "0 0 20px 0",
                  lineHeight: 1.55,
                  maxWidth: 560,
                }}
              >
                {meeting.prep.context.length > 320
                  ? `${meeting.prep.context.slice(0, 320)}…`
                  : meeting.prep.context}
              </p>
            ) : null}

            {/* The Room — calendar invitees grouped by side */}
            {(meeting.calendarAttendees?.length || meeting.prep?.stakeholders?.length) ? (
              <KeyPeopleFlow
                attendees={meeting.calendarAttendees}
                userDomain={userDomain}
                stakeholders={meeting.prep?.stakeholders}
              />
            ) : null}

            {/* Bridge link */}
            <div className={s.meetingLinks}>
              <Link
                to="/meeting/$meetingId"
                params={{ meetingId: meeting.id }}
                className={s.meetingLinkPrimary}
                onClick={(e) => e.stopPropagation()}
              >
                Read full briefing &rarr;
              </Link>
              <button
                className={s.expansionCollapse}
                onClick={(e) => {
                  e.stopPropagation();
                  setIsExpanded(false);
                }}
              >
                Collapse
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
