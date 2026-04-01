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
import { toast } from "sonner";
import { useSuggestedActions } from "@/hooks/useSuggestedActions";
import { SuggestedActionRow } from "@/components/shared/SuggestedActionRow";
import clsx from "clsx";
import { useCalendar } from "@/hooks/useCalendar";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import type { ReadinessStat } from "@/components/layout/FolioBar";
import {
  BriefingMeetingCard,
  getTemporalState,
} from "./BriefingMeetingCard";
import { FolioRefreshButton } from "@/components/ui/folio-refresh-button";

import type { WorkflowStatus } from "@/hooks/useWorkflow";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { formatDayTime, formatShortDate, stripMarkdown } from "@/lib/utils";
import { EmailEntityChip } from "@/components/ui/email-entity-chip";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type {
  BriefingCallout,
  DashboardData,
  DashboardLifecycleUpdate,
  DataFreshness,
  Meeting,
  Action,
  Email,
  PrioritizedAction,
} from "@/types";
import { HealthBadge } from "@/components/shared/HealthBadge";
import { compareEmailRank } from "@/lib/email-ranking";
import s from "@/styles/editorial-briefing.module.css";
import briefingStyles from "./DailyBriefing.module.css";

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
    const needsPrep = !m.intelligenceQuality || m.intelligenceQuality.level === "sparse";
    return isHighStakes && needsPrep;
  });
}

// ─── Readiness Computation ───────────────────────────────────────────────────

function computeReadiness(meetings: Meeting[], actions: Action[]) {
  const externalMeetings = meetings.filter((m) =>
    ["customer", "qbr", "partnership", "external"].includes(m.type) &&
    m.overlayStatus !== "cancelled"
  );
  const preppedCount = externalMeetings.filter((m) =>
    m.intelligenceQuality && m.intelligenceQuality.level !== "sparse"
  ).length;
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
  const [pendingLifecycleChangeId, setPendingLifecycleChangeId] = useState<number | null>(null);
  const [correctionTarget, setCorrectionTarget] = useState<DashboardLifecycleUpdate | null>(null);
  const [correctedLifecycle, setCorrectedLifecycle] = useState("");
  const [correctedStage, setCorrectedStage] = useState("");
  const [correctionNotes, setCorrectionNotes] = useState("");
  // Data
  const meetings = data.meetings;
  const actions = data.actions;
  const emails = data.emails ?? [];
  const lifecycleUpdates = data.lifecycleUpdates ?? [];

  // I395: Score-based email selection — scored emails first, then enriched fill.
  // Shows up to 5 emails: high-scored ones first, then enriched emails with summaries
  // that didn't meet the score threshold (avoids hiding useful intelligence).
  // Cached emails shown immediately even when briefing is stale — background
  // reconciliation will remove archived ones within seconds.
  const briefingEmails = (() => {
    if (emails.length === 0) return [];
    const ranked = [...emails].sort(compareEmailRank);
    const scored = ranked
      .filter((e) => (e.relevanceScore ?? 0) >= 0.15)
      .slice(0, 5);
    const scoredIds = new Set(scored.map((e) => e.id));
    // Fill remaining slots with enriched emails that have summaries but scored below threshold
    const enrichedFill = ranked
      .filter((e) => !scoredIds.has(e.id) && e.summary && e.summary.trim().length > 0)
      .slice(0, Math.max(0, 5 - scored.length));
    const selected = [...scored, ...enrichedFill].slice(0, 5);
    // Fallback: if no emails passed score/enrichment filters, show top by rank
    return selected.length > 0 ? selected : ranked.slice(0, 5);
  })();
  const emailSectionLabel = briefingEmails.length > 0 ? "WORTH YOUR ATTENTION" : "";

  // Up Next meeting (first upcoming, not past/cancelled)
  const upNext = findUpNextMeeting(meetings, now);
  const unpreppedHighStakes = findUnpreppedHighStakes(meetings, now, upNext?.id);
  // Only show meetings with invitees — filter out personal/solo calendar blocks
  // (classified as "personal" by google_api/classify.rs rule 2: 0-1 attendees)
  const scheduleMeetings = meetings.filter((m) => m.type !== "personal");

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
          stats.push({ label: `${readyCount} ready, ${buildingCount} limited`, color: "sage" });
        } else {
          stats.push({ label: `${readyCount}/${readiness.totalExternal} ready`, color: "sage" });
        }
      } else {
        stats.push({ label: `${readiness.preppedCount}/${readiness.totalExternal} ready`, color: "sage" });
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
      <FolioRefreshButton
        onClick={onRunBriefing}
        loading={!!isRunning}
        loadingLabel={phaseLabel ?? "Running\u2026"}
        title={isRunning ? "Briefing in progress" : "Refresh emails, actions, and context"}
      />
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
      toast.error("Failed to complete action");
    });
  }, []);

  const handleConfirmLifecycle = useCallback(async (update: DashboardLifecycleUpdate) => {
    setPendingLifecycleChangeId(update.changeId);
    try {
      await invoke("confirm_lifecycle_change", { changeId: update.changeId });
      toast.success(`${update.accountName} marked confirmed`);
      onRefresh?.();
    } catch (err) {
      console.error("confirm_lifecycle_change failed:", err);
      toast.error("Failed to confirm lifecycle change");
    } finally {
      setPendingLifecycleChangeId(null);
    }
  }, [onRefresh]);

  const openCorrection = useCallback((update: DashboardLifecycleUpdate) => {
    setCorrectionTarget(update);
    setCorrectedLifecycle(update.newLifecycle);
    setCorrectedStage(update.renewalStage ?? "");
    setCorrectionNotes(update.evidence ?? "");
  }, []);

  const closeCorrection = useCallback((open: boolean) => {
    if (open) return;
    setCorrectionTarget(null);
    setCorrectedLifecycle("");
    setCorrectedStage("");
    setCorrectionNotes("");
  }, []);

  const handleSubmitCorrection = useCallback(async () => {
    if (!correctionTarget) return;
    setPendingLifecycleChangeId(correctionTarget.changeId);
    try {
      await invoke("correct_lifecycle_change", {
        changeId: correctionTarget.changeId,
        correctedLifecycle,
        correctedStage: correctedStage || null,
        notes: correctionNotes.trim() || null,
      });
      toast.success(`${correctionTarget.accountName} updated`);
      setCorrectionTarget(null);
      setCorrectedLifecycle("");
      setCorrectedStage("");
      setCorrectionNotes("");
      onRefresh?.();
    } catch (err) {
      console.error("correct_lifecycle_change failed:", err);
      toast.error("Failed to correct lifecycle change");
    } finally {
      setPendingLifecycleChangeId(null);
    }
  }, [correctionNotes, correctedLifecycle, correctedStage, correctionTarget, onRefresh]);

  // Proposed actions for triage
  const { suggestedActions, acceptAction, rejectAction } = useSuggestedActions();

  // Meeting actions helper: find actions related to a specific meeting
  const getActionsForMeeting = useCallback((meetingId: string) => {
    return actions.filter((a) => a.source === meetingId && a.status !== "completed");
  }, [actions]);

  // Meeting outcomes counts for past meeting summary lines
  const getCapturedActionCount = useCallback((meetingId: string) => {
    return actions.filter((a) => a.source === meetingId).length;
  }, [actions]);

  const getSuggestedActionCount = useCallback((meetingId: string) => {
    return suggestedActions.filter((a) => a.sourceId === meetingId).length;
  }, [suggestedActions]);

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

        {/* Staleness indicator removed — orphaned "Last updated" with no date
            was confusing. The hero headline already communicates state. */}
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
                    className={briefingStyles.linkUnstyled}
                  >
                    &#9888; {m.title} at {m.time} — no briefing yet
                  </Link>
                </div>
              ))}

              <div className={s.scheduleRows}>
                {scheduleMeetings.map((meeting) => {
                  // I502: Find health data for first linked account
                  const healthMap = data.entityHealthMap;
                  const linkedAccountHealth = healthMap && meeting.linkedEntities
                    ? meeting.linkedEntities
                        .filter((e) => e.entityType === "account" && healthMap[e.id])
                        .map((e) => ({ entity: e, health: healthMap[e.id] }))[0]
                    : undefined;

                  return (
                    <div key={meeting.id}>
                      <BriefingMeetingCard
                        meeting={meeting}
                        now={now}
                        currentMeeting={currentMeeting}
                        meetingActions={getActionsForMeeting(meeting.id)}
                        onComplete={handleComplete}
                        completedIds={completedIds}
                        onEntitiesChanged={onRefresh}
                        capturedActionCount={getCapturedActionCount(meeting.id)}
                        suggestedActionCount={getSuggestedActionCount(meeting.id)}
                        isUpNext={upNext?.id === meeting.id}
                        userDomain={data.userDomains?.[0]}
                      />
                      {linkedAccountHealth && (
                        <div className={briefingStyles.healthBadgeRow}>
                          <HealthBadge
                            score={linkedAccountHealth.health.score}
                            band={linkedAccountHealth.health.band}
                            trend={linkedAccountHealth.health.trend}
                            size="compact"
                          />
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            </div>
          </div>
        </section>
      )}

      {/* ═══ ATTENTION ═══ */}
      {/* Cached emails shown even when stale — background reconciliation updates them */}
      <AttentionSection
        lifecycleUpdates={lifecycleUpdates}
        briefingCallouts={data.briefingCallouts ?? []}
        onConfirmLifecycle={handleConfirmLifecycle}
        onOpenLifecycleCorrection={openCorrection}
        pendingLifecycleChangeId={pendingLifecycleChangeId}
        suggestedActions={suggestedActions}
        acceptAction={acceptAction}
        rejectAction={rejectAction}
        focus={data.focus}
        pendingActions={pendingActions}
        completedIds={completedIds}
        onComplete={handleComplete}
        briefingEmails={briefingEmails}
        emailSectionLabel={emailSectionLabel}
        allEmails={emails}
        todayMeetingIds={new Set(meetings.map((m) => m.id))}
        emailSyncTimestamp={data.emailSync?.lastSuccessAt}
      />

      <Dialog open={!!correctionTarget} onOpenChange={closeCorrection}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Fix lifecycle change</DialogTitle>
            <DialogDescription>
              Update the lifecycle call for {correctionTarget?.accountName ?? "this account"}.
            </DialogDescription>
          </DialogHeader>
          <div className={briefingStyles.correctionFormGrid}>
            <label className={briefingStyles.correctionFieldLabel}>
              <span className={briefingStyles.correctionMonoLabel}>
                Lifecycle
              </span>
              <select
                value={correctedLifecycle}
                onChange={(event) => setCorrectedLifecycle(event.target.value)}
                className={briefingStyles.correctionSelect}
              >
                {["onboarding", "active", "renewing", "at_risk", "churned"].map((value) => (
                  <option key={value} value={value}>
                    {value.replace(/_/g, " ")}
                  </option>
                ))}
              </select>
            </label>
            <label className={briefingStyles.correctionFieldLabel}>
              <span className={briefingStyles.correctionMonoLabel}>
                Renewal stage
              </span>
              <select
                value={correctedStage}
                onChange={(event) => setCorrectedStage(event.target.value)}
                className={briefingStyles.correctionSelect}
              >
                <option value="">No stage</option>
                {["approaching", "negotiating", "contract_sent", "processed"].map((value) => (
                  <option key={value} value={value}>
                    {value.replace(/_/g, " ")}
                  </option>
                ))}
              </select>
            </label>
            <label className={briefingStyles.correctionFieldLabel}>
              <span className={briefingStyles.correctionMonoLabel}>
                Notes
              </span>
              <textarea
                value={correctionNotes}
                onChange={(event) => setCorrectionNotes(event.target.value)}
                rows={4}
                className={briefingStyles.correctionTextarea}
              />
            </label>
            <div className={briefingStyles.correctionButtonRow}>
              <button
                type="button"
                onClick={() => closeCorrection(false)}
                className={briefingStyles.correctionCancelBtn}
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={() => { void handleSubmitCorrection(); }}
                disabled={!correctedLifecycle || pendingLifecycleChangeId === correctionTarget?.changeId}
                className={briefingStyles.correctionSubmitBtn}
              >
                Save correction
              </button>
            </div>
          </div>
        </DialogContent>
      </Dialog>

      {/* ═══ FINIS ═══ */}
      <FinisMarker />
    </div>
  );
}

// ─── Attention Section (unified: suggested + actions + emails) ─────────────────

function AttentionSection({
  lifecycleUpdates,
  briefingCallouts,
  onConfirmLifecycle,
  onOpenLifecycleCorrection,
  pendingLifecycleChangeId,
  suggestedActions,
  acceptAction,
  rejectAction,
  focus,
  pendingActions,
  completedIds,
  onComplete,
  briefingEmails,
  emailSectionLabel,
  allEmails,
  todayMeetingIds,
  emailSyncTimestamp,
}: {
  lifecycleUpdates: DashboardLifecycleUpdate[];
  briefingCallouts: BriefingCallout[];
  onConfirmLifecycle: (update: DashboardLifecycleUpdate) => void;
  onOpenLifecycleCorrection: (update: DashboardLifecycleUpdate) => void;
  pendingLifecycleChangeId: number | null;
  suggestedActions: Array<{ id: string; title: string; sourceLabel?: string; sourceId?: string }>;
  acceptAction: (id: string) => void;
  rejectAction: (
    id: string,
    source?: "actions_page" | "daily_briefing" | "meeting_detail"
  ) => void;
  focus: DashboardData["focus"];
  pendingActions: Action[];
  completedIds: Set<string>;
  onComplete: (id: string) => void;
  briefingEmails: Email[];
  emailSectionLabel: string;
  allEmails: Email[];
  todayMeetingIds: Set<string>;
  emailSyncTimestamp?: string;
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

  const hasSuggested = suggestedActions.length > 0;
  const hasActions = attentionActions.length > 0;
  const hasEmails = briefingEmails.length > 0;
  const hasLifecycle = lifecycleUpdates.length > 0;
  // Callouts disabled: the signal propagation pipeline is populating
  // briefing_callouts with raw signal data (e.g., "Support health: tickets
  // updated") that violates ADR-0083 vocabulary rules. Until callouts are
  // filtered to curated, user-meaningful intelligence items, suppress them.
  const hasCallouts = false; // briefingCallouts.length > 0;
  const hasAnything = hasLifecycle || hasCallouts || hasSuggested || hasActions || hasEmails;

  if (!hasAnything) return null;

  // Determine if attentionActions are PrioritizedAction or raw Action.
  // Check the actual items in attentionActions, not the source array — the useMemo
  // may have fallen through to raw actions even when prioritizedActions exists.
  const hasPrioritizedActions = attentionActions.length > 0
    && attentionActions.every((item) => "action" in item && (item as PrioritizedAction).action?.id);

  return (
    <section className={s.prioritiesSection}>
      <div className={s.marginGrid}>
        <div className={s.marginLabel}>Attention</div>
        <div className={s.marginContent}>
          <div className={s.sectionRule} />


          {hasLifecycle && (
            <div className={briefingStyles.lifecycleGroup}>
              <div className={clsx(s.priorityGroupLabel, s.priorityGroupLabelToday)}>
                Lifecycle
              </div>
              <div className={s.priorityItems}>
                {lifecycleUpdates.slice(0, 3).map((update) => (
                  <LifecycleUpdateItem
                    key={update.changeId}
                    update={update}
                    pending={pendingLifecycleChangeId === update.changeId}
                    onConfirm={onConfirmLifecycle}
                    onCorrect={onOpenLifecycleCorrection}
                  />
                ))}
              </div>
            </div>
          )}

          {hasCallouts && (
            <div className={hasLifecycle ? briefingStyles.calloutsGroupSpaced : briefingStyles.calloutsGroupFlush}>
              <div className={clsx(s.priorityGroupLabel, s.priorityGroupLabelOverdue)}>
                Signals
              </div>
              <div className={s.priorityItems}>
                {briefingCallouts.slice(0, 5).map((callout) => (
                  <div key={callout.id} className={briefingStyles.calloutItem}>
                    <div className={briefingStyles.calloutSeverity} data-severity={callout.severity} />
                    <div className={briefingStyles.calloutContent}>
                      <span className={briefingStyles.calloutHeadline}>{callout.headline}</span>
                      {callout.entityName && (
                        <span className={briefingStyles.calloutEntity}>{callout.entityName}</span>
                      )}
                      {callout.detail && (
                        <span className={briefingStyles.calloutDetail}>{callout.detail}</span>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Suggested action triage (max 3) */}
          {hasSuggested && (
            <>
              <div className={briefingStyles.suggestedColumn}>
                {suggestedActions.slice(0, 3).map((action, i) => (
                  <SuggestedActionRow
                    key={action.id}
                    action={action}
                    onAccept={() => acceptAction(action.id)}
                    onReject={() => rejectAction(action.id, "daily_briefing")}
                    showBorder={i < Math.min(suggestedActions.length, 3) - 1}
                    compact
                  />
                ))}
              </div>
              {suggestedActions.length > 3 && (
                <button
                  onClick={() => navigate({ to: "/actions", search: { search: undefined } })}
                  className={briefingStyles.seeAllSuggestionsBtn}
                >
                  See all {suggestedActions.length} suggestions &rarr;
                </button>
              )}
            </>
          )}

          {/* Actions: meeting-relevant + overdue (max 3) */}
          {hasActions && (
            <div className={hasLifecycle || hasSuggested ? briefingStyles.actionsGroupSpaced : briefingStyles.actionsGroupFlush}>
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
                            className={clsx(s.priorityTitle, done ? briefingStyles.linkLineThrough : briefingStyles.linkNoDecoration)}
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

          {/* Emails: show scored/intelligence emails first, never raw repliesNeeded */}
          {hasEmails ? (
            <>
              <div className={clsx(s.priorityGroupLabel, s.priorityGroupLabelToday)}>
                {emailSectionLabel}
                {emailSyncTimestamp && (
                  <span className={briefingStyles.emailSyncTimestamp}>
                    as of {formatAsOfTime(emailSyncTimestamp)}
                  </span>
                )}
              </div>
              <div className={s.priorityItems}>
                {briefingEmails.map((email) => (
                  <PriorityEmailItem key={email.id} email={email} />
                ))}
              </div>
            </>
          ) : null}

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
        briefingStyles.linkNoDecoration,
      )}
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

// ─── Helpers ─────────────────────────────────────────────────────────────────

/** Format ISO timestamp as "X:XX AM" for the "as of" label. */
function formatAsOfTime(isoString: string): string {
  try {
    const date = new Date(isoString);
    return date.toLocaleTimeString(undefined, {
      hour: "numeric",
      minute: "2-digit",
      hour12: true,
    });
  } catch {
    return "";
  }
}

// ─── Priority Email Item ─────────────────────────────────────────────────────

function PriorityEmailItem({ email }: { email: Email }) {
  return (
    <Link
      to="/emails"
      className={clsx(s.priorityItem, s.priorityItemEmailType, briefingStyles.linkNoDecoration)}
    >
      <div
        className={clsx(
          s.priorityDot,
          email.priority === "high" ? s.priorityDotTerracotta : s.priorityDotTurmeric,
        )}
      />
      <div className={s.priorityContent}>
        {email.summary ? (
          <>
            <div className={s.priorityTitle}>{email.summary}</div>
            <div className={clsx(s.replyMeta, briefingStyles.emailMetaRow)}>
              {email.entityName && (
                <EmailEntityChip
                  entityType={email.entityType}
                  entityName={email.entityName}
                />
              )}
              {/* Only show sender when it adds info beyond entity name */}
              {(!email.entityName || !email.sender.includes(email.entityName)) && (
                <span>{email.sender}</span>
              )}
              {email.scoreReason && (() => {
                // Strip entity name from reason when chip already shows it
                const reason = email.entityName
                  ? email.scoreReason.replace(email.entityName, "").replace(/^[\s·]+|[\s·]+$/g, "")
                  : email.scoreReason;
                return reason ? <span className={s.emailScoreReason}>{reason}</span> : null;
              })()}
            </div>
          </>
        ) : (
          <div className={s.priorityTitle}>
            <span className={s.prioritySender}>{email.sender}</span>
            <span className={s.prioritySubjectSep}>&mdash;</span>
            <span>{email.subject}</span>
          </div>
        )}
      </div>
    </Link>
  );
}

function formatLifecycleLabel(value?: string | null) {
  return value ? value.replace(/_/g, " ") : "";
}

// ─── Lifecycle Update Item ──────────────────────────────────────────────────

function LifecycleUpdateItem({
  update,
  pending,
  onConfirm,
  onCorrect,
}: {
  update: DashboardLifecycleUpdate;
  pending: boolean;
  onConfirm: (update: DashboardLifecycleUpdate) => void;
  onCorrect: (update: DashboardLifecycleUpdate) => void;
}) {
  const transitionLabel = update.previousLifecycle
    ? `${formatLifecycleLabel(update.previousLifecycle)} → ${formatLifecycleLabel(update.newLifecycle)}`
    : formatLifecycleLabel(update.newLifecycle);
  const healthDelta = update.healthScoreBefore != null && update.healthScoreAfter != null
    ? `${Math.round(update.healthScoreBefore)} → ${Math.round(update.healthScoreAfter)}`
    : null;
  const contextBits = [
    update.renewalStage ? `Stage: ${update.renewalStage.replace(/_/g, " ")}` : null,
    healthDelta ? `Health ${healthDelta}` : null,
    update.actionState !== "pending"
      ? update.actionState.charAt(0).toUpperCase() + update.actionState.slice(1)
      : null,
    `${Math.round(update.confidence * 100)}% confidence`,
    formatShortDate(update.createdAt),
  ].filter(Boolean);

  return (
    <div
      className={clsx(s.priorityItem, s.priorityItemToday, s.priorityItemAccount)}
    >
      <div className={clsx(s.priorityDot, s.priorityDotTurmeric)} />
      <div className={s.priorityContent}>
        <Link
          to="/accounts/$accountId"
          params={{ accountId: update.accountId }}
          className={clsx(s.priorityTitle, briefingStyles.linkNoDecoration)}
        >
          {update.accountName}: {transitionLabel}
        </Link>
        <div className={s.priorityContext}>{contextBits.join(" · ")}</div>
        {update.evidence && (
          <div className={s.priorityWhy}>{update.evidence}</div>
        )}
        {update.actionState === "pending" ? (
          <div className={briefingStyles.lifecycleButtonRow}>
            <button
              type="button"
              onClick={() => onConfirm(update)}
              disabled={pending}
              className={briefingStyles.lifecycleConfirmBtn}
            >
              Looks good
            </button>
            <button
              type="button"
              onClick={() => onCorrect(update)}
              disabled={pending}
              className={briefingStyles.lifecycleCorrectBtn}
            >
              Fix something
            </button>
          </div>
        ) : null}
      </div>
    </div>
  );
}
