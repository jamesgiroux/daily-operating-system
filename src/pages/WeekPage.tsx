import { useState, useEffect, useCallback, useRef, useMemo } from "react";
import { BrandMark } from "@/components/ui/BrandMark";
import { invoke } from "@tauri-apps/api/core";
import { Skeleton } from "@/components/ui/skeleton";

import type {
  DayShape,
  TimelineMeeting,
  WeekOverview,
} from "@/types";
import { cn } from "@/lib/utils";
import {
  computeShapeEpigraph,
  computeWeekMeta,
  deriveShapeFromTimeline,
} from "@/pages/weekPageViewModel";
import w from "./WeekPage.module.css";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import { useTauriEvent } from "@/hooks/useTauriEvent";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { GeneratingProgress } from "@/components/editorial/GeneratingProgress";
import { MeetingCard } from "@/components/shared/MeetingCard";
import { formatDisplayTime, formatDurationFromIso } from "@/lib/meeting-time";
import { formatEntityByline } from "@/lib/entity-helpers";
import { FolioRefreshButton } from "@/components/ui/folio-refresh-button";
import {
  AlertTriangle,
  Database,
  Wand2,
  Package,
} from "lucide-react";

interface WeekResult {
  status: "success" | "not_found" | "error";
  data?: WeekOverview;
  message?: string;
}

type WorkflowPhase = "preparing" | "enriching" | "delivering";

const phaseSteps: {
  key: WorkflowPhase;
  label: string;
  icon: typeof Database;
}[] = [
  { key: "preparing", label: "Prepare", icon: Database },
  { key: "enriching", label: "Enrich", icon: Wand2 },
  { key: "delivering", label: "Deliver", icon: Package },
];

const waitingMessages = [
  `"You miss 100% of the shots you don't take." — Wayne Gretzky — Michael Scott`,
  "Combobulating your priorities...",
  "In a van, down by the river, preparing your week...",
  `"The secret of getting ahead is getting started." — Mark Twain`,
  "Manifesting your best week yet...",
  "Teaching the AI about your calendar...",
  `"It's not the load that breaks you down, it's the way you carry it." — Lou Holtz`,
  "Consulting the schedule oracle...",
  "Crunching context like it owes us money...",
  `"Preparation is the key to success." — Alexander Graham Bell`,
  "Cross-referencing all the things...",
  "Making meetings make sense since 2025...",
  `"By failing to prepare, you are preparing to fail." — Benjamin Franklin`,
  "Synthesizing the week ahead...",
  "Turning chaos into calendar clarity...",
  `"The best time to plant a tree was 20 years ago. The second best time is now."`,
  "Pondering your meetings with great intensity...",
  "Almost done thinking about thinking...",
  `"Plans are nothing; planning is everything." — Dwight D. Eisenhower`,
  "Polishing the details...",
];

export default function WeekPage() {
  const [data, setData] = useState<WeekOverview | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [running, setRunning] = useState(false);
  const [phase, setPhase] = useState<WorkflowPhase | null>(null);
  const [timeline, setTimeline] = useState<TimelineMeeting[]>([]);
  const [showEarlier, setShowEarlier] = useState(false);
  const loadingRef = useRef(false);

  const triggerMeetingEnrichment = useCallback(
    (meetings: TimelineMeeting[]) => {
      const now = new Date();
      const candidates = meetings
        .filter((m) => new Date(m.startTime) > now)
        .filter((m) => {
          const level = m.intelligenceQuality?.level;
          return level === "sparse" || level === "developing";
        })
        .slice(0, 5);

      candidates.forEach((m, i) => {
        setTimeout(() => {
          invoke("enrich_meeting_background", { meetingId: m.id }).catch(
            () => {}
          );
        }, i * 2000);
      });
    },
    []
  );

  const loadWeek = useCallback(
    async ({ includeLive = true }: { includeLive?: boolean } = {}) => {
      if (loadingRef.current) return;
      loadingRef.current = true;

      try {
        try {
          const timelineData = await invoke<TimelineMeeting[]>(
            "get_meeting_timeline",
            { daysBefore: 7, daysAfter: 7 }
          );
          console.log("[WeekPage] Timeline loaded:", timelineData?.length, "meetings");
          setTimeline(timelineData);
          if (includeLive) {
            triggerMeetingEnrichment(timelineData);
          }
        } catch (err) {
          console.error("[WeekPage] Timeline failed:", err);
          setTimeline([]);
        }

        try {
          const result = await invoke<WeekResult>("get_week_data");
          if (result.status === "success" && result.data) {
            setData(result.data);
            setError(null);
          } else if (result.status === "not_found") {
            setData(null);
          } else if (result.status === "error") {
            setError(result.message || "Failed to load week data");
          }
        } catch (err) {
          setError(err instanceof Error ? err.message : "Unknown error");
        } finally {
          setLoading(false);
        }
      } finally {
        loadingRef.current = false;
      }
    },
    [triggerMeetingEnrichment]
  );

  useEffect(() => {
    loadWeek();
    invoke<{ status: string; phase?: WorkflowPhase }>("get_workflow_status", {
      workflow: "week",
    })
      .then((status) => {
        if (status.status === "running") {
          setRunning(true);
          setPhase(status.phase ?? "preparing");
        }
      })
      .catch((err) => {
        console.error("get_workflow_status failed:", err);
      });
  }, [loadWeek]);

  useEffect(() => {
    if (!running) return;
    let sawRunning = false;

    const interval = setInterval(async () => {
      try {
        const status = await invoke<{
          status: string;
          phase?: WorkflowPhase;
          error?: { message: string; recoverySuggestion: string };
        }>("get_workflow_status", { workflow: "week" });

        if (status.status === "running") {
          sawRunning = true;
          if (status.phase) setPhase(status.phase);
          loadWeek({ includeLive: false });
        } else if (status.status === "completed" && sawRunning) {
          clearInterval(interval);
          setRunning(false);
          setPhase(null);
          loadWeek();
        } else if (status.status === "failed" && sawRunning) {
          clearInterval(interval);
          setRunning(false);
          setPhase(null);
          const msg = status.error?.message || "Week workflow failed";
          const hint = status.error?.recoverySuggestion;
          setError(hint ? `${msg}\n${hint}` : msg);
        } else if (status.status === "idle" && sawRunning) {
          clearInterval(interval);
          setRunning(false);
          setPhase(null);
          loadWeek({ includeLive: false });
        }
      } catch {
        // Ignore polling errors
      }
    }, 1000);

    const timeout = setTimeout(() => {
      clearInterval(interval);
      setRunning(false);
      setPhase(null);
      setError("Week workflow timed out. Check Settings for workflow status.");
    }, 300_000);

    return () => {
      clearInterval(interval);
      clearTimeout(timeout);
    };
  }, [running, loadWeek]);

  const handleRunWeek = useCallback(async () => {
    setRunning(true);
    setError(null);
    try {
      await invoke("run_workflow", { workflow: "week" });
    } catch (err) {
      setRunning(false);
      setError(
        err instanceof Error ? err.message : "Failed to queue week workflow"
      );
    }
  }, []);

  // Register magazine shell — larkspur atmosphere, global app nav
  const folioActions = useMemo(
    () => (
      <FolioRefreshButton
        onClick={handleRunWeek}
        loading={running}
        loadingLabel={phaseSteps.find((s) => s.key === phase)?.label ?? "Running\u2026"}
        title={running ? "Forecast in progress" : "Refresh weekly forecast"}
      />
    ),
    [handleRunWeek, running, phase]
  );

  // Readiness stats for FolioBar — driven by timeline intelligence quality
  const folioReadinessStats = useMemo(() => {
    const now = new Date();
    const futureMeetings = timeline.filter(
      (m) => new Date(m.startTime) > now
    );
    if (futureMeetings.length === 0) return undefined;

    const byLevel = { ready: 0, fresh: 0, developing: 0, sparse: 0 };
    futureMeetings.forEach((m) => {
      const level = m.intelligenceQuality?.level ?? "sparse";
      byLevel[level]++;
    });
    const readyCount = byLevel.ready + byLevel.fresh;
    const total = futureMeetings.length;

    const stats: { label: string; color: "sage" | "terracotta" }[] = [];
    if (readyCount === total) {
      stats.push({ label: `${total}/${total} ready`, color: "sage" });
    } else {
      stats.push({ label: `${readyCount} ready`, color: "sage" });
      const buildingCount = byLevel.developing + byLevel.sparse;
      if (buildingCount > 0) {
        stats.push({ label: `${buildingCount} building`, color: "terracotta" });
      }
    }
    return stats;
  }, [timeline]);

  const shellConfig = useMemo(
    () => {
      const meta = computeWeekMeta();
      const wn = data?.weekNumber ?? meta.weekNumber;
      const dr = data?.dateRange ?? meta.dateRange;
      return {
        folioLabel: "Weekly Forecast",
        atmosphereColor: "larkspur" as const,
        activePage: "week" as const,
        folioActions,
        folioDateText: `WEEK ${wn} \u00b7 ${dr.toUpperCase()}`,
        folioReadinessStats,
      };
    },
    [folioActions, data?.weekNumber, data?.dateRange, folioReadinessStats]
  );
  useRegisterMagazineShell(shellConfig);
  useRevealObserver(!loading && (!!data || timeline.length > 0), data ?? timeline);

  // Live event listeners — keep the page current without user action
  const silentRefresh = useCallback(() => loadWeek({ includeLive: false }), [loadWeek]);
  useTauriEvent("calendar-updated", silentRefresh);
  useTauriEvent("workflow-completed", silentRefresh);
  useTauriEvent("intelligence-updated", silentRefresh);
  useTauriEvent("prep-ready", silentRefresh);

  // ─── Derived data ──────────────────────────────────────────────────────────

  // Week meta: prefer AI data, fall back to date math
  const weekMeta = useMemo(() => {
    const computed = computeWeekMeta();
    return {
      weekNumber: data?.weekNumber ?? computed.weekNumber,
      dateRange: data?.dateRange ?? computed.dateRange,
    };
  }, [data?.weekNumber, data?.dateRange]);

  // Shape: AI dayShapes > mechanical from data.days > derived from timeline
  const shapeDays = useMemo((): DayShape[] => {
    if (data?.dayShapes?.length) return data.dayShapes;
    if (data?.days?.length) {
      return data.days.map((day) => {
        const count = day.meetings.length;
        const estMinutes = count * 45;
        return {
          date: day.date,
          dayName: day.dayName,
          meetingCount: count,
          meetingMinutes: estMinutes,
          density:
            count >= 5 ? "packed"
            : count >= 4 ? "heavy"
            : count >= 2 ? "moderate"
            : "light",
          meetings: day.meetings,
          availableBlocks: [],
        };
      });
    }
    return deriveShapeFromTimeline(timeline);
  }, [data?.dayShapes, data?.days, timeline]);

  const shapeEpigraph = useMemo(
    () => (shapeDays.length ? computeShapeEpigraph(shapeDays) : ""),
    [shapeDays]
  );

  // Readiness stats for header vitals — driven by timeline intelligence quality
  const readinessStats = useMemo(() => {
    const now = new Date();
    const futureMeetings = timeline.filter(
      (m) => new Date(m.startTime) > now
    );
    if (futureMeetings.length === 0) return [];

    const byLevel = { ready: 0, fresh: 0, developing: 0, sparse: 0 };
    futureMeetings.forEach((m) => {
      const level = m.intelligenceQuality?.level ?? "sparse";
      byLevel[level]++;
    });
    const readyCount = byLevel.ready + byLevel.fresh;
    const buildingCount = byLevel.developing + byLevel.sparse;

    const stats: { label: string; color: "sage" | "terracotta" }[] = [];
    stats.push({ label: `${readyCount} ready`, color: "sage" });
    if (buildingCount > 0) {
      stats.push({ label: `${buildingCount} building`, color: "terracotta" });
    }
    return stats;
  }, [timeline]);

  const totalMeetings = useMemo(() => {
    const now = new Date();
    return timeline.filter((m) => new Date(m.startTime) > now).length;
  }, [timeline]);

  // ─── Timeline grouping ────────────────────────────────────────────────────
  const timelineGroups = useMemo(() => {
    const now = new Date();
    const todayStr = now.toISOString().slice(0, 10);

    // Group meetings by date
    const byDate = new Map<string, TimelineMeeting[]>();
    for (const m of timeline) {
      const dateKey = m.startTime.slice(0, 10);
      if (!byDate.has(dateKey)) byDate.set(dateKey, []);
      byDate.get(dateKey)!.push(m);
    }

    // Sort dates
    const sortedDates = [...byDate.keys()].sort();

    const past: { dateKey: string; label: string; meetings: TimelineMeeting[] }[] = [];
    const today: { dateKey: string; label: string; meetings: TimelineMeeting[] }[] = [];
    const future: { dateKey: string; label: string; meetings: TimelineMeeting[] }[] = [];

    for (const dateKey of sortedDates) {
      const meetings = byDate.get(dateKey)!;
      const date = new Date(dateKey + "T12:00:00");
      const diffDays = Math.round(
        (date.getTime() - new Date(todayStr + "T12:00:00").getTime()) /
          (1000 * 60 * 60 * 24)
      );

      let label: string;
      if (diffDays === 0) {
        label = "Today";
      } else if (diffDays === -1) {
        label = "Yesterday";
      } else if (diffDays === 1) {
        label = "Tomorrow";
      } else if (diffDays < 0) {
        label = `${Math.abs(diffDays)} days ago \u2014 ${date.toLocaleDateString(
          "en-US",
          { weekday: "long", month: "short", day: "numeric" }
        )}`;
      } else {
        label = date.toLocaleDateString("en-US", {
          weekday: "long",
          month: "short",
          day: "numeric",
        });
      }

      const group = { dateKey, label, meetings };

      if (diffDays < 0) past.push(group);
      else if (diffDays === 0) today.push(group);
      else future.push(group);
    }

    // Split past into "earlier" (beyond 2 days) and "recent past"
    const earlierPast = past.filter((g) => {
      const diffDays = Math.round(
        (new Date(g.dateKey + "T12:00:00").getTime() -
          new Date(todayStr + "T12:00:00").getTime()) /
          (1000 * 60 * 60 * 24)
      );
      return diffDays < -2;
    });
    const recentPast = past.filter((g) => {
      const diffDays = Math.round(
        (new Date(g.dateKey + "T12:00:00").getTime() -
          new Date(todayStr + "T12:00:00").getTime()) /
          (1000 * 60 * 60 * 24)
      );
      return diffDays >= -2;
    });

    return { earlierPast, recentPast, today, future };
  }, [timeline]);

  // ─── Loading skeleton — matches new compact layout ──────────────────────────
  if (loading) {
    const skeletonBg = { background: "var(--color-rule-light)" };
    return (
      <div style={{ maxWidth: 760, margin: "0 auto", padding: "0 40px" }}>
        {/* Header skeleton */}
        <div style={{ paddingTop: 80 }}>
          <Skeleton className="h-3 w-20 mb-2" style={skeletonBg} />
          <Skeleton className="h-7 w-44 mb-6" style={skeletonBg} />
          <Skeleton className="h-3 w-52" style={skeletonBg} />
        </div>

        {/* Shape skeleton — 5 bars */}
        <div style={{ paddingTop: 56 }}>
          <Skeleton className="h-2.5 w-20 mb-4" style={skeletonBg} />
          {[1, 2, 3, 4, 5].map((i) => (
            <div key={i} style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 24 }}>
              <Skeleton className="h-3 w-9" style={skeletonBg} />
              <Skeleton className="h-2 flex-1 rounded-full" style={skeletonBg} />
              <Skeleton className="h-3 w-20" style={skeletonBg} />
            </div>
          ))}
        </div>

        {/* Timeline skeleton */}
        <div style={{ paddingTop: 64 }}>
          <div style={{ borderTop: "2px solid var(--color-rule-light)", marginBottom: 16 }} />
          <Skeleton className="h-7 w-36 mb-8" style={skeletonBg} />
          {[1, 2, 3].map((d) => (
            <div key={d} style={{ marginBottom: 20 }}>
              <Skeleton className="h-4 w-40 mb-3" style={skeletonBg} />
              <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 8, marginLeft: 12 }}>
                <Skeleton className="h-2 w-2 rounded-full" style={skeletonBg} />
                <Skeleton className="h-4 w-48" style={skeletonBg} />
                <Skeleton className="h-3 w-16" style={skeletonBg} />
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 12, marginLeft: 12 }}>
                <Skeleton className="h-2 w-2 rounded-full" style={skeletonBg} />
                <Skeleton className="h-4 w-36" style={skeletonBg} />
                <Skeleton className="h-3 w-16" style={skeletonBg} />
              </div>
            </div>
          ))}
        </div>

        {/* Finis skeleton */}
        <div style={{ textAlign: "center", padding: "72px 0 48px" }}>
          <Skeleton className="mx-auto h-4 w-16" style={skeletonBg} />
        </div>
      </div>
    );
  }

  // ─── Empty state — only when truly no data (no timeline either) ────────────
  if (!data && timeline.length === 0) {
    return (
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          minHeight: "70vh",
          textAlign: "center",
          padding: "80px 40px",
        }}
      >
        {running ? (
          <WorkflowProgress phase={phase ?? "preparing"} />
        ) : (
          <>
            <div
              style={{
                color: "var(--color-garden-larkspur)",
                marginBottom: 24,
              }}
            >
              <BrandMark size={48} />
            </div>
            <p
              style={{
                fontFamily: "var(--font-serif)",
                fontSize: 24,
                fontWeight: 400,
                color: "var(--color-text-primary)",
                marginBottom: 8,
              }}
            >
              No forecast yet
            </p>
            <p
              style={{
                fontFamily: "var(--font-serif)",
                fontSize: 15,
                fontStyle: "italic",
                color: "var(--color-text-tertiary)",
                marginBottom: 32,
                maxWidth: 360,
              }}
            >
              Connect your calendar to see your week.
            </p>
          </>
        )}
        {error && <ErrorCard error={error} />}
      </div>
    );
  }

  // ─── Workflow running — full-page takeover ──────────────────────────────────
  if (running) {
    return (
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          minHeight: "70vh",
          textAlign: "center",
          padding: "80px 40px",
        }}
      >
        <WorkflowProgress phase={phase ?? "preparing"} />
        {error && <ErrorCard error={error} />}
      </div>
    );
  }

  // ─── Full editorial render ─────────────────────────────────────────────────
  return (
    <>
      <div style={{ maxWidth: 760, margin: "0 auto", padding: "0 40px" }}>
        {/* ── Week Header (compact, left-aligned) ───────────────────── */}
        <section className={w.weekHeader}>
          <p className={w.weekLabel}>Week {weekMeta.weekNumber}</p>
          <p className={w.weekDate}>{weekMeta.dateRange}</p>
          {(readinessStats.length > 0 || totalMeetings > 0) && (
            <p className={w.weekVitals}>
              {readinessStats.map((stat) => (
                <span
                  key={stat.label}
                  className={stat.color === "sage" ? w.vitalsReady : w.vitalsAlert}
                >
                  {stat.label}
                </span>
              ))}
              {totalMeetings > 0 && (
                <span>
                  {totalMeetings} meeting{totalMeetings !== 1 ? "s" : ""}
                </span>
              )}
            </p>
          )}
        </section>

        {/* ── This Week — The Shape ────────────────────────────────────── */}
        {shapeDays.length > 0 && (
          <section className={cn("editorial-reveal", w.shapeSection)}>
            <p className={w.shapeLabel}>This Week</p>
            <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>
              {shapeDays.map((day) => {
                const barPct = Math.min(100, (day.meetingMinutes / 480) * 100);
                const isHeavy =
                  day.meetingCount >= 5 || day.meetingMinutes >= 360;
                const todayStr = new Date().toISOString().slice(0, 10);
                const isToday = day.date === todayStr;
                const isPast = day.date < todayStr;

                const feasible =
                  day.focusImplications?.achievableCount ?? 0;
                const total = day.focusImplications?.totalCount ?? 0;
                const achievabilityGood =
                  total === 0 || feasible / total >= 0.5;

                return (
                  <div
                    key={day.date}
                    className={cn(
                      w.shapeRow,
                      isPast && w.shapeRowPast,
                    )}
                  >
                    <span
                      className={cn(
                        w.shapeDayLabel,
                        isToday && w.shapeDayLabelToday,
                      )}
                    >
                      {day.dayName.slice(0, 3)}
                    </span>

                    <div className={w.shapeBar}>
                      <div
                        className={cn(
                          w.shapeBarFill,
                          isToday
                            ? w.shapeBarFillToday
                            : isHeavy
                              ? w.shapeBarFillHeavy
                              : undefined,
                        )}
                        style={{ width: `${barPct}%` }}
                      />
                    </div>

                    <span className={w.shapeCount}>
                      {day.meetingCount}m &middot; {day.density}
                    </span>

                    {total > 0 && (
                      <span
                        className={cn(
                          w.shapeAchievability,
                          achievabilityGood
                            ? w.shapeAchievabilityGood
                            : w.shapeAchievabilityAlert,
                        )}
                      >
                        {feasible}/{total}
                      </span>
                    )}
                  </div>
                );
              })}
            </div>
            {shapeEpigraph && (
              <p className={w.shapeEpigraph}>{shapeEpigraph}</p>
            )}
          </section>
        )}

        {/* ── Chapter 3: The Timeline ──────────────────────────────────── */}
        {timeline.length > 0 && (
          <section
            id="the-timeline"
            className="editorial-reveal"
            style={{ paddingTop: 64 }}
          >
            <ChapterHeading
              title="The Timeline"
              epigraph="Context across your meetings, ±7 days."
            />

            {/* Past — Earlier (collapsed) */}
            {timelineGroups.earlierPast.length > 0 && (
              <div style={{ marginBottom: 24 }}>
                <div
                  role="button"
                  tabIndex={0}
                  onClick={() => setShowEarlier(!showEarlier)}
                  onKeyDown={(e) => e.key === "Enter" && setShowEarlier(!showEarlier)}
                  style={{
                    display: "flex",
                    justifyContent: "space-between",
                    alignItems: "baseline",
                    borderBottom: "1px solid var(--color-rule-heavy)",
                    paddingBottom: 6,
                    marginBottom: showEarlier ? 16 : 0,
                    cursor: "pointer",
                  }}
                >
                  <p
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 11,
                      letterSpacing: "0.08em",
                      textTransform: "uppercase",
                      color: "var(--color-text-tertiary)",
                      margin: 0,
                    }}
                  >
                    Earlier
                  </p>
                  <span
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 11,
                      color: "var(--color-text-tertiary)",
                    }}
                  >
                    {showEarlier
                      ? "hide"
                      : `${timelineGroups.earlierPast.reduce((n, g) => n + g.meetings.length, 0)} meetings`}
                  </span>
                </div>
                {showEarlier &&
                  timelineGroups.earlierPast.map((group) => (
                    <TimelineDayGroup
                      key={group.dateKey}
                      label={group.label}
                      meetings={group.meetings}
                      isPast
                    />
                  ))}
              </div>
            )}

            {/* Past — Recent (last 2 days) */}
            {timelineGroups.recentPast.length > 0 && (
              <div>
                <p
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 11,
                    letterSpacing: "0.08em",
                    textTransform: "uppercase",
                    color: "var(--color-text-tertiary)",
                    marginBottom: 16,
                    borderBottom: "1px solid var(--color-rule-heavy)",
                    paddingBottom: 6,
                  }}
                >
                  Earlier
                </p>
                {timelineGroups.recentPast.map((group) => (
                  <TimelineDayGroup
                    key={group.dateKey}
                    label={group.label}
                    meetings={group.meetings}
                    isPast
                  />
                ))}
              </div>
            )}

            {/* Today */}
            {timelineGroups.today.length > 0 && (
              <div style={{ marginTop: 24 }}>
                <p
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 11,
                    letterSpacing: "0.08em",
                    textTransform: "uppercase",
                    color: "var(--color-garden-larkspur)",
                    marginBottom: 16,
                    borderBottom: "2px solid var(--color-garden-larkspur)",
                    paddingBottom: 6,
                  }}
                >
                  Today
                </p>
                {timelineGroups.today.map((group) => (
                  <TimelineDayGroup
                    key={group.dateKey}
                    label={group.label}
                    meetings={group.meetings}
                    isToday
                  />
                ))}
              </div>
            )}

            {/* Future */}
            {timelineGroups.future.length > 0 && (
              <div style={{ marginTop: 24 }}>
                <p
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 11,
                    letterSpacing: "0.08em",
                    textTransform: "uppercase",
                    color: "var(--color-text-tertiary)",
                    marginBottom: 16,
                    borderBottom: "1px solid var(--color-rule-heavy)",
                    paddingBottom: 6,
                  }}
                >
                  Ahead
                </p>
                {timelineGroups.future.map((group) => (
                  <TimelineDayGroup
                    key={group.dateKey}
                    label={group.label}
                    meetings={group.meetings}
                  />
                ))}
              </div>
            )}
          </section>
        )}

        {/* ── Error display ──────────────────────────────────────────────── */}
        {error && (
          <div style={{ marginTop: 48 }}>
            <ErrorCard error={error} />
          </div>
        )}

        {/* ── Finis ──────────────────────────────────────────────────────── */}
        <section className="editorial-reveal">
          <FinisMarker enrichedAt={undefined} />
          <p
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 15,
              fontStyle: "italic",
              color: "var(--color-text-tertiary)",
              textAlign: "center",
              paddingBottom: 48,
            }}
          >
            Your week at a glance.
          </p>
        </section>
      </div>

    </>
  );
}

// ---------------------------------------------------------------------------
// Chrome
// ---------------------------------------------------------------------------

function ErrorCard({ error }: { error: string }) {
  return (
    <div className="mt-6 max-w-md rounded-lg border border-destructive p-4 text-left">
      <div className="flex items-start gap-2">
        <AlertTriangle className="mt-0.5 size-4 shrink-0 text-destructive" />
        <div className="min-w-0 space-y-1">
          {error.split("\n").map((line, i) => (
            <p
              key={i}
              className={cn(
                "text-sm",
                i === 0 ? "text-destructive" : "text-muted-foreground"
              )}
            >
              {line}
            </p>
          ))}
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Timeline Components
// ---------------------------------------------------------------------------

/** Compute days until a meeting from now. */
function computeDaysUntil(startTime: string): number | null {
  try {
    const now = new Date();
    const start = new Date(startTime);
    const diffMs = start.getTime() - now.getTime();
    if (diffMs < 0) return null;
    return Math.ceil(diffMs / (1000 * 60 * 60 * 24));
  } catch {
    return null;
  }
}

function TimelineDayGroup({
  label,
  meetings,
  isPast,
  isToday,
}: {
  label: string;
  meetings: TimelineMeeting[];
  isPast?: boolean;
  isToday?: boolean;
}) {
  return (
    <div style={{ marginBottom: 20 }}>
      <p
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: isToday ? 16 : 14,
          fontWeight: isToday ? 500 : 400,
          color: isToday
            ? "var(--color-text-primary)"
            : "var(--color-text-secondary)",
          margin: "0 0 8px",
          ...(isToday
            ? {
                borderLeft: "3px solid var(--color-garden-larkspur)",
                paddingLeft: 12,
              }
            : {}),
        }}
      >
        {label}
      </p>
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          gap: 0,
          paddingLeft: isToday ? 15 : 0,
        }}
      >
        {meetings.map((m) => {
          const daysUntil = !isPast ? computeDaysUntil(m.startTime) : null;
          const isSparse = m.intelligenceQuality?.level === "sparse";

          return (
            <MeetingCard
              key={m.id}
              id={m.id}
              title={m.title}
              displayTime={formatDisplayTime(m.startTime)}
              duration={formatDurationFromIso(m.startTime, m.endTime) ?? undefined}
              meetingType={m.meetingType}
              entityByline={m.entities.length > 0 ? formatEntityByline(m.entities) ?? undefined : undefined}
              intelligenceQuality={!isPast ? (m.intelligenceQuality ?? undefined) : undefined}
              temporalState={isPast ? "past" : undefined}
              showNavigationHint={isPast}
              subtitleExtra={
                !isPast ? (
                  <>
                    {isSparse && (
                      <span
                        style={{
                          fontFamily: "var(--font-mono)",
                          fontSize: 10,
                          fontWeight: 600,
                          letterSpacing: "0.04em",
                          color: "var(--color-spice-terracotta)",
                          background: "rgba(192, 108, 80, 0.08)",
                          borderRadius: 4,
                          padding: "1px 6px",
                        }}
                      >
                        No prep
                      </span>
                    )}
                    {daysUntil != null && daysUntil > 0 && (
                      <span
                        style={{
                          fontFamily: "var(--font-mono)",
                          fontSize: 10,
                          fontWeight: 500,
                          color: "var(--color-text-tertiary)",
                        }}
                      >
                        {daysUntil === 1 ? "1 day" : `${daysUntil} days`}
                      </span>
                    )}
                  </>
                ) : undefined
              }
            >
              {/* Past meetings: outcome summary + follow-up count */}
              {isPast && m.hasOutcomes && m.outcomeSummary && (
                <span
                  style={{
                    display: "inline-flex",
                    alignItems: "center",
                    gap: 4,
                    fontFamily: "var(--font-mono)",
                    fontSize: 11,
                    color: "var(--color-garden-sage)",
                    marginTop: 4,
                  }}
                >
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                    <polyline points="20 6 9 17 4 12" />
                  </svg>
                  <span style={{ maxWidth: 240, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                    {m.outcomeSummary}
                  </span>
                  {m.followUpCount != null && m.followUpCount > 0 && (
                    <span
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 10,
                        fontWeight: 600,
                        color: "var(--color-garden-sage)",
                        background: "rgba(122, 151, 122, 0.10)",
                        borderRadius: 4,
                        padding: "1px 6px",
                        marginLeft: 4,
                      }}
                    >
                      {m.followUpCount} follow-up{m.followUpCount !== 1 ? "s" : ""}
                    </span>
                  )}
                </span>
              )}
              {/* Past meetings: show follow-up count even without outcome summary */}
              {isPast && !(m.hasOutcomes && m.outcomeSummary) && m.followUpCount != null && m.followUpCount > 0 && (
                <span
                  style={{
                    display: "inline-flex",
                    alignItems: "center",
                    gap: 4,
                    fontFamily: "var(--font-mono)",
                    fontSize: 10,
                    fontWeight: 600,
                    color: "var(--color-garden-sage)",
                    background: "rgba(122, 151, 122, 0.10)",
                    borderRadius: 4,
                    padding: "1px 6px",
                    marginTop: 4,
                  }}
                >
                  {m.followUpCount} follow-up{m.followUpCount !== 1 ? "s" : ""}
                </span>
              )}
            </MeetingCard>
          );
        })}
      </div>
    </div>
  );
}

// Timeline meetings now use shared MeetingCard (I362/I364)

const FORECAST_PHASES = [
  { key: "preparing", label: "Reading your calendar", detail: "Fetching meetings, classifying events, gathering account context" },
  { key: "enriching", label: "Writing the forecast", detail: "Synthesizing narrative, priorities, and time suggestions from your week" },
  { key: "delivering", label: "Assembling the briefing", detail: "Building day shapes, readiness checks, and commitment summaries" },
];

function WorkflowProgress({ phase }: { phase: WorkflowPhase }) {
  return (
    <GeneratingProgress
      title="Building Weekly Forecast"
      accentColor="var(--color-garden-larkspur)"
      phases={FORECAST_PHASES}
      currentPhaseKey={phase}
      quotes={waitingMessages}
    />
  );
}
