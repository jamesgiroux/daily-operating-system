/**
 * DailyBriefing.tsx — Magazine editorial daily briefing
 *
 * A morning document, not a dashboard. You read it top to bottom.
 * When you reach the end, you're briefed.
 *
 * Sections: Hero > Focus > Lead Story > Schedule > Priorities > Finis
 *
 * Design reference: design/mockups/daily-briefing-reimagined-v2.html
 * Layout: margin grid (100px label | content), section rules, no cards.
 */

import { useState, useCallback, useMemo } from "react";
import { Link } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import clsx from "clsx";
import { useCalendar } from "@/hooks/useCalendar";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import type { ReadinessStat } from "@/components/layout/FolioBar";
import {
  BriefingMeetingCard,
  KeyPeopleFlow,
  PrepGrid,
  MeetingActionChecklist,
  getTemporalState,
  formatDuration,
} from "./BriefingMeetingCard";
import { RefreshCw, Loader2 } from "lucide-react";
import type { WorkflowStatus } from "@/hooks/useWorkflow";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { MeetingEntityChips } from "@/components/ui/meeting-entity-chips";
import { formatDayTime, stripMarkdown } from "@/lib/utils";
import type { DashboardData, DataFreshness, Meeting, MeetingType, Action, Email, PrioritizedAction } from "@/types";
import s from "@/styles/editorial-briefing.module.css";

// ─── Types ───────────────────────────────────────────────────────────────────

interface DailyBriefingProps {
  data: DashboardData;
  freshness: DataFreshness;
  onRunBriefing?: () => void;
  isRunning?: boolean;
  workflowStatus?: WorkflowStatus;
  onRefresh?: () => void;
}

// ─── Featured Meeting Selection ──────────────────────────────────────────────

const MEETING_TYPE_WEIGHTS: Partial<Record<MeetingType, number>> = {
  qbr: 100,
  customer: 80,
  partnership: 60,
  external: 40,
  training: 20,
};

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

function getMeetingStartMs(meeting: Meeting): number | null {
  return parseDisplayTimeMs(meeting.time);
}

export function selectFeaturedMeeting(meetings: Meeting[], now: number): Meeting | null {
  const candidates = meetings.filter((m) => {
    const state = getTemporalState(m, now);
    if (state === "past" || state === "cancelled") return false;
    const isExternal = ["customer", "qbr", "partnership", "external"].includes(m.type);
    return isExternal && m.hasPrep;
  });

  if (candidates.length === 0) return null;

  return candidates.sort((a, b) => {
    const wa = MEETING_TYPE_WEIGHTS[a.type] ?? 0;
    const wb = MEETING_TYPE_WEIGHTS[b.type] ?? 0;
    if (wa !== wb) return wb - wa;
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

// ─── Capacity Formatting ─────────────────────────────────────────────────────

function formatMinutes(minutes: number): string {
  if (minutes < 60) return `${minutes}m`;
  const hrs = Math.floor(minutes / 60);
  const rem = minutes % 60;
  return rem > 0 ? `${hrs}h ${rem}m` : `${hrs}h`;
}

// ─── Component ───────────────────────────────────────────────────────────────

export function DailyBriefing({ data, freshness, onRunBriefing, isRunning, workflowStatus, onRefresh }: DailyBriefingProps) {
  const { now, currentMeeting } = useCalendar();
  const [completedIds, setCompletedIds] = useState<Set<string>>(new Set());

  // Data
  const meetings = data.meetings;
  const actions = data.actions;
  const emails = data.emails ?? [];
  const highPriorityEmails = emails.filter((e) => e.priority === "high").slice(0, 4);
  const briefingEmails = highPriorityEmails.length > 0
    ? highPriorityEmails
    : emails.filter((e) => e.priority === "medium").slice(0, 3);
  const emailSectionLabel = highPriorityEmails.length > 0
    ? "Emails Needing Response"
    : "Emails Worth Noting";

  // Featured meeting (still appears in schedule — lead story is a highlight, not a removal)
  const featured = selectFeaturedMeeting(meetings, now);
  const scheduleMeetings = meetings;

  // Readiness
  const readiness = computeReadiness(meetings, actions);

  // Date formatting
  const formattedDate = new Date().toLocaleDateString("en-US", {
    weekday: "long",
    month: "long",
    day: "numeric",
    year: "numeric",
  }).toUpperCase();

  // Register magazine shell with folio bar
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

  const folioActions = useMemo(() => {
    if (!onRunBriefing) return undefined;
    const phaseLabel = isRunning && workflowStatus?.status === "running"
      ? { preparing: "Preparing\u2026", enriching: "AI Processing\u2026", delivering: "Delivering\u2026" }[workflowStatus.phase]
      : null;
    return (
      <button
        onClick={onRunBriefing}
        disabled={isRunning}
        className="flex items-center gap-1.5 rounded-sm px-2 py-1 text-xs text-muted-foreground transition-colors hover:text-foreground disabled:opacity-50 disabled:cursor-not-allowed"
        title={isRunning ? "Briefing in progress" : "Run morning briefing"}
      >
        {isRunning ? (
          <Loader2 className="h-3 w-3 animate-spin" />
        ) : (
          <RefreshCw className="h-3 w-3" />
        )}
        <span>{phaseLabel ?? "Refresh"}</span>
      </button>
    );
  }, [onRunBriefing, isRunning, workflowStatus]);

  const shellConfig = useMemo(
    () => ({
      folioLabel: "Daily Briefing",
      atmosphereColor: "turmeric" as const,
      activePage: "today" as const,
      folioDateText: formattedDate,
      folioReadinessStats: folioReadinessStats,
      folioActions,
    }),
    [formattedDate, folioReadinessStats, folioActions],
  );
  useRegisterMagazineShell(shellConfig);

  // Pending actions (sorted by urgency)
  const pendingActions = actions
    .filter((a) => a.status !== "completed" && !completedIds.has(a.id))
    .sort((a, b) => {
      if (a.isOverdue && !b.isOverdue) return -1;
      if (!a.isOverdue && b.isOverdue) return 1;
      const priorityOrder = { P1: 0, P2: 1, P3: 2 };
      return (priorityOrder[a.priority] ?? 2) - (priorityOrder[b.priority] ?? 2);
    });

  // Action completion
  const handleComplete = useCallback((id: string) => {
    setCompletedIds((prev) => new Set(prev).add(id));
    invoke("complete_action", { id }).catch(() => {});
  }, []);

  // Meeting actions helper: find actions related to a specific meeting
  const getActionsForMeeting = useCallback((meetingId: string) => {
    return actions.filter((a) => a.source === meetingId && a.status !== "completed");
  }, [actions]);

  // Featured meeting actions
  const featuredActions = featured ? getActionsForMeeting(featured.id) : [];
  const featuredState = featured ? getTemporalState(featured, now) : "upcoming";
  const featuredDuration = featured ? formatDuration(featured) : null;

  // Schedule stats
  const activeMeetings = meetings.filter((m) => m.overlayStatus !== "cancelled");
  const hasSchedule = scheduleMeetings.some((m) => m.overlayStatus !== "cancelled");
  const scheduleCount = scheduleMeetings.filter((m) => m.overlayStatus !== "cancelled").length;

  // Build hero narrative from overview
  const heroHeadline = data.overview.summary || (activeMeetings.length === 0
    ? "A clear day. Nothing needs you."
    : "Your day is ready.");

  return (
    <div>
      {/* ═══ HERO ═══ */}
      <section className={s.hero}>
        <h1 className={s.heroHeadline}>{heroHeadline}</h1>

        {/* Staleness indicator */}
        {freshness.freshness === "stale" && (
          <div className={s.staleness}>
            Last updated {formatDayTime(freshness.generatedAt)}
          </div>
        )}
      </section>

      {/* ═══ FOCUS ═══ */}
      {data.overview.focus && (
        <section className={s.focusSection}>
          <div className={s.marginGrid}>
            <div className={s.marginLabel}>Focus</div>
            <div className={s.marginContent}>
              <div className={s.focusBlock}>
                <div className={s.focusText}>{data.overview.focus}</div>
              </div>
              {data.focus && (
                <div className={s.focusCapacity}>
                  {formatMinutes(data.focus.availableMinutes)} available
                  {data.focus.availableBlocks.filter((b) => b.durationMinutes >= 60).length > 0 && (
                    <> &middot; {data.focus.availableBlocks.filter((b) => b.durationMinutes >= 60).length} deep work block{data.focus.availableBlocks.filter((b) => b.durationMinutes >= 60).length !== 1 ? "s" : ""}</>
                  )}
                  {" "}&middot; {data.focus.meetingCount} meeting{data.focus.meetingCount !== 1 ? "s" : ""}
                </div>
              )}
            </div>
          </div>
        </section>
      )}

      {/* ═══ LEAD STORY (Featured Meeting) ═══ */}
      {featured && (
        <section className={s.leadStory}>
          <div className={s.marginGrid}>
            <div className={s.marginLabel}>The Meeting</div>
            <div className={s.marginContent}>
              <div className={s.sectionRule} />

              {/* Title */}
              <h2 className={s.leadTitle}>
                {featured.title}
                {featuredState === "in-progress" && (
                  <span className={s.nowPillInline}>NOW</span>
                )}
              </h2>

              {/* Meta line: time / duration / account / attendees */}
              <div className={s.leadMeta}>
                <span>{featured.time}{featured.endTime ? ` \u2013 ${featured.endTime}` : ""}</span>
                {featuredDuration && (
                  <>
                    <span className={s.leadMetaSep}>/</span>
                    <span>{featuredDuration}</span>
                  </>
                )}
                {featured.account && (
                  <>
                    <span className={s.leadMetaSep}>/</span>
                    <span>{featured.account}</span>
                  </>
                )}
                {featured.prep?.stakeholders && featured.prep.stakeholders.length > 0 && (
                  <>
                    <span className={s.leadMetaSep}>/</span>
                    <span>{featured.prep.stakeholders.length} attendee{featured.prep.stakeholders.length !== 1 ? "s" : ""}</span>
                  </>
                )}
              </div>

              {/* Narrative context — conclusions before evidence */}
              {featured.prep?.context && (
                <p className={s.leadNarrative}>{featured.prep.context}</p>
              )}

              {/* Key people */}
              {featured.prep?.stakeholders && featured.prep.stakeholders.length > 0 && (
                <KeyPeopleFlow stakeholders={featured.prep.stakeholders} />
              )}

              {/* Prep grid */}
              <PrepGrid meeting={featured} />

              {/* Before this meeting — related actions */}
              <MeetingActionChecklist
                actions={featuredActions}
                completedIds={completedIds}
                onComplete={handleComplete}
              />

              {/* Entity assignment */}
              <div style={{ marginBottom: 24 }}>
                <MeetingEntityChips
                  meetingId={featured.id}
                  meetingTitle={featured.title}
                  meetingStartTime={featured.startIso ?? new Date().toISOString()}
                  meetingType={featured.type}
                  linkedEntities={featured.linkedEntities ?? []}
                  onEntitiesChanged={onRefresh}
                />
              </div>

              {/* Bridge links */}
              <div className={s.meetingLinks}>
                <Link
                  to="/meeting/$meetingId"
                  params={{ meetingId: featured.id }}
                  className={s.meetingLinkPrimary}
                >
                  Read full intelligence &rarr;
                </Link>
              </div>
            </div>
          </div>
        </section>
      )}

      {/* ═══ SCHEDULE ═══ */}
      {hasSchedule && (
        <section className={s.scheduleSection}>
          <div className={s.marginGrid}>
            <div className={s.marginLabel}>
              Schedule
              <span className={s.marginLabelCount}>{scheduleCount} meetings</span>
            </div>
            <div className={s.marginContent}>
              <div className={s.sectionRule} />
              <div className={s.scheduleRows}>
                {scheduleMeetings.map((meeting) => (
                  <BriefingMeetingCard
                    key={meeting.id}
                    meeting={meeting}
                    now={now}
                    currentMeeting={currentMeeting}
                    meetingActions={getActionsForMeeting(meeting.id)}
                    onComplete={handleComplete}
                    completedIds={completedIds}
                    onEntitiesChanged={onRefresh}
                  />
                ))}
              </div>
            </div>
          </div>
        </section>
      )}

      {/* ═══ PRIORITIES ═══ */}
      {data.focus && data.focus.prioritizedActions.length > 0 ? (
        <PrioritiesSection
          focus={data.focus}
          completedIds={completedIds}
          onComplete={handleComplete}
          briefingEmails={briefingEmails}
          emailSectionLabel={emailSectionLabel}
          allEmails={emails}
          totalPendingActions={pendingActions.length}
        />
      ) : (pendingActions.length > 0 || briefingEmails.length > 0) ? (
        <LooseThreadsSection
          pendingActions={pendingActions}
          briefingEmails={briefingEmails}
          allEmails={emails}
          completedIds={completedIds}
          onComplete={handleComplete}
        />
      ) : null}

      {/* ═══ FINIS ═══ */}
      <FinisMarker />
    </div>
  );
}

// ─── Priorities Section (capacity-aware, tapering density) ───────────────────

function PrioritiesSection({
  focus,
  completedIds,
  onComplete,
  briefingEmails,
  emailSectionLabel,
  allEmails,
  totalPendingActions,
}: {
  focus: NonNullable<DashboardData["focus"]>;
  completedIds: Set<string>;
  onComplete: (id: string) => void;
  briefingEmails: Email[];
  emailSectionLabel: string;
  allEmails: Email[];
  totalPendingActions: number;
}) {
  // Group prioritized actions by urgency
  const overdueActions = focus.prioritizedActions.filter((pa) => pa.action.status !== "completed" && pa.atRisk);
  const todayActions = focus.prioritizedActions.filter((pa) => pa.action.status !== "completed" && !pa.atRisk && pa.feasible);
  const upcomingActions = focus.prioritizedActions.filter((pa) => pa.action.status !== "completed" && !pa.atRisk && !pa.feasible);

  const hasMore = totalPendingActions > focus.prioritizedActions.length;

  return (
    <section className={s.prioritiesSection}>
      <div className={s.marginGrid}>
        <div className={s.marginLabel}>Priorities</div>
        <div className={s.marginContent}>
          <div className={s.sectionRule} />

          {/* Prose intro — AI-synthesized capacity context */}
          {focus.implications.summary && (
            <p className={s.prioritiesIntro}>{focus.implications.summary}</p>
          )}

          {/* OVERDUE group */}
          {overdueActions.length > 0 && (
            <>
              <div className={clsx(s.priorityGroupLabel, s.priorityGroupLabelOverdue)}>Overdue</div>
              <div className={s.priorityItems}>
                {overdueActions.map((pa) => (
                  <PrioritizedActionItem
                    key={pa.action.id}
                    pa={pa}
                    urgency="overdue"
                    isCompleted={completedIds.has(pa.action.id)}
                    onComplete={onComplete}
                  />
                ))}
              </div>
            </>
          )}

          {/* DUE TODAY group */}
          {todayActions.length > 0 && (
            <>
              <div className={clsx(s.priorityGroupLabel, s.priorityGroupLabelToday)}>Due Today</div>
              <div className={s.priorityItems}>
                {todayActions.slice(0, 5).map((pa) => (
                  <PrioritizedActionItem
                    key={pa.action.id}
                    pa={pa}
                    urgency="today"
                    isCompleted={completedIds.has(pa.action.id)}
                    onComplete={onComplete}
                  />
                ))}
              </div>
            </>
          )}

          {/* EMAILS group (woven between action groups) */}
          {briefingEmails.length > 0 && (
            <>
              <div className={clsx(s.priorityGroupLabel, s.priorityGroupLabelToday)}>{emailSectionLabel}</div>
              <div className={s.priorityItems}>
                {briefingEmails.map((email) => (
                  <PriorityEmailItem key={email.id} email={email} />
                ))}
              </div>
            </>
          )}

          {/* UPCOMING group (tapered weight) */}
          {upcomingActions.length > 0 && (
            <>
              <div className={clsx(s.priorityGroupLabel, s.priorityGroupLabelUpcoming)}>Later This Week</div>
              <div className={s.priorityItems}>
                {upcomingActions.slice(0, 3).map((pa) => (
                  <PrioritizedActionItem
                    key={pa.action.id}
                    pa={pa}
                    urgency="upcoming"
                    isCompleted={completedIds.has(pa.action.id)}
                    onComplete={onComplete}
                  />
                ))}
              </div>
            </>
          )}

          {/* View all links */}
          <div className={s.prioritiesViewAll}>
            {hasMore && (
              <Link to="/actions" search={{ search: undefined }} className={s.viewAllLink}>
                View all {totalPendingActions} actions &rarr;
              </Link>
            )}
            {allEmails.length > briefingEmails.length && (
              <Link to="/emails" className={s.viewAllLink}>
                View all emails &rarr;
              </Link>
            )}
          </div>
        </div>
      </div>
    </section>
  );
}

// ─── Loose Threads Fallback (when no prioritized actions available) ──────────

function LooseThreadsSection({
  pendingActions,
  briefingEmails,
  allEmails,
  completedIds,
  onComplete,
}: {
  pendingActions: Action[];
  briefingEmails: Email[];
  allEmails: Email[];
  completedIds: Set<string>;
  onComplete: (id: string) => void;
}) {
  const visibleActions = pendingActions.slice(0, 5);
  const hasMore = pendingActions.length > 5;

  return (
    <section className={s.prioritiesSection}>
      <div className={s.marginGrid}>
        <div className={s.marginLabel}>Loose Threads</div>
        <div className={s.marginContent}>
          <div className={s.sectionRule} />

          <div className={s.priorityItems}>
            {visibleActions.map((action) => {
              const done = action.status === "completed" || completedIds.has(action.id);
              const isOverdue = action.isOverdue;
              return (
                <div
                  key={action.id}
                  className={clsx(
                    s.priorityItem,
                    done && s.priorityItemCompleted,
                    isOverdue ? s.priorityItemOverdue : s.priorityItemToday,
                    action.account && s.priorityItemAccount,
                  )}
                >
                  <button
                    className={clsx(
                      s.priorityCheck,
                      done && s.priorityCheckChecked,
                      isOverdue && !done && s.priorityCheckOverdue,
                    )}
                    onClick={() => !done && onComplete(action.id)}
                    disabled={done}
                  >
                    {done && (
                      <svg width="10" height="10" viewBox="0 0 12 12" fill="none">
                        <path d="M2.5 6L5 8.5L9.5 4" stroke="#fff" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
                      </svg>
                    )}
                  </button>
                  <div className={s.priorityContent}>
                    <Link
                      to="/actions/$actionId"
                      params={{ actionId: action.id }}
                      className={s.priorityTitle}
                      style={{ textDecoration: done ? "line-through" : "none" }}
                    >
                      {stripMarkdown(action.title)}
                    </Link>
                    {(isOverdue || action.dueDate || action.account) && (
                      <div className={s.priorityContext}>
                        {isOverdue && action.daysOverdue
                          ? `${action.daysOverdue} day${action.daysOverdue !== 1 ? "s" : ""} overdue`
                          : action.dueDate ?? ""}
                        {action.account && ` \u00B7 ${action.account}`}
                      </div>
                    )}
                  </div>
                </div>
              );
            })}

            {briefingEmails.map((email) => (
              <PriorityEmailItem key={email.id} email={email} />
            ))}
          </div>

          {/* View all links */}
          <div className={s.prioritiesViewAll}>
            {hasMore && (
              <Link to="/actions" search={{ search: undefined }} className={s.viewAllLink}>
                View all {pendingActions.length} actions &rarr;
              </Link>
            )}
            {allEmails.length > briefingEmails.length && (
              <Link to="/emails" className={s.viewAllLink}>
                View all emails &rarr;
              </Link>
            )}
          </div>
        </div>
      </div>
    </section>
  );
}

// ─── Prioritized Action Item ─────────────────────────────────────────────────

function PrioritizedActionItem({
  pa,
  urgency,
  isCompleted,
  onComplete,
}: {
  pa: PrioritizedAction;
  urgency: "overdue" | "today" | "upcoming";
  isCompleted: boolean;
  onComplete: (id: string) => void;
}) {
  const action = pa.action;
  const done = action.status === "completed" || isCompleted;

  const contextParts: string[] = [];
  if (urgency === "overdue") {
    contextParts.push("Overdue");
  }
  if (action.accountName) contextParts.push(action.accountName);
  else if (action.accountId) contextParts.push(action.accountId);
  contextParts.push(`~${formatMinutes(pa.effortMinutes)}`);

  const urgencyClass = {
    overdue: s.priorityItemOverdue,
    today: s.priorityItemToday,
    upcoming: s.priorityItemUpcoming,
  }[urgency];

  return (
    <Link
      to="/actions/$actionId"
      params={{ actionId: action.id }}
      className={clsx(
        s.priorityItem,
        urgencyClass,
        done && s.priorityItemCompleted,
        action.accountName && s.priorityItemAccount,
      )}
      style={{ textDecoration: "none" }}
    >
      <button
        className={clsx(
          s.priorityCheck,
          done && s.priorityCheckChecked,
          urgency === "overdue" && !done && s.priorityCheckOverdue,
        )}
        onClick={(e) => { e.preventDefault(); !done && onComplete(action.id); }}
        disabled={done}
      >
        {done && (
          <svg width="10" height="10" viewBox="0 0 12 12" fill="none">
            <path d="M2.5 6L5 8.5L9.5 4" stroke="#fff" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        )}
      </button>
      <div className={s.priorityContent}>
        <div className={s.priorityTitle}>{stripMarkdown(action.title)}</div>
        <div className={s.priorityContext}>{contextParts.join(" \u00B7 ")}</div>
        {urgency === "overdue" && pa.reason && (
          <div className={s.priorityWhy}>{pa.reason}</div>
        )}
      </div>
    </Link>
  );
}

// ─── Priority Email Item ─────────────────────────────────────────────────────

function PriorityEmailItem({ email }: { email: Email }) {
  return (
    <Link
      to="/emails"
      className={clsx(s.priorityItem, s.priorityItemEmailType)}
      style={{ textDecoration: "none" }}
    >
      <div
        className={clsx(
          s.priorityDot,
          email.priority === "high" ? s.priorityDotTerracotta : s.priorityDotTurmeric,
        )}
      />
      <div className={s.priorityContent}>
        <div className={s.priorityTitle}>
          <span className={s.prioritySender}>{email.sender}</span>
          <span className={s.prioritySubjectSep}>&mdash;</span>
          <span>{email.subject}</span>
        </div>
        {email.recommendedAction && (
          <div className={s.priorityAction}>{email.recommendedAction}</div>
        )}
      </div>
    </Link>
  );
}
