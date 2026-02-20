/**
 * DailyBriefing.tsx — Magazine editorial daily briefing
 *
 * A morning document, not a dashboard. You read it top to bottom.
 * When you reach the end, you're briefed.
 *
 * Sections: Day Frame > Schedule (Up Next) > Attention > Finis
 *
 * Design reference: design/mockups/daily-briefing-reimagined-v2.html
 * Layout: margin grid (100px label | content), section rules, no cards.
 */

import { useState, useCallback, useMemo } from "react";
import { Link, useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { useProposedActions } from "@/hooks/useProposedActions";
import { ProposedActionRow } from "@/components/shared/ProposedActionRow";
import clsx from "clsx";
import { useCalendar } from "@/hooks/useCalendar";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import type { ReadinessStat } from "@/components/layout/FolioBar";
import {
  BriefingMeetingCard,
  getTemporalState,
} from "./BriefingMeetingCard";
import { RefreshCw, Loader2 } from "lucide-react";
import type { WorkflowStatus } from "@/hooks/useWorkflow";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { formatDayTime, stripMarkdown } from "@/lib/utils";
import type { DashboardData, DataFreshness, Meeting, Action, Email, PrioritizedAction, ReplyNeeded } from "@/types";
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

// ─── Up Next Selection ──────────────────────────────────────────────────────

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

/** Find the first upcoming (not past, not cancelled) meeting. */
function findUpNextMeeting(meetings: Meeting[], now: number): Meeting | null {
  const upcoming = meetings.filter((m) => {
    const state = getTemporalState(m, now);
    return state !== "past" && state !== "cancelled";
  });
  if (upcoming.length === 0) return null;
  // Sort by start time, return earliest
  return upcoming.sort((a, b) => {
    const ta = getMeetingStartMs(a) ?? Infinity;
    const tb = getMeetingStartMs(b) ?? Infinity;
    return ta - tb;
  })[0];
}

/** Find high-stakes meetings (QBR/customer) that lack prep — for prep flags. */
function findUnpreppedHighStakes(meetings: Meeting[], now: number, upNextId?: string): Meeting[] {
  return meetings.filter((m) => {
    if (m.id === upNextId) return false;
    const state = getTemporalState(m, now);
    if (state === "past" || state === "cancelled") return false;
    const isHighStakes = ["qbr", "customer"].includes(m.type);
    return isHighStakes && !m.hasPrep;
  });
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

  // Up Next meeting (first upcoming, not past/cancelled)
  const upNext = findUpNextMeeting(meetings, now);
  const unpreppedHighStakes = findUnpreppedHighStakes(meetings, now, upNext?.id);
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
      // Use intelligence quality levels when available, fall back to hasPrep count
      const externalMeetings = meetings.filter((m) =>
        ["customer", "qbr", "partnership", "external"].includes(m.type) &&
        m.overlayStatus !== "cancelled"
      );
      const hasQualityData = externalMeetings.some((m) => m.intelligenceQuality);

      if (hasQualityData) {
        const readyCount = externalMeetings.filter(
          (m) => m.intelligenceQuality?.level === "ready" || m.intelligenceQuality?.level === "fresh"
        ).length;
        const buildingCount = externalMeetings.filter(
          (m) => m.intelligenceQuality?.level === "developing"
        ).length;

        if (readyCount === readiness.totalExternal) {
          stats.push({ label: `${readyCount}/${readiness.totalExternal} ready`, color: "sage" });
        } else if (buildingCount > 0) {
          stats.push({ label: `${readyCount} ready, ${buildingCount} building`, color: "sage" });
        } else {
          stats.push({ label: `${readyCount}/${readiness.totalExternal} ready`, color: "sage" });
        }
      } else {
        stats.push({ label: `${readiness.preppedCount}/${readiness.totalExternal} prepped`, color: "sage" });
      }
    }
    if (readiness.overdueCount > 0) {
      stats.push({ label: `${readiness.overdueCount} overdue`, color: "terracotta" });
    }
    return stats;
  }, [meetings, readiness.preppedCount, readiness.totalExternal, readiness.overdueCount]);

  const folioActions = useMemo(() => {
    if (!onRunBriefing) return undefined;
    const phaseLabel = isRunning && workflowStatus?.status === "running"
      ? { preparing: "Preparing…", enriching: "AI Processing…", delivering: "Delivering…" }[workflowStatus.phase]
      : null;
    return (
      <button
        onClick={onRunBriefing}
        disabled={isRunning}
        className="flex items-center gap-1.5 rounded-sm px-2 py-1 text-xs text-muted-foreground transition-colors hover:text-foreground disabled:opacity-50 disabled:cursor-not-allowed"
        title={isRunning ? "Briefing in progress" : "Refresh emails, actions, and intelligence"}
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
    invoke("complete_action", { id }).catch((err) => {
      console.error("complete_action failed:", err);
    });
  }, []);

  // Proposed actions for triage
  const { proposedActions, acceptAction, rejectAction } = useProposedActions();

  // Meeting actions helper: find actions related to a specific meeting
  const getActionsForMeeting = useCallback((meetingId: string) => {
    return actions.filter((a) => a.source === meetingId && a.status !== "completed");
  }, [actions]);

  // Meeting outcomes counts for past meeting summary lines
  const getCapturedActionCount = useCallback((meetingId: string) => {
    return actions.filter((a) => a.source === meetingId).length;
  }, [actions]);

  const getProposedActionCount = useCallback((meetingId: string) => {
    return proposedActions.filter((a) => a.sourceId === meetingId).length;
  }, [proposedActions]);

  // Schedule stats
  const activeMeetings = meetings.filter((m) => m.overlayStatus !== "cancelled");
  const hasSchedule = scheduleMeetings.some((m) => m.overlayStatus !== "cancelled");
  const scheduleCount = scheduleMeetings.filter((m) => m.overlayStatus !== "cancelled").length;

  // Build hero narrative from overview
  const heroHeadline = data.overview.summary || (activeMeetings.length === 0
    ? "A clear day. Nothing needs you."
    : "Your day is ready.");

  const isStale = freshness.freshness === "stale";

  return (
    <div>
      {/* Stale data indicator — non-blocking, shows refresh is in progress */}
      {isStale && (
        <div
          style={{
            padding: "8px 16px",
            borderBottom: "1px solid var(--color-rule-light)",
            fontFamily: "var(--font-mono)",
            fontSize: 10,
            fontWeight: 600,
            letterSpacing: "0.06em",
            textTransform: "uppercase",
            color: "var(--color-text-tertiary)",
            display: "flex",
            alignItems: "center",
            gap: 8,
          }}
        >
          <Loader2 className="h-3 w-3 animate-spin" style={{ width: 12, height: 12 }} />
          Morning refresh in progress
        </div>
      )}

      {/* ═══ DAY FRAME (Hero + Focus) ═══ */}
      <section className={s.hero}>
        <h1 className={s.heroHeadline}>{heroHeadline}</h1>

        {/* Capacity + focus directive */}
        {data.focus && (() => {
          const deepWorkBlocks = data.focus.availableBlocks.filter((b) => b.durationMinutes >= 60).length;
          return (
            <div className={s.focusCapacity}>
              {formatMinutes(data.focus.availableMinutes)} available
              {deepWorkBlocks > 0 && (
                <> &middot; {deepWorkBlocks} deep work block{deepWorkBlocks !== 1 ? "s" : ""}</>
              )}
              {" "}&middot; {data.focus.meetingCount} meeting{data.focus.meetingCount !== 1 ? "s" : ""}
            </div>
          );
        })()}

        {data.overview.focus && (
          <div className={s.focusBlock}>
            <div className={s.focusText}>{data.overview.focus}</div>
          </div>
        )}

        {/* Staleness indicator */}
        {freshness.freshness === "stale" && (
          <div className={s.staleness}>
            Last updated {formatDayTime(freshness.generatedAt)}
          </div>
        )}
      </section>

      {/* ═══ SCHEDULE (with Up Next) ═══ */}
      {hasSchedule && (
        <section className={s.scheduleSection}>
          <div className={s.marginGrid}>
            <div className={s.marginLabel}>
              Schedule
              <span className={s.marginLabelCount}>{scheduleCount} meetings</span>
            </div>
            <div className={s.marginContent}>
              <div className={s.sectionRule} />

              {/* Prep flags for high-stakes meetings without prep */}
              {unpreppedHighStakes.map((m) => (
                <div key={m.id} className={s.prepFlag}>
                  <Link
                    to="/meeting/$meetingId"
                    params={{ meetingId: m.id }}
                    style={{ textDecoration: "none", color: "inherit" }}
                  >
                    &#9888; {m.title} at {m.time} — no prep yet
                  </Link>
                </div>
              ))}

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
                    capturedActionCount={getCapturedActionCount(meeting.id)}
                    proposedActionCount={getProposedActionCount(meeting.id)}
                    isUpNext={upNext?.id === meeting.id}
                    userDomain={data.userDomains?.[0]}
                  />
                ))}
              </div>
            </div>
          </div>
        </section>
      )}

      {/* ═══ ATTENTION ═══ */}
      {/* When stale, only show attention section if we have actions (emails/narrative won't exist yet) */}
      <AttentionSection
        proposedActions={proposedActions}
        acceptAction={acceptAction}
        rejectAction={rejectAction}
        focus={data.focus}
        pendingActions={pendingActions}
        completedIds={completedIds}
        onComplete={handleComplete}
        briefingEmails={isStale ? [] : briefingEmails}
        emailSectionLabel={emailSectionLabel}
        allEmails={isStale ? [] : emails}
        emailNarrative={isStale ? undefined : data.emailNarrative}
        repliesNeeded={isStale ? undefined : data.repliesNeeded}
        todayMeetingIds={new Set(meetings.map((m) => m.id))}
      />

      {/* ═══ FINIS ═══ */}
      <FinisMarker />
    </div>
  );
}

// ─── Attention Section (unified: proposed + actions + emails) ─────────────────

function AttentionSection({
  proposedActions,
  acceptAction,
  rejectAction,
  focus,
  pendingActions,
  completedIds,
  onComplete,
  briefingEmails,
  emailSectionLabel,
  allEmails,
  emailNarrative,
  repliesNeeded,
  todayMeetingIds,
}: {
  proposedActions: Array<{ id: string; title: string; sourceLabel?: string; sourceId?: string }>;
  acceptAction: (id: string) => void;
  rejectAction: (id: string) => void;
  focus: DashboardData["focus"];
  pendingActions: Action[];
  completedIds: Set<string>;
  onComplete: (id: string) => void;
  briefingEmails: Email[];
  emailSectionLabel: string;
  allEmails: Email[];
  emailNarrative?: string;
  repliesNeeded?: ReplyNeeded[];
  todayMeetingIds: Set<string>;
}) {
  const navigate = useNavigate();

  // Filter attention-worthy actions: meeting-relevant for today OR overdue (max 3)
  const attentionActions = useMemo(() => {
    const prioritized = focus?.prioritizedActions ?? [];

    // Meeting-relevant: actions whose sourceId matches a today meeting
    const meetingRelevant = prioritized.filter(
      (pa) => pa.action.status !== "completed" && pa.action.sourceId && todayMeetingIds.has(pa.action.sourceId)
    );
    // Overdue/at-risk
    const atRisk = prioritized.filter(
      (pa) => pa.action.status !== "completed" && pa.atRisk && !meetingRelevant.includes(pa)
    );

    // If we have prioritized actions, use them
    if (meetingRelevant.length > 0 || atRisk.length > 0) {
      return [...meetingRelevant, ...atRisk].slice(0, 3);
    }

    // Fallback: use raw pending actions (overdue first, then meeting-relevant)
    const overdueRaw = pendingActions.filter((a) => a.isOverdue);
    const meetingRaw = pendingActions.filter(
      (a) => !a.isOverdue && a.source && todayMeetingIds.has(a.source)
    );
    return [...overdueRaw, ...meetingRaw].slice(0, 3);
  }, [focus, pendingActions, todayMeetingIds]);

  const hasProposed = proposedActions.length > 0;
  const hasActions = attentionActions.length > 0;
  const hasEmails = briefingEmails.length > 0;
  const hasNarrative = !!emailNarrative;
  const hasReplies = repliesNeeded && repliesNeeded.length > 0;
  const hasAnything = hasProposed || hasActions || hasEmails || hasNarrative || hasReplies;

  if (!hasAnything) return null;

  // Determine if attentionActions are PrioritizedAction or raw Action
  const hasPrioritizedActions = focus?.prioritizedActions && focus.prioritizedActions.length > 0;

  return (
    <section className={s.prioritiesSection}>
      <div className={s.marginGrid}>
        <div className={s.marginLabel}>Attention</div>
        <div className={s.marginContent}>
          <div className={s.sectionRule} />

          {/* AI capacity summary */}
          {focus?.implications?.summary && (
            <p className={s.prioritiesIntro}>{focus.implications.summary}</p>
          )}

          {/* Proposed action triage (max 3) */}
          {hasProposed && (
            <>
              <div style={{ display: "flex", flexDirection: "column", gap: 0 }}>
                {proposedActions.slice(0, 3).map((action, i) => (
                  <ProposedActionRow
                    key={action.id}
                    action={action}
                    onAccept={() => acceptAction(action.id)}
                    onReject={() => rejectAction(action.id)}
                    showBorder={i < Math.min(proposedActions.length, 3) - 1}
                    compact
                  />
                ))}
              </div>
              {proposedActions.length > 3 && (
                <button
                  onClick={() => navigate({ to: "/actions", search: { search: undefined } })}
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 11,
                    fontWeight: 500,
                    letterSpacing: "0.04em",
                    color: "var(--color-spice-turmeric)",
                    background: "none",
                    border: "none",
                    cursor: "pointer",
                    padding: "8px 0 0 14px",
                  }}
                >
                  See all {proposedActions.length} suggestions &rarr;
                </button>
              )}
            </>
          )}

          {/* Actions: meeting-relevant + overdue (max 3) */}
          {hasActions && (
            <div style={{ marginTop: hasProposed ? 28 : 0 }}>
              <div className={clsx(s.priorityGroupLabel, s.priorityGroupLabelOverdue)}>
                Actions
              </div>
              <div className={s.priorityItems}>
                {hasPrioritizedActions ? (
                  // Render as PrioritizedActionItem
                  (attentionActions as PrioritizedAction[]).map((pa) => (
                    <PrioritizedActionItem
                      key={pa.action.id}
                      pa={pa}
                      urgency={pa.atRisk ? "overdue" : "today"}
                      isCompleted={completedIds.has(pa.action.id)}
                      onComplete={onComplete}
                    />
                  ))
                ) : (
                  // Render raw actions
                  (attentionActions as Action[]).map((action) => {
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
                  })
                )}
              </div>
            </div>
          )}

          {/* Emails */}
          {hasEmails && (
            <>
              <div className={clsx(s.priorityGroupLabel, s.priorityGroupLabelToday)}>{emailSectionLabel}</div>
              <div className={s.priorityItems}>
                {briefingEmails.map((email) => (
                  <PriorityEmailItem key={email.id} email={email} />
                ))}
              </div>
            </>
          )}

          {/* Email narrative (I355) */}
          {hasNarrative && (
            <EmailBriefingNarrative narrative={emailNarrative!} />
          )}

          {/* Replies needed (I355/I356) */}
          {hasReplies && (
            <RepliesNeededList replies={repliesNeeded!} />
          )}

          {/* View all links */}
          <div className={s.prioritiesViewAll}>
            {pendingActions.length > 3 && (
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

// ─── Email Briefing Narrative (I355) ──────────────────────────────────────────

function EmailBriefingNarrative({ narrative }: { narrative: string }) {
  return (
    <div style={{ marginTop: 24 }}>
      <div className={clsx(s.priorityGroupLabel, s.priorityGroupLabelToday)}>Email Intelligence</div>
      <p className={s.emailNarrative}>{narrative}</p>
    </div>
  );
}

// ─── Replies Needed (I355/I356) ──────────────────────────────────────────────

function RepliesNeededList({ replies }: { replies: ReplyNeeded[] }) {
  return (
    <div style={{ marginTop: 24 }}>
      <div className={clsx(s.priorityGroupLabel, s.priorityGroupLabelOverdue)}>
        Awaiting Your Reply
        <span style={{ fontWeight: 400, opacity: 0.7, marginLeft: 8 }}>{replies.length}</span>
      </div>
      <div className={s.priorityItems}>
        {replies.map((reply) => (
          <div key={reply.threadId} className={s.replyItem}>
            <div className={s.replyDot} />
            <div className={s.replyContent}>
              <div className={s.replySubject}>{reply.subject}</div>
              <div className={s.replyMeta}>
                {reply.from}
                {reply.waitDuration && (
                  <> &middot; <span className={s.replyWait}>{reply.waitDuration}</span></>
                )}
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
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
