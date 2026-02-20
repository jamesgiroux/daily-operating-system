import { useState, useEffect, useCallback, useRef, useMemo } from "react";
import { BrandMark } from "@/components/ui/BrandMark";
import { Link } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";

import type {
  LiveProactiveSuggestion,
  TimelineMeeting,
  WeekOverview,
} from "@/types";
import { cn } from "@/lib/utils";
import {
  pickTopThree,
  resolveSuggestionLink,
  synthesizeReadiness,
} from "@/pages/weekPageViewModel";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { GeneratingProgress } from "@/components/editorial/GeneratingProgress";
import { IntelligenceQualityBadge } from "@/components/entity/IntelligenceQualityBadge";
import {
  Play,
  AlertTriangle,
  Database,
  Wand2,
  Package,
  Check,
  ChevronDown,
  ChevronRight,
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

// Circled number glyphs for The Three
const CIRCLED_NUMBERS = ["\u2460", "\u2461", "\u2462"];

export default function WeekPage() {
  const [data, setData] = useState<WeekOverview | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [_liveError, setLiveError] = useState<string | null>(null);
  const [liveSuggestions, setLiveSuggestions] = useState<
    LiveProactiveSuggestion[]
  >([]);
  const [running, setRunning] = useState(false);
  const [phase, setPhase] = useState<WorkflowPhase | null>(null);
  const [retryingEnrichment, setRetryingEnrichment] = useState(false);
  const [timeline, setTimeline] = useState<TimelineMeeting[]>([]);
  const [showEarlier, setShowEarlier] = useState(false);
  const loadingRef = useRef(false);

  const loadWeek = useCallback(
    async ({ includeLive = true }: { includeLive?: boolean } = {}) => {
      if (loadingRef.current) return;
      loadingRef.current = true;

      try {
        if (includeLive) {
          try {
            const live = await invoke<LiveProactiveSuggestion[]>(
              "get_live_proactive_suggestions"
            );
            setLiveSuggestions(live);
            setLiveError(null);
          } catch (err) {
            setLiveSuggestions([]);
            setLiveError(
              err instanceof Error
                ? err.message
                : "Live suggestions unavailable"
            );
          }
        }

        try {
          const timelineData = await invoke<TimelineMeeting[]>(
            "get_meeting_timeline",
            { daysBefore: 7, daysAfter: 7 }
          );
          setTimeline(timelineData);
        } catch {
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
    []
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
      .catch(() => {});
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

  const handleRetryEnrichment = useCallback(async () => {
    setRetryingEnrichment(true);
    setError(null);
    try {
      await invoke("retry_week_enrichment");
      await loadWeek();
    } catch (err) {
      setError(
        err instanceof Error
          ? err.message
          : "Failed to retry week enrichment",
      );
    } finally {
      setRetryingEnrichment(false);
    }
  }, [loadWeek]);

  // Register magazine shell — larkspur atmosphere, global app nav
  const folioActions = useMemo(
    () => (
      <button
        onClick={handleRunWeek}
        disabled={running}
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 11,
          fontWeight: 600,
          letterSpacing: "0.06em",
          textTransform: "uppercase" as const,
          color: "var(--color-garden-larkspur)",
          background: "none",
          border: "1px solid var(--color-garden-larkspur)",
          borderRadius: 4,
          padding: "2px 10px",
          cursor: running ? "wait" : "pointer",
          opacity: running ? 0.6 : 1,
        }}
      >
        {running
          ? (phaseSteps.find((s) => s.key === phase)?.label ?? "Running...")
          : "Refresh"}
      </button>
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
    () => ({
      folioLabel: "Weekly Forecast",
      atmosphereColor: "larkspur" as const,
      activePage: "week" as const,
      folioActions,
      folioDateText: data
        ? `WEEK ${data.weekNumber} \u00b7 ${data.dateRange.toUpperCase()}`
        : undefined,
      folioReadinessStats,
    }),
    [folioActions, data, folioReadinessStats]
  );
  useRegisterMagazineShell(shellConfig);
  useRevealObserver(!loading && !!data, data);

  // ─── Derived data ──────────────────────────────────────────────────────────
  const enrichmentIncomplete = data && !running && (!data.weekNarrative || !data.topPriority);
  const days = data?.days ?? [];

  const topThree = useMemo(
    () =>
      data
        ? pickTopThree(
            data.topPriority,
            data.actionSummary?.overdue ?? [],
            data.actionSummary?.dueThisWeekItems ?? [],
            liveSuggestions,
            days
          )
        : [],
    [data, liveSuggestions, days]
  );

  const readinessLine = useMemo(
    () =>
      data?.readinessChecks?.length
        ? synthesizeReadiness(data.readinessChecks)
        : null,
    [data?.readinessChecks]
  );

  const totalMeetings = useMemo(
    () => days.reduce((sum, d) => sum + d.meetings.length, 0),
    [days]
  );

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

  // ─── Loading skeleton — editorial shaped ────────────────────────────────────
  if (loading) {
    const skeletonBg = { background: "var(--color-rule-light)" };
    return (
      <div style={{ maxWidth: 760, margin: "0 auto", padding: "0 40px" }}>
        {/* Hero skeleton */}
        <div style={{ paddingTop: 80, textAlign: "center" }}>
          {/* Week number */}
          <Skeleton className="mx-auto h-3 w-44 mb-8" style={skeletonBg} />
          {/* Narrative lines */}
          <Skeleton className="mx-auto h-8 w-[520px] mb-3" style={skeletonBg} />
          <Skeleton className="mx-auto h-8 w-[440px] mb-3" style={skeletonBg} />
          <Skeleton className="mx-auto h-8 w-[280px] mb-10" style={skeletonBg} />
          {/* Vitals */}
          <Skeleton className="mx-auto h-3 w-52" style={skeletonBg} />
        </div>

        {/* Chapter 2: The Three skeleton */}
        <div style={{ paddingTop: 64 }}>
          <div style={{ borderTop: "2px solid var(--color-rule-light)", marginBottom: 16 }} />
          <Skeleton className="h-7 w-32 mb-8" style={skeletonBg} />
          {[1, 2, 3].map((i) => (
            <div key={i} style={{ display: "flex", gap: 16, marginBottom: 36 }}>
              <Skeleton className="h-5 w-5 rounded-full shrink-0" style={skeletonBg} />
              <div style={{ flex: 1 }}>
                <Skeleton className="h-5 w-64 mb-2" style={skeletonBg} />
                <Skeleton className="h-4 w-full mb-1" style={skeletonBg} />
                <Skeleton className="h-3 w-40" style={skeletonBg} />
              </div>
            </div>
          ))}
        </div>

        {/* Chapter 3: Timeline skeleton */}
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

  // ─── Empty state ───────────────────────────────────────────────────────────
  if (!data) {
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
              Generate your weekly forecast to see what your week actually means.
            </p>
            <Button className="gap-1.5" onClick={handleRunWeek}>
              <Play className="size-3.5" />
              Run Weekly Forecast
            </Button>
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
        {/* ── Chapter 1: Hero ────────────────────────────────────────────── */}
        <section style={{ paddingTop: 80, textAlign: "center" }}>
          {/* Week number + date range */}
          <p
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 13,
              letterSpacing: "0.08em",
              textTransform: "uppercase",
              color: "var(--color-text-tertiary)",
              marginBottom: 32,
            }}
          >
            WEEK {data.weekNumber} &middot; {data.dateRange.toUpperCase()}
          </p>

          {/* Narrative headline */}
          {data.weekNarrative ? (
            <p
              className="editorial-reveal"
              style={{
                fontFamily: "var(--font-serif)",
                fontSize: 32,
                fontWeight: 400,
                lineHeight: 1.4,
                letterSpacing: "-0.02em",
                color: "var(--color-text-primary)",
                maxWidth: 680,
                margin: "0 auto 32px",
              }}
            >
              {data.weekNarrative}
            </p>
          ) : (
            <p
              style={{
                fontFamily: "var(--font-serif)",
                fontSize: 24,
                fontWeight: 400,
                fontStyle: "italic",
                color: "var(--color-text-tertiary)",
                margin: "0 auto 32px",
                maxWidth: 480,
              }}
            >
              Enrichment pending. Mechanical data available below.
            </p>
          )}

          {/* Vitals line */}
          <p
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 12,
              color: "var(--color-text-tertiary)",
              marginBottom: 8,
            }}
          >
            {readinessLine && <span>{readinessLine}</span>}
            {readinessLine && totalMeetings > 0 && <span> &middot; </span>}
            {totalMeetings > 0 && (
              <span>
                {totalMeetings} meeting{totalMeetings !== 1 ? "s" : ""}
              </span>
            )}
          </p>

          {/* Enrichment incomplete notice */}
          {enrichmentIncomplete && (
            <p
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 11,
                color: "var(--color-text-tertiary)",
                marginTop: 16,
              }}
            >
              Analysis incomplete.{" "}
              <button
                onClick={handleRetryEnrichment}
                disabled={retryingEnrichment}
                style={{
                  background: "none",
                  border: "none",
                  color: "var(--color-garden-larkspur)",
                  cursor: "pointer",
                  fontFamily: "inherit",
                  fontSize: "inherit",
                  textDecoration: "underline",
                  padding: 0,
                }}
              >
                {retryingEnrichment ? "Retrying..." : "Retry"}
              </button>
            </p>
          )}

        </section>

        {/* ── Chapter 2: The Three ───────────────────────────────────────── */}
        <section
          id="the-three"
          className="editorial-reveal"
          style={{ paddingTop: 64 }}
        >
          <ChapterHeading
            title="The Three"
            epigraph="If everything is important, nothing is."
          />

          {topThree.length > 0 ? (
            <div style={{ display: "flex", flexDirection: "column", gap: 36 }}>
              {topThree.map((item) => {
                const linkTarget = resolveSuggestionLink(
                  item.actionId,
                  item.meetingId
                );
                const content = (
                  <>
                    <div style={{ display: "flex", gap: 16, alignItems: "flex-start" }}>
                      <span
                        style={{
                          fontFamily: "var(--font-mono)",
                          fontSize: 20,
                          lineHeight: 1,
                          color: "var(--color-garden-larkspur)",
                          flexShrink: 0,
                          marginTop: 2,
                        }}
                      >
                        {CIRCLED_NUMBERS[item.number - 1]}
                      </span>
                      <div style={{ minWidth: 0 }}>
                        <p
                          style={{
                            fontFamily: "var(--font-serif)",
                            fontSize: 17,
                            fontWeight: 500,
                            lineHeight: 1.4,
                            color: "var(--color-text-primary)",
                            margin: 0,
                          }}
                        >
                          {item.title}
                        </p>
                        <p
                          style={{
                            fontFamily: "var(--font-sans)",
                            fontSize: 14,
                            lineHeight: 1.6,
                            color: "var(--color-text-secondary)",
                            margin: "6px 0 0",
                          }}
                        >
                          {item.reason}
                        </p>
                        {item.contextLine && (
                          <p
                            style={{
                              fontFamily: "var(--font-mono)",
                              fontSize: 13,
                              color: "var(--color-text-tertiary)",
                              margin: "8px 0 0",
                            }}
                          >
                            &rarr; {item.contextLine}
                          </p>
                        )}
                      </div>
                    </div>
                  </>
                );

                if (linkTarget.kind === "action") {
                  return (
                    <Link
                      key={item.number}
                      to="/actions/$actionId"
                      params={{ actionId: linkTarget.id }}
                      style={{ textDecoration: "none", color: "inherit" }}
                    >
                      {content}
                    </Link>
                  );
                }
                if (linkTarget.kind === "meeting") {
                  return (
                    <Link
                      key={item.number}
                      to="/meeting/$meetingId"
                      params={{ meetingId: linkTarget.id }}
                      style={{ textDecoration: "none", color: "inherit" }}
                    >
                      {content}
                    </Link>
                  );
                }
                return <div key={item.number}>{content}</div>;
              })}
            </div>
          ) : (
            <p
              style={{
                fontFamily: "var(--font-serif)",
                fontSize: 15,
                fontStyle: "italic",
                color: "var(--color-text-tertiary)",
              }}
            >
              Refresh to see your three priorities.
            </p>
          )}
        </section>

        {/* ── Chapter 3: The Timeline ──────────────────────────────────── */}
        {timeline.length > 0 && (
          <section
            id="the-timeline"
            className="editorial-reveal"
            style={{ paddingTop: 64 }}
          >
            <ChapterHeading
              title="The Timeline"
              epigraph="Intelligence across your meetings, past and future."
            />

            {/* Past — Earlier (collapsed) */}
            {timelineGroups.earlierPast.length > 0 && (
              <div style={{ marginBottom: 24 }}>
                <button
                  onClick={() => setShowEarlier(!showEarlier)}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: 6,
                    background: "none",
                    border: "none",
                    cursor: "pointer",
                    fontFamily: "var(--font-mono)",
                    fontSize: 11,
                    letterSpacing: "0.06em",
                    textTransform: "uppercase",
                    color: "var(--color-text-tertiary)",
                    padding: "4px 0",
                  }}
                >
                  {showEarlier ? (
                    <ChevronDown size={14} />
                  ) : (
                    <ChevronRight size={14} />
                  )}
                  {showEarlier
                    ? "Hide earlier"
                    : `Show earlier (${timelineGroups.earlierPast.reduce(
                        (n, g) => n + g.meetings.length,
                        0
                      )} meetings)`}
                </button>
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
                  Past
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
                  Upcoming
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
            Your week is forecasted.
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
          gap: 6,
          paddingLeft: isToday ? 15 : 0,
        }}
      >
        {meetings.map((m) => (
          <TimelineMeetingRow key={m.id} meeting={m} isPast={isPast} />
        ))}
      </div>
    </div>
  );
}

function TimelineMeetingRow({
  meeting,
  isPast,
}: {
  meeting: TimelineMeeting;
  isPast?: boolean;
}) {
  const entityLabel =
    meeting.entities.length > 0
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
      {/* Meeting title — clickable */}
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

      {/* Entity name */}
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

      {/* Spacer */}
      <span style={{ flex: 1 }} />

      {/* Intelligence quality badge */}
      {quality && (
        <IntelligenceQualityBadge quality={quality} showLabel />
      )}

      {/* Past: outcome indicator */}
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
          <Check size={12} />
          {meeting.outcomeSummary ? (
            <span
              style={{
                maxWidth: 180,
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
            >
              {meeting.outcomeSummary}
            </span>
          ) : (
            "captured"
          )}
        </span>
      )}

      {/* Future: new signals indicator */}
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

      {/* Future: prior meeting link */}
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
