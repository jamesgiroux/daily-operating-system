/**
 * BriefingMeetingCard.tsx — Editorial meeting card for the daily briefing
 *
 * Float card with entity accent bar, temporal state awareness, and expandable
 * prep details. Collapsed: accent + time + title + meta + prep summary.
 * Expanded: full prep grid (At a Glance, Discuss, Watch, Wins), entity chips.
 *
 * Temporal states:
 * - Upcoming: full display, click to expand prep
 * - In Progress: gold accent, NOW pill, expanded by default
 * - Past: collapsed, faded, summary only, click navigates to meeting detail
 * - Cancelled: line-through, faded, no prep
 */

import { useState, useMemo } from "react";
import { Link, useNavigate } from "@tanstack/react-router";
import { ChevronDown } from "lucide-react";
import { stripMarkdown, formatMeetingType } from "@/lib/utils";
import type { Meeting, MeetingType, CalendarEvent } from "@/types";

// ─── Types ───────────────────────────────────────────────────────────────────

interface BriefingMeetingCardProps {
  meeting: Meeting;
  now: number;
  currentMeeting?: CalendarEvent;
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

function getTemporalState(meeting: Meeting, now: number, currentMeeting?: CalendarEvent): TemporalState {
  if (meeting.overlayStatus === "cancelled") return "cancelled";
  // Check against live calendar
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

function getAccentColor(meeting: Meeting, state: TemporalState): string {
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

function formatDuration(meeting: Meeting): string | null {
  const start = getMeetingStartMs(meeting);
  const end = getMeetingEndMs(meeting);
  if (!start || !end || end <= start) return null;
  const mins = Math.round((end - start) / 60000);
  if (mins < 60) return `${mins}m`;
  const hrs = Math.floor(mins / 60);
  const rem = mins % 60;
  return rem > 0 ? `${hrs}h ${rem}m` : `${hrs}h`;
}

const EXTERNAL_TYPES: MeetingType[] = ["customer", "qbr", "partnership", "external"];

// ─── Component ───────────────────────────────────────────────────────────────

export function BriefingMeetingCard({ meeting, now, currentMeeting }: BriefingMeetingCardProps) {
  const navigate = useNavigate();
  const state = useMemo(() => getTemporalState(meeting, now, currentMeeting), [meeting, now, currentMeeting]);

  // In-progress meetings expand by default
  const [isExpanded, setIsExpanded] = useState(state === "in-progress");

  const accentColor = getAccentColor(meeting, state);
  const duration = formatDuration(meeting);
  const isExternal = EXTERNAL_TYPES.includes(meeting.type);
  const hasPrepContent = !!(meeting.prep && Object.keys(meeting.prep).length > 0);
  const canExpand = state === "upcoming" && hasPrepContent;

  // Past meetings navigate on click
  const handleCardClick = () => {
    if (state === "past") {
      navigate({ to: "/meeting/$meetingId", params: { meetingId: meeting.id } });
    } else if (canExpand) {
      setIsExpanded(!isExpanded);
    }
  };

  // ── Cancelled ──
  if (state === "cancelled") {
    return (
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 12,
          padding: "10px 16px",
          opacity: 0.4,
        }}
      >
        <span
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 15,
            fontWeight: 500,
            color: "var(--color-text-tertiary)",
            width: 72,
            textAlign: "right",
            flexShrink: 0,
          }}
        >
          {meeting.time}
        </span>
        <div style={{ width: 1, height: 20, background: "var(--color-rule-light)" }} />
        <span
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 18,
            color: "var(--color-text-tertiary)",
            textDecoration: "line-through",
          }}
        >
          {meeting.title}
        </span>
      </div>
    );
  }

  // ── Past ──
  if (state === "past") {
    return (
      <div
        onClick={handleCardClick}
        style={{
          display: "flex",
          alignItems: "center",
          gap: 12,
          padding: "10px 16px",
          opacity: 0.5,
          cursor: "pointer",
          borderRadius: 8,
          transition: "background 0.15s ease",
        }}
        onMouseEnter={(e) => (e.currentTarget.style.background = "rgba(30, 37, 48, 0.03)")}
        onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
      >
        <span
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 15,
            fontWeight: 500,
            color: "var(--color-text-tertiary)",
            width: 72,
            textAlign: "right",
            flexShrink: 0,
          }}
        >
          {meeting.time}
        </span>
        <div style={{ width: 1, height: 20, background: "var(--color-rule-light)" }} />
        <span
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 18,
            color: "var(--color-text-tertiary)",
            flex: 1,
          }}
        >
          {meeting.title}
        </span>
        <span
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            color: "var(--color-garden-sage)",
          }}
        >
          &#10003;
        </span>
      </div>
    );
  }

  // ── Upcoming / In-Progress ──
  return (
    <div
      style={{
        background: state === "in-progress" ? "rgba(201, 162, 39, 0.04)" : "#fff",
        borderRadius: 16,
        borderLeft: `4px solid ${accentColor}`,
        boxShadow: "0 1px 3px rgba(26,31,36,0.04), 0 8px 24px rgba(26,31,36,0.06)",
        overflow: "hidden",
        transition: "box-shadow 0.15s ease",
      }}
    >
      {/* Main row */}
      <div
        onClick={handleCardClick}
        style={{
          display: "flex",
          gap: 16,
          padding: "20px 24px",
          cursor: canExpand ? "pointer" : "default",
        }}
      >
        {/* Time column */}
        <div
          style={{
            width: 72,
            flexShrink: 0,
            textAlign: "right",
            paddingTop: 2,
          }}
        >
          <div
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 15,
              fontWeight: 500,
              color: "var(--color-text-primary)",
            }}
          >
            {meeting.time}
          </div>
          {duration && (
            <div
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 12,
                color: "var(--color-text-tertiary)",
                marginTop: 2,
              }}
            >
              {duration}
            </div>
          )}
        </div>

        {/* Divider */}
        <div style={{ width: 1, background: "var(--color-rule-light)", flexShrink: 0 }} />

        {/* Content */}
        <div style={{ flex: 1, minWidth: 0 }}>
          {/* Title row */}
          <div style={{ display: "flex", alignItems: "flex-start", justifyContent: "space-between", gap: 12 }}>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <Link
                  to="/meeting/$meetingId"
                  params={{ meetingId: meeting.id }}
                  onClick={(e) => e.stopPropagation()}
                  style={{
                    fontFamily: "var(--font-serif)",
                    fontSize: 20,
                    fontWeight: 400,
                    color: "var(--color-text-primary)",
                    textDecoration: "none",
                    lineHeight: 1.3,
                  }}
                >
                  {meeting.title}
                </Link>
                {state === "in-progress" && (
                  <span
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 10,
                      fontWeight: 700,
                      letterSpacing: "0.06em",
                      padding: "2px 8px",
                      borderRadius: 4,
                      background: "rgba(201, 162, 39, 0.15)",
                      color: "var(--color-spice-turmeric)",
                      flexShrink: 0,
                    }}
                  >
                    NOW
                  </span>
                )}
              </div>

              {/* Meta line */}
              <div
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 14,
                  color: "var(--color-text-tertiary)",
                  marginTop: 4,
                }}
              >
                {meeting.account && <span>{meeting.account}</span>}
                {!meeting.account && (
                  <span>{formatMeetingType(meeting.type)}</span>
                )}
                {meeting.prep?.stakeholders && meeting.prep.stakeholders.length > 0 && (
                  <span>
                    {" \u00B7 "}
                    {meeting.prep.stakeholders.map((s) => s.name).slice(0, 2).join(", ")}
                    {meeting.prep.stakeholders.length > 2 &&
                      ` +${meeting.prep.stakeholders.length - 2}`}
                  </span>
                )}
              </div>

              {/* Prep summary for external meetings */}
              {isExternal && meeting.prep?.context && !isExpanded && (
                <p
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 14,
                    fontWeight: 300,
                    lineHeight: 1.5,
                    color: "var(--color-text-secondary)",
                    marginTop: 8,
                    display: "-webkit-box",
                    WebkitLineClamp: 2,
                    WebkitBoxOrient: "vertical",
                    overflow: "hidden",
                  }}
                >
                  {meeting.prep.context}
                </p>
              )}
            </div>

            {/* Expand toggle */}
            {canExpand && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  setIsExpanded(!isExpanded);
                }}
                style={{
                  background: "none",
                  border: "none",
                  cursor: "pointer",
                  padding: 4,
                  color: "var(--color-text-tertiary)",
                  transition: "transform 0.2s ease",
                  transform: isExpanded ? "rotate(180deg)" : "rotate(0deg)",
                  flexShrink: 0,
                }}
              >
                <ChevronDown size={16} />
              </button>
            )}
          </div>
        </div>
      </div>

      {/* Expanded prep details */}
      {isExpanded && hasPrepContent && (
        <div
          style={{
            borderTop: "1px solid var(--color-rule-light)",
            padding: "20px 24px 20px 112px", // align with content (72 + 16 + 24)
          }}
        >
          <ExpandedPrep meeting={meeting} />

          {/* Entity chips */}
          {meeting.linkedEntities && meeting.linkedEntities.length > 0 && (
            <div style={{ display: "flex", flexWrap: "wrap", gap: 6, marginTop: 16 }}>
              {meeting.linkedEntities.map((entity) => (
                <Link
                  key={entity.id}
                  to={entity.entityType === "project" ? "/projects/$projectId" : "/accounts/$accountId"}
                  params={entity.entityType === "project"
                    ? { projectId: entity.id }
                    : { accountId: entity.id }}
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 12,
                    padding: "3px 10px",
                    borderRadius: 6,
                    background: "var(--color-paper-linen)",
                    color: "var(--color-text-secondary)",
                    textDecoration: "none",
                    transition: "background 0.15s ease",
                  }}
                >
                  {entity.name}
                </Link>
              ))}
            </div>
          )}

          {/* View full meeting link */}
          <div style={{ marginTop: 16 }}>
            <Link
              to="/meeting/$meetingId"
              params={{ meetingId: meeting.id }}
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 13,
                fontWeight: 500,
                color: "var(--color-spice-turmeric)",
                textDecoration: "none",
              }}
            >
              View meeting details &rarr;
            </Link>
          </div>
        </div>
      )}
    </div>
  );
}

// ─── Expanded Prep Content ───────────────────────────────────────────────────

function ExpandedPrep({ meeting }: { meeting: Meeting }) {
  const prep = meeting.prep;
  if (!prep) return null;

  const atAGlance = prep.metrics?.slice(0, 4) ?? [];
  const discuss = prep.actions ?? prep.questions ?? [];
  const watch = prep.risks ?? [];
  const wins = prep.wins ?? [];

  const sections: { title: string; items: string[]; color: string }[] = [];
  if (atAGlance.length > 0) sections.push({ title: "At a Glance", items: atAGlance, color: "var(--color-text-primary)" });
  if (discuss.length > 0) sections.push({ title: "Discuss", items: discuss.slice(0, 4), color: "var(--color-spice-turmeric)" });
  if (watch.length > 0) sections.push({ title: "Watch", items: watch.slice(0, 3), color: "var(--color-spice-terracotta)" });
  if (wins.length > 0) sections.push({ title: "Wins", items: wins.slice(0, 3), color: "var(--color-garden-sage)" });

  if (sections.length === 0 && prep.context) {
    return (
      <p
        style={{
          fontFamily: "var(--font-sans)",
          fontSize: 14,
          fontWeight: 300,
          lineHeight: 1.6,
          color: "var(--color-text-secondary)",
        }}
      >
        {prep.context}
      </p>
    );
  }

  return (
    <div>
      {prep.context && (
        <p
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 14,
            fontWeight: 300,
            lineHeight: 1.6,
            color: "var(--color-text-secondary)",
            marginBottom: 16,
          }}
        >
          {prep.context}
        </p>
      )}
      <div
        style={{
          display: "grid",
          gridTemplateColumns: sections.length > 1 ? "1fr 1fr" : "1fr",
          gap: 20,
        }}
      >
        {sections.map((section) => (
          <div key={section.title}>
            <div
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 12,
                fontWeight: 600,
                color: section.color,
                marginBottom: 8,
                textTransform: "uppercase",
                letterSpacing: "0.04em",
              }}
            >
              {section.title}
            </div>
            <ul style={{ margin: 0, padding: 0, listStyle: "none" }}>
              {section.items.map((item, i) => (
                <li
                  key={i}
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 13,
                    lineHeight: 1.5,
                    color: "var(--color-text-secondary)",
                    padding: "3px 0",
                    display: "flex",
                    alignItems: "baseline",
                    gap: 8,
                  }}
                >
                  <span
                    style={{
                      width: 5,
                      height: 5,
                      borderRadius: "50%",
                      background: section.color,
                      flexShrink: 0,
                      marginTop: 6,
                    }}
                  />
                  <span>{stripMarkdown(item)}</span>
                </li>
              ))}
            </ul>
          </div>
        ))}
      </div>
    </div>
  );
}
