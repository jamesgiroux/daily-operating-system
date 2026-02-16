import { useState, useEffect, useCallback, useRef, useMemo } from "react";
import { BrandMark } from "@/components/ui/BrandMark";
import { Link } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import {
  AgendaDraftDialog,
  useAgendaDraft,
} from "@/components/ui/agenda-draft-dialog";

import type {
  ApplyPrepPrefillResult,
  LiveProactiveSuggestion,
  WeekOverview,
  WeekAction,
} from "@/types";
import { cn } from "@/lib/utils";
import {
  computeDeepWorkHours,
  computeShapeEpigraph,
  countExternalMeetings,
  countMeetingAccounts,
  filterDeepWorkBlocks,
  filterRelevantMeetings,
  formatBlockRange,
  formatDueContext,
  formatPrepStatus,
  pickTopThree,
  resolveSuggestionLink,
  synthesizeReadiness,
  synthesizeReadinessStats,
} from "@/pages/weekPageViewModel";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { PullQuote } from "@/components/editorial/PullQuote";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { GeneratingProgress } from "@/components/editorial/GeneratingProgress";
import {
  Target,
  Users,
  Clock,
  CheckSquare,
  BarChart,
  Play,
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

// Chapter definitions for the editorial layout
const CHAPTERS = [
  { id: "the-three", label: "The Three", icon: <Target size={18} strokeWidth={1.5} /> },
  { id: "the-shape", label: "The Shape", icon: <BarChart size={18} strokeWidth={1.5} /> },
  { id: "your-meetings", label: "Meetings", icon: <Users size={18} strokeWidth={1.5} /> },
  { id: "open-time", label: "Open Time", icon: <Clock size={18} strokeWidth={1.5} /> },
  { id: "commitments", label: "Commitments", icon: <CheckSquare size={18} strokeWidth={1.5} /> },
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
  const [prefillingMeetingId, setPrefillingMeetingId] = useState<string | null>(
    null
  );
  const draft = useAgendaDraft({ onError: setError });
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

  const handlePrefillPrep = useCallback(
    async (meetingId: string, suggestionText: string, reasonText?: string) => {
      setPrefillingMeetingId(meetingId);
      try {
        await invoke<ApplyPrepPrefillResult>("apply_meeting_prep_prefill", {
          meetingId,
          agendaItems: [suggestionText],
          notesAppend: reasonText || "",
        });
      } catch (err) {
        setError(
          err instanceof Error ? err.message : "Failed to prefill meeting prep"
        );
      } finally {
        setPrefillingMeetingId(null);
      }
    },
    []
  );

  // Register magazine shell — larkspur atmosphere, chapter mode
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

  // Readiness stats for FolioBar
  const folioReadinessStats = useMemo(() => {
    if (!data?.readinessChecks?.length) return undefined;
    const { preppedLabel, overdueLabel } = synthesizeReadinessStats(
      data.readinessChecks
    );
    const stats: { label: string; color: "sage" | "terracotta" }[] = [
      { label: preppedLabel, color: "sage" },
    ];
    if (overdueLabel) {
      stats.push({ label: overdueLabel, color: "terracotta" });
    }
    return stats;
  }, [data?.readinessChecks]);

  const shellConfig = useMemo(
    () => ({
      folioLabel: "Weekly Forecast",
      atmosphereColor: "larkspur" as const,
      activePage: "week" as const,
      chapters: CHAPTERS,
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
  const dayShapes = data?.dayShapes ?? [];
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

  const meetingDays = useMemo(() => filterRelevantMeetings(days), [days]);
  const externalCount = useMemo(() => countExternalMeetings(days), [days]);
  const accountCount = useMemo(() => countMeetingAccounts(days), [days]);

  const deepWorkBlocks = useMemo(
    () => filterDeepWorkBlocks(dayShapes, liveSuggestions),
    [dayShapes, liveSuggestions]
  );
  const deepWorkMinutes = useMemo(
    () => computeDeepWorkHours(dayShapes),
    [dayShapes]
  );
  const deepWorkHours = Math.round(deepWorkMinutes / 60);

  // Commitments: all overdue + top 5 due-this-week (excluding The Three items)
  const topThreeTitles = useMemo(
    () => new Set(topThree.map((t) => t.title)),
    [topThree]
  );

  const { visible: commitments, totalCount: commitmentsTotalCount, overdueCount: commitmentsOverdueCount } = useMemo(() => {
    const overdue: (WeekAction & { isOverdue: boolean })[] = [];
    const dueThisWeek: (WeekAction & { isOverdue: boolean })[] = [];

    if (data?.actionSummary) {
      for (const a of data.actionSummary.overdue ?? []) {
        if (!topThreeTitles.has(a.title)) {
          overdue.push({ ...a, isOverdue: true });
        }
      }
      for (const a of data.actionSummary.dueThisWeekItems ?? []) {
        if (!topThreeTitles.has(a.title)) {
          dueThisWeek.push({ ...a, isOverdue: false });
        }
      }
    }

    const totalCount = overdue.length + dueThisWeek.length;
    const overdueCount = overdue.length;
    // Cap: max 5 overdue (by priority) + top 3 due-this-week = max 8 items
    const cappedOverdue = overdue.slice(0, 5);
    const visible = [...cappedOverdue, ...dueThisWeek.slice(0, 3)];
    return { visible, totalCount, overdueCount };
  }, [data?.actionSummary, topThreeTitles]);

  const shapeEpigraph = useMemo(
    () => computeShapeEpigraph(dayShapes),
    [dayShapes]
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

        {/* Chapter 3: Meetings skeleton */}
        <div style={{ paddingTop: 64 }}>
          <div style={{ borderTop: "2px solid var(--color-rule-light)", marginBottom: 16 }} />
          <Skeleton className="h-7 w-40 mb-4" style={skeletonBg} />
          <Skeleton className="h-4 w-72 mb-8" style={skeletonBg} />
          {[1, 2, 3, 4].map((i) => (
            <div key={i} style={{ display: "flex", gap: 12, marginBottom: 16 }}>
              <Skeleton className="h-2 w-2 rounded-full shrink-0 mt-2" style={skeletonBg} />
              <Skeleton className="h-3 w-16 shrink-0" style={skeletonBg} />
              <Skeleton className="h-4 w-56" style={skeletonBg} />
            </div>
          ))}
        </div>

        {/* Chapter 6: Shape skeleton */}
        <div style={{ paddingTop: 64 }}>
          <div style={{ borderTop: "2px solid var(--color-rule-light)", marginBottom: 16 }} />
          <Skeleton className="h-7 w-28 mb-8" style={skeletonBg} />
          {["Mon", "Tue", "Wed", "Thu", "Fri"].map((d) => (
            <div key={d} style={{ display: "flex", alignItems: "center", gap: 16, marginBottom: 12 }}>
              <span
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 12,
                  color: "var(--color-rule-light)",
                  width: 36,
                  textTransform: "uppercase",
                }}
              >
                {d}
              </span>
              <div
                style={{
                  flex: 1,
                  height: 8,
                  borderRadius: 4,
                  background: "var(--color-paper-linen)",
                }}
              />
              <Skeleton className="h-3 w-28" style={skeletonBg} />
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
              AI enrichment incomplete.{" "}
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
                {retryingEnrichment ? "Retrying..." : "Retry enrichment"}
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
              Run enrichment to see your three priorities.
            </p>
          )}
        </section>

        {/* ── Chapter 3: The Shape ───────────────────────────────────────── */}
        {dayShapes.length > 0 && (
          <section
            id="the-shape"
            className="editorial-reveal"
            style={{ paddingTop: 64 }}
          >
            <ChapterHeading title="The Shape" epigraph={shapeEpigraph} />

            <div
              style={{
                display: "flex",
                flexDirection: "column",
                gap: 12,
              }}
            >
              {dayShapes.map((shape) => {
                const maxMinutes = 480;
                const barWidth = Math.min(
                  100,
                  (shape.meetingMinutes / maxMinutes) * 100
                );
                const densityLabel = shape.density || (
                  shape.meetingCount === 0
                    ? "clear"
                    : shape.meetingCount <= 2
                      ? "light"
                      : shape.meetingCount <= 4
                        ? "moderate"
                        : shape.meetingCount <= 6
                          ? "busy"
                          : "packed"
                );

                return (
                  <div
                    key={shape.dayName}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: 16,
                    }}
                  >
                    {/* Day label */}
                    <span
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 12,
                        color: "var(--color-text-primary)",
                        width: 36,
                        flexShrink: 0,
                        textTransform: "uppercase",
                      }}
                    >
                      {shape.dayName.slice(0, 3)}
                    </span>

                    {/* Bar */}
                    <div
                      style={{
                        flex: 1,
                        height: 8,
                        borderRadius: 4,
                        background: "var(--color-paper-linen)",
                        overflow: "hidden",
                      }}
                    >
                      <div
                        style={{
                          height: "100%",
                          width: `${barWidth}%`,
                          borderRadius: 4,
                          background: "var(--color-spice-turmeric)",
                          transition: "width 0.3s ease",
                        }}
                      />
                    </div>

                    {/* Count + density */}
                    <span
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 12,
                        color: "var(--color-text-tertiary)",
                        flexShrink: 0,
                        minWidth: 130,
                        textAlign: "right",
                      }}
                    >
                      {shape.meetingCount} meeting
                      {shape.meetingCount !== 1 ? "s" : ""} &middot;{" "}
                      {densityLabel}
                    </span>
                  </div>
                );
              })}
            </div>
          </section>
        )}

        {/* ── Chapter 4: Your Meetings ───────────────────────────────────── */}
        <section
          id="your-meetings"
          className="editorial-reveal"
          style={{ paddingTop: 64 }}
        >
          <ChapterHeading
            title="Your Meetings"
            epigraph={
              externalCount > 0
                ? `${externalCount} external meeting${externalCount !== 1 ? "s" : ""} across ${accountCount} account${accountCount !== 1 ? "s" : ""} this week.`
                : undefined
            }
          />

          {meetingDays.length > 0 ? (
            <div style={{ display: "flex", flexDirection: "column", gap: 0 }}>
              {meetingDays.map((group, groupIdx) => (
                <div key={group.date}>
                  {/* Day separator (between days, not before first) */}
                  {groupIdx > 0 && (
                    <div
                      style={{
                        borderTop: "1px solid var(--color-rule-light)",
                        margin: "24px 0 20px",
                      }}
                    />
                  )}
                  {/* Day subheading */}
                  <p
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 12,
                      fontWeight: 700,
                      textTransform: "uppercase",
                      letterSpacing: "0.06em",
                      color: "var(--color-text-primary)",
                      marginBottom: 12,
                    }}
                  >
                    {group.dayName}
                  </p>

                  {/* Meeting rows */}
                  <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                    {group.meetings.map((meeting, idx) => {
                      const prep = formatPrepStatus(meeting.prepStatus);
                      const dotColor = meeting.isExternal
                        ? prep.color === "sage"
                          ? "var(--color-garden-sage)"
                          : "var(--color-spice-turmeric)"
                        : "var(--color-garden-larkspur)";

                      const row = (
                        <div
                          style={{
                            display: "flex",
                            alignItems: "baseline",
                            gap: 12,
                            padding: "6px 0",
                          }}
                        >
                          {/* Dot */}
                          <span
                            style={{
                              width: 8,
                              height: 8,
                              borderRadius: "50%",
                              background: dotColor,
                              flexShrink: 0,
                              marginTop: 6,
                              display: "inline-block",
                            }}
                          />
                          {/* Time */}
                          <span
                            style={{
                              fontFamily: "var(--font-mono)",
                              fontSize: 13,
                              color: "var(--color-text-tertiary)",
                              flexShrink: 0,
                              minWidth: 80,
                            }}
                          >
                            {meeting.time}
                          </span>
                          {/* Title */}
                          <span
                            style={{
                              fontFamily: "var(--font-serif)",
                              fontSize: 17,
                              fontWeight: 400,
                              color: "var(--color-text-primary)",
                              flex: 1,
                              minWidth: 0,
                            }}
                          >
                            {meeting.title}
                          </span>
                          {/* Prep status */}
                          {meeting.isExternal && (
                            <span
                              style={{
                                fontFamily: "var(--font-mono)",
                                fontSize: 11,
                                color:
                                  prep.color === "sage"
                                    ? "var(--color-garden-sage)"
                                    : prep.color === "terracotta"
                                      ? "var(--color-spice-terracotta)"
                                      : "var(--color-text-tertiary)",
                                flexShrink: 0,
                              }}
                            >
                              {prep.text}
                            </span>
                          )}
                        </div>
                      );

                      // Subtitle line
                      const subtitle = (
                        <p
                          style={{
                            fontFamily: "var(--font-sans)",
                            fontSize: 13,
                            color: "var(--color-text-tertiary)",
                            margin: "0 0 0 32px",
                            paddingBottom: 4,
                          }}
                        >
                          {meeting.account && <span>{meeting.account}</span>}
                          {meeting.account && meeting.type && (
                            <span> &middot; </span>
                          )}
                          <span>{meeting.type.replace(/_/g, " ")}</span>
                        </p>
                      );

                      if (meeting.meetingId) {
                        return (
                          <Link
                            key={`${group.date}-${idx}`}
                            to="/meeting/$meetingId"
                            params={{ meetingId: meeting.meetingId }}
                            style={{
                              textDecoration: "none",
                              color: "inherit",
                              borderRadius: 6,
                              transition: "background 0.15s ease",
                            }}
                            className="hover:bg-[var(--color-rule-light)]"
                          >
                            {row}
                            {subtitle}
                          </Link>
                        );
                      }
                      return (
                        <div key={`${group.date}-${idx}`}>
                          {row}
                          {subtitle}
                        </div>
                      );
                    })}
                  </div>
                </div>
              ))}
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
              No external meetings this week.
            </p>
          )}
        </section>

        {/* ── Chapter 4: Open Time ───────────────────────────────────────── */}
        <section
          id="open-time"
          className="editorial-reveal"
          style={{ paddingTop: 64 }}
        >
          <ChapterHeading
            title="Open Time"
            epigraph={
              deepWorkMinutes > 0
                ? `${deepWorkHours} hour${deepWorkHours !== 1 ? "s" : ""} of deep work available this week.`
                : undefined
            }
          />

          {/* Pull quote — AI one-liner connecting best block to need */}
          {deepWorkBlocks.length > 0 && deepWorkBlocks[0].reason && (
            <PullQuote text={deepWorkBlocks[0].reason} />
          )}

          {deepWorkBlocks.length > 0 ? (
            <div style={{ display: "flex", flexDirection: "column", gap: 36 }}>
              {deepWorkBlocks.map((block, idx) => {
                const linkTarget = resolveSuggestionLink(
                  block.actionId,
                  block.meetingId
                );
                const hasSuggestion = !!block.suggestedUse?.trim();

                return (
                  <div key={`${block.day}-${block.start}-${idx}`}>
                    {/* Block header */}
                    <p
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 12,
                        fontWeight: 700,
                        textTransform: "uppercase",
                        letterSpacing: "0.06em",
                        color: "var(--color-text-primary)",
                        marginBottom: 8,
                      }}
                    >
                      {block.day} &middot;{" "}
                      {formatBlockRange(block.start, block.end)} &middot;{" "}
                      {block.durationMinutes} min
                    </p>

                    {hasSuggestion ? (
                      <>
                        <p
                          style={{
                            fontFamily: "var(--font-serif)",
                            fontSize: 17,
                            fontWeight: 500,
                            color: "var(--color-text-primary)",
                            margin: "0 0 6px",
                          }}
                        >
                          Suggested: {block.suggestedUse}
                        </p>
                        {block.reason && (
                          <p
                            style={{
                              fontFamily: "var(--font-sans)",
                              fontSize: 14,
                              lineHeight: 1.6,
                              color: "var(--color-text-secondary)",
                              margin: "0 0 8px",
                            }}
                          >
                            {block.reason}
                          </p>
                        )}
                      </>
                    ) : (
                      <p
                        style={{
                          fontFamily: "var(--font-serif)",
                          fontSize: 15,
                          fontStyle: "italic",
                          color: "var(--color-text-tertiary)",
                          margin: "0 0 8px",
                        }}
                      >
                        No suggestion &mdash; use for deep work
                      </p>
                    )}

                    {/* Action buttons + context */}
                    <div
                      style={{
                        display: "flex",
                        alignItems: "center",
                        gap: 12,
                        flexWrap: "wrap",
                      }}
                    >
                      {block.meetingId && (
                        <>
                          <Button
                            size="sm"
                            variant="outline"
                            disabled={prefillingMeetingId === block.meetingId}
                            onClick={() =>
                              handlePrefillPrep(
                                block.meetingId!,
                                block.suggestedUse ?? "",
                                block.reason
                              )
                            }
                            style={{ fontSize: 12 }}
                          >
                            Prefill Prep
                          </Button>
                          <Button
                            size="sm"
                            variant="ghost"
                            onClick={() =>
                              draft.openDraft(
                                block.meetingId!,
                                block.reason ?? block.suggestedUse ?? ""
                              )
                            }
                            style={{ fontSize: 12 }}
                          >
                            Draft agenda
                          </Button>
                        </>
                      )}
                      {linkTarget.kind !== "none" && (
                        <Link
                          to={
                            linkTarget.kind === "action"
                              ? "/actions/$actionId"
                              : "/meeting/$meetingId"
                          }
                          params={
                            linkTarget.kind === "action"
                              ? { actionId: linkTarget.id }
                              : { meetingId: linkTarget.id }
                          }
                          style={{
                            fontFamily: "var(--font-mono)",
                            fontSize: 13,
                            color: "var(--color-text-tertiary)",
                            textDecoration: "none",
                          }}
                        >
                          &rarr; View{" "}
                          {linkTarget.kind === "action" ? "action" : "meeting"}
                        </Link>
                      )}
                    </div>
                  </div>
                );
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
              Your week is fully booked. Consider moving a meeting to make
              space.
            </p>
          )}
        </section>

        {/* ── Chapter 5: Commitments ─────────────────────────────────────── */}
        <section
          id="commitments"
          className="editorial-reveal"
          style={{ paddingTop: 64 }}
        >
          <ChapterHeading title="Commitments" />

          {commitments.length > 0 ? (
            <>
              <div
                style={{
                  display: "flex",
                  flexDirection: "column",
                  gap: 2,
                }}
              >
                {commitments.map((action) => {
                  const dueContext = formatDueContext(
                    action.dueDate,
                    action.daysOverdue
                  );
                  return (
                    <Link
                      key={action.id}
                      to="/actions/$actionId"
                      params={{ actionId: action.id }}
                      style={{
                        display: "flex",
                        alignItems: "center",
                        gap: 12,
                        padding: "10px 8px",
                        textDecoration: "none",
                        color: "inherit",
                        borderRadius: 6,
                        transition: "background 0.15s ease",
                      }}
                      className="hover:bg-[var(--color-rule-light)]"
                    >
                      {/* Checkbox circle */}
                      <span
                        style={{
                          width: 10,
                          height: 10,
                          borderRadius: "50%",
                          border: `1.5px solid ${
                            action.isOverdue
                              ? "var(--color-spice-terracotta)"
                              : "var(--color-rule-heavy)"
                          }`,
                          background: action.isOverdue
                            ? "var(--color-spice-terracotta)"
                            : "transparent",
                          flexShrink: 0,
                        }}
                      />
                      {/* Title + context */}
                      <div style={{ flex: 1, minWidth: 0 }}>
                        <p
                          style={{
                            fontFamily: "var(--font-serif)",
                            fontSize: 17,
                            fontWeight: 400,
                            lineHeight: 1.4,
                            color: "var(--color-text-primary)",
                            margin: 0,
                          }}
                        >
                          {action.title}
                        </p>
                        <p
                          style={{
                            fontFamily: "var(--font-sans)",
                            fontSize: 13,
                            color: "var(--color-text-tertiary)",
                            margin: "2px 0 0",
                          }}
                        >
                          {dueContext && (
                            <span
                              style={{
                                color: action.isOverdue
                                  ? "var(--color-spice-terracotta)"
                                  : undefined,
                              }}
                            >
                              {dueContext}
                            </span>
                          )}
                          {dueContext && action.account && (
                            <span> &middot; </span>
                          )}
                          {action.account && <span>{action.account}</span>}
                        </p>
                      </div>
                      {/* Priority badge */}
                      <span
                        style={{
                          fontFamily: "var(--font-mono)",
                          fontSize: 11,
                          color:
                            action.priority === "P1"
                              ? "var(--color-spice-terracotta)"
                              : "var(--color-text-tertiary)",
                          flexShrink: 0,
                        }}
                      >
                        {action.priority}
                      </span>
                    </Link>
                  );
                })}
              </div>

              {/* Summary line */}
              <p
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 13,
                  color: "var(--color-text-tertiary)",
                  textAlign: "center",
                  marginTop: 24,
                }}
              >
                &mdash;&mdash;&mdash; {commitmentsTotalCount} total &middot;{" "}
                {commitmentsOverdueCount} overdue
                {commitmentsTotalCount > commitments.length && (
                  <span>
                    {" "}&middot;{" "}
                    <Link
                      to="/actions"
                      search={{ search: undefined }}
                      style={{
                        color: "var(--color-garden-larkspur)",
                        textDecoration: "underline",
                      }}
                    >
                      {commitmentsTotalCount - commitments.length} more &rarr;
                    </Link>
                  </span>
                )}
                {" "}&mdash;&mdash;&mdash;
              </p>
            </>
          ) : (
            <p
              style={{
                fontFamily: "var(--font-serif)",
                fontSize: 15,
                fontStyle: "italic",
                color: "var(--color-text-tertiary)",
              }}
            >
              Nothing due this week. Rare air.
            </p>
          )}
        </section>

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

      <AgendaDraftDialog
        open={draft.open}
        onOpenChange={draft.setOpen}
        loading={draft.loading}
        subject={draft.subject}
        body={draft.body}
      />
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
