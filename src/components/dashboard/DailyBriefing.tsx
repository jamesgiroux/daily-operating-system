/**
 * DailyBriefing.tsx — Editorial daily briefing page
 *
 * A morning document, not a dashboard. You read it top to bottom.
 * When you reach the end, you're briefed.
 *
 * Sections: Hero > Focus > Featured Meeting > Schedule > Priorities (or Loose Threads fallback) > End Mark
 */

import { useState, useCallback, useMemo } from "react";
import { Link } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { useCalendar } from "@/hooks/useCalendar";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import type { ReadinessStat } from "@/components/layout/FolioBar";
import { BriefingMeetingCard } from "./BriefingMeetingCard";
import { formatDayTime, stripMarkdown } from "@/lib/utils";
import type { DashboardData, DataFreshness, Meeting, MeetingType, Action, Email, PrioritizedAction } from "@/types";

// ─── Types ───────────────────────────────────────────────────────────────────

interface DailyBriefingProps {
  data: DashboardData;
  freshness: DataFreshness;
}

// ─── Featured Meeting Selection ──────────────────────────────────────────────

const MEETING_TYPE_WEIGHTS: Partial<Record<MeetingType, number>> = {
  qbr: 100,
  customer: 80,
  partnership: 60,
  external: 40,
  training: 20,
};

/** Parse a display time like "10:30 AM" to epoch ms (today). */
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

type TemporalState = "upcoming" | "in-progress" | "past" | "cancelled";

function getTemporalState(meeting: Meeting, now: number): TemporalState {
  if (meeting.overlayStatus === "cancelled") return "cancelled";
  const startMs = getMeetingStartMs(meeting);
  const endMs = getMeetingEndMs(meeting);
  if (startMs && endMs && startMs <= now && now < endMs) return "in-progress";
  if (endMs && now > endMs) return "past";
  return "upcoming";
}

export function selectFeaturedMeeting(meetings: Meeting[], now: number): Meeting | null {
  const candidates = meetings.filter((m) => {
    const state = getTemporalState(m, now);
    if (state === "past" || state === "cancelled") return false;
    // Must be external-flavored and have prep
    const isExternal = ["customer", "qbr", "partnership", "external"].includes(m.type);
    return isExternal && m.hasPrep;
  });

  if (candidates.length === 0) return null;

  return candidates.sort((a, b) => {
    const wa = MEETING_TYPE_WEIGHTS[a.type] ?? 0;
    const wb = MEETING_TYPE_WEIGHTS[b.type] ?? 0;
    if (wa !== wb) return wb - wa;
    // Tiebreak: nearest in time
    const ta = getMeetingStartMs(a) ?? Infinity;
    const tb = getMeetingStartMs(b) ?? Infinity;
    return ta - tb;
  })[0];
}

// ─── Readiness Computation ───────────────────────────────────────────────────

function computeReadiness(meetings: Meeting[], actions: Action[]) {
  const externalMeetings = meetings.filter((m) =>
    ["customer", "qbr", "partnership", "external"].includes(m.type) &&
    m.overlayStatus !== "cancelled"
  );
  const preppedCount = externalMeetings.filter((m) => m.hasPrep).length;
  const totalExternal = externalMeetings.length;
  const overdueActions = actions.filter((a) => a.isOverdue && a.status !== "completed");
  return { preppedCount, totalExternal, overdueCount: overdueActions.length };
}

// ─── Duration Formatting ─────────────────────────────────────────────────────

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

// ─── Accent Color Mapping ────────────────────────────────────────────────────

function getMeetingAccentColor(meeting: Meeting, state: TemporalState): string {
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

// ─── Capacity Formatting ─────────────────────────────────────────────────────

function formatMinutes(minutes: number): string {
  if (minutes < 60) return `${minutes}m`;
  const hrs = Math.floor(minutes / 60);
  const rem = minutes % 60;
  return rem > 0 ? `${hrs}h ${rem}m` : `${hrs}h`;
}

// ─── Component ───────────────────────────────────────────────────────────────

export function DailyBriefing({ data, freshness }: DailyBriefingProps) {
  const { now, currentMeeting } = useCalendar();
  const [completedIds, setCompletedIds] = useState<Set<string>>(new Set());

  // Data
  const meetings = data.meetings;
  const actions = data.actions;
  const emails = data.emails ?? [];
  const highPriorityEmails = emails.filter((e) => e.priority === "high").slice(0, 3);

  // Featured meeting
  const featured = selectFeaturedMeeting(meetings, now);
  const scheduleMeetings = meetings.filter((m) => m.id !== featured?.id);

  // Readiness
  const readiness = computeReadiness(meetings, actions);

  // Date formatting
  const formattedDate = new Date().toLocaleDateString("en-US", {
    weekday: "long",
    month: "long",
    day: "numeric",
    year: "numeric",
  }).toUpperCase();

  // Register magazine shell with folio bar date + readiness
  const folioReadinessStats = useMemo(() => {
    const stats: ReadinessStat[] = [];
    if (readiness.totalExternal > 0) {
      stats.push({ label: `${readiness.preppedCount}/${readiness.totalExternal} prepped`, color: "sage" });
    }
    if (readiness.overdueCount > 0) {
      stats.push({ label: `${readiness.overdueCount} overdue`, color: "terracotta" });
    }
    return stats;
  }, [readiness.preppedCount, readiness.totalExternal, readiness.overdueCount]);

  const shellConfig = useMemo(
    () => ({
      folioLabel: "Daily Briefing",
      atmosphereColor: "turmeric" as const,
      activePage: "today" as const,
      folioDateText: formattedDate,
      folioReadinessStats: folioReadinessStats,
    }),
    [formattedDate, folioReadinessStats],
  );
  useRegisterMagazineShell(shellConfig);

  // Pending actions (sorted by urgency)
  const pendingActions = actions
    .filter((a) => a.status !== "completed" && !completedIds.has(a.id))
    .sort((a, b) => {
      // Overdue first, then by priority
      if (a.isOverdue && !b.isOverdue) return -1;
      if (!a.isOverdue && b.isOverdue) return 1;
      const priorityOrder = { P1: 0, P2: 1, P3: 2 };
      return (priorityOrder[a.priority] ?? 2) - (priorityOrder[b.priority] ?? 2);
    });

  const looseThreadsCount = Math.min(pendingActions.length, 5) + highPriorityEmails.length;
  const visibleActions = pendingActions.slice(0, 5);
  const hasMoreActions = pendingActions.length > 5;

  // Action completion
  const handleComplete = useCallback((id: string) => {
    setCompletedIds((prev) => new Set(prev).add(id));
    invoke("complete_action", { id }).catch(() => {});
  }, []);

  // Meeting actions relevant to featured meeting
  const featuredActions = featured
    ? actions.filter((a) => a.source && a.source === featured.id && a.status !== "completed")
    : [];

  // Are there any non-cancelled meetings at all?
  const activeMeetings = meetings.filter((m) => m.overlayStatus !== "cancelled");
  const hasSchedule = scheduleMeetings.some((m) => m.overlayStatus !== "cancelled");

  return (
    <div style={{ maxWidth: 900, marginLeft: "auto", marginRight: "auto" }}>
      {/* ═══ HERO ═══ */}
      <section style={{ paddingTop: 80, paddingBottom: 48 }}>
        {/* Headline */}
        <h1
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 44,
            fontWeight: 400,
            letterSpacing: "-0.02em",
            lineHeight: 1.15,
            color: "var(--color-text-primary)",
            margin: 0,
          }}
        >
          {data.overview.summary || (activeMeetings.length === 0
            ? "A clear day. Nothing needs you."
            : "Your day is ready.")}
        </h1>

        {/* Staleness indicator */}
        {freshness.freshness === "stale" && (
          <div
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              color: "var(--color-text-tertiary)",
              marginTop: 12,
            }}
          >
            Last updated {formatDayTime(freshness.generatedAt)}
          </div>
        )}
      </section>

      {/* ═══ FOCUS ═══ */}
      {data.overview.focus && (
        <div
          style={{
            display: "block",
            borderLeft: "3px solid var(--color-spice-turmeric)",
            background: "rgba(201, 162, 39, 0.08)",
            borderRadius: 16,
            padding: "20px 24px",
            marginBottom: 48,
          }}
        >
          <div
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              fontWeight: 600,
              letterSpacing: "0.08em",
              color: "var(--color-spice-turmeric)",
              marginBottom: 8,
              textTransform: "uppercase",
            }}
          >
            Today's Focus
          </div>
          <div
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 20,
              fontWeight: 400,
              lineHeight: 1.5,
              color: "var(--color-text-primary)",
            }}
          >
            {data.overview.focus}
          </div>
          {data.focus && (
            <div
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 12,
                color: "var(--color-text-tertiary)",
                marginTop: 10,
              }}
            >
              {formatMinutes(data.focus.availableMinutes)} available
              {data.focus.availableBlocks.filter((b) => b.durationMinutes >= 60).length > 0 && (
                <> &middot; {data.focus.availableBlocks.filter((b) => b.durationMinutes >= 60).length} deep work block{data.focus.availableBlocks.filter((b) => b.durationMinutes >= 60).length !== 1 ? "s" : ""}</>
              )}
              {" "}&middot; {data.focus.meetingCount} meeting{data.focus.meetingCount !== 1 ? "s" : ""}
            </div>
          )}
        </div>
      )}

      {/* ═══ FEATURED MEETING (Lead Story) ═══ */}
      {featured && (
        <section style={{ marginBottom: 48 }}>
          <div
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 22,
              fontWeight: 400,
              letterSpacing: "-0.01em",
              color: "var(--color-text-primary)",
              marginBottom: 20,
            }}
          >
            The Meeting
          </div>

          <div
            style={{
              background: "#fff",
              borderRadius: 16,
              borderLeft: `6px solid ${getMeetingAccentColor(featured, getTemporalState(featured, now))}`,
              boxShadow: "0 1px 3px rgba(26,31,36,0.04), 0 8px 24px rgba(26,31,36,0.06)",
              padding: "28px 32px",
            }}
          >
            {/* Time + Duration */}
            <div
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 15,
                fontWeight: 500,
                color: "var(--color-text-tertiary)",
                marginBottom: 8,
                display: "flex",
                alignItems: "center",
                gap: 8,
              }}
            >
              <span>{featured.time}</span>
              {featured.endTime && (
                <>
                  <span style={{ opacity: 0.4 }}>&mdash;</span>
                  <span>{featured.endTime}</span>
                </>
              )}
              {formatDuration(featured) && (
                <span style={{ opacity: 0.5 }}>({formatDuration(featured)})</span>
              )}
              {getTemporalState(featured, now) === "in-progress" && (
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
                  }}
                >
                  NOW
                </span>
              )}
            </div>

            {/* Title */}
            <h2
              style={{
                fontFamily: "var(--font-serif)",
                fontSize: 24,
                fontWeight: 400,
                color: "var(--color-text-primary)",
                margin: "0 0 8px 0",
                lineHeight: 1.3,
              }}
            >
              {featured.title}
            </h2>

            {/* Account + attendees */}
            <div
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 13,
                color: "var(--color-text-tertiary)",
                marginBottom: featured.prep?.context ? 16 : 0,
              }}
            >
              {featured.account && <span>{featured.account}</span>}
              {featured.prep?.stakeholders && featured.prep.stakeholders.length > 0 && (
                <span>
                  {featured.account ? " \u00B7 " : ""}
                  {featured.prep.stakeholders.map((s) => s.name).slice(0, 3).join(", ")}
                  {featured.prep.stakeholders.length > 3 &&
                    ` +${featured.prep.stakeholders.length - 3}`}
                </span>
              )}
            </div>

            {/* Narrative prep — conclusions before evidence */}
            {featured.prep?.context && (
              <p
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 15,
                  fontWeight: 300,
                  lineHeight: 1.65,
                  color: "var(--color-text-secondary)",
                  margin: "0 0 20px 0",
                }}
              >
                {featured.prep.context}
              </p>
            )}

            {/* Before this meeting — related actions */}
            {featuredActions.length > 0 && (
              <div
                style={{
                  background: "rgba(201, 162, 39, 0.06)",
                  borderRadius: 12,
                  padding: "16px 20px",
                  marginBottom: 20,
                }}
              >
                <div
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 11,
                    fontWeight: 600,
                    letterSpacing: "0.06em",
                    color: "var(--color-spice-turmeric)",
                    marginBottom: 10,
                    textTransform: "uppercase",
                  }}
                >
                  Before this meeting
                </div>
                {featuredActions.slice(0, 3).map((action) => (
                  <div
                    key={action.id}
                    style={{
                      fontFamily: "var(--font-sans)",
                      fontSize: 14,
                      color: "var(--color-text-primary)",
                      padding: "4px 0",
                      display: "flex",
                      alignItems: "baseline",
                      gap: 8,
                    }}
                  >
                    <span style={{ color: "var(--color-spice-turmeric)", flexShrink: 0 }}>&bull;</span>
                    <span>{stripMarkdown(action.title)}</span>
                  </div>
                ))}
              </div>
            )}

            {/* Bridge to depth */}
            <Link
              to="/meeting/$meetingId"
              params={{ meetingId: featured.id }}
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 14,
                fontWeight: 500,
                color: "var(--color-spice-turmeric)",
                textDecoration: "none",
                display: "inline-flex",
                alignItems: "center",
                gap: 4,
              }}
            >
              Read full intelligence &rarr;
            </Link>
          </div>
        </section>
      )}

      {/* ═══ SCHEDULE ═══ */}
      {hasSchedule && (
        <section style={{ marginBottom: 48 }}>
          {/* Section header */}
          <div style={{ display: "flex", alignItems: "baseline", gap: 12, marginBottom: 20 }}>
            <h2
              style={{
                fontFamily: "var(--font-serif)",
                fontSize: 26,
                fontWeight: 400,
                letterSpacing: "-0.01em",
                color: "var(--color-text-primary)",
                margin: 0,
              }}
            >
              Schedule
            </h2>
            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 13,
                color: "var(--color-text-tertiary)",
              }}
            >
              {scheduleMeetings.filter((m) => m.overlayStatus !== "cancelled").length} meetings
            </span>
          </div>
          <div
            style={{
              height: 1,
              background: "var(--color-rule-heavy)",
              marginBottom: 20,
            }}
          />

          {/* Meeting cards */}
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            {scheduleMeetings.map((meeting) => (
              <BriefingMeetingCard
                key={meeting.id}
                meeting={meeting}
                now={now}
                currentMeeting={currentMeeting}
              />
            ))}
          </div>
        </section>
      )}

      {/* ═══ PRIORITIES / LOOSE THREADS ═══ */}
      {data.focus && data.focus.prioritizedActions.length > 0 ? (
        <PrioritiesSection
          focus={data.focus}
          completedIds={completedIds}
          onComplete={handleComplete}
          highPriorityEmails={highPriorityEmails}
          allEmails={emails}
          totalPendingActions={pendingActions.length}
        />
      ) : (visibleActions.length > 0 || highPriorityEmails.length > 0) ? (
        <section style={{ marginBottom: 48 }}>
          {/* Section header */}
          <div style={{ display: "flex", alignItems: "baseline", gap: 12, marginBottom: 20 }}>
            <h2
              style={{
                fontFamily: "var(--font-serif)",
                fontSize: 26,
                fontWeight: 400,
                letterSpacing: "-0.01em",
                color: "var(--color-text-primary)",
                margin: 0,
              }}
            >
              Loose Threads
            </h2>
            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 13,
                color: "var(--color-text-tertiary)",
              }}
            >
              {looseThreadsCount}
            </span>
          </div>
          <div
            style={{
              height: 1,
              background: "var(--color-rule-heavy)",
              marginBottom: 16,
            }}
          />

          {/* Action rows */}
          <div style={{ display: "flex", flexDirection: "column" }}>
            {visibleActions.map((action, i) => (
              <BriefingActionRow
                key={action.id}
                action={action}
                isCompleted={completedIds.has(action.id)}
                onComplete={handleComplete}
                showBorder={i < visibleActions.length - 1 || highPriorityEmails.length > 0}
              />
            ))}

            {/* Email rows */}
            {highPriorityEmails.map((email, i) => (
              <BriefingEmailRow
                key={email.id}
                email={email}
                showBorder={i < highPriorityEmails.length - 1}
              />
            ))}
          </div>

          {/* View all links */}
          <div style={{ display: "flex", gap: 24, marginTop: 16 }}>
            {hasMoreActions && (
              <Link
                to="/actions"
                search={{ search: undefined }}
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 14,
                  fontWeight: 500,
                  color: "var(--color-spice-turmeric)",
                  textDecoration: "none",
                }}
              >
                View all {pendingActions.length} actions &rarr;
              </Link>
            )}
            {emails.length > highPriorityEmails.length && (
              <Link
                to="/emails"
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 14,
                  fontWeight: 500,
                  color: "var(--color-spice-turmeric)",
                  textDecoration: "none",
                }}
              >
                View all emails &rarr;
              </Link>
            )}
          </div>
        </section>
      ) : null}

      {/* ═══ END MARK ═══ */}
      <div
        style={{
          borderTop: `1px solid var(--color-rule-heavy)`,
          marginTop: 64,
          paddingTop: 48,
          paddingBottom: 120,
          textAlign: "center",
        }}
      >
        <div
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 18,
            letterSpacing: "0.4em",
            color: "var(--color-text-tertiary)",
            marginBottom: 16,
          }}
        >
          * * *
        </div>
        <div
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 15,
            fontStyle: "italic",
            color: "var(--color-text-tertiary)",
          }}
        >
          You're briefed. Go get it.
        </div>
      </div>
    </div>
  );
}

// ─── Priorities Section (capacity-aware) ─────────────────────────────────────

function PrioritiesSection({
  focus,
  completedIds,
  onComplete,
  highPriorityEmails,
  allEmails,
  totalPendingActions,
}: {
  focus: NonNullable<DashboardData["focus"]>;
  completedIds: Set<string>;
  onComplete: (id: string) => void;
  highPriorityEmails: Email[];
  allEmails: Email[];
  totalPendingActions: number;
}) {
  const visible = focus.prioritizedActions.slice(0, 5);
  const hasMore = totalPendingActions > 5;

  return (
    <section style={{ marginBottom: 48 }}>
      {/* Section header */}
      <div style={{ display: "flex", alignItems: "baseline", gap: 12, marginBottom: 4 }}>
        <h2
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 26,
            fontWeight: 400,
            letterSpacing: "-0.01em",
            color: "var(--color-text-primary)",
            margin: 0,
          }}
        >
          Priorities
        </h2>
      </div>
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 13,
          color: "var(--color-text-tertiary)",
          marginBottom: 20,
        }}
      >
        {focus.implications.summary}
      </div>
      <div
        style={{
          height: 1,
          background: "var(--color-rule-heavy)",
          marginBottom: 16,
        }}
      />

      {/* Prioritized action rows */}
      <div style={{ display: "flex", flexDirection: "column" }}>
        {visible.map((pa, i) => (
          <BriefingPrioritizedActionRow
            key={pa.action.id}
            pa={pa}
            isCompleted={completedIds.has(pa.action.id)}
            onComplete={onComplete}
            showBorder={i < visible.length - 1 || highPriorityEmails.length > 0}
          />
        ))}

        {/* Email rows */}
        {highPriorityEmails.map((email, i) => (
          <BriefingEmailRow
            key={email.id}
            email={email}
            showBorder={i < highPriorityEmails.length - 1}
          />
        ))}
      </div>

      {/* View all links */}
      <div style={{ display: "flex", gap: 24, marginTop: 16 }}>
        {hasMore && (
          <Link
            to="/actions"
            search={{ search: undefined }}
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 14,
              fontWeight: 500,
              color: "var(--color-spice-turmeric)",
              textDecoration: "none",
            }}
          >
            View all {totalPendingActions} actions &rarr;
          </Link>
        )}
        {allEmails.length > highPriorityEmails.length && (
          <Link
            to="/emails"
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 14,
              fontWeight: 500,
              color: "var(--color-spice-turmeric)",
              textDecoration: "none",
            }}
          >
            View all emails &rarr;
          </Link>
        )}
      </div>
    </section>
  );
}

// ─── Prioritized Action Row ──────────────────────────────────────────────────

function BriefingPrioritizedActionRow({
  pa,
  isCompleted,
  onComplete,
  showBorder,
}: {
  pa: PrioritizedAction;
  isCompleted: boolean;
  onComplete: (id: string) => void;
  showBorder: boolean;
}) {
  const action = pa.action;
  const done = action.status === "completed" || isCompleted;

  // Context: effort + reason
  const contextParts: string[] = [];
  contextParts.push(`~${formatMinutes(pa.effortMinutes)}`);
  if (action.accountName) contextParts.push(action.accountName);
  else if (action.accountId) contextParts.push(action.accountId);

  const borderColor = pa.atRisk
    ? "var(--color-spice-terracotta)"
    : "var(--color-rule-heavy)";

  return (
    <div
      style={{
        display: "flex",
        alignItems: "flex-start",
        gap: 12,
        padding: "12px 0",
        borderBottom: showBorder ? "1px solid var(--color-rule-light)" : "none",
        opacity: done ? 0.4 : pa.feasible ? 1 : 0.5,
        transition: "opacity 0.15s ease",
      }}
    >
      {/* Checkbox */}
      <button
        onClick={() => !done && onComplete(action.id)}
        disabled={done}
        style={{
          width: 20,
          height: 20,
          borderRadius: 10,
          border: `2px solid ${borderColor}`,
          background: done ? "var(--color-garden-sage)" : "transparent",
          cursor: done ? "default" : "pointer",
          flexShrink: 0,
          marginTop: 2,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          transition: "all 0.15s ease",
        }}
      >
        {done && (
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
            <path d="M2.5 6L5 8.5L9.5 4" stroke="#fff" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        )}
      </button>

      {/* Content */}
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ display: "flex", alignItems: "baseline", gap: 8 }}>
          <Link
            to="/actions/$actionId"
            params={{ actionId: action.id }}
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 15,
              fontWeight: 400,
              color: "var(--color-text-primary)",
              textDecoration: done ? "line-through" : "none",
              lineHeight: 1.4,
            }}
          >
            {stripMarkdown(action.title)}
          </Link>
          {pa.atRisk && (
            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                fontWeight: 700,
                letterSpacing: "0.06em",
                padding: "1px 6px",
                borderRadius: 3,
                background: "rgba(196, 101, 74, 0.12)",
                color: "var(--color-spice-terracotta)",
                whiteSpace: "nowrap",
              }}
            >
              AT RISK
            </span>
          )}
        </div>
        <div
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 13,
            fontWeight: 300,
            color: "var(--color-text-tertiary)",
            marginTop: 2,
          }}
        >
          {contextParts.join(" \u00B7 ")}
        </div>
      </div>
    </div>
  );
}

// ─── Action Row (inline subcomponent) ────────────────────────────────────────

function BriefingActionRow({
  action,
  isCompleted,
  onComplete,
  showBorder,
}: {
  action: Action;
  isCompleted: boolean;
  onComplete: (id: string) => void;
  showBorder: boolean;
}) {
  const done = action.status === "completed" || isCompleted;

  // Context line: "2 days overdue . Account . Source"
  const contextParts: string[] = [];
  if (action.isOverdue && action.daysOverdue) {
    contextParts.push(`${action.daysOverdue} day${action.daysOverdue !== 1 ? "s" : ""} overdue`);
  } else if (action.dueDate) {
    contextParts.push(action.dueDate);
  }
  if (action.account) contextParts.push(action.account);
  if (action.source) contextParts.push(action.source);

  const contextColor = action.isOverdue
    ? "var(--color-spice-terracotta)"
    : action.dueDate
      ? "var(--color-spice-turmeric)"
      : "var(--color-text-tertiary)";

  return (
    <div
      style={{
        display: "flex",
        alignItems: "flex-start",
        gap: 12,
        padding: "12px 0",
        borderBottom: showBorder ? "1px solid var(--color-rule-light)" : "none",
        opacity: done ? 0.4 : 1,
        transition: "opacity 0.15s ease",
      }}
    >
      {/* Checkbox */}
      <button
        onClick={() => !done && onComplete(action.id)}
        disabled={done}
        style={{
          width: 20,
          height: 20,
          borderRadius: 10,
          border: `2px solid ${action.isOverdue ? "var(--color-spice-terracotta)" : "var(--color-rule-heavy)"}`,
          background: done ? "var(--color-garden-sage)" : "transparent",
          cursor: done ? "default" : "pointer",
          flexShrink: 0,
          marginTop: 2,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          transition: "all 0.15s ease",
        }}
      >
        {done && (
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
            <path d="M2.5 6L5 8.5L9.5 4" stroke="#fff" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        )}
      </button>

      {/* Content */}
      <div style={{ flex: 1, minWidth: 0 }}>
        <Link
          to="/actions/$actionId"
          params={{ actionId: action.id }}
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 15,
            fontWeight: 400,
            color: "var(--color-text-primary)",
            textDecoration: done ? "line-through" : "none",
            lineHeight: 1.4,
          }}
        >
          {stripMarkdown(action.title)}
        </Link>
        {contextParts.length > 0 && (
          <div
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 13,
              fontWeight: action.isOverdue ? 500 : 300,
              color: contextColor,
              marginTop: 2,
            }}
          >
            {contextParts.join(" \u00B7 ")}
          </div>
        )}
      </div>
    </div>
  );
}

// ─── Email Row (inline subcomponent) ─────────────────────────────────────────

function BriefingEmailRow({
  email,
  showBorder,
}: {
  email: Email;
  showBorder: boolean;
}) {
  return (
    <Link
      to="/emails"
      style={{
        display: "flex",
        alignItems: "flex-start",
        gap: 12,
        padding: "12px 0",
        borderBottom: showBorder ? "1px solid var(--color-rule-light)" : "none",
        textDecoration: "none",
      }}
    >
      {/* Priority dot */}
      <div
        style={{
          width: 8,
          height: 8,
          borderRadius: 4,
          background: email.priority === "high"
            ? "var(--color-spice-terracotta)"
            : "var(--color-spice-turmeric)",
          flexShrink: 0,
          marginTop: 7,
        }}
      />

      {/* Content */}
      <div style={{ flex: 1, minWidth: 0 }}>
        <div
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 15,
            color: "var(--color-text-primary)",
            lineHeight: 1.4,
          }}
        >
          <span style={{ fontWeight: 500 }}>{email.sender}</span>
          <span style={{ color: "var(--color-text-tertiary)", margin: "0 6px" }}>&mdash;</span>
          <span>{email.subject}</span>
        </div>
        {email.recommendedAction && (
          <div
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 13,
              fontWeight: 500,
              color: "var(--color-spice-turmeric)",
              marginTop: 2,
            }}
          >
            {email.recommendedAction}
          </div>
        )}
      </div>
    </Link>
  );
}
