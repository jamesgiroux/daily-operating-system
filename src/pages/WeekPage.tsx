import { useState, useEffect, useCallback, useRef, useMemo } from "react";
import { BrandMark } from "@/components/ui/BrandMark";
import { Link } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";

import type {
  LiveProactiveSuggestion,
  WeekOverview,
} from "@/types";
import { cn } from "@/lib/utils";
import {
  computeShapeEpigraph,
  pickTopThree,
  resolveSuggestionLink,
  synthesizeReadiness,
  synthesizeReadinessStats,
} from "@/pages/weekPageViewModel";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { GeneratingProgress } from "@/components/editorial/GeneratingProgress";
import {
  Target,
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

        {/* Chapter 3: Shape skeleton */}
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
                  <div key={shape.dayName}>
                    <div
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

                      {/* Achievability indicator (I279) */}
                      {shape.focusImplications && shape.focusImplications.totalCount > 0 && (
                        <span
                          style={{
                            fontFamily: "var(--font-mono)",
                            fontSize: 11,
                            color: shape.focusImplications.atRiskCount > 0
                              ? "var(--color-spice-terracotta)"
                              : "var(--color-garden-sage)",
                            flexShrink: 0,
                            minWidth: 100,
                            textAlign: "right",
                          }}
                        >
                          {shape.focusImplications.achievableCount} of{" "}
                          {shape.focusImplications.totalCount} achievable
                        </span>
                      )}
                    </div>

                    {/* Top 3 feasible actions (I279) */}
                    {shape.prioritizedActions && shape.prioritizedActions.filter(pa => pa.feasible).length > 0 && (
                      <div
                        style={{
                          marginLeft: 52,
                          borderLeft: "2px solid var(--color-rule-light)",
                          paddingLeft: 12,
                          marginTop: 4,
                          marginBottom: 8,
                        }}
                      >
                        {shape.prioritizedActions
                          .filter(pa => pa.feasible)
                          .slice(0, 3)
                          .map((pa, idx) => (
                            <p
                              key={pa.action.id || idx}
                              style={{
                                fontFamily: "var(--font-sans)",
                                fontSize: 13,
                                color: "var(--color-text-secondary)",
                                margin: "2px 0",
                                lineHeight: 1.4,
                              }}
                            >
                              {pa.action.title}
                            </p>
                          ))}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
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
