import { useState, useEffect, useCallback, useMemo } from "react";
import { BrandMark } from "@/components/ui/BrandMark";
import { invoke } from "@tauri-apps/api/core";
import { Skeleton } from "@/components/ui/skeleton";

import type { DayShape, TimelineMeeting } from "@/types";
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
import { MeetingCard } from "@/components/shared/MeetingCard";
import { formatDisplayTime, formatDurationFromIso } from "@/lib/meeting-time";
import { formatEntityByline } from "@/lib/entity-helpers";
import { FolioRefreshButton } from "@/components/ui/folio-refresh-button";
import { AlertTriangle } from "lucide-react";

// =============================================================================
// WeekPage — Single data source: get_meeting_timeline
//
// ADR-0086: Intelligence is a shared service. Meeting briefings consume
// entity intelligence mechanically. The refresh button requeues prep
// generation, not AI enrichment.
// =============================================================================

export default function WeekPage() {
  const [timeline, setTimeline] = useState<TimelineMeeting[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showEarlier, setShowEarlier] = useState(false);
  const [refreshing, setRefreshing] = useState(false);

  // ─── Data loading ─────────────────────────────────────────────────────────

  const loadTimeline = useCallback(async () => {
    try {
      const data = await invoke<TimelineMeeting[]>("get_meeting_timeline", {
        daysBefore: 7,
        daysAfter: 7,
      });
      setTimeline(data);
      setError(null);
    } catch (err) {
      console.error("[WeekPage] Timeline failed:", err);
      setError(err instanceof Error ? err.message : "Failed to load timeline");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadTimeline();
  }, [loadTimeline]);

  // ─── Refresh: clear + requeue meeting preps ───────────────────────────────

  const handleRefresh = useCallback(async () => {
    setRefreshing(true);
    try {
      await invoke("refresh_meeting_preps");
      await loadTimeline();
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to refresh meeting preps"
      );
    } finally {
      setRefreshing(false);
    }
  }, [loadTimeline]);

  // ─── Live events — keep the page current without user action ──────────────

  useTauriEvent("calendar-updated", loadTimeline);
  useTauriEvent("intelligence-updated", loadTimeline);
  useTauriEvent("prep-ready", loadTimeline);

  // ─── Derived data ─────────────────────────────────────────────────────────

  const weekMeta = useMemo(() => computeWeekMeta(), []);

  const shapeDays = useMemo(
    (): DayShape[] => deriveShapeFromTimeline(timeline),
    [timeline]
  );

  const shapeEpigraph = useMemo(
    () => (shapeDays.length ? computeShapeEpigraph(shapeDays) : ""),
    [shapeDays]
  );

  const futureMeetings = useMemo(() => {
    const now = new Date();
    return timeline.filter((m) => new Date(m.startTime) > now);
  }, [timeline]);

  const readinessStats = useMemo(() => {
    if (futureMeetings.length === 0) return [];

    const readyCount = futureMeetings.filter((m) => m.hasPrep).length;
    const needsPrepCount = futureMeetings.length - readyCount;

    const stats: { label: string; color: "sage" | "terracotta" }[] = [];
    stats.push({ label: `${readyCount} ready`, color: "sage" });
    if (needsPrepCount > 0) {
      stats.push({ label: `${needsPrepCount} needs prep`, color: "terracotta" });
    }
    return stats;
  }, [futureMeetings]);

  // ─── Magazine shell ───────────────────────────────────────────────────────

  const folioActions = useMemo(
    () => (
      <FolioRefreshButton
        onClick={handleRefresh}
        loading={refreshing}
        title="Refresh meeting preps"
      />
    ),
    [handleRefresh]
  );

  const folioReadinessStats = useMemo(() => {
    if (futureMeetings.length === 0) return undefined;

    const readyCount = futureMeetings.filter((m) => m.hasPrep).length;
    const total = futureMeetings.length;

    const stats: { label: string; color: "sage" | "terracotta" }[] = [];
    if (readyCount === total) {
      stats.push({ label: `${total}/${total} ready`, color: "sage" });
    } else {
      stats.push({ label: `${readyCount} ready`, color: "sage" });
      const needsPrepCount = total - readyCount;
      if (needsPrepCount > 0) {
        stats.push({ label: `${needsPrepCount} needs prep`, color: "terracotta" });
      }
    }
    return stats;
  }, [futureMeetings]);

  const shellConfig = useMemo(
    () => ({
      folioLabel: "Weekly Forecast",
      atmosphereColor: "larkspur" as const,
      activePage: "week" as const,
      folioActions,
      folioDateText: `WEEK ${weekMeta.weekNumber} \u00b7 ${weekMeta.dateRange.toUpperCase()}`,
      folioReadinessStats,
    }),
    [folioActions, weekMeta.weekNumber, weekMeta.dateRange, folioReadinessStats]
  );
  useRegisterMagazineShell(shellConfig);
  useRevealObserver(!loading && timeline.length > 0, timeline);

  // ─── Timeline grouping ────────────────────────────────────────────────────

  const timelineGroups = useMemo(() => {
    const now = new Date();
    const todayStr = now.toISOString().slice(0, 10);

    const byDate = new Map<string, TimelineMeeting[]>();
    for (const m of timeline) {
      const dateKey = m.startTime.slice(0, 10);
      if (!byDate.has(dateKey)) byDate.set(dateKey, []);
      byDate.get(dateKey)!.push(m);
    }

    const sortedDates = [...byDate.keys()].sort();

    const past: DateGroup[] = [];
    const today: DateGroup[] = [];
    const future: DateGroup[] = [];

    for (const dateKey of sortedDates) {
      const meetings = byDate.get(dateKey)!;
      const date = new Date(dateKey + "T12:00:00");
      const diffDays = Math.round(
        (date.getTime() - new Date(todayStr + "T12:00:00").getTime()) /
          (1000 * 60 * 60 * 24)
      );

      let label: string;
      if (diffDays === 0) label = "Today";
      else if (diffDays === -1) label = "Yesterday";
      else if (diffDays === 1) label = "Tomorrow";
      else if (diffDays < 0)
        label = `${Math.abs(diffDays)} days ago \u2014 ${date.toLocaleDateString("en-US", { weekday: "long", month: "short", day: "numeric" })}`;
      else
        label = date.toLocaleDateString("en-US", {
          weekday: "long",
          month: "short",
          day: "numeric",
        });

      const group = { dateKey, label, meetings };
      if (diffDays < 0) past.push(group);
      else if (diffDays === 0) today.push(group);
      else future.push(group);
    }

    const earlierPast = past.filter((g) => {
      const diff = Math.round(
        (new Date(g.dateKey + "T12:00:00").getTime() -
          new Date(todayStr + "T12:00:00").getTime()) /
          (1000 * 60 * 60 * 24)
      );
      return diff < -2;
    });
    const recentPast = past.filter((g) => {
      const diff = Math.round(
        (new Date(g.dateKey + "T12:00:00").getTime() -
          new Date(todayStr + "T12:00:00").getTime()) /
          (1000 * 60 * 60 * 24)
      );
      return diff >= -2;
    });

    return { earlierPast, recentPast, today, future };
  }, [timeline]);

  // ─── Loading skeleton ─────────────────────────────────────────────────────

  if (loading) {
    const skeletonBg = { background: "var(--color-rule-light)" };
    return (
      <div style={{ maxWidth: 760, margin: "0 auto", padding: "0 40px" }}>
        <div style={{ paddingTop: 80 }}>
          <Skeleton className="h-3 w-20 mb-2" style={skeletonBg} />
          <Skeleton className="h-7 w-44 mb-6" style={skeletonBg} />
          <Skeleton className="h-3 w-52" style={skeletonBg} />
        </div>
        <div style={{ paddingTop: 56 }}>
          <Skeleton className="h-2.5 w-20 mb-4" style={skeletonBg} />
          {[1, 2, 3, 4, 5].map((i) => (
            <div
              key={i}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 12,
                marginBottom: 24,
              }}
            >
              <Skeleton className="h-3 w-9" style={skeletonBg} />
              <Skeleton className="h-2 flex-1 rounded-full" style={skeletonBg} />
              <Skeleton className="h-3 w-20" style={skeletonBg} />
            </div>
          ))}
        </div>
        <div style={{ paddingTop: 64 }}>
          <div
            style={{
              borderTop: "2px solid var(--color-rule-light)",
              marginBottom: 16,
            }}
          />
          <Skeleton className="h-7 w-36 mb-8" style={skeletonBg} />
          {[1, 2, 3].map((d) => (
            <div key={d} style={{ marginBottom: 20 }}>
              <Skeleton className="h-4 w-40 mb-3" style={skeletonBg} />
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 12,
                  marginBottom: 8,
                  marginLeft: 12,
                }}
              >
                <Skeleton
                  className="h-2 w-2 rounded-full"
                  style={skeletonBg}
                />
                <Skeleton className="h-4 w-48" style={skeletonBg} />
                <Skeleton className="h-3 w-16" style={skeletonBg} />
              </div>
            </div>
          ))}
        </div>
        <div style={{ textAlign: "center", padding: "72px 0 48px" }}>
          <Skeleton className="mx-auto h-4 w-16" style={skeletonBg} />
        </div>
      </div>
    );
  }

  // ─── Empty state ──────────────────────────────────────────────────────────

  if (timeline.length === 0) {
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
        <div
          style={{ color: "var(--color-garden-larkspur)", marginBottom: 24 }}
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
        {error && <ErrorCard error={error} />}
      </div>
    );
  }

  // ─── Full editorial render ────────────────────────────────────────────────

  return (
    <>
      <div style={{ maxWidth: 760, margin: "0 auto", padding: "0 40px" }}>
        {/* ── Week Header ───────────────────────────────────────────── */}
        <section className={w.weekHeader}>
          <p className={w.weekLabel}>Week {weekMeta.weekNumber}</p>
          <p className={w.weekDate}>{weekMeta.dateRange}</p>
          {(readinessStats.length > 0 || futureMeetings.length > 0) && (
            <p className={w.weekVitals}>
              {readinessStats.map((stat) => (
                <span
                  key={stat.label}
                  className={
                    stat.color === "sage" ? w.vitalsReady : w.vitalsAlert
                  }
                >
                  {stat.label}
                </span>
              ))}
              {futureMeetings.length > 0 && (
                <span>
                  {futureMeetings.length} meeting
                  {futureMeetings.length !== 1 ? "s" : ""}
                </span>
              )}
            </p>
          )}
        </section>

        {/* ── This Week — The Shape ──────────────────────────────────── */}
        {shapeDays.length > 0 && (
          <section className={cn("editorial-reveal", w.shapeSection)}>
            <p className={w.shapeLabel}>This Week</p>
            <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>
              {shapeDays.map((day) => {
                const barPct = Math.min(
                  100,
                  (day.meetingMinutes / 480) * 100
                );
                const isHeavy =
                  day.meetingCount >= 5 || day.meetingMinutes >= 360;
                const todayStr = new Date().toISOString().slice(0, 10);
                const isToday = day.date === todayStr;
                const isPast = day.date < todayStr;

                return (
                  <div
                    key={day.date}
                    className={cn(w.shapeRow, isPast && w.shapeRowPast)}
                  >
                    <span
                      className={cn(
                        w.shapeDayLabel,
                        isToday && w.shapeDayLabelToday
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
                              : undefined
                        )}
                        style={{ width: `${barPct}%` }}
                      />
                    </div>
                    <span className={w.shapeCount}>
                      {day.meetingCount}m &middot; {day.density}
                    </span>
                  </div>
                );
              })}
            </div>
            {shapeEpigraph && (
              <p className={w.shapeEpigraph}>{shapeEpigraph}</p>
            )}
          </section>
        )}

        {/* ── The Timeline ───────────────────────────────────────────── */}
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
                onKeyDown={(e) =>
                  e.key === "Enter" && setShowEarlier(!showEarlier)
                }
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

          {/* Past — Recent */}
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

        {/* ── Error ──────────────────────────────────────────────────── */}
        {error && (
          <div style={{ marginTop: 48 }}>
            <ErrorCard error={error} />
          </div>
        )}

        {/* ── Finis ──────────────────────────────────────────────────── */}
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

// =============================================================================
// Supporting components
// =============================================================================

interface DateGroup {
  dateKey: string;
  label: string;
  meetings: TimelineMeeting[];
}

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
          const needsPrep = !m.hasPrep;

          return (
            <MeetingCard
              key={m.id}
              id={m.id}
              title={m.title}
              displayTime={formatDisplayTime(m.startTime)}
              duration={
                formatDurationFromIso(m.startTime, m.endTime) ?? undefined
              }
              meetingType={m.meetingType}
              entityByline={
                m.entities.length > 0
                  ? (formatEntityByline(m.entities) ?? undefined)
                  : undefined
              }
              intelligenceQuality={
                !isPast ? (m.intelligenceQuality ?? undefined) : undefined
              }
              temporalState={isPast ? "past" : undefined}
              showNavigationHint={isPast}
              subtitleExtra={
                !isPast ? (
                  <>
                    {needsPrep && (
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
                  <svg
                    width="12"
                    height="12"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <polyline points="20 6 9 17 4 12" />
                  </svg>
                  <span
                    style={{
                      maxWidth: 240,
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: "nowrap",
                    }}
                  >
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
                      {m.followUpCount} follow-up
                      {m.followUpCount !== 1 ? "s" : ""}
                    </span>
                  )}
                </span>
              )}
              {isPast &&
                !(m.hasOutcomes && m.outcomeSummary) &&
                m.followUpCount != null &&
                m.followUpCount > 0 && (
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
                    {m.followUpCount} follow-up
                    {m.followUpCount !== 1 ? "s" : ""}
                  </span>
                )}
            </MeetingCard>
          );
        })}
      </div>
    </div>
  );
}
